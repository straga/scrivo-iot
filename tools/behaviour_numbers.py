"""The numbers of the burner's behaviour that stand in the code and not in a setting: a sweep over the sources of the logic and of the burner's device, printing each numeric literal with its place.

    uv run --no-project python tools/behaviour_numbers.py [--all] [PATH ...]

WHAT IT SEES: a decimal literal or an integer above 1 in the non-test code of the folders it is given (default: the crates of the burner's logic and devices/burner/src), outside comments and strings, with the
`const` it stands in when it is one. WHAT IT DOES NOT SEE: a number written as a word (`Duration::from_secs(SOME)` shows the const, not the value behind a name from another file), a number in a macro that builds
the text, a literal in a test (everything after the first `#[cfg(test)]` is left out), and what a person must read by meaning - "how long", "how far", "how many" - is the reader's verdict, not this script's:
it lists; a person judges which is a behaviour and which a unit, a scale, a protocol or a hardware fact. Without `--all` it leaves out 0, 1, 2, 10, 100, 1000 and the powers of two, which are units and
masks far more often than a behaviour.
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT = ["crates/automaton", "crates/regulator", "crates/flame", "crates/draught", "crates/max6675", "crates/burner_map", "crates/dimmer", "crates/relay", "crates/module",
           "devices/burner/src", "devices/burner/api/src"]
COMMON = {"0", "1", "2", "10", "100", "1000", "4", "8", "16", "32", "64", "128", "256", "512", "1024", "0.0", "1.0", "2.0", "10.0", "100.0", "1000.0", "0.5"}
NUMBER = re.compile(r"(?<![\w.])(\d[\d_]*\.\d[\d_]*(?:e-?\d+)?|\d[\d_]*)(?:_?(?:f32|f64|u8|u16|u32|u64|i8|i16|i32|i64|usize|isize))?(?![\w.])")
CONST = re.compile(r"^\s*(?:pub(?:\([a-z]+\))?\s+)?(?:const|static)\s+([A-Z0-9_]+)\s*:")


def code_of(text):
    """TEXT without comments and string literals, line by line, the line numbers kept."""
    out, in_block = [], 0
    for line in text.splitlines():
        res, i = "", 0
        while i < len(line):
            two = line[i:i + 2]
            if in_block:
                if two == "*/":
                    in_block -= 1
                    i += 2
                elif two == "/*":
                    in_block += 1
                    i += 2
                else:
                    i += 1
                continue
            if two == "/*":
                in_block += 1
                i += 2
            elif two == "//":
                break
            elif line[i] == '"':
                j = i + 1
                while j < len(line) and line[j] != '"':
                    j += 2 if line[j] == "\\" else 1
                res += '""'
                i = j + 1
            else:
                res += line[i]
                i += 1
        out.append(res)
    return out


def sweep(path, every):
    with open(path, encoding="utf-8") as f:
        text = f.read()
    cut = text.find("#[cfg(test)]")
    if cut >= 0:
        text = text[:cut]
    found = []
    for n, line in enumerate(code_of(text), 1):
        const = CONST.match(line)
        # an attribute, a doc line and a line of a literal array of an index are not a behaviour: the attribute is cut, the rest the reader judges
        if line.strip().startswith("#["):
            continue
        for m in NUMBER.finditer(line):
            value = m.group(1).replace("_", "")
            if not every and value in COMMON:
                continue
            found.append((n, value, const.group(1) if const else "", line.strip()))
    return found


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    every = "--all" in sys.argv
    total = 0
    for folder in args or DEFAULT:
        base = os.path.join(ROOT, folder)
        for here, _, files in sorted(os.walk(base)):
            if os.sep + "tests" in here or os.sep + "examples" in here:
                continue
            for name in sorted(files):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(here, name)
                for n, value, const, line in sweep(path, every):
                    total += 1
                    print("%s:%d\t%s\t%s\t%s" % (os.path.relpath(path, ROOT), n, value, const or "-", line[:140]))
    print("# %d literals" % total, file=sys.stderr)


if __name__ == "__main__":
    main()
