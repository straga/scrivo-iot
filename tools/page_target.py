"""The target of the room is set on the scheme: the page on the stand, driven by the new burner's own frames, a hand on the line of the reason and on the panel under it.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_target.py [--shots DIR] [--langs ru,en] [--themes l,d] [--page DIR]

The frame is picked BY VALUE - the first second of tools/stand_scenarios/house_by_the_room.json where the demand says cold under the comfort profile -, and asked in each language and theme, 375 px wide, each verdict printing PASS or FAIL:

  line-is-pressable   the line of the reason is one button, still one row, and the panel is not drawn; the card is as tall as it was before the panel existed (the closed panel adds no height)
  panel-opens         a press opens the panel under the line: the profile's word, the target as the setting holds it with its unit on the right, a button each way, the strip and the hysteresis folded into one line; nothing stands wider than the phone
  five-presses        five quick presses of + move the number by five tenths at once and send NOTHING for the first second; two seconds after the last press the board hears ONE write, `auto_room_c` with the last number, and the stand answers it
  gap-is-folded       the hysteresis opens with a press, a press of + moves its number, and the fire's line moves with it; one write, `auto_room_gap_c`
  refusal-is-said     a write the board refuses, and one the board does not answer, put the number back to the board's and say so in the page's words
  closed-again        a press on the line closes the panel and the card is as tall as before
  goes-on-closed      a number pressed and the panel closed before the 2 s are out: the board hears it once, in its time
  goes-on-away        a number pressed and the tab of the page changed before the 2 s are out: the board hears it once, in its time
  last-write-wins     a number pressed and the same setting set from elsewhere before the 2 s are out: the board hears the other write and then the press - the press goes whatever the board holds (no check before the write)
  refusal-stays       the target and the hysteresis pressed in one 2 s and the board refusing the target and taking the hysteresis, in either order of the answers: the refusal of the target is under the panel after both answers; both taken: nothing is said
  goes-on-leaving     a number pressed and the page put away before the 2 s are out: the board hears it, with the request that outlives the page
  flow-has-five       the drawing carries five arrows of the water's flow and the manifold between the supply and the return, and the manifold stands over no text of the drawing

What is expected is taken from the scenario's settings and the menu's words in the language. The verdicts are `judge_*`: pure functions of what the browser answered, held by tools/test_page_target.py on answers that are good and answers that
are not. Exits 1 when a check failed.
"""
import argparse
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

WIDTH = 375
STAND_PORT = 8803
CHROME_PORT = 9419
SCENARIO = "house_by_the_room.json"
# how long the page waits after the last press before it writes, and how much longer than that the probe waits to be sure it has
WAIT = 2.0
SLACK = 0.8
# a pixel of rounding in a height
HEIGHT = 1.5
# how much two boxes may overlap and still stand apart
OVERLAP = 1.0


# ---- what the browser is asked ----

PEEK = """(function(){
  const rect = (e) => { if (!e) return null; const r = e.getBoundingClientRect(); return {left: r.left, right: r.right, top: r.top, bottom: r.bottom, width: r.width, height: r.height}; };
  const q = (s) => document.querySelector(s);
  const card = q('[data-ask]') ? q('[data-ask]').closest('section') : null;
  const panel = q('[data-target-panel]');
  const toggle = q('[data-target-toggle]');
  const ask = q('[data-drawn=ask]');
  const svg = card ? card.querySelector('svg.text-ink') : null;
  const texts = svg ? Array.from(svg.querySelectorAll('text')).map((t) => ({text: t.textContent.trim(), box: rect(t)})) : [];
  const wide = panel ? Array.from(panel.querySelectorAll('*')).filter((e) => e.getBoundingClientRect().right > %(width)d + 0.5).map((e) => e.tagName + ' ' + (e.textContent || '').trim().slice(0, 20)) : [];
  const unit = q('[data-target-value]');
  return JSON.stringify({
    card: rect(card), cardHeight: card ? card.getBoundingClientRect().height : null,
    toggle: !!toggle, expanded: toggle ? toggle.getAttribute('aria-expanded') : null, ask: ask ? ask.textContent.trim() : null, askBox: rect(ask), askLine: ask ? parseFloat(getComputedStyle(ask).lineHeight) || null : null,
    panel: !!panel, profile: q('[data-target-profile]') ? q('[data-target-profile]').textContent.trim() : null,
    value: unit ? unit.firstChild.textContent.trim() : null, unit: unit && unit.querySelector('small') ? unit.querySelector('small').textContent.trim() : null,
    valueBox: rect(unit), downBox: rect(q('[data-target-down]')), upBox: rect(q('[data-target-up]')),
    downOff: q('[data-target-down]') ? q('[data-target-down]').disabled : null, upOff: q('[data-target-up]') ? q('[data-target-up]').disabled : null,
    strip: !!q('[data-target-strip]'), band: rect(q('[data-strip-band]')), mark: rect(q('[data-strip-target]')), room: rect(q('[data-strip-room]')),
    gapValue: q('[data-gap-value]') ? q('[data-gap-value]').textContent.trim() : null, gapFire: q('[data-gap-fire]') ? q('[data-gap-fire]').textContent.trim() : null,
    gapOpen: !!q('[data-gap-panel]'), gapAria: q('[data-gap-toggle]') ? q('[data-gap-toggle]').getAttribute('aria-label') : null,
    said: q('[data-target-said]') ? q('[data-target-said]').textContent.trim() : null,
    wide, scrollWidth: document.documentElement.scrollWidth,
    arrows: svg ? svg.querySelectorAll('path.fill-flow-hot, path.fill-flow-cold').length : 0,
    hot: svg ? svg.querySelectorAll('path.fill-flow-hot').length : 0, cold: svg ? svg.querySelectorAll('path.fill-flow-cold').length : 0,
    manifold: rect(svg ? svg.querySelector('[data-drawn=manifold]') : null), texts,
  });
})()""" % {"width": WIDTH}


def click(p, selector):
    """A press on the first element the SELECTOR names, the way a finger does it."""
    return p.js("(function(){const e=document.querySelector(%s); if(!e) return 0; e.click(); return 1})()" % json.dumps(selector))


def writes(stand_url):
    """The writes of settings the board has heard, in order: (name, value) of each pair of each POST /settings."""
    import urllib.parse

    whole = json.load(urllib.request.urlopen(stand_url + "/sent"))
    out = []
    for one in whole:
        if one["what"] == "/settings":
            out.extend(urllib.parse.parse_qsl(one["body"] or "", keep_blank_values=True))
    return out


# ---- the verdicts: pure functions of what the browser answered ----

def judge_line_is_pressable(said, units=None):
    """The line is one button that has not opened anything, still one row, and the panel is not drawn. -> (ok, words)"""
    out = []
    if not said["toggle"]:
        out.append("the line is no button")
    if said["panel"]:
        out.append("the panel is drawn while closed")
    if said["expanded"] != "false":
        out.append("aria-expanded is %r where the panel is closed" % said["expanded"])
    one = said["askLine"]
    if one and said["askBox"] and said["askBox"]["height"] >= one * 1.5:
        out.append("the line is %.0f px tall, a line of its font is %.0f px: it wraps" % (said["askBox"]["height"], one))
    return (not out), "; ".join(out) if out else "'%s' is a button, closed" % said["ask"]


# A target goes by tenths of a degree (9.33): a press is STEP, the board hears the number as the page sends it (21.5, or 26 where the tenth is nought), and the panel says it with its tenth (21.5, 26.0).
STEP = 0.1


def sent(x):
    return "%g" % round(x, 1)


def shown(x):
    return "%.1f" % round(x, 1)


def judge_panel_opens(said, expect):
    """The panel stands with the profile's word, the target and its unit, a button each way, the strip and the folded hysteresis, and nothing is wider than the phone. EXPECT: `profile`, `value`, `unit`. -> (ok, words)"""
    out = []
    if not said["panel"]:
        return False, "the panel is not drawn"
    if said["expanded"] != "true":
        out.append("aria-expanded is %r where the panel is open" % said["expanded"])
    if said["profile"] != expect["profile"]:
        out.append("the profile says %r, the word is %r" % (said["profile"], expect["profile"]))
    if said["value"] != expect["value"]:
        out.append("the target says %r, the setting holds %r" % (said["value"], expect["value"]))
    if said["unit"] != expect["unit"]:
        out.append("the unit is %r, it is %r" % (said["unit"], expect["unit"]))
    elif said["valueBox"] and said["downBox"] and said["upBox"] and not (said["downBox"]["right"] <= said["valueBox"]["left"] + OVERLAP and said["valueBox"]["right"] <= said["upBox"]["left"] + OVERLAP):
        out.append("the buttons and the number are not in a row: - number +")
    for name in ("downBox", "upBox"):
        box = said[name]
        if box is None or box["width"] < 44 - 0.5 or box["height"] < 44 - 0.5:
            out.append("a button is smaller than 44 px")
    if not said["strip"] or not said["band"] or not said["mark"]:
        out.append("the strip lacks its band or its target")
    if said["gapValue"] is None or said["gapFire"] is None:
        out.append("the hysteresis is not folded under the strip")
    if said["wide"] or said["scrollWidth"] > WIDTH:
        out.append("wider than the phone: %s (page %d px)" % (", ".join(said["wide"]) or "-", said["scrollWidth"]))
    return (not out), "; ".join(out) if out else "%s %s %s, hysteresis %s, fire at %s" % (said["profile"], said["value"], said["unit"], said["gapValue"], said["gapFire"])


def judge_one_write(heard, expect):
    """The board heard exactly the writes EXPECT names - a list of (name, value), each once -, and no other. -> (ok, words)"""
    return (heard == expect), ("the board heard %s once" % (expect,)) if heard == expect else "the board heard %s, it should hear %s" % (heard, expect)


def judge_nothing_yet(heard):
    """Nothing has reached the board while the hand is still on the panel. -> (ok, words)"""
    return (not heard), "the board heard %s before the wait was over" % heard if heard else "the board has heard nothing"


def judge_back_to_the_board(said, board_value, unit_said):
    """After a refused or unanswered write the number is the board's again and the page has said why in words (not empty, not a code). -> (ok, words)"""
    out = []
    if said["value"] != board_value:
        out.append("the number is %r where the board holds %r" % (said["value"], board_value))
    if not (said["said"] or "").strip():
        out.append("nothing is said of the refusal")
    return (not out), "; ".join(out) if out else "back at %s, said: '%s'" % (said["value"], said["said"])


def judge_same_height(before, after):
    """The card is as tall as it was, to a pixel of rounding. -> (ok, words)"""
    ok = abs(before["cardHeight"] - after["cardHeight"]) <= HEIGHT
    return ok, "%.0f px before, %.0f px after" % (before["cardHeight"], after["cardHeight"])


def judge_flow(said):
    """Five arrows - two of the supply and three of the return -, the manifold between the supply and the return, and the manifold over no text. -> (ok, words)"""
    out = []
    if (said["hot"], said["cold"]) != (2, 3):
        out.append("%d warm and %d cold arrows where two and three stand" % (said["hot"], said["cold"]))
    box = said["manifold"]
    if box is None:
        return False, "; ".join(out + ["the manifold is not drawn"])
    over = [t["text"] for t in said["texts"] if t["box"] and t["text"] and not (t["box"]["right"] <= box["left"] + OVERLAP or t["box"]["left"] >= box["right"] - OVERLAP or t["box"]["bottom"] <= box["top"] + OVERLAP or t["box"]["top"] >= box["bottom"] - OVERLAP)]
    if over:
        out.append("the manifold lies over %s" % ", ".join("'%s'" % t for t in over))
    return (not out), "; ".join(out) if out else "5 arrows, the manifold over no text"


# ---- the run ----

def main():
    import page_fixes
    from chrome_page import CHROME, Page
    from page_sources import merged_menu
    from page_stand import dictionary_of
    from stand_frames import named
    import page_steady

    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", default=None)
    ap.add_argument("--langs", default="ru,en")
    ap.add_argument("--themes", default="l,d")
    ap.add_argument("--page", default=None, help="the page files to measure instead of the device's own - a revision taken out of git")
    args = ap.parse_args()
    menu = merged_menu("burner")
    dictionary = dictionary_of(ROOT, "burner")
    scenario_file = os.path.join(HERE, "stand_scenarios", SCENARIO)
    settings = json.load(open(scenario_file))["settings"]
    text, frames = page_fixes.frames_of(scenario_file)
    frame = next((f for f in frames if named(dictionary, f).get(("demand", "asked"), {}).get("word") == "cold" and named(dictionary, f).get(("demand", "profile"), {}).get("word") == "comfort"), None)
    if frame is None:
        raise SystemExit("page_target: the run never comes to cold under the comfort profile")
    work = tempfile.mkdtemp(prefix="page_target_")
    frames_file = os.path.join(work, "frames.json")
    with open(frames_file, "w") as f:
        f.write(text)
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    start = float(settings["auto_room_c"])
    gap = int(settings["auto_room_gap_c"])
    ok = True
    stand_url = "http://127.0.0.1:%d" % STAND_PORT

    def verdict(where, check, result):
        nonlocal ok
        passed, words_ = result
        print("%-10s %-18s %s  %s" % (where, check, "PASS" if passed else "FAIL", words_))
        ok = ok and passed

    try:
        for lang in args.langs.split(","):
            for theme in args.themes.split(","):
                where = "%s/%s" % (lang, theme)
                stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT), "--scenario", frames_file, "--at", str(frame["at"]), "--settings-of", scenario_file]
                                         + (["--page", args.page] if args.page else []), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                profile = tempfile.mkdtemp(prefix="page_target_chrome_")
                chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % CHROME_PORT, "--user-data-dir=%s" % profile, "--no-first-run", "about:blank"],
                                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                try:
                    targets = None
                    for _ in range(75):
                        try:
                            urllib.request.urlopen(stand_url + "/state", timeout=1)
                            targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % CHROME_PORT))
                            break
                        except OSError:
                            time.sleep(0.2)
                    if targets is None:
                        raise SystemExit("page_target: the stand or Chrome did not come up")
                    p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
                    p.call("Page.enable")
                    p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=900, deviceScaleFactor=2, mobile=True)
                    p.call("Page.navigate", url=stand_url + "/")
                    p.settle(1.0)
                    p.js("localStorage.setItem('t', %s); localStorage.setItem('lang', %s)" % (json.dumps(theme), json.dumps(lang)))
                    p.call("Page.navigate", url=stand_url + "/")
                    p.settle(2.0)
                    if not page_steady.wait_drawn(p):
                        raise SystemExit("page_target: the page did not draw - nothing is said of it")
                    peek = lambda: json.loads(p.js(PEEK))  # noqa: E731
                    words = menu["scheme"]["target"]
                    unit = "°C"

                    closed = peek()
                    verdict(where, "line-is-pressable", judge_line_is_pressable(closed))

                    click(p, "[data-target-toggle]")
                    p.settle(0.4)
                    opened = peek()
                    verdict(where, "panel-opens", judge_panel_opens(opened, {"profile": words["comfort"][lang], "value": shown(start), "unit": unit}))
                    if args.shots:
                        p.shot(os.path.join(args.shots, "panel_%s_%s.png" % (lang, theme)), full=True)

                    # five quick presses
                    for _ in range(5):
                        click(p, "[data-target-up]")
                        p.settle(0.15)
                    after_presses = peek()
                    p.settle(1.0)
                    verdict(where, "five-presses/quiet", judge_nothing_yet(writes(stand_url)))
                    p.settle(WAIT + SLACK)
                    heard = writes(stand_url)
                    now = peek()
                    result = judge_one_write(heard, [("auto_room_c", sent(start + 5 * STEP))])
                    shown_ok = after_presses["value"] == shown(start + 5 * STEP) and now["value"] == shown(start + 5 * STEP)
                    verdict(where, "five-presses", (result[0] and shown_ok, result[1] + ("" if shown_ok else "; the page shows %s then %s, not %s" % (after_presses["value"], now["value"], shown(start + 5 * STEP)))))

                    # the hysteresis
                    click(p, "[data-gap-toggle]")
                    p.settle(0.3)
                    click(p, "[data-gap-up]")
                    p.settle(WAIT + SLACK)
                    folded = peek()
                    heard = writes(stand_url)[1:]
                    fire_ok = folded["gapFire"] == "%s %s" % (shown(start + 5 * STEP - (gap + 1)), unit) and folded["gapValue"] == "−%d" % (gap + 1)
                    verdict(where, "gap-is-folded", (heard == [("auto_room_gap_c", str(gap + 1))] and folded["gapOpen"] and fire_ok, "the board heard %s; hysteresis %s, fire at %s, panel %s" % (heard, folded["gapValue"], folded["gapFire"], "open" if folded["gapOpen"] else "closed")))

                    # a refusal, and a board that does not answer
                    refusals = []
                    for how, script in (("refused", "Promise.resolve(new Response(JSON.stringify({error: 'the board says no'})))"), ("silent", "Promise.reject(new Error('down'))")):
                        p.js("window.__fetch = window.__fetch || window.fetch; window.fetch = (u, o) => String(u).includes('/settings') ? %s : window.__fetch(u, o); 1" % script)
                        click(p, "[data-target-up]")
                        p.settle(WAIT + SLACK)
                        said = peek()
                        refusals.append((how, said))
                        p.js("window.fetch = window.__fetch; 1")
                        if how == "refused":
                            p.js("document.querySelector('[data-target-down]').click(); 1")
                            p.settle(WAIT + SLACK)
                    board_value = shown(start + 5 * STEP)
                    for how, said in refusals:
                        # the board keeps what it was told before: start + 5 after the first write; the unanswered press is judged the same
                        verdict(where, "refusal-is-said/" + how, judge_back_to_the_board(said, board_value if how == "refused" else peek()["value"], unit))

                    click(p, "[data-target-toggle]")
                    p.settle(0.4)
                    back = peek()
                    verdict(where, "closed-again", (not back["panel"] and judge_same_height(closed, back)[0], "panel %s; %s" % ("drawn" if back["panel"] else "gone", judge_same_height(closed, back)[1])))
                    verdict(where, "flow-has-five", judge_flow(back))

                    # what happens to a number that waits: the panel closed, the tab changed, the page put away
                    def reopened():
                        p.call("Page.navigate", url=stand_url + "/")
                        p.settle(2.0)
                        page_steady.wait_drawn(p)
                        # the stand holds two streams and frees one a moment after a page is gone: the panel is waited for, not assumed
                        for _ in range(40):
                            if click(p, "[data-target-toggle]"):
                                break
                            p.settle(0.5)
                        p.settle(0.4)
                        return peek()

                    said = reopened()
                    first = float(said["value"])
                    base = len(writes(stand_url))
                    click(p, "[data-target-up]")
                    p.settle(0.3)
                    click(p, "[data-target-toggle]")
                    p.settle(WAIT + SLACK)
                    verdict(where, "goes-on-closed", judge_one_write(writes(stand_url)[base:], [("auto_room_c", sent(first + STEP))]))

                    base = len(writes(stand_url))
                    click(p, "[data-target-toggle]")
                    p.settle(0.4)
                    click(p, "[data-target-up]")
                    p.settle(0.3)
                    p.js("(function(){const b=Array.from(document.querySelectorAll('button')).find(x=>/\\u041c\\u043e\\u0434\\u0443\\u043b\\u0438|Modules/.test(x.textContent)); if(b) b.click(); return 1})()")
                    p.settle(WAIT + SLACK)
                    verdict(where, "goes-on-away", judge_one_write(writes(stand_url)[base:], [("auto_room_c", sent(first + 2 * STEP))]))

                    said = reopened()
                    value = float(said["value"])
                    base = len(writes(stand_url))
                    click(p, "[data-target-up]")
                    p.settle(0.3)
                    p.call("Page.navigate", url="about:blank")
                    time.sleep(1.2)
                    verdict(where, "goes-on-leaving", judge_one_write(writes(stand_url)[base:], [("auto_room_c", sent(value + STEP))]))

                    said = reopened()
                    value = float(said["value"])
                    base = len(writes(stand_url))
                    click(p, "[data-target-up]")
                    p.settle(0.3)
                    urllib.request.urlopen(urllib.request.Request(stand_url + "/settings", data=("auto_room_c=%s" % sent(value + 5 * STEP)).encode(), method="POST"))
                    p.settle(WAIT + SLACK)
                    verdict(where, "last-write-wins", judge_one_write(writes(stand_url)[base:], [("auto_room_c", sent(value + 5 * STEP)), ("auto_room_c", sent(value + STEP))]))

                    # the target and the hysteresis pressed in one 2 s, and the board answering them differently: the refusal of one stays under the panel whatever the other answer is and whenever it comes
                    def both_pressed(refuse_ms, ok_ms):
                        reopened()
                        if refuse_ms is not None:
                            p.js("window.__fetch = window.__fetch || window.fetch; window.fetch = (u, o) => { if (!String(u).includes('/settings')) return window.__fetch(u, o); const refuse = String((o && o.body) || '').startsWith('auto_room_c='); return new Promise((r) => setTimeout(r, refuse ? %d : %d)).then(() => refuse ? new Response(JSON.stringify({error: 'target refused'})) : window.__fetch(u, o)); }; 1" % (refuse_ms, ok_ms))
                        click(p, "[data-gap-toggle]")
                        p.settle(0.3)
                        click(p, "[data-target-down]")
                        p.settle(0.3)
                        click(p, "[data-gap-up]")
                        p.settle(WAIT + 1.8)
                        p.js("window.fetch = window.__fetch || window.fetch; 1")
                        return peek()["said"]

                    for label, refuse_ms, ok_ms in (("refusal-first", 0, 700), ("refusal-last", 700, 0)):
                        said = both_pressed(refuse_ms, ok_ms)
                        verdict(where, "refusal-stays/" + label, ("target refused" in (said or ""), "under the panel: %r" % said))
                    said = both_pressed(None, None)
                    verdict(where, "both-taken", ((said or "") == "", "under the panel: %r" % said))
                finally:
                    chrome.terminate()
                    stand.terminate()
                    chrome.wait(10)
                    stand.wait(10)
    finally:
        if os.path.exists(frames_file):
            os.remove(frames_file)
        if os.path.isdir(work) and not os.listdir(work):
            os.rmdir(work)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
