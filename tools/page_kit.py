"""Lays the kit a Rust device's vfs holds: build/<device>/kit, the folder upload_app.py put mirrors onto the board
through the recovery image.

    uv run --no-project python tools/page_kit.py <device>
    uv run --no-project python tools/page_kit.py --menu <device>     # the merged menu.json, as the kit carries it
    uv run --no-project python tools/page_kit.py --texts <device>    # the merged texts.json

The kit is the page and nothing else: /version.txt - the device's version, which upload_app requires - and under
/page ONE file, index.html, with its .gz copy beside it. The page carries everything it needs inside: the engine's
scripts and the device's, in the order the page loads them, each in a <script> of its own; the stylesheet in a <style>;
the icon as a data address; the menu and the table of texts as data elements (`kit-menu.json`, `kit-texts.json`).
Why one: a browser opens a connection to the board for every file it is told of - the six at once on 26 files -, and the
board's handlers and sockets are few (the recovery image with one connection takes the page for ~4 KB of
heap, ours with a pool of handlers for 18 KB). A device with a page folder of its own (devices/<device>/page/) carries
the engine's page with that folder on top, its menu.json and texts.json merged into the engine's. The .gz is made here
(`gzipped`): level 9, no time in the header, so the same source gives the same bytes and a kit laid twice does not
differ. `tools/upload_kit.py put` takes off the board whatever the kit has not got.

WHAT A DEVICE'S PAGE FOLDER MAY HOLD, and nothing else is accepted: page.json - {"scripts": [...]}, the files that go
into the page, in the order they stand -, those scripts, menu.json and texts.json. A file the engine's page has
already is the engine's: a device adds, it does not replace.
"""
import base64
import gzip
import json
import mimetypes
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

from page_sources import MERGED, addon_of, files_of, frame_of, merged_file, merged_menu, merged_texts, refuse, scripts_of  # noqa: E402,F401

FOLDER = "page"         # crates/web FOLDER, without its slash
DEVICE_JS = "device.js"
SLOT = "<!--device-->"  # where the engine's index.html wants the device's script
NAME = "{{device}}"     # where it wants the device's name


SCRIPT_REF = re.compile(r'<script src="([^"]+)"></script>')
STYLE_REF = re.compile(r'<link rel="stylesheet" href="([^"]+)">')
ICON_REF = re.compile(r'<link rel="icon" href="([^"]+)">')
LOCAL_REF = re.compile(r'\b(?:src|href)="(?!data:|https?:|#)([^"]+)"')


def table_element(name, value):
    """A table of the page as the data element the page reads it from. The angle bracket is written \\u003c: to JSON it is the same
    character, and inside the element no word of a table can end it."""
    text = json.dumps(value, ensure_ascii=False, separators=(",", ":")).replace("<", "\\u003c")
    return '<script type="application/json" id="kit-%s">%s</script>' % (name, text)


def plan(device, addon, root):
    """What the kit's page folder holds for DEVICE: {name: ("copy", path) | ("text", text)} - ONE file, index.html: the
    engine's page with the device's folder (ADDON, None for a device with none) on top, every script, the style, the
    icon and both tables inside it."""
    frame = frame_of(root)
    if not os.path.isfile(os.path.join(frame, "index.html")):
        refuse("no index.html in %s" % frame)
    frame_names = files_of(frame)
    scripts = scripts_of(device, addon, frame_names) if addon else []
    index = open(os.path.join(frame, "index.html"), encoding="utf-8").read().replace(NAME, device)
    if scripts:
        if SLOT not in index:
            refuse("the engine's index.html has no %s for the scripts of device %s" % (SLOT, device))
        index = index.replace(SLOT, "".join('<script src="%s"></script>\n' % name for name in scripts))
    else:
        index = index.replace(SLOT, "")

    def source_of(name):
        """The file NAME of the page, the device's own first when it is one of its scripts, as text; a name nobody has is a refusal."""
        for folder in (addon, frame):
            path = os.path.join(folder, name) if folder else None
            if path and os.path.isfile(path) and name not in MERGED:
                return path
        refuse("index.html of device %s names %s, and neither the engine's page nor the device's folder has it" % (device, name))

    def script(found):
        path = source_of(found.group(1))
        parses(path)
        text = open(path, encoding="utf-8").read()
        if re.search(r"</script", text, re.I):
            refuse("%s holds '</script', which would end its element inside the page: write it <\\/script" % found.group(1))
        return "<script>\n%s\n</script>" % text

    def style(found):
        text = open(source_of(found.group(1)), encoding="utf-8").read()
        if re.search(r"</style", text, re.I):
            refuse("%s holds '</style', which would end its element inside the page" % found.group(1))
        return "<style>\n%s\n</style>" % text

    def icon(found):
        path = source_of(found.group(1))
        kind = mimetypes.guess_type(path)[0] or "application/octet-stream"
        return '<link rel="icon" href="data:%s;base64,%s">' % (kind, base64.b64encode(open(path, "rb").read()).decode())

    tables = "".join(table_element(name, merged_file(device, name, root)) + "\n" for name in MERGED)
    # the tables stand before the first script: the page reads them when it starts, and everything it reads is already there
    first = SCRIPT_REF.search(index)
    index = index[:first.start()] + tables + index[first.start():] if first else index.replace("</body>", tables + "</body>")
    index = SCRIPT_REF.sub(script, index)
    index = STYLE_REF.sub(style, index)
    index = ICON_REF.sub(icon, index)
    left = LOCAL_REF.findall(index)
    if left:
        refuse("index.html of device %s still names files outside itself: %s" % (device, ", ".join(sorted(set(left)))))
    return {"index.html": ("text", index)}


def planned_of(device, root):
    """What the page folder of DEVICE holds, as {name: ("copy", path) | ("text", text)}, after every check a page earns: the engine's page with the device's folder on top when it has one, else
    the engine's page alone. A script that is not JavaScript or a folder that earns a refusal is an error."""
    planned = plan(device, addon_of(device, root), root)
    for name, (how, source) in planned.items():
        if how == "copy" and name.endswith(".js"):
            parses(source)
    return planned


def gzipped(source):
    """Lays SOURCE.gz beside a file a browser takes gzipped, and returns its path. The time is left out of the gzip header, so the same source gives the same bytes and a kit laid twice does not differ."""
    with open(source, "rb") as f:
        data = f.read()
    with open(source + ".gz", "wb") as f:
        f.write(gzip.compress(data, compresslevel=9, mtime=0))
    return source + ".gz"


def put(planned, folder, gz):
    """Writes PLANNED into FOLDER, each with its .gz beside it when GZ."""
    for name, (how, source) in planned.items():
        placed = os.path.join(folder, name)
        if how == "copy":
            shutil.copyfile(source, placed)
        else:
            with open(placed, "w", encoding="utf-8") as f:
                f.write(source)
        if gz:
            gzipped(placed)


def page_into(device, folder, root=ROOT):
    """Lays the page of DEVICE, as the kit carries it but without the .gz, into FOLDER - what a stand serves - and returns FOLDER."""
    planned = planned_of(device, root)
    shutil.rmtree(folder, ignore_errors=True)
    os.makedirs(folder)
    put(planned, folder, gz=False)
    return folder


def lay(device, root=ROOT):
    """Lays the kit of DEVICE and returns its folder - the engine's page with the device's folder on top when it has
    one -; a device or a page file that is not there, or a page folder that earns a refusal, is an error."""
    version_file = os.path.join(root, "devices", device, "version.txt")
    if not os.path.isfile(version_file):
        refuse("device %s has no version.txt at %s" % (device, version_file))
    planned = planned_of(device, root)
    kit = os.path.join(root, "build", device, "kit")
    shutil.rmtree(kit, ignore_errors=True)
    os.makedirs(os.path.join(kit, FOLDER))
    shutil.copyfile(version_file, os.path.join(kit, "version.txt"))
    put(planned, os.path.join(kit, FOLDER), gz=True)
    return kit


def parses(path):
    """Refuses a script that is not JavaScript. Nothing else in the build looks: to cargo a page file is a file,
    so a script with a syntax error goes into the kit, onto the board, and shows up as a page that never runs.
    The one this guard was written from was a backtick inside an HTML comment of an OWL template - the template is
    one backtick string, and the pair ended it."""
    try:
        done = subprocess.run(["node", "--check", path], capture_output=True, text=True)
    except FileNotFoundError:
        refuse("node is not on this machine, and without it %s cannot be checked. The kit is not laid: a "
               "script that does not parse would reach the board and the page would not run." % os.path.basename(path))
    if done.returncode != 0:
        refuse("%s is not JavaScript, nothing laid:\n%s" % (os.path.basename(path), done.stderr.strip()))


def main():
    args = sys.argv[1:]
    if len(args) == 2 and args[0] in ("--menu", "--texts"):
        whole = merged_menu(args[1]) if args[0] == "--menu" else merged_texts(args[1])
        sys.stdout.write(json.dumps(whole, ensure_ascii=False))
        return
    if len(args) != 1:
        raise SystemExit(__doc__)
    kit = lay(args[0])
    for base, _, files in sorted(os.walk(kit)):
        for name in sorted(files):
            path = os.path.join(base, name)
            print("%7d  /%s" % (os.path.getsize(path), os.path.relpath(path, kit)))


if __name__ == "__main__":
    main()
