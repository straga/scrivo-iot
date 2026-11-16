"""The burner's controls as Plum lays them out, asked of the page in a browser on the stand: one button that runs the burner, the pause's "light now", the manual control in the Service of the System tab, and nothing on the scheme that settings.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_plum.py [--shots DIR] [--langs ru,en] [--themes l,d] [--only run|pause|service|scheme]

The states are picked BY VALUE, the first frame of a scenario where the board's rows say it (tools/stand_scenarios): the switch on or off by the thermostat's `switch`, the stage by the automaton's `doing`, the pause by the
thermostat's `lock`. In each, in each language and theme, 375 px wide, each verdict printing PASS or FAIL:

  one-run-button     the page has ONE button that starts or stops the burner, on the scheme tab and nowhere else: no other button on the scheme, the modules tabs or the system tab has the words of Start or Stop
  run-by-switch      the button says Stop where the switch is on, Start where it is off, and Stop in a fault
  start-refused      where the fire burns out and the switch is off the button is disabled and the reason stands under it; in no other state is a reason there
  light-now          while the switch is on and the burner stands in a stop with no fire (in the pause, and where heat is not needed) the scheme has "light now" beside the line that says why it stands, and nowhere else; it sends the press of Light
  scheme-is-quiet    the scheme tab has no Auto / Semi switch and nothing that is typed into: no setting, no section of settings
  service-has-hands  the System tab has the Service under its heading, with the sections the menu groups there: the manual control, the relays, the fan's hands and the numbers of the fire
  one-press          two presses of the button in a row - at once, within the moment after the board's answer - send ONE command to the board; a press after that moment is a press of the new word and is sent; the same for light now,
                     for Take / Give back and for the feed's button on the hopper: a second click never takes the first back
  hold-to-press      Start, Stop and Light now are pressed by a HOLD of about a second, with a touch and with a mouse: a tap, a hold released at 0.6 s and a touch that scrolled send nothing, a hold of 1.3 s sends ONE
                     command, the button fills while it is held, and the feed's ban stays a single touch
  hands-button       the one button of the hands says Take by hand where the hands are free, with the words of what it does under it, and a press of it sends `on` to the automaton's switch `semi`

What is expected is taken from the stand's frame (the rows by value) and the menu's words in the language: a word a person changes is not a failing row. The verdicts are `judge_*`: pure functions of what the browser
answered, and tools/test_page_plum.py holds them on answers that are good and answers that are not. Exits 1 when a check failed.
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

from page_sources import merged_menu  # noqa: E402

WIDTH = 375
CHROME_PORT = 9421
STAND_PORT = 8805
TABS = {"scheme": 0, "modules": 1, "system": 2}

# what the browser is asked on a tab: the buttons that run the burner, the reason under them, every button's words, the titles of the cards, and whether the scheme has the controls of a setting
PROBE = r"""
(() => {
  const text = (e) => (e.innerText || e.textContent || "").trim();
  const main = document.querySelector("main");
  return JSON.stringify({
    run: Array.from(document.querySelectorAll("[data-run]")).map((b) => ({ text: text(b), disabled: b.disabled, press: b.getAttribute("data-press") })),
    cannot: Array.from(document.querySelectorAll("[data-run-cannot]")).map(text),
    now: Array.from(document.querySelectorAll("[data-light-now]")).map((b) => ({ text: text(b), disabled: b.disabled })),
    buttons: Array.from(main.querySelectorAll("button")).map((b) => text(b)),
    titles: Array.from(main.querySelectorAll("section h2")).map(text),
    mode: document.querySelectorAll("[data-mode]").length,
    inputs: main.querySelectorAll("input, select, textarea").length,
    ask: Array.from(document.querySelectorAll("[data-ask]")).map(text),
    hands: Array.from(document.querySelectorAll("[data-hands]")).map((b) => ({ text: text(b), disabled: b.disabled })),
    handsText: Array.from(document.querySelectorAll("[data-hands-text]")).map(text),
    service: document.querySelectorAll("[data-service]").length,
    note: Array.from(document.querySelectorAll("[data-note]")).map(text).join(" "),
    wide: document.documentElement.scrollWidth,
    text: text(main)
  });
})()
"""


def words_of(menu, lang):
    """The words the page says in LANG that the verdicts look for: the two of the run button, the reason, the pause's `light now`."""
    run = menu["scheme"]["run"]
    return {"start": run["start"][lang], "stop": run["stop"][lang], "burning_off": run["burning_off"][lang]}


def semi_address(dictionary):
    """The address of the automaton's switch `semi` in the board's dictionary."""
    return next(at for at, thing in enumerate(dictionary["things"]) if thing["card"] == "automaton" and thing["name"] == "semi")


def values(frame):
    """What a frame of a scenario says of the rows the verdicts decide by: the switch and the stage by their words, the pause by its seconds."""
    return {"switch": frame.get("thermostat/switch"), "stage": frame.get("automaton/doing"), "lock": frame.get("thermostat/lock")}


# ---- the verdicts: pure functions of what the browser answered ----

def judge_one_run_button(said, words):
    """The scheme has exactly one button that runs the burner, and no other button of the scheme or of the modules tab says Start or Stop. -> (ok, words)"""
    count = len(said["scheme"]["run"])
    if count != 1:
        return False, "%d buttons that run the burner on the scheme, expected 1" % count
    names = {words["start"], words["stop"]}
    # the system tab is not asked: the Service has a `Stop` beside each slider of the fan, which lets the hand go - it runs nothing
    for tab in ("scheme", "modules"):
        others = [t for t in said[tab]["buttons"] if t in names]
        allowed = 1 if tab == "scheme" else 0
        if len(others) != allowed:
            return False, "the %s tab has %d buttons that say %s, expected %d" % (tab, len(others), "/".join(sorted(names)), allowed)
    return True, "one button, on the scheme"


def judge_run_by_switch(said, words, switch, stage):
    """The button says what the switch and the stage call for: Stop where the switch is on, Start where it is off or never pressed, Stop in a fault. -> (ok, words)"""
    run = said["scheme"]["run"]
    if len(run) != 1:
        return False, "%d buttons, expected 1" % len(run)
    want = "stop" if (stage == "fault" or switch == "on") else "start"
    if run[0]["press"] != want or run[0]["text"] != words[want]:
        return False, "the switch is %s and the stage %s: the button says %r (press %s), expected %r (press %s)" % (switch, stage, run[0]["text"], run[0]["press"], words[want], want)
    return True, "%s for switch %s, stage %s" % (want, switch, stage)


def judge_start_refused(said, words, switch, stage):
    """Where the burner is switched off and burns out, the button is disabled and the reason stands under it; in every other state it is enabled and no reason stands. -> (ok, words)"""
    run = said["scheme"]["run"]
    reasons = said["scheme"]["cannot"]
    refused = switch in ("off", "unasked") and stage in ("burning_off", "burning_off_because") and stage != "fault"
    if len(run) != 1:
        return False, "%d buttons, expected 1" % len(run)
    if refused:
        if not run[0]["disabled"] or reasons != [words["burning_off"]]:
            return False, "the fire burns out and the switch is %s: disabled %s, reasons %s, expected a disabled button and %r" % (switch, run[0]["disabled"], reasons, words["burning_off"])
        return True, "disabled, with the reason"
    if run[0]["disabled"] or reasons:
        return False, "switch %s, stage %s: disabled %s, reasons %s, expected an enabled button and no reason" % (switch, stage, run[0]["disabled"], reasons)
    return True, "enabled, no reason"


def judge_light_now(said, words, switch, stage):
    """`Light now` stands in the row of the line that says why the burner stands - while the switch is on and the burner stands in a stop with no fire (in a pause, and where heat is not needed) -, once, and in no other state. -> (ok, words)"""
    now = said["scheme"]["now"]
    expected = 1 if (switch == "on" and stage == "stop") else 0
    if len(now) != expected:
        return False, "switch %s, stage %s: %d buttons of light-now, expected %d" % (switch, stage, len(now), expected)
    if expected and now[0]["disabled"]:
        return False, "light-now is disabled"
    return True, "%d" % expected


def judge_scheme_is_quiet(said):
    """The scheme tab holds no mode switch and nothing that is typed into, and none of the sections of the settings the page had on it (the boiler's, the thermostat's, the schedule's). -> (ok, words)"""
    scheme = said["scheme"]
    if scheme["mode"]:
        return False, "the scheme has %d mode switches" % scheme["mode"]
    if scheme["inputs"]:
        return False, "the scheme has %d fields" % scheme["inputs"]
    return True, "no mode switch, no field"


def judge_service_has_hands(said, titles):
    """The System tab has the Service heading once, and every section the menu groups under it, by its title. -> (ok, words)"""
    have = said["system"]["titles"]
    if said["system"]["service"] != 1:
        return False, "the system tab has %d Service headings, expected 1" % said["system"]["service"]
    missing = [t for t in titles if t not in have]
    if missing:
        return False, "the system tab has %s, no %s" % (have, missing)
    return True, "service: %s" % ", ".join(titles)


def judge_hands_button(said, take, take_text, sent=None, address=None):
    """With the hands free the System tab has one button, enabled, saying TAKE, and TAKE_TEXT under it; with SENT - the stand's log after a press of it - a command `on` to ADDRESS, the automaton's switch `semi`, was sent. -> (ok, words)"""
    hands = said["system"]["hands"]
    if len(hands) != 1:
        return False, "%d buttons of the hands, expected 1" % len(hands)
    if hands[0]["text"] != take or hands[0]["disabled"]:
        return False, "the button says %r (disabled %s), expected %r enabled" % (hands[0]["text"], hands[0]["disabled"], take)
    if said["system"]["handsText"] != [take_text]:
        return False, "the text under it is %s, expected %r" % (said["system"]["handsText"], take_text)
    if sent is not None and not any("/on/%d" % address in line for line in sent):
        return False, "a press sent %s, expected a command `on` to %d" % (sent, address)
    return True, "take by hand" + ("" if sent is None else ", sent on to %d" % address)


def sends_of(lines):
    """The commands the stand's log says were sent, in order: (way, address) of every on, off and press."""
    import re
    return [(m.group(1), int(m.group(2))) for line in lines for m in [re.search(r"/(on|off|press)/(\d+)", line)] if m]


def judge_one_press(sent, expected, what):
    """The commands sent for WHAT - the clicks a person made - are EXPECTED, in order, and no others: where two clicks came in a row the first is the only one sent, and where the second came after the moment the board has said it took the first it is sent too. -> (ok, words)"""
    got = sends_of(sent)
    if got != expected:
        return False, "%s: sent %s, expected %s" % (what, got, expected)
    return True, "%s: sent %s" % (what, got)


def judge_hands_given(said, give, give_text, sent=None, address=None):
    """With the hands taken the System tab has one button, enabled, saying GIVE, and GIVE_TEXT - what the automaton takes back - under it; with SENT a command `off` to ADDRESS, the automaton's switch `semi`, was sent. -> (ok, words)"""
    hands = said["system"]["hands"]
    if len(hands) != 1:
        return False, "%d buttons of the hands, expected 1" % len(hands)
    if hands[0]["text"] != give or hands[0]["disabled"]:
        return False, "the button says %r (disabled %s), expected %r enabled" % (hands[0]["text"], hands[0]["disabled"], give)
    if said["system"]["handsText"] != [give_text]:
        return False, "the text under it is %s, expected %r" % (said["system"]["handsText"], give_text)
    if sent is not None and not any("/off/%d" % address in line for line in sent):
        return False, "a press sent %s, expected a command `off` to %d" % (sent, address)
    return True, "give back" + ("" if sent is None else ", sent off to %d" % address)


# ---- the browser ----

def look(frames_file, second, lang, theme, tabs=("scheme",), shot_dir=None, name=None, press=None, state=None, clicks=None, gestures=None):
    """The stand on SECOND of the scenario, the page in LANG and THEME, asked on each of TABS. Without FRAMES_FILE the stand plays no scenario and holds STATE, a file of values by address. With PRESS - the probe's selector - that element is clicked after the asking, and the stand's log of what was sent is returned as `sent`."""
    import page_steady
    from chrome_page import CHROME, Page

    log = tempfile.mktemp(prefix="page_plum_", suffix=".log")
    plays = ["--scenario", frames_file, "--at", str(second)] if frames_file else ["--state", state]
    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT)] + plays + ["--log", log],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    profile = tempfile.mkdtemp(prefix="page_plum_")
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={CHROME_PORT}", f"--user-data-dir={profile}", "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        targets = None
        for _ in range(75):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{STAND_PORT}/state", timeout=1)
                targets = json.load(urllib.request.urlopen(f"http://127.0.0.1:{CHROME_PORT}/json"))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            raise SystemExit("page_plum: the stand or Chrome did not come up")
        page_target = next(t for t in targets if t["type"] == "page")
        p = Page(page_target["webSocketDebuggerUrl"])
        p.target_id = page_target["id"]
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=900, deviceScaleFactor=2, mobile=True)
        p.call("Emulation.setTouchEmulationEnabled", enabled=True, maxTouchPoints=1)
        url = f"http://127.0.0.1:{STAND_PORT}/"
        p.call("Page.navigate", url=url)
        p.settle(1.0)
        p.js("localStorage.setItem('t', %s); localStorage.setItem('lang', %s)" % (json.dumps(theme), json.dumps(lang)))
        p.call("Page.navigate", url=url)
        p.settle(2.0)
        if not page_steady.wait_drawn(p):
            raise SystemExit("page_plum: the page did not draw - the run did not take place; nothing is said of the page")
        said = {}
        for tab in tabs:
            p.js("document.querySelectorAll('main nav button')[%d].click(); 1" % TABS[tab])
            p.settle(0.8)
            said[tab] = json.loads(p.js(PROBE))
            if shot_dir:
                p.shot(os.path.join(shot_dir, "%s_%s_%s_%s.png" % (name, tab, lang, theme)), full=True)
        if press:
            p.js("document.querySelector(%s).click(); 1" % json.dumps(press))
            p.settle(1.0)
            said["sent"] = [line for line in open(log).read().splitlines() if line.strip()] if os.path.exists(log) else []
        if clicks:
            # CLICKS: (selector, seconds to wait after it) - each is a click of a person's finger on the element as the page has it at that moment, a button the page has drawn idle included
            for selector, wait in clicks:
                p.js("(() => { const e = document.querySelector(%s); if (e && !e.disabled) e.click(); })()" % json.dumps(selector))
                if wait:
                    p.settle(wait)
            p.settle(0.5)
            said["sent"] = [line for line in open(log).read().splitlines() if line.strip()] if os.path.exists(log) else []
            said["after"] = json.loads(p.js(PROBE))
            if shot_dir:
                p.shot(os.path.join(shot_dir, "%s_%s_%s_%s_after.png" % (name, tabs[-1], lang, theme)), full=False)
        if gestures:
            # GESTURES: what a hand does to an element, with the input of the browser itself (touch or mouse), not a click made from the page; see `gesture`
            said["fills"] = []
            for g in gestures:
                said["fills"].append(gesture(p, g))
                if g.get("wait"):
                    p.settle(g["wait"])
            p.settle(0.5)
            said["sent"] = [line for line in open(log).read().splitlines() if line.strip()] if os.path.exists(log) else []
            said["after"] = json.loads(p.js(PROBE))
        return said
    finally:
        chrome.terminate()
        stand.terminate()
        chrome.wait(10)
        stand.wait(10)


# how long a hold has to last, in seconds, and the holds the checks make: a tap, a hold let go early, a full hold
HOLD = 1.0
TAP, EARLY, FULL = 0.1, 0.6, 1.3

FILL = "(() => { const e = document.querySelector(%s); const f = e && e.querySelector('[data-hold-fill]'); return f ? f.getBoundingClientRect().width / e.getBoundingClientRect().width : null; })()"
WHERE = "(() => { const e = document.querySelector(%s); if (!e) return null; e.scrollIntoView({block: 'center'}); const r = e.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()"


KEYS = {
    "Enter": {"key": "Enter", "code": "Enter", "windowsVirtualKeyCode": 13},
    " ": {"key": " ", "code": "Space", "windowsVirtualKeyCode": 32},
    "Escape": {"key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27},
}


def press_key(p, name, down=True, up=True):
    if down:
        p.call("Input.dispatchKeyEvent", type="rawKeyDown", **KEYS[name])
    if up:
        p.call("Input.dispatchKeyEvent", type="keyUp", **KEYS[name])


def happen(p, what, state):
    """Something that happens to the page while a hand is on a button. 'away': another tab is brought to the front, so this page is hidden (visibilitychange - the browser's own); 'back': this tab again;
    'freeze' and 'thaw': the browser stops the page's timers and runs them again, as a phone does with a page put away; 'blur': the window loses focus (a made-up event: a headless browser has no window to take focus from);
    'escape': the Escape key; 'other_key': the other of the two keys that hold, pressed and let go."""
    if what == "away":
        state["other"] = p.call("Target.createTarget", url="about:blank")["targetId"]
        p.call("Target.activateTarget", targetId=state["other"])
    elif what == "back":
        p.call("Target.activateTarget", targetId=p.target_id)
    elif what == "freeze":
        state["froze"] = True
        p.call("Page.setWebLifecycleState", state="frozen")
    elif what == "thaw":
        p.call("Page.setWebLifecycleState", state="active")
    elif what == "blur":
        p.js("window.dispatchEvent(new Event('blur')); 1")
    elif what == "escape":
        press_key(p, "Escape")
    elif what == "other_key":
        press_key(p, " " if state["key"] == "Enter" else "Enter")


def gesture(p, g):
    """A hand on the element G['selector'] for G['hold'] seconds, with a finger (G['kind'] 'touch'), a mouse ('mouse') or a key held down ('key', G['key'] Enter or a space); G['move'] pixels down in the first third of the hold, as a finger scrolls or a mouse
    is dragged away; G['events'] - (second, what) - what happens to the page meanwhile (`happen`). The width of the button's fill, as a share of the button's, is read at each second of G['sample'] after the press and once after the release.
    -> {'during': [...], 'after': share or None}"""
    keyed = g["kind"] == "key"
    if keyed:
        if not p.js("(() => { const e = document.querySelector(%s); if (!e) return false; e.focus(); return true; })()" % json.dumps(g["selector"])):
            return {"during": [], "after": None, "missing": True}
        x = y = 0
    else:
        at = p.js(WHERE % json.dumps(g["selector"]))
        if not at:
            return {"during": [], "after": None, "missing": True}
        x, y = at
    touch = g["kind"] == "touch"
    state = {"other": None, "key": g.get("key")}
    if keyed:
        press_key(p, g["key"], up=False)
    elif touch:
        p.call("Input.dispatchTouchEvent", type="touchStart", touchPoints=[{"x": x, "y": y}])
    else:
        p.call("Input.dispatchMouseEvent", type="mouseMoved", x=x, y=y)
        p.call("Input.dispatchMouseEvent", type="mousePressed", x=x, y=y, button="left", clickCount=1, buttons=1)
    began, during, moved = time.time(), [], False
    samples = sorted(g.get("sample", []))
    events = sorted(g.get("events", []))
    frozen = False
    try:
        while time.time() - began < g["hold"]:
            gone = time.time() - began
            if events and gone >= events[0][0]:
                _, what = events.pop(0)
                happen(p, what, state)
                frozen = what == "freeze" or (frozen and what != "thaw")
            if g.get("move") and not moved and gone >= g["hold"] / 3:
                moved = True
                for step in range(1, 5):
                    py = y + g["move"] * step / 4
                    if touch:
                        p.call("Input.dispatchTouchEvent", type="touchMove", touchPoints=[{"x": x, "y": py}])
                    else:
                        p.call("Input.dispatchMouseEvent", type="mouseMoved", x=x, y=py, buttons=1)
            if samples and gone >= samples[0] and not frozen:
                samples.pop(0)
                during.append(p.js(FILL % json.dumps(g["selector"])))
            time.sleep(0.02)
        for _, what in events:
            happen(p, what, state)
        end_y = y + g.get("move", 0)
        # after the page was frozen the browser does not answer the end of a touch that began before it: the end is sent, and not waited for
        p.ws.settimeout(3 if state.get("froze") else 30)
        try:
            if keyed:
                press_key(p, g["key"], down=False)
            elif touch:
                p.call("Input.dispatchTouchEvent", type="touchEnd", touchPoints=[])
            else:
                p.call("Input.dispatchMouseEvent", type="mouseReleased", x=x, y=end_y, button="left", clickCount=1)
        except Exception:
            if not state.get("froze"):
                raise
        finally:
            p.ws.settimeout(30)
        p.settle(0.3)
        return {"during": during, "after": p.js(FILL % json.dumps(g["selector"]))}
    finally:
        if frozen:
            p.call("Page.setWebLifecycleState", state="active")
        if state["other"]:
            p.call("Target.activateTarget", targetId=p.target_id)
            p.call("Target.closeTarget", targetId=state["other"])


def judge_fill(fills, what):
    """A button held fills while it is held: SEEN at two moments of the hold, the share is more at the later one, not more than the whole, and it is empty again once the hand is let go. FILLS is what `gesture` answered. -> (ok, words)"""
    if fills.get("missing"):
        return False, "%s: the button is not on the page" % what
    seen = fills["during"]
    if len(seen) < 2 or any(v is None for v in seen):
        return False, "%s: the button has no fill to look at (%s)" % (what, seen)
    if not (0 < seen[0] < seen[1] <= 1.0):
        return False, "%s: the fill is %s at the two moments of the hold, expected it to grow from above nothing to the whole at the most" % (what, seen)
    if fills["after"] not in (0, 0.0):
        return False, "%s: the fill is %s once the hand is let go, expected it empty" % (what, fills["after"])
    return True, "%s: the fill grew %s and emptied" % (what, ["%.2f" % v for v in seen])


def holds(args, menu, dictionary):
    """Start, Stop and Light now with the hands a person has: a finger and a mouse. For each, a tap, a hold let go at 0.6 s, a hold of 1.3 s, and a hand that moved away (a finger that scrolls, a mouse that is dragged off): only the full hold sends, and sends once; the fill is looked at on the early let-go.
    The feed's ban is the one the page keeps at a single touch. -> (where, (ok, words)) for each."""
    from page_fixes import frames_of
    from stand_frames import named
    start, stop, light = address_of(dictionary, "thermostat", "start"), address_of(dictionary, "thermostat", "stop"), address_of(dictionary, "thermostat", "light")
    feed = address_of(dictionary, "automaton", "feed")
    plan = [
        ("burning_out_then_blow.json", lambda r: switch_of(r) == "off" and stage_of(r) == "stop", "scheme", "[data-run]", "Start", ("press", start)),
        ("lighting.json", lambda r: switch_of(r) == "on" and stage_of(r) == "working", "scheme", "[data-run]", "Stop", ("press", stop)),
        ("room_stops_with_pause.json", lambda r: switch_of(r) == "on" and stage_of(r) == "stop" and lock_of(r) > 0, "scheme", "[data-light-now]", "Light now", ("press", light)),
    ]
    out = []
    for scenario, picks, tab, selector, what, command in plan:
        text, frames = frames_of(os.path.join(HERE, "stand_scenarios", scenario))
        path = os.path.join(tempfile.mkdtemp(prefix="page_plum_"), "frames.json")
        open(path, "w").write(text)
        frame = next((f for f in frames if picks(named(dictionary, f))), None)
        if frame is None:
            out.append((what, "hold", (False, "no state of %s to press it in" % scenario)))
            continue
        for kind in ("touch", "mouse"):
            away = "scrolled" if kind == "touch" else "dragged off"
            cases = [
                ("tapped", {"hold": TAP}, []),
                ("let go at %.1f s" % EARLY, {"hold": EARLY, "sample": [0.25, 0.5]}, []),
                ("held %.1f s" % FULL, {"hold": FULL}, [command]),
                ("held %.1f s and %s" % (FULL, away), {"hold": FULL, "move": 60}, []),
            ]
            # the hold ends when whoever holds is gone: the page put away, the window out of focus, a key that is not the held one's
            if kind == "touch":
                cases += [
                    ("held %.1f s and the page put away at 0.4 s" % FULL, {"hold": FULL, "events": [(0.4, "away"), (0.9, "back")]}, []),
                    ("held %.1f s and the page frozen from 0.4 to 1.0 s" % FULL, {"hold": FULL, "events": [(0.4, "freeze"), (1.0, "thaw")]}, []),
                    ("held %.1f s and the window out of focus at 0.4 s" % FULL, {"hold": FULL, "events": [(0.4, "blur")]}, []),
                ]
            else:
                cases += [("held %.1f s and the page put away at 0.4 s" % FULL, {"hold": FULL, "events": [(0.4, "away"), (0.9, "back")]}, [])]
            for label, g, expected in cases:
                said = look(path, frame["at"], "ru", "l", tabs=(tab,), gestures=[{"selector": selector, "kind": kind, **g}])
                name = "%s %s %s" % (what, kind, label)
                out.append((name, "hold-to-press", judge_one_press(said["sent"], expected, name)))
                if g.get("sample"):
                    out.append((name, "hold-fill", judge_fill(said["fills"][0], name)))
        for key, label in (("Enter", "Enter"), (" ", "Space")):
            for text, g, expected in (
                ("held %.1f s" % FULL, {"hold": FULL}, [command]),
                ("held %.1f s and the page put away at 0.4 s" % FULL, {"hold": FULL, "events": [(0.4, "away"), (0.9, "back")]}, []),
                ("held %.1f s and the page frozen from 0.4 to 1.0 s" % FULL, {"hold": FULL, "events": [(0.4, "freeze"), (1.0, "thaw")]}, []),
                ("held %.1f s and the window out of focus at 0.4 s" % FULL, {"hold": FULL, "events": [(0.4, "blur")]}, []),
                ("held %.1f s and Escape at 0.4 s" % FULL, {"hold": FULL, "events": [(0.4, "escape")]}, []),
                ("held %.1f s and the other key pressed at 0.4 s" % FULL, {"hold": FULL, "events": [(0.4, "other_key")]}, [command]),
            ):
                said = look(path, frame["at"], "ru", "l", tabs=(tab,), gestures=[{"selector": selector, "kind": "key", "key": key, **g}])
                name = "%s key %s %s" % (what, label, text)
                out.append((name, "hold-to-press", judge_one_press(said["sent"], expected, name)))
    # the feed's ban is an emergency, one touch: the same finger and mouse send it at a tap
    text, frames = frames_of(os.path.join(HERE, "stand_scenarios", "lighting.json"))
    path = os.path.join(tempfile.mkdtemp(prefix="page_plum_"), "frames.json")
    open(path, "w").write(text)
    frame = next((f for f in frames if switch_of(named(dictionary, f)) == "on" and stage_of(named(dictionary, f)) == "working"), None)
    for kind in ("touch", "mouse"):
        said = look(path, frame["at"], "ru", "l", tabs=("scheme",), gestures=[{"selector": "[data-feed]", "kind": kind, "hold": TAP}])
        name = "Ban feed %s tapped" % kind
        out.append((name, "feed-one-touch", judge_one_press(said["sent"], [("off", feed)], name)))
    return out


# the states each verdict is asked in: the scenario, a label, and what picks the frame by the values of its rows
def switch_of(rows):
    return rows.get(("thermostat", "switch"), {}).get("word")


def stage_of(rows):
    return rows.get(("automaton", "doing"), {}).get("word")


def lock_of(rows):
    return rows.get(("thermostat", "lock"), {}).get("seconds") or 0


STATES = [
    ("lighting.json", "on, working", lambda r: switch_of(r) == "on" and stage_of(r) == "working"),
    ("burning_out_then_blow.json", "off, burning out", lambda r: switch_of(r) == "off" and stage_of(r) in ("burning_off", "burning_off_because")),
    ("burning_out_then_blow.json", "off, stopped", lambda r: switch_of(r) == "off" and stage_of(r) == "stop"),
    ("dark.json", "fault", lambda r: stage_of(r) == "fault"),
    ("room_stops_with_pause.json", "on, in the pause", lambda r: switch_of(r) == "on" and stage_of(r) == "stop" and lock_of(r) > 0),
    ("room_stops_with_pause.json", "on, stopped, no pause", lambda r: switch_of(r) == "on" and stage_of(r) == "stop" and lock_of(r) == 0),
    ("room_stops_with_pause.json", "on, no pause", lambda r: switch_of(r) == "on" and lock_of(r) == 0 and stage_of(r) == "working"),
]


def main():
    import page_fixes
    from page_stand import dictionary_of
    from stand_frames import named

    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", default=None)
    ap.add_argument("--langs", default="ru,en")
    ap.add_argument("--themes", default="l,d")
    ap.add_argument("--only", default=None, choices=["run", "pause", "scheme", "service", "press", "hold"])
    args = ap.parse_args()
    menu = merged_menu("burner")
    dictionary = dictionary_of(ROOT, "burner")
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    ok, missing, laid = True, [], {}
    # the checks of the presses ask their own states: the loop over the verdicts of the states is for the rest
    for scenario, label, picks in ([] if args.only in ("press", "hold") else STATES):
        if scenario not in laid:
            text, frames = page_fixes.frames_of(os.path.join(HERE, "stand_scenarios", scenario))
            work = tempfile.mkdtemp(prefix="page_plum_")
            path = os.path.join(work, "frames.json")
            with open(path, "w") as f:
                f.write(text)
            laid[scenario] = (path, frames)
        path, frames = laid[scenario]
        frame = next((f for f in frames if picks(named(dictionary, f))), None)
        if frame is None:
            missing.append("%s (%s)" % (label, scenario))
            continue
        rows = named(dictionary, frame)
        switch, stage, lock = switch_of(rows), stage_of(rows), lock_of(rows)
        for lang in args.langs.split(","):
            try:
                words = words_of(menu, lang)
            except KeyError:
                print("FAIL  menu-has-words     %s  the menu says nothing at scheme.run" % lang)
                ok = False
                words = {"start": "", "stop": "", "burning_off": ""}
            for theme in args.themes.split(","):
                said = look(path, frame["at"], lang, theme, tabs=("scheme", "modules", "system"), shot_dir=args.shots, name=label.replace(", ", "_").replace(" ", "_"))
                where = "%s/%s %s" % (lang, theme, label)
                verdicts = []
                if args.only in (None, "run"):
                    verdicts += [("one-run-button", judge_one_run_button(said, words)), ("run-by-switch", judge_run_by_switch(said, words, switch, stage)),
                                 ("start-refused", judge_start_refused(said, words, switch, stage))]
                if args.only in (None, "pause"):
                    verdicts.append(("light-now", judge_light_now(said, words, switch, stage)))
                if args.only in (None, "scheme"):
                    verdicts.append(("scheme-is-quiet", judge_scheme_is_quiet(said)))
                if args.only in (None, "service"):
                    verdicts.append(("service-has-hands", judge_service_has_hands(said, service_titles(menu, lang))))
                    hands = menu["scheme"]["hands"]
                    verdicts.append(("hands-button", judge_hands_button(said, hands["take"][lang], hands["take_text"][lang])))
                    if label == STATES[0][1] and lang == "ru" and theme == "l":
                        pressed = look(path, frame["at"], lang, theme, tabs=("system",), press="[data-hands]")
                        verdicts.append(("hands-press", judge_hands_button(pressed, hands["take"][lang], hands["take_text"][lang], pressed["sent"], semi_address(dictionary))))
                for name, (good, why) in verdicts:
                    print("%s  %-18s %s  %s" % ("PASS" if good else "FAIL", name, where, why))
                    ok = ok and good
    if args.only in (None, "press"):
        for line in presses(args, menu, dictionary):
            good, why = line[1]
            print("%s  %-18s %s  %s" % ("PASS" if good else "FAIL", "one-press", line[0], why))
            ok = ok and good
    if args.only in (None, "hold"):
        for name, verdict, (good, why) in holds(args, menu, dictionary):
            print("%s  %-18s %s" % ("PASS" if good else "FAIL", verdict, why))
            ok = ok and good
    if args.only in (None, "service"):
        # the hands taken: the stand holds the switch `semi` on, and no scenario plays
        from page_stand import fresh_state
        # the stand's snapshot made over this dictionary - the system card and every setting at its default - with the switch on, and said to be of this dictionary, or the stand drops what is not of it
        base = fresh_state(dictionary, json.load(open(os.path.join(HERE, "page_stand_values.json"))))
        base = {**base, "dictionary": dictionary["version"], "values": sorted([v for v in base["values"] if v[0] != semi_address(dictionary)] + [[semi_address(dictionary), True]])}
        taken = os.path.join(tempfile.mkdtemp(prefix="page_plum_"), "taken.json")
        json.dump(base, open(taken, "w"))
        for lang in args.langs.split(","):
            hands = menu["scheme"]["hands"]
            for theme in args.themes.split(","):
                said = look(None, 0, lang, theme, tabs=("system",), shot_dir=args.shots, name="hands_taken", state=taken)
                good, why = judge_hands_given(said, hands["give"][lang], hands["give_text"][lang])
                print("%s  %-18s %s  %s" % ("PASS" if good else "FAIL", "hands-given", "%s/%s hands taken" % (lang, theme), why))
                ok = ok and good
        pressed = look(None, 0, "ru", "l", tabs=("system",), press="[data-hands]", state=taken)
        good, why = judge_hands_given(pressed, menu["scheme"]["hands"]["give"]["ru"], menu["scheme"]["hands"]["give_text"]["ru"], pressed["sent"], semi_address(dictionary))
        print("%s  %-18s %s  %s" % ("PASS" if good else "FAIL", "hands-give-press", "ru/l hands taken", why))
        ok = ok and good
    for m in missing:
        print("FAIL  state-not-found    %s" % m)
        ok = False
    raise SystemExit(0 if ok else 1)


def address_of(dictionary, card, name):
    return next(at for at, thing in enumerate(dictionary["things"]) if thing["card"] == card and thing["name"] == name and not thing.get("part_of"))


# the pace of the clicks, in seconds: AT ONCE is a double click; WITHIN THE MOMENT is a second click before the board has said the first took - the stand's stream tells a changed row in about two seconds, the board's in at most its
# period of five - or within the moment the lock keeps after it (COOL_MS 600); AFTER is a second press of a person who has read the new word: later than the stand's two seconds and the moment
AT_ONCE, WITHIN, AFTER = 0.0, 0.25, 3.8


# the buttons that are pressed by holding them (page.js, HoldButton): the presses of a person on them are holds
HELD = ("[data-run]", "[data-light-now]")


def presses(args, menu, dictionary):
    """The clicks of a person on each button that sends an absolute command by the state it sees, on a stand that does what the board does with them: (where, (ok, words)) for each. Four buttons, and for the burner's
    one both directions; for each the double click, the click within the moment after the answer, and the second press after it."""
    from stand_frames import named
    lang, theme = "ru", "l"
    start, stop, light = address_of(dictionary, "thermostat", "start"), address_of(dictionary, "thermostat", "stop"), address_of(dictionary, "thermostat", "light")
    semi, feed = address_of(dictionary, "automaton", "semi"), address_of(dictionary, "automaton", "feed")
    plan = [
        # (scenario, picks the state, tab, selector, what, then the commands expected for each pace)
        ("burning_out_then_blow.json", lambda r: switch_of(r) == "off" and stage_of(r) == "stop", "scheme", "[data-run]", "Start", {AT_ONCE: [("press", start)], WITHIN: [("press", start)], AFTER: [("press", start), ("press", stop)]}),
        ("lighting.json", lambda r: switch_of(r) == "on" and stage_of(r) == "working", "scheme", "[data-run]", "Stop", {AT_ONCE: [("press", stop)], WITHIN: [("press", stop)], AFTER: [("press", stop), ("press", start)]}),
        ("room_stops_with_pause.json", lambda r: switch_of(r) == "on" and stage_of(r) == "stop" and lock_of(r) > 0, "scheme", "[data-light-now]", "Light now", {AT_ONCE: [("press", light)], WITHIN: [("press", light)]}),
        ("lighting.json", lambda r: switch_of(r) == "on" and stage_of(r) == "working", "system", "[data-hands]", "Take by hand", {AT_ONCE: [("on", semi)], WITHIN: [("on", semi)], AFTER: [("on", semi), ("off", semi)]}),
        ("lighting.json", lambda r: switch_of(r) == "on" and stage_of(r) == "working", "scheme", "[data-feed]", "Ban feed", {AT_ONCE: [("off", feed)], WITHIN: [("off", feed)], AFTER: [("off", feed), ("on", feed)]}),
    ]
    # a relay of the Service, the engine's own button: the pump, whose first press is the opposite of the state the frame holds
    pump = address_of(dictionary, "relays", "pump")
    plan.append(("lighting.json", lambda r: switch_of(r) == "on" and stage_of(r) == "working", "system", '[data-thing="relays/pump"]', "Relay",
                 lambda r: ("off", "on") if r.get(("relays", "pump"), {}).get("on") else ("on", "off")))
    out = []
    from page_fixes import frames_of
    for scenario, picks, tab, selector, what, paces in plan:
        text, frames = frames_of(os.path.join(HERE, "stand_scenarios", scenario))
        path = os.path.join(tempfile.mkdtemp(prefix="page_plum_"), "frames.json")
        open(path, "w").write(text)
        frame = next((f for f in frames if picks(named(dictionary, f))), None)
        if frame is None:
            out.append((what, (False, "no state of %s to press it in" % scenario)))
            continue
        if callable(paces):
            first, second = paces(named(dictionary, frame))
            paces = {AT_ONCE: [(first, pump)], WITHIN: [(first, pump)], AFTER: [(first, pump), (second, pump)]}
        for pace, expected in paces.items():
            if selector in HELD:
                # a button pressed by holding it: a person's two presses are two holds of a finger, the second begun PACE seconds after the first was let go
                said = look(path, frame["at"], lang, theme, tabs=(tab,), gestures=[{"selector": selector, "kind": "touch", "hold": FULL, "wait": pace}, {"selector": selector, "kind": "touch", "hold": FULL}])
            else:
                said = look(path, frame["at"], lang, theme, tabs=(tab,), clicks=[(selector, pace), (selector, 0)])
            out.append(("%s %s" % (what, {AT_ONCE: "twice at once", WITHIN: "twice within the moment", AFTER: "twice, the second after it"}[pace] + (", held" if selector in HELD else "")), judge_one_press(said["sent"], expected, what)))
    # a banned feed is banned to the hand: the Service's auger, pressed while the feed is banned, is refused by the board, and the page says the board's words and does not hold the button
    auger, banned = address_of(dictionary, "relays", "auger"), address_of(dictionary, "automaton", "feed")
    text, frames = frames_of(os.path.join(HERE, "stand_scenarios", "feed_banned.json"))
    path = os.path.join(tempfile.mkdtemp(prefix="page_plum_"), "frames.json")
    open(path, "w").write(text)
    frame = next((f for f in frames if named(dictionary, f).get(("automaton", "feed"), {}).get("on") is False), None)
    if frame is None:
        out.append(("Manual auger under a ban", (False, "no state of feed_banned.json with the feed banned")))
    else:
        shots = os.path.join(args.shots) if args.shots else None
        said = look(path, frame["at"], lang, theme, tabs=("system",), shot_dir=shots, name="auger_under_ban", clicks=[('[data-thing="relays/auger"]', 1.0), ('[data-thing="relays/auger"]', 0)])
        out.append(("Manual auger under a ban", judge_refused_hand(said, auger, ("banned", "Allow feed"))))
    return out


def judge_refused_hand(said, address, words):
    """The auger pressed by hand under a ban: the board was asked and refused, the page says the board's words - WORDS, each of them - in a note, and the button is not held (a second press 600 ms after the refusal goes to the board again). -> (ok, words)"""
    sent = [way_address for way_address in sends_of(said["sent"]) if way_address[1] == address]
    if not sent or sent[0] != ("on", address):
        return False, "the press was not sent to the board as `on %d`: %s" % (address, sent)
    note = said["after"].get("note", "")
    missing = [w for w in words if w not in note]
    if missing:
        return False, "the page says %r, which has not %s" % (note, missing)
    if len(sent) != 2:
        return False, "the second press after the refusal sent %d commands, expected the board to be asked again (2 in all): %s" % (len(sent), sent)
    return True, "asked, refused with the board's words, asked again after the moment: %s" % sent


def service_titles(menu, lang):
    """The titles of the sections the menu groups under the Service on the system tab."""
    return [s["title"][lang] for s in menu["tree"]["system"]["sections"] if s.get("group") == "service"]


if __name__ == "__main__":
    main()
