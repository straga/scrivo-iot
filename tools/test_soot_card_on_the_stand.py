"""The card of the boiler's cleanliness as a person sees it, on the burner's real page behind the stand: its rows and its button, read from the browser's drawing.

Expected from what a person reads, not from the page's code: with the median of the flue over the water at 39.5 degrees, the edges of the scale at their defaults (36 clean, 51 dirty) and 29.6 kilograms burned since the
cleaning of 08.11 (day 20765) the card says 77 % clean, 29.6 kg since the cleaning and `08.11` as the day of it - each value alone, a number with its unit or a date, never a phrase (REF_Device_Page_Look, rule 7) - and has a
button to press after a cleaning. It has no row of the flue's difference over the water: a person could not tell what that number says, so it stands in the help of the row, in words, with the two ends of the scale.
With no burn counted yet it has the same rows and says a dash where there is no number. A boiler under a quarter clean is asked to be cleaned in the card of what needs a person, by the percent. The kilograms and the day of the
scenario are those `.33` showed on 16.11; the 39.5 degrees is what the count of the window says over its record of that day.

    uv run --no-project --with pytest --with websocket-client --with pyyaml pytest tools/test_soot_card_on_the_stand.py
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from chrome_page import CHROME, Page  # noqa: E402
from page_steady_modules import STATE  # noqa: E402

CHROME_PORT = 9421
STAND_PORT = 8801

READ = """(function(){
  const headings = Array.from(document.querySelectorAll('main h2')).map((h) => h.textContent.trim());
  const attention = Array.from(document.querySelectorAll('[data-what]')).map((e) => e.textContent.trim());
  const card = document.querySelector('section[data-card="soot"]');
  if (!card) return JSON.stringify({headings: headings, attention: attention, card: null});
  const rows = Array.from(card.querySelectorAll('[data-soot-row]')).map((r) => r.getAttribute('data-soot-row'));
  const labels = {};
  card.querySelectorAll('[data-soot-row]').forEach((r) => { labels[r.getAttribute('data-soot-row')] = r.querySelector('span').textContent.trim(); });
  const values = {};
  card.querySelectorAll('[data-soot-value]').forEach((v) => { values[v.getAttribute('data-soot-value')] = v.textContent.trim(); });
  const buttons = Array.from(card.querySelectorAll('button')).map((b) => b.textContent.trim());
  return JSON.stringify({headings: headings, attention: attention, card: {rows: rows, labels: labels, values: values, buttons: buttons}});
})()"""

# the entries of the help: the name and the text of each, as a person reads them after opening the tab
HELP = """JSON.stringify(Array.from(document.querySelectorAll('main section div.py-2')).map((d) => ({name: d.children[0].textContent.trim(), text: d.children[1].textContent.trim()})))"""
OPEN = "(function(w){for (const b of document.querySelectorAll('nav button')) if (b.textContent.trim() === w) { b.click(); return 1 } return 0})(%s)"
REFERENCE = {"en": "Reference", "ru": "Помощь"}


def frames_for(scenario, work):
    """The stand's frames of SCENARIO (a dict), made by the burner's own API crate, in a file of WORK."""
    path = os.path.join(work, "scenario.json")
    with open(path, "w") as f:
        json.dump(scenario, f)
    out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path], cwd=ROOT, capture_output=True, text=True)
    assert out.returncode == 0, "the frames were not made:\n" + out.stderr[-1500:]
    frames = os.path.join(work, "frames.json")
    with open(frames, "w") as f:
        f.write(out.stdout)
    return frames


def card_of(scenario, lang="en"):
    """What the browser draws on the burner's page in LANG, with SCENARIO played on the stand: {headings: the headings of the screen, attention: the lines of the card of what needs a person,
    card: None when there is no card of the soot, else {rows, labels, values, buttons}, help: [{name, text}] of the tab of the help}."""
    work = tempfile.mkdtemp(prefix="soot_card_")
    stand = chrome = None
    try:
        frames = frames_for(scenario, work)
        stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--device", "burner", "--port", str(STAND_PORT), "--scenario", frames, "--at", "0"],
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % CHROME_PORT, "--user-data-dir=%s" % os.path.join(work, "profile"), "--no-first-run", "about:blank"],
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        targets = None
        for _ in range(100):
            try:
                urllib.request.urlopen("http://127.0.0.1:%d/state" % STAND_PORT, timeout=1)
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % CHROME_PORT, timeout=1))
                break
            except OSError:
                time.sleep(0.3)
        assert targets, "the stand or Chrome did not come up"
        page = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        page.call("Page.enable")
        page.call("Emulation.setDeviceMetricsOverride", width=375, height=900, deviceScaleFactor=1, mobile=True)
        page.call("Page.navigate", url="http://127.0.0.1:%d/?lang=%s" % (STAND_PORT, lang))
        end = time.time() + 20
        while time.time() < end and not page.js("!!document.querySelector('h1') && document.body.innerText.length > 200"):
            time.sleep(0.3)
        page.settle(2.0)
        said = json.loads(page.js(READ))
        page.js(OPEN % json.dumps(REFERENCE[lang]))
        page.settle(1.0)
        said["help"] = json.loads(page.js(HELP))
        return said
    finally:
        for process in (chrome, stand):
            if process:
                process.terminate()
                process.wait(timeout=10)
        shutil.rmtree(work, ignore_errors=True)


COUNTED = {"seconds": 10, "soot": {"soot_c": 39.5, "burns": 6, "grams": 29600, "cleaned_day": 20765}}
# 47.5 °C over the water with the edges at their defaults is 23 % clean: under a quarter, so the card of what needs a person asks for the cleaning
DIRTY = {"seconds": 10, "soot": {"soot_c": 47.5, "burns": 6, "grams": 29600, "cleaned_day": 20765}}
UNCOUNTED = {"seconds": 10, "soot": {"soot_c": None, "burns": 0, "grams": 0, "cleaned_day": None}}


@pytest.fixture(scope="module")
def counted():
    return card_of(COUNTED)


@pytest.fixture(scope="module")
def counted_ru():
    return card_of(COUNTED, "ru")


@pytest.fixture(scope="module")
def dirty():
    return card_of(DIRTY)


@pytest.fixture(scope="module")
def dirty_ru():
    return card_of(DIRTY, "ru")


@pytest.fixture(scope="module")
def uncounted():
    return card_of(UNCOUNTED)


def test_the_page_is_drawn_and_has_the_heading_of_the_cleanliness(counted):
    # a page that did not come up shows no card either: this tells that apart from a card that is not drawn
    assert "Boiler cleanliness" in counted["headings"], counted["headings"]


def test_the_card_of_a_counted_boiler_has_three_rows_and_the_button_and_none_of_the_flue(counted):
    card = counted["card"]
    assert card is not None, "the page draws no card of the soot; its headings: %s" % counted["headings"]
    assert card["rows"] == ["clean", "since", "cleaned"]
    assert card["buttons"] == ["Cleaned"]


def test_the_rows_say_the_percent_the_kilograms_and_the_day_of_the_cleaning_each_alone(counted):
    assert counted["card"]["values"] == {"percent": "77 %", "since": "29.6 kg", "cleaned": "08.11"}


def test_the_labels_of_the_rows_say_what_each_is_in_both_languages(counted, counted_ru):
    assert counted["card"]["labels"] == {"clean": "Clean", "since": "Since the cleaning", "cleaned": "Last cleaning"}
    assert counted_ru["card"]["labels"] == {"clean": "Чистота котла", "since": "С чистки", "cleaned": "Последняя чистка"}


def test_a_boiler_with_no_burn_counted_has_the_same_rows_and_says_a_dash_for_what_it_does_not_know(uncounted):
    card = uncounted["card"]
    assert card is not None, "the page draws no card of the soot; its headings: %s" % uncounted["headings"]
    assert card["rows"] == ["clean", "since", "cleaned"]
    assert card["values"] == {"percent": "-", "since": "0.0 kg", "cleaned": "-"}
    assert card["buttons"] == ["Cleaned"]


def phrases(values):
    """The rows whose value is no word, dash or number with its unit - the form rule 7 holds every value of a card to (tools/page_steady_modules.py, STATE): {row: value}."""
    return {row: said for row, said in values.items() if not STATE.match(said)}


def test_every_value_of_the_card_is_a_word_a_dash_or_a_number_with_its_unit(counted, uncounted):
    assert phrases(counted["card"]["values"]) == {}
    assert phrases(uncounted["card"]["values"]) == {}


def test_the_form_rule_catches_a_value_that_is_a_phrase():
    # the shape the card had: the kilograms with the day after a comma and the word of the label said again
    assert phrases({"since": "29.6 kg, since 08.11", "percent": "77 %"}) == {"since": "29.6 kg, since 08.11"}
    assert phrases({"since": "29.6 кг, с 08.11"}) == {"since": "29.6 кг, с 08.11"}


def test_a_dirty_boiler_is_asked_to_be_cleaned_by_the_percent_it_stands_at(dirty, dirty_ru):
    assert dirty["attention"] == ["The boiler needs cleaning: cleanliness 23 %"]
    assert dirty_ru["attention"] == ["Котёл пора чистить: чистота 23 %"]


def test_a_boiler_that_is_clean_enough_is_not_asked_to_be_cleaned(counted):
    assert counted["attention"] == []


def scale_help(said, title):
    """The entries of the help named TITLE that give the two ends of the scale, 36 and 51 °C."""
    return [e["text"] for e in said["help"] if e["name"] == title and "36 °C" in e["text"] and "51 °C" in e["text"]]


def test_the_help_of_the_cleanliness_says_what_the_number_is_made_of_and_the_ends_of_the_scale(counted, counted_ru):
    # the difference of the flue over the water is no row of the card: it is said here, in words, with the ends it is measured between
    (en,) = scale_help(counted, "Boiler cleanliness")
    assert "hotter" in en and "flue gas" in en and "water" in en
    (ru,) = scale_help(counted_ru, "Чистота котла")
    assert "горячее воды" in ru
