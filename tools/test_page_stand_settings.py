"""The stand starts from the settings a scenario was played with: the frames of a burner that went down three steps and a page that says one step contradict each other on one screen.
Expected from what a person reads on that page - the number each setting of the scenario names - and from what the stand must not do: touch a setting the scenario does not name, or take a name that is no setting.

    uv run --no-project --with pytest pytest -q tools/test_page_stand_settings.py
"""
import pytest

from page_stand import with_settings

DICTIONARY = {
    "contract": 6, "version": "k+1", "board": "B",
    "things": [{"card": "system", "name": "uptime", "kinds": ["seconds"]}]
    + [{"card": "settings", "name": n, "kinds": ["text"]} for n in ("auto_step", "auto_step_mid", "auto_mid_c")]
    + [{"card": "thermostat", "name": "rung", "kinds": ["word"]}],
    "settings": [{"name": "auto_step", "default": "6"}, {"name": "auto_step_mid", "default": "chosen"}, {"name": "auto_mid_c", "default": "8"}],
}
STATE = {"values": [[0, {"seconds": 5}], [1, {"text": "6"}], [2, {"text": "chosen"}], [3, {"text": "8"}]]}


def texts(state):
    return {at: value["text"] for at, value in state["values"] if isinstance(value, dict) and "text" in value}


def test_a_setting_the_scenario_names_is_written_at_its_own_address_and_the_others_stand():
    now = with_settings(DICTIONARY, STATE, {"auto_step_mid": "5", "auto_mid_c": "10"})
    assert texts(now) == {1: "6", 2: "5", 3: "10"}
    assert [at for at, _ in now["values"]] == [0, 1, 2, 3], "no address was added or lost"
    assert now["values"][0] == [0, {"seconds": 5}], "a row that is no setting moved"


def test_the_state_it_was_given_is_not_changed():
    with_settings(DICTIONARY, STATE, {"auto_step_mid": "5"})
    assert texts(STATE) == {1: "6", 2: "chosen", 3: "8"}


def test_a_scenario_with_no_settings_leaves_the_state_as_it_is():
    assert with_settings(DICTIONARY, STATE, {}) == STATE


def test_a_name_that_is_no_setting_of_the_dictionary_is_refused_by_the_name():
    with pytest.raises(ValueError, match="auto_nothing"):
        with_settings(DICTIONARY, STATE, {"auto_nothing": "1"})


def test_a_row_of_another_card_with_the_name_of_a_setting_is_not_taken_for_one():
    with pytest.raises(ValueError, match="rung"):
        with_settings(DICTIONARY, STATE, {"rung": "big"})
