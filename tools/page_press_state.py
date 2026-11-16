"""A press says it was taken: every button that commands the burner shows "accepted" in the frame its press ends, and the three ends of a press - the board answers, refuses, is silent - are shown.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_press_state.py [--board HOST | --device NAME] [--only NAME] [--theme l|d] [--modes lag,refuse,silent]

Without --board the page is the stand's (tools/page_stand.py, a frame where the burner is switched on and stands in a pause, so that Stop and "light now" are both there). With --board HOST the page is the board's own
and the buttons are the ones the board's state shows. The page's `fetch` is wrapped in the browser to make the board's three ends, and the browser reports what a person would see, frame by frame:

  accepted-at       the first frame the button says `accepted` (data-phase) after the press ended - the begin of the POST -, in ms; at most LIMIT_MS (100); the button is as wide as before and the page no wider than the phone (375 px)
  one-press         a second press while the first waits sends nothing: one POST in the whole window
  answered          the board answered after a lag of 1 s: the button leaves `accepted` once its stream has said it (at most 5 s later) and is free after the moment that follows
  refused           the board refuses: no button stays `accepted`, the button is free within 0.5 s and the page says the board's reason
  silent            the board says nothing for 9.5 s: at 8.5 s the button is free and the page says the phrase of the silence, with its number and what to do
  again-false /     the press A is not answered for 11 s and is let go by the limit; the press B goes to the board; then A's late word comes (a refusal / the board's answer): the press B is still
  again-true        held and `accepted`, a third press goes nowhere, and only B's own answer (at 15 s) lets it go. `again-true` is asked of the buttons the board's answer settles (the engine's rows);
                    the burner's own are settled by the board's rows, which A's late effect rightly changes

Exits 0 when all hold, 1 when one does not. Stops the stand and Chrome whatever happens.
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

STAND_PORT = 8806
CHROME_PORT = 9422
WIDTH = 375
LIMIT_MS = 100
TABS = {"scheme": 0, "system": 2, "main": 0}

# the buttons: (name, tab, selector, held). Held ones are pressed by a hold of a second and their press ends where the hold does.
BUTTONS = [
    ("stop", "scheme", "[data-run]", True),
    ("light-now", "scheme", "[data-light-now]", True),
    ("feed", "scheme", "[data-feed]", False),
    ("hands", "system", "[data-hands]", False),
    ("relay", "system", '[data-thing="relays/pump"]', False),
    ("slider-zero", "system", "[data-zero]", False),
]

# what the browser is asked of one press. MODE makes the board's end: `lag` forwards the POST after 1 s, `refuse` answers an error at once, `silent` forwards it after 9.5 s.
# It reports, from the end of the press (the begin of the POST): when the button first said `accepted`, its phase and disabled over time, and how many POSTs went.
CASE = r"""
(selector, held, mode) => new Promise((resolve) => {
  const find = () => document.querySelector(selector);
  const real = window.fetch.__real || window.fetch;
  const posts = [];
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  let t0 = null;
  const seenPosts = [];
  window.fetch = Object.assign(function (url, options) {
    if (options && options.method === "POST") seenPosts.push(String(url));
    if (options && options.method === "POST" && /^\/?(on|off|press|toggle|hold)\//.test(String(url).replace(/^https?:\/\/[^/]+/, ""))) {
      posts.push(performance.now());
      if (t0 === null) t0 = performance.now();
      if (mode === "refuse") return Promise.resolve(new Response(JSON.stringify({ error: "refused_by_probe" }), { status: 200 }));
      if (mode === "real") return real.apply(window, arguments).then((r) => { r.clone().text().then((t) => { out.answer = t.slice(0, 200); }); return r; });
      return wait(mode === "silent" ? 9500 : 1000).then(() => real.apply(window, arguments));
    }
    return real.apply(window, arguments);
  }, { __real: real });
  const phase = () => { const e = find(); return e ? (e.getAttribute("data-phase") || "") : "GONE"; };
  // a button the board's own state holds (Start while the fire burns out) is disabled with its reason standing under it: that is the board's word and not a lock of the press
  const held_by_board = () => Boolean(document.querySelector("[data-run-cannot]"));
  const free = () => { const e = find(); return Boolean(e) && (!e.disabled || held_by_board()); };
  const note = () => Array.from(document.querySelectorAll("[data-note]")).map((n) => (n.innerText || "").trim()).join(" ");
  const out = { answer: null, trace: [], urls: seenPosts, leftAt: null, freeAt: null, acceptedAt: null, text: null, at300: null, at500: null, at8500: null, afterAnswer: null, posts: 0, note: "" };
  const b = find();
  const r = b.getBoundingClientRect();
  out.w0 = r.width;
  const seen = performance.now();
  if (held) b.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 1, button: 0, clientX: r.left + 5, clientY: r.top + 5, bubbles: true }));
  else b.click();
  let second = false;
  const frame = () => {
    const now = performance.now();
    if (t0 !== null) {
      const since = now - t0;
      const here = phase() + "/" + (free() ? "free" : "held");
      if (!out.last || out.last !== here) { out.trace.push([Math.round(since), here]); out.last = here; }
      if (out.acceptedAt === null && phase() === "accepted") { out.acceptedAt = Math.round(since); out.text = (find().innerText || "").trim(); const svg = find().parentElement.querySelector("svg"); out.drawn = svg ? svg.textContent : ""; out.w1 = find().getBoundingClientRect().width; out.wide = document.documentElement.scrollWidth; }
      if (out.acceptedAt !== null && out.leftAt === null && phase() !== "accepted") out.leftAt = Math.round(since);
      if (out.leftAt !== null && out.freeAt === null && phase() === "" && free()) out.freeAt = Math.round(since);
      if (mode === "real" && (out.freeAt !== null && since > out.freeAt + 200 || since > 9500)) { out.posts = posts.length; window.fetch = real; resolve(out); return; }
      if (!second && since > 300) {
        second = true;
        out.at300 = { phase: phase(), free: free() };
        const e = find();
        if (e && !e.disabled) { if (held) e.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 2, button: 0, clientX: 5, clientY: 5, bubbles: true })); else e.click(); }
        else if (e && held) e.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 2, button: 0, clientX: 5, clientY: 5, bubbles: true }));
      }
      if (out.at500 === null && since > 500) out.at500 = { phase: phase(), free: free() };
      if (mode === "lag" && out.afterAnswer === null && since > 7500) out.afterAnswer = { phase: phase(), free: free() };
      if (mode === "refuse" && since > 1800) { out.posts = posts.length; out.note = note(); window.fetch = real; resolve(out); return; }
      if (mode === "lag" && since > 7600) { out.posts = posts.length; window.fetch = real; resolve(out); return; }
      if (mode === "silent") {
        if (out.at8500 === null && since > 8500) { out.at8500 = { phase: phase(), free: free(), note: note() }; }
        if (since > 8700) { out.posts = posts.length; window.fetch = real; resolve(out); return; }
      }
    } else if (now - seen > 4000) { out.posts = 0; window.fetch = real; resolve(out); return; }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
})
"""


A_MS, B_MS = 11000, 6000
ROW_BUTTONS = {"relay", "slider-zero", "relay1", "relay2"}

# a device whose page is the engine's and nothing of the burner's (`--device`): the relays of the card on its main screen are the engine's own row buttons
DEVICE_BUTTONS = [
    ("relay1", "main", '[data-thing="relays/relay1"]', False),
    ("relay2", "main", '[data-thing="relays/relay2"]', False),
]

# the press A is held in the page for A_MS (the board is silent), is let go by the limit, the press B goes and is held for B_MS; then A's word comes (forwarded to the board, or a refusal). What the browser reports: the button
# at the limit, at A's word and after B's, and how many commands went, a third press included.
CASE_AGAIN = r"""
(selector, held, answerA, A_MS, B_MS) => new Promise((resolve) => {
  const find = () => document.querySelector(selector);
  const real = window.fetch.__real || window.fetch;
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const posts = [];
  let t0 = null;
  window.fetch = Object.assign(function (url, options) {
    if (options && options.method === "POST" && /^\/?(on|off|press|toggle|hold)\//.test(String(url).replace(/^https?:\/\/[^/]+/, ""))) {
      posts.push(performance.now());
      if (t0 === null) t0 = performance.now();
      if (posts.length === 1) return wait(A_MS).then(() => (answerA === "refuse" ? new Response(JSON.stringify({ error: "late_refusal_of_A" }), { status: 200 }) : real.apply(window, arguments)));
      if (posts.length === 2) return wait(B_MS).then(() => real.apply(window, arguments));
      return real.apply(window, arguments);
    }
    return real.apply(window, arguments);
  }, { __real: real });
  const phase = () => { const e = find(); return e ? (e.getAttribute("data-phase") || "") : "GONE"; };
  const free = () => { const e = find(); return Boolean(e) && !e.disabled; };
  const note = () => Array.from(document.querySelectorAll("[data-note]")).map((n) => (n.innerText || "").trim()).join(" ");
  const press = (id) => {
    const e = find();
    if (!e) return;
    const r = e.getBoundingClientRect();
    if (held) e.dispatchEvent(new PointerEvent("pointerdown", { pointerId: id, button: 0, clientX: r.left + 5, clientY: r.top + 5, bubbles: true })); else e.click();
  };
  const out = { atLimit: null, pressedB: null, lateA: null, afterThird: null, final: null, posts: 0 };
  press(1);
  let stage = "A";
  const frame = () => {
    const now = performance.now();
    if (t0 === null) { if (now > 4000 + (window.__s || 0)) { window.fetch = real; resolve(out); return; } requestAnimationFrame(frame); return; }
    const since = now - t0;
    // the lock of A is let go when the button has no phase any more (a button of the engine's rows is never disabled, so `free` says nothing of it)
    if (stage === "A" && since > 7000 && phase() === "" && free()) { out.atLimit = { at: Math.round(since), phase: phase(), free: true, note: note() }; press(2); out.pressedB = Math.round(since); stage = "B"; out.bAt = null; }
    if (out.bAt === null && posts.length >= 2) out.bAt = Math.round(posts[1] - t0);
    if (stage === "B" && since > A_MS + 700) { out.lateA = { phase: phase(), free: free(), posts: posts.length, note: note() }; press(3); stage = "C"; }
    if (stage === "C" && since > A_MS + 2800) { out.afterThird = { phase: phase(), free: free(), posts: posts.length }; stage = "D"; }
    if (stage === "D" && out.bAt !== null && since > out.bAt + B_MS + 1800) { out.final = { phase: phase(), free: free(), posts: posts.length }; out.posts = posts.length; window.fetch = real; resolve(out); return; }
    if (since > A_MS + B_MS + 9000) { out.posts = posts.length; window.fetch = real; resolve(out); return; }
    requestAnimationFrame(frame);
  };
  window.__s = performance.now();
  requestAnimationFrame(frame);
})
"""


def judge_again(name, mode, got):
    """(passed, words): B stays held and `accepted` through A's late word, a third press goes nowhere, and B's own answer lets it go."""
    if mode == "again-true" and name not in ROW_BUTTONS:
        return True, "not asked: the lock of this button is settled by the board's rows, which A's late effect rightly changes"
    if not got.get("atLimit"):
        return False, "the limit did not let A go: %s" % got
    late, third, last = got.get("lateA"), got.get("afterThird"), got.get("final")
    # a button of the engine's rows is never disabled while its press waits: the phase says it is held. The burner's are disabled as well.
    def held(r):
        return r["phase"] == "accepted" and (not r["free"] or name in ROW_BUTTONS)

    ok = bool(late and third and last)
    ok = ok and got["pressedB"] is not None and late["posts"] == 2 and held(late)
    ok = ok and third["posts"] == 2 and held(third)
    # the stand does not model Light: its board never leaves the stop, so B ends by its limit and not by an answer
    unsettled = name == "light-now" and not ARGS.board
    ok = ok and (unsettled or ((last["free"] and last["phase"] == "") or last["phase"] == "answered"))
    return bool(ok), "A let go at %s ms (free, note: %s); B pressed at %s ms; at A's word %s; after a third press %s; after B's answer %s" % (
        got["atLimit"]["at"], bool(got["atLimit"]["note"]), got["pressedB"], late, third, last)


def judge(name, mode, got, phrase):
    """(passed, words) of one case from what the browser reported."""
    if mode.startswith("again"):
        return judge_again(name, mode, got)
    if got["posts"] == 0:
        return False, "no command was sent"
    if mode == "real":
        # the board itself answers: what is asked is the frame the button says `accepted` in; when it leaves it and when it is free are the measures
        ok = got["acceptedAt"] is not None and got["acceptedAt"] <= LIMIT_MS and got["posts"] == 1
        return bool(ok), "accepted at %s ms (\"%s\"), width %.0f -> %.0f, word at %s ms, free at %s ms, %d POST" % (got["acceptedAt"], got["text"], got.get("w0", 0), got.get("w1", 0), got["leftAt"], got["freeAt"], got["posts"])
    if mode == "lag":
        ok = got["acceptedAt"] is not None and got["acceptedAt"] <= LIMIT_MS
        # the feed's button is laid over the drawing and says nothing itself: its word is the drawing's
        if name == "feed":
            ok = ok and "Принято" in got.get("drawn", "")
        # the button does not jump when it speaks, and the page is not wider than the phone
        ok = ok and abs(got.get("w1", 0) - got.get("w0", 0)) <= 1 and got.get("wide", 0) <= WIDTH
        # the stand does not model Light: its board never leaves the stop, so the press is not settled there and the lock ends by its limit (the silent case)
        unsettled = name == "light-now" and not ARGS.board
        ok = ok and got["posts"] == 1 and (unsettled or (got["afterAnswer"] and got["afterAnswer"]["phase"] == "" and got["afterAnswer"]["free"]))
        return bool(ok), "accepted at %s ms (\"%s\"), width %.0f -> %.0f, page %s, %d POST, after the answer %s" % (got["acceptedAt"], got["text"], got.get("w0", 0), got.get("w1", 0), got.get("wide"), got["posts"], got["afterAnswer"])
    if mode == "refuse":
        # a refusal that comes in the same breath as the press may be gone before any frame shows `accepted`: what is asked is that nothing stays held and that the page says the board's reason
        ok = got["at500"] and got["at500"]["phase"] == "" and got["at500"]["free"] and "refused_by_probe" in got["note"]
        return bool(ok), "accepted at %s ms, at 500 ms %s, the page says: %s" % (got["acceptedAt"], got["at500"], got["note"])
    last = got["at8500"]
    ok = bool(last) and last["phase"] == "" and last["free"] and bool(phrase) and phrase in last["note"]
    return ok, "at 8.5 s %s" % (last,)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--board", default=None)
    ap.add_argument("--device", default=None, help="the stand of this device of devices/ (the engine's own page, no burner on it): its relays are the buttons asked")
    ap.add_argument("--only", default=None)
    ap.add_argument("--shots", default=None)
    ap.add_argument("--modes", default="lag,refuse,silent")
    ap.add_argument("--theme", default="l", choices=["l", "d"])
    args = ap.parse_args()
    global ARGS
    ARGS = args
    from chrome_page import CHROME, Page
    import page_steady

    menu = merged_menu(args.device or "burner")
    phrase = menu["ui"]["pressSilent"]["ru"] if "pressSilent" in menu["ui"] else None
    work = tempfile.mkdtemp(prefix="page_press_state_")
    stand = None
    if args.board:
        url = "http://%s/" % args.board
        start = None
    elif args.device:
        url = "http://127.0.0.1:%d/" % STAND_PORT

        def start():
            one = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--device", args.device, "--port", str(STAND_PORT), "--log", os.path.join(work, "stand.log")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for _ in range(300):
                try:
                    urllib.request.urlopen(url + "state", timeout=1)
                    return one
                except OSError:
                    time.sleep(0.2)
            raise SystemExit("page_press_state: the stand of %s did not come up" % args.device)
    else:
        from page_fixes import frames_of
        from page_stand import dictionary_of
        from stand_frames import named
        text, frames = frames_of(os.path.join(HERE, "stand_scenarios", "room_stops_with_pause.json"))
        dictionary = dictionary_of(ROOT, "burner")

        def pick(rows):
            return rows.get(("thermostat", "switch"), {}).get("word") == "on" and rows.get(("automaton", "doing"), {}).get("word") == "stop" and (rows.get(("thermostat", "lock"), {}).get("seconds") or 0) > 0

        frame = next((f for f in frames if pick(named(dictionary, f))), None)
        if frame is None:
            raise SystemExit("page_press_state: no frame of the scenario with the burner on, stopped, in a pause")
        path = os.path.join(work, "frames.json")
        open(path, "w").write(text)
        url = "http://127.0.0.1:%d/" % STAND_PORT

        def start():
            # a stand of its own for each case: a command the stand took (Stop) would change the state the next case is asked in
            one = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT), "--scenario", path, "--at", str(frame["at"]), "--log", os.path.join(work, "stand.log")],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for _ in range(100):
                try:
                    urllib.request.urlopen(url + "state", timeout=1)
                    return one
                except OSError:
                    time.sleep(0.2)
            raise SystemExit("page_press_state: the stand did not come up")
    chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % CHROME_PORT, "--user-data-dir=%s/profile" % work, "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    verdicts = []
    try:
        targets = None
        for _ in range(100):
            try:
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % CHROME_PORT))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            raise SystemExit("page_press_state: Chrome did not come up")
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=900, deviceScaleFactor=2, mobile=True)
        for name, tab, selector, held in (DEVICE_BUTTONS if args.device else BUTTONS):
            if args.only and args.only != name:
                continue
            for mode in args.modes.split(","):
                if start:
                    if stand:
                        stand.terminate()
                        stand.wait(10)
                    stand = start()
                p.call("Page.navigate", url=url)
                p.settle(1.0)
                p.js("localStorage.setItem('t', %s); localStorage.setItem('lang', 'ru')" % json.dumps(args.theme))
                p.call("Page.navigate", url=url)
                p.settle(2.0)
                page_steady.wait_drawn(p)
                ready = selector if args.device else "[data-feed]:not([disabled])"
                for _ in range(40):
                    # the stream has told the rows: the feed's button is not idle any more (a device with no burner: the button asked is there)
                    if json.loads(p.js("JSON.stringify(Boolean(document.querySelector(%s)))" % json.dumps(ready))):
                        break
                    p.settle(0.25)
                p.js("document.querySelectorAll('main nav button')[%d].click(); 1" % TABS[tab])
                p.settle(0.8)
                if os.environ.get("DEBUG"):
                    print("tab", tab, p.js("JSON.stringify([...document.querySelectorAll('main button')].map(b=>[b.textContent.trim().slice(0,20),[...b.attributes].map(a=>a.name).join(',')]))"))
                if not json.loads(p.js("JSON.stringify(Boolean(document.querySelector(%s)))" % json.dumps(selector))):
                    verdicts.append(("%s/%s" % (name, mode), False, "the page has no such button (%s) in this state" % selector))
                    continue
                if mode.startswith("again"):
                    if mode == "again-true" and name not in ROW_BUTTONS:
                        verdicts.append(("%s/%s" % (name, mode),) + judge_again(name, mode, {}))
                        continue
                    got = json.loads(p.js("(%s)(%s, %s, %s, %d, %d).then((x) => JSON.stringify(x))" % (CASE_AGAIN, json.dumps(selector), "true" if held else "false", json.dumps("refuse" if mode == "again-false" else "forward"), A_MS, B_MS)))
                else:
                    got = json.loads(p.js("(%s)(%s, %s, %s).then((x) => JSON.stringify(x))" % (CASE, json.dumps(selector), "true" if held else "false", json.dumps(mode))))
                if os.environ.get("DEBUG"):
                    print("got", json.dumps(got, ensure_ascii=False))
                    if stand:
                        print("stand", open(os.path.join(work, "stand.log")).read()[-400:])
                        print("switch", [v for a, v in json.load(urllib.request.urlopen(url + "state"))["values"] if a == 149])
                verdicts.append(("%s/%s" % (name, mode),) + judge(name, mode, got, phrase))
                p.settle(2.0)
    finally:
        chrome.terminate()
        if stand:
            stand.terminate()
            stand.wait(10)
        chrome.wait(10)
    ok = True
    for check, passed, words in verdicts:
        print("%-22s %s  %s" % (check, "PASS" if passed else "FAIL", words))
        ok = ok and passed
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
