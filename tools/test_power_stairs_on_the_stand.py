"""The burner heats the boiler on three steps by the stand: the new burner's own thermostat and automaton (burner_api, example `burner_frames`) play a run of the water climbing to the setpoint, and the card says the
level the order is on while the fire's draught follows it - 6 kW far from the setpoint, 5 kW from 8 degrees under it, 3 kW from 3 degrees under it, and at the setpoint the fire smoulders. Expected from the run's own points - the
second the water crosses a line is worked out here from the scenario - and from the owner's table of steps (draught 22, 18 and 11 Pa on 6, 5 and 3 kW), not from the code that makes the frames.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_power_stairs_on_the_stand.py
"""
import copy
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from module_compare import frames_of, named  # noqa: E402
from page_stand import dictionary_of  # noqa: E402

SCENARIO = os.path.join(HERE, "stand_scenarios", "heating_steps_down.json")
# the owner's table: the draught of the step in pascals
DRAUGHT = {"6": 22.0, "5": 18.0, "3": 11.0}
SETPOINT, H2, H1 = 60.0, 8.0, 3.0


def played(scenario):
    """The seconds of the run: the level and the distance of the order, the stage, and the draught asked."""
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "scenario.json")
        with open(path, "w") as f:
            json.dump(scenario, f)
        frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
    return named(dictionary_of(ROOT, "burner"), frames, ["thermostat/rung", "thermostat/left", "automaton/doing", "automaton/fan"])


def water_at(points, t):
    """The water at second T on the straight lines between the scenario's points."""
    for (t0, v0), (t1, v1) in zip(points, points[1:]):
        if t0 <= t <= t1:
            return v0 + (v1 - v0) * (t - t0) / (t1 - t0)
    return points[-1][1]


def first_second(points, seconds, past):
    """The first second the water, by the scenario's own points, is past the level PAST."""
    return next(t for t in range(seconds) if water_at(points, t) > past)


def scenario():
    return json.load(open(SCENARIO))


def test_the_card_says_the_level_the_order_is_on_and_the_draught_follows_it_from_6_to_5_to_3_kilowatts():
    sc = scenario()
    run = played(sc)
    working = [(t, s) for t, s in enumerate(run) if s.get("automaton/doing") == "working"]
    assert working, "the burner never worked"
    seen = []
    for _, s in working:
        if not seen or seen[-1] != s["thermostat/rung"]:
            seen.append(s["thermostat/rung"])
    assert seen == ["big", "mid", "small"], seen
    for kw, rung in (("6", "big"), ("5", "mid"), ("3", "small")):
        draughts = {s["automaton/fan"] for _, s in working if s["thermostat/rung"] == rung}
        assert draughts == {DRAUGHT[kw]}, "%s on the %s level: %s" % (kw, rung, draughts)


def test_each_level_begins_the_second_the_water_crosses_its_line_by_the_scenarios_own_points():
    sc = scenario()
    run = played(sc)
    points, seconds = sc["boiler_c"], sc["seconds"]
    started = {rung: next(t for t, s in enumerate(run) if s["thermostat/rung"] == rung) for rung in ("mid", "small")}
    # the middle level is at H2 under the setpoint, the small one nearer than H1: the water is past SETPOINT - H2, and past SETPOINT - H1
    assert abs(started["mid"] - first_second(points, seconds, SETPOINT - H2)) <= 2, started
    assert abs(started["small"] - first_second(points, seconds, SETPOINT - H1)) <= 2, started


def test_at_the_setpoint_the_fire_smoulders_on_the_small_level_and_the_distance_goes_below_nought():
    sc = scenario()
    run = played(sc)
    smoulder = next(t for t, s in enumerate(run) if s.get("automaton/doing") == "smouldering")
    assert abs(smoulder - first_second(sc["boiler_c"], sc["seconds"], SETPOINT)) <= 2
    assert run[smoulder]["thermostat/rung"] == "small"
    assert run[smoulder + 10]["thermostat/left"] <= 0.0, "past the setpoint the water is not under it"


def test_the_distance_the_card_says_is_the_setpoint_less_the_water_of_that_second():
    sc = scenario()
    run = played(sc)
    for t in (160, 250, 300, 350, 380):
        said = run[t]["thermostat/left"]
        assert abs(said - (SETPOINT - water_at(sc["boiler_c"], t))) < 0.3, (t, said)


def test_with_no_step_chosen_the_run_is_on_the_one_step_from_the_first_second_to_the_setpoint():
    sc = copy.deepcopy(scenario())
    sc["settings"]["auto_step_mid"] = "chosen"
    sc["settings"]["auto_step_small"] = "chosen"
    run = played(sc)
    working = [s for s in run if s.get("automaton/doing") == "working"]
    assert working
    assert {s["automaton/fan"] for s in working} == {DRAUGHT["6"]}, "the draught moved with no step of its own chosen"
    assert {s["thermostat/rung"] for s in working} == {"big", "small"}, "the card tells the level it is on even when the step is one: the way down is the same"


def test_a_step_below_a_chosen_one_that_is_the_same_row_is_told_as_the_row_it_is():
    sc = copy.deepcopy(scenario())
    sc["settings"]["auto_step_mid"] = "6"
    run = played(sc)
    working = [s for s in run if s.get("automaton/doing") == "working"]
    assert "mid" not in {s["thermostat/rung"] for s in working}, "a middle that is the big row's was told as a level of its own"
