"""The lines of the journal a run of the stand would write, before the quiet console and after it: the same seconds counted both ways, by the policies the board runs (crates/flame `voice`, crates/automaton `voice`) -
the table of 'place -> lines in ten minutes before -> after' of the stage.

    uv run --no-project --with pyyaml python tools/journal_lines.py [SCENARIO...]       # file names of tools/stand_scenarios without .json

What it feeds the policies is the stand's own frames: the flame's pin in millivolts, the automaton's `doing`, what the organs are told. The policy is the crates'; here are the seconds, the counting and the oracle the tests hold it against.
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

# runs a person reads the journal of: the two the real boiler burned (27.09, 04.10), a low fire that holds for half an hour, a flame that blinks, one that blinks while rising, three steps of the way up, smouldering
DEFAULT = ["burn_04_10", "burn_27_09", "work_low_fire_flat", "work_flame_blinks", "rising_blinks", "heating_steps_down", "smoulder_and_wait"]
# a word of the flame is settled when it has held this long (crates/flame, voice::SETTLED_S)
SETTLED_S = 5


def organs_of(rows):
    """What the organs are told in a second, as a text that differs when they do: the auger, the igniter, the pump and the fan, the fan's pascals to a whole number as the journal wrote them."""
    fan = rows.get(("automaton", "fan"), {})
    asked = "%.0f Pa" % fan["number"]["value"] if "number" in fan else fan.get("word", "")
    on = lambda key: rows.get(("automaton", key), {}).get("on")  # noqa: E731
    return "%s/%s/%s/%s" % (on("auger"), on("igniter"), on("pump"), asked)


def series_of(scenario):
    """The seconds of SCENARIO as the policies are fed them, and the flame's words the board's own rows say of each (the oracle's input). -> (input for the example, [flame word or None])"""
    import page_fixes
    from page_stand import dictionary_of
    from stand_frames import named

    path = os.path.join(HERE, "stand_scenarios", scenario + ".json")
    _, frames = page_fixes.frames_of(path)
    dictionary = dictionary_of(ROOT, "burner")
    defaults = {s["name"]: s["default"] for s in dictionary["settings"]}
    held = {**defaults, **json.load(open(path)).get("settings", {})}
    seconds, words = [], []
    for frame in frames:
        rows = named(dictionary, frame)
        pin = rows.get(("flame", "pin"), {}).get("number")
        seconds.append({"mv": None if pin is None else pin["value"], "state": rows.get(("automaton", "doing"), {}).get("word", "stop"), "organs": organs_of(rows)})
        words.append(rows.get(("flame", "flame"), {}).get("word"))
    return {"lit_mv": int(held["flame_lit"]), "steady_mv": int(held["flame_steady"]), "seconds": seconds}, words


def lines_of(scenario):
    """The seconds each line would be written at, by the example of burner_api, and the board's flame words. -> (answer, words)"""
    made, words = series_of(scenario)
    out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "journal_lines"], cwd=ROOT, input=json.dumps(made), capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("journal_lines: the example did not run:\n" + out.stderr[-1500:])
    return json.loads(out.stdout), words


def settled_seconds(words):
    """The seconds at which a word of the flame had held for SETTLED_S seconds and was not the word said before: the changes a person would call real, counted from the board's own words and not from the policy. A first word that holds counts as one."""
    found, said, run, run_word = [], None, 0, None
    for at, word in enumerate(words):
        if word is None:
            run, run_word = 0, None
            continue
        run = run + 1 if word == run_word else 1
        run_word = word
        # held for SETTLED_S seconds: the first reading and this one are that far apart, so SETTLED_S + 1 readings of the same word
        if run == SETTLED_S + 1 and word != said:
            found.append(at)
            said = word
    return found


def settled_changes(words):
    """How many of them there are in all."""
    return len(settled_seconds(words))


def in_window(seconds, start, length=60):
    return sum(1 for at in seconds if start <= at < start + length)


def per_ten_minutes(seconds, total):
    """Lines in ten minutes of the run, over the whole of it."""
    return round(len(seconds) * 600 / total, 1) if total else 0.0


def main():
    names = sys.argv[1:] or DEFAULT
    print("%-20s %6s | %-24s | %-24s" % ("run", "min", "flame: was -> info (debug)", "automaton: was -> info (debug)"))
    for name in names:
        got, _ = lines_of(name)
        n = got["seconds"]
        print("%-20s %6.1f | %4d -> %-3d (%-4d)  per 10 min %5.1f -> %-5.1f | %4d -> %-3d (%-4d)  per 10 min %5.1f -> %-5.1f" % (
            name, n / 60, len(got["flame_was"]), len(got["flame_info"]), len(got["flame_debug"]), per_ten_minutes(got["flame_was"], n), per_ten_minutes(got["flame_info"], n),
            len(got["automaton_was"]), len(got["automaton_info"]), len(got["automaton_debug"]), per_ten_minutes(got["automaton_was"], n), per_ten_minutes(got["automaton_info"], n)))


if __name__ == "__main__":
    main()
