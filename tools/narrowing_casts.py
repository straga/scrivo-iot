"""Every narrowing `as` cast in the Rust sources, so a class of overflow can be closed by a list rather than by
the two places somebody happened to notice.

    uv run --no-project python tools/narrowing_casts.py [ROOT ...]

A cast whose left side is a bare name only ever loses the top of a value somebody already holds. A cast whose
left side is ARITHMETIC - a product, a sum, a shift - can lose the top of a value that never existed anywhere
else, and what comes out is an ordinary small number with nothing to mark it. Those are printed first, because
they are the ones that have to be read.

WHAT IT DOES NOT SEE, and this is not a small list: it reads text, not types, so it cannot tell a u64 from a u8
and reports every narrowing-looking cast to a fixed-width integer. It reads one line at a time, so a cast split
across lines is missed. It skips nothing inside macros, strings or comments beyond the obvious `//`. Treat its
answer as the set of places to read, never as a verdict on any of them.

Self-check: run with --check to put the samples below, whose answers are known, through the same reading.
"""
import os
import re
import sys

NARROW = ("u8", "u16", "u32", "i8", "i16", "i32", "usize", "isize", "f32")
CAST = re.compile(r"as\s+(" + "|".join(NARROW) + r")\b")
# arithmetic anywhere to the left of the cast, on the same line, that is not the cast's own operand alone
ARITHMETIC = re.compile(r"[*+\-/%]|<<|\.pow|\.sum|\.product")

SAMPLES = [
    ("let n = (a as u64 * b as u64 / c) as u32;", True),
    ("Some((pin_mv as u64 * span / bottom_ohm as u64) as u32)", True),
    ("let mv = raw as u16;", False),
    ("self.high as u32", False),
    ("// let n = (a * b) as u32;", None),
    ('let s = format!("{} as u32", x);', None),
]


def places(path):
    """Every narrowing cast of the file at PATH: its line number, the line, and whether arithmetic feeds it."""
    found = []
    for n, line in enumerate(open(path, encoding="utf-8", errors="replace"), 1):
        bare = line.split("//")[0]
        if '"' in bare:
            bare = re.sub(r'"[^"]*"', '""', bare)
        for cast in CAST.finditer(bare):
            left = bare[: cast.start()]
            found.append((n, line.rstrip(), bool(ARITHMETIC.search(left))))
    return found


def check():
    """The samples with known answers, through the same reading. None means 'no cast to be found here at all'."""
    bad = 0
    for text, want in SAMPLES:
        got = places_of_text(text)
        said = None if not got else got[0][2]
        mark = "ok " if said == want else "BAD"
        bad += said != want
        print("%s %-52s expected %-5s read %s" % (mark, text[:52], want, said))
    return bad


def places_of_text(text):
    import tempfile

    with tempfile.NamedTemporaryFile("w", suffix=".rs", delete=False) as f:
        f.write(text + "\n")
    try:
        return places(f.name)
    finally:
        os.unlink(f.name)


def main():
    if "--check" in sys.argv:
        sys.exit(1 if check() else 0)
    roots = [a for a in sys.argv[1:] if not a.startswith("-")] or ["crates", "devices"]
    files, arithmetic, plain = 0, [], []
    for root in roots:
        for here, dirs, names in os.walk(root):
            dirs[:] = [d for d in dirs if d not in ("target", ".git")]
            for name in sorted(names):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(here, name)
                files += 1
                for n, line, feeds in places(path):
                    (arithmetic if feeds else plain).append((path, n, line.strip()))

    print("read %d .rs files under %s" % (files, ", ".join(roots)))
    print("\nARITHMETIC FEEDS THE CAST - %d places, read every one:" % len(arithmetic))
    for path, n, line in arithmetic:
        print("  %s:%d\n      %s" % (path, n, line))
    print("\na name is cast, nothing computed - %d places:" % len(plain))
    for path, n, line in plain:
        print("  %s:%d  %s" % (path, n, line[:110]))


if __name__ == "__main__":
    main()
