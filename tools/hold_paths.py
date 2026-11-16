"""The ways to the commands a held button sends, besides the hold: every button of the page, on each tab and in each state that has one of the three, pressed once (and a second time within a second, for the ones that ask first), and
the commands the stand's log says went out. A command to Start, Stop or Light that went out of a button that is not held is a way round the hold.

    uv run --no-project --with websocket-client --with pyyaml python tools/hold_paths.py

The states are the ones tools/page_plum.py asks the held buttons in: Start where the burner is switched off and stands, Stop where it burns, Light now in the pause. The buttons that are held (they hold a
`[data-hold-fill]`) are not pressed: tools/page_plum.py --only hold is their check. Exits 1 on a way round.

What it does not see: a command that a button sends by a path the stand's log does not carry (the stand writes down every POST), and a button that appears only after a press on another.
"""
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

import page_plum as pp  # noqa: E402

TAG = "(() => { const all = Array.from(document.querySelectorAll('main button')); return JSON.stringify(all.map((b, i) => ({i, text: (b.innerText || b.textContent || '').trim().slice(0, 40), held: !!b.querySelector('[data-hold-fill]'), disabled: b.disabled, tab: b.closest('nav') !== null}))); })()"


def tour(path, second, tabs):
    """(state, tab, button, commands sent) for every button not held of every tab, in one stand and one browser."""
    import page_steady
    from chrome_page import CHROME, Page

    log = tempfile.mktemp(prefix="hold_paths_", suffix=".log")
    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(pp.STAND_PORT), "--scenario", path, "--at", str(second), "--log", log], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={pp.CHROME_PORT}", f"--user-data-dir={tempfile.mkdtemp(prefix='hold_paths_')}", "--no-first-run", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    seen = []
    try:
        targets = None
        for _ in range(75):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{pp.STAND_PORT}/state", timeout=1)
                targets = json.load(urllib.request.urlopen(f"http://127.0.0.1:{pp.CHROME_PORT}/json"))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            raise SystemExit("hold_paths: the stand or Chrome did not come up")
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=pp.WIDTH, height=900, deviceScaleFactor=2, mobile=True)
        url = f"http://127.0.0.1:{pp.STAND_PORT}/"
        p.call("Page.navigate", url=url)
        p.settle(1.0)
        p.js("localStorage.setItem('t', 'l'); localStorage.setItem('lang', 'ru')")
        p.call("Page.navigate", url=url)
        p.settle(2.0)
        if not page_steady.wait_drawn(p):
            raise SystemExit("hold_paths: the page did not draw")
        for tab in tabs:
            p.js("document.querySelectorAll('main nav button')[%d].click(); 1" % pp.TABS[tab])
            p.settle(0.8)
            buttons = json.loads(p.js(TAG))
            for b in buttons:
                if b["held"] or b["disabled"] or b["tab"]:
                    continue
                before = len(open(log).read().splitlines()) if os.path.exists(log) else 0
                # the button is taken by its place among the buttons of the page at the moment of the press: a mark put on it earlier is lost when the page is drawn again
                clicked = p.js("(() => { const e = document.querySelectorAll('main button')[%d]; if (!e || e.disabled) return false; e.click(); setTimeout(() => { const f = document.querySelectorAll('main button')[%d]; if (f && !f.disabled) f.click(); }, 700); return true; })()" % (b["i"], b["i"]))
                p.settle(1.6)
                now = open(log).read().splitlines() if os.path.exists(log) else []
                seen.append((tab, b["text"] + ("" if clicked else " (not pressed)"), pp.sends_of(now[before:])))
                # a button may have moved the page to another tab or opened a form: back to the tab asked
                p.js("document.querySelectorAll('main nav button')[%d].click(); 1" % pp.TABS[tab])
                p.settle(0.3)
        return seen
    finally:
        chrome.terminate()
        stand.terminate()
        chrome.wait(10)
        stand.wait(10)


def main():
    from page_fixes import frames_of
    from page_stand import dictionary_of
    from stand_frames import named

    dictionary = dictionary_of(ROOT, "burner")
    ours = {pp.address_of(dictionary, "thermostat", name): name for name in ("start", "stop", "light")}
    states = [
        ("Start (off, stopped)", "burning_out_then_blow.json", lambda r: pp.switch_of(r) == "off" and pp.stage_of(r) == "stop"),
        ("Stop (on, working)", "lighting.json", lambda r: pp.switch_of(r) == "on" and pp.stage_of(r) == "working"),
        ("Light now (on, in the pause)", "room_stops_with_pause.json", lambda r: pp.switch_of(r) == "on" and pp.stage_of(r) == "stop" and pp.lock_of(r) > 0),
    ]
    bad, pressed, sending = 0, 0, 0
    for label, scenario, picks in states:
        text, frames = frames_of(os.path.join(HERE, "stand_scenarios", scenario))
        path = os.path.join(tempfile.mkdtemp(prefix="hold_paths_"), "frames.json")
        open(path, "w").write(text)
        frame = next((f for f in frames if picks(named(dictionary, f))), None)
        if frame is None:
            print("MISSING   no state of %s for %s" % (scenario, label))
            bad += 1
            continue
        for tab, button, sent in tour(path, frame["at"], ("scheme", "modules", "system")):
            pressed += 1
            sending += bool(sent)
            round_way = [(way, at) for way, at in sent if at in ours]
            if round_way:
                bad += 1
                print("WAY ROUND %s: tab %s, button %r sent %s" % (label, tab, button, [(w, ours[a]) for w, a in round_way]))
    # the tour sees a send when there is one: the buttons that sent some command, any, are counted, so that a tour that sees nothing is not taken for a page with no way round
    print("%d buttons pressed in %d states, %d of them sent a command (any), %d ways round the hold" % (pressed, len(states), sending, bad))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
