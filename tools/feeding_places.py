"""Every place in the shipped code (no tests, no examples) where the auger is told what to do or the relay's owner keeps what it was told: the class "a word of the automaton that lives without a term".

    uv run --no-project python tools/feeding_places.py            # file:line  what

It reads the Rust sources line by line for the names of the road - the types `Feeding` and `Told`, the words `feed`, `still`, `plan`, `tell`, `auger_still`, the owner's `step` and `HORIZON_MS` - and says which of the three things each line is: the SAYING (a word is made or handed on), the KEEPING (the task holds a plan) or the TERM (the horizon after which the plan stands for nothing). A place that keeps without a term, or says without a confirmation beside it, is what `tools/test_feeding_places.py` fails on.
What it cannot see: a name made by a macro, a word handed on under another name, and C. The road is short and by a type, so the compiler's own check (a required `auger_still` on `Hands`) stands beside it.
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SKIP_DIRS = {"target", "build", "tests", "examples", "node_modules", ".git"}

SAYING = re.compile(r"\bFeeding::(?:Stop|Dose|Rhythm)\b|\bTold::(?:Say|Still)\b|\bfn auger\b|\bfn auger_still\b|\.auger_still\(|\.auger\(|relay::board::tell\(|Organs::tell\(")
KEEPING = re.compile(r"\bself\.feeds\b|\bMutex<\[Feed\b")
TERM = re.compile(r"\bHORIZON_MS\b|\bf\.end\b|\bend_first\b")


def sources():
    for base in ("crates", "devices"):
        for folder, dirs, names in os.walk(os.path.join(ROOT, base)):
            dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
            for name in names:
                if name.endswith(".rs"):
                    yield os.path.join(folder, name)


def places():
    """[(path relative to the tree, line number, kind, text)]"""
    found = []
    for path in sources():
        rel = os.path.relpath(path, ROOT)
        lines = open(path, encoding="utf-8").read().split("\n")
        # the road is by two types: a file that names neither is not on it
        on_the_road = any(re.search(r"\bFeeding\b|\bTold\b", line) for line in lines)
        for number, line in enumerate(lines, 1):
            text = line.strip()
            if text.startswith("//") or not on_the_road:
                continue
            for kind, rule in (("saying", SAYING), ("keeping", KEEPING), ("term", TERM)):
                if rule.search(line):
                    found.append((rel, number, kind, text[:110]))
    return found


def main():
    for rel, number, kind, text in places():
        print("%s:%d  %-7s %s" % (rel, number, kind, text))
    return 0


if __name__ == "__main__":
    sys.exit(main())
