"""Two pages open on one board at once: what the second one sends while a person works on the first.

    uv run --no-project --with websocket-client python tools/page_two_tabs.py [--url http://127.0.0.1:8790/]

Run tools/page_stand.py first; this drives its board, reads its record of every POST and judges by that record.

WHAT IS BEING ASKED. A slider moved on one page must reach the board once, from the hand that moved it. Nothing
else may reach the board at all: not a page being opened, not a page being reloaded, not a page redrawing itself
because the state arrived. A second page holding what the slider looked like when it loaded and sending that on
is how a fan set to 30 % went out on 21.09 - which on a lit boiler is the draught under the fire.

Checks, each printing one verdict line:

  opening    a page is opened and left alone: the board hears nothing
  reload     a page is reloaded with the fan held somewhere: the board hears nothing
  follows    the fan is moved on page A; page B shows the new number without sending anything
  no-revert  page B, open since before the move, is made to redraw: it must not send the old number back
  one-hand   over the whole run, every POST the board heard was a hand on a control

Exits 1 when a check failed, 0 when all passed.
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))


from chrome_page import CHROME, Page  # noqa: E402

PORT = 9344
# The row whose slider this probe moves, and the numbers it moves it to.
THING = "gate"
FIRST = 30
THEN = 20


def tab(url, at):
    """A new tab on URL, driven on its own connection."""
    # PUT and not GET: Chrome stopped opening tabs on a plain GET of /json/new, to keep a page from opening them.
    ask = urllib.request.Request("http://127.0.0.1:%d/json/new?%s" % (at, urllib.parse.quote(url, safe=":/")),
                                 method="PUT")
    made = json.load(urllib.request.urlopen(ask))
    p = Page(made["webSocketDebuggerUrl"])
    p.call("Page.enable")
    p.call("Runtime.enable")
    return p


def look(p):
    """Brings a tab to the front before it is read or driven.

    A tab in the background is throttled by the browser, and a page that has not been given the state yet looks
    exactly like a page that refused it. What is being judged here is what a person sees on the tab they are
    looking at, so the tab is made the one being looked at.
    """
    p.call("Page.bringToFront")


def ready(p, seconds=12):
    """Waits until the state has arrived: the name of the device stands in the header."""
    end = time.time() + seconds
    while time.time() < end:
        if p.js("!!document.querySelector('h1') && document.querySelector('h1').textContent.trim().length > 0"):
            return True
        time.sleep(0.25)
    return False


def slider_value(p):
    return p.js("(function(){const r=document.querySelector('input[type=range]');"
                " return r ? Number(r.value) : null})()")


def shown_number(p):
    """What stands beside the slider - the number a person reads."""
    return p.js("(function(){const r=document.querySelector('input[type=range]');"
                " if(!r) return null; const s=r.parentElement.querySelector('span:last-of-type');"
                " return s ? s.textContent.trim() : null})()")


def move(p, to):
    """Moves the slider the way a hand does: the number follows while it is dragged, the board hears it on release."""
    p.js("(function(v){const r=document.querySelector('input[type=range]'); if(!r) return 0;"
         " r.value=v; r.dispatchEvent(new Event('input',{bubbles:true}));"
         " r.dispatchEvent(new Event('change',{bubbles:true})); return 1})(%d)" % to)


def posts(url):
    """What the board has been SENT so far, from its own record.

    Opening and closing the stream is in that record too, and it is not a request that changes anything: the
    question here is what the page WRITES to the board, so the stream is read out of the way rather than counted.
    """
    whole = json.load(urllib.request.urlopen(url.rstrip("/") + "/sent"))
    return [one for one in whole if one["what"] != "/events"]


def streams(url):
    """What happened to the stream over the run - opened, refused, closed - for the report."""
    whole = json.load(urllib.request.urlopen(url.rstrip("/") + "/sent"))
    return [one["body"] for one in whole if one["what"] == "/events"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="http://127.0.0.1:8790/")
    ap.add_argument("--keep", action="store_true", help="leave Chrome running")
    args = ap.parse_args()

    profile = tempfile.mkdtemp(prefix="page_two_tabs_")
    chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % PORT,
                               "--user-data-dir=%s" % profile, "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    failures = []
    try:
        for _ in range(50):
            try:
                urllib.request.urlopen("http://127.0.0.1:%d/json" % PORT)
                break
            except OSError:
                time.sleep(0.2)

        # ---- opening ----
        before = len(posts(args.url))
        a = tab(args.url, PORT)
        if not ready(a):
            raise SystemExit("page A never drew the state - is the stand running?")
        a.settle(2.0)
        after = len(posts(args.url))
        said = "opening   %s" % ("the board heard nothing" if after == before
                                 else "THE BOARD HEARD %d request(s)" % (after - before))
        print(said)
        if after != before:
            failures.append(said)

        # ---- the fan is moved on A ----
        look(a)
        move(a, FIRST)
        a.settle(1.5)
        b = tab(args.url, PORT)
        if not ready(b):
            raise SystemExit("page B never drew the state")
        b.settle(2.0)

        # ---- reload ----
        before = len(posts(args.url))
        look(b)
        b.call("Page.reload")
        ready(b)
        b.settle(2.0)
        after = len(posts(args.url))
        said = "reload    %s" % ("the board heard nothing" if after == before
                                 else "THE BOARD HEARD %d request(s)" % (after - before))
        print(said)
        if after != before:
            failures.append(said)

        # ---- follows ----
        before = posts(args.url)
        look(a)
        move(a, THEN)
        a.settle(1.0)
        look(b)
        b.settle(2.5)
        after = posts(args.url)
        mine = [p for p in after[len(before):]]
        on_b = slider_value(b)
        said = "follows   page B shows %s, the board heard %s" % (on_b, [p["body"] for p in mine])
        print(said)
        if on_b != THEN:
            failures.append("follows: page B shows %s, not %d" % (on_b, THEN))
        if [p["body"] for p in mine] != [str(THEN)]:
            failures.append("follows: the board heard %s, and it should have heard one %d"
                            % ([p["body"] for p in mine], THEN))

        # ---- no-revert: B is made to redraw and must not send its old number ----
        before = len(posts(args.url))
        # what a page does by itself: the tab is hidden and shown again, and the state arrives meanwhile
        look(b)
        b.js("document.dispatchEvent(new Event('visibilitychange'))")
        b.js("window.dispatchEvent(new Event('focus'))")
        b.js("window.dispatchEvent(new Event('pageshow'))")
        b.settle(3.0)
        after = len(posts(args.url))
        said = "no-revert %s" % ("page B sent nothing" if after == before
                                 else "PAGE B SENT %d request(s): %s"
                                 % (after - before, [p["body"] for p in posts(args.url)[before:]]))
        print(said)
        if after != before:
            failures.append(said)

        # ---- one-hand ----
        every = posts(args.url)
        hands = [p for p in every if p["what"].startswith("/hold/")]
        said = "one-hand  the board heard %d request(s) in all: %s" % (len(every), [p["what"] for p in every])
        print(said)
        print("          the stream, over the run: %s" % streams(args.url))
        if len(every) != 2 or [p["body"] for p in hands] != [str(FIRST), str(THEN)]:
            failures.append("one-hand: expected exactly two holds, %d and %d; heard %s"
                            % (FIRST, THEN, [(p["what"], p["body"]) for p in every]))

    finally:
        if not args.keep:
            chrome.terminate()

    for f in failures:
        print("FAILED:", f)
    print("%d of 5 checks failed" % len(failures))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
