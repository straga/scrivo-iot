"""The settings of a card stand in one grid, and a pair stands on one line: the page is asked in a browser, by numbers.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_grid.py [--shots DIR] [--langs ru,en] [--themes l,d] [--page DIR]

Raises the stand (tools/page_stand.py) and a Chrome, opens the page 375 wide in each language and theme, and asks the card of the thermostat's settings - the card that holds the setpoint, on the
page of the thermostat module (the scheme holds no settings) - each verdict printing PASS or FAIL:

  rows-as-menu-says     the card has as many rows as its settings less the second of every pair the menu names, and as many two-field rows as the menu names pairs
  one-column            the FIRST field of every row that holds a field - a single row's, a pair's - begins at one left edge
  one-width-per-kind    the first fields that are numbers are one width, the first fields that are choices one width, and the second fields of the pairs one width
  labels-one-row        no label of any row runs onto a second line or out of the place it has (a label wider than its column runs under the field)
  one-font              every label is of one size, a pair's like the others
  one-row-height        every row that is not a switch is as tall as the next
  pair-on-one-line      the fields of a pair stand on one line, their middles within 6 px
  rows-inside-card      no row runs out of the card, and the page is no wider than the phone

WHAT A PAIR IS is the menu's (`pairs` of the section: `{of: [first, second], ...}`), never the page's by names; the card is found by its setpoint field, not by a title. The verdicts are `judge_*`: pure functions of
what the browser answered, held by tools/test_page_grid.py on answers that are good and answers that are not. Exits 1 when a check failed, 2 when the run did not take place.
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from page_sources import merged_menu  # noqa: E402
WIDTH = 375
STAND_PORT = 8794
CHROME_PORT = 9428
# the card is found by the field of the setpoint: whatever its title says in whatever language
ANCHOR = "auto_hold_c"
# the same tolerances as the other checks of the page: a pixel of rounding is no difference, a line is
EDGE = 1.0
HEIGHT = 1.5
ONE_ROW = 30
SAME_LINE = 6


class NotRun(Exception):
    """The run did not take place: a stand or a browser that did not come up. Not a verdict, and never a pass."""


ASK = """(function(){
  const rect = (el) => { const r = el.getBoundingClientRect(); return {left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height}; };
  const anchor = document.querySelector('input[data-name="%s"]');
  const card = anchor ? anchor.closest('section') : null;
  if (!card) return JSON.stringify(null);
  const root = anchor.closest('div.border-b').parentElement;
  const rows = Array.from(root.children).filter(el => el.matches('div.border-b')).map(row => ({
    box: rect(row),
    over: row.firstElementChild ? row.firstElementChild.scrollWidth > row.firstElementChild.clientWidth + 0.5 : false,
    switch: row.querySelector('button') !== null,
    labels: Array.from(row.querySelectorAll('span')).filter(s => !s.hasAttribute('data-unit') && s.className.includes('text-dim')).map(s => ({text: s.textContent.trim(), box: rect(s), size: parseFloat(getComputedStyle(s).fontSize), fits: s.scrollWidth <= s.clientWidth + 0.5})),
    fields: Array.from(row.querySelectorAll('input[type="text"], select')).filter(f => f.offsetParent !== null).map(f => ({tag: f.tagName.toLowerCase(), name: f.dataset.name || '', box: rect(f)}))
  }));
  return JSON.stringify({card: rect(card), rows, wide: document.documentElement.scrollWidth});
})()""" % ANCHOR


def firsts(said):
    """The first field of every row that holds a field and is not a switch: the fields the one column is about. A row of a switch has buttons and no field of the grid."""
    return [r["fields"][0] for r in said["rows"] if r["fields"] and not r["switch"]]


def seconds(said):
    """The second field of every pair."""
    return [r["fields"][1] for r in said["rows"] if len(r["fields"]) >= 2]


def judge_rows(said, expect):
    """The card has the rows the menu says: its settings less the second of each pair, and as many rows of two fields as pairs. EXPECT: `rows`, `pairs`. -> (ok, words)"""
    rows = said["rows"]
    pairs = [r for r in rows if len(r["fields"]) >= 2]
    out = []
    if len(rows) != expect["rows"]:
        out.append("%d rows, the menu's settings and pairs make %d" % (len(rows), expect["rows"]))
    if len(pairs) != expect["pairs"]:
        out.append("%d rows of two fields, the menu names %d pairs" % (len(pairs), expect["pairs"]))
    return (not out), "; ".join(out) if out else "%d rows, %d of them pairs" % (len(rows), len(pairs))


def judge_one_column(said):
    """The first field of every row begins at one left edge: there must be more than one to say it, a column of one is none. -> (ok, words)"""
    fields = firsts(said)
    if len(fields) < 2:
        return False, "%d first fields: a column needs two" % len(fields)
    lefts = [f["box"]["left"] for f in fields]
    spread = max(lefts) - min(lefts)
    if spread <= EDGE:
        return True, "%d first fields at %.0f px" % (len(fields), lefts[0])
    return False, "the left edges of %d fields are %.0f px apart: %s" % (len(fields), spread, ", ".join("%s %.0f" % (f["name"] or f["tag"], f["box"]["left"]) for f in fields))


def judge_one_width_per_kind(said):
    """The first fields that are numbers are one width, the first fields that are choices one width, and the second fields of the pairs one width. -> (ok, words)"""
    out, seen = [], 0
    for group, kind in (([f for f in firsts(said) if f["tag"] == "input"], "first numbers"), ([f for f in firsts(said) if f["tag"] == "select"], "first choices"), (seconds(said), "second fields")):
        widths = [f["box"]["width"] for f in group]
        seen += len(widths)
        if widths and max(widths) - min(widths) > EDGE:
            out.append("%s are %s px wide" % (kind, "/".join("%.0f" % w for w in widths)))
    if not seen:
        return False, "no field to ask"
    return (not out), "; ".join(out) if out else "one width of each kind"


def judge_labels(said):
    """No label runs onto a second line, and none runs out of the place it has: a label wider than its column is drawn under the field after it. -> (ok, words)"""
    labels = [l for r in said["rows"] for l in r["labels"]]
    out = ["'%s' is %.0f px high" % (l["text"], l["box"]["height"]) for l in labels if l["box"]["height"] > ONE_ROW]
    out += ["'%s' runs out of its place" % l["text"] for l in labels if not l.get("fits", True)]
    return (not out), "; ".join(out) if out else "every label on one line, in its place"


def judge_one_font(said):
    """Every label is of one size, a pair's as the others. -> (ok, words)"""
    sizes = sorted({l["size"] for r in said["rows"] for l in r["labels"]})
    return len(sizes) <= 1, "labels of %s px" % "/".join("%g" % s for s in sizes)


def judge_row_height(said):
    """Every row that is not a switch is as tall as the next. -> (ok, words)"""
    heights = [r["box"]["height"] for r in said["rows"] if not r["switch"]]
    if not heights:
        return False, "no row to ask"
    spread = max(heights) - min(heights)
    return spread <= HEIGHT, ("%.0f px each" % heights[0]) if spread <= HEIGHT else "rows from %.0f to %.0f px high: %s" % (min(heights), max(heights), "/".join("%.0f" % h for h in heights))


def judge_rows_inside(said, width=WIDTH):
    """No row runs out of the card (a line that does not wrap has to fit, or it goes past the edge), and the page is no wider than the phone. -> (ok, words)"""
    out = ["%d row%s run%s out of the card" % (n, "" if n == 1 else "s", "s" if n == 1 else "") for n in [sum(1 for r in said["rows"] if r.get("over"))] if n]
    if said["wide"] > width:
        out.append("the page is %d px wide" % said["wide"])
    return (not out), "; ".join(out) if out else "every row inside the card, %d px wide" % said["wide"]


def judge_pairs_on_a_line(said):
    """The fields of a pair stand on one line: their middles within SAME_LINE px. -> (ok, words)"""
    bad = []
    for r in said["rows"]:
        if len(r["fields"]) < 2:
            continue
        middles = [(f["box"]["top"] + f["box"]["bottom"]) / 2 for f in r["fields"]]
        if max(middles) - min(middles) > SAME_LINE:
            bad.append("/".join("%.0f" % m for m in middles))
    return (not bad), "; ".join("fields at %s" % b for b in bad) if bad else "every pair on one line"


def expectations(menu, names):
    """What the card must have, from the menu: the section whose settings hold the setpoint. NAMES: the settings of the card by the board's names, in the menu's order. -> {"rows", "pairs"}"""
    section = next(s for s in sections(menu) if ANCHOR in sectionNames(s))
    listed = sectionNames(section)
    pairs = [p for p in section.get("pairs", []) if p["of"][0] in listed and p["of"][1] in listed]
    return {"rows": len(listed) - len(pairs), "pairs": len(pairs)}


def sections(menu):
    """Every section of the menu's tree, the main page's and every module page's."""
    found = list(menu["tree"]["main"]["sections"])
    for page in menu["tree"]["modules"]["pages"]:
        found += page.get("sections", [])
    return found


def sectionNames(section):  # noqa: N802 - the page's own name for the same answer (dictionary.sectionNames)
    return list(section.get("settings", []))


def stand_up(page):
    """The stand on its own port, answering; NotRun when it does not."""
    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT)] + (["--page", page] if page else []), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(75):
        try:
            urllib.request.urlopen("http://127.0.0.1:%d/state" % STAND_PORT, timeout=1)
            return stand
        except OSError:
            time.sleep(0.2)
    stand.terminate()
    raise NotRun("the stand did not come up on port %d" % STAND_PORT)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", default=None)
    ap.add_argument("--langs", default="ru,en")
    ap.add_argument("--themes", default="l,d")
    ap.add_argument("--page", default=None, help="the page files to measure instead of the device's own - a revision taken out of git")
    args = ap.parse_args()
    from chrome_page import CHROME, Page

    menu = merged_menu("burner")
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    expect = expectations(menu, None)
    ok = True
    stand = chrome = None
    profile = tempfile.mkdtemp(prefix="page_grid_")
    try:
        stand = stand_up(args.page)
        chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % CHROME_PORT, "--user-data-dir=%s" % profile, "--no-first-run", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        targets = None
        for _ in range(75):
            try:
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % CHROME_PORT))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            raise NotRun("Chrome did not come up")
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=900, deviceScaleFactor=1, mobile=True)
        url = "http://127.0.0.1:%d/" % STAND_PORT
        press = lambda text: p.js("(function(t){for (const b of document.querySelectorAll('button')) if (b.textContent.trim() === t) { b.click(); return 1 } return 0})(%s)" % json.dumps(text))  # noqa: E731
        for lang in args.langs.split(","):
            word = lambda node: node[lang] if isinstance(node, dict) else node  # noqa: E731
            for theme in args.themes.split(","):
                p.call("Page.navigate", url=url)
                p.settle(1.0)
                p.js("try { localStorage.setItem('lang', %s); localStorage.setItem('t', %s) } catch (e) {}" % (json.dumps(lang), json.dumps(theme)))
                p.call("Page.navigate", url=url)
                end = time.time() + 15
                while time.time() < end and not p.js("!!document.querySelector('button')"):
                    time.sleep(0.25)
                p.settle(2.0)
                for where, steps in (("thermostat", [menu["ui"]["modules"], next(pg["title"] for pg in menu["tree"]["modules"]["pages"] if pg["id"] == "thermostat")]),):
                    for step in steps:
                        if not press(word(step)):
                            raise NotRun("no button '%s' on the page" % word(step))
                        p.settle(0.8)
                    if not steps:
                        press(word(menu["ui"]["scheme"]))
                        p.settle(0.8)
                    p.js("Array.from(document.querySelectorAll('button[aria-expanded=\"false\"]')).forEach(b => b.click()); 1")
                    p.settle(0.6)
                    said = json.loads(p.js(ASK))
                    if said is None:
                        raise NotRun("the card of the setpoint is not on %s" % where)
                    if args.shots:
                        p.shot(os.path.join(args.shots, "%s_%s_%s.png" % (where, lang, theme)), full=True)
                    for check, (passed, words_) in (("rows-as-menu-says", judge_rows(said, expect)), ("one-column", judge_one_column(said)), ("one-width-per-kind", judge_one_width_per_kind(said)),
                                                    ("labels-one-row", judge_labels(said)), ("one-font", judge_one_font(said)), ("one-row-height", judge_row_height(said)),
                                                    ("pair-on-one-line", judge_pairs_on_a_line(said)), ("rows-inside-card", judge_rows_inside(said))):
                        print("%-22s %-20s %s  %s" % ("%s/%s %s" % (lang, theme, where), check, "PASS" if passed else "FAIL", words_))
                        ok = ok and passed
    except NotRun as why:
        print("page_grid: the run did not take place: %s" % why)
        sys.exit(2)
    finally:
        if chrome:
            chrome.terminate()
        if stand:
            stand.terminate()
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
