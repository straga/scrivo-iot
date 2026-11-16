"""tools/page_units.py: what is judged a unit beside a number. Made-up settings and words, so no phrase of the burner is held."""
import io
import json

import pytest

from page_units import NotRun, page_url, raw_codes, unit_problems

WORDS = {"Pa", "s", "ratio per miss"}
SETTINGS = {
    "hold": {"name": "hold", "unit": "Pa"},
    "gain": {"name": "gain", "unit": "ratio per miss"},
    "address": {"name": "address", "unit": ""},
}
EXEMPT = {"address": "a place on a bus"}


def field(name="", unit=None, **more):
    return {"name": name, "field": more.get("field", ""), "kind": more.get("kind", "number"), "unit": unit, "shown": True}


def kinds(fields, exempt=EXEMPT, settings=SETTINGS):
    return [kind for kind, _ in unit_problems(fields, settings, WORDS, exempt)]


def test_a_field_with_the_unit_of_its_setting_beside_it_is_in_order():
    assert unit_problems([field("hold", "Pa"), field("gain", "ratio per miss")], SETTINGS, WORDS, EXEMPT) == []


def test_a_field_with_no_unit_beside_it_is_named():
    said = unit_problems([field("hold", None)], SETTINGS, WORDS, EXEMPT)
    assert [k for k, _ in said] == ["missing"] and "hold" in said[0][1]


def test_a_unit_that_is_not_the_one_of_the_setting_is_named():
    assert kinds([field("hold", "s")]) == ["missing"]


def test_a_unit_the_menu_has_no_word_for_is_named_as_such():
    raw = {"raw": {"name": "raw", "unit": "m%/%"}}
    assert kinds([field("raw", "m%/%")], settings=raw) == ["no-word"]


def test_a_number_the_dictionary_gives_no_unit_stands_without_one_only_when_it_is_named():
    assert kinds([field("address", None)]) == []
    assert kinds([field("address", None)], exempt={}) == ["missing"]


def test_a_number_named_as_without_a_unit_that_has_one_is_named():
    assert kinds([field("address", "s")]) == ["missing"]


def test_a_field_that_is_no_setting_still_needs_a_unit_of_the_menu():
    assert kinds([field(field="every", unit="s")]) == []
    assert kinds([field(field="every", unit=None)]) == ["missing"]
    assert kinds([field(field="every", unit="moons")]) == ["no-word"]


def test_a_code_the_menu_words_as_something_else_is_raw_when_it_stands_in_the_text():
    assert raw_codes("hold 31 m%/%", {"m%/%", "Pa"}, WORDS) == ["m%/%"]


def test_a_code_the_menu_words_as_itself_is_the_word_and_is_not_raw():
    assert raw_codes("hold 31 Pa", {"Pa"}, WORDS) == []


def test_a_code_inside_a_longer_word_is_not_raw():
    assert raw_codes("administrator, minimum", {"min"}, WORDS) == []


def test_a_slider_with_a_word_of_the_menu_after_its_number_is_in_order():
    assert kinds([field(kind="slider", unit="Pa")]) == []


def test_a_slider_with_no_unit_after_its_number_is_named_as_a_slider():
    said = unit_problems([field(kind="slider", unit=None)], SETTINGS, WORDS, EXEMPT)
    assert [k for k, _ in said] == ["missing"] and "slider" in said[0][1]


def test_a_slider_with_a_unit_the_menu_has_no_word_for_is_named_as_such():
    assert kinds([field(kind="slider", unit="m%/%")]) == ["no-word"]


# ---- a run that did not take place ----

def asked(answers):
    """An opener that answers each look with the next of ANSWERS: an exception is raised, a list of targets is served."""
    left = list(answers)

    def opener(url):
        one = left.pop(0) if left else OSError("no more")
        if isinstance(one, Exception):
            raise one
        return io.StringIO(json.dumps(one))
    return opener


def test_a_chrome_that_never_answers_is_a_run_that_did_not_take_place_and_not_a_crash():
    with pytest.raises(NotRun) as said:
        page_url(9999, tries=3, pause=0, opener=asked([]), sleep=lambda s: None)
    assert "9999" in str(said.value) and "Chrome" in str(said.value)


def test_a_chrome_with_no_page_tab_is_a_run_that_did_not_take_place():
    with pytest.raises(NotRun):
        page_url(9999, tries=2, pause=0, opener=asked([[{"type": "service_worker"}]] * 2), sleep=lambda s: None)


def test_a_chrome_that_answers_after_a_look_or_two_gives_the_address_of_its_page():
    targets = [{"type": "browser"}, {"type": "page", "webSocketDebuggerUrl": "ws://x/1"}]
    assert page_url(9999, tries=3, pause=0, opener=asked([OSError("not yet"), targets]), sleep=lambda s: None) == "ws://x/1"
