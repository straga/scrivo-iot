"""Every number a person types has its unit beside it: the page is asked, tab by tab, what stands next to each numeric field.

    uv run --no-project --with websocket-client python tools/page_units.py [--url http://127.0.0.1:8792/] [--lang ru] [--shots DIR]

Run tools/page_stand.py first (its own port: `--port 8792`).

WHAT IS BEING ASKED. The board sends a unit as a code from its dictionary (`Kind::Whole { unit }`, seconds for
`Kind::Seconds`); the words of the codes are the menu's `units`, in the language the page speaks. A field with a bare
number - "Hold the draught 31", "Fan, not below 8" - asks the person what they are changing. The verdicts, over every
numeric field on every tab, the sections the folds close opened:

  unit-beside-field  every numeric field is followed by its unit, and the unit is the word the dictionary's unit has in the
                     page's language; a field whose setting has no unit is named in EXEMPT, with the reason, or it fails
  unit-has-word      every unit on the screen is a word of the menu's `units` - a code the menu has no word for would
                     stand as itself (`m%/%`)
  no-raw-code        no text of any tab holds a unit code of the board's dictionary that the menu words
  fits-the-phone     no view is wider than the 375 px of the phone: a unit beside a field must not push a line out of the page

Exits 1 when a check failed, 0 when all passed, 2 when the run did not take place (no Chrome, no page, no tab): a run that
did not take place says nothing of the page, and is told apart from a failed check by its code.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from page_sources import merged_menu  # noqa: E402


PORT = 9427


class NotRun(Exception):
    """The run did not take place: a browser that did not come up, a tab that is not there. Not a verdict, and never a pass."""


def page_url(port, tries=50, pause=0.2, opener=urllib.request.urlopen, sleep=time.sleep):
    """The debugging address of the page tab of the Chrome on PORT, asked TRIES times PAUSE apart; NotRun, in words, when the browser never answers or has no page."""
    for _ in range(tries):
        try:
            targets = json.load(opener("http://127.0.0.1:%d/json" % port))
        except (OSError, ValueError):
            sleep(pause)
            continue
        for target in targets:
            if target.get("type") == "page":
                return target["webSocketDebuggerUrl"]
        sleep(pause)
    raise NotRun("Chrome did not show a page on port %d in %d looks" % (port, tries))

# Numbers that stand without a unit, and why: a count and an address are plain numbers, and a unit written after them would be a word made up to fill the place. A new number with no unit is a decision, not a gap that passes.
EXEMPT = {
    "ntc_address": "the Modbus address of the temperature module: a place on a bus, not a quantity",
    "auto_tries": "how many attempts at lighting: a count of tries, which is the whole of what it says",
    "net_point_tries": "how many failed joins in a row leave an access point: a count, and the label says what is counted",
    "net_reset_laps": "how many laps without a network reset the radio: a count of laps, and the label says so",
    "net_reboot_laps": "how many laps after the reset restart the board: a count of laps, and the label says so",
}

# What the browser holds of each numeric field: which setting (or window field) it is and the words standing right after it
# in the unit's place. A unit is the element marked `data-unit`; a field with none after it has none.
FIELDS = """(function(){
  return JSON.stringify(Array.from(document.querySelectorAll('input[inputmode="numeric"], input[type="range"]')).map(i => {
    const next = i.nextElementSibling;
    // a slider's unit is a Unit like any other, in the row of the slider: the element marked `data-unit` beside its number
    const slider = i.type === 'range';
    const mark = slider ? i.parentElement.querySelector('[data-unit]') : (next && next.hasAttribute('data-unit') ? next : null);
    const unit = mark ? mark.textContent.trim() || null : null;
    return {name: i.dataset.name || '', field: i.dataset.field || '', kind: slider ? 'slider' : 'number', unit, shown: i.offsetParent !== null};
  }));
})()"""

# Every text the tab shows, for the one scan that no unit code stands raw; and the codes the board's dictionary gives its settings, as the board sends them.
TEXT = "document.body.innerText"
WIDTH = 375
WIDER = "document.documentElement.scrollWidth"
CODES = """(function(){
  const x = new XMLHttpRequest(); x.open('GET', '/dictionary', false); x.send();
  return JSON.stringify(Array.from(new Set(JSON.parse(x.responseText).settings.map(s => s.unit).filter(Boolean))));
})()"""


def unit_problems(fields, settings, words, exempt):
    """What is wrong with the numeric FIELDS of the page, in words. SETTINGS is the page's own settings by name - the
    `unit` of each already the word of the language -, WORDS the words the menu gives the units in that language, EXEMPT the
    names of the settings that stand without a unit, with the reason. A field that is not a setting (a window's hours and
    minutes) must still have a unit, and a word of the menu. Each problem is a pair: `missing` (a unit that is not there, or not the dictionary's)
    or `no-word` (a unit the menu has no word for)."""
    out = []
    for f in fields:
        label = f["name"] or "%s %s" % ("slider" if f.get("kind") == "slider" else "window field", f["field"] or "?")
        setting = settings.get(f["name"]) if f["name"] else None
        if setting is not None:
            want = setting.get("unit") or ""
            if not want:
                if f["name"] not in exempt:
                    out.append(("missing", "%s has no unit in the dictionary and is not named as a number without one" % label))
                elif f["unit"]:
                    out.append(("missing", "%s is named as a number without a unit and stands with '%s'" % (label, f["unit"])))
                continue
            if f["unit"] != want:
                out.append(("missing", "%s: the unit beside the field is %s, the dictionary's is '%s'" % (label, "'%s'" % f["unit"] if f["unit"] else "missing", want)))
        elif not f["unit"]:
            out.append(("missing", "%s: no unit beside the field" % label))
        if f["unit"] and f["unit"] not in words:
            out.append(("no-word", "%s: '%s' is not a word of the menu's units" % (label, f["unit"])))
    return out


def raw_in(text, code):
    """The words around the first place CODE stands in TEXT as a whole token, or None."""
    found = re.search(r"(?<![\w%%/])%s(?![\w%%/])" % re.escape(code), text)
    return text[max(0, found.start() - 24):found.end() + 12].replace("\n", " | ") if found else None


def raw_codes(text, codes, words):
    """The unit CODES of the board's dictionary that the menu words and that stand in TEXT as themselves. A code the menu words
    as itself (°C, %) is the word and is not raw. A code is raw as a whole token: `min` is not in `administrator`."""
    return sorted(c for c in codes if c not in words and re.search(r"(?<![\w%%/])%s(?![\w%%/])" % re.escape(c), text))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://127.0.0.1:8792/")
    ap.add_argument("--lang", default="ru", choices=["ru", "en"])
    ap.add_argument("--shots", default=None, help="a folder to put a picture of every view in, for the eyes")
    args = ap.parse_args()

    from chrome_page import CHROME, Page  # noqa: E402
    from stand_frames import drawn_js  # noqa: E402

    menu = merged_menu("burner")
    word = lambda node: node[args.lang]
    words = {w[args.lang] for w in menu["units"].values()}
    views = [("scheme", None), ("modules", None)] + [("modules", page) for page in menu["tree"]["modules"]["pages"]] + [("system", None)]

    profile = tempfile.mkdtemp(prefix="page_units_")
    chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % PORT, "--user-data-dir=%s" % profile,
                               "--no-first-run", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    problems, raw, seen, wide = [], set(), [], []
    try:
        p = Page(page_url(PORT))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=900, deviceScaleFactor=1, mobile=True)
        p.call("Page.navigate", url=args.url)
        p.settle(1.0)
        p.js("try { localStorage.setItem('lang', %s) } catch (e) {}" % json.dumps(args.lang))
        p.call("Page.navigate", url=args.url)
        end = time.time() + 15
        while time.time() < end and not p.js("!!document.querySelector('button')"):
            time.sleep(0.25)
        p.settle(2.0)
        settings = {s["name"]: s for s in json.loads(p.js(drawn_js("settings")))}
        codes = set(json.loads(p.js(CODES)))

        press = lambda text: p.js("(function(t){for (const b of document.querySelectorAll('button')) if (b.textContent.trim() === t) { b.click(); return 1 } return 0})(%s)" % json.dumps(text))
        for tab, page in views:
            if not press(word(menu["ui"][tab])):
                raise NotRun("no tab '%s' on the page" % word(menu["ui"][tab]))
            p.settle(0.8)
            if page is not None and not press(word(page["title"])):
                raise NotRun("no page '%s' on the tab" % word(page["title"]))
            p.settle(0.8)
            p.js("Array.from(document.querySelectorAll('button[aria-expanded=\"false\"]')).forEach(b => b.click()); 1")
            p.settle(0.8)
            # a window of the schedule draws its hours and minutes only when it is "every": ask each so, unsaved - nothing is pressed that sends
            p.js("Array.from(document.querySelectorAll('select[data-field=\"kind\"]')).forEach(s => { s.value = 'every'; s.dispatchEvent(new Event('change', {bubbles: true})) }); 1")
            p.settle(0.5)
            here = [f for f in json.loads(p.js(FIELDS)) if f["shown"]]
            where = "%s%s" % (tab, ">" + page["id"] if page else "")
            if args.shots:
                os.makedirs(args.shots, exist_ok=True)
                p.shot(os.path.join(args.shots, "%s_%s.png" % (args.lang, where.replace(">", "-"))), full=True)
            seen += [(where, f) for f in here]
            problems += [(kind, "%s / %s" % (where, said)) for kind, said in unit_problems(here, settings, words, EXEMPT)]
            if p.js(WIDER) > WIDTH:
                wide.append("%s is %d px wide" % (where, p.js(WIDER)))
            text = p.js(TEXT)
            for code in raw_codes(text, codes, words):
                raw.add("%s (%s: ...%s...)" % (code, where, raw_in(text, code)))
    finally:
        chrome.terminate()

    fields = [f for _, f in seen]
    if not fields:
        raise NotRun("not one numeric field on any tab")
    named = sorted({f["name"] for f in fields if f["name"] in EXEMPT})
    numbers = {n for n, one in settings.items() if one.get("seconds") or one.get("months") or one.get("whole")}
    unseen = sorted(numbers - {f["name"] for f in fields})
    print("   %-18s the board sends %d number settings, %d have a field on a view%s" % ("coverage", len(numbers), len(numbers) - len(unseen), "; none drawn: " + ", ".join(unseen) if unseen else ""))
    print("   %-18s %d numeric fields on %d views, %d of them without a unit by name: %s" % ("fields", len(fields), len(views), len(named), ", ".join(named) or "-"))
    missing = [said for kind, said in problems if kind == "missing"]
    nowords = [said for kind, said in problems if kind == "no-word"]
    cut = lambda l: "; ".join(l[:8]) + (" (%d more)" % (len(l) - 8) if len(l) > 8 else "")
    verdicts = [("unit-beside-field", not missing, cut(missing) if missing else "every field has its unit"),
                ("unit-has-word", not nowords, cut(nowords) if nowords else "every unit is a word of the menu's"),
                ("fits-the-phone", not wide, "; ".join(wide) if wide else "no view wider than %d px" % WIDTH),
                ("no-raw-code", not raw, "no unit code stands raw" if not raw else "raw codes on the screen: %s" % ", ".join(sorted(raw)))]
    failed = 0
    for check, ok, said in verdicts:
        print("   %-18s %s  %s" % (check, "PASS" if ok else "FAIL", said))
        failed += not ok
    print("%d check(s) failed" % failed)
    return 1 if failed else 0


def cli():
    """main, with a run that did not take place told in words and by the code 2."""
    try:
        return main()
    except NotRun as why:
        print("page_units: %s - the run did not take place; nothing is said of the page" % why, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(cli())
