"""The screen shows what matters: the page on the stand, driven by the new burner's own automaton through its scenarios, asked what it paints, what it carries on the drawing and what it asks of a person.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_screen.py [--shots DIR] [--langs ru,en] [--themes l,d] [--page DIR] [--only badge|pellets|ink|zones|attention]

Each moment is picked BY VALUE - the first frame of a run where the automaton's `doing` says it, the fault where its `why` says it - and asked in each language and theme, 375 px wide, each verdict printing PASS or FAIL:

  badge-by-mode      the thermostat's badge stands in the title row of the scheme's card, between the word of the card and the buttons of the mode, with its icon - the thing it holds, and to how much -, in each of the four modes (boiler, return, air, power), and nothing of "set ..." stands at the boiler
  pellets-at-hopper  four numbers - today, yesterday, the month, in all - stand at the hopper, each beside its own icon, with no word among them
  norm-in-ink        a value in order is in the ink of the card's title in every card, nothing in a card but the drawing is green, and a warning and a failure are the colours of their levels
  zone-of-number     the draught, the boiler's water and the flame each stand on a short scale with its tolerance as a band, the value as a mark inside the band or out of it as the numbers say, and in the warning colour only where the fire works and the mark is out
  attention-card     the card of what asks for a person appears on a fault, a burning off with a reason, a silent sensor, an alarm - each with the action in the menu's words - and is not on the page while nothing asks; it stands under the card of the scheme, so the buttons of Start and Stop do not move
  fits-the-phone     the page is no wider than the phone

What is expected is taken from the stand's frame (the rows by value, the settings) and the menu's words in the language: so a word a person changes is not a failing row. The verdicts are `judge_*`: pure functions of what the browser answered, held by
tools/test_page_screen.py on answers that are good and answers that are not. Exits 1 when a check failed.
"""
import argparse
import json
import os
import re
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from page_sources import merged_menu  # noqa: E402
WIDTH = 375
# the pellets stand at the hopper: inside this share of the drawing's width from its left edge (the hopper is 14..88 of 320)
LEFT_SHARE = 0.34
# a figure the owner reads on a phone: its number no smaller than this on the screen, and its icon no smaller than this a side
MIN_FIGURE_PX = 10
MIN_ICON_PX = 9
# a zone on the screen: a bar as long as this and a band as long as that, or the tolerance is a hair
MIN_BAR_PX = 24
MIN_BAND_PX = 3
# how far a mark's middle may stand outside its band's edge and still be said to stand in it: the edges are antialiased
EDGE = 0.5
# how far two middles of one row may stand apart
SAME_LINE = 6
    # the words of the old setpoint at the boiler, as the page said them: "set 44 °C" and the same in Russian
OLD_SET = re.compile(r"(?:set|уст\.) [\d.]+ °C")

MODES = ["boiler", "return", "room", "power"]
# a mark is in the warning colour after it has stood out of its zone this long (scheme.js, DWELL_MS): the draught came back within the band 0 to 5 s after a change of its setpoint on the boiler's records, so ten seconds is twice the longest
DWELL_S = 10
# a page asked after a dwell is asked this long after the dwell has run out, for the page draws a second at a time
PAST_DWELL_S = 2.5


def word(node, lang):
    """A text of the menu in LANG."""
    return node[lang] if isinstance(node, dict) else node


def spell(template, **names):
    """A text with its {places} filled from NAMES, as the page fills the menu's words."""
    return re.sub(r"\{(\w+)\}", lambda m: str(names[m.group(1)]) if m.group(1) in names else m.group(0), template)


def degrees(n):
    """A number of degrees as the page writes it: whole where it is whole, one decimal otherwise (scheme.js, degrees)."""
    n = float(n)
    return str(int(n)) if n == int(n) else str(round(n * 10) / 10)


def whole(n):
    """A number rounded to a whole one as the page does it (Math.round: a half goes up)."""
    return str(int(n + 0.5))


def middle(box, axis):
    return (box["top"] + box["bottom"]) / 2 if axis == "y" else (box["left"] + box["right"]) / 2


# ---------------------------------------------------------------- the expectations: from the stand's frame, the settings and the menu

def badge_expect(rows, menu, lang, held, steps):
    """What the badge must say, from the frame ROWS (card, name -> value), the settings HELD by name and the kilowatts of the steps (big, mid, small) - and the mode it is in. -> {"mode", "text"}"""
    mode = held["auto_hold"]
    said = menu["scheme"]["badge"]
    what = word(menu["scheme"]["hold"][mode], lang)
    if mode == "power":
        rung = rows.get(("thermostat", "rung"), {}).get("word")
        kw = steps[["big", "mid", "small"].index(rung)] if rung in ("big", "mid", "small") else steps[0]
        return {"mode": mode, "text": spell(word(said["power"], lang), what=what, kw=kw)}
    unit = word(menu["units"]["°C"], lang)
    now = rows.get(("thermostat", "setpoint"), {}).get("number", {}).get("value")
    to = float(now if now is not None else held["auto_hold_c"])
    return {"mode": mode, "text": spell(word(said["temp"], lang), what=what, to=degrees(to), unit=unit, drop=degrees(float(held["auto_drop_c"])))}


def pellets_expect(rows):
    """The four numbers at the hopper from the frame's rows of the card `pellets`, each to a tenth; a row not a number - unknown, or none at all - is a dash. -> [text, text, text, text]"""
    out = []
    for key in ("today", "yesterday", "month", "total"):
        n = rows.get(("pellets", key), {}).get("number")
        out.append("-" if n is None else "%.1f" % n["value"])
    return out


def boiler_water(rows, held):
    """The number of the socket whose role is the boiler's, from the frame's rows: {"value", "unit"} or None. The role is a setting, so the row is found by it and not by a name."""
    for n in range(1, 8):
        if held.get("role_ntc_%d" % n) == "boiler":
            return rows.get(("temperatures", "ntc_%d" % n), {}).get("number")
    return None


def zones_expect(rows, held, dwelled=True):
    """What the three zones must say, from the numbers: whether each mark stands in its zone, and whether it is in the warning colour - out of the zone while the fire works, and DWELLED: it has stood out for as long as the page waits to colour it (a page asked at the
    first seconds out has not coloured it yet). A zone the numbers make none of is None. -> {"draught", "water", "flame"}, each None or {"inside", "warn"}"""
    stage = rows.get(("automaton", "doing"), {}).get("word")
    works = stage in ("working", "smouldering") and dwelled
    out = {"draught": None, "water": None, "flame": None}
    pa = rows.get(("draught", "blast_pressure"), {})
    pa = pa.get("number", pa.get("at_least"))
    asked = rows.get(("automaton", "fan"), {}).get("number")
    if pa is not None and asked is not None and float(held["fan_band"]) > 0:
        band = float(held["fan_band"])
        inside = asked["value"] - band <= pa["value"] <= asked["value"] + band
        out["draught"] = {"inside": inside, "warn": works and not inside}
    water = boiler_water(rows, held)
    if water is not None and held["auto_hold"] == "boiler" and float(held["auto_drop_c"]) > 0:
        now = rows.get(("thermostat", "setpoint"), {}).get("number", {}).get("value")
        to = float(now if now is not None else held["auto_hold_c"])
        out["water"] = {"inside": to - float(held["auto_drop_c"]) <= water["value"] <= to, "warn": False}
    mv = rows.get(("flame", "pin"), {}).get("number")
    if mv is not None:
        inside = mv["value"] <= float(held["flame_steady"])
        out["flame"] = {"inside": inside, "warn": works and not inside}
    return out


def attention_expect(rows, why, menu, lang, dictionary):
    """What the card of what asks for a person must hold, from the frame: the fault or the burning off with a reason by its CODE (WHY, the value of the part `why`), and the rows at the level bad the menu names. -> [{"key", "action", "level"}], in the page's order."""
    said = menu["scheme"]["attention"]
    out = []
    stage = rows.get(("automaton", "doing"), {}).get("word")
    if stage in ("fault", "burning_off_because"):
        action = word(said["why"][why], lang) if why in said["why"] else word(said["other"], lang)
        out.append({"key": "automaton/doing", "action": action + (" " + word(said["ack"], lang) if stage == "fault" else ""), "level": "bad" if stage == "fault" else "warn"})
    # the rows ask in the dictionary's order, which is the page's
    for thing in dictionary["things"]:
        key = "%s/%s" % (thing["card"], thing["name"])
        if thing.get("part_of") or key not in said["rows"]:
            continue
        action, value = said["rows"][key], rows.get((thing["card"], thing["name"]), {})
        silent = isinstance(value.get("seconds"), (int, float)) and value["seconds"] > 0 and thing["name"].startswith("silent_")
        if silent or (key == "automaton/alarm" and value.get("on") is True):
            out.append({"key": key, "action": word(action, lang), "level": "bad"})
    return out


# ---------------------------------------------------------------- the verdicts: pure functions of what the browser answered

def judge_badge(said, expect):
    """One badge, in the menu's words, in the TITLE ROW of the scheme's card - right of the word of the card, left of the buttons of the mode, on the title's line, over the drawing and not in it -, with its icon on the line before the words; nothing of
    "set ..." left at the boiler (in the drawing it stood over the boiler's number and read as its caption). EXPECT: `mode`, `text`. -> (ok, words)"""
    screen, svg = said["screen"], said["thermo"]["svg"]
    badges = screen["badge"]
    if len(badges) != 1:
        return False, "the card carries %d badges, one was asked" % len(badges)
    one = badges[0]
    out = []
    if one["text"] != expect["text"]:
        out.append("it says '%s', not '%s'" % (one["text"], expect["text"]))
    if one["mode"] != expect["mode"]:
        out.append("it is the badge of '%s', not of '%s'" % (one["mode"], expect["mode"]))
    if one.get("inSvg"):
        out.append("it is drawn inside the scheme, not in the title row of the card")
    box = one["box"]
    header = screen.get("header") or {}
    title, buttons = header.get("title"), header.get("mode")
    if title is None:
        out.append("the title row of the card was not found: where the badge stands was not judged")
    else:
        if abs(middle(box, "y") - middle(title, "y")) > SAME_LINE:
            out.append("it does not stand on the line of the title")
        if box["left"] < title["right"] - EDGE or (one["icon"] and one["icon"]["box"]["left"] < title["right"] - EDGE):
            out.append("it stands over or before the end of the title")
        if buttons is not None and box["right"] > buttons["left"] + EDGE:
            out.append("it runs under the buttons of the mode")
    if box["bottom"] > svg["top"] + EDGE:
        out.append("it stands over the drawing's top edge: it is not over the drawing")
    card = screen.get("schemeCard")
    if card and (box["left"] < card["left"] - EDGE or box["right"] > card["right"] + EDGE):
        out.append("it stands outside the card")
    icon = one["icon"]
    if icon is None:
        out.append("it has no icon")
    else:
        if icon["box"]["width"] < MIN_ICON_PX or icon["box"]["height"] < MIN_ICON_PX:
            out.append("its icon is %dx%d px, under %d" % (icon["box"]["width"], icon["box"]["height"], MIN_ICON_PX))
        if icon["box"]["right"] > box["left"] + EDGE:
            out.append("its icon does not stand before the words")
        if abs(middle(icon["box"], "y") - middle(box, "y")) > SAME_LINE:
            out.append("its icon is not on the line of the words")
    old = [t for t in said["thermo"]["svgTexts"] if OLD_SET.search(t)]
    if old:
        out.append("the drawing still says %s" % old)
    if expect["text"] in said["thermo"]["svgTexts"]:
        out.append("the drawing still says '%s'" % expect["text"])
    return (not out), "; ".join(out) if out else "'%s' in the title row, with its icon" % one["text"]


def judge_pellets(said, expect):
    """Four numbers at the hopper, today, yesterday, the month, in all - in that order, each beside its own icon, no word among them, none smaller than a figure on a phone. EXPECT: the four texts, or None where the board has no pellets. -> (ok, words)"""
    screen, svg = said["screen"], said["thermo"]["svg"]
    items = screen["pellets"]
    if expect is None:
        return (not items), "no pellets drawn" if not items else "pellets are drawn where the board has none: %s" % [i["text"] for i in items]
    if [i["key"] for i in items] != ["today", "yesterday", "month", "total"]:
        return False, "the drawing carries %s, today, yesterday, month and total were asked" % [i["key"] for i in items]
    out = []
    for item, want in zip(items, expect):
        if item["text"] != want:
            out.append("%s says '%s', not '%s'" % (item["key"], item["text"], want))
        if not re.fullmatch(r"[\d.\-]+", item["text"]):
            out.append("%s says a word: '%s'" % (item["key"], item["text"]))
        if item["box"]["right"] > svg["left"] + svg["width"] * LEFT_SHARE:
            out.append("%s does not stand at the hopper" % item["key"])
        if item["fontPx"] < MIN_FIGURE_PX:
            out.append("%s is %.1f px, under %d" % (item["key"], item["fontPx"], MIN_FIGURE_PX))
        icon = item["icon"]
        if icon is None or icon["box"]["width"] < MIN_ICON_PX or icon["box"]["height"] < MIN_ICON_PX:
            out.append("%s has no icon of %d px" % (item["key"], MIN_ICON_PX))
        elif abs(middle(icon["box"], "y") - middle(item["box"], "y")) > SAME_LINE or icon["box"]["right"] > item["box"]["left"] + EDGE:
            out.append("%s's icon does not stand before its number on its line" % item["key"])
    kinds = [i["icon"]["kind"] for i in items if i["icon"]]
    if len(set(kinds)) != len(kinds):
        out.append("two numbers share an icon")
    return (not out), "; ".join(out) if out else "%s at the hopper" % " ".join(expect)


def judge_norm_in_ink(said, deviations):
    """A value in order is in the ink of the card's title; nothing in a card but the drawing is green; a warning and a failure are the colours of their levels. DEVIATIONS: whether the moment holds a row at warn or bad - then one must be seen, or nothing was judged. -> (ok, words)"""
    screen = said["screen"]
    levels = screen["levels"]
    out = []
    ok_rows = [r for r in levels if r["level"] == "ok"]
    if not ok_rows:
        out.append("no value in order was seen: nothing was judged")
    for row in ok_rows:
        if row["color"] != screen["ink"]:
            out.append("'%s' in order is painted %s, not the ink %s" % (row["text"], row["color"], screen["ink"]))
    if screen["greens"]:
        out.append("green text in the cards: %s" % screen["greens"][:4])
    seen = 0
    for row in levels:
        for level in ("warn", "bad"):
            if row["level"] == level:
                seen += 1
                if row["color"] != screen[level]:
                    out.append("'%s' at %s is painted %s, not the colour of its level" % (row["text"], level, row["color"]))
    if deviations and not seen:
        out.append("the moment holds a warning or a failure and none was seen")
    return (not out), "; ".join(out) if out else "%d in order in ink, %d deviations in colour, no green in the cards" % (len(ok_rows), seen)


def judge_zones(said, expect):
    """Each of the three main numbers has its zone as asked: a bar with a band and a mark, the band and the mark inside the bar, the mark in the band or out of it as the numbers say, in the warning colour only where asked. EXPECT: {"draught", "water", "flame"}, each None or {"inside", "warn"}. -> (ok, words)"""
    screen = said["screen"]
    zones = {z["name"]: z for z in screen["zones"]}
    out, told = [], []
    for name in ("draught", "water", "flame"):
        want = expect.get(name)
        got = zones.get(name)
        if want is None:
            if got is not None:
                out.append("%s has a zone where there is none to draw" % name)
            continue
        if got is None or not (got["bar"] and got["band"] and got["mark"]):
            out.append("%s has no zone: a bar, a band and a mark were asked" % name)
            continue
        bar, band, mark = got["bar"], got["band"], got["mark"]
        if bar["width"] < MIN_BAR_PX or band["width"] < MIN_BAND_PX:
            out.append("%s: the bar is %.1f px and the band %.1f px, under %d and %d" % (name, bar["width"], band["width"], MIN_BAR_PX, MIN_BAND_PX))
        if band["left"] < bar["left"] - EDGE or band["right"] > bar["right"] + EDGE:
            out.append("%s: the band leaves the bar" % name)
        centre = middle(mark, "x")
        if centre < bar["left"] - EDGE or centre > bar["right"] + EDGE:
            out.append("%s: the mark leaves the bar" % name)
        inside = band["left"] - EDGE <= centre <= band["right"] + EDGE
        if inside != want["inside"]:
            out.append("%s: the mark stands %s the band, the numbers say %s" % (name, "in" if inside else "out of", "in" if want["inside"] else "out"))
        warn = got["markFill"] == screen["warn"]
        if warn != want["warn"]:
            out.append("%s: the mark is %s the warning colour, it should be %s" % (name, "in" if warn else "not in", "in" if want["warn"] else "out of"))
        if got["markFill"] == screen["ok"]:
            out.append("%s: the mark is green" % name)
        told.append("%s %s%s" % (name, "in" if want["inside"] else "out", " (warn)" if want["warn"] else ""))
    return (not out), "; ".join(out) if out else ", ".join(told) or "no zone asked, none drawn"


def judge_attention(said, expect, title):
    """The card of what asks for a person: not on the page while nothing asks; else under the scheme's card, titled as the menu words it, one row for each thing that asks - each with its action as the menu words it -, outlined in the colour of the worst of them.
    EXPECT: [{"key", "action", "level"}] in the page's order. -> (ok, words)"""
    screen = said["screen"]
    card = screen["attention"]
    if not expect:
        return (card is None), "no card" if card is None else "the card stands with nothing to ask: %s" % [i["key"] for i in card["items"]]
    if card is None:
        return False, "nothing shows what asks for a person: %s" % [e["key"] for e in expect]
    out = []
    if card["title"] != title:
        out.append("the card is titled '%s', not '%s'" % (card["title"], title))
    if [i["key"] for i in card["items"]] != [e["key"] for e in expect]:
        out.append("the card lists %s, %s were asked" % ([i["key"] for i in card["items"]], [e["key"] for e in expect]))
    else:
        for item, want in zip(card["items"], expect):
            if item["action"] != want["action"]:
                out.append("%s: the action is '%s', not '%s'" % (item["key"], item["action"], want["action"]))
            if not item["what"]:
                out.append("%s says nothing of what asks" % item["key"])
    worst = "bad" if any(e["level"] == "bad" for e in expect) else "warn"
    if card["border"] != screen[worst]:
        out.append("the card is outlined %s, not the colour of %s" % (card["border"], worst))
    if card["box"]["top"] < screen["schemeCard"]["bottom"] - EDGE:
        out.append("the card does not stand under the card of the scheme")
    return (not out), "; ".join(out) if out else "%d asking, %s" % (len(expect), ", ".join(e["key"] for e in expect))


def judge_fits(said, width=WIDTH):
    """The page is no wider than the phone. -> (ok, words)"""
    wide = said["thermo"]["wide"]
    return (wide <= width), "%d px wide" % wide


# ---------------------------------------------------------------- the run

def patched(frames_text, dictionary, changes):
    """The frames of a run as text, with the values of some rows set by hand in every frame - the stand goes on for a few seconds while the page loads, so one frame would not hold: CHANGES {(card, name): value as the wire says it}. What the stand cannot make - a draught off its setpoint, the pellets of a season - is
    what the page is asked about there."""
    data = json.loads(frames_text)
    addresses = {(t["card"], t["name"]): n for n, t in enumerate(dictionary["things"]) if not t.get("part_of")}
    for frame in data["frames"]:
        values = dict((a, v) for a, v in frame["values"])
        for key, value in changes.items():
            values[addresses[key]] = value
        frame["values"] = [[a, v] for a, v in values.items()]
    return json.dumps(data)


def main():
    import page_fixes
    from page_stand import dictionary_of
    from page_thermostat import stair_step  # noqa: F401  (the step of a rung, in numbers)
    from stand_frames import named

    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", default=None)
    ap.add_argument("--langs", default="ru,en")
    ap.add_argument("--themes", default="l,d")
    ap.add_argument("--page", default=None, help="the page files to measure instead of the device's own - a revision taken out of git")
    ap.add_argument("--only", default=None, choices=["badge", "pellets", "ink", "zones", "attention"])
    args = ap.parse_args()
    menu = merged_menu("burner")
    dictionary = dictionary_of(ROOT, "burner")
    defaults = {s["name"]: s["default"] for s in dictionary["settings"]}
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    work = tempfile.mkdtemp(prefix="page_screen_")
    made = []

    def laid(scenario, changes=None, settings=None):
        """The frames of SCENARIO (a file of tools/stand_scenarios, or a dict made here with SETTINGS over it) laid in a file; -> (path, frames, scenario file the page's settings are taken from)."""
        src = os.path.join(HERE, "stand_scenarios", scenario)
        if settings:
            data = json.load(open(src))
            data["settings"] = {**data.get("settings", {}), **settings}
            src = os.path.join(work, "scenario_%d.json" % len(made))
            json.dump(data, open(src, "w"))
        text, frames = page_fixes.frames_of(src)
        if changes:
            text = patched(text, dictionary, changes)
            # the rows the page is judged against are the patched ones too
            for frame, as_made in zip(frames, json.loads(text)["frames"]):
                frame["values"] = as_made["values"]
        path = os.path.join(work, "frames_%d.json" % len(made))
        open(path, "w").write(text)
        made.append(path)
        return path, frames, src

    def scenario_held(src):
        return {**defaults, **json.load(open(src)).get("settings", {})}

    runs = []  # (label, check, path, frame, scenario file, seconds the page waits before it is asked)

    def add(label, check, path, frame, src, wait=0.0):
        runs.append((label, check, path, frame, src, wait))

    first = lambda frames, holds: next((f for f in frames if holds(f)), None)  # noqa: E731

    if args.only in (None, "badge"):
        for mode in MODES:
            path, frames, src = laid("heating_steps_down.json", settings={"auto_hold": mode})
            frame = first(frames, lambda f: f["picked"]["doing"] == "working")
            add("working, holds " + mode, "badge", path, frame, src)
    if args.only in (None, "pellets"):
        path, frames, src = laid("pellets.json")
        add("pellets, a day", "pellets", path, frames[-1], src)
        path, frames, src = laid("lighting.json", changes={("pellets", "today"): 1.234, ("pellets", "yesterday"): 3.31, ("pellets", "month"): 37.6, ("pellets", "total"): 412.4})
        add("pellets, a season", "pellets", path, first(frames, lambda f: f["picked"]["doing"] == "working"), src)
        path, frames, src = laid("lighting.json", changes={("pellets", "today"): 1.234, ("pellets", "yesterday"): {"unknown": "none_yet"}, ("pellets", "month"): 37.6, ("pellets", "total"): 412.4})
        add("pellets, no yesterday yet", "pellets", path, first(frames, lambda f: f["picked"]["doing"] == "working"), src)
    if args.only in (None, "ink"):
        path, frames, src = laid("lighting.json")
        add("working", "ink", path, first(frames, lambda f: f["picked"]["doing"] == "working"), src)
        path, frames, src = laid("dark.json")
        add("fault", "ink", path, first(frames, lambda f: f["picked"]["doing"] == "fault"), src)
    if args.only in (None, "zones"):
        path, frames, src = laid("heating_steps_down.json")
        add("working, water in the band", "zones", path, first(frames, lambda f: f["picked"]["doing"] == "working" and (boiler_water(named(dictionary, f), scenario_held(src)) or {}).get("value", 0) >= 56), src)
        add("working, water below the band", "zones", path, first(frames, lambda f: f["picked"]["doing"] == "working"), src)
        path, frames, src = laid("work_flame_gone.json")
        add("working, the flame gone for ten seconds", "zones", path, first(frames, lambda f: f["picked"]["doing"] == "working" and named(dictionary, f).get(("flame", "flame"), {}).get("word") == "out"), src, DWELL_S + PAST_DWELL_S)
        path, frames, src = laid("lighting.json")
        working = first(frames, lambda f: f["picked"]["doing"] == "working")
        add("working, all in", "zones", path, working, src)
        path, frames, src = laid("lighting.json", changes={("draught", "blast_pressure"): 150.0})
        add("working, draught out just now", "zones", path, first(frames, lambda f: f["picked"]["doing"] == "working"), src)
        add("working, draught out for ten seconds", "zones", path, first(frames, lambda f: f["picked"]["doing"] == "working"), src, DWELL_S + PAST_DWELL_S)
        add("rising, draught out", "zones", path, first(frames, lambda f: f["picked"]["doing"] == "rising"), src)
        path, frames, src = laid("lighting.json")
        add("stop, dark pot", "zones", path, first(frames, lambda f: f["picked"]["doing"] == "stop"), src)
    if args.only in (None, "attention"):
        path, frames, src = laid("lighting.json")
        add("working, nothing asks", "attention", path, first(frames, lambda f: f["picked"]["doing"] == "working"), src)
        path, frames, src = laid("dark.json")
        add("fault, never lit", "attention", path, first(frames, lambda f: f["picked"]["doing"] == "fault"), src)
        path, frames, src = laid("work_flue_falls.json")
        add("burning off, no fuel", "attention", path, first(frames, lambda f: f["picked"]["doing"] == "burning_off_because"), src)
        path, frames, src = laid("silent_return.json")
        add("return sensor silent", "attention", path, first(frames, lambda f: named(dictionary, f).get(("thermostat", "silent_return"), {}).get("seconds", 0) > 0), src)
        path, frames, src = laid("steady.json")
        add("alarm", "attention", path, first(frames, lambda f: named(dictionary, f).get(("automaton", "alarm"), {}).get("on") is True), src)
    missing = [r[0] for r in runs if r[3] is None]
    if missing:
        raise SystemExit("page_screen: the runs never come to: " + ", ".join(missing))

    ok = True
    try:
        for lang in args.langs.split(","):
            for theme in args.themes.split(","):
                for label, check, path, frame, src, wait in runs:
                    held = scenario_held(src)
                    rows = named(dictionary, frame)
                    shot = os.path.join(args.shots, "%s_%s_%s.png" % (re.sub(r"\W+", "_", label), lang, theme)) if args.shots else None
                    said = page_fixes.look(path, frame["at"], lang, theme, shot, page=args.page, settings_of=src, after=wait)["first"]
                    where = "%s/%s %s" % (lang, theme, label)
                    if check == "badge":
                        steps = [int(held["auto_step"])] + [int(held["auto_step"]) if held[k] == "chosen" else int(held[k]) for k in ("auto_step_mid", "auto_step_small")]
                        verdicts = [("badge-by-mode", judge_badge(said, badge_expect(rows, menu, lang, held, steps)))]
                    elif check == "pellets":
                        verdicts = [("pellets-at-hopper", judge_pellets(said, pellets_expect(rows)))]
                    elif check == "ink":
                        verdicts = [("norm-in-ink", judge_norm_in_ink(said, rows.get(("automaton", "doing"), {}).get("word") in ("fault", "burning_off_because")))]
                    elif check == "zones":
                        verdicts = [("zone-of-number", judge_zones(said, zones_expect(rows, held, wait >= DWELL_S)))]
                    else:
                        values = dict((a, v) for a, v in frame["values"])
                        why_at = next((n for n, t in enumerate(dictionary["things"]) if t["card"] == "automaton" and t["name"] == "why" and t.get("part_of") == "doing"), None)
                        expect = attention_expect(rows, values.get(why_at), menu, lang, dictionary)
                        verdicts = [("attention-card", judge_attention(said, expect, word(menu["scheme"]["attention"]["title"], lang)))]
                    verdicts.append(("fits-the-phone", judge_fits(said)))
                    for name, (passed, words_) in verdicts:
                        print("%-34s %-18s %s  %s" % (where, name, "PASS" if passed else "FAIL", words_))
                        ok = ok and passed
    finally:
        for path in made:
            if os.path.exists(path):
                os.remove(path)
        for name in os.listdir(work):
            os.remove(os.path.join(work, name))
        os.rmdir(work)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
