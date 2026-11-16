"""The silence of the sensor the thermostat holds by is on its card (moved to the thermostat's card): the return says how many seconds it has been silent - and says
nothing while it speaks, and nothing at all when no role is given to it. The air - the room - is the demand's: its silence is on the card of the demand, and what the burner does about it is in tools/test_house_asks_on_the_stand.py. Played on the new burner's own automaton (burner_api, example `burner_frames`) on runs of the readings by the second. Expected from what a
person must be told at the boiler - which sensor the burner is waiting on, and for how long - and not from how the code reads the sensors.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_silent_sensors_on_the_stand.py
"""
import copy
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

RETURN_SCENARIO = os.path.join(HERE, "stand_scenarios", "silent_return.json")

# (the sensor, the line of the scenario, the socket that plays its role, what the burner holds by)
SENSORS = [("return", "return_c", "role_ntc_3", "return")]


def scenario_of(sensor, line, socket, hold, silent_from=301, assigned=True, speaks=False):
    """The committed run with the sensor's line made what a case needs: silent from SILENT_FROM, speaking all through, or - ASSIGNED False - no line and no role."""
    sc = copy.deepcopy(json.load(open(RETURN_SCENARIO)))
    sc.pop("return_c")
    sc["press"] = [[2, "start"], [3, "light"]]
    sc["settings"] = {"auto_hold": hold, "auto_hold_c": "40", "role_ntc_1": "boiler"}
    if assigned:
        sc["settings"][socket] = sensor
        sc[line] = [[0, 30], [500, 30]] if speaks else [[0, 30], [silent_from - 1, 30], [silent_from, None], [500, None]]
    else:
        sc["settings"]["auto_hold"] = "boiler"
    return sc


def played(scenario):
    """The seconds of the run: what the card said of the two silences, the stage and the cause, second by second."""
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "scenario.json")
        with open(path, "w") as f:
            json.dump(scenario, f)
        frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
    entities = ["thermostat/silent_return", "demand/silent_room", "automaton/doing", "automaton/doing.why"]
    return named(dictionary_of(ROOT, "burner"), frames, entities)


@pytest.mark.parametrize("sensor,line,socket,hold", SENSORS)
def test_a_sensor_that_went_silent_while_the_burner_holds_by_it_says_the_seconds_and_the_burner_names_it(sensor, line, socket, hold):
    run = played(scenario_of(sensor, line, socket, hold))
    row = "thermostat/silent_" + sensor
    before = [s.get(row) for s in run[:301]]
    assert before == [None] * 301, "a sensor that spoke said it was silent: %s" % [x for x in before if x is not None][:3]
    after = [s.get(row) for s in run[301:]]
    said = [(at, n) for at, n in enumerate(after, 301) if isinstance(n, int)]
    assert said, "the card never said how long the %s had been silent" % sensor
    # it counts up by the second from the first second it is said
    for (a, x), (b, y) in zip(said, said[1:]):
        assert (b - a, y - x) == (1, 1), "the silence of the %s did not count up by the second at %d: %s then %s" % (sensor, a, x, y)
    # and the burner names it: the second it burns off the fire for it, the card says a silence long enough to stop a burner (30 s and more)
    named_at = [at for at, s in enumerate(run) if s.get("automaton/doing.why") == sensor + "_silent"]
    assert named_at, "the burner never named the %s" % sensor
    assert isinstance(run[named_at[0]].get(row), int) and run[named_at[0]][row] >= 30, "when it burned off for the %s the card said %s" % (sensor, run[named_at[0]].get(row))


@pytest.mark.parametrize("sensor,line,socket,hold", SENSORS)
def test_a_sensor_that_speaks_says_no_silence_all_through_the_run(sensor, line, socket, hold):
    run = played(scenario_of(sensor, line, socket, hold, speaks=True))
    said = [s.get("thermostat/silent_" + sensor) for s in run if s.get("thermostat/silent_" + sensor) is not None]
    assert said == [], "a sensor that speaks said it was silent: %s" % said[:3]
    assert all(s.get("automaton/doing.why") != sensor + "_silent" for s in run)


@pytest.mark.parametrize("sensor,line,socket,hold", SENSORS)
def test_a_sensor_no_role_is_given_to_says_no_silence_ever(sensor, line, socket, hold):
    run = played(scenario_of(sensor, line, socket, hold, assigned=False))
    for name in ("thermostat/silent_return", "demand/silent_room"):
        said = [s.get(name) for s in run if s.get(name) is not None]
        assert said == [], "%s was said with no role given to a sensor: %s" % (name, said[:3])


def test_the_committed_run_is_the_one_of_a_return_that_goes_silent_in_the_hold_by_the_return():
    sc = json.load(open(RETURN_SCENARIO))
    assert sc["settings"]["auto_hold"] == "return" and sc["settings"]["role_ntc_3"] == "return"
    assert sc["return_c"][-1][1] is None and sc["return_c"][-2] == [301, None]


def test_a_return_silent_from_the_start_keeps_the_burner_from_lighting_and_the_card_says_for_how_long():
    sc = scenario_of("return", "return_c", "role_ntc_3", "return", silent_from=1)
    run = played(sc)
    assert all(s.get("automaton/doing") not in ("kindling", "rising", "working") for s in run), "the burner lit with the return silent"
    named_at = [at for at, s in enumerate(run) if s.get("automaton/doing.why") == "return_silent"]
    assert named_at, "the burner did not say why it does not light"
    for at in named_at:
        assert isinstance(run[at].get("thermostat/silent_return"), int), "second %d: the burner names the return and the card says no silence" % at


@pytest.mark.parametrize("sensor,line,socket,hold", SENSORS)
def test_a_line_the_scenario_has_for_a_sensor_no_role_is_given_to_says_no_silence_either(sensor, line, socket, hold):
    # the board decides a role is played by the sockets set to it, and the stand decided by whether the scenario had a line - a line with no role given to it was said as silent
    sc = scenario_of(sensor, line, socket, hold)
    sc["settings"].pop(socket)
    sc["settings"]["auto_hold"] = "boiler"
    assert sc[line][-1][1] is None, "the line of the case does not go silent"
    run = played(sc)
    for name in ("thermostat/silent_return", "demand/silent_room"):
        said = [s.get(name) for s in run if s.get(name) is not None]
        assert said == [], "%s was said for a sensor no socket is set to: %s" % (name, said[:3])


@pytest.mark.parametrize("sensor,line,socket,hold", SENSORS)
def test_a_role_given_to_a_socket_by_the_scenario_is_a_role_played_whatever_the_line_is_called(sensor, line, socket, hold):
    # the other side of the same rule: the role is given by the setting of the socket, so a different socket plays it as well
    sc = scenario_of(sensor, line, socket, hold)
    other = "role_ntc_6"
    sc["settings"].pop(socket)
    sc["settings"][other] = sensor
    run = played(sc)
    said = [s.get("thermostat/silent_" + sensor) for s in run if isinstance(s.get("thermostat/silent_" + sensor), int)]
    assert said, "the role was played by %s and the card said no silence" % other


def test_the_automatons_card_has_no_row_for_the_silence_of_the_return_or_the_air_and_each_stands_on_the_card_of_whoever_decides_by_it():
    # the silence stands on the card of whoever decides by the sensor, once. A client that reads it from the automaton's card finds none, and the dictionary says so.
    # The return is the thermostat's; the room is the demand's
    things = dictionary_of(ROOT, "burner")["things"]
    on = {(t["card"], t["name"]) for t in things}
    for card, name in (("thermostat", "silent_return"), ("demand", "silent_room")):
        assert (card, name) in on, "the %s card has no %s" % (card, name)
        assert ("automaton", name) not in on, "the automaton's card still says %s" % name
    assert ("thermostat", "silent_room") not in on, "the thermostat's card still says the silence of the room"
