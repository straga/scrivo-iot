"""One card of a device's page as a person sees it, and the words on it: a screenshot of just that card, 375 wide, and its text printed.

    uv run --no-project --with websocket-client python tools/page_card_shot.py URL OUT.png CARD [--tab Burner] [--open] [--save] [--theme l|d]

CARD is the heading of the card, as the page writes it (`Flame`). Waits for the state to arrive, opens the tab, finds the section whose first heading is
CARD and saves its box. Made for comparing one module's card on two pages: two screenshots of the same card, no matter what else the pages hold.
"""
import base64
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
from chrome_page import CHROME, PORT, Page  # noqa: E402


def main():
    args = sys.argv[1:]
    tab = None
    theme = None
    if "--theme" in args:
        i = args.index("--theme")
        theme = args[i + 1]
        del args[i:i + 2]
    press_save = "--save" in args
    if press_save:
        args.remove("--save")
    unfold = "--open" in args or press_save
    if "--open" in args:
        args.remove("--open")
    if "--tab" in args:
        i = args.index("--tab")
        tab = args[i + 1]
        del args[i:i + 2]
    url, out, card = args
    profile = tempfile.mkdtemp(prefix="page_card_")
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}", "--no-first-run", "about:blank"],
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
        p.call("Emulation.setDeviceMetricsOverride", width=375, height=900, deviceScaleFactor=1, mobile=True)
        if theme:
            # the page keeps its theme in localStorage (`t`: l or d); set it and open the page again
            p.call("Page.navigate", url=url)
            p.settle(1.0)
            p.js("try { localStorage.setItem('t', %s) } catch (e) {}" % json.dumps(theme))
        p.call("Page.navigate", url=url)
        end = time.time() + 15
        while time.time() < end and not p.js("(function(){var d=document.querySelector('h1'); return d && d.textContent.trim().length>0})()"):
            time.sleep(0.3)
        p.settle(3.0 if theme else 1.5)
        if tab:
            p.js("(function(w){for (const b of document.querySelectorAll('nav button')) if (b.textContent.trim().toLowerCase() === w) { b.click(); return 1 } return 0})(%s)" % json.dumps(tab.lower()))
            p.settle(1.5)
        if unfold:
            # the button that folds the card's settings, pressed: what a person does to see them
            p.js("(function(w){for (const h of document.querySelectorAll('section h2')) if (h.textContent.trim().toLowerCase() === w) {"
                 " h.closest('section').querySelectorAll('button[aria-expanded=\"false\"]').forEach(b => b.click()) } return 1})(%s)" % json.dumps(card.lower()))
            p.settle(0.8)
        if press_save:
            # the Save of the open card, pressed: what a person does once a setting is typed
            p.js("(function(w){for (const h of document.querySelectorAll('section h2')) if (h.textContent.trim().toLowerCase() === w) {"
                 " Array.from(h.closest('section').querySelectorAll('button')).filter(b => b.textContent.trim() === 'Save').forEach(b => b.click()) } return 1})(%s)" % json.dumps(card.lower()))
            p.settle(1.5)
        box = p.js("(function(w){for (const h of document.querySelectorAll('section h2')) if (h.textContent.trim().toLowerCase() === w) {"
                   " const s = h.closest('section'), r = s.getBoundingClientRect();"
                   " return {x: r.left + scrollX, y: r.top + scrollY, w: r.width, h: r.height, text: s.innerText} } return null})(%s)" % json.dumps(card.lower()))
        if not box:
            raise SystemExit("page_card_shot: no card headed %r on %s" % (card, url))
        shot = p.call("Page.captureScreenshot", format="png", captureBeyondViewport=True,
                      clip={"x": box["x"], "y": box["y"], "width": box["w"], "height": box["h"], "scale": 1})
        open(out, "wb").write(base64.b64decode(shot["data"]))
        print(box["text"])
    finally:
        chrome.terminate()


if __name__ == "__main__":
    main()
