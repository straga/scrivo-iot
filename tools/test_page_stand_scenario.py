"""The stand plays a scenario: each second a frame the automaton's own code wrote
(crates/automaton_card, example `scenario`) puts its values in place of what the stand carried, and the rest stay -
and the stand with a frame in it answers what the board's dictionary describes, nothing missing and nothing foreign.

    uv run --no-project --with pytest pytest -q tools/test_page_stand_scenario.py
"""
import json
import os
import subprocess
import threading
import time

from page_stand import Board, dictionary_of, play
from state_contract import values_check

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
VALUES = os.path.join(HERE, "page_stand_values.json")


def test_a_frame_puts_its_values_in_place_and_leaves_the_others():
    b = Board({"contract": 7, "version": "k", "things": [{"card": "c", "name": n, "kinds": ["on"]} for n in "abc"]},
              {"values": [[0, False], [1, False], [2, False]]}, None, 2)
    b.show([[1, True]])
    assert b.values == [False, True, False]


def test_the_stand_with_a_frame_of_the_scenario_keeps_the_burners_dictionary():
    dictionary = dictionary_of(ROOT)
    out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "automaton_card", "--example", "scenario", "--",
                          VALUES, os.path.join(HERE, "stand_scenarios", "steady.json")],
                         cwd=ROOT, capture_output=True, text=True)
    assert out.returncode == 0, out.stderr[-2000:]
    frames = json.loads(out.stdout)["frames"]
    b = Board(dictionary, json.load(open(VALUES)), None, 2)
    for frame in (frames[0], frames[-1]):
        b.show(frame["values"])
        assert values_check(dictionary, b.state()) == []


def test_a_state_with_a_value_left_out_is_caught_by_its_name():
    dictionary = dictionary_of(ROOT)
    state = Board(dictionary, json.load(open(VALUES)), None, 2).state()
    gone = state["values"].pop(3)
    refused = values_check(dictionary, state)
    name = dictionary["things"][gone[0]]["name"]
    assert len(refused) == 1 and name in refused[0] and "no value" in refused[0], refused


# ---- the burner holds what a person chose, on the stand ----
#
# The frames come from the automaton's own code on the scenario, and are read by the words the card says: the stage and, at a
# burning off, why. THE SETPOINT'S CROSSING IS WORKED OUT HERE FROM THE SCENARIO'S OWN POINTS, not read off the frames, so
# the second the stand changes stage is judged against the second the sensor said it.

def frames_of(scenario, tmp_path, name):
    """The frames [(stage, why)] of SCENARIO (a dict) on the stand's state, one per second."""
    path = tmp_path / name
    path.write_text(json.dumps(scenario))
    out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "automaton_card", "--example", "scenario", "--", VALUES, str(path)],
                         cwd=ROOT, capture_output=True, text=True)
    assert out.returncode == 0, out.stderr[-2000:]
    dictionary = dictionary_of(ROOT)
    at = {(t["card"], t["name"], t.get("part_of")): i for i, t in enumerate(dictionary["things"])}
    stage, why = at[("automaton", "stage", None)], at[("automaton", "why", "doing")]
    seen = []
    for frame in json.loads(out.stdout)["frames"]:
        values = dict((a, v) for a, v in frame["values"])
        seen.append((values.get(stage), values.get(why)))
    return seen


def line_at(points, t):
    """The reading at second T on straight lines between POINTS [second, value], as the stand reads it (scenario.rs, line_at)."""
    if t <= points[0][0]:
        return points[0][1]
    if t >= points[-1][0]:
        return points[-1][1]
    for (a, x), (b, y) in zip(points, points[1:]):
        if a <= t <= b:
            return x + ((y if y is not None else x) - x) * (t - a) / (b - a)


def first_at_or_over(points, limit, seconds):
    return next(t for t in range(seconds) if (line_at(points, t) or -1) >= limit)


def scenario_file(name):
    return json.load(open(os.path.join(HERE, "stand_scenarios", name)))


def test_the_recorded_run_held_by_the_return_smoulders_the_second_the_return_reaches_the_setpoint(tmp_path):
    scenario = scenario_file("hold_by_return.json")
    seconds = scenario["seconds"]
    t_return = first_at_or_over(scenario["return_c"], 40, seconds)
    t_boiler = first_at_or_over(scenario["boiler_c"], 40, seconds)
    stages = [s for s, _ in frames_of(scenario, tmp_path, "held.json")]
    assert t_boiler < t_return, "the record's boiler must pass the setpoint first, or the pair proves nothing"
    assert stages[t_return - 1] == "working" and stages[t_return] == "smouldering", (t_return, stages[t_return - 3:t_return + 3])
    assert "smouldering" not in stages[:t_return], "the burner smouldered before the return reached the setpoint"
    # the boiler passed the same setpoint earlier, and the burner did not notice: it worked through that second
    assert stages[t_boiler - 1:t_boiler + 2] == ["working"] * 3, stages[t_boiler - 3:t_boiler + 3]


def test_the_same_run_held_by_the_boiler_never_smoulders_on_the_return_it_passes(tmp_path):
    scenario = scenario_file("hold_by_return.json")
    scenario.pop("settings")
    stages = [s for s, _ in frames_of(scenario, tmp_path, "boiler.json")]
    assert "smouldering" not in stages, "the burner smouldered by the return without being set to hold it"


def test_a_return_whose_socket_is_let_go_in_the_middle_of_the_work_burns_the_fire_off_naming_the_return(tmp_path):
    scenario = scenario_file("hold_by_return_socket_let_go.json")
    last_number = max(t for t in range(scenario["seconds"]) if line_at(scenario["return_c"], t) is not None)
    # ten periods of the round (2 s) and at least 30 s of silence: the board's rule for a sensor's role (crates/module)
    burnt = last_number + 31
    seen = frames_of(scenario, tmp_path, "let_go.json")
    assert [s for s, _ in seen[last_number:burnt]] == ["working"] * (burnt - last_number), seen[burnt - 3:burnt + 2]
    assert seen[burnt] == ("burning_off", "return_silent"), seen[burnt - 1:burnt + 3]


def test_the_same_socket_let_go_under_a_burner_that_holds_the_boiler_changes_nothing(tmp_path):
    scenario = scenario_file("hold_by_return_socket_let_go.json")
    scenario.pop("settings")
    seen = frames_of(scenario, tmp_path, "let_go_boiler.json")
    assert {s for s, _ in seen[300:]} == {"working"}, "a sensor the mode does not use put the fire out"


# ---- a scenario that plays for as long as a measure needs ----
#
# A memory measure of thirty minutes needs a stream that does not end with the scenario, and one that comes as often as the
# burner's own (about 1.8 events a second, the board's; the stand puts out a frame a second).

class Shown:
    """A board that only writes down what it is shown."""

    def __init__(self):
        self.frames = []

    def show(self, values):
        self.frames.append(values)


FRAMES = [{"at": 0, "values": [[0, 1]]}, {"at": 1, "values": [[0, 2]]}]


def played(**how):
    board, stop = Shown(), threading.Event()
    thread = threading.Thread(target=play, args=(board, FRAMES, 0, False), kwargs=dict(how, stop=stop), daemon=True)
    thread.start()
    return board, stop, thread


def test_a_scenario_played_round_again_starts_over_at_its_end_and_keeps_its_pace():
    board, stop, thread = played(loop=True, pace=0.01)
    end = time.time() + 5
    while len(board.frames) < 5 and time.time() < end:
        time.sleep(0.01)
    stop.set()
    thread.join(timeout=2)
    assert board.frames[:5] == [[[0, 1]], [[0, 2]], [[0, 1]], [[0, 2]], [[0, 1]]]
    assert not thread.is_alive()


def test_a_scenario_not_played_round_again_ends_with_its_last_frame():
    board, stop, thread = played(pace=0.01)
    thread.join(timeout=2)
    assert board.frames == [[[0, 1]], [[0, 2]]] and not thread.is_alive()
