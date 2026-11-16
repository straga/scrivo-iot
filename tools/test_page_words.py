"""The engine's menu and table of texts (page/menu.json, page/texts.json) say nothing of a particular device.

    uv run --no-project --with pytest pytest tools/test_page_words.py

Every device that carries the engine's page shows these words: a Sonoff with no module yet must not be told it is a burner. A device's own words are
its page folder's (devices/<device>/page/menu.json), laid over these by tools/page_kit.py. What a person reads is the product, so the words are
read as data - every text and every key of both files - and the stems below are the names of the plants the engine has met.
"""
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# The things a particular plant is called, in both languages the page speaks. A stem, not a word: `boiler` and `boilers`, and the same for the Russian words (a word and its case forms).
DEVICE_STEMS = ["burner", "boiler", "pellet", "auger", "igniter", "flue", "draught", "flame",
                "горелк", "горелоч", "котёл", "котел", "котл", "пеллет", "шнек", "розжиг", "дымоход", "пламя", "пламен"]
FOUND = re.compile("|".join(DEVICE_STEMS), re.IGNORECASE)


def strings_of(whole, at=""):
    """Every text and every key of WHOLE with its path: [(path, text)]."""
    if isinstance(whole, dict):
        for key, value in whole.items():
            yield (at + "." + key, key)
            yield from strings_of(value, at + "." + key)
    elif isinstance(whole, list):
        for number, value in enumerate(whole):
            yield from strings_of(value, "%s[%d]" % (at, number))
    elif isinstance(whole, str):
        yield (at, whole)


def device_words(whole):
    """The places of WHOLE that name a plant: [(path, text)]."""
    return [(path, text) for path, text in strings_of(whole) if FOUND.search(text)]


def test_a_text_that_names_a_plant_is_found_in_both_languages_and_a_neutral_one_is_not():
    menu = {"ui": {"noModules": {"en": "This burner has no module yet.", "ru": "В этой горелке пока нет модулей."},
                   "save": {"en": "Save", "ru": "Сохранить"}},
            "help": {"entries": {"zone": {"text": {"en": "The city.", "ru": "Нужно у котла."}}}}}
    assert [path for path, _ in device_words(menu)] == [".ui.noModules.en", ".ui.noModules.ru", ".help.entries.zone.text.ru"]
    assert device_words({"ui": {"noModules": {"en": "This device has no module yet.", "ru": "У этого устройства пока нет модулей."}}}) == []


def test_a_key_that_names_a_plant_is_found_too():
    assert device_words({"rows": {"burner/stage": {"label": "stage"}}}) == [(".rows.burner/stage", "burner/stage")]


def test_the_engines_menu_says_nothing_of_a_particular_device():
    whole = json.load(open(os.path.join(ROOT, "page", "menu.json"), encoding="utf-8"))
    assert device_words(whole) == []


def test_the_engines_table_of_texts_says_nothing_of_a_particular_device():
    whole = json.load(open(os.path.join(ROOT, "page", "texts.json"), encoding="utf-8"))
    assert device_words(whole) == []
