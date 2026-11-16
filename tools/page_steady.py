"""The page stands still while the burner goes through its states: every card as tall, every button where it was, every
card with the same rows, whatever the automaton, the organs and the sensors are doing (
REF_Device_Page_Look, rule 7).

    uv run --no-project --with websocket-client python tools/page_steady.py [--scenario FILE] [--shots DIR]

Makes the frames itself (cargo run -p automaton_card --example scenario, on tools/page_stand_values.json and FILE -
tools/stand_scenarios/steady.json by default), picks the moments BY VALUE - the stages of the automaton,
a banned feed, a silent flue sensor, the alarm, the organs on and off -, raises tools/page_stand.py on each, opens the
page 375 wide in the light theme, and on each of its tabs Burner and Scheme asks the browser where every card and
every button is. Then, each printing one verdict per tab:

  rows-steady     every card shows the same rows in every moment - a row gone or new is named with its card and moment
  heights-steady  every card is as tall in every moment as in the first, to the pixel
  buttons-steady  every button stands where it stood in the first moment, to the pixel
  controls-in-view  on Scheme, Start, Stop and the feed's button on the auger stand whole on a 375x667 screen
                  unscrolled, in every moment
  automaton-heads-card  on Scheme, the strip of stages and what the automaton does stand at the top of the card
                  Automaton, above its first row, in every moment - and the scheme's card carries no strip
  captions-clear-lines  on Scheme, what each organ does - its words beside it on the drawing - touches no line, box or
                  circle of the drawing in any moment; the igniter's words are measured once more at their longest,
                  WORST_IGNITER, put in by the measure and taken out again; the flame's number too, at
                  WORST_FLAME
  pot-holds-its-words  on Scheme, every text in the pot - the box the word "pot" stands in - lies inside it with
                  POT_ROOM px to its edges and runs over no other text and no circle, the flame's word and number
                  measured at their longest too
  flame-number-on-scheme  on Scheme, the volts the flame's pin holds in that moment stand on the drawing - and a
                  pin that reads nothing stands there as a dash, with no "0.00 V" anywhere

What is measured is what the browser drew, not the page's source. Only the automaton's card is the board's own code on
the stand (crates/automaton_card); the relays, the sensors and the fan are the scenario's rows in the board's words,
so this measures the PAGE against those words. With --shots DIR each moment and tab is saved DIR/<moment>_<tab>.png.
Exits 1 when a check failed. Stops the stand and Chrome after each moment.
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

CHROME_PORT = 9415
STAND_PORT = 8799
WIDTH = 375
# the lowest common phone of that width: what a person sees of the page before scrolling
HEIGHT = 667
TABS = ["Scheme", "Modules"]
# the language the page is asked in, the words of its tabs by the names this tool calls them, and the word of its Save button: the page's own are the menu's
LANG = "en"
TAB_WORDS = {}
SAVE = "Save"
# the longest the igniter's words can be: heating or burning, with a clock of two-digit minutes
WORST_IGNITER = "burning 59 min 59 s"
# the most the flame's pin can hold: its 3.3 V supply
WORST_FLAME = "3.30 V"
# the unit of volts as the page says it in the language asked
VOLT = "V"
# the longest word of the flame the page draws in the pot
WORST_FLAME_WORD = "steady"
# what a text in the pot keeps from its edges, in px on the screen
POT_ROOM = 4
# THE WORDS THE DRAWING SAYS, which a page of another menu says otherwise: how an organ's action begins, the word the pot's box carries, the flame's words in the pot, the six words of the strip
# of stages and the card the automaton stands in. A revision whose words stand in its code (this tool's own moments) keeps these; the new burner's are the menu's, put in by page_steady_modules.
WORDS = {"starts": ["heating", "burning"], "pot": "pot", "flame": ["out", "weak", "steady"],
         "stages": ["stop", "purge", "lighting", "rising", "working", "burn-off"], "automaton": "Automaton"}


def value(frame, group, name):
    """The row NAME of the card GROUP in FRAME, as a page of contract 1 read its value (tools/stand_frames.py)."""
    return frame["named"].get((group, name), {})


def stage(frame):
    return value(frame, "automaton", "stage").get("word")


# The moments, each the first frame of the scenario where it holds - by the values of the cards, not by the second.
MOMENTS = [
    ("stop", lambda f: stage(f) == "stop"),
    ("purge", lambda f: stage(f) == "purge"),
    ("igniter_burning", lambda f: value(f, "automaton", "igniter").get("on")),
    ("rising", lambda f: stage(f) == "rising"),
    ("feeding", lambda f: stage(f) == "working" and value(f, "automaton", "auger").get("on")),
    ("pause", lambda f: stage(f) == "working" and value(f, "automaton", "auger").get("on") is False),
    ("feed_banned", lambda f: value(f, "automaton", "feed").get("on") is False),
    ("flue_silent", lambda f: "unknown" in value(f, "flue", "flue gas")),
    ("smouldering", lambda f: stage(f) == "smouldering"),
    ("burning_off", lambda f: stage(f) == "burning_off"),
    ("fault", lambda f: stage(f) == "fault"),
    ("alarm", lambda f: value(f, "automaton", "alarm").get("on") is True),
    ("flame_silent", lambda f: "unknown" in value(f, "flame", "pin")),
]

# Where the page drew each card and each button, asked of the browser. A card is a section, named by its heading; its
# rows are its lines with a divider, named by their first words. Buttons are counted in the order they stand in, so a
# button whose words change - Ban feed, Allow feed - is still the same button in the same place.
ASK = """(function(){
  const STAGES = %s;
  const cards = Array.from(document.querySelectorAll('main section, section')).filter((s, i, all) => all.indexOf(s) === i);
  const said = cards.map((s, i) => {
    const h = s.querySelector('h2, h3');
    const r = s.getBoundingClientRect();
    const rows = Array.from(s.querySelectorAll('div.border-b')).map(d => {
      const first = d.querySelector('span');
      return (first ? first.textContent : d.textContent).trim();
    });
    // THE STRIP IS KNOWN BY ITS WORDS, the six stages in order, and not by a mark in the markup
    const strip = Array.from(s.querySelectorAll('div')).find(d => d.children.length === STAGES.length &&
        Array.from(d.children).every((c, k) => c.textContent.trim() === STAGES[k]));
    const first = s.querySelector('div.border-b');
    // what each row says: its name and, beside it, the words of its state - the second span of a row (class Row of the page, `py-1.5`; the forms of the settings are not rows)
    const says = Array.from(s.querySelectorAll('div.border-b[class*="py-1.5"]')).map(d => {
      const spans = d.querySelectorAll('span');
      return [spans.length ? spans[0].textContent.trim() : '', spans.length > 1 ? spans[1].textContent.trim() : ''];
    });
    // a table of rows, one a step (data-step): which row the page marks as the one the burner burns on (aria-current) and what each cell of it holds
    const steps = Array.from(s.querySelectorAll('[data-step]')).map(d => ({step: d.dataset.step, chosen: d.getAttribute('aria-current') === 'true',
        values: Array.from(d.querySelectorAll('input')).map(i => i.value)}));
    const curves = s.querySelectorAll('svg[viewBox="0 0 100 30"]').length;
    const sliders = s.querySelectorAll('input[type="range"]').length;
    return {card: h ? h.textContent.trim() : ('card ' + (i + 1)), top: r.top + window.scrollY, height: r.height, rows, says, steps, curves, sliders,
            text: s.innerText, strip_top: strip ? strip.getBoundingClientRect().top + window.scrollY : null,
            first_row_top: first ? first.getBoundingClientRect().top + window.scrollY : null};
  });
  const buttons = Array.from(document.querySelectorAll('button')).map((b, i) => {
    const r = b.getBoundingClientRect();
    return {button: i + ' ' + b.textContent.trim(), top: r.top + window.scrollY, left: r.left, width: r.width,
            bottom: r.bottom + window.scrollY};
  });
  return JSON.stringify({cards: said, buttons});
})()"""


# WHAT THE ORGANS' WORDS TOUCH ON THE DRAWING, asked of the browser: the words are the drawing's texts of 12 px (the
# numbers are 13, the labels 9 and 10), the lines every path, rect and circle, walked point by point in the browser's
# own geometry. Each text is asked as drawn, and the igniter's once more at its longest.
LINES = """(function(worst, worstFlame, starts){
  const svg = document.querySelector('section svg');
  if (!svg) return JSON.stringify({captions: 0, crossed: []});
  const shapes = Array.from(svg.querySelectorAll('path, rect, circle, line'));
  const points = [];
  for (const g of shapes) {
    const m = g.getScreenCTM(), n = g.getTotalLength();
    for (let d = 0; d <= n; d += 0.5) {
      const p = g.getPointAtLength(d);
      points.push({x: m.a * p.x + m.c * p.y + m.e, y: m.b * p.x + m.d * p.y + m.f, what: g.tagName});
    }
  }
  const crossed = [];
  const touch = (t, words) => {
    const r = t.getBoundingClientRect();
    const hit = points.find(p => p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom);
    if (hit) crossed.push("'" + words + "' on a " + hit.what);
  };
  const captions = Array.from(svg.querySelectorAll('text[font-size="12"]'));
  for (const t of captions) {
    const said = t.textContent;
    touch(t, said);
    const longest = starts.some(w => said.startsWith(w + ' ')) ? worst : (/^\\d+\\.\\d\\d V$/.test(said) ? worstFlame : null);
    if (longest) {
      t.textContent = longest;
      touch(t, longest);
      t.textContent = said;
    }
  }
  const texts = Array.from(svg.querySelectorAll('text')).map(t => t.textContent.trim());
  return JSON.stringify({captions: captions.length, crossed, texts});
})(%s, %s, %s)"""


# EVERY WORD OF THE DRAWING AGAINST ITS FRAMES, asked of the browser: each text of the drawing - the names of the parts, the numbers, the organs' words, the buttons' - lies across no outline (path, rect, circle, line, walked
# point by point) and over no other text, and the buttons laid over the drawing are crossed by no text that is not wholly inside them. Geometry, not words: it holds for whatever language the page is asked in.
LABELS = """(function(){
  const svg = document.querySelector('section svg');
  if (!svg) return JSON.stringify({texts: 0, crossed: []});
  const texts = Array.from(svg.querySelectorAll('text')).filter(t => t.textContent.trim());
  const points = [];
  for (const g of Array.from(svg.querySelectorAll('path, rect, circle, line'))) {
    const m = g.getScreenCTM(), n = g.getTotalLength();
    for (let d = 0; d <= n; d += 0.5) {
      const p = g.getPointAtLength(d);
      points.push({x: m.a * p.x + m.c * p.y + m.e, y: m.b * p.x + m.d * p.y + m.f, what: g.tagName});
    }
  }
  const over = (a, b) => a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5;
  const inside = (a, b) => a.left >= b.left - 1 && a.right <= b.right + 1 && a.top >= b.top - 1 && a.bottom <= b.bottom + 1;
  const crossed = [];
  const buttons = Array.from(svg.parentElement.querySelectorAll('button')).map(b => b.getBoundingClientRect());
  texts.forEach((t, i) => {
    const r = t.getBoundingClientRect(), said = t.textContent.trim();
    const hit = points.find(p => p.x > r.left + 1 && p.x < r.right - 1 && p.y > r.top + 1 && p.y < r.bottom - 1);
    if (hit) crossed.push("'" + said + "' on a " + hit.what);
    texts.forEach((u, j) => { if (j > i && over(r, u.getBoundingClientRect())) crossed.push("'" + said + "' over '" + u.textContent.trim() + "'"); });
    buttons.forEach(b => { if (over(r, b) && !inside(r, b)) crossed.push("'" + said + "' across a button"); });
  });
  return JSON.stringify({texts: texts.length, crossed: Array.from(new Set(crossed))});
})()"""


def labels_clear(seen):
    """The verdict that no word of the drawing lies across a frame, a line, another word or a button: SEEN is [(moment, labels)], LABELS what the browser said. A crossing is said once, with the first moment it stands in and how many moments it
    stands in."""
    found = {}
    for moment, labels in seen:
        for c in labels["crossed"]:
            found.setdefault(c, []).append(moment)
    out = ["%s in %s%s" % (c, moments[0], " (and %d more moments)" % (len(moments) - 1) if len(moments) > 1 else "") for c, moments in found.items()]
    shown = "; ".join(out[:12]) + (" (%d more crossings)" % (len(out) - 12) if len(out) > 12 else "")
    words = sum(labels["texts"] for _, labels in seen)
    return ("labels-clear-frames", not out, shown if out else "%d words of the drawing clear of every frame, line and button in %d moments" % (words, len(seen)))


def tabs_one_row(buttons, tabs, width=375):
    """The verdict that the tabs stand in one row: BUTTONS are what the browser said of the page's buttons - `button` is '<index> <words>', `top`, `left`, `width` -, TABS the words of the tabs. All of them at one height, and none past
    the right edge of a page WIDTH px wide: five tabs that wrap to a second row move every card under them."""
    mine = [b for b in buttons if b["button"].split(" ", 1)[-1] in tabs]
    if len(mine) != len(tabs):
        return ("tabs-one-row", False, "%d of the %d tabs on the page" % (len(mine), len(tabs)))
    tops = sorted({round(b["top"]) for b in mine})
    past = [b["button"] for b in mine if b["left"] + b["width"] > width + 0.5]
    out = (["tabs at %d heights: %s" % (len(tops), ", ".join(str(t) for t in tops))] if len(tops) > 1 else []) + ["'%s' goes past the edge" % w for w in past]
    return ("tabs-one-row", not out, "; ".join(out) if out else "%d tabs in one row" % len(mine))


def clear(seen):
    """The verdict on the organs' words on the drawing: SEEN is [(moment, lines)], LINES what the browser said."""
    out = ["%s in %s" % (c, moment) for moment, lines in seen for c in lines["crossed"]]
    shown = "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "")
    words = sum(lines["captions"] for _, lines in seen)
    return ("captions-clear-lines", not out, shown if out else
            "%d organ words clear of every line in %d moments" % (words, len(seen)))


# WHAT THE POT HOLDS, asked of the browser: the pot is the rect the text "pot" stands in; every text inside it, its
# circles, and each text's box - the flame's word and number put in at their longest and taken out again.
POT = """(function(worstWord, worstNumber, potWord, flameWords){
  const svg = document.querySelector('section svg');
  if (!svg) return JSON.stringify(null);
  const inside = (r, b) => b.left >= r.left && b.right <= r.right && b.top >= r.top && b.bottom <= r.bottom;
  const texts = Array.from(svg.querySelectorAll('text'));
  const label = texts.find(t => t.textContent.trim() === potWord);
  const box = label && Array.from(svg.querySelectorAll('rect')).map(r => r.getBoundingClientRect())
      .find(r => inside(r, label.getBoundingClientRect()));
  if (!box) return JSON.stringify({pot: null});
  const centre = (b) => ({x: (b.left + b.right) / 2, y: (b.top + b.bottom) / 2});
  const held = (b) => { const c = centre(b); return c.x > box.left && c.x < box.right && c.y > box.top && c.y < box.bottom; };
  const said = (t) => t.textContent.trim();
  const longest = (t) => /^\\d+\\.\\d\\d V$|^-$/.test(said(t)) && t.getAttribute('font-size') === '12' ? worstNumber
      : (flameWords.includes(said(t)) ? worstWord : null);
  const boxes = [];
  for (const t of texts) {
    const was = t.textContent, worst = longest(t);
    if (worst) t.textContent = worst;
    const b = t.getBoundingClientRect();
    if (held(b)) boxes.push({text: said(t), left: b.left, right: b.right, top: b.top, bottom: b.bottom});
    t.textContent = was;
  }
  const circles = Array.from(svg.querySelectorAll('circle')).map(c => c.getBoundingClientRect()).filter(held)
      .map(b => ({left: b.left, right: b.right, top: b.top, bottom: b.bottom}));
  return JSON.stringify({pot: {left: box.left, right: box.right, top: box.top, bottom: box.bottom}, texts: boxes, circles});
})(%s, %s, %s, %s)"""


def crowded(seen, room=POT_ROOM):
    """The verdict on the pot: SEEN is [(moment, pot)], POT what the browser said of it."""
    out = []
    over = lambda a, b: a["left"] < b["right"] and b["left"] < a["right"] and a["top"] < b["bottom"] and b["top"] < a["bottom"]
    for moment, pot in seen:
        if not pot or not pot.get("pot"):
            out.append("no pot in %s" % moment)
            continue
        edge = pot["pot"]
        for t in pot["texts"]:
            short = min(t["left"] - edge["left"], edge["right"] - t["right"], t["top"] - edge["top"], edge["bottom"] - t["bottom"])
            if short < room:
                out.append("'%s' %.1f px from the pot's edge in %s" % (t["text"], short, moment))
        for i, a in enumerate(pot["texts"]):
            for b in pot["texts"][i + 1:]:
                if over(a, b):
                    out.append("'%s' over '%s' in %s" % (a["text"], b["text"], moment))
            for c in pot["circles"]:
                if over(a, c):
                    out.append("'%s' over a circle in %s" % (a["text"], moment))
    shown = "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "")
    return ("pot-holds-its-words", not out, shown if out else "the pot holds its words with %d px to spare in %d moments" % (room, len(seen)))


def flame_volts(frame):
    """What the drawing must say of the flame's pin in FRAME: its volts to the hundredth, or a dash when it reads nothing."""
    n = value(frame, "flame", "pin").get("number")
    return "%.2f %s" % (n["value"] / 1000, VOLT) if n and n.get("unit") == "mV" else "-"


def flame_number(seen):
    """The verdict on the flame's number on the drawing: SEEN is [(moment, lines, want)], WANT what flame_volts says."""
    out = []
    for moment, lines, want in seen:
        if want not in lines["texts"]:
            out.append("'%s' not on the drawing in %s" % (want, moment))
        if "0.00 %s" % VOLT in lines["texts"]:
            out.append("'0.00 V' on the drawing in %s" % moment)
    shown = "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "")
    dashes = sum(1 for _, _, want in seen if want == "-")
    return ("flame-number-on-scheme", not out, shown if out else
            "the flame's volts on the drawing in %d moments, a dash in %d of them" % (len(seen), dashes))


def judge(seen):
    """The verdicts on one tab: SEEN is [(moment, said)] in the order of the scenario, the first the one every other is
    held against. Returns [(check, ok, words)]."""
    first_moment, first = seen[0]
    gone_or_new, taller, moved = [], [], []
    want_cards = {c["card"]: c for c in first["cards"]}
    want_buttons = [b for b in first["buttons"]]
    for moment, said in seen[1:]:
        cards = {c["card"]: c for c in said["cards"]}
        for name, want in want_cards.items():
            got = cards.get(name)
            if got is None:
                gone_or_new.append("card '%s' gone in %s" % (name, moment))
                continue
            for row in want["rows"]:
                if row not in got["rows"]:
                    gone_or_new.append("'%s' of '%s' gone in %s" % (row, name, moment))
            for row in got["rows"]:
                if row not in want["rows"]:
                    gone_or_new.append("'%s' of '%s' new in %s" % (row, name, moment))
            if abs(got["height"] - want["height"]) > 0:
                taller.append("'%s' %+.1f px in %s" % (name, got["height"] - want["height"], moment))
        for name in cards:
            if name not in want_cards:
                gone_or_new.append("card '%s' new in %s" % (name, moment))
        if len(said["buttons"]) != len(want_buttons):
            moved.append("%d buttons in %s against %d in %s" % (len(said["buttons"]), moment, len(want_buttons), first_moment))
        for want, got in zip(want_buttons, said["buttons"]):
            dy, dx = got["top"] - want["top"], got["left"] - want["left"]
            if dy or dx:
                moved.append("'%s' %+.1f px down %+.1f px right in %s" % (got["button"], dy, dx, moment))
    count = lambda words, n: "%s (%d more)" % ("; ".join(words[:n]), len(words) - n) if len(words) > n else "; ".join(words)
    return [
        ("rows-steady", not gone_or_new, count(gone_or_new, 6) if gone_or_new else
         "%d cards, the same rows in %d moments" % (len(want_cards), len(seen))),
        ("heights-steady", not taller, count(taller, 6) if taller else "every card as tall in %d moments" % len(seen)),
        ("buttons-steady", not moved, count(moved, 6) if moved else
         "%d buttons in place in %d moments" % (len(want_buttons), len(seen))),
    ]


# The buttons a person runs the burner by, by their words: the feed's changes its words with the feed, and it is the
# auger of the drawing itself.
CONTROLS = [("Start",), ("Stop",), ("Ban feed", "Allow feed")]


def in_view(seen, height=HEIGHT, controls=None):
    """The verdict on the controls of one tab: every one of CONTROLS stands whole above HEIGHT in every moment of SEEN,
    [(moment, said)]. A control the page does not draw at all is out of view."""
    out = []
    for moment, said in seen:
        for words in controls or CONTROLS:
            got = next((b for b in said["buttons"] if b["button"].split(" ", 1)[1] in words), None)
            if got is None:
                out.append("'%s' not drawn in %s" % (words[0], moment))
            elif got["bottom"] > height:
                out.append("'%s' ends %.0f px down in %s" % (words[0], got["bottom"], moment))
    shown = "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "")
    return ("controls-in-view", not out, shown if out else
            "%d controls above %d px in %d moments" % (len(controls or CONTROLS), height, len(seen)))


def heads(seen, title=None):
    """The verdict on where the automaton's strip and state stand on one tab: SEEN is [(moment, said, doing)], DOING
    the words the automaton's card says it does in that moment."""
    out = []
    for moment, said, doing in seen:
        cards = {c["card"]: c for c in said["cards"]}
        title = title or WORDS["automaton"]
        card = cards.get(title)
        if card is None:
            out.append("no card %s in %s" % (title, moment))
            continue
        if card["strip_top"] is None:
            out.append("no strip in %s in %s" % (title, moment))
        elif card["first_row_top"] is not None and card["strip_top"] > card["first_row_top"]:
            out.append("the strip under the first row of %s in %s" % (title, moment))
        if doing and doing not in card["text"]:
            out.append("'%s' not in %s in %s" % (doing, title, moment))
        for name, other in cards.items():
            if name != title and other["strip_top"] is not None:
                out.append("a strip in '%s' in %s" % (name, moment))
    shown = "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "")
    return ("automaton-heads-card", not out, shown if out else
            "the strip and the state at the top of %s in %d moments" % (title, len(seen)))


def frames_of(scenario):
    """The scenario's frames on the stand's state (tools/page_stand_values.json), each with its rows by name."""
    from page_stand import dictionary_of
    from stand_frames import frames_of as made, named

    dictionary = dictionary_of(ROOT)
    frames = made(scenario, ROOT)
    for frame in frames:
        frame["named"] = named(dictionary, frame)
    return frames


# What the page has drawn, asked of the browser: its cards and the rows in them - or, on a tab of shut sections, the folds that open them.
DRAWN = """(function(){
  return JSON.stringify({cards: document.querySelectorAll('main section').length, rows: document.querySelectorAll('div.border-b').length,
    folds: document.querySelectorAll('button[aria-expanded]').length});
})()"""


def wait_drawn(p, tries=80, pause=0.25, sleep=time.sleep):
    """Whether the page has drawn - a card, and a row in it - within TRIES looks at it, PAUSE apart. A fixed wait is a guess about how long a loaded machine takes to draw a page; the
    guess was wrong twice in one run (no card, no button), and the verdicts then said the cards were gone from the PAGE. A browser that cannot answer yet - it is
    navigating - is a page not drawn yet."""
    for _ in range(tries):
        try:
            seen = json.loads(p.js(DRAWN))
        except Exception:  # noqa: BLE001 - whatever the browser says while a page is on its way, it is not a page yet
            seen = None
        if seen and seen["cards"] > 0 and (seen["rows"] > 0 or seen.get("folds", 0) > 0):
            return True
        sleep(pause)
    return False


def drawn_or_stop(p, what, moment):
    """The page waited for; one that does not draw stops the run - a RUN THAT DID NOT TAKE PLACE, the third outcome beside a verdict that passes and one that fails, and never a verdict
    that the cards are gone."""
    if not wait_drawn(p):
        raise SystemExit("page_steady: the page did not draw %s in %s - the run did not take place; nothing is said of the page" % (what, moment))


def look(frames_file, second, shots, moment, page=None, save=False):
    """Raises the stand on SECOND, opens the page and asks each tab; stops both whatever happens."""
    from chrome_page import CHROME, Page

    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT),
                              "--scenario", frames_file, "--at", str(second)] + (["--page", page] if page else []),
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    profile = tempfile.mkdtemp(prefix="page_steady_")
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={CHROME_PORT}",
                               f"--user-data-dir={profile}", "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
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
            raise SystemExit("page_steady: the stand or Chrome did not come up")
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=HEIGHT, deviceScaleFactor=2, mobile=True)
        url = f"http://127.0.0.1:{STAND_PORT}/"
        p.call("Page.navigate", url=url)
        p.settle(1.0)
        p.js("localStorage.setItem('t', 'l'); localStorage.setItem('lang', %s)" % json.dumps(LANG))
        p.call("Page.navigate", url=url)
        p.settle(2.0)
        drawn_or_stop(p, "at all", moment)
        from stand_frames import THINGS_JS, drawn_js
        # the things and the settings the page drew from, made by its own dictionary.js: what the checks below hold the drawing to
        said = {"things": json.loads(p.js(THINGS_JS)), "settings": json.loads(p.js(drawn_js("settings")))}
        for tab in TABS:
            # a view is a tab, or a tab and the page of a module on it: `Modules>fan` - each step a button pressed by its words, the tab first
            for step in tab.split(">"):
                p.js("Array.from(document.querySelectorAll('button')).find(b => b.textContent.trim() === %s).click(); 1"
                     % json.dumps(TAB_WORDS.get(step, step)))
                p.settle(0.6)
            drawn_or_stop(p, "on its tab %s" % tab, moment)
            if save:
                # a Save pressed in every open card: what a person does once a setting is typed
                p.js("Array.from(document.querySelectorAll('button')).filter(b => b.textContent.trim() === %s).forEach(b => b.click()); 1" % json.dumps(SAVE))
                p.settle(1.2)
                drawn_or_stop(p, "after a Save", moment)
            said[tab] = json.loads(p.js(ASK % json.dumps(WORDS["stages"])))
            if tab == "Scheme":
                said["lines"] = json.loads(p.js(LINES % (json.dumps(WORST_IGNITER), json.dumps(WORST_FLAME), json.dumps(WORDS["starts"]))))
                said["labels"] = json.loads(p.js(LABELS))
                said["pot"] = json.loads(p.js(POT % (json.dumps(WORST_FLAME_WORD), json.dumps(WORST_FLAME), json.dumps(WORDS["pot"]), json.dumps(WORDS["flame"]))))
            if shots:
                p.shot(os.path.join(shots, "%s_%s.png" % (moment, tab.replace(">", "-"))), full=True)
        return said
    finally:
        chrome.terminate()
        stand.terminate()
        chrome.wait(10)
        stand.wait(10)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scenario", default=os.path.join(HERE, "stand_scenarios", "steady.json"))
    ap.add_argument("--shots", default=None)
    ap.add_argument("--page", default=None, help="the page files to measure instead of the device's own - a revision taken out of git")
    args = ap.parse_args()
    frames = frames_of(args.scenario)
    work = tempfile.mkdtemp(prefix="page_steady_frames_")
    frames_file = os.path.join(work, "frames.json")
    json.dump({"frames": [{"at": f["at"], "values": f["values"]} for f in frames]}, open(frames_file, "w"))
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    failed = False
    seen = {tab: [] for tab in TABS}
    told = []
    drawn = []
    flames = []
    pots = []
    for name, holds in MOMENTS:
        frame = next((f for f in frames if holds(f)), None)
        if frame is None:
            print("%-16s FAIL  the scenario never comes to it" % name)
            failed = True
            continue
        said = look(frames_file, frame["at"], args.shots, name, args.page)
        for tab in TABS:
            seen[tab].append(("%s @%d" % (name, frame["at"]), said[tab]))
        doing = next((t.get("state") for t in said["things"] if t.get("group") == "automaton" and t["name"] == "doing"), None)
        told.append(("%s @%d" % (name, frame["at"]), said["Scheme"], doing))
        drawn.append(("%s @%d" % (name, frame["at"]), said["lines"]))
        flames.append(("%s @%d" % (name, frame["at"]), said["lines"], flame_volts(frame)))
        pots.append(("%s @%d" % (name, frame["at"]), said["pot"]))
    for tab in TABS:
        for check, ok, words in judge(seen[tab]):
            print("%-7s %-15s %s  %s" % (tab, check, "PASS" if ok else "FAIL", words))
            failed = failed or not ok
    for check, ok, words in (in_view(seen["Scheme"]), heads(told), clear(drawn), flame_number(flames), crowded(pots)):
        print("%-7s %-15s %s  %s" % ("Scheme", check, "PASS" if ok else "FAIL", words))
        failed = failed or not ok
    os.remove(frames_file)
    os.rmdir(work)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
