"""The verdicts of tools/page_grid.py, held on what a browser could answer - answers that are good and answers that are not, each of the kind the owner saw on the boiler on 04.10."""
import copy

import page_grid as g


def box(left=0, top=0, width=100, height=20):
    return {"left": left, "top": top, "right": left + width, "bottom": top + height, "width": width, "height": height}


def label(text, left=0, width=66, height=20, size=13, fits=True):
    return {"text": text, "box": box(left, 0, width, height), "size": size, "fits": fits}


def field(tag, name, left=99, width=40, top=6):
    return {"tag": tag, "name": name, "box": box(left, top, width, 44)}


def row(labels, fields, height=57, switch=False, over=False):
    return {"box": box(0, 0, 319, height), "switch": switch, "over": over, "labels": labels, "fields": fields}


GOOD = {"wide": 375, "rows": [
    row([label("Hold the burner by")], [], height=89, switch=True),
    row([label("Hold"), label("hysteresis")], [field("input", "auto_hold_c"), field("input", "auto_drop_c", left=210, width=32)]),
    row([label("Big")], [field("select", "auto_step", width=120)]),
    row([label("Middle"), label("nearer than")], [field("select", "auto_step_mid", width=120), field("input", "auto_mid_c", left=250, width=32)]),
    row([label("Not sooner"), label("but at once below")], [field("input", "auto_lock_min"), field("input", "auto_lock_c", left=240, width=32)]),
]}
EXPECT = {"rows": 5, "pairs": 3}


def bad(rows):
    said = copy.deepcopy(GOOD)
    said["rows"] = rows(said["rows"])
    return said


def test_a_card_with_its_rows_in_one_grid_is_in_order():
    for judge in (g.judge_one_column, g.judge_one_width_per_kind, g.judge_labels, g.judge_one_font, g.judge_row_height, g.judge_pairs_on_a_line, g.judge_rows_inside):
        assert judge(GOOD)[0], judge.__name__
    assert g.judge_rows(GOOD, EXPECT)[0]


def test_a_card_with_more_rows_than_the_menu_makes_is_named():
    ok, words = g.judge_rows(GOOD, {"rows": 8, "pairs": 3})
    assert not ok and "5 rows" in words and "8" in words
    ok, words = g.judge_rows(GOOD, {"rows": 5, "pairs": 4})
    assert not ok and "3 rows of two fields" in words


def test_first_fields_that_do_not_begin_at_one_left_edge_are_named_and_a_column_of_one_is_no_column():
    # 04.10: the choice of the step began at 267 px, the middle ones at 157, the numbers at 225 and 215
    wide = bad(lambda rows: rows[:2] + [row([label("Big")], [field("select", "auto_step", left=267, width=120)])] + rows[3:])
    ok, words = g.judge_one_column(wide)
    assert not ok and "auto_step 267" in words
    one = {"wide": 375, "rows": [GOOD["rows"][2]]}
    assert not g.judge_one_column(one)[0], "one field says nothing of a column"


def test_choices_or_numbers_of_different_widths_are_named():
    ok, words = g.judge_one_width_per_kind(bad(lambda rows: rows[:2] + [row([label("Big")], [field("select", "auto_step", width=81)])] + rows[3:]))
    assert not ok and "first choices" in words
    ok, words = g.judge_one_width_per_kind(bad(lambda rows: [rows[0], row([label("Hold")], [field("input", "auto_hold_c", width=96)])] + rows[2:]))
    assert not ok and "first numbers" in words
    ok, words = g.judge_one_width_per_kind(bad(lambda rows: rows[:4] + [row([label("Not sooner")], [field("input", "auto_lock_min"), field("input", "auto_lock_c", left=240, width=44)])]))
    assert not ok and "second fields" in words


def test_a_label_on_two_lines_or_out_of_its_place_is_named():
    # 04.10: "Level of power, middle" stood on three lines; and a label wider than its column runs under the field
    ok, words = g.judge_labels(bad(lambda rows: rows[:2] + [row([label("Level of power, middle", height=72)], [field("select", "auto_step", width=120)])] + rows[3:]))
    assert not ok and "72 px high" in words
    ok, words = g.judge_labels(bad(lambda rows: rows[:2] + [row([label("Hold until", fits=False)], [field("input", "auto_hold_c")])] + rows[3:]))
    assert not ok and "out of its place" in words


def test_labels_of_two_sizes_are_named():
    ok, words = g.judge_one_font(bad(lambda rows: rows[:2] + [row([label("Big", size=16)], [field("select", "auto_step", width=120)])] + rows[3:]))
    assert not ok and "13/16" in words


def test_a_row_taller_than_the_next_is_named_and_a_switch_is_not_counted():
    ok, words = g.judge_row_height(bad(lambda rows: rows[:2] + [row([label("Big")], [field("select", "auto_step", width=120)], height=89)] + rows[3:]))
    assert not ok and "89" in words
    assert g.judge_row_height(GOOD)[0], "the switch row is taller and is no row of the grid"


def test_a_pair_whose_fields_stand_on_two_lines_is_named():
    ok, words = g.judge_pairs_on_a_line(bad(lambda rows: rows[:1] + [row([label("Hold")], [field("input", "auto_hold_c"), field("input", "auto_drop_c", left=210, top=60)])] + rows[2:]))
    assert not ok and "fields at" in words


def test_a_row_that_runs_out_of_its_card_or_a_page_wider_than_the_phone_is_named():
    ok, words = g.judge_rows_inside(bad(lambda rows: rows[:1] + [row([label("Hold")], [field("input", "auto_hold_c")], over=True)] + rows[2:]))
    assert not ok and "runs out of the card" in words
    wide = copy.deepcopy(GOOD)
    wide["wide"] = 391
    assert not g.judge_rows_inside(wide)[0]
