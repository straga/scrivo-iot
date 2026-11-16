"""The names of the board's things and settings that the code of a device's page still writes.

    uv run --no-project python tools/names_in_page.py [--dictionary FILE] [--json]

The page draws by what the board's dictionary says, so no name of a thing, a setting or a card is written in its code: a name written there is
a place the page breaks when the board renames it, and a thing the board cannot say to the page. What it reads is the engine's page, page/: the
parts that know a device (the burner's scheme, its automaton) are in the device's page folder, and a device's folder is the device's to name its things in.
The page's own table of texts (`texts.json`) holds names as
keys, by design, and is no code, and it is looked at for one thing: it says words and how to show them, and no relation of the board's things (`relations_in_texts`).

WHAT IT LOOKS AT. In every .js file of the engine's page: each string literal - a `'...'`, a `"..."` or the text of a template - and
each identifier. A string is a NAME when it is one whole, or is a path (`/press/start`) with one for a segment: prose that happens to hold a word
a name is spelled as is no use of the name. An identifier is a name when it is one and has an underscore in it: a setting is `auto_hold_c` and
a variable is `feed`. What the board calls the rows every board has - the memory, the time it has run, the clock, the signal - is the contract's
own vocabulary (docs/CONTRACT_Device_Api.md) and stands in CONTRACT; the names of cards and choices' codes are checked as names too.

WHAT IT DOES NOT SEE. A name put together at run time (`"role_" + n`), a name written in a regular expression, and a name in a language it
does not read - the CSS, the markup of index.html. It reads the code, and the page it draws is judged by the tests that draw it with a dictionary
whose names are made up.

Exits 1 when a name is written, 0 when none is, and prints each with its file and line.
"""
import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# The cards every board has, and in the system card what stands in it - the memory, the time it has run, the clock, the console, the network -:
# the contract's own vocabulary (docs/CONTRACT_Device_Api.md), not a device's things. The row each setting has in the settings card is the
# setting's, and its name is checked.
CONTRACT_CARDS = {"system", "settings", "other"}
CONTRACT_ROW_CARDS = {"system"}
# What a page asks a board by: the contract's endpoints, whose names a thing may be spelled like.
ENDPOINTS = {"dictionary", "state", "events", "settings", "restart", "down", "hold", "curve", "on", "off", "press", "toggle", "console"}
# A name this short is not told from a word of the code that is spelled the same: the part `at` of a row is not the field `at` of the page's own.
SHORTEST = 3


def names_of(dictionary):
    """Every name the board's dictionary gives - things, parts, settings, cards and the codes of a setting's choices - but the contract's cards and what stands in them - a setting or a choice spelled like one of those rows is left out too - and the ones too short to tell from a word of the code."""
    names = set(dictionary.get("cards", []))
    contract = {t["name"] for t in dictionary.get("things", []) if t["card"] in CONTRACT_ROW_CARDS}
    for t in dictionary.get("things", []):
        if t["card"] in CONTRACT_ROW_CARDS:
            continue
        names.add(t["name"])
        names.add(t["card"])
    for s in dictionary.get("settings", []):
        names.add(s["name"])
        names.update(c["value"] for c in s.get("choices", []))
    machine = dictionary.get("machine") or {}
    for key in ("card", "fixed", "stage", "says", "cause"):
        if machine.get(key):
            names.add(machine[key])
    names.update(machine.get("press", []))
    names.update(machine.get("aside", []))
    for section in machine.get("sections", []):
        names.add(section["name"])
    return {n for n in names - CONTRACT_CARDS - contract if len(n) >= SHORTEST}


# What the table of texts may say of a card Now row and of a setting's line: the page's word for it and the page's choices of how to show it. Anything
# else in them is a relation of the board's things - which rows, which settings, which numbers - and is the board's to say.
NOW_MAY_SAY = {"as", "spec"}
LINE_MAY_SAY = {"words", "late", "verb", "way", "miss", "near"}


def relations_in_texts(texts):
    """The places the page's table of texts (`texts.json`) says how the board's things hang together, when it should say words and what to show:
    [path]. A name of the board stands in it as a key of the words for that thing and nowhere else; the card Now is a table of those keys, and a
    setting's line carries words and constants of showing, and no name of a setting or a number."""
    found = []
    now = texts.get("now")
    if isinstance(now, list):
        found.append("now")
    elif isinstance(now, dict):
        found += ["now.%s.%s" % (key, field) for key, entry in now.items() for field in entry if field not in NOW_MAY_SAY]
    if "numbers" in texts:
        found.append("numbers")
    for name, words in (texts.get("settings") or {}).items():
        found += ["settings.%s.line.%s" % (name, field) for field in (words.get("line") or {}) if field not in LINE_MAY_SAY]
    return found


def tokens(source):
    """The strings and identifiers of JS SOURCE, each with its line: [(kind, text, line)], kind 'string' or 'ident'. Comments and regular
    expressions are skipped, and a template literal is read as its static text and the code of its `${}` places."""
    out = []
    i, line, n = 0, 1, len(source)
    previous = ""
    while i < n:
        c = source[i]
        if c == "\n":
            line += 1
            i += 1
        elif c.isspace():
            i += 1
        elif source.startswith("//", i):
            while i < n and source[i] != "\n":
                i += 1
        elif source.startswith("/*", i):
            end = source.find("*/", i + 2)
            end = n if end < 0 else end + 2
            line += source.count("\n", i, end)
            i = end
        elif c in "'\"":
            j = i + 1
            while j < n and source[j] != c:
                j += 2 if source[j] == "\\" else 1
            out.append(("string", source[i + 1 : j], line))
            i = j + 1
        elif c == "`":
            j = i + 1
            static, start = [], j
            depth = 0
            while j < n:
                if source[j] == "\\":
                    j += 2
                    continue
                if source.startswith("${", j) and depth == 0:
                    static.append(source[start:j])
                    depth = 1
                    j += 2
                    inner = j
                    while j < n and depth:
                        if source[j] == "{":
                            depth += 1
                        elif source[j] == "}":
                            depth -= 1
                        j += 1
                    out.extend((k, t, line + source.count("\n", i, inner) + l - 1) for k, t, l in tokens(source[inner : j - 1]))
                    start = j
                    continue
                if source[j] == "`":
                    break
                j += 1
            static.append(source[start:j])
            text = "".join(static)
            out.append(("string", text, line))
            line += source.count("\n", i, j)
            i = j + 1
        elif c == "/" and previous in ("", "(", ",", "=", ":", "[", "!", "&", "|", "?", "{", "}", ";", "return", "=>"):
            # a regular expression: its names are not seen
            j = i + 1
            in_class = False
            while j < n and (source[j] != "/" or in_class):
                if source[j] == "\\":
                    j += 1
                elif source[j] == "[":
                    in_class = True
                elif source[j] == "]":
                    in_class = False
                j += 1
            i = j + 1
            previous = "regex"
            continue
        elif c.isalpha() or c in "_$":
            j = i
            while j < n and (source[j].isalnum() or source[j] in "_$"):
                j += 1
            out.append(("ident", source[i:j], line))
            previous = source[i:j]
            i = j
            continue
        else:
            previous = c
            i += 1
            continue
        previous = "string"
    return out


def is_name_string(text, names):
    """Whether the string TEXT is a name: one whole, or a path with one for a segment."""
    if text in names:
        return text
    if text.startswith("/"):
        for segment in text.split("/"):
            if segment in names and segment not in ENDPOINTS:
                return segment
    return None


def scan(source, names):
    """The names SOURCE writes: [(name, line, how)]."""
    found = []
    for kind, text, line in tokens(source):
        if kind == "string":
            hit = is_name_string(text.strip(), names)
            if hit:
                found.append((hit, line, "string"))
        elif text in names and "_" in text:
            found.append((text, line, "identifier"))
    return found


def report_of(page, names):
    """What the page folder PAGE writes of NAMES: {file: [(name, line, how)]} - every script of it but a minified library, and its table of texts, which may say
    no relation of the board's things."""
    report = {}
    for file in sorted(os.listdir(page)):
        if not file.endswith(".js") or file.endswith(".min.js"):
            continue
        found = scan(open(os.path.join(page, file), encoding="utf-8").read(), names)
        if found:
            report[file] = found
    with open(os.path.join(page, "texts.json"), encoding="utf-8") as f:
        related = relations_in_texts(json.load(f))
    if related:
        report["texts.json"] = [(path, 0, "relation of the board's things") for path in related]
    return report


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dictionary", default=None, help="a dictionary file instead of the one the crate makes")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    if args.dictionary:
        dictionary = json.load(open(args.dictionary))
    else:
        sys.path.insert(0, HERE)
        from page_stand import dictionary_of

        dictionary = dictionary_of(ROOT)
    names = names_of(dictionary)
    report = report_of(os.path.join(ROOT, "page"), names)
    if args.json:
        print(json.dumps({f: [list(x) for x in v] for f, v in report.items()}, indent=1))
    else:
        for file, found in report.items():
            for name, line, how in found:
                print("%s:%d  %s  (%s)" % (file, line, name, how))
        total = sum(len(v) for v in report.values())
        print("%d names of the board written in %d files; %d names known to the dictionary" % (total, len(report), len(names)))
    sys.exit(1 if report else 0)


if __name__ == "__main__":
    main()
