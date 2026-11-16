"""The burner is not lit again by the thermostat before the wait is up, by the stand: the new burner's own thermostat and automaton (burner_api, example `burner_frames`) play a run of the water climbing to the
setpoint, the fire smouldering and burning out by itself, and the water cooling under the line it would be lit at - and the card counts the wait down while the order is Keep where it would be Burn. Expected from the run's own
points - the second the fire stops is read from the run, and every other second is worked out from the scenario's water and the wait it sets - and from what the stage says a wait is, not from the code that makes the frames.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_burner_does_not_cycle_on_the_stand.py
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

SCENARIO = os.path.join(HERE, "stand_scenarios", "cooling_waits.json")
SETPOINT, HYSTERESIS = 60.0, 4.0
WAIT = 240
COLUMNS = ["thermostat/heat", "thermostat/lock", "thermostat/switch", "automaton/stage"]


def played(scenario):
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "scenario.json")
        with open(path, "w") as f:
            json.dump(scenario, f)
        frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
    return named(dictionary_of(ROOT, "burner"), frames, COLUMNS)


def scenario():
    return json.load(open(SCENARIO))


def water_at(points, t):
    for (t0, v0), (t1, v1) in zip(points, points[1:]):
        if t0 <= t <= t1:
            return v0 + (v1 - v0) * (t - t0) / (t1 - t0)
    return points[-1][1]


def stopped_at(run):
    """The first second the fire stands in Stop after it burned: the end of burning, read from the run."""
    burned = next(t for t, s in enumerate(run) if s["automaton/stage"] == "working")
    return next(t for t in range(burned, len(run)) if run[t]["automaton/stage"] == "stop")


def test_the_fire_burns_out_by_itself_and_stops_before_the_water_is_asked_to_light_it_again():
    run = played(scenario())
    stop = stopped_at(run)
    assert "smouldering" in {s["automaton/stage"] for s in run[:stop]} and "burning_off" in {s["automaton/stage"] for s in run[:stop]}
    assert water_at(scenario()["boiler_c"], stop) < SETPOINT - HYSTERESIS, "the water is already under the line the fire is lit at when it stops: this is the case the wait is for"


def test_the_card_counts_the_wait_down_from_the_end_of_burning_a_second_at_a_time_and_says_none_before_and_after():
    run = played(scenario())
    stop = stopped_at(run)
    # the first second the thermostat is told the fire is out is the end of burning: the whole wait is left
    started = next(t for t in range(stop, len(run)) if run[t]["thermostat/lock"] is not None)
    assert abs(started - stop) <= 1 and run[started]["thermostat/lock"] == WAIT, (stop, started, run[started]["thermostat/lock"])
    for k in range(0, WAIT - 1):
        assert run[started + k]["thermostat/lock"] == WAIT - k, (k, run[started + k]["thermostat/lock"])
    assert all(run[t]["thermostat/lock"] is None for t in range(0, stop - 1)), "the fire burned: no wait"
    assert run[started + WAIT]["thermostat/lock"] is None, "the wait is over"


def test_while_it_runs_the_water_under_the_line_is_kept_and_the_fire_stays_out_and_when_it_is_over_the_fire_is_lit():
    sc = scenario()
    run = played(sc)
    stop = stopped_at(run)
    started = next(t for t in range(stop, len(run)) if run[t]["thermostat/lock"] is not None)
    for t in range(started, started + WAIT):
        assert water_at(sc["boiler_c"], t) < SETPOINT - HYSTERESIS
        assert run[t]["thermostat/heat"] == "keep", (t, run[t]["thermostat/heat"], "where it would be burn")
        assert run[t]["automaton/stage"] == "stop", (t, "a fire lit inside the wait")
    lit = next(t for t in range(started, len(run)) if run[t]["automaton/stage"] == "purge")
    assert abs(lit - (started + WAIT)) <= 2, "lit at %d, the wait is up at %d" % (lit, started + WAIT)
    assert run[lit]["thermostat/heat"] == "burn"
    assert any(s["automaton/stage"] == "working" for s in run[lit:]), "the fire the end of the wait lit never worked"


def test_with_no_wait_the_same_run_lights_the_fire_the_second_the_water_is_under_the_line_and_the_card_says_no_time():
    sc = copy.deepcopy(scenario())
    del sc["settings"]["auto_lock_min"]
    run = played(sc)
    stop = stopped_at(run)
    assert all(s["thermostat/lock"] is None for s in run), "no wait is set: the card has no time to say"
    relit = next(t for t in range(stop, len(run)) if run[t]["automaton/stage"] == "purge")
    assert relit - stop <= 3, "no wait: lit %d s after the fire stopped" % (relit - stop)


def test_a_light_in_the_middle_of_the_wait_lights_the_fire_then_and_ends_the_wait():
    sc = copy.deepcopy(scenario())
    sc["press"] = [[2, "start"], [650, "light"]]
    run = played(sc)
    assert run[640]["automaton/stage"] == "stop" and run[640]["thermostat/lock"] is not None, "still inside the wait"
    lit = next(t for t in range(645, len(run)) if run[t]["automaton/stage"] == "purge")
    assert lit == 650, "the Light of 650 s lit the fire at %d" % lit
    assert all(run[t]["thermostat/lock"] is None for t in range(650, len(run))), "a Light ends the wait"


def test_a_start_in_the_middle_of_the_wait_is_the_switch_and_the_wait_goes_on():
    sc = copy.deepcopy(scenario())
    sc["press"] = [[2, "start"], [650, "start"]]
    run = played(sc)
    assert run[640]["automaton/stage"] == "stop" and run[640]["thermostat/lock"] is not None, "still inside the wait"
    assert all(run[t]["automaton/stage"] == "stop" and run[t]["thermostat/lock"] is not None for t in range(645, 700)), "a Start lit the fire or ended the wait"


def test_water_under_the_floor_lights_the_fire_the_second_it_gets_under_it_whatever_is_left():
    sc = copy.deepcopy(scenario())
    sc["boiler_c"] = [[0, 40], [150, 40], [400, 60.5], [620, 50], [700, 40], [1000, 40]]
    sc["settings"]["auto_lock_c"] = "45"
    run = played(sc)
    stop = stopped_at(run)
    crossing = next(t for t in range(620, 1000) if water_at(sc["boiler_c"], t) < 45.0)
    assert run[crossing - 3]["automaton/stage"] == "stop" and run[crossing - 3]["thermostat/lock"] is not None, "kept above the floor"
    lit = next(t for t in range(crossing - 3, len(run)) if run[t]["automaton/stage"] == "purge")
    assert crossing <= lit <= crossing + 2, "the water went under the floor at %d, the fire was lit at %d" % (crossing, lit)
    assert lit < stop + WAIT, "and the wait was not over"


def test_a_stop_in_the_middle_of_the_wait_leaves_the_fire_out_and_the_switch_off():
    sc = copy.deepcopy(scenario())
    sc["press"] = [[2, "start"], [650, "stop"]]
    run = played(sc)
    assert run[655]["thermostat/switch"] == "off"
    assert "purge" not in {s["automaton/stage"] for s in run[651:]}, "a stop was a light"
