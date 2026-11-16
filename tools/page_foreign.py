"""A page given a dictionary of another contract draws nothing of the board and says why: the same way a person meets it - a page from
the board's own kit, a board behind it that answers the contract before.

    uv run --no-project --with websocket-client python tools/page_foreign.py [--page DIR] [--dictionary FILE]

Raises tools/page_stand.py on a free port with the dictionary FILE (default: a real dictionary of contract 2, `tools/test_old_dictionary.json`,
which says nothing of the automaton), opens the page in headless Chrome and reads what it shows. Then the same page against the burner's own
dictionary, which it must draw: a refusal that also refused the board it reads would be no verdict.

  refuses   the words name both numbers and say to reload, and no button that starts or stops the automaton is drawn
  draws     with the dictionary of its own contract the page has those buttons and says no refusal

Exits 0 when both hold, 1 when not, 2 when the ports it needs are held. Stops the stand and Chrome whatever happens.
"""
import argparse
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

from page_heap import taken  # noqa: E402

CHROME_PORT = 9417
STAND_PORT = 8797
OLD = os.path.join(HERE, "test_old_dictionary.json")
# the number the page reads is the document's (docs/CONTRACT_Device_Api.md): a refusal names it, and the check asks for what the document says and not for a number it wrote down once
CONTRACT = int(re.search(r"Contract version: `(\d+)`", open(os.path.join(ROOT, "docs", "CONTRACT_Device_Api.md"), encoding="utf-8").read()).group(1))

READ = """JSON.stringify({text: document.body.innerText, buttons: Array.from(document.querySelectorAll('button')).map((b) => b.innerText.trim())})"""


def shown(dictionary_file, page_dir, page_class, chrome_binary):
    """What the page at PAGE_DIR shows against a board answering DICTIONARY_FILE (None - the burner's own): {text, buttons}."""
    stand_args = [sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT)]
    if dictionary_file:
        stand_args += ["--dictionary", dictionary_file]
    if page_dir:
        stand_args += ["--page", page_dir]
    stand = subprocess.Popen(stand_args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    profile = tempfile.mkdtemp(prefix="page_foreign_")
    chrome = subprocess.Popen([chrome_binary, "--headless=new", f"--remote-debugging-port={CHROME_PORT}", f"--user-data-dir={profile}",
                               "--no-first-run", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(100):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{STAND_PORT}/state", timeout=1)
                break
            except OSError:
                time.sleep(0.2)
        targets = None
        for _ in range(75):
            try:
                targets = json.load(urllib.request.urlopen(f"http://127.0.0.1:{CHROME_PORT}/json"))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            raise SystemExit("page_foreign: Chrome did not come up")
        p = page_class(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=1000, height=900, deviceScaleFactor=1, mobile=False)
        url = f"http://127.0.0.1:{STAND_PORT}/"
        p.call("Page.navigate", url=url)
        p.settle(1.0)
        # the words below are English, and the page keeps its language in localStorage
        p.js("localStorage.setItem('lang', 'en')")
        p.call("Page.navigate", url=url)
        p.settle(4.0)
        return json.loads(p.js(READ))
    finally:
        for proc in (chrome, stand):
            proc.terminate()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--page", default=None, help="the page files to serve instead of the device's own (a revision out of git)")
    ap.add_argument("--dictionary", default=OLD, help="the dictionary of the board before, a file")
    args = ap.parse_args()
    busy = taken([CHROME_PORT, STAND_PORT])
    if busy:
        print("not measured: port %s is held by another process" % ", ".join(map(str, busy)))
        sys.exit(2)
    from chrome_page import CHROME, Page

    old = shown(args.dictionary, args.page, Page, CHROME)
    own = shown(None, args.page, Page, CHROME)
    runs = lambda text: "start" in text.lower() or "stop" in text.lower()  # noqa: E731
    checks = [
        ("refuses-names-both", "contract 2" in old["text"] and "contract %d" % CONTRACT in old["text"], old["text"][:200].replace("\n", " | ")),
        ("refuses-says-reload", "reload" in old["text"].lower(), ""),
        ("refuses-draws-no-automaton-buttons", not any(runs(b) for b in old["buttons"]), str(old["buttons"])),
        ("draws-its-own", any(runs(b) for b in own["buttons"]) and "contract" not in own["text"].lower(), str(own["buttons"])),
    ]
    for name, ok, why in checks:
        print("%s %s %s" % ("PASS" if ok else "FAIL", name, "" if ok else why))
    sys.exit(0 if all(ok for _, ok, _ in checks) else 1)


if __name__ == "__main__":
    main()
