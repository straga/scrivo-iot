"""The line of why the burner stands or burns reads like a thermostat: the page on the stand, driven by the new burner's own automaton through its scenarios, asked in each of the cases a person meets what the line under the title's row says.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_reason_line.py [--shots DIR] [--langs ru,en] [--themes l,d] [--page DIR]

The cases are picked BY VALUE, the first frame of a run where the rows say it - a manual cycle to the room's target (light_to_target.json) and to the water's setpoint (light_to_water.json), heat asked and the fire burning, no heat asked
(house_by_the_room.json), heat asked and the fire waiting for the pause to end (room_cold_in_the_pause.json), the switch off, the reasons that have no numbers - the schedule on and off, the profile off, a silent room, a room not read, the sensors not yet asked. In each, in each language and theme, 375 px wide, each verdict printing PASS or FAIL:

  one-row            the line stands on one row: its text is as tall as one line of its font and no wider than the phone
  few-words          the line has at most two words; numbers, units, clocks and signs are not words
  numbers-are-ours   the numbers the line says are the ones a thermostat says - the target, the end of a cycle - or the ones the drawing already says as the readings, and the clock of the pause where the card counts one; no number of a setting,
                     and none said twice: the reading and the target, each once
  now-is-held        the first number of the line is the reading of the sensor the case is about - the room's, the boiler's or the return's by the setting the thermostat holds - and a dash where that sensor reads nothing

What is expected is taken from the stand's frame (the rows by value); the words are the menu's and a person changes them, so a word is never a failing row - how many there are is. The verdicts are `judge_*`: pure functions of what the browser answered,
held by tools/test_page_reason_line.py on answers that are good and answers that are not. Exits 1 when a check failed.
"""
import argparse
import os
import re
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from page_thermostat import degrees  # noqa: E402

WIDTH = 375
# how many words the line may have: the state or the mode, and one more where the state cannot be told without it
MOST_WORDS = 2
# the reading and the target
MOST_NUMBERS = 2
# a text a line tall is as tall as 1.5 lines is not: the browser wraps at a whole line
ONE_LINE = 1.5

# a word is a run of letters; the units of the menu, the clocks and the signs of a thermostat are not words
WORD = re.compile(r"[A-Za-zА-Яа-яЁё]{2,}")
NUMBER = re.compile(r"\d+(?::\d\d)?(?:[.,]\d+)?")

# the cases and what each is found by: name, the scenario its frames are made from, what finds the frame by the rows of a frame, and
#   `patch` - a row of the frames written over, where the board's own run never comes to the state (the sensors "not yet asked all round" are over before the first second of the stand) -,
#   `now` - the role of the sensor the first number must read: "room", "held" (the one the setting `auto_hold` names), or None where the line says no reading,
#   `says` - the keys of the menu's `ask` whose words the line must carry (the fixed words before the first {place} of a text), so a case that found a frame of another state is named and not passed
def why(word):
    return lambda r: r.get(("demand", "why"), {}).get("word") == word


def switched(word):
    return lambda r: r.get(("thermostat", "switch"), {}).get("word") == word


def burning_on(r, *conditions):
    """A frame where the switch is on and no cycle of the person's is run: the house is what the line speaks of."""
    return r.get(("thermostat", "switch"), {}).get("word") == "on" and r.get(("thermostat", "manual"), {}).get("word") not in ("room", "water") and all(c(r) for c in conditions)


def asked(word):
    return lambda r: r.get(("demand", "asked"), {}).get("word") == word


def doing(*words):
    return lambda r: r.get(("automaton", "doing"), {}).get("word") in words


def pause(r):
    return (r.get(("thermostat", "lock"), {}).get("seconds") or 0) > 0


def no_pause(r):
    return not pause(r)


CASES = [
    ("manual room", "light_to_target.json", lambda r: r.get(("thermostat", "manual"), {}).get("word") == "room", None, "room", ["manual/room"]),
    ("manual water", "light_to_water.json", lambda r: r.get(("thermostat", "manual"), {}).get("word") == "water", None, "held", ["manual/water"]),
    ("manual water return", "light_to_water_return.json", lambda r: r.get(("thermostat", "manual"), {}).get("word") == "water", None, "held", ["manual/water"]),
    ("heating", "house_by_the_room.json", lambda r: burning_on(r, asked("cold"), doing("working")), None, "room", ["cold"]),
    ("no heat asked", "house_by_the_room.json", lambda r: burning_on(r, asked("warm")), None, "room", ["warm"]),
    ("waits, no pause", "house_by_the_room.json", lambda r: burning_on(r, asked("cold"), doing("stop"), no_pause), (("automaton", "doing"), "stop"), "room", ["waits", "wait"]),
    ("pause", "room_cold_in_the_pause.json", lambda r: burning_on(r, asked("cold"), pause), None, "room", ["waits", "pause"]),
    ("switched off", "burning_out_then_blow.json", switched("off"), None, None, ["off"]),
    ("nobody pressed Start", "lighting.json", switched("unasked"), None, None, ["unasked"]),
    ("by the schedule", "house_by_the_schedule_plain.json", lambda r: burning_on(r, why("scheduled_on")), None, None, ["why/scheduled_on"]),
    ("off the schedule", "house_by_the_schedule_plain.json", lambda r: burning_on(r, why("scheduled_off")), None, None, ["why/scheduled_off"]),
    ("profile off", "house_profile_off.json", lambda r: burning_on(r, why("profile_off")), None, None, ["why/profile_off"]),
    ("room silent", "house_room_silent.json", lambda r: burning_on(r, why("source_silent")), None, None, ["why/source_silent"]),
    ("room not read", "house_room_unread.json", lambda r: burning_on(r, why("not_read")), None, None, ["why/not_read"]),
    ("sensors not asked", "house_room_unread.json", lambda r: burning_on(r, why("starting")), (("demand", "why"), "starting"), None, ["why/starting"]),
]

# the texts of `scheme/ask` that are said in a line, by the keys of the menu: every one is measured in the browser by a case above (test_page_reason_line.py holds the list against the menu)
ASK_TEXTS = {"off", "unasked", "cold", "warm", "waits", "manual/room", "manual/water", "pause", "wait", "why/profile_off", "why/scheduled_on", "why/scheduled_off", "why/source_silent", "why/not_read", "why/starting"}


def words_of(text):
    """The words of a LINE: runs of letters of two or more; numbers, units (°C), clocks and signs are not."""
    return WORD.findall(text)


def numbers_of(text):
    """The numbers a LINE says, as written: whole, with a decimal, or a clock."""
    return NUMBER.findall(text)


def judge_one_row(said):
    """The line is as tall as one line of its font and no wider than the phone. -> (ok, words)"""
    line = (said["thermo"]).get("ask")
    if line is None:
        return False, "no line stands"
    ink, one = line["ink"], line["lineHeight"]
    out = []
    if one and ink["height"] >= one * ONE_LINE:
        out.append("the text is %.0f px tall, a line of its font is %.0f px: it wraps" % (ink["height"], one))
    if line["box"]["right"] > WIDTH + 0.5 or ink["width"] > WIDTH:
        out.append("the text is %.0f px wide, the phone %d px" % (ink["width"], WIDTH))
    return (not out), "; ".join(out) if out else "'%s' %.0f px wide on one row" % (line["text"], ink["width"])


def judge_few_words(said):
    """The line has at most MOST_WORDS words. -> (ok, words)"""
    line = (said["thermo"]).get("ask")
    if line is None:
        return False, "no line stands"
    found = words_of(line["text"])
    return (len(found) <= MOST_WORDS), "%d words: %s" % (len(found), " ".join(found) if found else "-")


def judge_numbers(said, expect):
    """The numbers of the line are the target and the end of a cycle (EXPECT `ours`, as written) or numbers the drawing already says as readings (its texts), and a clock only where the card counts a pause (EXPECT `pause`); a number of a setting
    of the card of the heat is none of these. -> (ok, words)"""
    line = (said["thermo"]).get("ask")
    if line is None:
        return False, "no line stands"
    # the drawing writes a reading with a decimal and the line may write it whole: they are the same number
    drawn = set(float(n.replace(",", ".")) for t in said["thermo"]["svgTexts"] for n in numbers_of(t) if ":" not in n)
    mine = set(float(n) for n in expect["ours"])
    out = []
    said_numbers = [n for n in numbers_of(line["text"]) if ":" not in n]
    for n in numbers_of(line["text"]):
        if ":" in n:
            if not expect["pause"]:
                out.append("%s is a clock where the card counts no pause" % n)
        elif float(n.replace(",", ".")) not in mine | drawn:
            out.append("%s is neither the target nor a reading on the drawing" % n)
    # the reading and the target, each once: a third number is one of the two said again, or a neighbour's
    if len(said_numbers) > MOST_NUMBERS:
        out.append("%d numbers %s where the reading and the target are %d: one is said again" % (len(said_numbers), " ".join(said_numbers), MOST_NUMBERS))
    if len([n for n in numbers_of(line["text"]) if ":" in n]) > 1:
        out.append("the clock is said more than once")
    return (not out), "; ".join(out) if out else "numbers: %s" % " ".join(numbers_of(line["text"]))


def judge_says(said, expect):
    """The line carries the words of the texts the case is about (EXPECT `says`: the fixed words of each, from the menu in the page's language): a case that found a frame of another state is named, and a line that says nothing of its reason fails. -> (ok, words)"""
    line = (said["thermo"]).get("ask")
    if line is None:
        return False, "no line stands"
    missing = [w for w in expect["says"] if w not in line["text"]]
    return (not missing), ("the line '%s' lacks %s" % (line["text"], ", ".join("'%s'" % w for w in missing))) if missing else "'%s'" % line["text"]


def fixed_words(menu, keys, lang):
    """The words a line must carry for the KEYS of the menu's `ask` in LANG: the text of each up to its first {place}. -> [str]"""
    out = []
    for key in keys:
        node = menu["scheme"]["ask"]
        for part in key.split("/"):
            node = node[part]
        out.append(node[lang].split("{")[0].strip())
    return out


def judge_now(said, expect):
    """The first number of the line is the reading of the sensor the case is about (EXPECT `now`: the reading as a number, or None where that sensor reads nothing): the sensor's own, to the tenth, and a dash where it reads nothing - never the
    number of another sensor. -> (ok, words)"""
    line = (said["thermo"]).get("ask")
    if line is None:
        return False, "no line stands"
    first = re.search(r"(-|\d+(?:[.,]\d+)?)\s*[\u2192/]", line["text"])
    if first is None:
        return False, "the line says no reading before its sign: '%s'" % line["text"]
    got = None if first.group(1) == "-" else float(first.group(1).replace(",", "."))
    wanted = expect["now"]
    if wanted is None or got is None:
        return (wanted is None and got is None), "the line says %s, the sensor reads %s" % (first.group(1), "nothing" if wanted is None else "%.1f" % wanted)
    return (abs(got - wanted) <= 0.06), "the line says %s, the sensor reads %.1f" % (first.group(1), wanted)


def ours(rows):
    """The numbers a line may say from the stand's frame ROWS, as the page writes them: the room's target and the end of a manual cycle."""
    mine = set()
    for key in (("demand", "target"), ("thermostat", "manual_c"), ("thermostat", "setpoint")):
        n = rows.get(key, {}).get("number", {}).get("value")
        if n is not None:
            mine.add(degrees(n))
    return mine


def role_reading(rows, settings, role):
    """The reading of the sensor that plays ROLE (`boiler`, `return`, `room`) in a frame ROWS, by the roles of SETTINGS (`role_ntc_N`): the number, or None where nothing plays it or it reads nothing. The sensor named by a role is found by the
    board's own setting, never by a place on the drawing. -> float or None"""
    for n in range(1, 8):
        if settings.get("role_ntc_%d" % n) == role:
            value = rows.get(("temperatures", "ntc_%d" % n), {}).get("number", {}).get("value")
            return None if value is None else float(value)
    return None


def now_of(kind, rows, settings, defaults):
    """The reading a case's first number must be, by what the board says: the room's, or the sensor the setting `auto_hold` holds (the boiler's, the return's; the mode of power holds none). KIND: "room" or "held". -> float or None"""
    if kind == "room":
        return role_reading(rows, settings, "room")
    return role_reading(rows, settings, {"boiler": "boiler", "return": "return"}.get(settings.get("auto_hold", defaults.get("auto_hold")), ""))


def patched(text, dictionary, patch):
    """The frames TEXT with the row PATCH = ((card, name), word) written over in every frame: the frames of a state the board's own run never comes to."""
    import json

    (card, name), word = patch
    at = next(i for i, t in enumerate(dictionary["things"]) if t["card"] == card and t["name"] == name and not t.get("part_of"))
    doc = json.loads(text)
    for frame in doc["frames"]:
        frame["values"] = [[a, word if a == at else v] for a, v in frame["values"]]
    return json.dumps(doc)


def main():
    import json

    import page_fixes
    from page_sources import merged_menu
    from page_stand import dictionary_of
    from stand_frames import named

    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", default=None)
    ap.add_argument("--langs", default="ru,en")
    ap.add_argument("--themes", default="l,d")
    ap.add_argument("--page", default=None, help="the page files to measure instead of the device's own - a revision taken out of git")
    args = ap.parse_args()
    menu = merged_menu("burner")
    dictionary = dictionary_of(ROOT, "burner")
    defaults = {s["name"]: s["default"] for s in dictionary["settings"]}
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    ok, missing, runs = True, [], []
    for name, scenario, holds, patch, now, says in CASES:
        scenario_file = os.path.join(HERE, "stand_scenarios", scenario)
        text, frames = page_fixes.frames_of(scenario_file)
        if patch:
            text = patched(text, dictionary, patch)
            frames = json.loads(text)["frames"]
        work = tempfile.mkdtemp(prefix="page_reason_line_")
        path = os.path.join(work, "frames.json")
        with open(path, "w") as f:
            f.write(text)
        frame = next((f for f in frames if holds(named(dictionary, f))), None)
        if frame is None:
            missing.append("%s (%s)" % (name, scenario))
            continue
        runs.append((name, path, work, scenario_file, frame, now, says, {**defaults, **json.load(open(scenario_file)).get("settings", {})}))
    if missing:
        raise SystemExit("page_reason_line: the runs never come to: " + ", ".join(missing))
    try:
        for lang in args.langs.split(","):
            for theme in args.themes.split(","):
                for name, path, _, scenario_file, frame, now, says, settings in runs:
                    rows = named(dictionary, frame)
                    shot = os.path.join(args.shots, "%s_%s_%s.png" % (name.replace(" ", "_"), lang, theme)) if args.shots else None
                    said = page_fixes.look(path, frame["at"], lang, theme, shot, page=args.page, settings_of=scenario_file)["first"]
                    where = "%s/%s %s" % (lang, theme, name)
                    verdicts = [("says-its-case", judge_says(said, {"says": fixed_words(menu, says, lang)})), ("one-row", judge_one_row(said)), ("few-words", judge_few_words(said)),
                                ("numbers-are-ours", judge_numbers(said, {"ours": ours(rows), "pause": (rows.get(("thermostat", "lock"), {}).get("seconds") or 0) > 0}))]
                    if now is not None:
                        verdicts.append(("now-is-held", judge_now(said, {"now": now_of(now, rows, settings, defaults)})))
                    for check, (passed, words_) in verdicts:
                        print("%-26s %-17s %s  %s" % (where, check, "PASS" if passed else "FAIL", words_))
                        ok = ok and passed
    finally:
        for _, path, work, _, _, _, _, _ in runs:
            if os.path.exists(path):
                os.remove(path)
            if os.path.isdir(work) and not os.listdir(work):
                os.rmdir(work)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
