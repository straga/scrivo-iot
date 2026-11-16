"""The pause after the fire goes out follows a fire that burned. Played on the stand by the new burner's own thermostat and automaton (burner_api, example `burner_frames`) with a pause of two hours set:
a burner whose flame never came (dark.json: a kindling that did not take, then the fault) and that a person lets out of the fault into a stop has NO pause; one that burned (room_stops_with_pause.json) and went
out has one, as it always had. Expected from what a person at the boiler finds: the pause is for a boiler that burned and must rest, and 20.10 one stood 1:59 for a flame it never had.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_pause_follows_a_real_fire_on_the_stand.py
"""
import copy
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import json  # noqa: E402
import tempfile  # noqa: E402

from module_compare import frames_of, named  # noqa: E402
from page_stand import dictionary_of  # noqa: E402
from test_house_asks_on_the_stand import ROOT, scenario  # noqa: E402

COLUMNS = ["thermostat/lock", "automaton/stage"]

PAUSE_MIN = "120"


def played(sc):
    """The scenario played by the burner's own thermostat and automaton: the rows of every second, by (card, name), as the page draws them."""
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "scenario.json")
        with open(path, "w") as f:
            json.dump(sc, f)
        frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
    return named(dictionary_of(ROOT, "burner"), frames, COLUMNS)


def run_of(name, press):
    sc = copy.deepcopy(scenario(name))
    sc["settings"] = {**sc.get("settings", {}), "auto_lock_min": PAUSE_MIN}
    sc["press"] = press
    return played(sc)


def stage(rows):
    return rows.get("automaton/stage")


def paused(run):
    """The seconds of the run in which the thermostat says there is a wait before a light: its row `lock` says seconds."""
    return [t for t, rows in enumerate(run) if rows.get("thermostat/lock")]


def test_a_kindling_that_never_took_and_a_fault_let_go_into_a_stop_leave_no_pause():
    run = run_of("dark.json", [[2, "start"], [1300, "stop"]])
    stages = [stage(rows) for rows in run]
    assert "kindling" in stages and "fault" in stages and stages[-1] == "stop", "the scaffolding: the burner tried, faulted and was let out into a stop: %s" % sorted(set(map(str, stages)))
    assert paused(run) == [], "a pause stood for a fire that never burned, at seconds %s" % paused(run)[:3]


def test_a_burner_that_burned_and_went_out_has_its_pause_as_before():
    run = played(copy.deepcopy(scenario("room_stops_with_pause.json")))
    assert paused(run), "the scenario of a fire and its pause has no pause: the scaffolding or the pause is lost"
