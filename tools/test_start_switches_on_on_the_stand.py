"""A Start switches the burner on and the house lights it; a Light lights it past the house. Played on the stand by the new burner's own thermostat, demand and automaton (burner_api, example
`burner_frames`), with the room playing its band against the target of 21 degrees (warm at 21 and over, cold at 20 and under). Expected from what a person at the boiler finds: Start with a warm room
leaves the fire out and the switch on; the room falling to the lower line of its band lights the fire with nobody pressing anything; "light now" lights it with the room warm.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_start_switches_on_on_the_stand.py
"""
import copy
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from test_house_asks_on_the_stand import played, scenario, stages  # noqa: E402

WARM_C, COLD_C = 22.0, 19.0
SEMI_COLD_AT = 300


def run_with(room, press, seconds=420):
    sc = copy.deepcopy(scenario("house_by_the_room.json"))
    sc["seconds"] = seconds
    sc["press"] = press
    sc["room_c"] = room
    sc["boiler_c"] = [[0, 40], [seconds, 40]]
    return played(sc)


def test_a_start_with_the_room_warm_switches_on_and_lights_nothing_until_the_room_falls_cold():
    run = run_with([[0, WARM_C], [SEMI_COLD_AT - 1, WARM_C], [SEMI_COLD_AT, COLD_C], [420, COLD_C]], [[2, "start"]])
    assert run[10]["thermostat/switch"] == "on", "the Start did not switch the burner on"
    assert run[10]["demand/asked"] == "warm" and run[10]["thermostat/heat"] == "enough", "the scaffolding: the room is warm and the water asks for nothing"
    assert stages(run, 3, SEMI_COLD_AT - 2) == {"stop"}, "the Start lit a fire in a warm room: %s" % sorted(stages(run, 3, SEMI_COLD_AT - 2))
    assert run[SEMI_COLD_AT - 2]["thermostat/switch"] == "on"
    lit = next(t for t in range(SEMI_COLD_AT - 2, 420) if run[t]["automaton/stage"] == "purge")
    assert SEMI_COLD_AT <= lit <= SEMI_COLD_AT + 4, "the room fell cold at %d and the fire was lit at %d" % (SEMI_COLD_AT, lit)


def test_a_start_with_the_room_cold_lights_the_fire_by_the_water():
    run = run_with([[0, COLD_C], [420, COLD_C]], [[2, "start"]])
    assert run[10]["demand/asked"] == "cold" and run[10]["thermostat/heat"] == "burn", "the scaffolding: the room is cold and the water under the line"
    assert any(s["automaton/stage"] == "purge" for s in run[2:30]), "the switch is on, the house is cold and the fire was not lit"


def test_a_light_lights_the_fire_with_the_room_warm_and_a_start_alone_does_not():
    run = run_with([[0, WARM_C], [420, WARM_C]], [[2, "start"], [100, "light"]])
    assert stages(run, 3, 99) == {"stop"}, "the Start alone lit the fire"
    assert run[103]["automaton/stage"] == "purge", "the Light did not light the fire: %s" % run[103]["automaton/stage"]
    assert run[103]["demand/asked"] == "warm" and run[103]["thermostat/heat"] == "enough", "the room was not warm when the Light came"
