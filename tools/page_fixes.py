"""The page as the owner saw it on the boiler on 02.10, asked in a browser by numbers and by order, never by a phrase.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_fixes.py [--shots DIR] [--langs ru,en] [--themes l,d]

Raises the stand (tools/page_stand.py) on a run of the new burner's own automaton (tools/stand_scenarios/smoulder_and_wait.json, frames by `cargo run -p burner_api --example burner_frames`), picks the moments BY
VALUE - working, smouldering, a stop that heat is wanted after -, opens the page 375 wide in each language and theme and asks it, each verdict printing PASS or FAIL:

  header-one-row        the title, the version, the period of Live, the language and the theme stand on one line, the three buttons square
  live-period           the button of Live says the period the board sends at, goes red when pressed with no count of seconds on it, and goes back to the period when pressed again
  smoulder-line         the line of smouldering says the time gone and the whole of it, the dose and the pause, and what is left, in the card of the automaton (9.26: the drawing carries no line of text)
  nothing-untranslated  nothing the board's English table leaves on a Russian page ("next", "off") stands on it
  strip-uncut           no word of the strip of stages is cut

The verdicts are `judge_*`: pure functions of what the browser answered, held by tools/test_page_fixes.py on answers that are good and answers that are not. Exits 1 when a check failed.
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

CHROME_PORT = 9417
STAND_PORT = 8801
WIDTH = 375
SCENARIO = os.path.join(HERE, "stand_scenarios", "smoulder_and_wait.json")
from page_sources import merged_menu  # noqa: E402

# (name, how to find its second): by what the automaton says, never by the second the scenario happens to reach it at
MOMENTS = [
    ("working", lambda f: f["doing"] == "working" and f["auger"] is True),
    ("smouldering", lambda f: f["doing"] == "smouldering" and f["for"] >= 40),
    ("waiting", lambda f: f["doing"] == "stop" and f["wanted"] is True),
]

# the English the board's table leaves a Russian page, as whole texts: a text on the page that is one of these, alone, is a word nobody translated
ENGLISH_LEFT = ["next", "off", "on", "yes", "no", "idle"]

ASK = """(function(){
  const rect = (el) => { const r = el.getBoundingClientRect(); return {left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height}; };
  const header = document.querySelector('main > header');
  const kids = header ? Array.from(header.children).map(el => ({tag: el.tagName, text: el.textContent.trim(), box: rect(el)})) : [];
  const buttons = header ? Array.from(header.querySelectorAll('button')).map(el => ({text: el.textContent.trim(), title: el.title, box: rect(el), bg: getComputedStyle(el).backgroundColor, pressed: el.getAttribute('aria-pressed')})) : [];
  const line = (name) => { const el = document.querySelector('[data-line="' + name + '"]'); return el ? {text: el.textContent.trim(), box: rect(el)} : null; };
  const cells = Array.from(document.querySelectorAll('[data-strip="stage"] > span')).map(el => ({text: el.textContent.trim(), clipped: el.scrollWidth > el.clientWidth + 0.5, right: rect(el).right}));
  const strip = document.querySelector('[data-strip="stage"]');
  const alone = Array.from(document.querySelectorAll('main *')).filter(el => el.children.length === 0 && el.textContent.trim() !== '').map(el => el.textContent.trim());
  // the scheme's card: its title, the mode switch in the title's row, the text that stands under the drawing - there is none -, the numbers the drawing carries for the thermostat, every text of the drawing with its box, and the buttons
  const drawing = document.querySelector('section svg[viewBox="0 0 320 250"]');
  const card = drawing ? drawing.closest('section') : null;
  const title = card ? card.querySelector('h2') : null;
  const mode = card ? card.querySelector('[data-mode]') : null;
  const under = card && drawing ? Array.from(card.children).filter(el => el.tagName !== 'H2' && !el.contains(drawing) && !el.querySelector('button') && el.tagName !== 'BUTTON' && el.textContent.trim() !== '' && rect(el).top >= rect(drawing).bottom - 1).map(el => ({tag: el.tagName, text: el.textContent.trim(), box: rect(el)})) : [];
  const thermo = {svg: drawing ? rect(drawing) : null, title: title ? {text: title.textContent.trim(), box: rect(title)} : null,
    mode: mode ? {text: mode.textContent.trim(), box: rect(mode), disabled: mode.disabled} : null,
    card: card ? rect(card) : null,
    blocks: card ? card.querySelectorAll('[data-block]').length : 0, under,
    ask: (() => { const el = card ? card.querySelector('[data-ask]') : null; if (!el) return null; const own = el.querySelector('[data-drawn="ask"]'); const r = document.createRange(); if (own) r.selectNodeContents(own); const ink = r.getBoundingClientRect(); return {word: el.getAttribute('data-ask'), text: own ? own.textContent.trim() : '', box: rect(el), ink: {width: ink.width, height: ink.height}, lineHeight: parseFloat(getComputedStyle(el).lineHeight) || 0}; })(),
    drawn: drawing ? Array.from(drawing.querySelectorAll('text[data-drawn]')).map(t => ({kind: t.getAttribute('data-drawn'), text: t.textContent.trim(), box: rect(t)})) : [],
    texts: drawing ? Array.from(drawing.querySelectorAll('text')).filter(t => t.textContent.trim() !== '').map(t => ({text: t.textContent.trim(), box: rect(t)})) : [],
    svgTexts: drawing ? Array.from(drawing.querySelectorAll('text')).map(t => t.textContent.trim()) : [],
    cardText: card ? card.innerText : '',
    buttons: card ? Array.from(card.querySelectorAll('button')).map(b => ({text: b.textContent.trim(), box: rect(b)})) : [],
    wide: document.documentElement.scrollWidth};
  // the screen: the colours of what is said in the cards, the badge of the thermostat, the pellets at the hopper, the zones of the three main numbers, and the card of what asks for a person
  const token = (name) => { const probe = document.createElement('span'); probe.style.color = 'var(' + name + ')'; document.body.appendChild(probe); const c = getComputedStyle(probe).color; probe.remove(); return c; };
  const first = document.querySelector('main section h2');
  const levels = Array.from(document.querySelectorAll('main [data-level]')).map(el => ({level: el.getAttribute('data-level'), color: getComputedStyle(el).color, text: el.textContent.trim()}));
  const okColour = token('--ok');
  const greens = Array.from(document.querySelectorAll('main section :not(svg):not(svg *)')).filter(el => el.children.length === 0 && el.textContent.trim() !== '' && !el.closest('button') && getComputedStyle(el).color === okColour).map(el => el.textContent.trim());
  const tid = (t) => ({text: t.textContent.trim(), box: rect(t), key: t.getAttribute('data-key')});
  const icon = (g) => { const i = g ? g.querySelector('[data-icon]') : null; return i ? {box: rect(i), kind: i.getAttribute('data-icon')} : null; };
  const badgeG = card ? Array.from(card.querySelectorAll('[data-badge]')) : [];
  const pelletG = drawing ? Array.from(drawing.querySelectorAll('[data-pellet]')) : [];
  const zoneG = drawing ? Array.from(drawing.querySelectorAll('[data-zone]')) : [];
  const att = document.querySelector('section[data-card="attention"]');
  const screen = {ink: first ? getComputedStyle(first).color : null, ok: okColour, warn: token('--warn'), bad: token('--bad'), levels, greens,
    badge: badgeG.map(g => { const t = g.querySelector('[data-drawn="badge"]'); return {mode: g.getAttribute('data-badge'), text: t ? t.textContent.trim() : '', box: t ? rect(t) : null, inSvg: !!g.closest('svg'), icon: icon(g)}; }),
    header: (() => { const h = card ? card.querySelector('h2') : null; const m = card ? card.querySelector('button[data-mode]') : null; return {title: h ? rect(h) : null, mode: m ? rect(m) : null}; })(),
    pellets: pelletG.map(g => ({key: g.getAttribute('data-pellet'), text: g.querySelector('text') ? g.querySelector('text').textContent.trim() : '', box: g.querySelector('text') ? rect(g.querySelector('text')) : null, icon: icon(g), fontPx: g.querySelector('text') ? parseFloat(getComputedStyle(g.querySelector('text')).fontSize) * (drawing ? rect(drawing).width / 320 : 1) : 0})),
    zones: zoneG.map(g => { const part = (n) => { const e = g.querySelector('[data-zone-' + n + ']'); return e ? rect(e) : null; }; const mark = g.querySelector('[data-zone-mark]'); return {name: g.getAttribute('data-zone'), bar: part('bar'), band: part('band'), mark: part('mark'), markFill: mark ? getComputedStyle(mark).fill : null}; }),
    attention: att ? {title: att.querySelector('h2') ? att.querySelector('h2').textContent.trim() : '', box: rect(att), border: getComputedStyle(att).borderTopColor, items: Array.from(att.querySelectorAll('[data-attention]')).map(li => ({key: li.getAttribute('data-attention'), what: (li.querySelector('[data-what]') || {textContent: ''}).textContent.trim(), action: (li.querySelector('[data-action]') || {textContent: ''}).textContent.trim()}))} : null,
    schemeCard: card ? rect(card) : null};
  return JSON.stringify({thermo, screen, kids, buttons, doing: line('doing'), cells, stripRight: strip ? rect(strip).right : null, alone, bad: getComputedStyle(document.documentElement).getPropertyValue('--bad').trim()});
})()"""


def picked_of(rows):
    """What the moments are picked by in one frame, from its ROWS by (card, name): what the fire does, whether the auger turns, whether heat is asked for, and the seconds in the state. **HEAT IS ASKED FOR WHEN THE
    THERMOSTAT'S SWITCH SAYS `on`** (the row `automaton/wanted` went with the buttons to the thermostat's card); none is no row at all, and never "not asked"."""
    switch = rows.get(("thermostat", "switch"), {}).get("word")
    return {
        "doing": rows.get(("automaton", "doing"), {}).get("word"),
        "auger": rows.get(("automaton", "auger"), {}).get("on"),
        "wanted": None if switch is None else switch == "on",
        "for": rows.get(("automaton", "for"), {}).get("seconds", 0),
    }


def frames_of(scenario):
    """The frames of the scenario on the new burner's own automaton, as a list of dicts of what the moments are picked by."""
    from page_stand import dictionary_of
    from stand_frames import named

    out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", scenario], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("page_fixes: the frames were not made:\n" + out.stderr[-1500:])
    dictionary = dictionary_of(ROOT, "burner")
    frames = json.loads(out.stdout)["frames"]
    for frame in frames:
        frame["picked"] = picked_of(named(dictionary, frame))
    return out.stdout, frames


def moments_of(frames):
    """Each moment's second: the first frame of the run where it holds; a moment the run never comes to is None."""
    found = []
    for name, holds in MOMENTS:
        frame = next((f for f in frames if holds(f["picked"])), None)
        found.append((name, None if frame is None else frame["at"]))
    return found


def look(frames_file, second, lang, theme, shot=None, press=False, page=None, settings_of=None, after=0.0):
    """The stand on SECOND, the page in LANG and THEME asked. With SETTINGS_OF - the scenario file the frames were made from - the page holds the settings the frames were made with, and not the defaults. With AFTER seconds the page is asked that long after it has drawn - what a page shows only once something has stood a while is asked there. With PRESS the button of Live is pressed and asked again, then pressed back and asked a third time."""
    import page_steady
    from chrome_page import CHROME, Page

    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT), "--scenario", frames_file, "--at", str(second)] + (["--page", page] if page else []) + (["--settings-of", settings_of] if settings_of else []), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    profile = tempfile.mkdtemp(prefix="page_fixes_")
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={CHROME_PORT}", f"--user-data-dir={profile}", "--no-first-run", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        targets = None
        for _ in range(75):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{STAND_PORT}/state", timeout=1)
                targets = json.load(urllib.request.urlopen(f"http://127.0.0.1:{CHROME_PORT}/json"))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            raise SystemExit("page_fixes: the stand or Chrome did not come up")
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=900, deviceScaleFactor=2, mobile=True)
        url = f"http://127.0.0.1:{STAND_PORT}/"
        p.call("Page.navigate", url=url)
        p.settle(1.0)
        p.js("localStorage.setItem('t', %s); localStorage.setItem('lang', %s)" % (json.dumps(theme), json.dumps(lang)))
        p.call("Page.navigate", url=url)
        p.settle(2.0)
        if not page_steady.wait_drawn(p):
            raise SystemExit("page_fixes: the page did not draw - the run did not take place; nothing is said of the page")
        if after:
            p.settle(after)
        said = {"first": json.loads(p.js(ASK))}
        if shot:
            p.shot(shot, full=True)
        if press:
            button = "Array.from(document.querySelectorAll('main > header button')).find(b => b.hasAttribute('aria-pressed')).click(); 1"
            p.js(button)
            p.settle(0.8)
            said["pressed"] = json.loads(p.js(ASK))
            p.js(button)
            p.settle(0.8)
            said["back"] = json.loads(p.js(ASK))
        return said
    finally:
        chrome.terminate()
        stand.terminate()
        chrome.wait(10)
        stand.wait(10)


# ---- the verdicts: pure functions of what the browser answered ----

def judge_header_one_row(said, tolerance=2.0):
    """The children of the header share one line, and the buttons of it are squares. -> (ok, words)"""
    kids = said["kids"]
    if not kids:
        return False, "no header on the page"
    tops = [k["box"]["top"] for k in kids]
    middles = [(k["box"]["top"] + k["box"]["bottom"]) / 2 for k in kids]
    one_line = max(middles) - min(middles) <= tolerance
    squares = [b for b in said["buttons"]]
    not_square = ["%s %.0fx%.0f" % (b["text"], b["box"]["width"], b["box"]["height"]) for b in squares if abs(b["box"]["width"] - b["box"]["height"]) > 1.0]
    if not one_line:
        lines = []
        for middle in sorted(middles):
            if not lines or middle - lines[-1] > tolerance:
                lines.append(middle)
        return False, "the header stands on %d lines: tops %s" % (len(lines), ", ".join("%.0f" % t for t in tops))
    if len(squares) < 3:
        return False, "%d buttons in the header, expected the period, the language and the theme" % len(squares)
    if not_square:
        return False, "not square: " + "; ".join(not_square)
    return True, "%d children on one line, %d square buttons" % (len(kids), len(squares))


def judge_live_period(said, period, unit):
    """Idle the button says PERIOD and UNIT and is grey; pressed it is red with no count of seconds on it; pressed again it is as it was. -> (ok, words)"""
    def button(step):
        return next((b for b in said[step]["buttons"] if b["pressed"] is not None), None)

    idle, pressed, back = button("first"), button("pressed"), button("back")
    if idle is None:
        return False, "no button of Live in the header"
    want = "%d%s" % (period, unit)
    if idle["text"] != want:
        return False, "idle it says %r, the board's period is %r" % (idle["text"], want)
    if pressed["pressed"] != "true" or pressed["bg"] == idle["bg"]:
        return False, "pressed, the button did not change: %s" % pressed["bg"]
    if said["pressed"]["bad"] and not same_colour(pressed["bg"], said["pressed"]["bad"]):
        return False, "pressed, it is %s and not the red of the page %s" % (pressed["bg"], said["pressed"]["bad"])
    if re.search(r"\d", pressed["text"]):
        return False, "pressed, a count of seconds stands on it: %r" % pressed["text"]
    if back["text"] != want or back["pressed"] != "false":
        return False, "pressed again it says %r and not %r" % (back["text"], want)
    return True, "%s idle, red pressed (%r), %s again" % (want, pressed["text"], want)


def same_colour(computed, token):
    """Whether the colour the browser computed (rgb(r, g, b)) is the page's token (#rrggbb or rgb)."""
    def parts(text):
        text = text.strip()
        if text.startswith("#"):
            text = text[1:]
            if len(text) == 3:
                text = "".join(c * 2 for c in text)
            return tuple(int(text[i:i + 2], 16) for i in (0, 2, 4))
        return tuple(int(float(x)) for x in re.findall(r"[\d.]+", text)[:3])
    try:
        return all(abs(a - b) <= 2 for a, b in zip(parts(computed), parts(token)))
    except ValueError:
        return True  # a token the page keeps in another notation says nothing about this one


def clock(s):
    return "%d:%02d" % (s // 60, s % 60)


def seconds_of(clock_text):
    minutes, seconds = clock_text.split(":")
    return int(minutes) * 60 + int(seconds)


def judge_smoulder_line(said, lasted, total, on, off, drift=15):
    """Smouldering, in the card of the automaton, says the time gone, the whole, the dose and the pause, and what is left. The time gone is the frame's LASTED and what the page has counted on since it drew, up to DRIFT
    seconds; what is left is exactly the whole less the time gone it says. -> (ok, words)"""
    line = (said["doing"] or {}).get("text", "")
    found = re.findall(r"\d+:\d\d|\d+", line)
    if len(found) < 5 or not (":" in found[0] and ":" in found[1] and ":" in found[4]):
        return False, "in the card: %r has %s, expected a time gone, a whole, a dose, a pause and a time left" % (line, found)
    gone, whole, dose, pause, left = seconds_of(found[0]), seconds_of(found[1]), found[2], found[3], seconds_of(found[4])
    if not lasted <= gone <= lasted + drift:
        return False, "in the card: %r says %s gone, the run has it at %s" % (line, found[0], clock(lasted))
    if (whole, dose, pause) != (total, str(on), str(off)):
        return False, "in the card: %r has %s of %s, %s / %s; expected %s, %s / %s" % (line, found[0], found[1], found[2], found[3], clock(total), on, off)
    if left != max(0, total - gone):
        return False, "in the card: %r leaves %s of what is %s less %s" % (line, found[4], found[1], found[0])
    return True, "%s of %s, %s / %s, %s left" % (clock(gone), clock(total), on, off, clock(left))


def judge_nothing_untranslated(said, lang):
    """On a Russian page no text that is alone one of the English words the board's table leaves. -> (ok, words)"""
    if lang != "ru":
        return True, "english page: nothing to check"
    left = sorted({t for t in said["alone"] if t.lower() in ENGLISH_LEFT})
    return (not left), ("left in English: " + ", ".join(left)) if left else "nothing left in English"


def judge_strip_uncut(said):
    """No word of the strip of stages is cut or ends past the strip. -> (ok, words)"""
    cells = said["cells"]
    if not cells:
        return False, "no strip on the page"
    cut = [c["text"] for c in cells if c["clipped"] or "…" in c["text"] or c["right"] > said["stripRight"] + 0.5]
    return (not cut), ("cut: " + ", ".join(cut)) if cut else "%d words, none cut" % len(cells)


# ---- the run ----

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", default=None)
    ap.add_argument("--langs", default="ru,en")
    ap.add_argument("--themes", default="l,d")
    ap.add_argument("--page", default=None, help="the page files to measure instead of the device's own - a revision taken out of git")
    args = ap.parse_args()
    menu = merged_menu("burner")
    text, frames = frames_of(SCENARIO)
    work = tempfile.mkdtemp(prefix="page_fixes_frames_")
    frames_file = os.path.join(work, "frames.json")
    with open(frames_file, "w") as f:
        f.write(text)
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    found = moments_of(frames)
    missing = [n for n, at in found if at is None]
    if missing:
        raise SystemExit("page_fixes: the run never comes to: " + ", ".join(missing))
    from page_stand import dictionary_of
    from stand_frames import named

    dictionary = dictionary_of(ROOT, "burner")
    scenario = json.load(open(SCENARIO))
    # the stand holds the defaults of the dictionary for every setting: the numbers of smouldering are those
    default = lambda name: int(next(s["default"] for s in dictionary["settings"] if s["name"] == name))  # noqa: E731
    total, on, off = default("auto_smoulder_s"), default("auto_smould_on"), default("auto_smould_off")
    ok = True
    try:
        for lang in args.langs.split(","):
            words = lambda leaf: leaf[lang] if isinstance(leaf, dict) else leaf  # noqa: E731
            unit = words(menu["units"]["s"])
            for theme in args.themes.split(","):
                first = True
                for moment, second in found:
                    frame = next(f for f in frames if f["at"] == second)
                    rows = named(dictionary, frame)
                    shot = os.path.join(args.shots, "%s_%s_%s.png" % (moment, lang, theme)) if args.shots else None
                    said = look(frames_file, second, lang, theme, shot, press=first, page=args.page)
                    first_said = said["first"]
                    verdicts = [("%s/%s %s" % (lang, theme, moment), "nothing-untranslated", judge_nothing_untranslated(first_said, lang)),
                                ("%s/%s %s" % (lang, theme, moment), "strip-uncut", judge_strip_uncut(first_said)),
                                ("%s/%s %s" % (lang, theme, moment), "header-one-row", judge_header_one_row(first_said))]
                    if first:
                        verdicts.append(("%s/%s %s" % (lang, theme, moment), "live-period", judge_live_period(said, 5, unit)))
                    if moment == "smouldering":
                        verdicts.append(("%s/%s %s" % (lang, theme, moment), "smoulder-line", judge_smoulder_line(first_said, frame["picked"]["for"], total, on, off)))
                    first = False
                    for where, check, (passed, said_words) in verdicts:
                        print("%-22s %-22s %s  %s" % (where, check, "PASS" if passed else "FAIL", said_words))
                        ok = ok and passed
    finally:
        os.remove(frames_file)
        os.rmdir(work)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
