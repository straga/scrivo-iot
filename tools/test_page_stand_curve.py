"""What the stand answers at /curve/<name> is what the board answers (crates/page, `Curve`): `step_ms` and `lines`, each line its unit and
points. A module the menu places (the flame, ; the water's temperatures and the flue, ; the draught) says neither words nor span:
what a line is called and what it is drawn against are the menu's.

    uv run --no-project --with pytest pytest tools

WHY THIS IS A TEST AND NOT A COMMENT. The stand answered with the form `raw`/`value` the board stopped speaking, and the page, which reads
`lines`, drew no curve on it at all: a picture nobody could see was judged by nobody. A stand that does not speak the board's form is a
page judged against something no board says.
"""
from page_stand import curve


class Board:
    turn = 0


def test_a_curve_of_the_flame_is_lines_of_unit_and_points_with_no_words_and_no_span():
    drawn = curve(Board(), "flame_fast")
    assert set(drawn) == {"step_ms", "lines"}, "the answer keeps the old form: %s" % sorted(drawn)
    assert drawn["step_ms"] == 100
    (line,) = drawn["lines"]
    assert set(line) == {"unit", "points"}, "the flame's line says what the menu says: %s" % sorted(line)
    assert line["unit"] == "mV"
    assert len(line["points"]) == 600 and all(isinstance(p, (int, float)) for p in line["points"])
    assert curve(Board(), "flame_slow")["step_ms"] == 1000


def test_a_curve_of_the_water_and_of_the_flue_is_lines_of_unit_and_points_with_no_words_and_no_span():
    for name, step_ms, count in [("water_1", 2000, 450), ("water_7", 2000, 450), ("flue", 1000, 900)]:
        drawn = curve(Board(), name)
        assert set(drawn) == {"step_ms", "lines"}, "%s keeps the old form: %s" % (name, sorted(drawn))
        assert drawn["step_ms"] == step_ms
        (line,) = drawn["lines"]
        assert set(line) == {"unit", "points"}, "%s says what the menu says: %s" % (name, sorted(line))
        assert line["unit"] == "\u00b0C"
        assert len(line["points"]) == count and all(isinstance(p, (int, float)) for p in line["points"])


def test_a_curve_of_the_draught_is_two_lines_of_unit_and_points_with_no_words_and_no_span():
    for name, step_ms in [("draught_fast", 100), ("draught_slow", 1000)]:
        drawn = curve(Board(), name)
        assert drawn["step_ms"] == step_ms
        pascals, millivolts = drawn["lines"]
        assert (set(pascals), set(millivolts)) == ({"unit", "points"}, {"unit", "points"}), "%s says what the menu says: %s" % (name, drawn["lines"])
        assert (pascals["unit"], millivolts["unit"]) == ("Pa", "mV")
        assert len(pascals["points"]) == len(millivolts["points"]) == (600 if name == "draught_fast" else 900)


def test_the_curves_of_the_fan_are_three_lines_of_unit_and_points_as_the_board_sends_them():
    # asked and came out in pascals, the angle in percent; the regulator's miss in pascals, P and I in percent a second (fan.rs, `regulator_curve`)
    for name, units in [("fan_holding", ("Pa", "Pa", "%")), ("fan_response", ("Pa", "Pa", "%")), ("fan_regulator", ("Pa", "% a second", "% a second"))]:
        drawn = curve(Board(), name)
        assert drawn is not None, "the stand has no curve %s, and the page's tab of the fan draws nothing" % name
        assert set(drawn) == {"step_ms", "lines"}
        assert tuple(line["unit"] for line in drawn["lines"]) == units
        assert all(set(line) == {"unit", "points"} for line in drawn["lines"]), "the words and spans are the menu's: %s" % drawn["lines"]
        assert len({tuple(line["points"]) for line in drawn["lines"]}) == 3, "two lines of %s are the same numbers, and the picture could not tell them apart" % name


def test_a_curve_the_board_has_not_is_none():
    assert curve(Board(), "no_such_curve") is None
