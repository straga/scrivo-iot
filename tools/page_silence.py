"""The card of the thermostat says the silence of the return and of the air as the page's rows, asked in a browser by what stands in them (moved from the automaton's card).

    uv run --no-project --with websocket-client --with pyyaml python tools/page_silence.py [--shots DIR] [--langs ru,en]

Raises the stand (tools/page_stand.py) on three runs of the new burner's own automaton - the committed tools/stand_scenarios/silent_return.json (the return goes silent in the hold by the return) and the same run with
a return that speaks, and with no return and no role given to one -, opens the page 375 wide in each language, and asks the card Thermostat for its rows. Each verdict prints PASS or FAIL:

  silent-return-said      the return has gone silent: the card has the row of it, and in it the seconds - one more than the run's own frame counted, up to a few that the page counted on since it drew
  the-burner-names-it     the burner burnt the fire off for the return: the thermostat's row of what the water says names the return, and the row of the silence is there at the same moment
  speaking-says-nothing   the return speaks: no row of its silence on the card
  no-role-says-nothing    no role is given to a return: no row of its silence, ever

The verdicts are `judge_*`: pure functions of what the browser answered, held by tools/test_page_silence.py. Exits 1 when a check failed.
"""
import argparse
import copy
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

CHROME_PORT = 9419
STAND_PORT = 8803
WIDTH = 375
SCENARIO = os.path.join(HERE, "stand_scenarios", "silent_return.json")
from page_sources import merged_menu  # noqa: E402

# What the browser is asked: every row - its label and its value - of the thermostat's card: on the page of the module it is the first section, its title standing above it.
ASK = """(function(title){
  const card = Array.from(document.querySelectorAll('main section')).find(s => { const h = s.querySelector('h2'); return h && h.textContent.trim().toLowerCase() === title; }) || document.querySelector('main section');
  if (!card) return JSON.stringify({card: false, rows: []});
  const rows = Array.from(card.querySelectorAll('div.border-b')).map(r => { const spans = r.querySelectorAll('span'); return {label: spans[0] ? spans[0].textContent.trim() : '', value: spans[spans.length - 1] ? spans[spans.length - 1].textContent.trim() : ''}; });
  const line = card.querySelector('[data-line="doing"]');
  return JSON.stringify({card: true, rows, doing: line ? line.textContent.trim() : ''});
})(%s)"""


def judge_silent_return_said(said, label, lasted, drift=15):
    """The row of the return's silence is on the card with the seconds the run counted, up to DRIFT more. -> (ok, words)"""
    row = next((r for r in said["rows"] if r["label"] == label), None)
    if row is None:
        return False, "no row %r on the card (%s)" % (label, ", ".join(r["label"] for r in said["rows"]) or "no rows")
    found = re.findall(r"\d+", row["value"])
    seconds = sum(int(n) * k for n, k in zip(found, ([60, 1] if len(found) == 2 else [1])))
    if not lasted <= seconds <= lasted + drift:
        return False, "the row says %r - %d s - and the run has the return silent for %d s" % (row["value"], seconds, lasted)
    return True, "%r: %s" % (label, row["value"])


def judge_no_row(said, label):
    """No row of the silence is on the card. -> (ok, words)"""
    if not said["card"]:
        return False, "no card of the thermostat on the page"
    row = next((r for r in said["rows"] if r["label"] == label), None)
    return (row is None), ("the row %r stands there: %r" % (label, row["value"])) if row else "no row %r" % label


def judge_the_burner_names_it(said, label, why_word, heat_label):
    """The row of what the water says names the return, and the row of the silence is on the card with it. -> (ok, words)"""
    heat = next((r for r in said["rows"] if r["label"] == heat_label), None)
    if heat is None or why_word not in heat["value"]:
        return False, "the row of what the water says is %r and says no %r" % (heat["value"] if heat else None, why_word)
    return judge_silent_return_said(said, label, 0, drift=10 ** 6)


def frames_of(scenario_path):
    """The frames of the run on the new burner's automaton, each with what the card said of the return's silence and why the burner is doing what it does, by the names the dictionary gives them."""
    from module_compare import named
    from page_stand import dictionary_of

    out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", scenario_path], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("page_silence: the frames were not made:\n" + out.stderr[-1500:])
    frames = json.loads(out.stdout)["frames"]
    seconds = named(dictionary_of(ROOT, "burner"), frames, ["thermostat/silent_return", "automaton/doing.why"])
    for frame, said in zip(frames, seconds):
        frame["silent_return"] = said.get("thermostat/silent_return")
        frame["why"] = said.get("automaton/doing.why")
    return out.stdout, frames


def look(frames_file, second, lang, shot=None):
    """The stand on SECOND, the page in LANG, the card Automaton asked."""
    import page_steady
    from chrome_page import CHROME, Page

    menu = merged_menu("burner")
    word = lambda leaf: leaf[lang] if isinstance(leaf, dict) else leaf  # noqa: E731
    title = next(word(pg["title"]) for pg in menu["tree"]["modules"]["pages"] if pg["id"] == "thermostat").lower()
    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT), "--scenario", frames_file, "--at", str(second)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    profile = tempfile.mkdtemp(prefix="page_silence_")
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={CHROME_PORT}", f"--user-data-dir={profile}", "--no-first-run", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
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
            raise SystemExit("page_silence: the stand or Chrome did not come up")
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=900, deviceScaleFactor=2, mobile=True)
        url = f"http://127.0.0.1:{STAND_PORT}/"
        p.call("Page.navigate", url=url)
        p.settle(1.0)
        p.js("localStorage.setItem('t', 'l'); localStorage.setItem('lang', %s)" % json.dumps(lang))
        p.call("Page.navigate", url=url)
        p.settle(2.0)
        if not page_steady.wait_drawn(p):
            raise SystemExit("page_silence: the page did not draw - the run did not take place; nothing is said of the page")
        # the thermostat's card is the page of the module, under the tab Modules: open the tab (the second of the page's four), then the page whose title is the thermostat's
        p.js("(function(){const tabs = document.querySelectorAll('nav button'); if (tabs[1]) tabs[1].click()})()")
        p.settle(1.0)
        p.js("(function(w){for (const b of document.querySelectorAll('button')) if (b.textContent.trim().toLowerCase() === w) { b.click(); return 1 } return 0})(%s)" % json.dumps(title))
        p.settle(1.5)
        said = json.loads(p.js(ASK % json.dumps(title)))
        if shot:
            p.shot(shot, full=True)
        return said
    finally:
        chrome.terminate()
        stand.terminate()
        chrome.wait(10)
        stand.wait(10)


def variants():
    """The three runs: the committed one, the same with a return that speaks, and the same with no return and no role given to one."""
    base = json.load(open(SCENARIO))
    speaks = copy.deepcopy(base)
    speaks["return_c"] = [[0, 30], [500, 30]]
    none = copy.deepcopy(base)
    none.pop("return_c")
    none["settings"].pop("role_ntc_3")
    none["settings"]["auto_hold"] = "boiler"
    return [("silent", base), ("speaks", speaks), ("no-role", none)]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shots", default=None)
    ap.add_argument("--langs", default="ru,en")
    args = ap.parse_args()
    menu = merged_menu("burner")
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    ok = True
    for lang in args.langs.split(","):
        label = menu["modules"]["thermostat"]["rows"]["silent_return"]["label"][lang]
        heat_label = menu["modules"]["thermostat"]["rows"]["heat"]["label"][lang]
        for name, scenario in variants():
            with tempfile.TemporaryDirectory() as work:
                path = os.path.join(work, "scenario.json")
                json.dump(scenario, open(path, "w"))
                text, frames = frames_of(path)
                frames_file = os.path.join(work, "frames.json")
                open(frames_file, "w").write(text)
                if name == "silent":
                    silent = next(f for f in frames if (f["silent_return"] or 0) >= 12 and f["why"] != "return_silent")
                    named_at = next(f for f in frames if f["why"] == "return_silent")
                    steps = [("silent-return-said", silent), ("the-burner-names-it", named_at)]
                else:
                    steps = [("speaks-says-nothing" if name == "speaks" else "no-role-says-nothing", frames[320])]
                for check, frame in steps:
                    shot = os.path.join(args.shots, "%s_%s_%s.png" % (name, check, lang)) if args.shots else None
                    said = look(frames_file, frame["at"], lang, shot)
                    if check == "silent-return-said":
                        passed, words = judge_silent_return_said(said, label, frame["silent_return"])
                    elif check == "the-burner-names-it":
                        passed, words = judge_the_burner_names_it(said, label, "return" if lang == "en" else "обрат", heat_label)
                    else:
                        passed, words = judge_no_row(said, label)
                    print("%-4s %-20s %-22s %s  %s" % (lang, name, check, "PASS" if passed else "FAIL", words))
                    ok = ok and passed
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
