"""The target of the room can be changed in any minute (9.35): the page on the stand, a hand on the target where it was shut before and on the thermostat's page.

    uv run --no-project --with websocket-client --with pyyaml python tools/page_target_reach.py [--langs ru,en] [--themes l,d]

Each moment is picked BY VALUE from the frames of a scenario, and the page is asked 375 px wide, each verdict printing PASS or FAIL:

  in-a-cycle          a Light's cycle of the room goes: the line of the reason is a button, a press opens the panel, and a press of + is heard by the board once, as the target and a tenth over it
  demand-unread       the board has not read the demand (the line of the reason says nothing): the target is still a button, it opens the panel, and a press of + is heard once
  on-the-thermostat   the thermostat's page (the demand's card) holds the panel open with no list of settings and no Save beside it, and five quick presses of + are heard once, as the target and five tenths over it

The target a press starts from is the setting's own in the scenario; the writes are the stand's (`/sent`). The verdicts are `judge_*`, pure functions of what the browser answered, held by tools/test_page_target_reach.py.
Exits 1 when a check failed.
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

import page_target as t  # noqa: E402

STAND_PORT = 8805
CHROME_PORT = 9421

PEEK_REACH = """(function(){
  const q = (s) => document.querySelector(s);
  const toggle = q('[data-target-toggle]');
  const panel = q('[data-target-panel]');
  const value = q('[data-target-value]');
  const saves = Array.from(document.querySelectorAll('button')).filter((b) => /^(Save|\\u0421\\u043e\\u0445\\u0440\\u0430\\u043d\\u0438\\u0442\\u044c)/.test(b.textContent.trim())).length;
  const near = panel;
  return JSON.stringify({
    toggle: !!toggle, panel: !!panel, value: value && value.firstChild ? value.firstChild.textContent.trim() : null,
    up: !!q('[data-target-up]'), down: !!q('[data-target-down]'),
    saves: near ? Array.from(near.querySelectorAll('button')).filter((b) => /^(Save|\\u0421\\u043e\\u0445\\u0440\\u0430\\u043d\\u0438\\u0442\\u044c)/.test(b.textContent.trim())).length : saves,
    inputs: near ? near.querySelectorAll('input').length : 0,
    scrollWidth: document.documentElement.scrollWidth,
  });
})()"""


# ---- the verdicts: pure functions of what the browser answered ----

def judge_opens_by_a_press(before, after):
    """The target is a button where it was asked about, and a press of it draws the panel with its buttons. -> (ok, words)"""
    out = []
    if not before["toggle"]:
        out.append("no button of the target stands")
    if not after["panel"]:
        out.append("a press draws no panel")
    elif not (after["up"] and after["down"]):
        out.append("the panel has no button each way")
    return (not out), "; ".join(out) if out else "the target is a button, and its panel stands after a press"


def judge_open_with_no_form(said):
    """The thermostat's page holds the panel open, with no field and no Save beside it, and nothing wider than the phone. -> (ok, words)"""
    out = []
    if not said["panel"]:
        out.append("the page has no panel of the target")
    if said["toggle"]:
        out.append("the panel waits for a press where the page has no line to press")
    if said["saves"]:
        out.append("%d Save button(s) beside the panel" % said["saves"])
    if said["inputs"]:
        out.append("%d field(s) beside the panel" % said["inputs"])
    if said["scrollWidth"] > t.WIDTH + 1:
        out.append("the page is %d px wide" % said["scrollWidth"])
    return (not out), "; ".join(out) if out else "the panel stands open, with no form"


# ---- the stand ----

def address_of(dictionary, card, key):
    """The address of the row KEY of CARD in the stand's values: the order of the dictionary's things."""
    from page_sources import merged_texts

    rows = merged_texts("burner").get("rows", {})
    for at, thing in enumerate(dictionary["things"]):
        if thing.get("part_of") or thing["card"] != card:
            continue
        label = rows.get("%s/%s" % (card, thing["name"]), {}).get("label", thing["name"])
        if "{" in label:
            label = thing["name"]
        if label == key:
            return at
    raise SystemExit("page_target_reach: the dictionary has no row %s/%s" % (card, key))


def unread(text, at):
    """The frames TEXT with the row at AT never read."""
    whole = json.loads(text)
    for frame in whole["frames"]:
        frame["values"] = [[a, v] for a, v in frame["values"] if a != at] + [[at, None]]
    return json.dumps(whole)


class Run:
    """A stand on the frames FRAMES at second AT, settings of SCENARIO, and a headless Chrome that has drawn its page."""

    def __init__(self, frames, at, scenario_file, lang, theme):
        self.work = tempfile.mkdtemp(prefix="page_target_reach_")
        self.frames_file = os.path.join(self.work, "frames.json")
        with open(self.frames_file, "w") as f:
            f.write(frames)
        self.url = "http://127.0.0.1:%d" % STAND_PORT
        self.stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT), "--scenario", self.frames_file, "--at", str(at), "--settings-of", scenario_file],
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        profile = tempfile.mkdtemp(prefix="page_target_reach_chrome_")
        from chrome_page import CHROME, Page
        import page_steady

        self.chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % CHROME_PORT, "--user-data-dir=%s" % profile, "--no-first-run", "about:blank"],
                                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        targets = None
        for _ in range(75):
            try:
                urllib.request.urlopen(self.url + "/state", timeout=1)
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % CHROME_PORT))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            self.close()
            raise SystemExit("page_target_reach: the stand or Chrome did not come up")
        self.p = Page(next(x["webSocketDebuggerUrl"] for x in targets if x["type"] == "page"))
        self.p.call("Page.enable")
        self.p.call("Emulation.setDeviceMetricsOverride", width=t.WIDTH, height=900, deviceScaleFactor=2, mobile=True)
        self.p.call("Page.navigate", url=self.url + "/")
        self.p.settle(1.0)
        self.p.js("localStorage.setItem('t', %s); localStorage.setItem('lang', %s)" % (json.dumps(theme), json.dumps(lang)))
        self.p.call("Page.navigate", url=self.url + "/")
        self.p.settle(2.0)
        if not page_steady.wait_drawn(self.p):
            self.close()
            raise SystemExit("page_target_reach: the page did not draw - nothing is said of it")

    def peek(self):
        return json.loads(self.p.js(PEEK_REACH))

    def close(self):
        for proc in (getattr(self, "chrome", None), getattr(self, "stand", None)):
            if proc:
                proc.terminate()
                proc.wait(10)
        if os.path.exists(self.frames_file):
            os.remove(self.frames_file)
        if os.path.isdir(self.work) and not os.listdir(self.work):
            os.rmdir(self.work)


def open_thermostat_page(p, words):
    """The way a person goes: the Modules button, then the module whose title is WORDS."""
    p.js("(function(){const b=Array.from(document.querySelectorAll('button')).find(x=>/\\u041c\\u043e\\u0434\\u0443\\u043b\\u0438|Modules/.test(x.textContent)); if(b) b.click(); return 1})()")
    p.settle(0.6)
    p.js("(function(){const w=%s; const b=Array.from(document.querySelectorAll('button')).find(x=>w.some(s=>x.textContent.trim()===s)); if(b) b.click(); return 1})()" % json.dumps(words))
    p.settle(0.6)


def main():
    import page_fixes
    from page_sources import merged_menu
    from page_stand import dictionary_of
    from stand_frames import named

    ap = argparse.ArgumentParser()
    ap.add_argument("--langs", default="ru,en")
    ap.add_argument("--themes", default="l,d")
    args = ap.parse_args()
    menu = merged_menu("burner")
    dictionary = dictionary_of(ROOT, "burner")
    ok = True

    def verdict(where, check, result):
        nonlocal ok
        passed, words_ = result
        print("%-10s %-18s %s  %s" % (where, check, "PASS" if passed else "FAIL", words_))
        ok = ok and passed

    def pick(scenario_name, wanted):
        scenario_file = os.path.join(HERE, "stand_scenarios", scenario_name)
        text, frames = page_fixes.frames_of(scenario_file)
        frame = next((f for f in frames if wanted(named(dictionary, f))), None)
        if frame is None:
            raise SystemExit("page_target_reach: %s never comes to the moment asked for" % scenario_name)
        return scenario_file, text, frame

    def comfort(n):
        return n.get(("demand", "profile"), {}).get("word") in ("comfort", "eco")

    def in_room_cycle(n):
        return comfort(n) and n.get(("thermostat", "manual"), {}).get("word") == "room" and "number" in n.get(("thermostat", "manual_c"), {})

    def cold(n):
        return comfort(n) and n.get(("demand", "asked"), {}).get("word") == "cold" and n.get(("thermostat", "switch"), {}).get("word") == "on"

    cycle_file, cycle_text, cycle_frame = pick("light_to_target.json", in_room_cycle)
    room_file, room_text, room_frame = pick("house_by_the_room.json", cold)
    asked_at = address_of(dictionary, "demand", "asked")
    moments = [
        ("in-a-cycle", cycle_file, cycle_text, cycle_frame, "toggle"),
        ("demand-unread", room_file, unread(room_text, asked_at), room_frame, "toggle"),
        ("on-the-thermostat", room_file, room_text, room_frame, "page"),
    ]
    for lang in args.langs.split(","):
        for theme in args.themes.split(","):
            where = "%s/%s" % (lang, theme)
            for check, scenario_file, text, frame, how in moments:
                settings = json.load(open(scenario_file))["settings"]
                start = float(settings["auto_room_c"])
                run = Run(text, frame["at"], scenario_file, lang, theme)
                try:
                    p = run.p
                    if how == "toggle":
                        before = run.peek()
                        t.click(p, "[data-target-toggle]")
                        p.settle(0.4)
                        after = run.peek()
                        opens = judge_opens_by_a_press(before, after)
                        t.click(p, "[data-target-up]")
                        p.settle(t.WAIT + t.SLACK)
                        heard = t.judge_one_write(t.writes(run.url), [("auto_room_c", t.sent(start + t.STEP))])
                        verdict(where, check, (opens[0] and heard[0], opens[1] + "; " + heard[1]))
                    else:
                        open_thermostat_page(p, [next(x for x in menu["tree"]["modules"]["pages"] if x["id"] == "demand")["title"][lang]])
                        said = run.peek()
                        form = judge_open_with_no_form(said)
                        for _ in range(5):
                            t.click(p, "[data-target-up]")
                            p.settle(0.15)
                        p.settle(t.WAIT + t.SLACK)
                        heard = t.judge_one_write(t.writes(run.url), [("auto_room_c", t.sent(start + 5 * t.STEP))])
                        verdict(where, check, (form[0] and heard[0], form[1] + "; " + heard[1]))
                finally:
                    run.close()
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
