"""Every place of the page that reads, AFTER A WAIT, a store a press shares with the next press of its channel (11.1/9.34, the late answer): the scan that makes the list, so that the list is not made by eye.

    uv run --no-project python tools/press_late_readers.py [--all]

A place is DEFERRED when it stands inside the body of a callback handed to something that calls it later (`.then(`, `.catch(`, `.finally(`, `setInterval(`, `setTimeout(`, `requestAnimationFrame(`, `addEventListener(`, `every(`), or in a function whose name is handed to
one of them (`every(() => look())` defers `look`), or in an async function after an `await`. The
stores are the names a press keeps its state in: `locks`, `watching`, `hand`, `waits`, `flight`, `aimed`, `aimSeq`, `note_press`. Prints each reference as FILE:LINE, the store, the function it stands in and what defers it; without --all only the deferred ones.

WHAT IT DOES NOT SEE: a store reached through another name, a callback handed to a function of ours that defers it in its turn, and a template literal's `${}` (blanked as text); a function body is taken to run from the opening of the function to the next function that opens, so a read after the end of a nested function is counted to it. A place the scan does not list is not shown to be clean by it; a place it lists is read by eye.
"""
import argparse
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FILES = ["page/page.js", "page/press.js", "devices/burner/page/parts.js", "devices/burner/page/scheme.js"]
STORES = r"\b(locks|watching|hand|waits|flight|aimed|aimSeq|note_press)\b"
DEFERRERS = {"then", "catch", "finally", "setInterval", "setTimeout", "requestAnimationFrame", "addEventListener", "every"}


def blank(text):
    """TEXT with comments, strings and template literals turned into spaces of the same length (newlines kept), so that what is left is code."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i)); i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", text[i:j])); i = j
        elif c in "'\"`":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            j = min(n, j + 1)
            out.append(re.sub(r"[^\n]", " ", text[i:j])); i = j
        else:
            out.append(c); i += 1
    return "".join(out)


def enclosing_calls(code, at):
    """The names of the calls whose parentheses enclose position AT, innermost first."""
    depth, names, i = 0, [], at
    while i > 0:
        i -= 1
        c = code[i]
        if c == ")":
            depth += 1
        elif c == "(":
            if depth == 0:
                m = re.search(r"([A-Za-z_$][\w$]*)\s*$", code[:i])
                names.append(m.group(1) if m else "")
            else:
                depth -= 1
    return names


def functions(code):
    """[(name, start, is_async)] of the functions, methods and arrow constants of CODE, in order."""
    out = []
    for m in re.finditer(r"(?:^|\n)[ \t]*(async\s+)?(?:function\s+([\w$]+)|(?:static\s+)?(?:get\s+)?([\w$]+)\s*\([^)]*\)\s*\{|(?:const|let)\s+([\w$]+)\s*=\s*(async\s*)?\(?[\w$,\s]*\)?\s*=>)", code):
        name = m.group(2) or m.group(3) or m.group(4)
        if name and name not in ("if", "for", "while", "switch", "catch", "return"):
            out.append((name, m.start(), bool(m.group(1) or m.group(5))))
    for m in re.finditer(r"\(\s*async\s*\(\)\s*=>\s*\{", code):
        out.append(("(async IIFE)", m.start(), True))
    out.sort(key=lambda f: f[1])
    return out


def deferred_names(code):
    """{name: the deferrer} of every identifier handed, anywhere in its arguments, to a call that runs it later: `every(() => look())` defers `look`."""
    names = {}
    for m in re.finditer(r"\b(%s)\s*\(" % "|".join(sorted(DEFERRERS)), code):
        depth, i = 1, m.end()
        while i < len(code) and depth:
            depth += {"(": 1, ")": -1}.get(code[i], 0)
            i += 1
        for ident in re.findall(r"[A-Za-z_$][\w$]*", code[m.end():i]):
            names.setdefault(ident, m.group(1))
    return names


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--all", action="store_true")
    args = ap.parse_args()
    seen = 0
    for rel in FILES:
        text = open(os.path.join(ROOT, rel), encoding="utf-8").read()
        code = blank(text)
        funcs, handed = functions(code), deferred_names(code)
        for m in re.finditer(STORES, code):
            line = code.count("\n", 0, m.start()) + 1
            calls = [c for c in enclosing_calls(code, m.start()) if c in DEFERRERS]
            before = [f for f in funcs if f[1] < m.start()]
            fn = before[-1] if before else ("", 0, False)
            why = ""
            if calls:
                why = "inside the callback of " + calls[0]
            elif fn[0] in handed:
                why = "the function %s is handed to %s" % (fn[0], handed[fn[0]])
            elif fn[2] and re.search(r"\bawait\b", code[fn[1]:m.start()]):
                why = "after an await of %s" % fn[0]
            if why or args.all:
                seen += 1
                print("%s:%d  %-12s in %-14s %s" % (rel, line, m.group(1), fn[0] or "-", why or "synchronous"))
    print("%d places" % seen, file=sys.stderr)


if __name__ == "__main__":
    main()
