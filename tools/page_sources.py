"""What a device's page is made of, before it is laid in a kit: the engine's page (page/), the device's page folder
(devices/<device>/page/) and the menu and the table of texts that come of merging the two.

    uv run --no-project python tools/page_sources.py --menu <device>     # the merged menu.json
    uv run --no-project python tools/page_sources.py --texts <device>    # the merged texts.json

tools/page_kit.py lays the kit from these; the tools and the tests that need the menu of a device read it here, so
the two halves are merged in ONE place and the page, the kit and a test see the same words.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

MERGED = ("menu.json", "texts.json")
# what the device's page folder may hold besides its scripts
OWN = ("page.json",) + MERGED


def refuse(words):
    raise SystemExit("page_sources: " + words)


def frame_of(root=ROOT):
    return os.path.join(root, "page")


def addon_of(device, root=ROOT):
    """The device's page folder, or None when it has none."""
    folder = os.path.join(root, "devices", device, "page")
    return folder if os.path.isdir(folder) else None


def files_of(folder):
    """The files of FOLDER a browser could take - no Python, no hidden ones, no folders."""
    return sorted(n for n in os.listdir(folder)
                  if os.path.isfile(os.path.join(folder, n)) and not n.endswith(".py") and not n.startswith((".", "__")))


def scripts_of(device, addon, frame_names):
    """The device's scripts in the order page.json gives, after every refusal a page folder can earn: a file the
    engine has, a script page.json does not name, one it names twice or that is not there, a file of no kind."""
    spec = os.path.join(addon, "page.json")
    scripts = json.load(open(spec)).get("scripts", []) if os.path.isfile(spec) else []
    names = files_of(addon)
    for name in names:
        if name in frame_names and name not in MERGED:
            refuse("device %s carries %s, which the engine's page has: a device adds files to the page, it does not "
                   "replace the engine's" % (device, name))
    for name in scripts:
        if scripts.count(name) > 1:
            refuse("page.json of %s names %s twice" % (device, name))
        if name not in names:
            refuse("page.json of %s names %s, and %s has no such file" % (device, name, addon))
    for name in names:
        if name not in OWN and name not in scripts:
            refuse("%s of %s is in no script list: page.json names the scripts of the page, and a file nobody names "
                   "is a file nobody loads" % (name, device))
    return scripts


def sections_of(part):
    """PART as a list of sections - dictionaries that each carry an `id` - or None when it is not one: the lists the menu keeps its sections in."""
    if isinstance(part, list) and part and all(isinstance(one, dict) and "id" in one for one in part):
        return part
    return None


def merged(frame_part, addon_part, where):
    """The two halves of a menu laid into one. A key only one of them has is kept; where both have a dictionary they
    are merged one level down; a list of sections the device also has is the engine's with the device's sections after
    them - a section both have is merged by its `id`, so one the device says differently is refused -; a word both have
    but say differently is refused with its path."""
    if isinstance(frame_part, dict) and isinstance(addon_part, dict):
        out = dict(frame_part)
        for key, value in addon_part.items():
            out[key] = merged(frame_part[key], value, where + [str(key)]) if key in frame_part else value
        return out
    if sections_of(frame_part) and sections_of(addon_part):
        by_id = {one["id"]: at for at, one in enumerate(frame_part)}
        out = list(frame_part)
        for one in addon_part:
            if one["id"] in by_id:
                out[by_id[one["id"]]] = merged(frame_part[by_id[one["id"]]], one, where + [str(one["id"])])
            else:
                out.append(one)
        return out
    if frame_part != addon_part:
        refuse("the engine says %s and the device says %s at %s: one of them has to give way"
               % (json.dumps(frame_part, ensure_ascii=False), json.dumps(addon_part, ensure_ascii=False), ".".join(where)))
    return frame_part


def merged_file(device, name, root=ROOT):
    """The engine's NAME (menu.json or texts.json) with the device's laid over it; the engine's alone for a device
    that has no such file."""
    whole = json.load(open(os.path.join(frame_of(root), name), encoding="utf-8"))
    addon = addon_of(device, root)
    if addon and os.path.isfile(os.path.join(addon, name)):
        whole = merged(whole, json.load(open(os.path.join(addon, name), encoding="utf-8")), [name])
    return whole


def merged_menu(device, root=ROOT):
    return merged_file(device, "menu.json", root)


def merged_texts(device, root=ROOT):
    return merged_file(device, "texts.json", root)


def main():
    args = sys.argv[1:]
    if len(args) != 2 or args[0] not in ("--menu", "--texts"):
        raise SystemExit(__doc__)
    whole = merged_menu(args[1]) if args[0] == "--menu" else merged_texts(args[1])
    sys.stdout.write(json.dumps(whole, ensure_ascii=False))


if __name__ == "__main__":
    main()
