"""The stand takes the commands by address, as the board does: the one it asks, the state it
says, and in words what it cannot do.

    uv run --no-project --with pytest pytest -q tools/test_page_stand_commands.py
"""
from test_page_stand_stream import DICTIONARY
from page_stand import Board

WITH_A_SLIDER = dict(DICTIONARY, things=DICTIONARY["things"] + [
    {"card": "c", "name": "knob", "kinds": ["number"], "unit": "Pa", "can": ["hold"], "range": {"min": 0, "max": 300, "step": 1, "at": 6}},
    {"card": "c", "name": "slider", "part_of": "knob", "kinds": ["number"], "unit": "Pa"},
])


def board():
    return Board(WITH_A_SLIDER, {"values": [[0, False], [1, 21.5], [2, 1000], [5, 0], [6, 31]]}, None, 2)


def test_on_twice_is_on_and_off_twice_is_off():
    b = board()
    for what, want in (("on", True), ("on", True), ("off", False), ("off", False)):
        assert b.command(what, "0") == {"values": [[0, want]]}


def test_toggle_is_the_other_side_of_what_the_value_is():
    b = board()
    assert b.command("toggle", "0") == {"values": [[0, True]]}
    assert b.command("toggle", "0") == {"values": [[0, False]]}


def test_a_number_held_moves_the_thing_and_its_slider_and_one_outside_its_range_is_refused():
    b = board()
    assert b.command("hold", "5", "45") == {"values": [[5, 45], [6, 45]]}
    refused = b.command("hold", "5", "301")
    assert "301" in refused["error"] and b.values[5] == 45


def test_an_address_nobody_has_a_part_and_a_command_a_thing_does_not_take_are_refused_in_words():
    b = board()
    for address, what in (("42", "on"), ("6", "hold"), ("x", "on"), ("1", "on"), ("4", "on"), ("0", "press")):
        said = b.command(what, address, "1")
        assert "error" in said and said["error"], (address, what, said)
    assert b.values[:2] == [False, 21.5], "a refused command changed something"
