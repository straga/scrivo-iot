"""tools/page_controls.py: where it goes to find a setting on the page - by the menu's tree, never by a tab written in the tool. Made-up menu, so the words are the menu's and no phrase of the burner is held."""
import pytest

from page_controls import where_to_find

MENU = {
    "ui": {"scheme": {"en": "Main"}, "modules": {"en": "Parts"}, "system": {"en": "Box"}},
    "tree": {
        "main": {"sections": [{"id": "a", "title": {"en": "Alpha"}, "settings": ["x", "shared"]}]},
        "system": {"sections": [{"id": "t", "title": {"en": "Tau"}, "settings": ["clock"]}]},
        "modules": {"groups": [], "pages": [
            {"id": "m", "title": {"en": "Motor"}, "sections": [{"id": "b", "title": {"en": "Beta"}, "settings": ["y", "shared"]}, {"id": "c", "settings": ["z"]}]},
        ]},
    },
}


def test_a_setting_only_the_main_screen_holds_is_found_on_the_scheme_tab_in_its_section():
    assert where_to_find(MENU, "x") == {"tab": "Main", "module": None, "section": "Alpha"}


def test_a_setting_of_a_module_is_found_on_the_modules_tab_on_the_page_of_its_module():
    assert where_to_find(MENU, "y") == {"tab": "Parts", "module": "Motor", "section": "Beta"}


def test_a_section_with_no_title_is_found_by_its_page_alone():
    assert where_to_find(MENU, "z") == {"tab": "Parts", "module": "Motor", "section": None}


def test_a_setting_of_the_board_is_found_on_the_system_tab_with_no_module_between():
    assert where_to_find(MENU, "clock") == {"tab": "Box", "module": None, "section": "Tau"}


def test_a_setting_the_main_screen_shows_again_is_found_on_its_modules_page_first():
    assert where_to_find(MENU, "shared") == {"tab": "Parts", "module": "Motor", "section": "Beta"}


def test_a_setting_the_tree_does_not_hold_is_refused_in_words_and_not_looked_for_on_any_tab():
    with pytest.raises(LookupError) as refusal:
        where_to_find(MENU, "nowhere")
    assert "nowhere" in str(refusal.value) and "tree" in str(refusal.value)
