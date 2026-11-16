"""Screens of a board's page as a person sees it, at a phone's width and a computer's, in headless Chrome.

    uv run --no-project --with websocket-client python tools/page_shots.py OUT_DIR NAME URL [--type COMMAND]
        [--down BOARD] [--tab TAB]

For each width, 375 and 900: opens URL, waits until the state has arrived (the name of the device is on the page),
and saves OUT_DIR/NAME_<width>.png of the whole page. Prints whether the terminal card is shown. With --type it
types COMMAND into the page's terminal, presses Enter and saves NAME_<width>_typed.png once the answer is in the
log. With --tab TAB it opens the page's tab TAB (the data-p of its button, e.g. settings) before the screens, names the
files NAME_TAB_<width>.png and prints the text a person sees on that tab. With --down BOARD, last of all, it takes the board down the way a person does: "recovery image...", the
board's name typed, "go down" - and prints what the page says. The Chrome driving is tools/chrome_page.py (class Page).
"""
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
from chrome_page import CHROME, PORT, Page  # noqa: E402


# Where the terminal's log stands: an id on the old page, the only <pre> on the new one.
LOG = "((document.getElementById('term-log') || document.querySelector('pre')).textContent)"


def wait(p, expr, seconds):
    end = time.time() + seconds
    while time.time() < end:
        if p.js(expr):
            return True
        time.sleep(0.3)
    return False


def main():
    args = sys.argv[1:]
    typed = down = tab = None
    if "--tab" in args:
        i = args.index("--tab")
        tab = args[i + 1]
        del args[i:i + 2]
    if "--type" in args:
        i = args.index("--type")
        typed = args[i + 1]
        del args[i:i + 2]
    if "--down" in args:
        i = args.index("--down")
        down = args[i + 1]
        del args[i:i + 2]
    out, name, url = args
    os.makedirs(out, exist_ok=True)
    profile = tempfile.mkdtemp(prefix="page_shots_")
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={PORT}",
                               f"--user-data-dir={profile}", "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                targets = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json"))
                break
            except OSError:
                time.sleep(0.2)
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        for width in (375, 900):
            p.call("Emulation.setDeviceMetricsOverride", width=width, height=900, deviceScaleFactor=1,
                   mobile=width < 600)
            p.call("Page.navigate", url=url)
            # the device's name is an id on the old page and the first heading on the new one
            drawn = wait(p, "(function(){var d = document.getElementById('device') || document.querySelector('h1');"
                            " return d && d.textContent.trim().length > 0})()", 15)
            p.settle(1.5)
            shown = p.js("(function(){var t=document.getElementById('term');"
                         " if (t) return !t.classList.contains('hide');"
                         " for (const b of document.querySelectorAll('nav button'))"
                         "   if (b.textContent.trim().toLowerCase() === 'terminal') return true;"
                         " return null})()")
            print(f"{name} {width}: state drawn {drawn}, terminal card shown {shown}")
            if tab:
                # By the words on the button, not by an id: the Rust board's page is built of components and carries
                # none of the old page's ids, and the words are what a person presses either way.
                p.js("(function(w){for (const b of document.querySelectorAll('nav button, #tabs button'))"
                     " if (b.textContent.trim().toLowerCase() === w) { b.click(); return 1 } return 0})"
                     f"({json.dumps(tab.lower())})")
                p.settle(1)
                seen = p.js("(document.getElementById('p-%s') || document.querySelector('main') || document.body)"
                            ".innerText" % tab)
                print(f"{name} {width}: tab {tab} reads:\n{seen}")
                p.shot(os.path.join(out, f"{name}_{tab}_{width}.png"), full=True)
                if not typed:
                    continue
            else:
                p.shot(os.path.join(out, f"{name}_{width}.png"), full=True)
            if typed and shown:
                # The old page names its terminal by ids; the new one is components, so the field is found by what
                # it offers to type and the button by the word on it - what a person goes by on either page.
                p.js("(function(){var f = document.getElementById('term-line') ||"
                     " document.querySelector('input[placeholder*=status]');"
                     " f.value = %s;"
                     " f.dispatchEvent(new Event('input', {bubbles: true}));"
                     " var b = document.getElementById('term-send');"
                     " if (!b) for (const x of document.querySelectorAll('button'))"
                     "   if (x.textContent.trim().toLowerCase() === 'send') b = x;"
                     " b.click(); return 1})()" % json.dumps(typed))
                answered = wait(p, LOG + ".includes('rust> ') && " + LOG + ".split(" + json.dumps(typed)
                                + ").length > 1", 8)
                p.settle(1.5)
                tail = p.js(LOG + ".slice(-400)")
                print(f"{name} {width}: typed {typed!r}, answered {answered}; log ends:\n{tail}")
                p.shot(os.path.join(out, f"{name}_{width}_typed.png"), full=True)
        if down:
            p.js("document.getElementById('down').click(); 1")
            p.settle(0.5)
            p.js(f"var n = document.getElementById('down-name'); n.value = {json.dumps(down)};"
                 " n.dispatchEvent(new Event('input')); 1")
            enabled = p.js("!document.getElementById('down-go').disabled")
            p.js("document.getElementById('down-go').click(); 1")
            p.settle(2)
            print(f"{name}: go down enabled {enabled}; the page says:",
                  p.js("document.getElementById('board-note').textContent"))
            p.shot(os.path.join(out, f"{name}_down.png"), full=True)
    finally:
        chrome.terminate()
        chrome.wait(timeout=10)


if __name__ == "__main__":
    main()
