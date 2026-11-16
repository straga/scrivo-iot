"""tools/names_in_page.py on pages made up here, whose answer is known: what is a name of the board written in the code, and what is a word that
happens to be spelled like one.

    uv run --no-project --with pytest pytest tools/test_names_in_page.py
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from names_in_page import names_of, relations_in_texts, report_of, scan

DICTIONARY = {
    "cards": ["system", "engine"],
    "things": [
        {"card": "engine", "name": "rpm"},
        {"card": "engine", "name": "state"},
        {"card": "engine", "name": "oil_temp"},
        {"card": "system", "name": "uptime"},
        {"card": "system", "name": "free"},
    ],
    "settings": [{"name": "idle_rpm"}, {"name": "gear", "choices": [{"value": "low"}, {"value": "high"}]}],
    "machine": {"card": "engine", "fixed": "engine_fixed", "stage": "phase", "says": "doing", "cause": "broke_at", "press": ["start_it"], "aside": ["parts"],
                "sections": [{"name": "warm_up", "items": [["idle_rpm"]]}]},
}
NAMES = names_of(DICTIONARY)


def found(source):
    return [(name, line) for name, line, _ in scan(source, NAMES)]


def test_the_names_the_dictionary_gives_are_things_settings_cards_choices_and_the_machines_but_not_the_contracts():
    assert {"rpm", "oil_temp", "idle_rpm", "gear", "low", "high", "engine", "engine_fixed", "phase", "doing", "broke_at", "start_it", "parts", "warm_up"} <= NAMES
    assert not ({"uptime", "free", "system"} & NAMES), "the cards and the rows every board has are the contract's words"
    assert "state" not in names_of({"things": [{"card": "system", "name": "state"}]})
    assert "go" not in NAMES, "a name of two letters is not told from a word of the code"


def test_a_string_that_is_a_name_and_a_path_with_a_name_for_a_segment_are_written_names():
    assert found('const a = "oil_temp";') == [("oil_temp", 1)]
    assert found("const b = 'idle_rpm';\n\nconst c = `low`;") == [("idle_rpm", 1), ("low", 3)]
    assert found('act("/press/phase");') == [("phase", 1)]
    assert found('fetch("/state"); fetch("/press/rpm");') == [("rpm", 1)], "the contract's endpoints are no names of the board's"


def test_prose_that_holds_a_word_a_name_is_spelled_as_is_no_use_of_the_name():
    assert found('say("the engine is warm and low on oil");') == []
    assert found('const label = "phase one";') == []
    assert found('const text = `free ${x} bytes`;') == []


def test_an_identifier_is_a_name_only_when_it_has_an_underscore_in_it():
    assert found("const idle_rpm = 3; oil_temp += 1;") == [("idle_rpm", 1), ("oil_temp", 1)]
    assert found("const gear = 3; const doing = 1; rpm++;") == []


def test_comments_and_regular_expressions_are_not_read():
    assert found("// oil_temp is the one\n/* idle_rpm\n and more */\nconst x = 1;") == []
    assert found("const r = /^oil_temp$/; const s = str.replace(/idle_rpm/g, 'x');") == []
    assert found("const a = 6 / 2; const b = 'idle_rpm';") == [("idle_rpm", 1)]


def test_a_name_in_a_place_of_a_template_is_read_as_code_and_the_lines_are_counted():
    assert found("const t = `a ${x === \"oil_temp\"} b`;") == [("oil_temp", 1)]
    assert found("const t = `\nfirst\n${y}\n`;\nconst n = 'idle_rpm';") == [("idle_rpm", 5)]


def page_of(tmp_path, files):
    folder = tmp_path / "page"
    folder.mkdir()
    for name, text in files.items():
        (folder / name).write_text(text)
    return str(folder)


def test_every_script_of_the_page_folder_is_read_and_a_name_in_any_of_them_is_reported_with_its_file(tmp_path):
    page = page_of(tmp_path, {"page.js": 'const a = "oil_temp";\n', "wire.js": "const b = 1;\n", "texts.json": "{}"})
    assert report_of(page, NAMES) == {"page.js": [("oil_temp", 1, "string")]}


def test_a_page_folder_that_writes_no_name_reports_nothing_and_a_minified_library_is_not_read(tmp_path):
    page = page_of(tmp_path, {"page.js": "const a = 1;\n", "owl.iife.min.js": 'var x = "oil_temp";', "texts.json": "{}"})
    assert report_of(page, NAMES) == {}


def test_a_table_of_texts_that_says_a_relation_of_the_boards_things_is_reported(tmp_path):
    page = page_of(tmp_path, {"page.js": "const a = 1;\n", "texts.json": '{"now": {"rpm": {"as": "revs", "from": "state"}}}'})
    assert list(report_of(page, NAMES)) == ["texts.json"]


def test_a_setting_or_a_choice_spelled_like_a_row_every_board_has_is_left_out_with_it():
    dictionary = {"things": [{"card": "system", "name": "time"}], "settings": [{"name": "time"}, {"name": "zone", "choices": [{"value": "time"}]}]}
    assert "time" not in names_of(dictionary) and "zone" in names_of(dictionary)


def test_the_row_a_setting_has_in_the_settings_card_is_the_settings_and_its_name_is_checked():
    dictionary = {"things": [{"card": "settings", "name": "idle_rpm"}], "settings": [{"name": "idle_rpm"}]}
    assert "idle_rpm" in names_of(dictionary)


# ---- the table of texts says words, and names the board's things only as the keys of the words it has for them ----
WORDS = {
    "rows": {"engine/rpm": {"label": "Speed", "says": [["number", "{:.0} rpm"]]}},
    "now": {"engine/rpm": {"as": "speed", "spec": "volts"}, "engine/temp=hot": {"as": "hot one"}},
    "settings": {"idle_rpm": {"label": "Idle", "line": {"words": "a miss of {miss} moves it {x}", "late": "not back", "verb": "moves", "way": "up", "miss": 10, "near": 3}}},
}


def test_a_table_of_words_that_names_the_board_only_by_keys_says_no_relation():
    assert relations_in_texts(WORDS) == []


def test_a_list_of_the_rows_a_card_is_made_of_is_a_relation_and_so_is_a_row_named_beside_another():
    listed = {**WORDS, "now": [{"card": "engine", "row": "rpm", "as": "speed"}]}
    assert relations_in_texts(listed) == ["now"], "a list of rows in the table's order is the board's plan"
    beside = {**WORDS, "now": {"engine/rpm": {"as": "speed", "also": {"row": "oil_temp", "spec": "volts"}}}}
    assert relations_in_texts(beside) == ["now.engine/rpm.also"]


def test_a_line_that_names_the_settings_and_numbers_it_is_worked_out_of_is_a_relation_and_its_words_and_constants_are_not():
    named = {**WORDS, "settings": {"idle_rpm": {"line": {"kind": "ramp", "step": "idle_rpm", "min": "low_rpm", "words": "x", "miss": 10}}}}
    assert sorted(relations_in_texts(named)) == ["settings.idle_rpm.line.kind", "settings.idle_rpm.line.min", "settings.idle_rpm.line.step"]


def test_a_table_of_numbers_of_the_board_by_a_key_is_a_relation():
    assert relations_in_texts({**WORDS, "numbers": {"steepness": ["engine", "holding", "steepness"]}}) == ["numbers"]
