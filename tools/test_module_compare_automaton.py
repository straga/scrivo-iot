"""The new burner's automaton against the first burner's, second by second, on the scenarios of the stand: what each side says of the automaton - the stage, what it does, the clocks, the organs, the pump,
the fan it asks, the alarm - and of the relays it commands, every second of every scenario. Expected from the first burner's own frames, made by its own code (crates/automaton_card, example `scenario`): the new
burner is held to what the first one does, and a second where they differ is named with what each said.

The rows run both sides for real - two cargo runs a scenario - so they take minutes; `tools/module_compare.py SCENARIO --modules automaton` is the same run by hand."""
import os

import pytest

from module_compare import HERE, entities_of, main

SCENARIOS = [
    # a start, the lighting, the work; the stop and the start again; the water past what it may be and the alarm
    "lighting",
    "steady",
    # the feed banned in the middle of a dose
    "feed_banned",
    # what the burner holds is the return, off a record of the boiler
    "hold_by_return",
    "hold_by_return_socket_let_go",
    # the fire that blinks in rising, that goes out in rising, that blinks and that is gone at work
    "rising_blinks",
    "rising_goes_out",
    "work_flame_blinks",
    "work_flame_gone",
    # no fuel arriving: a flue that does not climb, a flue that climbs slowly
    "work_flue_flat",
    "work_flue_slow",
    # a stop pressed while the pot is dark
    "work_stop_while_dark",
]


def test_the_automaton_is_held_to_every_row_the_first_burners_stand_says_of_it_and_to_the_organs_it_commands():
    got = entities_of({}, ["automaton"])
    for name in [
        "automaton/stage",
        "automaton/doing",
        "automaton/doing.attempt",
        "automaton/doing.why",
        "automaton/doing.tries",
        "automaton/doing.water",
        "automaton/broke_at",
        "automaton/for",
        "automaton/lighting_in",
        "automaton/blowing_clear_ends_in",
        "automaton/auger",
        "automaton/feed",
        "automaton/semi",
        "automaton/igniter",
        "automaton/pump",
        "automaton/pump_by",
        "automaton/pump_by.water",
        "automaton/fan",
        "automaton/alarm",
        "automaton/alarm.past",
        "automaton/organs",
        "relays/auger",
        "relays/igniter",
        "relays/pump",
    ]:
        assert name in got, f"{name} is not compared"


def test_what_the_first_burners_stand_never_says_is_not_held_against_the_new_one():
    # the silence of the sensors is the board's alone: the first burner's stand does not make it up, and its card says nothing of it
    got = entities_of({}, ["automaton"])
    for name in ["automaton/silent_draught", "automaton/silent_flame", "automaton/silent_flue", "automaton/silent_water"]:
        assert name not in got, name


@pytest.mark.parametrize("scenario", SCENARIOS)
def test_the_new_automaton_says_second_by_second_what_the_first_one_says(scenario, capsys):
    code = main([os.path.join(HERE, "stand_scenarios", scenario + ".json"), "--modules", "automaton"])
    out = capsys.readouterr().out
    assert code == 0, out
    assert out.strip().endswith(": 0 differences"), out
