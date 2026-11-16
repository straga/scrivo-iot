"""The verdicts of tools/page_plum.py on what the browser said, made up here: a page with one button that runs the burner passes, a page with two, with a button where none belongs or with a mode switch on the scheme is named where.

    uv run --no-project --with pytest --with websocket-client pytest tools/test_page_plum.py
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from page_plum import judge_fill, judge_hands_button, judge_hands_given, judge_one_press, sends_of, judge_light_now, judge_one_run_button, judge_run_by_switch, judge_scheme_is_quiet, judge_service_has_hands, judge_start_refused

WORDS = {"start": "Go", "stop": "Halt", "burning_off": "Burning out"}


def tab(run=(), cannot=(), now=(), buttons=(), titles=(), mode=0, inputs=0, hands=(), hands_text=(), service=0):
    return {"run": list(run), "cannot": list(cannot), "now": list(now), "buttons": list(buttons), "titles": list(titles), "mode": mode, "inputs": inputs,
            "hands": list(hands), "handsText": list(hands_text), "service": service}


def run(press, disabled=False):
    return {"text": WORDS[press], "disabled": disabled, "press": press}


def page(scheme=None, modules=None, system=None):
    return {"scheme": scheme or tab(run=[run("stop")], buttons=["Halt"]), "modules": modules or tab(), "system": system or tab()}


def test_one_button_on_the_scheme_and_none_elsewhere_passes():
    assert judge_one_run_button(page(), WORDS)[0]


def test_two_buttons_on_the_scheme_fail():
    said = page(scheme=tab(run=[run("start"), run("stop")], buttons=["Go", "Halt"]))
    ok, why = judge_one_run_button(said, WORDS)
    assert not ok and "2 buttons" in why


def test_a_second_pair_of_buttons_on_a_modules_page_fails_and_names_the_tab():
    ok, why = judge_one_run_button(page(modules=tab(buttons=["Go", "Halt"])), WORDS)
    assert not ok and "modules" in why


def test_the_button_follows_the_switch_and_a_fault_is_always_stop():
    assert judge_run_by_switch(page(scheme=tab(run=[run("stop")])), WORDS, "on", "working")[0]
    assert judge_run_by_switch(page(scheme=tab(run=[run("start")])), WORDS, "off", "stop")[0]
    assert judge_run_by_switch(page(scheme=tab(run=[run("start")])), WORDS, "unasked", "stop")[0]
    assert judge_run_by_switch(page(scheme=tab(run=[run("stop")])), WORDS, "off", "fault")[0]


def test_the_wrong_word_for_the_switch_fails():
    ok, why = judge_run_by_switch(page(scheme=tab(run=[run("start")])), WORDS, "on", "working")
    assert not ok and "expected" in why
    assert not judge_run_by_switch(page(scheme=tab(run=[run("start")])), WORDS, "off", "fault")[0], "a fault is acknowledged by Stop, not Start"


def test_a_switched_off_burner_that_burns_out_has_a_disabled_start_and_its_reason():
    said = page(scheme=tab(run=[run("start", disabled=True)], cannot=["Burning out"]))
    assert judge_start_refused(said, WORDS, "off", "burning_off")[0]


def test_a_start_that_is_enabled_while_the_fire_burns_out_fails():
    ok, why = judge_start_refused(page(scheme=tab(run=[run("start")])), WORDS, "off", "burning_off")
    assert not ok and "disabled" in why


def test_a_reason_standing_where_nothing_is_refused_fails():
    said = page(scheme=tab(run=[run("start")], cannot=["Burning out"]))
    assert not judge_start_refused(said, WORDS, "off", "stop")[0]
    assert judge_start_refused(page(scheme=tab(run=[run("stop")])), WORDS, "on", "burning_off")[0]


def test_light_now_stands_once_while_a_switched_on_burner_stands_in_a_stop_with_no_fire():
    said = page(scheme=tab(run=[run("stop")], now=[{"text": "Now", "disabled": False}]))
    assert judge_light_now(said, WORDS, "on", "stop")[0]


def test_light_now_in_a_stage_with_a_fire_or_a_fault_or_with_the_switch_off_fails():
    said = page(scheme=tab(run=[run("stop")], now=[{"text": "Now", "disabled": False}]))
    for stage in ("purge", "working", "smouldering", "burning_off", "fault"):
        assert not judge_light_now(said, WORDS, "on", stage)[0], stage
    assert not judge_light_now(said, WORDS, "off", "stop")[0]
    assert judge_light_now(page(), WORDS, "off", "stop")[0]
    assert judge_light_now(page(), WORDS, "on", "working")[0]


def test_a_missing_light_now_in_a_stop_fails():
    ok, why = judge_light_now(page(), WORDS, "on", "stop")
    assert not ok and "0 buttons" in why


def test_a_disabled_light_now_fails():
    said = page(scheme=tab(run=[run("stop")], now=[{"text": "Now", "disabled": True}]))
    assert not judge_light_now(said, WORDS, "on", "stop")[0]


def test_a_scheme_with_a_mode_switch_or_a_field_is_not_quiet():
    assert judge_scheme_is_quiet(page())[0]
    assert not judge_scheme_is_quiet(page(scheme=tab(mode=1)))[0]
    assert not judge_scheme_is_quiet(page(scheme=tab(inputs=3)))[0]


def test_the_system_tab_needs_the_service_heading_once_and_the_sections_the_menu_names():
    assert judge_service_has_hands(page(system=tab(titles=["Time", "Hands", "Fire"], service=1)), ["Hands", "Fire"])[0]
    ok, why = judge_service_has_hands(page(system=tab(titles=["Time"], service=1)), ["Hands", "Fire"])
    assert not ok and "Hands" in why
    assert not judge_service_has_hands(page(system=tab(titles=["Hands"], service=0)), ["Hands"])[0], "no heading"
    assert not judge_service_has_hands(page(system=tab(titles=["Hands"], service=2)), ["Hands"])[0], "two headings"


def hands_page(text="Take", disabled=False, body="what it does"):
    return page(system=tab(hands=[{"text": text, "disabled": disabled}], hands_text=[body]))


def test_the_hands_button_says_take_with_its_words_under_it_and_a_press_sends_on_to_the_switch():
    assert judge_hands_button(hands_page(), "Take", "what it does")[0]
    assert judge_hands_button(hands_page(), "Take", "what it does", ["POST /on/77"], 77)[0]


def test_a_hands_button_that_is_missing_idle_worded_otherwise_or_silent_under_a_press_fails():
    assert not judge_hands_button(page(), "Take", "what it does")[0], "no button"
    assert not judge_hands_button(hands_page(disabled=True), "Take", "what it does")[0], "idle where the hands are free"
    assert not judge_hands_button(hands_page(text="Give back"), "Take", "what it does")[0], "the other button"
    assert not judge_hands_button(hands_page(body=""), "Take", "what it does")[0], "no words under it"
    ok, why = judge_hands_button(hands_page(), "Take", "what it does", ["POST /off/77"], 77)
    assert not ok and "on" in why


def given_page(text="Give back", disabled=False, body="takes back"):
    return page(system=tab(hands=[{"text": text, "disabled": disabled}], hands_text=[body]))


def test_the_hands_taken_have_a_give_back_button_with_what_the_automaton_takes_and_a_press_sends_off():
    assert judge_hands_given(given_page(), "Give back", "takes back")[0]
    assert judge_hands_given(given_page(), "Give back", "takes back", ["POST /off/77"], 77)[0]


def test_a_take_button_over_taken_hands_or_a_press_that_sends_on_fails():
    assert not judge_hands_given(given_page(text="Take"), "Give back", "takes back")[0]
    assert not judge_hands_given(given_page(body=""), "Give back", "takes back")[0]
    assert not judge_hands_given(given_page(), "Give back", "takes back", ["POST /on/77"], 77)[0]


def test_the_commands_the_stand_logged_are_read_in_order_by_their_way_and_their_address():
    assert sends_of(["12:00:01 POST /press/9", "noise", "12:00:02 POST /off/237 "]) == [("press", 9), ("off", 237)]


def test_one_command_for_two_clicks_in_a_row_passes_and_two_fail():
    assert judge_one_press(["POST /press/9"], [("press", 9)], "Start")[0]
    ok, why = judge_one_press(["POST /press/9", "POST /press/10"], [("press", 9)], "Start")
    assert not ok and "sent [('press', 9), ('press', 10)]" in why, "the second click took the first back"


def test_a_press_after_the_moment_must_be_sent_and_a_lost_one_fails():
    assert judge_one_press(["POST /press/9", "POST /press/10"], [("press", 9), ("press", 10)], "Start")[0]
    assert not judge_one_press(["POST /press/9"], [("press", 9), ("press", 10)], "Start")[0], "a second press the lock kept from the board"
    assert not judge_one_press([], [("press", 9)], "Start")[0], "no press sent"


from page_plum import judge_refused_hand


def after_note(note):
    return {"after": {"note": note}, "sent": []}


def test_an_auger_refused_with_the_boards_words_and_asked_again_after_the_moment_passes():
    said = {**after_note("the feed is banned: press Allow feed first"), "sent": ["POST /on/9", "POST /on/9"]}
    assert judge_refused_hand(said, 9, ("banned", "Allow feed"))[0]


def test_a_refusal_the_page_does_not_say_a_button_not_pressed_and_a_button_held_after_it_fail():
    sent = ["POST /on/9", "POST /on/9"]
    assert not judge_refused_hand({**after_note(""), "sent": sent}, 9, ("banned", "Allow feed"))[0], "the page says nothing"
    assert not judge_refused_hand({**after_note("the feed is banned: press Allow feed"), "sent": []}, 9, ("banned",))[0], "never sent"
    ok, why = judge_refused_hand({**after_note("the feed is banned: press Allow feed"), "sent": ["POST /on/9"]}, 9, ("banned",))
    assert not ok and "again" in why, "the button stayed held after the refusal"


def test_a_fill_that_grows_while_held_and_empties_on_release_passes():
    assert judge_fill({"during": [0.25, 0.5], "after": 0}, "Start")[0]


def test_a_button_with_no_fill_or_a_fill_that_does_not_grow_or_stays_after_release_fails():
    assert not judge_fill({"during": [None, None], "after": None}, "Start")[0], "no fill on the button"
    assert not judge_fill({"during": [0.5, 0.5], "after": 0}, "Start")[0], "a fill that stands still"
    assert not judge_fill({"during": [0.3, 1.4], "after": 0}, "Start")[0], "a fill wider than the button"
    assert not judge_fill({"during": [0.25, 0.5], "after": 0.5}, "Start")[0], "a fill left after the hand let go"
    assert not judge_fill({"during": [], "after": None, "missing": True}, "Start")[0], "no such button"
