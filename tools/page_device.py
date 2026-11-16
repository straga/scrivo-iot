"""What a person sees and does on the page of a device with no scheme of its own, in headless Chrome, behind the device's stand: the Dual R3.

    uv run --no-project --with websocket-client python tools/page_device.py [--device bench_dualr3] [--shots DIR] [--lang en|ru]

Three things, each asked of the page and answered from what it drew and what it sent (the stand writes every POST down):

  main      the page opens on its main tab, and the cards the menu puts there are drawn with values: the relays, the wall keys, the meter, the energy
  relay     one press of a relay's button sends its command at once, and the page shows the relay as the stand's answer says
  reset     the energy's reset is asked about: the first press sends nothing and asks, a cancel takes it back, and only the second press sends - once

A page that drew cards without values, or sent a command on the first press of a reset, would be no verdict - which is why every line says what it found. Exits 0 when all hold, 1 when not, 2 when
the ports it needs are held. Stops the stand and Chrome whatever happens.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

import page_screens as ps  # noqa: E402
from page_heap import taken  # noqa: E402

CHROME_PORT = 9419
STAND_PORT = 8799

TEXT = "document.body.innerText.toLowerCase()"
TABS = "JSON.stringify(Array.from(document.querySelectorAll('nav button')).map((b) => b.innerText.trim()))"
# the buttons of the page by the words on them, with the row they stand in: the label of the row is the first words of it
BUTTONS = ("JSON.stringify(Array.from(document.querySelectorAll('main button')).map((b) => ({words: b.innerText.trim(), row: (b.closest('div') || {}).innerText || ''})))")
CLICK = ("(function(i){const all=Array.from(document.querySelectorAll('main button')); if (!all[i]) return 0; all[i].click(); return 1})(%d)")
CLICK_WORDS = ("(function(w, row){for (const b of document.querySelectorAll('main button')) if (b.innerText.trim().toLowerCase() === w.toLowerCase() && ((b.closest('div')||{}).innerText||'').toLowerCase().includes(row.toLowerCase())) { b.click(); return 1 } return 0})(%s, %s)")


def posts(log):
    """The POSTs the stand has written down, [(path, body)]."""
    if not os.path.isfile(log):
        return []
    out = []
    for line in open(log).read().splitlines():
        words = line.split(None, 2)
        if len(words) >= 2 and words[1].startswith("/"):
            out.append((words[1], words[2] if len(words) > 2 else None))
    return out


def run(device, lang, shots):
    """[what was found, problem or None] for each thing asked."""
    tmp = tempfile.mkdtemp(prefix="page_device_")
    log = os.path.join(tmp, "posts.log")
    dictionary = None
    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--device", device, "--port", str(STAND_PORT), "--log", log],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    chrome = subprocess.Popen([ps.CHROME, "--headless=new", "--remote-debugging-port=%d" % CHROME_PORT, "--user-data-dir=%s" % os.path.join(tmp, "profile"), "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    found = []
    try:
        targets = None
        for _ in range(150):
            try:
                dictionary = json.load(urllib.request.urlopen("http://127.0.0.1:%d/dictionary" % STAND_PORT, timeout=1))
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % CHROME_PORT))
                break
            except OSError:
                time.sleep(0.4)
        if targets is None:
            raise SystemExit("page_device: the stand or Chrome did not come up")
        at = {(t["card"], t["name"]): i for i, t in enumerate(dictionary["things"]) if not t.get("part_of")}
        page = ps.Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        page.call("Page.enable")
        page.call("Emulation.setDeviceMetricsOverride", width=900, height=1200, deviceScaleFactor=1, mobile=False)
        page.call("Page.navigate", url="http://127.0.0.1:%d/?lang=%s" % (STAND_PORT, lang))
        end = time.time() + 20
        while time.time() < end:
            if page.js("document.body.innerText.length > 80"):
                break
            time.sleep(0.25)
        page.settle(1.5)
        text = page.js(TEXT)
        tabs = json.loads(page.js(TABS))
        if shots:
            os.makedirs(shots, exist_ok=True)
            page.shot(os.path.join(shots, "main_%s.png" % lang), full=True)
        words = WORDS[lang]

        # main
        problems = []
        if not tabs or tabs[0] != words["main"]:
            problems.append("the first tab is %r, not %r (tabs: %s)" % (tabs[:1], words["main"], tabs))
        for need in words["seen"]:
            if need.lower() not in text:
                problems.append("the page does not say %r" % need)
        found.append(("main", problems, tabs, text))

        # relay
        problems = []
        before = len(posts(log))
        relay1 = at[("relays", "relay1")]
        if not page.js(CLICK_WORDS % (json.dumps(words["off_button"]), json.dumps(words["relay1"]))):
            problems.append("relay 1 has no button saying %r" % words["off_button"])
        else:
            page.settle(1.2)
            sent = posts(log)[before:]
            if sent != [("/off/%d" % relay1, "")] and sent != [("/off/%d" % relay1, None)]:
                problems.append("one press of the relay sent %s, not one /off/%d" % (sent, relay1))
            if words["relay1_on_state"] in page.js(TEXT).split(words["relay1"])[1].split("\n")[1]:
                problems.append("the page still says relay 1 is on after the press")
        found.append(("relay", problems, None, None))

        # reset
        problems = []
        reset = at[("energy", "reset")]
        before = len(posts(log))
        if not page.js(CLICK_WORDS % (json.dumps(words["zero"]), json.dumps(""))):
            problems.append("no button %r on the main screen" % words["zero"])
        else:
            page.settle(0.6)
            asked = page.js(TEXT)
            if posts(log)[before:]:
                problems.append("the first press already sent %s" % posts(log)[before:])
            if words["sure"].lower() not in asked or words["cancel"].lower() not in asked or words["irreversible"].lower() not in asked:
                problems.append("the page does not ask: it lacks %r, %r or %r" % (words["sure"], words["cancel"], words["irreversible"]))
            if shots:
                page.shot(os.path.join(shots, "asking_%s.png" % lang), full=True)
            page.js(CLICK_WORDS % (json.dumps(words["cancel"]), json.dumps("")))
            page.settle(0.6)
            if words["sure"].lower() in page.js(TEXT) or posts(log)[before:]:
                problems.append("the cancel did not take the asking back, or sent something: %s" % posts(log)[before:])
            page.js(CLICK_WORDS % (json.dumps(words["zero"]), json.dumps("")))
            page.settle(0.4)
            page.js(CLICK_WORDS % (json.dumps(words["sure"]), json.dumps("")))
            page.settle(1.0)
            sent = posts(log)[before:]
            if [p for p, _ in sent] != ["/press/%d" % reset]:
                problems.append("asked and confirmed, the page sent %s, not one /press/%d" % (sent, reset))
        found.append(("reset", problems, None, None))
        return found
    finally:
        chrome.terminate()
        stand.terminate()
        shutil.rmtree(tmp, ignore_errors=True)


WORDS = {
    "en": {
        "main": "Main", "seen": ["relay 1 (L1)", "switch 1 (S1)", "voltage", "237.4 V", "L1", "today", "kWh"], "off_button": "off", "relay1": "relay 1 (l1)", "relay1_on_state": "on",
        "zero": "Zero the counters", "sure": "Yes, do it", "cancel": "Cancel", "irreversible": "This cannot be taken back.",
    },
    "ru": {
        "main": "Главная", "seen": ["реле 1 (L1)", "выключатель 1 (S1)", "напряжение", "237.4 В", "L1", "сегодня", "кВт·ч"], "off_button": "выкл", "relay1": "реле 1 (l1)", "relay1_on_state": "вкл",
        "zero": "Обнулить счётчики", "sure": "Да, выполнить", "cancel": "Отмена", "irreversible": "Это нельзя отменить.",
    },
}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--device", default="bench_dualr3")
    ap.add_argument("--lang", default="en", choices=sorted(WORDS))
    ap.add_argument("--shots", default=None)
    args = ap.parse_args()
    busy = taken([CHROME_PORT, STAND_PORT])
    if busy:
        print("page_device: the port%s %s %s held" % ("s" if len(busy) > 1 else "", ", ".join(map(str, busy)), "are" if len(busy) > 1 else "is"))
        return 2
    failed = 0
    for name, problems, _, _ in run(args.device, args.lang, args.shots):
        print("%-6s %s" % (name, "ok" if not problems else "FAIL: " + "; ".join(problems)))
        failed += bool(problems)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
