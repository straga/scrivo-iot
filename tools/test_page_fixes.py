"""The verdicts of tools/page_fixes.py, held on what a browser could answer - answers that are good and answers that are not, each of the kind the owner saw on the boiler on 02.10. What is held is the
numbers and the order of what a page drew; the browser itself is the tool's run, not this file's."""
import copy

import page_fixes as f


def box(left=0, top=0, width=44, height=44):
    return {"left": left, "top": top, "right": left + width, "bottom": top + height, "width": width, "height": height}


def kid(text, left, top=10, width=44, height=44, tag="BUTTON"):
    return {"tag": tag, "text": text, "box": box(left, top, width, height)}


def header(period="5s", pressed=None, bg="rgb(230, 225, 210)", top=10):
    kids = [kid("burner", 0, top, 120, 44, "H1"), kid("stragabases3-26.10.02.1", 126, top + 8, 130, 28, "SPAN"), kid(period, 262, top), kid("EN", 308, top), kid("◐", 354, top)]
    buttons = [{"text": period, "title": "Live", "box": kids[2]["box"], "bg": bg, "pressed": pressed}, {"text": "EN", "title": "", "box": kids[3]["box"], "bg": "rgb(1, 1, 1)", "pressed": None},
               {"text": "◐", "title": "", "box": kids[4]["box"], "bg": "rgb(1, 1, 1)", "pressed": None}]
    return {"kids": kids, "buttons": buttons, "bad": "#b3261e"}


def page(**more):
    base = {**header(pressed="false"), "doing": None,
            "cells": [{"text": "stop", "clipped": False, "right": 100}], "stripRight": 330, "alone": []}
    base.update(more)
    return base


# ---- the header: one line, three squares ----

def test_a_header_of_one_line_with_three_squares_passes():
    assert f.judge_header_one_row(page())[0]


def test_a_header_that_broke_onto_a_second_line_fails_and_says_where_the_tops_are():
    # the owner's: the button of Live on a line of its own under the language and the theme
    said = page()
    said["kids"][2]["box"] = box(262, 70)
    ok, words = f.judge_header_one_row(said)
    assert not ok and "2 lines" in words, words


def test_a_button_that_is_not_square_fails():
    said = page()
    said["buttons"][1]["box"] = box(308, 10, 60, 44)
    ok, words = f.judge_header_one_row(said)
    assert not ok and "EN" in words, words


def test_a_header_without_the_three_buttons_fails():
    said = page()
    said["buttons"] = said["buttons"][1:]
    assert not f.judge_header_one_row(said)[0]


# ---- the button of Live: the period, red when pressed, no count ----

def steps(idle="5s", pressed_text="Live", pressed_bg="rgb(179, 38, 30)", back="5s"):
    first = page()
    pressed = {**header(pressed_text, "true", pressed_bg), "h2": [], "pairs": []}
    again = {**header(back, "false"), "h2": [], "pairs": []}
    first.update(header(idle, "false"))
    return {"first": first, "pressed": pressed, "back": again}


def test_a_button_that_says_the_period_goes_red_without_a_count_and_comes_back_passes():
    assert f.judge_live_period(steps(), 5, "s")[0]


def test_a_button_still_counting_seconds_when_pressed_fails():
    # the old button: "Live 9:58"
    ok, words = f.judge_live_period(steps(pressed_text="Live 9:58"), 5, "s")
    assert not ok and "count of seconds" in words, words


def test_a_button_that_does_not_turn_red_when_pressed_fails():
    ok, words = f.judge_live_period(steps(pressed_bg="rgb(230, 225, 210)"), 5, "s")
    assert not ok and "did not change" in words, words


def test_a_button_pressed_that_turns_some_other_colour_than_the_pages_red_fails():
    ok, words = f.judge_live_period(steps(pressed_bg="rgb(0, 90, 200)"), 5, "s")
    assert not ok and "red of the page" in words, words


def test_a_button_that_does_not_say_the_boards_period_fails():
    ok, words = f.judge_live_period(steps(idle="Live"), 5, "s")
    assert not ok and "period" in words, words


def test_a_button_that_does_not_come_back_to_the_period_fails():
    ok, words = f.judge_live_period(steps(back="Live"), 5, "s")
    assert not ok and "pressed again" in words, words


def line(text):
    return {"text": text, "box": box(0, 600, 315, 20)}


# ---- smouldering in numbers ----

def test_smouldering_says_the_time_gone_the_whole_the_feed_and_what_is_left_in_the_card():
    said = page(doing=line("тление · 3:39 из 15:00 · подача 1 с / 50 с · до гашения 11:21"))
    assert f.judge_smoulder_line(said, 219, 900, 1, 50)[0]


def test_the_time_the_page_counted_on_since_the_frame_is_not_a_fault_and_a_time_from_nowhere_is():
    said = page(doing=line("тление · 3:49 из 15:00 · подача 1 с / 50 с · до гашения 11:11"))
    assert f.judge_smoulder_line(said, 219, 900, 1, 50)[0]
    wrong = page(doing=line("тление · 9:00 из 15:00 · подача 1 с / 50 с · до гашения 6:00"))
    ok, words = f.judge_smoulder_line(wrong, 219, 900, 1, 50)
    assert not ok and "gone" in words, words


def test_smouldering_with_the_wrong_whole_the_wrong_feed_or_what_is_left_not_the_whole_less_the_gone_fails():
    ok, words = f.judge_smoulder_line(page(doing=line("тление · 3:39 из 5:00 · подача 1 с / 50 с · до гашения 1:21")), 219, 900, 1, 50)
    assert not ok and "15:00" in words, words
    ok, words = f.judge_smoulder_line(page(doing=line("тление · 3:39 из 15:00 · подача 2 с / 50 с · до гашения 11:21")), 219, 900, 1, 50)
    assert not ok
    ok, words = f.judge_smoulder_line(page(doing=line("тление · 3:39 из 15:00 · подача 1 с / 50 с · до гашения 11:00")), 219, 900, 1, 50)
    assert not ok and "leaves" in words, words


def test_smouldering_with_no_numbers_in_the_card_fails():
    ok, words = f.judge_smoulder_line(page(doing=line("тление")), 219, 900, 1, 50)
    assert not ok and "in the card" in words, words


def test_what_is_left_never_goes_below_nought():
    said = page(doing=line("тление · 16:40 из 15:00 · подача 1 с / 50 с · до гашения 0:00"))
    assert f.judge_smoulder_line(said, 1000, 900, 1, 50)[0]


# ---- nothing untranslated, nothing cut ----

def test_a_russian_page_with_the_english_the_board_leaves_fails_and_names_it():
    ok, words = f.judge_nothing_untranslated(page(alone=["next", "свеча", "off", "выкл"]), "ru")
    assert not ok and "next" in words and "off" in words and "свеча" not in words, words


def test_a_russian_page_with_none_of_it_passes_and_an_english_page_is_not_asked():
    assert f.judge_nothing_untranslated(page(alone=["дальше", "выкл", "вкл"]), "ru")[0]
    assert f.judge_nothing_untranslated(page(alone=["next", "off"]), "en")[0]


def test_a_word_of_the_strip_cut_by_its_cell_or_ended_with_an_ellipsis_fails():
    cut = page(cells=[{"text": "стоп", "clipped": False, "right": 90}, {"text": "разгоран…", "clipped": False, "right": 200}])
    ok, words = f.judge_strip_uncut(cut)
    assert not ok and "разгоран…" in words, words
    clipped = page(cells=[{"text": "разгорание", "clipped": True, "right": 200}])
    assert not f.judge_strip_uncut(clipped)[0]


def test_a_strip_whose_last_cell_ends_past_the_strip_fails_and_a_whole_one_passes():
    over = page(cells=[{"text": "догорание", "clipped": False, "right": 340}], stripRight=330)
    assert not f.judge_strip_uncut(over)[0]
    assert f.judge_strip_uncut(page(cells=[{"text": "догорание", "clipped": False, "right": 330}], stripRight=330))[0]


def test_the_samples_are_not_shared_between_tests():
    a, b = page(), copy.deepcopy(page())
    a["kids"][0]["text"] = "changed"
    assert b["kids"][0]["text"] == "burner"


# ---- what the moments are picked by (the buttons and the question "is heat asked for" are the thermostat's since 6.10)

def rows_of(switch=None, doing="stop"):
    rows = {("automaton", "doing"): {"word": doing}, ("automaton", "auger"): {"on": False}, ("automaton", "for"): {"seconds": 3}}
    if switch is not None:
        rows[("thermostat", "switch")] = {"word": switch}
    return rows


def test_heat_is_asked_for_when_the_thermostat_switch_says_on():
    assert f.picked_of(rows_of("on"))["wanted"] is True


def test_heat_is_not_asked_for_when_the_switch_says_off_or_unasked():
    assert f.picked_of(rows_of("off"))["wanted"] is False
    assert f.picked_of(rows_of("unasked"))["wanted"] is False


def test_a_frame_with_no_switch_row_says_nothing_of_heat_and_not_that_it_was_not_asked():
    assert f.picked_of(rows_of(None))["wanted"] is None


def test_a_stop_that_heat_is_asked_after_is_the_waiting_moment_and_a_stop_nobody_asked_after_is_not():
    waiting = dict(f.MOMENTS)["waiting"]
    assert waiting(f.picked_of(rows_of("on", doing="stop"))) is True
    assert waiting(f.picked_of(rows_of("unasked", doing="stop"))) is False
    assert waiting(f.picked_of(rows_of("on", doing="working"))) is False
