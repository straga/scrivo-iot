"""The scheme says what every organ does, big enough to read and on nothing else: the page on the stand, driven by the
automaton's own code through a scenario, asked frame by frame what it drew.

    uv run --no-project --with websocket-client python tools/page_scheme.py [--scenario FILE] [--shots DIR]

Makes the frames itself (cargo run -p automaton_card --example scenario, on tools/page_stand_values.json and FILE -
tools/stand_scenarios/lighting.json by default), picks the moments BY VALUE - the burner stopped, the purge, the
igniter burning with air, the auger feeding, the auger pausing - raises
tools/page_stand.py on each, opens the Scheme tab 375 wide in the light theme and checks, each printing one verdict:

  every-organ-said  every organ the automaton runs now has its action written on the drawing
  action-size       an action is drawn no smaller than ACTION_PX on the screen - it is a value, read at a glance
  nothing-on-words  no text of the scheme lies on another text or across a line of the drawing

The words expected are the page's own (burning.onScheme, burning.schemeText), so the check is of what is DRAWN and
where, not of the rules that pick the words - those are tools/test_page_burning.js's. With --shots DIR each moment is
saved DIR/<moment>.png. Exits 1 when a check failed. Stops the stand and Chrome after each moment.
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

from chrome_page import CHROME, Page  # noqa: E402

CHROME_PORT = 9413
STAND_PORT = 8798
WIDTH = 375
# The smallest an organ's action is drawn, in screen pixels at a phone's width: the numbers of the scheme are drawn at
# 13, the names of its parts at 9-10, and an action is read as a value, not as a name (REF_Device_Page_Look).
ACTION_PX = 12.0


def value(frame, group, name):
    """The row NAME of the card GROUP in FRAME, as a page of contract 1 read its value (tools/stand_frames.py)."""
    return frame["named"].get((group, name), {})


# The moments, each the first frame of the scenario where it holds - by the automaton's values, not by its second.
MOMENTS = [
    ("stop", lambda f: value(f, "automaton", "stage").get("word") == "stop"),
    ("purge", lambda f: value(f, "automaton", "stage").get("word") == "purge"),
    ("igniter_burning", lambda f: value(f, "automaton", "igniter").get("on")),
    ("feeding", lambda f: value(f, "automaton", "stage").get("word") == "working" and value(f, "automaton", "auger").get("on")),
    ("pause", lambda f: value(f, "automaton", "stage").get("word") == "working" and value(f, "automaton", "auger").get("on") is False),
]

# What the page drew, asked of the page: the actions it should say (its own rules), each text of the scheme with its
# box and size on the screen, and whether a text lies across a line of the drawing.
ASK = """(function(){
  const svg = document.querySelector('svg[viewBox="0 0 320 250"]');
  if (!svg) return JSON.stringify({error: 'no scheme on the page'});
  const things = window.__things || [];
  const want = Object.keys(burning.PLACES)
    .map(organ => ({organ, label: burning.onScheme(organ, things)}))
    .filter(one => one.label)
    .map(one => ({organ: one.organ, text: burning.schemeText(one.label)}));
  const texts = Array.from(svg.querySelectorAll('text')).filter(t => t.textContent.trim());
  const shapes = Array.from(svg.querySelectorAll('path, rect, circle'));
  const pt = svg.createSVGPoint();
  const crosses = (t) => {
    const b = t.getBBox();
    const points = [];
    for (let x = b.x + 0.5; x < b.x + b.width - 0.5; x += 0.5) points.push([x, b.y + b.height / 2]);
    for (let y = b.y + 1; y < b.y + b.height - 1; y += 0.5) points.push([b.x + b.width / 2, y]);
    for (const s of shapes) {
      for (const [x, y] of points) {
        pt.x = x; pt.y = y;
        if (s.isPointInStroke(pt)) return s.tagName + (s.getAttribute('d') ? ' ' + s.getAttribute('d') : '');
      }
    }
    return null;
  };
  const drawn = texts.map(t => {
    const r = t.getBoundingClientRect();
    const px = parseFloat(t.getAttribute('font-size')) * svg.getScreenCTM().a;
    return {text: t.textContent.trim(), px, box: [r.left, r.top, r.right, r.bottom], crosses: crosses(t)};
  });
  return JSON.stringify({want, drawn});
})()"""


def overlap(a, b):
    """Whether two boxes on the screen share more than a hairline."""
    return min(a[2], b[2]) - max(a[0], b[0]) > 0.5 and min(a[3], b[3]) - max(a[1], b[1]) > 0.5


def judge(said):
    """The verdicts on one moment: (check, ok, words)."""
    drawn = said["drawn"]
    texts = [d["text"] for d in drawn]
    missing = [w["organ"] + ": " + w["text"] for w in want_of(said) if w["text"] not in texts]
    small = ["%s %.1f px" % (d["text"], d["px"]) for d in drawn if d["text"] in {w["text"] for w in want_of(said)}
             and d["px"] < ACTION_PX]
    on_words = []
    for i, a in enumerate(drawn):
        for b in drawn[i + 1:]:
            if overlap(a["box"], b["box"]):
                on_words.append("'%s' on '%s'" % (a["text"], b["text"]))
        if a["crosses"]:
            on_words.append("'%s' across %s" % (a["text"], a["crosses"]))
    return [
        ("every-organ-said", not missing, "missing " + "; ".join(missing) if missing else "%d said" % len(want_of(said))),
        ("action-size", not small, "under %.0f px: %s" % (ACTION_PX, "; ".join(small)) if small else "at least %.0f px" % ACTION_PX),
        ("nothing-on-words", not on_words, "; ".join(on_words) if on_words else "clear"),
    ]


def want_of(said):
    return said["want"]


def frames_of(scenario):
    """The scenario's frames on the stand's state (tools/page_stand_values.json), each with its rows by name."""
    from page_stand import dictionary_of
    from stand_frames import frames_of as made, named

    dictionary = dictionary_of(ROOT)
    frames = made(scenario, ROOT)
    for frame in frames:
        frame["named"] = named(dictionary, frame)
    return frames


def look(frames_file, second, frame, shot):
    """Raises the stand on SECOND, opens the page, asks it; stops both whatever happens."""
    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT),
                              "--scenario", frames_file, "--at", str(second)],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    profile = tempfile.mkdtemp(prefix="page_scheme_")
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={CHROME_PORT}",
                               f"--user-data-dir={profile}", "--no-first-run", "about:blank"],
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
            raise SystemExit("page_scheme: the stand or Chrome did not come up")
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=WIDTH, height=900, deviceScaleFactor=2, mobile=True)
        url = f"http://127.0.0.1:{STAND_PORT}/"
        p.call("Page.navigate", url=url)
        p.settle(1.0)
        p.js("localStorage.setItem('t', 'l')")
        p.call("Page.navigate", url=url)
        p.settle(2.0)
        # the things the page drew from, made by its own dictionary.js, so the page's own rules say what it should show
        from stand_frames import THINGS_JS
        p.js("window.__things = JSON.parse(" + THINGS_JS + "); 1")
        said = json.loads(p.js(ASK))
        if shot:
            p.shot(shot, full=True)
        return said
    finally:
        chrome.terminate()
        stand.terminate()
        chrome.wait(10)
        stand.wait(10)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scenario", default=os.path.join(HERE, "stand_scenarios", "lighting.json"))
    ap.add_argument("--shots", default=None)
    args = ap.parse_args()
    frames = frames_of(args.scenario)
    work = tempfile.mkdtemp(prefix="page_scheme_frames_")
    frames_file = os.path.join(work, "frames.json")
    json.dump({"frames": [{"at": f["at"], "values": f["values"]} for f in frames]}, open(frames_file, "w"))
    if args.shots:
        os.makedirs(args.shots, exist_ok=True)
    failed = False
    for name, holds in MOMENTS:
        frame = next((f for f in frames if holds(f)), None)
        if frame is None:
            print("%-16s FAIL  the scenario never comes to it" % name)
            failed = True
            continue
        said = look(frames_file, frame["at"], frame, os.path.join(args.shots, name + ".png") if args.shots else None)
        if "error" in said:
            print("%-16s FAIL  %s" % (name, said["error"]))
            failed = True
            continue
        for check, ok, words in judge(said):
            print("%-16s %-17s %s  %s" % (name + " @" + str(frame["at"]), check, "PASS" if ok else "FAIL", words))
            failed = failed or not ok
    os.remove(frames_file)
    os.rmdir(work)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
