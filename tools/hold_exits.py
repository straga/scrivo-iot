"""The exits of a hold, listed from the page's files and set against the list of them: every handler of the pointer, the keys, the page's visibility and the focus that the held button has, every other
place in the page that begins something with a hand and a timer, and the places that call what a held button does.

    uv run --no-project python tools/hold_exits.py [--all]

THE LIST IS `EXITS`: a hold ends when whoever holds is gone, and each way it can be gone is a line there with the events that would see it (page/page.js, HoldButton says them in words). The scan reads HoldButton's
class - the bindings of its template (`t-on-...`) and the listeners it adds to the document and the window (`document[how]("...", this.x)`) - and says, per line of the list, whether an event for it stands, and what
stands in HoldButton that no line of the list asks for. Exit 1 on a line without its event or on an event without its line.

It also lists, for the rest of the page's scripts (page/*.js, devices/*/page/*.js, without the engine of the framework): the other classes that take a hand down (`pointerdown`, `mousedown`, `touchstart`, `keydown`) and
keep a timer or a frame (`setTimeout`, `setInterval`, `requestAnimationFrame`) - a second holder would show here -, and what each `onHold` of a host calls and who else calls that.

What it does not see: a handler added by a way other than the three above (a listener put on by a string built at run time), and what the browser itself does with an event. The browser's side is tools/page_plum.py --only hold.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# the exits of a hold: a name, and the (target, event) pairs of which every one has to stand - or a mark in the class (a word that has to be in it)
EXITS = [
    ("the hand goes down: pointer", [("button", "pointerdown")]),
    ("the hand goes down: key", [("button", "keydown")]),
    ("the hand lets go early: pointer", [("button", "pointerup")]),
    ("the hand lets go early: key", [("button", "keyup")]),
    ("the hand moves away", [("button", "pointermove")]),
    ("the browser takes the touch", [("button", "pointercancel")]),
    ("the capture of the pointer is lost", [("button", "lostpointercapture")]),
    ("the button loses its focus", [("button", "blur")]),
    ("the window loses its focus", [("window", "blur")]),
    ("the page is hidden", [("document", "visibilitychange")]),
    ("the page is put away", [("window", "pagehide")]),
    ("the page is frozen", [("document", "freeze")]),
    ("Escape is pressed", [("document", "keydown")]),
    ("the page stopped drawing frames", ["hold.watched("]),
    ("the host disables the button", ["onWillUpdateProps"]),
    ("the button leaves the page", ["onWillUnmount"]),
    ("a hold belongs to who began it", ["this.owner"]),
]
# events that the hold needs for the ways in (down, contextmenu pressed by a long touch) and that no exit asks for
ALLOWED_EXTRA = {("button", "contextmenu")}
DOWN = ("pointerdown", "mousedown", "touchstart", "keydown")
TIMER = re.compile(r"\b(setTimeout|setInterval|requestAnimationFrame)\b")


def scripts():
    out = []
    for pattern in ("page/*.js", "devices/*/page/*.js"):
        for path in sorted(glob.glob(os.path.join(ROOT, pattern))):
            if os.path.basename(path) != "owl.iife.min.js":
                out.append(path)
    return out


def classes(path):
    """(name, first line, last line, text) of every class of a script whose `class` stands at the start of a line and whose end is a `}` at the start of one."""
    lines = open(path, encoding="utf-8").read().splitlines()
    out, i = [], 0
    while i < len(lines):
        m = re.match(r"class (\w+)", lines[i])
        if m:
            j = i
            while j < len(lines) and not lines[j].startswith("}"):
                j += 1
            out.append((m.group(1), i + 1, j + 1, "\n".join(lines[i : j + 1])))
            i = j
        i += 1
    return out


def events_of(text):
    """The (target, event, handler) a class binds: its template's `t-on-` and the listeners it adds to the document and the window."""
    found = []
    for m in re.finditer(r't-on-([a-z]+)(?:\.[a-z]+)*="(\w+)"', text):
        found.append(("button", m.group(1), m.group(2)))
    for m in re.finditer(r'(document|window)\[how\]\("(\w+)",\s*this\.(\w+)\)', text):
        found.append((m.group(1), m.group(2), m.group(3)))
    return found


def holder_in(root_text_of_page):
    for path in scripts():
        for name, first, last, text in classes(path):
            if name == "HoldButton":
                return path, first, last, text
    return None


def main():
    show = "--all" in sys.argv
    held = holder_in(None)
    bad = 0
    if not held:
        print("MISSING   class HoldButton: no script of the page has it")
        return 1
    path, first, last, text = held
    rel = os.path.relpath(path, ROOT)
    found = events_of(text)
    have = {(t, e) for t, e, _ in found}
    print("HoldButton %s:%d-%d binds %d events: %s" % (rel, first, last, len(found), ", ".join("%s %s -> %s" % f for f in found)))
    needed = set()
    for name, wants in EXITS:
        missing = []
        for want in wants:
            if isinstance(want, tuple):
                needed.add(want)
                if want not in have:
                    missing.append("%s %s" % want)
            elif want not in text:
                missing.append(repr(want))
        if missing:
            bad += 1
            print("NO EXIT   %s: nothing in HoldButton for %s" % (name, ", ".join(missing)))
        elif show:
            print("exit      %s" % name)
    for t, e, h in found:
        if (t, e) not in needed and (t, e) not in ALLOWED_EXTRA:
            bad += 1
            print("NO LINE   %s %s -> %s: an event of HoldButton that no exit of the list asks for" % (t, e, h))
    # the other holders: a class that takes a hand down and keeps a timer or a frame
    others = 0
    for path in scripts():
        for name, first, last, text in classes(path):
            if name == "HoldButton":
                continue
            downs = sorted({e for _, e, _ in events_of(text) if e in DOWN} | set(re.findall(r'addEventListener\(\s*["\'](%s)' % "|".join(DOWN), text)))
            if downs and TIMER.search(text):
                others += 1
                bad += 1
                print("HOLDER?   %s:%d class %s takes %s down and keeps a timer or a frame: another hold, not HoldButton's" % (os.path.relpath(path, ROOT), first, name, "/".join(downs)))
            elif downs and show:
                print("takes down %s:%d class %s: %s, no timer" % (os.path.relpath(path, ROOT), first, name, "/".join(downs)))
    # what a host does at the end of a hold, and who else calls it: a handler is a method of the class that holds the button, and what calls it is `this.<name>(` in that class - a method of another class that has the same name is
    # another method, said apart
    for path in scripts():
        for name, first, last, text in classes(path):
            for m in re.finditer(r'onHold="\(\) => this\.(\w+)\(\)"', text):
                handler = m.group(1)
                line_of = first + text[: m.start()].count("\n")
                inside = [first + i for i, line in enumerate(text.splitlines()) if re.search(r"this\.%s\(" % handler, line) and "onHold=" not in line]
                same_name = [
                    "%s:%d class %s" % (os.path.relpath(p2, ROOT), f2, n2)
                    for p2 in scripts()
                    for n2, f2, l2, t2 in classes(p2)
                    if n2 != name and re.search(r"^    %s\(" % handler, t2, re.M)
                ]
                if inside:
                    bad += 1
                    print("ALSO CALLED  %s of class %s (onHold at %s:%d) is called outside the hold, in its own class: %s" % (handler, name, os.path.relpath(path, ROOT), line_of, ", ".join(":%d" % n for n in inside)))
                else:
                    print("onHold    %s:%d class %s -> %s: reached by the hold alone%s" % (os.path.relpath(path, ROOT), line_of, name, handler, ("; same name, other methods: " + ", ".join(same_name)) if same_name else ""))
    print("%d exits listed, %d events of HoldButton, %d other holders, %d findings" % (len(EXITS), len(found), others, bad))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
