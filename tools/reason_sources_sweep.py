"""Every place of the reason line where a number is taken from a sensor or a row, found by reading `askOf` in devices/burner/page/scheme.js - the class of "the line reads a sensor the thermostat does not hold".

    uv run --no-project python tools/reason_sources_sweep.py [--page DIR]

Prints one line for each call of `reading(...)` (a sensor of the drawing, by its place) and of `degrees(...)` (a number of a row) inside `askOf`: its line in the file and the expression it is given. What it does NOT see: a number taken in a helper
`askOf` calls (the pause's clock is `root.dictionary.shown`, the settings are read in `heldPlace`), a call that is spelled any other way (`reading ( x )` with a break, an alias), and what selects the call - the guard around it stays for a person to name;
tools/test_page_reason_line.py names it for each site and fails where the sites are not the ones named, so a new source cannot come in without a pair of tests.
"""
import argparse
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SCHEME = os.path.join(ROOT, "devices", "burner", "page", "scheme.js")

CALLS = ("reading", "degrees")


def function_body(source, name):
    """The text of the function NAME from its head to its closing brace, and the line it begins at: braces are counted outside strings and comments' quotes the way `askOf` is written. -> (text, first line)"""
    head = re.search(r"^\s*function %s\(" % re.escape(name), source, re.M)
    if head is None:
        raise SystemExit("reason_sources_sweep: no function %s in the file" % name)
    start = head.start()
    depth, i, opened = 0, source.index("{", head.end()), False
    while i < len(source):
        ch = source[i]
        if ch in "\"'`":
            i = source.index(ch, i + 1)
            while source[i - 1] == "\\":
                i = source.index(ch, i + 1)
        elif source.startswith("//", i):
            i = source.index("\n", i)
            continue
        elif ch == "{":
            depth, opened = depth + 1, True
        elif ch == "}":
            depth -= 1
            if opened and depth == 0:
                return source[start:i + 1], source.count("\n", 0, start) + 1
        i += 1
    raise SystemExit("reason_sources_sweep: the function %s does not close" % name)


def arguments(text, at):
    """The text between the parenthesis that opens at AT and the one that closes it."""
    depth = 0
    for i in range(at, len(text)):
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return text[at + 1:i]
    raise SystemExit("reason_sources_sweep: a call does not close")


def sites(source):
    """The calls of `reading` and `degrees` in `askOf`, in the order of the file: [(call, expression, line)]. The line that DEFINES `reading` (`const reading = (place) =>`) is not a call."""
    body, first = function_body(source, "askOf")
    found = []
    for hit in re.finditer(r"(?<![\w.])(%s)\(" % "|".join(CALLS), body):
        if re.search(r"=\s*$", body[:hit.start()].rsplit("\n", 1)[-1]) is not None:
            continue
        found.append((hit.group(1), " ".join(arguments(body, hit.end() - 1).split()), first + body.count("\n", 0, hit.start())))
    return found


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--page", default=None, help="the page folder to read instead of the device's own")
    args = ap.parse_args()
    path = os.path.join(args.page, "scheme.js") if args.page else SCHEME
    for call, expression, line in sites(open(path).read()):
        print("scheme.js:%d  %s(%s)" % (line, call, expression))


if __name__ == "__main__":
    main()
