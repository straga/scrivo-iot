"""The scheme carries the thermostat on the drawing, and no text stands under it: the page on the stand, driven by the new burner's own automaton through its scenarios, asked in each of the eight states what stands in the
drawing, under it and in the title's row.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_thermostat.py [--shots DIR] [--langs ru,en] [--themes l,d] [--page DIR] [--only stair|notes]

The states are picked BY VALUE, the first frame of a run where the automaton's `doing` says it - stop, purge, kindling, rising, working (tools/stand_scenarios/lighting.json), smouldering (smoulder_and_wait.json), burning off and fault (dark.json: no flame at
any second). In each, in each language and theme, 375 px wide, each verdict printing PASS or FAIL:

  nothing-under      no block and no line of text stands under the drawing, between it and the buttons of Start and Stop: what the thermostat holds is on the drawing
 (the setpoint the thermostat holds stands as the badge at the top of the drawing, and tools/page_screen.py asks it: nothing of it stands at the boiler)
  step-at-pot        while the fire works the drawing says the step and, where the next step is another one, the water it burns until; in no other state it says a step; on the way up on three steps (tools/stand_scenarios/heating_steps_down.json), at the
                     first second of work on each rung it says that rung's step and its threshold, the last step none
  notes-at-boiler    a window of the schedule that is not the main profile (house_by_the_schedule.json) and the wait before a light (cooling_waits.json) are said small at the boiler, only while they act - none in the states where neither does
  nothing-overlaps   none of the thermostat's numbers on the drawing stands over another text of it, or outside it
  no-old-words       the words of the old block - "off", "burn on", "hysteresis", "comfort, main profile" - are not in the card of the scheme
  no-mode-on-scheme  the card of the scheme has no Auto / Semi switch in its title's row, and the drawing says neither word: the hands are the System tab's Service
  fits-the-phone     the page is no wider than the phone
  ask-line           why the burner stands or burns stands as one line over the drawing, under the title's row, by the switch's and the demand's rows' values - and none where there is nothing to say
  card-height-steady the card of the scheme is as tall in every state as in the next (the eight states, the window of the schedule, the wait, the three steps of the way up); the states where the thermostat asks are held apart, as tall as one another: a line the setting of who asks
                     puts there is the same line in every state of the fire

What is expected is taken from the stand's frame (the thermostat's rows by value) and the menu's words in the language: so a word a person changes is not a failing row. The verdicts are `judge_*`: pure functions of what the browser answered, held by
tools/test_page_thermostat.py on answers that are good and answers that are not. Exits 1 when a check failed.
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
# how much the height of the card may differ between two states: a pixel of rounding, not a line
HEIGHT_STEADY = 1.5
# how much two texts of the drawing may overlap and still be said to stand apart: the boxes of glyphs touch by a pixel
OVERLAP = 1.0

# the words of the old block under the drawing, by language: what the owner saw contradict itself on 04.10 (a switch "off" over an order "burn on 6 kW")
OLD_WORDS = {"ru": ["выключен", "гореть на", "гистерезис", "основной профиль"], "en": ["hysteresis", "burn on", "main profile"]}

# the scenario each state is found in, and the words of the automaton's `doing` that are it (a burning off with its reason is a burning off)
STATES = [
    ("lighting.json", [("stop", {"stop"}), ("purge", {"purge"}), ("kindling", {"kindling"}), ("rising", {"rising"}), ("working", {"working"})]),
    ("smoulder_and_wait.json", [("smouldering", {"smouldering"})]),
    ("dark.json", [("burning_off", {"burning_off", "burning_off_because"}), ("fault", {"fault"})]),
]

# a window of the schedule that holds a setpoint other than the setting's: the drawing must say the frame's setpoint, and the window as a note
PROFILE = ("house_by_the_schedule.json", "window")

# the wait before a light: the first frame whose card counts one down
WAIT = ("cooling_waits.json", "wait")

# what the house asks of the boiler: the first frame where it says cold, and the first where it says warm, in the run of a room that reaches its target
ASKS = ("house_by_the_room.json", [("asks cold", "cold"), ("asks warm", "warm")])

# the way up on three steps: the drawing says the step of the rung the fire is on and the next threshold, asked at the first second of work on each rung
STAIRS = ("heating_steps_down.json", ["big", "mid", "small"])


def degrees(n):
    """A number of degrees as the page writes it: whole where it is whole, one decimal otherwise (scheme.js, degrees)."""
    n = float(n)
    return str(int(n)) if n == int(n) else str(round(n * 10) / 10)


def spell(template, **names):
    """A text with its {places} filled from NAMES, as the page fills the menu's words."""
    return re.sub(r"\{(\w+)\}", lambda m: str(names[m.group(1)]) if m.group(1) in names else m.group(0), template)


def stair_step(rung, to, steps, far):
    """What the drawing says at the pot on RUNG, in numbers: the step in kilowatts and the water the step burns until - where the next DIFFERENT step begins: the setpoint TO less its distance -, None on the last. STEPS: the kilowatts of big,
    mid and small; FAR: the degrees under the setpoint mid and small begin at. -> {"kw", "next" (None on the last)}"""
    order = ["big", "mid", "small"]
    here = order.index(rung)
    fars = [None, far[0], far[1]]
    ahead = next((i for i in range(here + 1, 3) if steps[i] != steps[here]), None)
    return {"kw": steps[here], "next": None if ahead is None else to - fars[ahead]}


def step_words(menu, lang, numbers, unit):
    """The text of the step at the pot from its NUMBERS (`stair_step`), in the menu's words."""
    said = menu["scheme"]["pot"]
    word = lambda node: node[lang] if isinstance(node, dict) else node  # noqa: E731
    if numbers["next"] is None:
        return spell(word(said["step_last"]), kw=numbers["kw"])
    return spell(word(said["step"]), kw=numbers["kw"], next=degrees(numbers["next"]), unit=unit)


def judge_nothing_under(said):
    """No block and no line of text stands under the drawing. -> (ok, words)"""
    thermo = said["thermo"]
    out = []
    if thermo.get("blocks"):
        out.append("a block of the thermostat stands in the card")
    out += ["a line '%s' stands under the drawing" % line["text"] for line in thermo.get("under", [])]
    return (not out), "; ".join(out) if out else "nothing between the drawing and the buttons"


def judge_step(said, expect):
    """While the fire works the drawing says the step as asked; in no other state it says one. EXPECT: `step` - the text, or None where the fire does not work. -> (ok, words)"""
    drawn = [d for d in said["thermo"]["drawn"] if d["kind"] == "step"]
    if expect["step"] is None:
        return (not drawn), "no step in this state" if not drawn else "the drawing says a step '%s' where the fire does not work" % drawn[0]["text"]
    if len(drawn) != 1:
        return False, "the drawing says the step %d times, once was asked" % len(drawn)
    same = drawn[0]["text"] == expect["step"]
    return same, ("'%s'" % drawn[0]["text"]) if same else "the drawing says '%s', not '%s'" % (drawn[0]["text"], expect["step"])


def clock_seconds(text):
    """The seconds a clock text 'm:ss' or 'h:mm:ss' says, found in TEXT; None where there is none."""
    found = re.search(r"(?:(\d+):)?(\d+):(\d\d)", text)
    if not found:
        return None
    hours, minutes, seconds = found.groups()
    return int(hours or 0) * 3600 + int(minutes) * 60 + int(seconds)


def judge_notes(said, expect, drift=15):
    """The small notes at the boiler are the ones asked, in order, and none else. EXPECT: `notes` - the texts of a window, as it must be said -, `wait` - the seconds the card counts down, or None. The wait counts down on the page after the
    frame was drawn, so it is read within DRIFT seconds under the frame's. -> (ok, words)"""
    notes = [d["text"] for d in said["thermo"]["drawn"] if d["kind"] == "note"]
    asked = list(expect["notes"]) + ([None] if expect["wait"] is not None else [])
    if len(notes) != len(asked):
        return False, "the drawing says %d notes %s, %d were asked" % (len(notes), notes, len(asked))
    out = []
    for got, wanted in zip(notes, asked):
        if wanted is not None:
            if got != wanted:
                out.append("a note says '%s', not '%s'" % (got, wanted))
        else:
            left = clock_seconds(got)
            if left is None or not expect["wait"] - drift <= left <= expect["wait"]:
                out.append("the wait '%s' is not within %d s under the %d s of the frame" % (got, drift, expect["wait"]))
    return (not out), "; ".join(out) if out else "notes %s" % notes


def intersect(a, b):
    width = min(a["right"], b["right"]) - max(a["left"], b["left"])
    height = min(a["bottom"], b["bottom"]) - max(a["top"], b["top"])
    return width > OVERLAP and height > OVERLAP


def judge_no_overlap(said):
    """None of the thermostat's numbers on the drawing stands over another text of it, or outside the drawing. -> (ok, words)"""
    thermo = said["thermo"]
    svg = thermo["svg"]
    out = []
    for one in thermo["drawn"]:
        box = one["box"]
        if box["left"] < svg["left"] - 0.5 or box["right"] > svg["right"] + 0.5 or box["top"] < svg["top"] - 0.5 or box["bottom"] > svg["bottom"] + 0.5:
            out.append("'%s' stands outside the drawing" % one["text"])
        for other in thermo["texts"]:
            if other["text"] == one["text"] and other["box"] == box:
                continue
            if intersect(box, other["box"]):
                out.append("'%s' stands over '%s'" % (one["text"], other["text"]))
    return (not out), "; ".join(out) if out else "%d numbers, none over another text" % len(thermo["drawn"])


def judge_no_old_words(said, lang):
    """The words of the old block are not in the card of the scheme. -> (ok, words)"""
    text = said["thermo"]["cardText"].lower()
    found = [w for w in OLD_WORDS[lang] if w in text]
    return (not found), ("the card still says: " + ", ".join(found)) if found else "none of the old words"


def asks(rows):
    """Whether the line over the drawing has something to say, from the stand's frame ROWS (by card and name, values): the switch is off or was never pressed, or the house asks for heat or none. The words are the menu's and a person
    changes them, so what is held is that the line stands where it has something to say, is silent where it has not, and stands in its place. -> bool"""
    if rows.get(("thermostat", "switch"), {}).get("word") in ("off", "unasked"):
        return True
    return rows.get(("demand", "asked"), {}).get("word") in ("cold", "warm")


def judge_ask(said, expect):
    """The line of why the burner stands or burns stands over the drawing, under the title's row, and says something - and no line stands where there is nothing to say. EXPECT: `ask` - whether it must stand. -> (ok, words)"""
    thermo = said["thermo"]
    line = thermo.get("ask")
    if not expect["ask"]:
        return (line is None), "no line: nothing to say" if line is None else "the card says '%s' where there is nothing to say" % line["text"]
    if line is None or not line["text"].strip():
        return False, "the card says nothing of why the burner stands or burns"
    out = []
    if thermo["title"] and line["box"]["top"] < thermo["title"]["box"]["bottom"] - 1:
        out.append("the line is not under the title's row")
    if thermo["svg"] and line["box"]["bottom"] > thermo["svg"]["top"] + 1:
        out.append("the line is not over the drawing")
    return (not out), "; ".join(out) if out else "'%s'" % line["text"]


def judge_card_height(heights):
    """The card of the scheme is as tall in every state of the automaton as in the next: what changes its height moves the cards and the buttons under it. HEIGHTS: {state: px}. -> (ok, words)"""
    known = {state: h for state, h in heights.items() if h is not None}
    if not known or len(known) != len(heights):
        return False, "the card was not measured in %s" % ", ".join(sorted(s for s in heights if heights[s] is None) or ["any state"])
    low, high = min(known.values()), max(known.values())
    if high - low <= HEIGHT_STEADY:
        return True, "%.0f px in %d states" % (high, len(known))
    tall = sorted(s for s, h in known.items() if h > high - HEIGHT_STEADY)
    short = sorted(s for s, h in known.items() if h <= high - HEIGHT_STEADY)
    return False, "the card is %.0f px taller in %s than in %s" % (high - low, ", ".join(tall), ", ".join(short))


# the words the mode switch once had, in both languages: a drawing that says one of them says a switch that is not on the scheme any more
MODE_WORDS = ("Auto", "Semi", "Авто", "Полу")


def judge_no_mode_on_scheme(said):
    """The title's row of the scheme's card has no Auto / Semi switch - the hands are the Service's - and the drawing says none of its words. -> (ok, words)"""
    thermo = said["thermo"]
    out = []
    if thermo["mode"]:
        out.append("the card still has a switch '%s'" % thermo["mode"]["text"])
    inside = [t for t in thermo["svgTexts"] if t in MODE_WORDS]
    if inside:
        out.append("the drawing still says %s" % inside)
    return (not out), "; ".join(out) if out else "no mode switch in the card, none of its words in the drawing"


def judge_fits(said, width=WIDTH):
    """The page is no wider than the phone. -> (ok, words)"""
    wide = said["thermo"]["wide"]
    return (wide <= width), "%d px wide" % wide


def expectations(rows, menu, lang, defaults, steps=None, far=None):
    """What a state must say on the drawing, from the stand's frame ROWS (by card and name, values) and the menu in LANG. DEFAULTS: the stand's settings by name - and what a scenario sets over them, STEPS and FAR the same for the steps of the way up
    (kilowatts of big, mid, small; degrees under the setpoint mid and small begin at). -> {"step"}"""
    word = lambda node: node[lang] if isinstance(node, dict) else node  # noqa: E731
    unit = word(menu["units"]["°C"])
    # the setpoint is the one the thermostat holds NOW - a profile's, a window's - and the setting only while the card says none
    held = rows.get(("thermostat", "setpoint"), {}).get("number", {}).get("value")
    to = float(held if held is not None else defaults["auto_hold_c"])
    kws = steps or [int(defaults["auto_step"])] + [int(defaults["auto_step"]) if defaults[k] == "chosen" else int(defaults[k]) for k in ("auto_step_mid", "auto_step_small")]
    distances = far or [int(defaults["auto_mid_c"]), int(defaults["auto_small_c"])]
    stage = rows.get(("automaton", "doing"), {}).get("word")
    step = None
    if stage == "working":
        rung = rows.get(("thermostat", "rung"), {}).get("word")
        step = step_words(menu, lang, stair_step(rung if rung in ("big", "mid", "small") else "big", to, kws, distances), unit)
    return {"step": step}


def part_value(dictionary, frame, card, row, part):
    """The value of a PART of a row of a CARD in a FRAME, by the dictionary's addresses; None where the frame says nothing of it."""
    values = dict((at, v) for at, v in frame["values"])
    for at, thing in enumerate(dictionary["things"]):
        if thing["card"] == card and thing.get("part_of") == row and thing["name"] == part:
            return values.get(at)
    return None


def window_note(rows, menu, lang, dictionary, frame):
    """The note a window of the schedule is said by, in the menu's words: the profile and the minute the window ends at, as 'eco until 06:00' - or None where the main profile sets the setpoint. -> str or None"""
    word = lambda node: node[lang] if isinstance(node, dict) else node  # noqa: E731
    if rows.get(("demand", "source"), {}).get("word") != "window":
        return None
    said = menu["modules"]["demand"]["rows"]
    code = rows[("demand", "profile")]["word"]
    profile = next(word(w) for c, w in said["profile"]["says"] if c == code)
    until = part_value(dictionary, frame, "demand", "source", "until")
    minutes = int(until["number"] if isinstance(until, dict) else until)
    return spell(word(said["source"]["short"]), profile=profile).replace("{until:daytime}", "%02d:%02d" % (minutes // 60 % 24, minutes % 60))


def main():
    import page_fixes
    from page_stand import dictionary_of
    from stand_frames import named

    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", default=None)
    ap.add_argument("--langs", default="ru,en")
    ap.add_argument("--themes", default="l,d")
    ap.add_argument("--page", default=None, help="the page files to measure instead of the device's own - a revision taken out of git")
    ap.add_argument("--only", default=None, choices=["stair", "notes"], help="ask the step of the way up alone, or the notes alone")
    args = ap.parse_args()
    menu = merged_menu("burner")
    dictionary = dictionary_of(ROOT, "burner")
    defaults = {s["name"]: s["default"] for s in dictionary["settings"]}
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    ok, missing, runs, with_settings = True, [], [], {}

    def laid(scenario):
        text, frames = page_fixes.frames_of(os.path.join(HERE, "stand_scenarios", scenario))
        work = tempfile.mkdtemp(prefix="page_thermostat_")
        path = os.path.join(work, "frames.json")
        with open(path, "w") as f:
            f.write(text)
        return path, work, frames

    if not args.only:
        for scenario, states in STATES:
            path, work, frames = laid(scenario)
            # the page holds the settings the frames were made with - the roles of the sockets among them, which put the boiler's number on the drawing
            with_settings[path] = os.path.join(HERE, "stand_scenarios", scenario)
            for state, doings in states:
                frame = next((f for f in frames if f["picked"]["doing"] in doings), None)
                if frame is None:
                    missing.append("%s (%s)" % (state, scenario))
                    continue
                runs.append((state, path, frame, work))
    if args.only in (None, "notes"):
        # the window of the schedule: the first frame whose source says a window
        profile_file = os.path.join(HERE, "stand_scenarios", PROFILE[0])
        path, work, frames = laid(PROFILE[0])
        by_window = next((f for f in frames if named(dictionary, f).get(("demand", "source"), {}).get("word") == "window"), None)
        if by_window is None:
            missing.append("%s (%s)" % (PROFILE[1], PROFILE[0]))
        else:
            runs.append((PROFILE[1], path, by_window, work))
            with_settings[path] = profile_file
        # what the house asks: the room that is cold, and the room that has reached its target
        asks_file = os.path.join(HERE, "stand_scenarios", ASKS[0])
        path, work, frames = laid(ASKS[0])
        with_settings[path] = asks_file
        for state, wanted in ASKS[1]:
            found = next((f for f in frames if named(dictionary, f).get(("demand", "asked"), {}).get("word") == wanted), None)
            if found is None:
                missing.append("%s (%s)" % (state, ASKS[0]))
            else:
                runs.append((state, path, found, work))
        # the wait before a light: the first frame whose card counts one down
        wait_file = os.path.join(HERE, "stand_scenarios", WAIT[0])
        path, work, frames = laid(WAIT[0])
        waiting = next((f for f in frames if (named(dictionary, f).get(("thermostat", "lock"), {}).get("seconds") or 0) > 0), None)
        if waiting is None:
            missing.append("%s (%s)" % (WAIT[1], WAIT[0]))
        else:
            runs.append((WAIT[1], path, waiting, work))
            with_settings[path] = wait_file
    # the way up: the first frame of work on each rung, with what the drawing must say on it from the scenario's own settings
    stair_runs, stair_path, scenario_file = [], None, os.path.join(HERE, "stand_scenarios", STAIRS[0])
    if args.only in (None, "stair"):
        scenario = json.load(open(scenario_file))
        held = {**defaults, **scenario["settings"]}
        steps = [int(held["auto_step"])] + [int(held["auto_step"]) if held[k] == "chosen" else int(held[k]) for k in ("auto_step_mid", "auto_step_small")]
        far = [int(held["auto_mid_c"]), int(held["auto_small_c"])]
        stair_path, stair_work, frames = laid(STAIRS[0])
        for rung in STAIRS[1]:
            frame = next((f for f in frames if f["picked"]["doing"] == "working" and named(dictionary, f).get(("thermostat", "rung"), {}).get("word") == rung), None)
            if frame is None:
                missing.append("working on %s (%s)" % (rung, STAIRS[0]))
                continue
            stair_runs.append((rung, frame, steps, far))
        runs.append(("stairs", stair_path, None, stair_work))
    if missing:
        raise SystemExit("page_thermostat: the runs never come to: " + ", ".join(missing))
    try:
        for lang in args.langs.split(","):
            word = lambda node: node[lang] if isinstance(node, dict) else node  # noqa: E731
            unit = word(menu["units"]["°C"])
            for theme in args.themes.split(","):
                heights, ask_heights = {}, {}
                for rung, frame, steps, far in stair_runs:
                    rows = named(dictionary, frame)
                    said = page_fixes.look(stair_path, frame["at"], lang, theme, None, page=args.page, settings_of=scenario_file)["first"]
                    heights["working on " + rung] = ((said.get("thermo") or {}).get("card") or {}).get("height")
                    to = float(rows[("thermostat", "setpoint")]["number"]["value"])
                    passed, words_ = judge_step(said, {"step": step_words(menu, lang, stair_step(rung, to, steps, far), unit)})
                    print("%-22s %-18s %s  %s" % ("%s/%s %s" % (lang, theme, "working on " + rung), "step-at-pot", "PASS" if passed else "FAIL", words_))
                    ok = ok and passed
                for state, path, frame, _ in [r for r in runs if r[0] != "stairs"]:
                    rows = named(dictionary, frame)
                    shot = os.path.join(args.shots, "%s_%s_%s.png" % (state, lang, theme)) if args.shots else None
                    said = page_fixes.look(path, frame["at"], lang, theme, shot, page=args.page, settings_of=with_settings.get(path))["first"]
                    expect_ask = {"ask": asks(rows)}
                    (ask_heights if expect_ask["ask"] else heights)[state] = ((said.get("thermo") or {}).get("card") or {}).get("height")
                    where = "%s/%s %s" % (lang, theme, state)
                    expect = expectations(rows, menu, lang, defaults)
                    note = window_note(rows, menu, lang, dictionary, frame)
                    lock = rows.get(("thermostat", "lock"), {}).get("seconds")
                    notes = {"notes": [note] if note else [], "wait": lock if lock else None}
                    verdicts = [("nothing-under", judge_nothing_under(said)), ("step-at-pot", judge_step(said, expect)),
                                ("notes-at-boiler", judge_notes(said, notes)), ("nothing-overlaps", judge_no_overlap(said)), ("no-old-words", judge_no_old_words(said, lang)),
                                ("no-mode-on-scheme", judge_no_mode_on_scheme(said)), ("fits-the-phone", judge_fits(said)), ("ask-line", judge_ask(said, expect_ask))]
                    for check, (passed, words_) in verdicts:
                        print("%-22s %-18s %s  %s" % (where, check, "PASS" if passed else "FAIL", words_))
                        ok = ok and passed
                passed, words_ = judge_card_height(heights)
                print("%-22s %-18s %s  %s" % ("%s/%s all states" % (lang, theme), "card-height-steady", "PASS" if passed else "FAIL", words_))
                ok = ok and passed
                if ask_heights:
                    passed, words_ = judge_card_height(ask_heights)
                    print("%-22s %-18s %s  %s" % ("%s/%s asking states" % (lang, theme), "card-height-steady", "PASS" if passed else "FAIL", words_))
                    ok = ok and passed
    finally:
        for _, path, _, work in runs:
            if os.path.exists(path):
                os.remove(path)
            if os.path.isdir(work) and not os.listdir(work):
                os.rmdir(work)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
