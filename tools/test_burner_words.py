"""The words of the burner's page never call the difference of the flue over the water "a gas": a person asked "what gas?" of the row whose Russian label says "gases over the boiler", and the number the page means is how much hotter the flue runs than the water.

    uv run --no-project --with pytest pytest tools/test_burner_words.py

The menu and the table of texts of the burner, as the kit carries them (the engine's with the burner's laid over, tools/page_sources.py), are read as data - every text of both - and a text that says the difference by a gas is named
with its path. What stays: the thermocouple of the flue gases (the Russian for flue gases, `flue gas` as the sensor's name and what it reads), and the sentence that says what the number is made of - how much hotter the flue gas runs than the water -,
because the gas there is the thing measured and the difference is said with the word `hotter`.
"""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from page_sources import merged_menu, merged_texts  # noqa: E402

# a gas standing for a DIFFERENCE: "gases over the boiler", "gases minus water", "the difference of gases and water", "gases {n} °C higher". "Flue minus water" says it right and is not caught: the flue is the thing measured and
# the minus is the difference.
DIFFERENCE_BY_A_GAS = re.compile(
    r"газ\w*\s+(минус|над|выше)|разниц\w*\s+газ|газ\w*\s+и\s+вод|газ\w*\s+на\s+\{|выше чистого"
    r"|flue (gas )?over (the )?water|between the flue( gas)? and the water|flue is \{n\} °C over",
    re.IGNORECASE)


def strings_of(whole, at=""):
    """Every text of WHOLE with its path: [(path, text)]."""
    if isinstance(whole, dict):
        for key, value in whole.items():
            yield from strings_of(value, at + "." + key)
    elif isinstance(whole, list):
        for number, value in enumerate(whole):
            yield from strings_of(value, "%s[%d]" % (at, number))
    elif isinstance(whole, str):
        yield (at, whole)


def by_a_gas(whole):
    """The places of WHOLE that say a difference by a gas: [(path, text)]."""
    return [(path, text) for path, text in strings_of(whole) if DIFFERENCE_BY_A_GAS.search(text)]


def test_the_words_that_asked_a_person_what_gas_are_caught():
    for old in ["Газы над котлом", "газы над котлом", "Чистый котёл: газы минус вода", "Разница газов и воды в стадии работы", "Котёл пора чистить — газы на {n} °C выше чистого",
                "Flue over the water", "The boiler needs cleaning: the flue is {n} °C over a clean one", "The difference between the flue gas and the water in the working stage"]:
        assert by_a_gas({"a": old}), old


def test_the_thermocouple_of_the_flue_gases_and_the_sentence_of_what_the_number_is_made_of_are_not_caught():
    for legit in ["Температура дымовых газов с термопары.", "Сколько секунд молчит термопара дымовых газов.", "дымовые газы",
                  "Считаем по тому, насколько дымовые газы горячее воды во время горения — медиана последних шести горений.",
                  "Counted by how much hotter the flue gas runs than the water during a burn", "Чистый котёл: разница дым − вода", "Clean boiler: flue minus water"]:
        assert not by_a_gas({"a": legit}), legit


def test_the_menu_and_the_texts_of_the_burner_say_no_difference_by_a_gas():
    found = by_a_gas(merged_menu("burner")) + by_a_gas(merged_texts("burner"))
    assert found == [], "\n".join("%s: %s" % one for one in found)


def test_the_two_edges_of_the_scale_are_named_by_the_difference_of_the_flue_and_the_water_and_their_help_says_the_same():
    entries = merged_menu("burner")["help"]["entries"]
    clean, dirty = entries["soot_clean_c"], entries["soot_dirty_c"]
    assert clean["name"] == {"en": "Clean boiler: flue minus water", "ru": "Чистый котёл: разница дым − вода"}
    assert dirty["name"] == {"en": "Dirty boiler: flue minus water", "ru": "Грязный котёл: разница дым − вода"}
    for one in (clean, dirty):
        assert "разница дым − вода" in one["text"]["ru"].lower(), one["text"]["ru"]
        assert "flue minus water" in one["text"]["en"].lower(), one["text"]["en"]
    assert "разница дым − вода" in entries["section:soot_scale"]["text"]["ru"].lower()
    assert "flue minus water" in entries["section:soot_scale"]["text"]["en"].lower()
