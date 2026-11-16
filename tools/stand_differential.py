"""The stand of this tree against the stand of an earlier commit, second by second, on random scenarios.

    uv run --no-project python tools/stand_differential.py SHA [--count 300] [--keep] [--bytes] [--steps] [--clock] [--lock] [--mimic-old]

A change that moves a decision from one crate to another has to move no second of the burner. The sheet of the 38 scenarios of the first burner says so on the few it has;
this plays the same file by `burner_frames` built from SHA (a temporary worktree, removed after) and by this tree's, on COUNT scenarios made up by a seeded generator - the
four holds, the setpoints and hysteresis around which the water walks, a sensor silent for 5 s to 2 min, presses of Start and Stop, a ban of the feed.

THE FRAMES ARE COMPARED BY WHAT EACH ROW IS CALLED, each side read through its own dictionary (`served_json` of its own tree): an address means what the dictionary of the day made it, and a
change that adds a row moves every row after it. What moved on purpose is named here and held as what it is: the thermostat's card is the new side's alone, and whether heat was asked
is the old `automaton/wanted` (on, off) and the new `thermostat/switch` (`on` - asked; `unasked` and `off` - not). With --bytes the frames must be the same bytes instead, for a change
that moves no row (the step that took the decision out of the automaton, `5788b07`, was held so). Exits 1 when one differs, naming the second. The generator knows no answer: the answer is what SHA's code says. A SHA from 6.10 on already has the thermostat's card and is held row by row, but for the rows the new side adds (`rung`, `left` of 6.11; `profile`, `source`, `setpoint` and the window and the end that are parts of the source, 6.12; `lock`, 6.13). With --clock the scenarios carry a wall clock and a window (an eco profile for most of the day): the control of the schedule, which moved the setpoint of the water until 6.19 and moves it no more - against a base before 6.19 it has to be found, and the nine that follow it are the old side's windows at work. **FROM 6.19 THE COMPARISON WITH A BASE BEFORE IT HOLDS THE CASE THAT NOBODY ASKS** (`auto_asked_by` none, the default): every second of the thermostat's and the automaton's cards is the base's, the room is no mode of the water (the generator holds the other three), and the rows that moved to the demand's card are held by name. With --mimic-old the new side is given the numbers of 6.6 that make its
automaton ask what the one of 6.13 asked, as far as numbers can: a flame is a fire after one second (`auto_lit_s` 1), a cold flue is under 100 °C for 300 s and a fall is of 50 °C, the most the card allows (`auto_flue_cold`, `auto_flue_win_s`, `auto_flue_fall`). What numbers cannot
make of the new rule is the old one's exemption of a flue that climbs under 100 °C, and a fall of more than 50 °C inside the window, which the new rule reads as no fuel at once and the old one waited 300 s for: where a scenario has those, the two sides part.

What it cannot see: a fire the scenario does not light (the stand makes up the flame from the scenario's line, not from a pot), the board's tasks, the card's words (the frames carry values).
"""
import argparse
import collections
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def walk(rng, seconds, centre, spread, silent=None, step=10):
    """[[second, value or None]] every STEP seconds: a walk around CENTRE; SILENT is (from, to) where the line says nothing."""
    pts, v = [], centre + rng.uniform(-spread, spread)
    for t in range(0, seconds, step):
        v += rng.uniform(-4, 4)
        v = max(centre - spread, min(centre + spread, v))
        pts.append([t, None if silent and silent[0] <= t < silent[1] else round(v, 1)])
    return pts


def scenario(i, steps=False, clock=False, lock=False):
    """The scenario number I: the same every run. With STEPS the middle and the small step are chosen (5 and 3 kW) over what the scenario holds: the control of the comparison, which has to find what the step down moves. With CLOCK the scenario has a wall clock and a window of eco hours: the control for the schedule. With LOCK the thermostat waits 5 to 40 minutes after the fire goes out before it lights it again: the control for the wait."""
    rng = random.Random(1000 + i)
    seconds = rng.choice([300, 500, 800, 1200])
    # the room is no mode of the water since 6.19: a scenario that held it would be a comparison of a mode with its removal, and the room as a source of the demand has its own scenarios (tests/test_house_asks_on_the_stand.py)
    hold = rng.choice(["power", "boiler", "boiler", "return"])
    want = rng.choice([30, 40, 45, 55, 60, 70])
    drop = rng.choice([2, 4, 4, 8, 12])
    roles = {"role_ntc_1": "boiler", "role_ntc_2": "supply", "role_ntc_3": "return", "role_ntc_4": "room"}
    if rng.random() < 0.15:
        roles.pop(rng.choice(["role_ntc_3", "role_ntc_4"]))
    settings = dict(roles, auto_hold=hold, auto_hold_c=str(want), auto_drop_c=str(drop), auto_step=rng.choice(["14", "10", "8", "6", "5", "4", "3"]),
                    auto_smoulder_s=rng.choice(["0", "30", "90", "300"]))
    if steps:
        settings.update(auto_step_mid="5", auto_step_small="3")
    if lock:
        settings.update(auto_lock_min=str(rng.choice([5, 10, 20, 40])))
    if clock:
        # an eco setpoint over the hysteresis and under the setpoint, for the hours the window names: the day is put through at 1 to 5 minutes a second
        settings.update(auto_eco_c=str(max(drop + 3, want - 10)), window_1="set:daily %02d:%02d-%02d:%02d eco" % (rng.randint(0, 11), rng.choice([0, 30]), rng.randint(13, 23), rng.choice([0, 30])))
    first = rng.randint(2, 40)
    presses = [[first, "start"]]
    t = first
    for _ in range(rng.choice([0, 0, 1, 2, 3])):
        t += rng.randint(60, 350)
        if t < seconds - 5:
            presses.append([t, rng.choice(["stop", "start", "start"])])

    def silence():
        if rng.random() < 0.3:
            a = rng.randint(50, max(60, seconds - 100))
            return (a, a + rng.choice([5, 20, 45, 120]))
        return None

    sc = {"about": "random %d" % i, "seconds": seconds, "press": presses, "settings": settings}
    if clock:
        sc["clock"] = {"start": "%s %02d:%02d" % (rng.choice(["mon", "tue", "wed", "thu", "fri", "sat", "sun"]), rng.randint(0, 23), rng.randint(0, 59)), "minutes_a_second": rng.choice([1, 2, 5])}
    sc["boiler_c"] = walk(rng, seconds, want + 2, 22, silent=silence())
    if "role_ntc_3" in roles:
        sc["return_c"] = walk(rng, seconds, want - 2, 20, silent=silence())
    if "role_ntc_4" in roles:
        sc["room_c"] = walk(rng, seconds, want - 6, 20, silent=silence())
    lit, on = [], False
    for t in range(0, seconds, 2):
        starts = [p[0] for p in presses if p[1] == "start" and p[0] <= t]
        stopped = starts and any(p[1] == "stop" and starts[-1] < p[0] <= t for p in presses)
        if starts and not stopped and t >= starts[-1] + 60 + (i % 7):
            on = (rng.random() >= 0.02) if on else (rng.random() < 0.9)
        else:
            on = False
        lit.append([t, 2600 if on else 150])
    sc["flame_mv"] = lit
    sc["flue_c"] = [[t, 140 if v > 1000 else 30] for t, v in lit]
    if rng.random() < 0.2:
        sc["feed"] = [[rng.randint(20, seconds - 20), False], [rng.randint(seconds - 19, seconds - 1), True]]
    return sc


# The numbers that make the automaton of 6.6 ask what the one of 6.13 asked, as far as numbers go (`--mimic-old`): see the head of this file.
MIMIC_OLD = {"auto_lit_s": "1", "auto_flue_cold": "100", "auto_flue_win_s": "300", "auto_flue_fall": "50"}


def build(tree):
    subprocess.run(["cargo", "+stable", "build", "-q", "-p", "burner_api", "--example", "burner_frames", "--example", "served_json"], cwd=tree, check=True)
    return os.path.join(tree, "target", "debug", "examples", "burner_frames")


def dictionary(tree):
    out = subprocess.run([os.path.join(tree, "target", "debug", "examples", "served_json")], cwd=tree, capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def named(dic, frames):
    """FRAMES second by second as [{card/name: value}], each second carrying what the earlier ones last said."""
    keys = {}
    for at, t in enumerate(dic["things"]):
        keys[at] = "%s/%s.%s" % (t["card"], t["part_of"], t["name"]) if t.get("part_of") else "%s/%s" % (t["card"], t["name"])
    state, out = {}, []
    for frame in frames:
        for at, value in frame["values"]:
            state[keys[at]] = value
        out.append(dict(state))
    return out


# What moved on purpose: the rows that are the old automaton's and are the thermostat's now, and the thermostat's own.
MOVED_OLD = {"automaton/wanted", "automaton/start", "automaton/stop"}
# The rows a SHA before 6.15 says on the automaton's card and the new side says on the thermostat's: the silence of the return and the air.
MOVED_CARD = {"automaton/silent_return": "thermostat/silent_return", "automaton/silent_room": "demand/silent_room", "thermostat/silent_room": "demand/silent_room"}
# What the new side adds to a base that already has the thermostat's card: the level its order is on and how far the water is; the profile that stands, what chose it with the window and its end, and the setpoint.
# From 6.19 the profile that stands, what chose it and the target belong to the demand's card, which is the new side's alone, and nobody asking says none of them: what the old side says of them is held by the scenarios of the stand, not by this comparison.
NEW_ONLY = {"thermostat/rung", "thermostat/left", "thermostat/profile", "thermostat/source", "thermostat/source.window", "thermostat/source.until", "thermostat/setpoint", "thermostat/lock",
            "demand/asked", "demand/why", "demand/profile", "demand/source", "demand/source.window", "demand/source.until", "demand/target"}


def same_by_name(old_dic, old_frames, new_dic, new_frames):
    """The seconds at which the two sides say something else of a row both have: [(second, name, old, new)]."""
    old, new = named(old_dic, old_frames), named(new_dic, new_frames)
    # what moved from one card to another is held as what it is: the old side's row by the name the new side gives it, so the seconds it says are still compared
    old = [{MOVED_CARD.get(name, name): value for name, value in second.items()} for second in old]
    found = []
    if len(old) != len(new):
        return [(min(len(old), len(new)), "seconds", len(old), len(new))]
    for second, (o, n) in enumerate(zip(old, new)):
        has_card = "thermostat/switch" in o
        for name in sorted(set(o) | set(n)):
            # a base from before the thermostat has none of its card: it is the new side's alone; a base that has it is held row by row, but for what only the new side says
            if name in NEW_ONLY or (name in MOVED_OLD or name.startswith("thermostat/")) and not has_card:
                continue
            if o.get(name, "<nothing>") != n.get(name, "<nothing>"):
                found.append((second, name, o.get(name, "<nothing>"), n.get(name, "<nothing>")))
        if has_card:
            continue
        asked_then = o.get("automaton/wanted", {})
        asked_now = n.get("thermostat/switch")
        # a board nobody pressed anything on says `unasked`, the old one said "not wanted" - on and nothing else is wanted
        if (asked_then == {"on": True} or asked_then is True) != (asked_now == {"word": "on"} or asked_now == "on"):
            found.append((second, "wanted/switch", asked_then, asked_now))
    return found


def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("sha")
    ap.add_argument("--count", type=int, default=300)
    ap.add_argument("--keep", action="store_true", help="keep the worktree and the frames")
    ap.add_argument("--steps", action="store_true", help="the control: the same scenarios with the middle and the small step chosen, which the comparison has to find")
    ap.add_argument("--clock", action="store_true", help="the control for the schedule: the same scenarios with a wall clock and an eco window, which the comparison has to find")
    ap.add_argument("--lock", action="store_true", help="the control for the wait before a light: the same scenarios with a wait of 5 to 40 minutes, which the comparison has to find")
    ap.add_argument("--mimic-old", action="store_true", help="give the new side the numbers of 6.6 that make it ask what the old one asked (a flame is a fire after 1 s, a cold flue under 100 °C for 300 s, a fall of 50 °C): the differences that remain are the ones numbers cannot take away")
    ap.add_argument("--bytes", action="store_true", help="the same bytes, and not the same rows by name")
    args = ap.parse_args(argv)
    work = tempfile.mkdtemp(prefix="stand_differential_")
    old_tree = os.path.join(work, "old")
    subprocess.run(["git", "worktree", "add", "--detach", old_tree, args.sha], cwd=ROOT, check=True, capture_output=True)
    differ = []
    tally = collections.Counter()
    kinds = collections.Counter()
    try:
        old, new = build(old_tree), build(ROOT)
        old_dic, new_dic = dictionary(old_tree), dictionary(ROOT)
        for i in range(args.count):
            path = os.path.join(work, "s%03d.json" % i)
            with open(path, "w") as f:
                json.dump(scenario(i, args.steps, args.clock, args.lock), f)
            new_path = path
            if args.mimic_old:
                sc = scenario(i, args.steps, args.clock, args.lock)
                sc["settings"].update(MIMIC_OLD)
                new_path = os.path.join(work, "s%03d_new.json" % i)
                with open(new_path, "w") as f:
                    json.dump(sc, f)
            frames = [subprocess.run([binary, p], capture_output=True) for binary, p in ((old, path), (new, new_path))]
            if [r.returncode for r in frames] != [0, 0]:
                differ.append((i, [r.returncode for r in frames]))
            elif args.bytes:
                if frames[0].stdout != frames[1].stdout:
                    differ.append((i, "bytes"))
            else:
                new_frames = json.loads(frames[1].stdout)["frames"]
                found = same_by_name(old_dic, json.loads(frames[0].stdout)["frames"], new_dic, new_frames)
                if found:
                    differ.append((i, found[:3]))
                    tally.update(name for _, name, _, _ in found)
                    # what the new side says at the first second the two part, as the row of the cause: the kind of difference, not its seconds
                    said = named(new_dic, new_frames)[found[0][0]]
                    kinds[(said.get("automaton/stage"), said.get("automaton/doing.why"))] += 1
    finally:
        if not args.keep:
            subprocess.run(["git", "worktree", "remove", "--force", old_tree], cwd=ROOT, capture_output=True)
            shutil.rmtree(work, ignore_errors=True)
    for i, why in differ[:20]:
        print("scenario %d differs: %s" % (i, why))
    if kinds:
        print("what the new side says at the first second the two part - (stage, why), and how many scenarios: %s" % dict(kinds.most_common()))
    if tally:
        print("rows that differ, with the seconds they differ at over all scenarios: %s" % dict(tally.most_common()))
    print("%d scenarios against %s: %d differ" % (args.count, args.sha, len(differ)))
    return 1 if differ else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
