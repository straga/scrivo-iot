"""The verdicts of tools/page_steady.py on what the browser said, made up here: a still page passes, a page that moves
or hides its controls is named where and when.

    uv run --no-project --with pytest --with websocket-client pytest tools/test_page_steady.py
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from page_steady import clear, crowded, flame_volts, flame_number, heads, in_view, judge


def button(i, words, top, bottom=None):
    return {"button": "%d %s" % (i, words), "top": top, "left": 16, "width": 100,
            "bottom": top + 44 if bottom is None else bottom}


def page(height=400, rows=("for", "next"), feed="Ban feed", start_top=500):
    return {"cards": [{"card": "Automaton", "top": 600, "height": height, "rows": list(rows)}],
            "buttons": [button(0, "Scheme", 60), button(1, feed, start_top - 300), button(2, "Start", start_top),
                        button(3, "Stop", start_top)]}


def verdicts(seen):
    return {check: (ok, words) for check, ok, words in judge(seen)}


def test_a_page_that_stands_still_passes_every_check():
    got = verdicts([("stop @0", page()), ("purge @2", page())])
    assert all(ok for ok, _ in got.values()), got


def test_a_card_taller_in_one_moment_is_named_with_the_moment():
    ok, words = verdicts([("stop @0", page()), ("purge @2", page(height=419.5))])["heights-steady"]
    assert not ok and "'Automaton' +19.5 px in purge @2" in words


def test_a_row_gone_in_one_moment_is_named():
    ok, words = verdicts([("stop @0", page()), ("purge @2", page(rows=("for",)))])["rows-steady"]
    assert not ok and "'next' of 'Automaton' gone in purge @2" in words


def test_a_button_moved_down_is_named():
    ok, words = verdicts([("stop @0", page()), ("purge @2", page(start_top=520))])["buttons-steady"]
    assert not ok and "'2 Start' +20.0 px down" in words


def test_the_feed_button_that_changes_its_words_is_the_same_button():
    got = verdicts([("stop @0", page()), ("banned @9", page(feed="Allow feed"))])
    assert got["buttons-steady"][0], got


def test_controls_above_the_fold_in_every_moment_pass():
    check, ok, words = in_view([("stop @0", page(start_top=500)), ("banned @9", page(feed="Allow feed"))])
    assert check == "controls-in-view" and ok, words


def test_a_control_below_the_fold_is_named_with_how_far_down():
    _, ok, words = in_view([("stop @0", page()), ("purge @2", page(start_top=640))])
    assert not ok and "'Start' ends 684 px down in purge @2" in words


def test_a_control_not_drawn_is_out_of_view():
    bare = page()
    bare["buttons"] = [b for b in bare["buttons"] if "Stop" not in b["button"]]
    _, ok, words = in_view([("stop @0", bare)])
    assert not ok and "'Stop' not drawn in stop @0" in words


def card(name, strip_top=None, first_row_top=700, text=""):
    return {"card": name, "top": 0, "height": 100, "rows": [], "text": text, "strip_top": strip_top,
            "first_row_top": first_row_top}


def told(automaton, scheme=None):
    return {"cards": [scheme or card("Scheme", first_row_top=None), automaton], "buttons": []}


def test_the_strip_and_the_state_at_the_top_of_automaton_pass():
    check, ok, words = heads([("stop @0", told(card("Automaton", 650, 700, "stopped")), "stopped")])
    assert check == "automaton-heads-card" and ok, words


def test_a_strip_under_the_scheme_is_named():
    moved = told(card("Automaton", None, 700, "stopped"), card("Scheme", 500, None, "stopped"))
    _, ok, words = heads([("stop @0", moved, "stopped")])
    assert not ok and "no strip in Automaton in stop @0" in words and "a strip in 'Scheme' in stop @0" in words


def test_a_strip_below_the_first_row_is_named():
    _, ok, words = heads([("purge @2", told(card("Automaton", 900, 700, "purging")), "purging")])
    assert not ok and "the strip under the first row of Automaton in purge @2" in words


def test_a_state_said_elsewhere_is_named():
    _, ok, words = heads([("purge @2", told(card("Automaton", 650, 700, "stopped")), "purging")])
    assert not ok and "'purging' not in Automaton in purge @2" in words


def test_organ_words_clear_of_the_lines_pass_and_are_counted():
    check, ok, words = clear([("stop @0", {"captions": 0, "crossed": []}), ("heating @65", {"captions": 2, "crossed": []})])
    assert check == "captions-clear-lines" and ok and words == "2 organ words clear of every line in 2 moments"


def test_words_on_a_line_are_named_with_the_moment():
    _, ok, words = clear([("heating @65", {"captions": 2, "crossed": ["'burning 59 min 59 s' on a path"]})])
    assert not ok and "'burning 59 min 59 s' on a path in heating @65" in words


def frame_with_pin(value):
    # the rows of a frame by card and name, as tools/stand_frames.py builds them from the dictionary
    return {"at": 0, "named": {("flame", "pin"): value}}


def test_the_flame_wanted_is_its_millivolts_or_a_dash():
    assert flame_volts(frame_with_pin({"number": {"value": 3153, "unit": "mV"}})) == "3.15 V"
    assert flame_volts(frame_with_pin({"unknown": {}})) == "-"
    assert flame_volts({"at": 0, "named": {}}) == "-"


def test_the_flame_number_on_the_drawing_passes():
    check, ok, words = flame_number([("steady @110", {"texts": ["steady", "0.90 V", "flame"]}, "0.90 V"),
                                     ("flame_silent @770", {"texts": ["-", "flame"]}, "-")])
    assert check == "flame-number-on-scheme" and ok and "a dash in 1 of them" in words


def test_a_missing_number_and_a_made_up_nought_are_named():
    _, ok, words = flame_number([("steady @110", {"texts": ["steady", "flame"]}, "0.90 V"),
                                 ("flame_silent @770", {"texts": ["0.00 V", "flame"]}, "-")])
    assert not ok and "'0.90 V' not on the drawing in steady @110" in words
    assert "'-' not on the drawing in flame_silent @770" in words and "'0.00 V' on the drawing in flame_silent @770" in words


def box(text, left, top, right, bottom):
    return {"text": text, "left": left, "top": top, "right": right, "bottom": bottom}


POT_EDGE = {"left": 100, "top": 100, "right": 240, "bottom": 170}


def test_a_pot_with_room_around_every_word_passes():
    pot = {"pot": POT_EDGE, "texts": [box("steady", 150, 110, 200, 125), box("3.30 V", 150, 128, 205, 140)],
           "circles": [box("", 110, 120, 126, 136)]}
    check, ok, words = crowded([("feeding @155", pot)])
    assert check == "pot-holds-its-words" and ok, words


def test_a_word_at_the_pots_edge_is_named_with_how_close():
    pot = {"pot": POT_EDGE, "texts": [box("3.30 V", 190, 128, 238, 140)], "circles": []}
    _, ok, words = crowded([("feeding @155", pot)])
    assert not ok and "'3.30 V' 2.0 px from the pot's edge in feeding @155" in words


def test_words_over_words_and_over_the_plug_are_named():
    pot = {"pot": POT_EDGE, "texts": [box("steady", 150, 110, 200, 125), box("3.30 V", 150, 120, 205, 135),
                                      box("igniter", 105, 118, 140, 130)], "circles": [box("", 110, 120, 126, 136)]}
    _, ok, words = crowded([("feeding @155", pot)])
    assert not ok and "'steady' over '3.30 V' in feeding @155" in words and "'igniter' over a circle in feeding @155" in words


def test_a_drawing_without_a_pot_is_named():
    _, ok, words = crowded([("stop @0", {"pot": None})])
    assert not ok and "no pot in stop @0" in words


# ---- the page is waited for until it has drawn, and a page that never draws is a run that did not take place, not a verdict (the tail of 9.5) ----

import json as _json

from page_steady import wait_drawn


class FakePage:
    """A page that answers what the browser would, in order: the answers of each look at the page."""

    def __init__(self, *answers):
        self.answers = list(answers)
        self.looks = 0

    def js(self, _):
        self.looks += 1
        return _json.dumps(self.answers.pop(0) if len(self.answers) > 1 else self.answers[0])


EMPTY = {"cards": 0, "rows": 0}
DRAWN = {"cards": 5, "rows": 21}


def test_a_page_that_has_drawn_at_the_first_look_is_not_waited_for():
    p = FakePage(DRAWN)
    assert wait_drawn(p, tries=10, pause=0, sleep=lambda _: None) and p.looks == 1


def test_a_page_that_draws_late_is_waited_for_until_it_has():
    p = FakePage(EMPTY, EMPTY, {"cards": 5, "rows": 0}, DRAWN)
    assert wait_drawn(p, tries=10, pause=0, sleep=lambda _: None) and p.looks == 4


def test_a_page_that_never_draws_is_said_not_to_have_and_is_not_asked_for_cards_it_has_not():
    p = FakePage(EMPTY)
    assert not wait_drawn(p, tries=5, pause=0, sleep=lambda _: None) and p.looks == 5


def test_a_page_the_browser_cannot_answer_for_yet_is_waited_for_too():
    class Late(FakePage):
        def js(self, code):
            self.looks += 1
            if self.looks < 3:
                raise OSError("the page is navigating")
            return _json.dumps(DRAWN)

    p = Late(DRAWN)
    assert wait_drawn(p, tries=10, pause=0, sleep=lambda _: None) and p.looks == 3


def test_a_tab_of_shut_sections_is_drawn_by_the_folds_that_open_them():
    # the service's sections stand shut: a card with its heading and the button that opens it, and no row yet
    p = FakePage(EMPTY, {"cards": 6, "rows": 0, "folds": 6})
    assert wait_drawn(p, tries=10, pause=0, sleep=lambda _: None) and p.looks == 2


def test_cards_with_neither_a_row_nor_a_fold_are_not_a_page_drawn():
    p = FakePage({"cards": 6, "rows": 0, "folds": 0})
    assert not wait_drawn(p, tries=3, pause=0, sleep=lambda _: None)
