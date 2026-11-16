"""The power steps on the stand: the burner burns on the numbers of the chosen step in every hold, and a number of the table a person edits reaches the auger and the fan the second it is saved - played by the new burner's
own automaton (burner_api, example `burner_frames`) on a run made up here, the readings by the second. Expected from what a step is - a dose, a pause and a draught - and not from how the code reads them: the auger turns
for the dose and rests for the pause, the fan is asked for the draught.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_steps_on_the_stand.py
"""
import json
import os
import sys
import tempfile

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from module_compare import frames_of, named  # noqa: E402
from page_stand import dictionary_of  # noqa: E402

ENTITIES = ["automaton/stage", "relays/auger", "automaton/fan"]

# a run: Start at 2 s, the flame lit and steady, the flue climbing so the fuel counts as arriving, the water cold so a hold by a temperature keeps working
RUN = {"seconds": 900, "press": [[2, "start"], [3, "light"]], "flame_mv": [[0, 3150], [94, 3150], [95, 1800], [104, 1800], [105, 900], [900, 900]],
       "boiler_c": [[0, 38]], "flue_c": [[0, 24], [100, 30], [400, 140], [900, 145]]}


def played(settings):
    """The seconds of the run on the new burner with SETTINGS over the defaults: what each entity said, second by second."""
    scenario = dict(RUN, settings=settings)
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "scenario.json")
        with open(path, "w") as f:
            json.dump(scenario, f)
        frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
    return named(dictionary_of(ROOT, "burner"), frames, ENTITIES)


def working(seconds):
    """What the auger did in the stretch the burner worked: the lengths of each turn and each rest in whole seconds, the partial first and last left out, and the draughts the fan was asked for."""
    inside = [s for s in seconds if s.get("automaton/stage") == "working"]
    runs, last = [], None
    for s in inside:
        on = s.get("relays/auger") is True
        if last is None or last[0] != on:
            runs.append([on, 0])
        runs[-1][1] += 1
        last = runs[-1]
    inner = runs[1:-1]
    return {"turns": sorted({n for on, n in inner if on}), "rests": sorted({n for on, n in inner if not on}), "fan": sorted({s["automaton/fan"] for s in inside if isinstance(s.get("automaton/fan"), (int, float))}),
            "seconds": len(inside), "runs": len(inner)}


TABLE = {"auto_8_on_s": "3", "auto_8_off_s": "9", "auto_8_pa": "27"}


@pytest.mark.parametrize("hold", ["power", "boiler"])
def test_in_every_hold_the_burner_burns_on_the_numbers_of_the_chosen_step(hold):
    got = working(played(dict(TABLE, auto_hold=hold, auto_step="8", role_ntc_1="boiler")))
    assert got["runs"] >= 6, got
    assert (got["turns"], got["rests"], got["fan"]) == ([3], [9], [27]), got


@pytest.mark.parametrize("hold", ["power", "boiler"])
def test_another_chosen_step_burns_on_its_own_numbers_and_the_edit_of_the_first_is_not_in_it(hold):
    got = working(played(dict(TABLE, auto_hold=hold, auto_step="5", role_ntc_1="boiler")))
    assert (got["turns"], got["rests"], got["fan"]) == ([2], [25], [18]), got


def test_an_edit_of_the_chosen_steps_pause_is_the_pause_the_auger_rests_for():
    short = working(played({"auto_hold": "power", "auto_step": "6", "auto_6_off_s": "11"}))
    long = working(played({"auto_hold": "power", "auto_step": "6", "auto_6_off_s": "40"}))
    assert (short["turns"], short["rests"]) == ([2], [11]), short
    assert (long["turns"], long["rests"]) == ([2], [40]), long
