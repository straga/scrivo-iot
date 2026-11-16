"""Every place a setting is read OUTSIDE the round that uses it, so "changed on the page, works after a restart"
can be answered by a list rather than by whichever module somebody happened to open.

    uv run --no-project python tools/settings_read_once.py [ROOT ...]

A setting is a value a person changes while the boiler runs. A thread that reads it once before its `loop` holds
that value until the board restarts - and the page, which reads the same setting afresh every time it draws,
then shows numbers that do not belong together. The failure looks like "the update broke my device", which is
the dearest kind: nobody suspects the setting they themselves changed.

It reads text, not types: functions are found by `fn name(`, their extent by counting braces, and a read is a
call to `nvs_settings::value` / `settings::value` or to a WRAPPER - a function of the same file whose own body
reads one. Two levels of wrapper are followed, no more. A read inside a closure that the loop calls later is
reported as being where it is written, which may be before the loop and perfectly fine. **A round written as
`while` or `for` is not seen as a round at all**, so a setting read before one of those is not reported: today
every standing thread of devices/ turns on `loop`, and the day one does not, this script will quietly say
nothing about it. So the answer is the set of places to read, never a verdict.

Self-check: --check puts samples whose answers are known through the same reading.
"""
import os
import re
import sys

READS = re.compile(r"\b(?:nvs_settings|settings)::value\s*\(")
FN = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*[(<]")

SAMPLES = [
    ("a thread that takes the setting before its loop", """
fn run() {
    let (top, bottom) = divider();
    loop {
        use_them(top, bottom);
    }
}
fn divider() -> u32 { nvs_settings::value(&settings::DIVIDER_TOP).parse().unwrap_or(0) }
""", ["run"]),
    ("a thread that takes it inside the loop", """
fn run() {
    loop {
        let (top, bottom) = divider();
        use_them(top, bottom);
    }
}
fn divider() -> u32 { nvs_settings::value(&settings::DIVIDER_TOP).parse().unwrap_or(0) }
""", []),
    ("a function with no loop at all is nobody's business here", """
fn rows() -> Vec<String> {
    let (top, bottom) = divider();
    vec![format!("{top}/{bottom}")]
}
fn divider() -> u32 { nvs_settings::value(&settings::DIVIDER_TOP).parse().unwrap_or(0) }
""", []),
]


def functions(text):
    """(name, first line, body) for every function of TEXT, by counting braces."""
    lines = text.splitlines()
    out = []
    i = 0
    while i < len(lines):
        m = FN.match(lines[i])
        if not m:
            i += 1
            continue
        depth, body, started = 0, [], False
        j = i
        while j < len(lines):
            body.append(lines[j])
            depth += lines[j].count("{") - lines[j].count("}")
            started = started or "{" in lines[j]
            if started and depth <= 0:
                break
            j += 1
        out.append((m.group(1), i + 1, body))
        i = j + 1
    return out


def wrappers(text):
    """Functions of this file whose own body reads a setting - two levels deep."""
    named = {name: body for name, _, body in functions(text)}
    found = {n for n, body in named.items() if any(READS.search(l) for l in body)}
    for _ in range(2):
        for n, body in named.items():
            if any(re.search(r"\b%s\s*\(" % w, l) for l in body for w in found):
                found.add(n)
    return found


def suspects(path):
    """(function, line of the read, the line) for every setting read before the loop of a looping function."""
    text = open(path, encoding="utf-8", errors="replace").read()
    reading = wrappers(text)
    out = []
    for name, first, body in functions(text):
        loop_at = next((k for k, l in enumerate(body) if re.search(r"\bloop\s*\{", l)), None)
        if loop_at is None:
            continue
        for k, line in enumerate(body[:loop_at]):
            bare = line.split("//")[0]
            hit = READS.search(bare) or next((w for w in reading if w != name and re.search(r"\b%s\s*\(" % w, bare)), None)
            if hit:
                out.append((name, first + k, line.strip()))
    return out


def check():
    import tempfile

    bad = 0
    for words, text, want in SAMPLES:
        with tempfile.NamedTemporaryFile("w", suffix=".rs", delete=False) as f:
            f.write(text)
        try:
            got = [name for name, _, _ in suspects(f.name)]
        finally:
            os.unlink(f.name)
        mark = "ok " if got == want else "BAD"
        bad += got != want
        print("%s %-50s expected %-10s read %s" % (mark, words, want, got))
    return bad


def main():
    if "--check" in sys.argv:
        sys.exit(1 if check() else 0)
    roots = [a for a in sys.argv[1:] if not a.startswith("-")] or ["crates", "devices"]
    files, found = 0, []
    for root in roots:
        for here, dirs, names in os.walk(root):
            dirs[:] = [d for d in dirs if d not in ("target", ".git")]
            for name in sorted(names):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(here, name)
                files += 1
                for fn, line, text in suspects(path):
                    found.append((path, fn, line, text))
    print("read %d .rs files under %s" % (files, ", ".join(roots)))
    print("\nA SETTING READ BEFORE THE LOOP THAT USES IT - %d places:" % len(found))
    for path, fn, line, text in found:
        print("  %s:%d  in %s()\n      %s" % (path, line, fn, text))
    if not found:
        print("  none")


if __name__ == "__main__":
    main()
