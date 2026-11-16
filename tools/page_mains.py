"""The mains line on the REAL page of the bench board, in the browser at 375 px: is it drawn, what does it say, does it lie over another text of the scheme or across a line of the drawing - in both languages and both themes.

    uv run --no-project --with websocket-client python tools/page_mains.py <board address> <out dir>

Uses tools/chrome_page.Page (the Chrome the other page tools start). The verdict is geometry of what the browser drew, not of the page's source.
"""
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
from chrome_page import CHROME, Page  # noqa: E402

PORT = 9433
ASK = """(function(){
  const svg = document.querySelector('svg[viewBox="0 0 320 250"]');
  if (!svg) return JSON.stringify({error: 'no scheme on the page'});
  const texts = Array.from(svg.querySelectorAll('text')).filter(t => t.textContent.trim());
  const shapes = Array.from(svg.querySelectorAll('path, rect, circle, line'));
  const pt = svg.createSVGPoint();
  const crosses = (t) => {
    const b = t.getBBox();
    const points = [];
    for (let x = b.x + 0.5; x < b.x + b.width - 0.5; x += 0.5) points.push([x, b.y + b.height / 2], [x, b.y + 1], [x, b.y + b.height - 1]);
    for (let y = b.y + 1; y < b.y + b.height - 1; y += 0.5) points.push([b.x + 1, y], [b.x + b.width - 1, y]);
    for (const s of shapes) {
      for (const [x, y] of points) {
        pt.x = x; pt.y = y;
        let hit = false;
        try { hit = s.isPointInStroke(pt) || (s.tagName === 'circle' && s.isPointInFill(pt)); } catch (e) {}
        if (hit) return s.tagName + (s.getAttribute('d') ? ' ' + s.getAttribute('d').slice(0, 40) : '');
      }
    }
    return null;
  };
  const box = (t) => { const r = t.getBoundingClientRect(); return [r.left, r.top, r.right, r.bottom]; };
  const all = texts.map(t => ({text: t.textContent.trim(), mine: t.getAttribute('data-drawn') === 'mains', px: parseFloat(t.getAttribute('font-size')) * svg.getScreenCTM().a, box: box(t), crosses: crosses(t)}));
  const s = svg.getBoundingClientRect();
  return JSON.stringify({svg: [s.left, s.top, s.right, s.bottom], texts: all});
})()"""


def overlap(a, b):
    return min(a[2], b[2]) - max(a[0], b[0]) > 0.5 and min(a[3], b[3]) - max(a[1], b[1]) > 0.5


MIN_PX = 12.0


def judge(said):
    """Whether the mains line is drawn, readable and clear of everything: (ok, words). SAID is what ASK returned."""
    if "error" in said:
        return False, said["error"]
    mine = [t for t in said["texts"] if t["mine"]]
    if not mine:
        return False, "the mains line is not drawn"
    m = mine[0]
    bad = []
    s = said["svg"]
    if m["box"][0] < s[0] - 0.5 or m["box"][2] > s[2] + 0.5 or m["box"][1] < s[1] - 0.5 or m["box"][3] > s[3] + 0.5:
        bad.append("outside the drawing")
    for t in said["texts"]:
        if t is not m and overlap(m["box"], t["box"]):
            bad.append("lies over '%s'" % t["text"])
    if m["crosses"]:
        bad.append("crosses %s" % m["crosses"])
    if m["px"] < MIN_PX:
        bad.append("drawn at %.1f px, less than %d" % (m["px"], MIN_PX))
    if bad:
        return False, "'%s': %s" % (m["text"], "; ".join(bad))
    return True, "'%s' at %.1f px; clear of %d other texts and of every line" % (m["text"], m["px"], len(said["texts"]) - 1)


def main():
    board, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    profile = tempfile.mkdtemp(prefix="page_mains_")
    chrome = subprocess.Popen([CHROME, "--headless=new", f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}", "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    verdicts = []
    try:
        targets = None
        for _ in range(75):
            try:
                targets = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json"))
                break
            except OSError:
                time.sleep(0.2)
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=375, height=900, deviceScaleFactor=2, mobile=True)
        url = f"http://{board}/"
        for lang in ("ru", "en"):
            for theme in ("l", "d"):
                p.call("Page.navigate", url=url)
                p.settle(1.0)
                p.js("localStorage.setItem('t', %s); localStorage.setItem('lang', %s)" % (json.dumps(theme), json.dumps(lang)))
                p.call("Page.navigate", url=url)
                p.settle(4.0)
                p.js("Array.from(document.querySelectorAll('button')).filter(b => /^(Scheme|Схема)$/.test(b.textContent.trim())).forEach(b => b.click()); 1")
                p.settle(1.5)
                said = json.loads(p.js(ASK))
                name = f"{lang}_{theme}"
                p.shot(os.path.join(out, f"{name}.png"), full=True)
                ok, words = judge(said)
                verdicts.append((name, ok, words))
    finally:
        chrome.terminate()
        chrome.wait(10)
    for name, ok, words in verdicts:
        print("%s %-6s %s" % ("PASS" if ok else "FAIL", name, words))
    sys.exit(0 if verdicts and all(ok for _, ok, _ in verdicts) else 1)


if __name__ == "__main__":
    main()
