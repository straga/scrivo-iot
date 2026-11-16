"""tools/page_steady_modules.py: the verdict on the words of a card's rows (REF_Device_Page_Look, rule 7) on what the browser said, made up here - a state of one word, a dash, or a
number with its unit passes; a phrase in a row is named with the card, the row and the moment.

    uv run --no-project --with pytest --with websocket-client pytest tools/test_page_steady_modules.py
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from page_steady_modules import one_word


def card(name, *says):
    return {"cards": [{"card": name, "says": [list(s) for s in says]}]}


def verdict(seen, cards=("Flame",)):
    (check, ok, words), = [one_word(seen, cards)]
    assert check == "state-is-one-word"
    return ok, words


def test_a_word_a_dash_and_a_number_with_its_unit_are_states():
    seen = [("dark @5", card("Flame", ("flame", "out"), ("pin", "3150 mV"))), ("silent @95", card("Flame", ("flame", "-"), ("pin", "-")))]
    assert verdict(seen)[0]
    assert verdict([("warm @5", card("Temperatures", ("water in the boiler", "61.5 °C"), ("bus", "ok")))], ("Temperatures",))[0]
    assert verdict([("below @5", card("Temperatures", ("the shed", "-23.5 °C")))], ("Temperatures",))[0]


def test_in_now_the_word_may_have_the_number_beside_it_and_no_more():
    assert verdict([("dark @5", card("Now", ("flame", "out \u00b7 3.15 V")))], ("Now",))[0]
    assert not verdict([("dark @5", card("Now", ("flame", "out \u00b7 3.15 V \u00b7 spread 0 mV")))], ("Now",))[0]


def test_a_phrase_in_a_row_is_named_with_the_card_the_row_and_the_moment():
    ok, words = verdict([("weak @35", card("Flame", ("flame", "weak - lit, but not steady by the thresholds below")))])
    assert not ok and "'flame' of 'Flame' says 'weak - lit, but not steady by the thresholds below' in weak @35" in words


def test_two_numbers_in_a_row_are_a_phrase_too():
    ok, words = verdict([("steady @65", card("Flame", ("pin", "1900 mV · spread 0 mV of 20 · at the top of the scale")))])
    assert not ok and "'pin' of 'Flame'" in words


def test_only_the_cards_asked_about_are_held_to_it():
    said = card("Flame", ("flame", "out"))
    said["cards"] += card("Scheme", ("mode", "a long sentence of a card nobody asked about"))["cards"]
    assert verdict([("stop @0", said)])[0]


def test_a_heading_the_page_writes_in_lower_case_is_the_card_of_its_capitalised_name():
    assert verdict([("dark @5", card("flame", ("flame", "out")))])[0]


def test_a_card_that_is_not_there_is_named_and_does_not_pass():
    ok, words = verdict([("stop @0", card("Flue", ("flue gas", "148.0 °C")))])
    assert not ok and "no card 'Flame' in stop @0" in words


# ---- a time is a number with its units ----


def test_a_time_with_its_units_is_a_state_and_a_sentence_about_time_is_not():
    seen = [("lit @60", card("Automaton", ("for", "4 min 22 s"), ("next", "igniter-off · 45 s"), ("pump by", "water · 66 °C"), ("auger", "banned")))]
    assert verdict(seen, ("Automaton",))[0]
    assert verdict([("lit @60", card("Automaton", ("for", "1 h 5 min")))], ("Automaton",))[0]
    for phrase in ("for 4 minutes", "4 min 22 s ago", "igniter off in 45 s", "the water is past 95 °C", "off - feed banned"):
        ok, words = verdict([("lit @60", card("Automaton", ("row", phrase)))], ("Automaton",))
        assert not ok and phrase in words, phrase


# ---- the line of what the automaton does is a code, never a phrase (REF_Device_Page_Look, rule 7) ----

from page_steady_modules import doing_is_code


STRIP = ["stop", "purge", "lighting", "rising", "working", "burn-off"]


def doing_card(line, strip=STRIP):
    """What the browser said of the automaton's card: its heading, the cells of the strip, the line of what it does, the rows."""
    # as the browser says it: a blank line after the strip and after the line
    text = "\n".join(["Automaton"] + strip + ["", line, "", "Start", "Stop", "for", "28 s"])
    return {"cards": [{"card": "Automaton", "text": text}]}


def doing_seen(*lines):
    return [("m%d" % n, doing_card(line)) for n, line in enumerate(lines)]


def test_a_stage_word_with_its_attempt_or_its_cause_as_a_code_is_the_line_of_what_the_automaton_does():
    seen = doing_seen("stopped", "lighting \u00b7 2", "burning-off", "burning-off \u00b7 no-light 3", "fault \u00b7 too-hot 97 \u00b0C", "fault \u00b7 no-fuel", "fault \u00b7 draught-silent")
    check, ok, words = doing_is_code(seen, "Automaton", len(STRIP))
    assert (check, ok) == ("doing-is-code", True), words


def test_in_smouldering_the_numbers_of_it_follow_the_word_and_a_phrase_there_is_still_none():
    # the time gone of the whole, the feed, what is left - numbers, in either language
    for good in ("smouldering \u00b7 3:39 of 15:00 \u00b7 feed 1 s / 50 s \u00b7 11:21 until it goes out", "\u0442\u043b\u0435\u043d\u0438\u0435 \u00b7 3:39 \u0438\u0437 15:00 \u00b7 \u043f\u043e\u0434\u0430\u0447\u0430 1 \u0441 / 50 \u0441 \u00b7 \u0434\u043e \u0433\u0430\u0448\u0435\u043d\u0438\u044f 11:21"):
        check, ok, words = doing_is_code(doing_seen(good), "Automaton", len(STRIP))
        assert ok, words
    for bad in ("smouldering \u00b7 the water came to the setpoint and the fire is kept low", "smouldering \u00b7 3:39 of 15:00", "smouldering \u00b7 soon \u00b7 a while \u00b7 later"):
        check, ok, words = doing_is_code(doing_seen(bad), "Automaton", len(STRIP))
        assert not ok and bad in words, bad


def test_a_phrase_in_the_line_is_named_with_the_moment():
    for phrase in ("lighting, attempt 1", "fault: did not light in 3 attempts", "burning off: no fuel arriving", "fault \u00b7 the water is past 95 \u00b0C", "burning_off", "something did not add up and the burner stopped"):
        check, ok, words = doing_is_code(doing_seen("working", phrase), "Automaton", len(STRIP))
        assert not ok and "m1" in words and ("'%s'" % phrase) in words, phrase


def test_a_card_that_is_not_there_or_has_no_line_is_named_and_does_not_pass():
    check, ok, words = doing_is_code([("m0", {"cards": []})], "Automaton", len(STRIP))
    assert not ok and "no card" in words
    check, ok, words = doing_is_code([("m0", {"cards": [{"card": "Automaton", "text": "Automaton\nstop"}]})], "Automaton", len(STRIP))
    assert not ok and "no line" in words


# ---- the card a module is looked for under is the menu's name for it, never a name written in the check ----

import page_steady_modules


def test_a_card_is_looked_for_under_the_title_the_menu_gives_it_and_the_card_of_the_page_under_its_own():
    real = page_steady_modules.menu_of
    try:
        page_steady_modules.menu_of = lambda module: {"card": {"title": {"en": "Burner %s" % module}}}
        assert page_steady_modules.title_of("Flame") == "Burner flame"
        assert page_steady_modules.title_of("Temperatures") == "Burner temperatures"
        assert page_steady_modules.title_of("Now") == "Now", "the card Now is the page's own and the menu has no word for it"
        assert page_steady_modules.title_of("How it runs") == "How it runs"
    finally:
        page_steady_modules.menu_of = real


# ---- the words the Scheme tab is looked for under are the menu's: a menu worded otherwise is looked for under the other words ----
from page_steady_modules import pin_volts, scheme_words  # noqa: E402


def words(en):
    return {"en": en, "ru": ""}


MENU = {
    "scheme": {
        "title": words("T"), "parts": {"pot": words("P")},
        "feed": {"ban": words("Fb"), "allow": words("Fa")},
        "run": {"start": words("Go"), "stop": words("Halt")},
        "actions": {"burning": words("Ab")},
    },
    "modules": {
        "automaton": {"card": {"title": words("Au")}, "rows": {
            "stage": {"strip": {"cells": [["stop", words("s1")], ["purge", words("s2")]]}}}},
        # the buttons are the thermostat's since 6.10; the scheme draws them
        "thermostat": {"rows": {"start": {"label": words("go")}, "stop": {"label": words("halt")}}},
        "flame": {"rows": {"flame": {"says": [["out", words("o")], ["steady", words("st")], ["unknown:not_read", words("-")]]}}},
    },
}


def test_the_scheme_is_looked_for_under_the_words_of_the_menu():
    got = scheme_words(MENU)
    # the one button that runs the burner has the two words it changes between, and the feed's likewise
    assert got["controls"] == [("Go", "Halt"), ("Fb", "Fa")]
    assert got["starts"] == ["Ab"] and got["pot"] == "P" and got["automaton"] == "Au"
    assert got["stages"] == ["s1", "s2"]
    assert got["flame"] == ["o", "st"], "a flame not read is no word of the pot"


def test_a_menu_worded_otherwise_is_looked_for_under_the_other_words():
    other = {**MENU, "scheme": {**MENU["scheme"], "feed": {"ban": words("Stop feed"), "allow": words("Feed again")}}}
    assert scheme_words(other)["controls"][1] == ("Stop feed", "Feed again")
    assert scheme_words(MENU)["controls"][1] == ("Fb", "Fa")
    run = {**MENU, "scheme": {**MENU["scheme"], "run": {"start": words("Light"), "stop": words("Put out")}}}
    assert scheme_words(run)["controls"][0] == ("Light", "Put out")


def test_the_flame_pin_of_a_second_is_the_last_value_the_frames_gave_up_to_it():
    frames = [{"at": 0, "values": [[7, 900.0], [8, 1]]}, {"at": 1, "values": [[8, 2]]}, {"at": 2, "values": [[7, 3153.0]]}]
    assert pin_volts(frames, 7, 1) == "0.90 V"
    assert pin_volts(frames, 7, 2) == "3.15 V"


def test_a_pin_that_reads_nothing_or_was_never_said_is_a_dash_not_nought():
    frames = [{"at": 0, "values": [[7, {"unknown": "not_read"}]]}, {"at": 1, "values": [[8, 1]]}]
    assert pin_volts(frames, 7, 0) == "-"
    assert pin_volts(frames, 9, 1) == "-"


# ---- the modules' pages: every setting stands on the page of its module (or the system tab) and on no other, and a page holds what its module is ----
from page_steady_modules import module_pages, tree_places, tree_plan, view_of  # noqa: E402

TREE_MENU = {
    "ui": {"system": words("Box")},
    "modules": {
        "m": {"settings": {"a": {"label": words("Alpha")}, "b": {"label": words("Beta")}, "c": {"label": words("Gamma")}},
              "rows": {"r": {"label": words("relay")}, "q": {"label": words("quiet")}, "r.part": {"label": words("a part")}}},
    },
    "tree": {"main": {"sections": [{"id": "s1", "title": words("One"), "settings": ["a", "b"]}]},
             "modules": {"groups": [], "pages": [
                 {"id": "p", "title": words("Motor"), "sections": [{"id": "s2", "cards": ["m"], "settings": ["a", "b"]}]},
                 {"id": "k", "title": words("Other"), "sections": [{"id": "s4", "title": words("Four"), "settings": ["c"]}]}]},
             "system": {"sections": [{"id": "s3", "title": words("Three"), "settings": ["t"]}]}},
}
TREE_TEXTS = {"settings": {"t": {"label": "Time"}}}
TREE_DICT = {"settings": [{"name": "a", "choices": [{"value": "x", "shows": ["b"]}, {"value": "y"}]}, {"name": "b"}, {"name": "c"}, {"name": "t"}]}


def test_the_plan_of_the_tree_is_the_menus_labels_by_page_and_what_a_choice_governs_may_be_absent():
    plan = tree_plan(TREE_MENU, TREE_TEXTS, TREE_DICT)
    assert plan["p"] == {"title": "Motor", "labels": ["Alpha", "Beta"], "optional": ["Beta"], "cards": ["m"]}
    assert plan["k"] == {"title": "Other", "labels": ["Gamma"], "optional": [], "cards": []}
    assert plan["system"] == {"title": "Box", "labels": ["Time"], "optional": [], "cards": []}


def test_the_second_of_a_pair_stands_on_the_line_of_the_first_and_is_no_label_of_the_page():
    # a setpoint and its hysteresis are one line, labelled by the setpoint and the pair's word
    menu = {**TREE_MENU, "tree": {**TREE_MENU["tree"], "modules": {"groups": [], "pages": [
        {"id": "p", "title": words("Motor"), "sections": [{"id": "s2", "settings": ["a", "b"], "pairs": [{"of": ["a", "b"], "second": words("Hyst")}]}]}]}}}
    plan = tree_plan(menu, TREE_TEXTS, TREE_DICT)
    assert plan["p"]["labels"] == ["Alpha"], plan["p"]


def test_the_first_of_a_pair_and_a_single_row_stand_under_the_words_the_menu_gives_them_in_that_list():
    # the pair names its first where the setting's own label is longer than the column, and the section names a single row; the page draws those words, so those are the labels looked for
    menu = {**TREE_MENU, "tree": {**TREE_MENU["tree"], "modules": {"groups": [], "pages": [
        {"id": "p", "title": words("Motor"), "sections": [{"id": "s2", "settings": ["a", "b", "c"], "pairs": [{"of": ["a", "b"], "first": words("Al"), "second": words("Hyst")}], "labels": {"c": words("Ga")}}]}]}}}
    plan = tree_plan(menu, TREE_TEXTS, TREE_DICT)
    assert plan["p"]["labels"] == ["Al", "Ga"], plan["p"]


def test_the_system_tab_is_a_view_of_its_own_and_a_module_a_page_on_the_modules_tab():
    assert view_of("system") == "System" and view_of("fan") == "Modules>fan"


def view(*lines):
    return {"cards": [{"card": "card 1", "text": "\n".join(lines), "rows": []}]}


def views(p=None, k=None, system=None):
    return {"Modules>p": p or view("Alpha", "Beta", "Save"), "Modules>k": k or view("Four", "Gamma"), "System": system or view("Time")}


def test_every_label_stands_on_its_own_page_and_nowhere_else():
    plan = tree_plan(TREE_MENU, TREE_TEXTS, TREE_DICT)
    check, ok, said = tree_places(views(), plan)
    assert check == "tree-places" and ok, said


def test_a_label_missing_from_its_page_or_standing_on_another_is_named():
    plan = tree_plan(TREE_MENU, TREE_TEXTS, TREE_DICT)
    missing = tree_places(views(p=view("Beta")), plan)
    assert not missing[1] and "'Alpha' is not on the page of 'Motor'" in missing[2]
    twice = tree_places(views(p=view("Alpha", "Beta", "Gamma")), plan)
    assert not twice[1] and "'Gamma' stands on 'Other' and on 'Motor'" in twice[2]


def test_a_label_a_choice_governs_may_be_absent_while_the_choice_is_not_the_one_set():
    plan = tree_plan(TREE_MENU, TREE_TEXTS, TREE_DICT)
    held = tree_places(views(p=view("Alpha")), plan)
    assert held[1], held[2]


PAGES_DICT = {"things": [{"card": "m", "name": "r", "curves": ["c1"]}, {"card": "m", "name": "q", "range": {"min": 0, "max": 9, "step": 1, "at": 2}}, {"card": "m", "name": "z", "part_of": "r", "curves": ["x"]}]}


def page_of(rows, curves=1, sliders=1):
    return {"Modules>p": {"cards": [{"card": "card 1", "text": "\n".join(rows), "rows": rows, "curves": curves, "sliders": sliders}]}}


def test_a_page_with_every_reading_curve_and_slider_of_its_module_passes():
    check, ok, said = module_pages(page_of(["Relay", "Quiet"]), TREE_MENU, PAGES_DICT)
    assert check == "module-pages" and ok, said


def test_sockets_that_have_no_role_are_not_drawn_so_their_pictures_are_one_at_least():
    sockets = {"things": [{"card": "m", "name": "ntc_%d" % n, "curves": ["c"]} for n in range(1, 8)]}
    assert module_pages(page_of(["Relay", "Quiet"], curves=1, sliders=0), TREE_MENU, sockets)[1], "one picture of seven sockets is a page with its sockets' pictures"
    assert "draws 0 pictures of the 1 rows that have curves" in module_pages(page_of(["Relay", "Quiet"], curves=0, sliders=0), TREE_MENU, sockets)[2]


def test_a_reading_a_curve_or_a_slider_the_page_leaves_out_is_named_and_a_part_is_no_reading():
    assert "'Quiet' is not on the page of 'Motor'" in module_pages(page_of(["Relay"]), TREE_MENU, PAGES_DICT)[2]
    assert "draws 0 pictures of the 1 rows that have curves" in module_pages(page_of(["Relay", "Quiet"], curves=0), TREE_MENU, PAGES_DICT)[2]
    assert "draws 0 sliders where the board says 1 rows are held by one" in module_pages(page_of(["Relay", "Quiet"], sliders=0), TREE_MENU, PAGES_DICT)[2]
    assert "a part" not in module_pages(page_of(["Relay", "Quiet"]), TREE_MENU, PAGES_DICT)[2]


# ---- the power steps: a row a step, three numbers in it equal to the board's, and the row the burner burns on marked ----
from page_steady_modules import steps_table  # noqa: E402

STEPS_SECTION = {"id": "steps", "title": words("Power steps"), "table": {"chosen": "level", "columns": [{"key": "on_s", "title": words("A")}, {"key": "off_s", "title": words("B")}, {"key": "pa", "title": words("C")}],
                 "rows": [{"step": "10", "title": words("10 kW"), "settings": ["l10_on", "l10_off", "l10_pa"]}, {"step": "6", "title": words("6 kW"), "settings": ["l6_on", "l6_off", "l6_pa"]}]}}
HELD = {"level": "6", "l10_on": "2", "l10_off": "12", "l10_pa": "36", "l6_on": "2", "l6_off": "21", "l6_pa": "22"}


def steps_card(*rows):
    return {"card": "Power steps", "steps": [{"step": s, "chosen": c, "values": v} for s, c, v in rows]}


def good_rows():
    return [("10", False, ["2", "12", "36"]), ("6", True, ["2", "21", "22"])]


def test_a_table_drawn_as_the_menu_says_with_the_boards_numbers_and_the_burning_row_marked_passes():
    check, ok, said = steps_table(steps_card(*good_rows()), STEPS_SECTION, HELD)
    assert check == "steps-table" and ok, said


def test_a_number_that_is_not_the_boards_is_named_by_its_row_and_column():
    rows = good_rows()
    rows[1] = ("6", True, ["2", "20", "22"])
    check, ok, said = steps_table(steps_card(*rows), STEPS_SECTION, HELD)
    assert not ok and "6" in said and "l6_off" in said and "20" in said and "21" in said, said


def test_the_marked_row_is_the_one_the_board_holds_and_one_only():
    wrong = steps_card(("10", True, ["2", "12", "36"]), ("6", False, ["2", "21", "22"]))
    assert not steps_table(wrong, STEPS_SECTION, HELD)[1]
    both = steps_card(("10", True, ["2", "12", "36"]), ("6", True, ["2", "21", "22"]))
    assert not steps_table(both, STEPS_SECTION, HELD)[1]
    none = steps_card(("10", False, ["2", "12", "36"]), ("6", False, ["2", "21", "22"]))
    assert not steps_table(none, STEPS_SECTION, HELD)[1]


def test_a_level_the_table_has_no_row_for_marks_no_row():
    odd = dict(HELD, level="7")
    unmarked = steps_card(("10", False, ["2", "12", "36"]), ("6", False, ["2", "21", "22"]))
    assert steps_table(unmarked, STEPS_SECTION, odd)[1]
    marked = steps_card(("10", False, ["2", "12", "36"]), ("6", True, ["2", "21", "22"]))
    assert not steps_table(marked, STEPS_SECTION, odd)[1]


def test_a_missing_row_or_a_missing_section_or_a_cell_short_of_three_is_a_failure():
    assert not steps_table(steps_card(("6", True, ["2", "21", "22"])), STEPS_SECTION, HELD)[1]
    assert not steps_table(steps_card(("10", False, ["2", "12", "36"]), ("6", True, ["2", "21"])), STEPS_SECTION, HELD)[1]
    assert not steps_table(None, STEPS_SECTION, HELD)[1]


# ---- the checks in the language the page is asked in ----
from page_steady_modules import in_language, texts_in  # noqa: E402


def test_the_checks_read_the_menus_words_in_the_language_asked_under_the_key_they_always_read():
    menu = {"a": {"title": {"en": "One", "ru": "Раз"}, "list": [{"en": "Two", "ru": ""}, {"en": "Three", "ru": "Три"}]}, "plain": "kept"}
    ru = in_language(menu, "ru")
    assert ru["a"]["title"]["en"] == "Раз"
    assert [w["en"] for w in ru["a"]["list"]] == ["Two", "Три"], "a word with no Russian is the English one, as the page says it"
    assert ru["plain"] == "kept"
    assert in_language(menu, "en")["a"]["title"]["en"] == "One"


def test_the_table_of_texts_is_laid_with_its_russian_the_way_the_page_lays_it():
    texts = {"rows": {"x": {"label": "console", "says": ["a"]}, "y": {"label": "wifi"}}, "ru": {"rows": {"x": {"label": "консоль"}}}}
    got = texts_in(texts, "ru")
    assert got["rows"]["x"]["label"] == "консоль" and got["rows"]["y"]["label"] == "wifi" and got["rows"]["x"]["says"] == ["a"]
    assert "ru" not in got
    assert texts_in(texts, "en") is texts


# ---- the reference tab says every item of the menu ----
from page_steady_modules import reference_complete  # noqa: E402

HELP_MENU = {"help": {"entries": {"a": {"name": words("Alpha"), "text": words("What alpha is and what it is for")}, "b": {"name": words("Beta"), "text": words("What beta is")}}}}


def test_a_reference_that_says_every_name_and_every_line_whole_passes():
    tabbed = {"cards": [{"card": "One", "text": "One\nAlpha\nWhat alpha is and\nwhat it is for\nBeta\nWhat beta is"}]}
    check, ok, said = reference_complete(tabbed, HELP_MENU)
    assert check == "reference-complete" and ok, said


def test_a_name_or_a_line_the_page_left_out_or_cut_is_named_by_its_key():
    cut = {"cards": [{"card": "One", "text": "Alpha\nWhat alpha is and\nBeta\nWhat beta is"}]}
    assert not reference_complete(cut, HELP_MENU)[1] and "a: line" in reference_complete(cut, HELP_MENU)[2]
    gone = {"cards": [{"card": "One", "text": "Alpha\nWhat alpha is and what it is for"}]}
    assert "b: name" in reference_complete(gone, HELP_MENU)[2]


# ---- the shape of a state is the same in any language the page speaks ----
def test_a_russian_word_a_number_with_its_unit_and_a_time_are_states_and_a_phrase_is_not():
    for good in ("темно", "40.5 °C", "ровный \u00b7 0.90 В", "4 мин 22 с", "-", "держит \u00b7 22 Па"):
        assert verdict([("m @1", card("Flame", ("flame", good)))])[0], good
    assert not verdict([("m @1", card("Flame", ("flame", "огонь не виден нигде")))])[0]
    assert not verdict([("m @1", card("Flame", ("flame", "the flame cannot be seen")))])[0]


# ---- every word of the drawing stands clear of its frames ----
from page_steady import labels_clear, tabs_one_row  # noqa: E402


def test_a_drawing_whose_words_cross_nothing_passes_and_one_that_does_is_named_by_the_word_and_the_moment():
    clean = [("rising @120", {"texts": 12, "crossed": []})]
    check, ok, said = labels_clear(clean)
    assert check == "labels-clear-frames" and ok, said
    dirty = [("rising @120", {"texts": 12, "crossed": ["'вентилятор' on a rect", "'труба' over 'Авто'"]})]
    check, ok, said = labels_clear(dirty)
    assert not ok and "'вентилятор' on a rect in rising @120" in said and "'труба' over 'Авто'" in said


def button(words, top, left, width):
    return {"button": "0 " + words, "top": top, "left": left, "width": width}


TABS5 = ["Scheme", "Burner", "User", "Service", "Help"]


def test_five_tabs_at_one_height_inside_the_page_are_one_row():
    row = [button(w, 130, 16 + i * 66, 64) for i, w in enumerate(TABS5)]
    check, ok, said = tabs_one_row(row + [button("Save", 900, 16, 80)], TABS5)
    assert check == "tabs-one-row" and ok, said


def test_tabs_that_wrap_to_a_second_row_or_run_past_the_edge_or_are_missing_fail_naming_it():
    wrapped = [button(w, 130 if i < 3 else 190, 16 + (i % 3) * 112, 112) for i, w in enumerate(TABS5)]
    assert not tabs_one_row(wrapped, TABS5)[1] and "2 heights" in tabs_one_row(wrapped, TABS5)[2]
    past = [button(w, 130, 16 + i * 80, 78) for i, w in enumerate(TABS5)]
    assert not tabs_one_row(past, TABS5)[1] and "past the edge" in tabs_one_row(past, TABS5)[2]
    assert not tabs_one_row(past[:4], TABS5)[1]
