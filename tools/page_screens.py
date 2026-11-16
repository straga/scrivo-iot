"""Screens of the device page, every tab, both themes, a phone's width and a computer's.

    uv run --no-project --with websocket-client python tools/page_screens.py OUT_DIR [--url http://127.0.0.1:8790/]
        [--tabs Burner,Scheme,Settings] [--widths 375,900] [--themes l,d] [--count]

A tab may be followed by `>` and the words of a button inside it - `Modules>Energy` opens the tab Modules, then the page of the module
Energy on its list; the file is called modules_energy_<theme>_<width>.png.

Saves OUT_DIR/<tab>_<theme>_<width>.png of the whole page, and prints one line per screen. With --count it also
prints, per card, how many lines of label and note stand on it - the number a "the page says less" answer is
judged by, taken from the page rather than from the source.

The theme is set the way the page's own button sets it, through the kept choice, so what is captured is what a
person who pressed that button sees.

tools/page_shots.py is the one that talks to a LIVE board and types into its terminal. This one is for a page in
front of a stand (tools/page_stand.py), where every tab can be opened and both themes can be asked for without
taking the boiler off a person for an hour.
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

PORT = 9399

# Opens the tab whose button carries these words.
PICK = ("(function(w){for(const b of document.querySelectorAll('nav button'))"
        " if(b.textContent.trim()===w){b.click(); return 1} return 0})(%s)")

# Opens, inside the tab that is open, the button that carries these words - a module's name on the list of modules.
OPEN = ("(function(w){for(const b of document.querySelectorAll('button'))"
        " if(!b.closest('nav') && b.textContent.trim()===w){b.click(); return 1} return 0})(%s)")

# How much a card says in labels and notes: the count a shorter page is judged by. Rows of readings are not
# counted - they are what the card is FOR; what is counted is the words written around the fields.
COUNT = """(function(){
  const out = [];
  for (const s of document.querySelectorAll('section')) {
    const h = s.querySelector('h2');
    if (!h) continue;
    let labels = 0, notes = 0, chars = 0;
    for (const f of s.querySelectorAll('input, select')) {
      const line = f.closest('div');
      if (!line) continue;
      const lab = line.querySelector(':scope > span');
      if (lab && lab.textContent.trim()) { labels += 1; chars += lab.textContent.trim().length; }
    }
    for (const n of s.querySelectorAll('p, div.text-\\\\[13px\\\\]')) {
      const t = n.textContent.trim();
      if (t) { notes += 1; chars += t.length; }
    }
    out.push({card: h.textContent.trim(), labels: labels, notes: notes, chars: chars});
  }
  return JSON.stringify(out);
})()"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--url", default="http://127.0.0.1:8790/")
    ap.add_argument("--tabs", default="Burner,Scheme,Settings")
    ap.add_argument("--widths", default="375,900")
    ap.add_argument("--themes", default="l,d")
    ap.add_argument("--count", action="store_true", help="also print how much each card says")
    ap.add_argument("--name", default="", help="a word put in front of every file name")
    ap.add_argument("--wait", type=float, default=0.0, help="seconds to let a page stand after it has its state, before the first tab: a mark of the scheme "
                    "is coloured only 10 s after it stood out (scheme.js, DWELL_MS), so screens taken to be compared are taken after that")
    args = ap.parse_args()

    os.makedirs(args.out, exist_ok=True)
    profile = tempfile.mkdtemp(prefix="page_screens_")
    chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % PORT,
                               "--user-data-dir=%s" % profile, "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % PORT))
                break
            except OSError:
                time.sleep(0.2)
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")

        for theme in args.themes.split(","):
            # set the way the page's own button sets it, before anything is drawn
            p.call("Page.navigate", url=args.url)
            p.settle(1.5)
            p.js("try { localStorage.setItem('t', %s) } catch (e) {}" % json.dumps(theme))
            for width in [int(w) for w in args.widths.split(",")]:
                p.call("Emulation.setDeviceMetricsOverride", width=width, height=900,
                       deviceScaleFactor=1, mobile=width < 500)
                p.call("Page.navigate", url=args.url)
                # the state has to be there, or the screen is of an empty page
                end = time.time() + 15
                while time.time() < end:
                    if p.js("!!document.querySelector('h1') && document.querySelector('h1').textContent.trim()"):
                        break
                    time.sleep(0.25)
                p.settle(args.wait)
                for tab in args.tabs.split(","):
                    top, _, inner = tab.partition(">")
                    if not p.js(PICK % json.dumps(top)):
                        print("no tab called %s" % top)
                        continue
                    p.settle(2.0)
                    if inner:
                        if not p.js(OPEN % json.dumps(inner)):
                            print("no button called %s in the tab %s" % (inner, top))
                            continue
                        p.settle(2.0)
                    name = "%s%s_%s_%d.png" % (args.name and args.name + "_", tab.lower().replace(">", "_"), theme, width)
                    p.shot(os.path.join(args.out, name), full=True)
                    if args.count and width == 375:
                        for card in json.loads(p.js(COUNT)):
                            print("   %-8s %-14s %2d labels  %2d notes  %4d chars"
                                  % (tab.lower(), card["card"], card["labels"], card["notes"], card["chars"]))
    finally:
        chrome.terminate()


if __name__ == "__main__":
    main()
