"""A control looks like what it does: the page is asked what its controls are painted, and the answers compared.

    uv run --no-project --with websocket-client python tools/page_controls.py [--url http://127.0.0.1:8790/]
        [--themes l,d]

Run tools/page_stand.py first.

WHAT IS BEING ASKED. Three different things stand side by side on a card, and a person has to tell them apart
before pressing, not after:

  acts now     On / Off / Stop / restart - the relay clicks, the auger moves, the igniter heats
  confirms     Save - what was typed goes to the board, and nothing happened until it was pressed
  takes input  a field or a list - changing it does nothing at all yet

On 21.09 all three were one beige rounded rectangle. The cost of that is not "it looks plain": it is a hand
reaching for a field and switching a relay on a boiler.

The judging is done on what the BROWSER computed, not on the classes in the source: a class that is missing from
the stylesheet leaves the markup looking right and the page looking wrong, and this has to catch that too.

Checks, each printing one verdict line, in each theme:

  acts-vs-input     a button that acts now is not painted as a field
  confirms-vs-input a button that confirms is not painted as a field
  acts-vs-confirms  acting now and confirming are told apart from each other
  input-is-not-lit  no field is filled with the accent, which is what "press me" is drawn with
  card-title-is-ink  the name of every card is painted the ink of the values, not the label's colour, and is heavier than the label (REF_Device_Page_Look, rule 3; tools/card_title.py)
  choice-acts-on-press  a choice of two or three is a switch: its words are one or two, a press on it sets the
                    board without Save, and the line under it says what the choice now set means

Exits 1 when a check failed, 0 when all passed.
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


from chrome_page import CHROME, Page  # noqa: E402

PORT = 9411

from card_title import problems as title_problems  # noqa: E402
from page_sources import merged_menu  # noqa: E402

# What the browser painted the names of the cards, a row's label and a value that carries no level.
TITLES = """(function(){
  const paint = (e) => { const c = getComputedStyle(e); return {color: c.color, weight: parseInt(c.fontWeight, 10)}; };
  const titles = Array.from(document.querySelectorAll('section h2')).map(h => ({text: h.textContent.trim(), ...paint(h)}));
  const rows = Array.from(document.querySelectorAll('section div.border-b[class*="py-1.5"]'));
  const label = rows.map(r => r.querySelector('span.text-dim')).find(Boolean);
  const value = rows.map(r => r.querySelector('span.text-right')).find(v => v && !/text-(ok|bad|warn|dim)/.test(v.className));
  return JSON.stringify({titles, label: label ? paint(label) : null, value: value ? paint(value) : null});
})()"""

# What each control is painted, taken from the browser. The three kinds are picked out by WHAT THEY DO, which on
# this page is readable from the markup: a button whose words switch a thing, the one Save of a card, and the
# fields and lists a person types into.
LOOK = """(function(){
  const paint = (el) => {
    const s = getComputedStyle(el);
    return {background: s.backgroundColor, border: s.borderColor, width: s.borderTopWidth, ink: s.color};
  };
  const acts = [], confirms = [], inputs = [];
  for (const b of document.querySelectorAll('button')) {
    const w = b.textContent.trim();
    if (w === 'On' || w === 'Off' || w === 'Stop' || w === 'restart') acts.push({words: w, ...paint(b)});
    else if (w === 'Save') confirms.push({words: w, ...paint(b)});
  }
  for (const f of document.querySelectorAll('input:not([type=range]), select')) {
    inputs.push({words: f.tagName.toLowerCase() + (f.type ? ':' + f.type : ''), ...paint(f)});
  }
  return JSON.stringify({acts, confirms, inputs});
})()"""


def one_look(group):
    """The paint a group of controls shares, or the several it does not."""
    seen = {}
    for one in group:
        key = (one["background"], one["border"], one["width"])
        seen.setdefault(key, []).append(one["words"])
    return seen


def told_apart(a, b):
    """Two kinds are told apart when no painting of one is the painting of the other."""
    return not (set(one_look(a)) & set(one_look(b)))


def where_to_find(menu, name):
    """Where the page keeps the setting NAME, by the menu's tree: {tab, module, section} - the word of the tab, the title of the module's page (None on the main screen and the system tab) and of the section on it (None where the section
    is not titled). A setting is on the page of its module; one the main screen shows again is looked for there first, the main screen's being the Scheme tab. A setting the tree does not hold is refused in words: it is
    looked for on no tab."""
    tree, ui = menu["tree"], menu["ui"]
    for page in tree["modules"]["pages"]:
        for section in page["sections"]:
            if name in section.get("settings", []):
                return {"tab": ui["modules"]["en"], "module": page["title"]["en"], "section": section["title"]["en"] if "title" in section else None}
    for key, tab in (("system", "system"), ("main", "scheme")):
        for section in tree[key]["sections"]:
            if name in section.get("settings", []):
                return {"tab": ui[tab]["en"], "module": None, "section": section["title"]["en"]}
    raise LookupError("the menu's tree does not hold the setting %s" % name)


def go_to(p, where):
    """Show the section WHERE says: press its tab and, for a module's, the button of its page. False when the page has no such tab, page or section."""
    pressed = lambda words: p.js("(function(words, tab){for (const b of document.querySelectorAll(tab ? 'nav button' : 'main button'))"
                                 " if (b.textContent.trim() === words) { b.click(); return 1 } return 0})(%s, %s)" % (json.dumps(words), "true" if words == where["tab"] else "false"))
    if not pressed(where["tab"]):
        return False
    p.settle(1.0)
    if where["module"] is not None:
        if not pressed(where["module"]):
            return False
        p.settle(1.0)
    if where["section"] is None:
        return True
    found = p.js("(function(title){for (const s of document.querySelectorAll('section')) {"
                 " const h = s.querySelector('h2'); if (h && h.textContent.trim() === title) return 1 } return 0})(%s)" % json.dumps(where["section"]))
    p.settle(0.5)
    return bool(found)


# The row of a setting in its section, found by its label - the words a person goes by.
ROW_OF = """(function(label){
  for (const d of document.querySelectorAll('span.text-dim')) {
    if (d.textContent.trim() === label) return d.closest('div.py-2');
  }
  return null;
})"""


def settings_of(p):
    """The settings as the page draws them - its dictionary.js over the board's dictionary, values and the page's
    texts - by name."""
    from stand_frames import drawn_js
    return {s["name"]: s for s in json.loads(p.js(drawn_js("settings")))}


def choices_act_on_press(p, url):
    """Every choice of two or three, pressed on the page without Save: what the board holds after, the words on its
    buttons, and the line under it. Each is put back as it was found. The failures, in words."""
    failures = []
    few = [s for s in settings_of(p).values() if 2 <= len(s.get("options") or []) <= 3]
    if not few:
        return ["choice-acts-on-press: the board sends no choice of two or three - nothing was pressed"]
    menu = merged_menu("burner")
    for s in few:
        where = where_to_find(menu, s["name"])
        if not go_to(p, where):
            failures.append("choice-acts-on-press: %s has no section %s on the page %s of the tab %s" % (s["label"], where["section"], where["module"], where["tab"]))
            continue
        found = s["value"]
        other = next((o for o in s["options"] if o["value"] != found and not o.get("takes")), None)
        if other is None:
            continue
        said = json.loads(p.js("(function(){const row = %s(%s); if (!row) return 'null';"
                               " return JSON.stringify({buttons: Array.from(row.querySelectorAll('button')).map(b => b.textContent.trim()),"
                               " text: row.innerText})})()" % (ROW_OF, json.dumps(s["label"]))))
        if said is None:
            failures.append("choice-acts-on-press: %s is not in %s of the tab %s" % (s["label"], where["section"], where["tab"]))
            continue
        long = [b for b in said["buttons"] if len(b.split()) > 2]
        if long:
            failures.append("choice-acts-on-press: %s offers words of more than two: %s" % (s["label"], long))
        p.js("(function(){const row = %s(%s); for (const b of row.querySelectorAll('button'))"
             " if (b.textContent.trim() === %s) { b.click(); return 1 } return 0})()"
             % (ROW_OF, json.dumps(s["label"]), json.dumps(other["words"])))
        p.settle(1.5)
        now = settings_of(p)[s["name"]]["value"]
        if now != other["value"]:
            failures.append("choice-acts-on-press: %s pressed to %s, the board holds %s - it waits for Save"
                            % (s["label"], other["value"], now))
        else:
            text = p.js("%s(%s).innerText" % (ROW_OF, json.dumps(s["label"])))
            if other.get("means") and other["means"] not in text:
                failures.append("choice-acts-on-press: %s set to %s says nothing of what it means (%r)"
                                % (s["label"], other["value"], other["means"]))
        # put back as found, and the same way: a press
        back = next(o for o in s["options"] if o["value"] == found)
        p.js("(function(){const row = %s(%s); for (const b of row.querySelectorAll('button'))"
             " if (b.textContent.trim() === %s) { b.click(); return 1 } return 0})()"
             % (ROW_OF, json.dumps(s["label"]), json.dumps(back["words"])))
        p.settle(1.5)
        if settings_of(p)[s["name"]]["value"] != found:
            failures.append("choice-acts-on-press: %s NOT put back to %s - set it by hand" % (s["label"], found))
        print("   %-18s %s: %s -> %s -> %s" % ("choice-acts-on-press", s["label"], found, now,
                                               settings_of(p)[s["name"]]["value"]))
    return failures


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://127.0.0.1:8790/")
    ap.add_argument("--themes", default="l,d")
    args = ap.parse_args()

    profile = tempfile.mkdtemp(prefix="page_controls_")
    chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % PORT,
                               "--user-data-dir=%s" % profile, "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    failures = []
    try:
        for _ in range(50):
            try:
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % PORT))
                break
            except OSError:
                time.sleep(0.2)
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=375, height=900, deviceScaleFactor=1, mobile=True)

        for theme in args.themes.split(","):
            p.call("Page.navigate", url=args.url)
            p.settle(1.0)
            p.js("try { localStorage.setItem('t', %s) } catch (e) {}" % json.dumps(theme))
            p.call("Page.navigate", url=args.url)
            end = time.time() + 15
            while time.time() < end:
                if p.js("!!document.querySelector('button')"):
                    break
                time.sleep(0.25)
            p.settle(2.0)
            named = json.loads(p.js(TITLES))
            if not named["label"] or not named["value"]:
                raise SystemExit("theme %s: no row with a label and a value to hold the names of the cards against" % theme)
            wrong = title_problems(named["titles"], named["label"], named["value"])
            if wrong:
                for said in wrong:
                    print("   card-title-is-ink (%s): %s" % (theme, said))
                    failures.append("card-title-is-ink (%s): %s" % (theme, said))
            else:
                print("   %-18s %d card names in the ink of the values, heavier than the label" % ("card-title-is-ink", len(named["titles"])))

            # the settings of a module stand folded (REF_Device_Page_Look, rule 8): a field and its Save are there once a person has pressed the button, and that is what is compared
            p.js("Array.from(document.querySelectorAll('button[aria-expanded=\"false\"]')).forEach(b => b.click()); 1")
            p.settle(0.8)
            seen = json.loads(p.js(LOOK))
            acts, confirms, inputs = seen["acts"], seen["confirms"], seen["inputs"]
            if not named["titles"]:
                raise SystemExit("theme %s: no card drawn - the page did not draw" % theme)
            if not acts:
                # a device with nothing that acts - the burner before its relays are laid through - has no kinds of control to tell apart; its names of cards are held all the same
                print("theme %s: no control acts on this device, the three kinds are not compared" % theme)
                continue
            if not confirms or not inputs:
                raise SystemExit("theme %s: found %d acting, %d confirming, %d fields - the page did not draw"
                                 % (theme, len(acts), len(confirms), len(inputs)))

            print("theme %s: %d act now, %d confirm, %d take input" % (theme, len(acts), len(confirms), len(inputs)))
            for name, group in (("acts now", acts), ("confirms", confirms), ("takes input", inputs)):
                for key, words in one_look(group).items():
                    print("   %-12s %-28s %s" % (name, key[0] + " / " + key[1], ", ".join(sorted(set(words)))))

            for name, a, b in (("acts-vs-input", acts, inputs),
                               ("confirms-vs-input", confirms, inputs),
                               ("acts-vs-confirms", acts, confirms)):
                if told_apart(a, b):
                    print("   %-18s told apart" % name)
                else:
                    shared = sorted(set(one_look(a)) & set(one_look(b)))
                    said = "%s (%s): PAINTED THE SAME - %s" % (name, theme, shared)
                    print("   " + said)
                    failures.append(said)

            # a field must not be filled the way a thing that acts is filled
            lit = [f for f in inputs if f["background"] in {a["background"] for a in acts}
                   and f["background"] not in ("rgba(0, 0, 0, 0)", "transparent")]
            if lit:
                said = "input-is-not-lit (%s): fields are filled like a button: %s" % (theme, lit[:2])
                print("   " + said)
                failures.append(said)
            else:
                print("   %-18s no field is filled like a button" % "input-is-not-lit")

            if theme == args.themes.split(",")[0]:
                for said in choices_act_on_press(p, args.url):
                    print("   " + said)
                    failures.append(said)
    finally:
        chrome.terminate()

    for f in failures:
        print("FAILED:", f)
    print("%d check(s) failed" % len(failures))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
