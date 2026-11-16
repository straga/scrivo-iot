"""The verdicts of tools/page_thermostat.py, held on what a browser could answer - answers that are good and answers that are not, each of the kind the owner saw on the boiler on 04.10. What is held is the places and the words of what a
page drew: the numbers of the thermostat stand on the drawing by the things they are about, and no text stands under it. The browser is the tool's run."""
import copy
import json

import page_thermostat as t
from page_sources import merged_menu


def box(left=0, top=0, width=100, height=20):
    return {"left": left, "top": top, "right": left + width, "bottom": top + height, "width": width, "height": height}


SVG = box(14, 60, 347, 270)
# the boiler's number and the word that names its place, right-aligned at x 300; the setpoint stands over them
BOILER_NUMBER = {"text": "44.0 °C", "box": box(250, 90, 50, 14)}
BOILER_WORD = {"text": "boiler", "box": box(270, 106, 30, 10)}
SET = {"kind": "badge", "text": "boiler 44 °C −4", "box": box(235, 74, 65, 10)}
STEP = {"kind": "step", "text": "6 kW → 36 °C", "box": box(150, 250, 60, 11)}
NOTE = {"kind": "note", "text": "eco until 06:00", "box": box(100, 82, 55, 9)}
OTHER = {"text": "hopper", "box": box(30, 70, 30, 10)}


def said_of(**changes):
    thermo = {
        "svg": SVG,
        "title": {"text": "Scheme", "box": box(14, 20, 60, 22)},
        "mode": None,
        "card": box(8, 20, 359, 330),
        "blocks": 0,
        "under": [],
        "drawn": [SET],
        "texts": [OTHER, BOILER_NUMBER, BOILER_WORD, SET],
        "svgTexts": ["hopper", "44.0 °C", "boiler", "set 44 °C −4"],
        "cardText": "Scheme\nAuto Semi\nhopper 44.0 °C boiler set 44 °C −4\nStart Stop",
        "buttons": [{"text": "Start", "box": box(14, 340, 80, 44)}, {"text": "Stop", "box": box(100, 340, 80, 44)}],
        "wide": 375,
    }
    thermo.update(changes)
    return {"thermo": thermo}


EXPECT = {"step": None}


# ---------------------------------------------------------------- nothing stands under the drawing

def test_a_card_with_nothing_between_the_drawing_and_its_buttons_is_in_order():
    assert t.judge_nothing_under(said_of())[0]


def test_a_block_of_the_thermostat_still_in_the_card_is_named():
    ok, words = t.judge_nothing_under(said_of(blocks=1))
    assert not ok and "a block of the thermostat" in words


def test_a_line_of_text_under_the_drawing_is_named_with_what_it_says():
    # 04.10: "off · boiler 44 °C, hysteresis 4 (40…44)" and "burn on 6 kW · comfort, main profile" under the drawing, saying two things at once
    lines = [{"tag": "P", "text": "off · boiler 44 °C, hysteresis 4", "box": box(14, 335)}, {"tag": "P", "text": "burn on 6 kW", "box": box(14, 355)}]
    ok, words = t.judge_nothing_under(said_of(under=lines))
    assert not ok and "'off · boiler 44 °C, hysteresis 4' stands under the drawing" in words and "'burn on 6 kW'" in words


# ---------------------------------------------------------------- the step at the pot

def test_a_step_where_the_fire_works_as_asked_is_in_order_and_where_it_does_not_there_is_none():
    assert t.judge_step(said_of(drawn=[SET, STEP]), {**EXPECT, "step": "6 kW → 36 °C"})[0]
    assert t.judge_step(said_of(), EXPECT)[0]


def test_a_step_where_the_fire_does_not_work_is_named():
    ok, words = t.judge_step(said_of(drawn=[SET, STEP]), EXPECT)
    assert not ok and "where the fire does not work" in words


def test_a_step_that_says_another_threshold_or_none_where_the_fire_works_is_named():
    expect = {**EXPECT, "step": "6 kW → 36 °C"}
    ok, words = t.judge_step(said_of(drawn=[SET, {**STEP, "text": "6 kW → 41 °C"}]), expect)
    assert not ok and "not '6 kW → 36 °C'" in words
    assert not t.judge_step(said_of(), expect)[0]


def test_the_threshold_of_each_step_is_the_setpoint_less_the_distance_the_next_different_step_begins_at():
    steps, far = [6, 5, 3], [8, 3]
    assert t.stair_step("big", 44, steps, far) == {"kw": 6, "next": 36}
    assert t.stair_step("mid", 44, steps, far) == {"kw": 5, "next": 41}
    assert t.stair_step("small", 44, steps, far) == {"kw": 3, "next": None}
    assert t.stair_step("big", 44, [6, 6, 3], far)["next"] == 41, "a middle step that is the big one is passed over"
    assert t.stair_step("mid", 44, [6, 5, 5], far)["next"] is None


def test_the_step_is_said_in_the_menu_s_words_and_the_last_step_says_itself_alone():
    menu = merged_menu("burner")
    assert t.step_words(menu, "en", {"kw": 6, "next": 36}, "°C") == "6 kW → 36 °C"
    assert t.step_words(menu, "ru", {"kw": 3, "next": None}, "°C") == "3 кВт"


# ---------------------------------------------------------------- the notes at the boiler

def test_no_note_where_none_is_asked_is_in_order_and_one_that_stands_uninvited_is_named():
    assert t.judge_notes(said_of(), {"notes": [], "wait": None})[0]
    ok, words = t.judge_notes(said_of(drawn=[SET, NOTE]), {"notes": [], "wait": None})
    assert not ok and "1 notes" in words and "0 were asked" in words


def test_a_window_said_as_asked_is_in_order_and_a_wrong_one_or_a_missing_one_is_named():
    assert t.judge_notes(said_of(drawn=[SET, NOTE]), {"notes": ["eco until 06:00"], "wait": None})[0]
    assert not t.judge_notes(said_of(drawn=[SET, NOTE]), {"notes": ["eco until 07:00"], "wait": None})[0]
    assert not t.judge_notes(said_of(), {"notes": ["eco until 06:00"], "wait": None})[0]


def test_the_wait_is_read_within_the_drift_under_the_frame_s_and_not_over_it_or_far_under_it():
    wait = {"kind": "note", "text": "⏱ 1:10", "box": box(100, 82, 40, 9)}
    assert t.judge_notes(said_of(drawn=[SET, wait]), {"notes": [], "wait": 83})[0]
    assert not t.judge_notes(said_of(drawn=[SET, wait]), {"notes": [], "wait": 60})[0], "a wait longer than the frame's has not counted down"
    assert not t.judge_notes(said_of(drawn=[SET, wait]), {"notes": [], "wait": 300})[0], "a wait far under what the frame says"
    assert t.clock_seconds("⏱ 1:23:10") == 4990 and t.clock_seconds("no clock") is None


# ---------------------------------------------------------------- nothing overlaps

def test_numbers_that_stand_apart_from_every_text_of_the_drawing_are_in_order():
    assert t.judge_no_overlap(said_of(drawn=[SET, STEP, NOTE], texts=[OTHER, BOILER_NUMBER, BOILER_WORD, SET, STEP, NOTE]))[0]


def test_a_number_standing_over_another_text_of_the_drawing_is_named_with_both():
    over = {**NOTE, "box": box(40, 72, 55, 9)}
    ok, words = t.judge_no_overlap(said_of(drawn=[SET, over], texts=[OTHER, BOILER_NUMBER, BOILER_WORD, SET, over]))
    assert not ok and "'eco until 06:00' stands over 'hopper'" in words


def test_a_number_outside_the_drawing_is_named():
    out = {**STEP, "box": box(300, 250, 90, 11)}
    ok, words = t.judge_no_overlap(said_of(drawn=[SET, out], texts=[OTHER, BOILER_NUMBER, BOILER_WORD, SET, out]))
    assert not ok and "outside the drawing" in words


def test_boxes_that_touch_by_a_pixel_do_not_overlap():
    touching = {**NOTE, "box": box(15, 70, 16, 9)}
    assert t.judge_no_overlap(said_of(drawn=[SET, touching], texts=[{**OTHER, "box": box(30, 70, 30, 10)}, BOILER_NUMBER, BOILER_WORD, SET, touching]))[0]


# ---------------------------------------------------------------- the old words

def test_a_card_that_says_none_of_the_old_words_is_in_order_and_one_that_says_one_is_named_in_either_language():
    assert t.judge_no_old_words(said_of(), "en")[0] and t.judge_no_old_words(said_of(), "ru")[0]
    ok, words = t.judge_no_old_words(said_of(cardText="выключен · котёл 44 °C, гистерезис 4\nгореть на 6 кВт"), "ru")
    assert not ok and "выключен" in words and "гистерезис" in words and "гореть на" in words
    assert not t.judge_no_old_words(said_of(cardText="off · boiler 44 °C, hysteresis 4"), "en")[0]


# ---------------------------------------------------------------- the title's row, the phone and the height of the card

def test_a_scheme_with_no_mode_switch_and_none_of_its_words_in_the_drawing_is_in_order():
    assert t.judge_no_mode_on_scheme(said_of())[0]
    assert t.judge_no_mode_on_scheme(said_of(svgTexts=["hopper", "Pause"]))[0]


def test_a_mode_switch_still_in_the_title_row_is_named():
    ok, words = t.judge_no_mode_on_scheme(said_of(mode={"text": "AutoSemi", "box": box(270, 12, 90, 44), "disabled": False}))
    assert not ok and "still has a switch" in words


def test_a_mode_word_still_in_the_drawing_is_named_in_either_language():
    for word in ("Auto", "Semi", "Авто", "Полу"):
        ok, words = t.judge_no_mode_on_scheme(said_of(svgTexts=["hopper", word]))
        assert not ok and "still says" in words, word


def test_a_page_wider_than_the_phone_is_named():
    assert t.judge_fits(said_of())[0]
    assert not t.judge_fits(said_of(wide=391))[0]


def test_a_card_that_stands_as_tall_in_every_state_is_in_order():
    assert t.judge_card_height({"stop": 330, "purge": 330, "working": 330.4, "smouldering": 330})[0]


def test_a_card_that_is_a_line_taller_in_one_state_is_named_with_the_states_that_part():
    # 04.10, Russian: the card stood 21 px higher in the stop than in work, and no verdict of a single state saw it
    ok, words = t.judge_card_height({"stop": 351, "purge": 330, "working": 330})
    assert not ok and "stop" in words and "21" in words
    assert not t.judge_card_height({"stop": 330, "working": 309})[0]


def test_a_card_nobody_measured_is_named():
    assert not t.judge_card_height({})[0]
    assert not t.judge_card_height({"stop": None, "working": 330})[0]


# ---------------------------------------------------------------- what a state must say, from its frame and the menu

def test_a_state_where_the_fire_works_must_say_its_step_and_the_threshold_and_the_others_must_say_none():
    menu = merged_menu("burner")
    defaults = {"auto_hold_c": "60", "auto_drop_c": "4", "auto_step": "6", "auto_step_mid": "5", "auto_step_small": "3", "auto_mid_c": "8", "auto_small_c": "3"}
    working = {("thermostat", "setpoint"): {"number": {"value": 60.0}}, ("automaton", "doing"): {"word": "working"}, ("thermostat", "rung"): {"word": "mid"}}
    assert t.expectations(working, menu, "en", defaults)["step"] == "5 kW → 57 °C"
    for stage in ("stop", "purge", "kindling", "rising", "smouldering", "burning_off", "fault"):
        assert t.expectations({**working, ("automaton", "doing"): {"word": stage}}, menu, "en", defaults)["step"] is None, stage


def test_a_window_is_said_as_the_profile_and_the_minute_it_ends_at_and_the_main_profile_says_nothing():
    menu = merged_menu("burner")
    dictionary = {"things": [{"card": "demand", "name": "source"}, {"card": "demand", "name": "until", "part_of": "source"}]}
    frame = {"values": [[1, 360.0]]}
    rows = {("demand", "source"): {"word": "window"}, ("demand", "profile"): {"word": "eco"}}
    assert t.window_note(rows, menu, "en", dictionary, frame) == "eco until 06:00"
    assert t.window_note(rows, menu, "ru", dictionary, frame) == "эконом до 06:00"
    assert t.window_note({**rows, ("demand", "source"): {"word": "main"}}, menu, "en", dictionary, frame) is None


# ---------------------------------------------------------------- what the house asks, and why

ASK_LINE = {"word": "cold", "text": "asks for heat \u00b7 room under 21 \u00b0C", "box": box(14, 46, 160, 14)}


def test_the_line_stands_where_the_switch_is_off_or_unpressed_or_the_house_asks_heat_or_none():
    assert t.asks({("thermostat", "switch"): {"word": "off"}})
    assert t.asks({("thermostat", "switch"): {"word": "unasked"}})
    assert t.asks({("thermostat", "switch"): {"word": "on"}, ("demand", "asked"): {"word": "cold"}})
    assert t.asks({("demand", "asked"): {"word": "warm"}}), "a board of an older image has no switch"


def test_where_nobody_asks_of_a_switched_on_burner_the_page_must_say_no_line_and_a_word_nobody_knows_is_no_ask():
    assert not t.asks({("thermostat", "switch"): {"word": "on"}, ("demand", "asked"): {"unknown": "none"}})
    assert not t.asks({})
    assert not t.asks({("demand", "asked"): {"word": "lukewarm"}})


def test_a_card_that_says_the_line_asked_over_the_drawing_and_under_the_title_is_in_order():
    ok, words = t.judge_ask(said_of(ask=ASK_LINE), {"ask": True})
    assert ok, words


def test_a_line_that_is_missing_or_empty_or_stands_in_the_wrong_place_is_named():
    ok, words = t.judge_ask(said_of(ask=None), {"ask": True})
    assert not ok and "says nothing of why" in words
    ok, words = t.judge_ask(said_of(ask={**ASK_LINE, "text": " "}), {"ask": True})
    assert not ok and "says nothing of why" in words
    ok, words = t.judge_ask(said_of(ask={**ASK_LINE, "box": box(14, 10, 160, 14)}), {"ask": True})
    assert not ok and "not under the title" in words
    ok, words = t.judge_ask(said_of(ask={**ASK_LINE, "box": box(14, 200, 160, 14)}), {"ask": True})
    assert not ok and "not over the drawing" in words


def test_a_line_where_nobody_asks_is_named_and_none_is_in_order():
    assert t.judge_ask(said_of(), {"ask": False})[0]
    ok, words = t.judge_ask(said_of(ask=ASK_LINE), {"ask": False})
    assert not ok and "where there is nothing to say" in words
