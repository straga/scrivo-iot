"""A Light carries the fire to the nominal, on the stand: the burner's own automaton, thermostat, demand and page cards play stand_scenarios/light_to_target.json (29.10 15:31: the room at 21.1 °C under a target of 22 with a band of 2) and
light_refused.json (the room at 22.3 over its target). Second by second the table of stage, cause, what the house asks, the manual cycle and the room is the proof of each.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_light_to_nominal_on_the_stand.py
"""
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from module_compare import frames_of, named  # noqa: E402
from page_stand import dictionary_of  # noqa: E402

COLUMNS = ["automaton/stage", "demand/asked", "demand/why", "thermostat/manual", "thermostat/manual_c", "temperatures/ntc_4", "thermostat/heat", "temperatures/ntc_1"]
RUNS = {}


def run(name):
    if name not in RUNS:
        sc = json.load(open(os.path.join(HERE, "stand_scenarios", name + ".json")))
        with tempfile.TemporaryDirectory() as work:
            path = os.path.join(work, "scenario.json")
            with open(path, "w") as f:
                json.dump(sc, f)
            frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
        RUNS[name] = (sc, named(dictionary_of(ROOT, "burner"), frames, COLUMNS))
    return RUNS[name]


def table(frames, lo, hi, step=1):
    return "\n".join("%4d %-12s %-5s %-15s %-6s %-5s %-6s %-7s %s" % ((t,) + tuple(frames[t].get(c) for c in COLUMNS)) for t in range(lo, min(hi, len(frames)), step))


def first(frames, stage, after=0):
    return next((t for t in range(after, len(frames)) if frames[t]["automaton/stage"] == stage), None)


def room(frames, t):
    value = frames[t].get("temperatures/ntc_4")
    return value["number"]["value"] if isinstance(value, dict) and "number" in value else value


def test_a_light_at_21_1_under_a_target_of_22_burns_until_the_room_reaches_it_and_only_then_goes_down():
    sc, frames = run("light_to_target")
    light = sc["press"][1][0]
    assert all(frames[t]["automaton/stage"] == "stop" for t in range(2, light)), "the scaffolding: the burner stands before the Light\n" + table(frames, 0, light + 3)
    working = first(frames, "working", light)
    assert working is not None, "the Light never brought the burner to work:\n" + table(frames, light, 200, 5)
    # THE SECOND OF 29.10: the house says warm, and the burner used to leave work for smouldering within a second
    smouldering = first(frames, "smouldering", working)
    assert smouldering is not None, "the burner never went down:\n" + table(frames, working, 520, 10)
    reached = next(t for t in range(working, len(frames)) if (room(frames, t) or 0) >= 22.0)
    assert smouldering > working + 60, "the burner went down %d s after it came to work, with the room at %s:\n%s" % (smouldering - working, room(frames, smouldering), table(frames, working - 2, working + 8))
    assert reached - 3 <= smouldering <= reached + 30, "the burner went down at %d, the room reached 22 at %d:\n%s" % (smouldering, reached, table(frames, reached - 5, reached + 35, 3))
    # while it burned the card said what for
    mid = (working + reached) // 2
    assert frames[mid]["thermostat/manual"] == "room", table(frames, mid - 2, mid + 2)


def test_a_light_over_a_room_at_its_target_lights_nothing():
    sc, frames = run("light_refused")
    assert all(frame["automaton/stage"] == "stop" for frame in frames), "the burner left Stop on a refused Light:\n" + table(frames, 30, 80, 3)
    assert all(frame.get("thermostat/manual") in (None, "none") or isinstance(frame.get("thermostat/manual"), dict) for frame in frames)


def test_the_water_over_its_setpoint_ends_a_cycle_of_the_room_and_the_fire_goes_down_with_the_room_still_cold():
    sc, frames = run("light_water_ceiling")
    working = first(frames, "working", sc["press"][1][0])
    assert working is not None, table(frames, 40, 300, 10)
    at_ceiling = next(t for t in range(working, len(frames)) if (frames[t].get("temperatures/ntc_1") or 0) >= 60.0)
    # before the ceiling the cycle burns, with the room cold and the house saying warm
    before = at_ceiling - 20
    assert frames[before]["thermostat/manual"] == "room" and frames[before]["automaton/stage"] == "working", table(frames, before - 2, before + 2)
    # after it the cycle is gone and the burner goes down by the ordinary way, the room unchanged
    smouldering = first(frames, "smouldering", at_ceiling)
    assert smouldering is not None and smouldering <= at_ceiling + 30, "the burner did not go down with the water over its setpoint:\n" + table(frames, at_ceiling - 3, at_ceiling + 60, 4)
    assert frames[at_ceiling + 3]["thermostat/manual"] in (None, "none") or isinstance(frames[at_ceiling + 3]["thermostat/manual"], dict), table(frames, at_ceiling - 1, at_ceiling + 6)
    assert room(frames, smouldering) < 22.0
