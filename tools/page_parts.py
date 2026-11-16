"""What a person sees when a device adds its parts to the engine's page wrongly, and when it adds them right: the page as the board would serve it,
in headless Chrome, behind a stand.

    uv run --no-project --with websocket-client python tools/page_parts.py [--root TREE] [--shots DIR]

Nine devices, each made up here - a version, a page folder with a script and a menu - and laid by tools/page_kit.py into a page the stand serves:

  right     one part added under its name, a section of the menu asks for it with its `part` field   the part is drawn, no refusal is said
  twice     the same part added twice                                                                 the page draws nothing but the refusal, which names the part
  tab twice the same tab added twice                                                                  the same, naming the tab
  missing   a section asks for a part no script added                                                the section says so in its place, by the name of the part and
                                                                                                      of the section, and the rest of the page is drawn

and the same three of the MAIN SCREEN, where a section of the menu's `main` asks for a part as it does on a module's page:

  main      the part is added and a `main` section asks for it                                        the part is drawn on the main screen
  maintab   the same under a tab of the device's own, whose main screen is that tab's part            the part is drawn under the tab
  mainmiss  a `main` section asks for a part no script added                                          the section says so in its place, and the rest of the screen is drawn

and the same two of the SYSTEM TAB, in a section of the Service group (`tree.system.sections`, `group: service`) next to a plain one:

  system    the part is added and a system section asks for it                                        the part is drawn in the system tab, the plain section is drawn beside it
  sysmiss   a system section asks for a part no script added                                          the section says so in its place, and the rest of the tab is drawn

Between them the four places a section is drawn in - a module's page, the main screen, the screen under a tab of the device's own, the system tab - each meet a part added, a part not added and a plain
section (page.js, `Placed`). A refusal that also refused the right device would be no verdict, which is why the first is here. TREE is a tree with `page/` in it (default: this one) - a revision
taken out of git shows what the page did before. Exits 0 when all nine hold, 1 when not, 2 when the ports it needs are held. Stops the stand and Chrome whatever
happens.
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

import page_kit  # noqa: E402
import page_screens as ps  # noqa: E402
from page_heap import taken  # noqa: E402

CHROME_PORT = 9418
STAND_PORT = 8798

READ = "JSON.stringify({text: document.body.innerText, tabs: Array.from(document.querySelectorAll('nav button')).map((b) => b.innerText.trim())})"
OPEN = ("(function(w){for (const b of document.querySelectorAll('main button')) if (b.textContent.trim() === w) { b.click(); return 1 } return 0})(%s)")
OPEN_TAB = ("(function(w){for (const b of document.querySelectorAll('nav button')) if (b.textContent.trim() === w) { b.click(); return 1 } return 0})(%s)")

PART = """{
const { Component, xml } = owl;
class Marked extends Component {
    static props = { given: { type: String, optional: true } };
    static template = xml`<section class="p-3"><h2>PART-DRAWN</h2><p t-esc="props.given"/></section>`;
}
page.parts.section("marked", Marked);
%s
}
"""
TAB = """{
const { Component, xml } = owl;
class Mine extends Component {
    static props = { act: Function };
    static template = xml`<section class="p-3"><h2>TAB-DRAWN</h2></section>`;
}
page.parts.tab({ id: "mine", label: "modules", component: Mine });
%s
}
"""


def menu(section):
    """The device's menu: one module with a plain section and SECTION."""
    return {"tree": {"modules": {"groups": [{"id": "g", "title": {"en": "Group", "ru": "Группа"}, "modules": ["m"]}],
                                 "pages": [{"id": "m", "title": {"en": "Module", "ru": "Модуль"},
                                            "sections": [{"id": "plain", "title": {"en": "Plain section", "ru": "Простой раздел"}}, section]}]}}}


DEVICES = {
    "right": {"script": PART % "", "menu": menu({"id": "asks", "title": {"en": "Asks", "ru": "Просит"}, "part": "marked", "given": "ABOUT-IT"})},
    "twice": {"script": PART % 'page.parts.section("marked", class Other {});', "menu": menu({"id": "asks", "title": {"en": "Asks", "ru": "Просит"}, "part": "marked"})},
    "tabtwice": {"script": TAB % 'page.parts.tab({ id: "mine", label: "modules", component: class Other {} });', "menu": menu({"id": "asks", "title": {"en": "Asks", "ru": "Просит"}})},
    "missing": {"script": PART % "", "menu": menu({"id": "asks", "title": {"en": "Asks", "ru": "Просит"}, "part": "gauge"})},
}


def main_menu(section):
    """The device's menu with the main screen's sections: a plain one and SECTION."""
    return {"tree": {"main": {"sections": [{"id": "plain", "title": {"en": "Plain section", "ru": "Простой раздел"}}, section]}}}


ASKS_MARKED = {"id": "asks", "title": {"en": "Asks", "ru": "Просит"}, "part": "marked", "given": "ABOUT-IT"}

DEVICES.update({
    "main": {"script": PART % "", "menu": main_menu(ASKS_MARKED)},
    "maintab": {"script": (PART % "") + (TAB % ""), "menu": main_menu(ASKS_MARKED)},
    "mainmiss": {"script": PART % "", "menu": main_menu({"id": "asks", "title": {"en": "Asks", "ru": "Просит"}, "part": "gauge"})},
})


def system_menu(section):
    """The device's menu with sections in the system tab, in the Service: a plain one and SECTION."""
    return {"tree": {"system": {"sections": [{"id": "plain", "group": "service", "title": {"en": "Plain section", "ru": "Простой раздел"}}, {"group": "service", **section}]}}}


DEVICES.update({
    "system": {"script": PART % "", "menu": system_menu(ASKS_MARKED)},
    "sysmiss": {"script": PART % "", "menu": system_menu({"id": "asks", "title": {"en": "Asks", "ru": "Просит"}, "part": "gauge"})},
})

# where a device is opened before it is read: a module's page is a button of the main area, the system tab a button of the tabs
OPENING = {"right": "Module", "missing": "Module", "system": "System", "sysmiss": "System"}


def lay_device(tree, name, spec):
    """Makes the device NAME in TREE and returns the page folder the stand serves."""
    device = os.path.join(tree, "devices", name)
    os.makedirs(os.path.join(device, "page"))
    open(os.path.join(device, "version.txt"), "w").write("probe-26.10.08.1\n")
    open(os.path.join(device, "page", "page.json"), "w").write(json.dumps({"scripts": ["part.js"]}))
    open(os.path.join(device, "page", "part.js"), "w").write(spec["script"])
    open(os.path.join(device, "page", "menu.json"), "w").write(json.dumps(spec["menu"]))
    return page_kit.page_into(name, os.path.join(tree, "served", name), root=tree)


def seen(page_dir, opening):
    """What Chrome shows of the page in PAGE_DIR: {text, tabs}; OPENING is what is opened first: the module `Module`, the tab `System`, or none."""
    profile = tempfile.mkdtemp(prefix="page_parts_")
    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT), "--page", page_dir], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    chrome = subprocess.Popen([ps.CHROME, "--headless=new", "--remote-debugging-port=%d" % CHROME_PORT, "--user-data-dir=%s" % profile, "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        targets = None
        for _ in range(75):
            try:
                urllib.request.urlopen("http://127.0.0.1:%d/state" % STAND_PORT, timeout=1)
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % CHROME_PORT))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            raise SystemExit("page_parts: the stand or Chrome did not come up")
        page = ps.Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        page.call("Page.enable")
        page.call("Page.navigate", url="http://127.0.0.1:%d/?lang=en" % STAND_PORT)
        end = time.time() + 15
        while time.time() < end:
            if page.js("document.body.innerText.length > 40"):
                break
            time.sleep(0.25)
        page.settle(1.5)
        if opening == "Module":
            page.js(OPEN % json.dumps("Module"))
            page.settle(1.0)
        elif opening:
            page.js(OPEN_TAB % json.dumps(opening))
            page.settle(1.0)
        return json.loads(page.js(READ))
    finally:
        chrome.terminate()
        stand.terminate()
        shutil.rmtree(profile, ignore_errors=True)


def judged(name, shown):
    """[problem] of what the page showed for device NAME: [] when it is what the device earns."""
    text, tabs = shown["text"], shown["tabs"]
    refusal = "added twice" in text or "no script of this device" in text
    problems = []
    if name == "right":
        if "PART-DRAWN" not in text or "ABOUT-IT" not in text:
            problems.append("the part the section asks for is not drawn, or not given what the section gives it")
        if refusal:
            problems.append("a right device is refused")
    elif name == "twice":
        if "marked" not in text or "added twice" not in text:
            problems.append("the part added twice is not said in words with its name")
        if tabs:
            problems.append("the page is drawn though its parts are in doubt: %s" % tabs)
    elif name == "tabtwice":
        if "mine" not in text or "added twice" not in text:
            problems.append("the tab added twice is not said in words with its name")
        if tabs:
            problems.append("the page is drawn though its tabs are in doubt: %s" % tabs)
    elif name in ("main", "maintab", "system"):
        if "PART-DRAWN" not in text or "ABOUT-IT" not in text:
            problems.append("the part a section asks for is not drawn, or not given what the section gives it")
        if refusal:
            problems.append("a right device is refused")
        if "Plain section" not in text:
            problems.append("the plain section beside it is not drawn")
    elif name in ("missing", "mainmiss", "sysmiss"):
        if "gauge" not in text or "asks" not in text or "no script of this device" not in text:
            problems.append("the section that asks for a part nobody added does not say so with the names of both")
        if "Plain section" not in text:
            problems.append("the rest of the page is not drawn")
        if "PART-DRAWN" in text:
            problems.append("a part is drawn that nobody added")
    return problems


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=ROOT, help="a tree with page/ in it - a revision taken out of git shows what the page did before")
    ap.add_argument("--shots", default=None, help="not used by the verdict; the text each device showed is written here, one file a device")
    args = ap.parse_args()
    busy = taken([CHROME_PORT, STAND_PORT])
    if busy:
        print("page_parts: the port%s %s %s held" % ("s" if len(busy) > 1 else "", ", ".join(map(str, busy)), "are" if len(busy) > 1 else "is"))
        return 2
    tree = tempfile.mkdtemp(prefix="page_parts_tree_")
    os.symlink(os.path.join(os.path.abspath(args.root), "page"), os.path.join(tree, "page"))
    failed = 0
    try:
        for name, spec in DEVICES.items():
            folder = lay_device(tree, name, spec)
            shown = seen(folder, OPENING.get(name))
            if args.shots:
                os.makedirs(args.shots, exist_ok=True)
                open(os.path.join(args.shots, name + ".txt"), "w").write(shown["text"])
            problems = judged(name, shown)
            print("%-9s %s" % (name, "ok" if not problems else "FAIL: " + "; ".join(problems)))
            failed += bool(problems)
    finally:
        shutil.rmtree(tree, ignore_errors=True)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
