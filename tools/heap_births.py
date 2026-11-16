"""Every place in the firmware's sources where a value goes to the heap through a constructor that takes it READY -
built first where the call stands, on the stack of the task, and copied over after: Box::new, Box::pin, Arc::new,
Rc::new, and vec![a, b, ..] (a list of elements is an array on the stack first; vec![x; n] is not, it is listed apart).
And every future made where it stands: an async block, and whatever is handed to block_on or pin! - it lives on the
stack of the task for as long as it runs. A future weighs all its awaits at once from the moment it is made, so a
future of a few captures can be 86 KB; its size is only the compiler's to know, and the build holds it
to round::ON_STACK_AT_MOST. This list says where to look; the column "future" marks a place whose value is one.

    uv run --no-project python tools/heap_births.py [ROOT]

One line per place: file:line, the constructor, "future" or nothing, the text of its argument up to the matching bracket. Sources are
src/ of every crate and device; tests/, examples/, benches/ and #[cfg(test)] modules are left out - they never run on
the board. What it does NOT see: a macro of ours that expands to one of these, and a ready value moved into the heap by
something else (Vec::push of a large element, a channel that stores by value) - named in the report beside its output;
a future counts as one when it is an async block, a call of an `async fn` of the same crate, or given to block_on or
pin! - a future made by an async fn of another crate or a library and boxed is listed as a Box::pin without the mark,
and a plain fn of the crate named like an async fn of it is marked a future.
"""
import os
import re
import sys

ROOT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
CALLS = re.compile(
    r"\b(Box::new|Box::pin|Arc::new|Rc::new|std::sync::Arc::new|alloc::sync::Arc::new|vec!|block_on|pin!)\s*[\(\[]"
    r"|\b(async)\s+(?:move\s+)?\{"
)
ASYNC_FN = re.compile(r"\basync\s+fn\s+(\w+)")
# what a future is handed to: whatever it is given is one
TAKES_A_FUTURE = ("block_on", "pin!")


def argument(text, start):
    """The text from START (just past the opening bracket) to the bracket that closes it."""
    opening = text[start - 1]
    closing = {"(": ")", "[": "]", "{": "}"}[opening]
    depth, i = 1, start
    while i < len(text) and depth:
        c = text[i]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        i += 1
    return text[start:i - 1]


def test_spans(text):
    """Byte spans of #[cfg(test)] modules: from the attribute to the brace that closes the module."""
    spans = []
    for m in re.finditer(r"#\[cfg\(test\)\]\s*(pub\s+)?mod\s+\w+\s*\{", text):
        depth, i = 1, m.end()
        while i < len(text) and depth:
            depth += {"{": 1, "}": -1}.get(text[i], 0)
            i += 1
        spans.append((m.start(), i))
    return spans


def strip_comments(text):
    """Comments blanked out with spaces, so positions and line numbers stay."""
    text = re.sub(r"/\*.*?\*/", lambda m: re.sub(r"[^\n]", " ", m.group()), text, flags=re.S)
    return re.sub(r"//[^\n]*", lambda m: " " * len(m.group()), text)


def sources(root):
    """(path, text without comments, spans of test modules) for every .rs under src/ of the crates and devices."""
    for top in ("crates", "devices"):
        for dirpath, dirs, files in os.walk(os.path.join(root, top)):
            dirs[:] = [d for d in dirs if d not in ("tests", "examples", "benches", "target", "build", "page")]
            if os.sep + "src" not in dirpath + os.sep and not dirpath.endswith("src"):
                continue
            for name in sorted(files):
                if name.endswith(".rs"):
                    path = os.path.join(dirpath, name)
                    text = strip_comments(open(path, encoding="utf-8").read())
                    yield path, text, test_spans(text)


def is_future(kind, arg, async_fns):
    """Whether the value at the place is a future, as far as the text tells."""
    if kind in TAKES_A_FUTURE or kind == "async block":
        return True
    if arg.startswith("async"):
        return True
    called = re.match(r"[\w:.&\s]*?(\w+)\s*(?:::<[^>]*>)?\s*\(", arg)
    return bool(called) and called.group(1) in async_fns


def crate_of(path):
    """The directory of the crate PATH is in - what an `async fn` name is known inside: a namesake elsewhere is not it."""
    return path.split(os.sep + "src" + os.sep)[0]


def places(root):
    found = list(sources(root))
    async_fns = {}
    for path, text, _ in found:
        async_fns.setdefault(crate_of(path), set()).update(m.group(1) for m in ASYNC_FN.finditer(text))
    for path, text, skip in found:
        for m in CALLS.finditer(text):
            if any(a <= m.start() < b for a, b in skip):
                continue
            arg = " ".join(argument(text, m.end()).split())
            if m.group(2):
                kind = "async block"
            elif "::" in m.group(1):
                kind = "::".join(m.group(1).split("::")[-2:])
            else:
                kind = m.group(1)
            if kind == "vec!" and re.search(r";\s*[^;]+$", arg) and arg.count(";") == 1:
                kind = "vec![x; n]"
            future = "future" if is_future(kind, arg, async_fns.get(crate_of(path), set())) else ""
            yield os.path.relpath(path, root), text.count("\n", 0, m.start()) + 1, kind, future, arg


if __name__ == "__main__":
    rows = list(places(ROOT))
    for path, line, kind, future, arg in rows:
        print(f"{path}:{line}\t{kind}\t{future}\t{arg[:140]}")
    print(f"# {len(rows)} places", file=sys.stderr)
