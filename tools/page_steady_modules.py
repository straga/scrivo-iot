"""The cards of the new burner's modules and the card Now stand still while each module goes through every state it has (REF_Device_Page_Look, rule 7).

    uv run --no-project --with websocket-client --with pyyaml python tools/page_steady_modules.py [--module flame|water_and_flue|draught|relays|all] [--shots DIR]

page_steady.py holds the first burner's cards to it through its automaton's stages; the new burner has no automaton yet, so this holds the cards of the modules laid through, and
Now where they stand in it, through the moments of each module's scenario (tools/stand_scenarios): for the flame dark, each threshold touched exactly, weak, steady, the pin silent,
dark again; for the water and the flue the sockets warming, a socket read as nothing and a flue read as nothing, the bus silent, the thermocouple open, the chip saying nothing, and
back. The frames are the new burner's own (`cargo run -p burner_api --example burner_frames`); the stand, the browser and the three verdicts are page_steady's: rows-steady,
heights-steady, buttons-steady, each printed PASS or FAIL - and a fourth, rule 7's: state-is-one-word, every state of these cards is a word, a dash, or a number with its unit.
The note under the flame's thresholds is not in these moments on purpose: it speaks only when the two are crossed, and a card that grows then says what is wrong.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

import page_steady  # noqa: E402
from page_sources import merged_menu, merged_texts  # noqa: E402

# (name, second of the scenario): every state the module has and the edges between them. The cards held to the one-word rule are the modules' own and Now.
MODULES = {
    "flame": {
        "scenario": "flame.json",
        "views": [("Modules", ["Now"]), ("Modules>flame", ["card 1"])],
        "moments": [("out", 5), ("at-seen", 20), ("weak", 35), ("at-steady", 50), ("steady", 65), ("not-read", 95), ("out-again", 125)],
    },
    "water_and_flue": {
        "scenario": "modules.json",
        "views": [("Modules", ["Now"]), ("Modules>temperatures", ["card 1"]), ("Modules>flue", ["card 1"])],
        "moments": [("normal", 5), ("warming", 45), ("flue-not-read", 76), ("boiler-not-read", 95), ("recovered", 105), ("bus-silent", 114),
                    ("flue-open", 122), ("flue-silent", 127)],
    },
    "relays": {
        "scenario": "modules.json",
        # the relays are the hands of the Service: their rows stand in the section Relays of the System tab
        "views": [("Modules", ["Now"]), ("System", ["Relays"])],
        "moments": [("all-off", 5), ("auger-on", 15), ("igniter-too", 25), ("pump-too", 35), ("igniter-off", 45), ("power-on", 55), ("auger-expired", 75), ("auger-again", 105)],
    },
    "draught": {
        "scenario": "modules.json",
        "views": [("Modules", ["Now"]), ("Modules>draught", ["card 1"])],
        "moments": [("under", 5), ("number", 30), ("over", 58), ("not-read", 85), ("missing", 95), ("back", 105)],
    },
    "fan": {
        "scenario": "modules.json",
        "views": [("Modules", ["Now"]), ("Modules>fan", ["card 1"])],
        "moments": [("off", 5), ("holding", 35), ("purging", 58), ("held-again", 75), ("not-read", 85), ("kicking", 95), ("by-hand", 112), ("stopped", 122)],
    },
}

# The automaton: the whole run of tools/stand_scenarios/automaton.json - stopped, purged, lighting on its first attempt, rising, the auger turning and resting at work, the feed banned, smouldering, burning off,
# stopped again, lighting a dark pot, burning off because of the water, the alarm, the fault. The card Now stands on the list of modules and is held to the one-word rule there; the readings of the automaton and the drawing
# above it are the Scheme tab's, held by `scheme` below. The settings of "How it runs" are the card's: what is turned every day stands out of the fold (the mode and, in the mode the burner is set to, its numbers), every other
# number stands in it.
AUTOMATON_MOMENTS = [("stopped", 1), ("purging", 10), ("lighting", 60), ("rising", 120), ("feeding", 200), ("resting", 201), ("feed-banned", 340), ("smouldering", 500),
                     ("burning-off", 700), ("stopped-again", 800), ("dark-pot", 1000), ("burning-off-because", 1380), ("alarm", 1400), ("fault", 1460)]
MODULES["automaton"] = {"scenario": "automaton.json", "views": [("Modules", ["Now"])], "moments": AUTOMATON_MOMENTS}
# THE MODULES' PAGES: each module's page holds its readings, its curves, its controls and its settings, every setting stands on the one page the menu puts it on, and the page stands still through the run
MODULES["tree"] = {"scenario": "automaton.json", "tab": "tree", "views": [], "moments": [("stopped", 1), ("lighting", 60), ("feeding", 200), ("feed-banned", 340), ("burning-off", 700), ("fault", 1460)]}
MODULES["scheme"] = {
    "scenario": "automaton.json",
    "tab": "Scheme",
    "views": [("Scheme", ["Automaton"])],
    "module": "automaton",
    "line": True,
    "moments": AUTOMATON_MOMENTS,
}

# A state of a row is a word, a dash, or a number with its unit - `weak`, `-`, `61.5 °C`, `3150 mV` - and in Now the word with the number that stands beside it: `out · 3.15 V`.
# A time is a number with its units, however many: `45 s`, `4 min 22 s`, `1 h 5 min` (REF_Device_Page_Look, rule 7).
# THE SHAPE OF A WORD, NOT ITS LETTERS: a letter is any letter of any alphabet, and a unit of time is a word after a number - the words are the menu's and a person changes them.
_LETTER = r"[^\W\d_]"
_TIME = r"\d+ %s+( \d+ %s+)*" % (_LETTER, _LETTER)
_ONE = r"(-|%s[\w-]*|%s|-?\d+(\.\d+)?( \S+)?)" % (_LETTER, _TIME)
STATE = re.compile(r"^%s( \u00b7 (%s|-?\d+(\.\d+)? \S+))?$" % (_ONE, _TIME))


def one_word(seen, cards):
    """The verdict on the words of the rows of CARDS (rule 7): SEEN is [(moment, said)], SAID what the browser said of the tab - each card with its rows' names and states."""
    out = []
    for moment, said in seen:
        # the page writes a heading in lower case and its style capitalises it
        by_name = {c["card"].lower(): c for c in said["cards"]}
        for name in cards:
            card = by_name.get(name.lower())
            if card is None:
                out.append("no card '%s' in %s" % (name, moment))
                continue
            for row, state in card.get("says", []):
                if not STATE.match(state):
                    out.append("'%s' of '%s' says '%s' in %s" % (row, name, state, moment))
    shown = "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "")
    rows = sum(len(c.get("says", [])) for _, said in seen for c in said["cards"] if c["card"].lower() in [x.lower() for x in cards])
    return ("state-is-one-word", not out, shown if out else "%d states of %s, each a word, a dash or a number with its unit" % (rows, ", ".join(cards)))


# What the automaton does, said as a code: the stage's word, then the attempt (`lighting \u00b7 2`) or the cause of a failure - a word with the number it carries, if it carries one (`no-light 3`, `too-hot 97 \u00b0C`).
_CODE = r"%s+(-%s+)*" % (_LETTER, _LETTER)
# In smouldering the numbers of it follow the word: the time gone of the whole, then the feed and what is left - numbers with their small words, never a sentence.
_SMOULDER = r"\d+:\d\d \S+ \d+:\d\d \u00b7 (\S+ )?\d+ \S+ / \d+ \S+ \u00b7 (\S+ )*\d+:\d\d( \S+)*"
DOING = re.compile(r"^%s( \u00b7 (\d+|%s( \d+( \S+)?)?|%s))?$" % (_CODE, _CODE, _SMOULDER))


def doing_is_code(seen, card, cells):
    """The verdict on the line of what the automaton does (rule 7), as a person reads it: SEEN is [(moment, said)], SAID what the browser said of the tab, CARD the automaton's card, CELLS how many cells the strip has -
    the line is the one under the heading and the strip."""
    out = []
    for moment, said in seen:
        text = next((c.get("text", "") for c in said["cards"] if c["card"].lower() == card.lower()), None)
        if text is None:
            out.append("no card '%s' in %s" % (card, moment))
            continue
        # the browser puts a blank line between blocks of the card: only the lines with words count
        rows = [row for row in text.split("\n") if row.strip()]
        line = rows[1 + cells] if len(rows) > 1 + cells else None
        if not line:
            out.append("no line of what the automaton does in %s" % moment)
        elif not DOING.match(line):
            out.append("'%s' is no code, in %s" % (line, moment))
    shown = "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "")
    return ("doing-is-code", not out, shown if out else "%d moments, the line a stage word with its attempt or its cause as a code" % len(seen))


def menu_of(module):
    """What the device's menu says of MODULE."""
    return in_language(merged_menu("burner"), page_steady.LANG)["modules"][module]


def title_of(card):
    """The name CARD is looked for under on the page: the menu's title of the module of that name - the page writes it, and a person may change it -, and the page's own cards (Now, How it runs) as they are."""
    if card.startswith("card "):
        return card
    if card in ("Now", "How it runs"):
        ui = in_language(merged_menu("burner"), page_steady.LANG)["ui"]
        return ui["now" if card == "Now" else "howItRuns"]["en"]
    return menu_of(card.lower())["card"]["title"]["en"]


def scheme_words(menu):
    """WHAT THE SCHEME TAB IS LOOKED FOR UNDER, all of it the menu's words as the page draws them: the button that runs the burner - the two words it changes between -, and the feed's likewise, how an organ's action begins, the word the pot's box carries, the flame's words in the pot, the cells of the strip of stages, and the title of the card
    of the automaton. A menu worded otherwise is looked for under the other words, so a check never fails on a word a person changed."""
    en = lambda node: node["en"]
    raised = lambda label: label[:1].upper() + label[1:]
    automaton, thermostat, scheme = menu["modules"]["automaton"], menu["modules"]["thermostat"], menu["scheme"]
    return {
        # one button runs the burner - Stop where the switch is on, Start where it is off -, and the feed's has the two words it changes between
        "controls": [(en(scheme["run"]["start"]), en(scheme["run"]["stop"])), (en(scheme["feed"]["ban"]), en(scheme["feed"]["allow"]))],
        "starts": [en(scheme["actions"]["burning"])],
        "pot": en(scheme["parts"]["pot"]),
        "flame": [en(said) for code, said in menu["modules"]["flame"]["rows"]["flame"]["says"] if not code.startswith("unknown")],
        "stages": [en(cell[1]) for cell in automaton["rows"]["stage"]["strip"]["cells"]],
        "automaton": en(automaton["card"]["title"]),
        "title": en(scheme["title"]),
    }


def pin_volts(frames, address, second):
    """What the drawing must say of the flame's pin at SECOND: its volts to the hundredth, or a dash when the pin reads nothing or the frames never said it. FRAMES are the scenario's, each
    giving the values that changed, so the value at a second is the last one given up to it."""
    seen = None
    for frame in frames:
        if frame["at"] > second:
            break
        for at, value in frame["values"]:
            if at == address:
                seen = value
    return "%.2f %s" % (seen / 1000, page_steady.VOLT) if isinstance(seen, (int, float)) else "-"


def scheme_verdicts(seen, told, frames, dictionary, words):
    """The verdicts on the Scheme tab beyond the steadiness every tab is held to: the controls stand whole on a phone's first screen, what each organ does touches no line of the drawing, the pot holds its
    words, the flame's volts stand on the drawing. SEEN is [(moment, said)], TOLD [(moment, said)] with the browser's lines and pot, FRAMES the scenario's."""
    pin = next(i for i, t in enumerate(dictionary["things"]) if t["card"] == "flame" and t["name"] == "pin" and not t.get("part_of"))
    drawn = [(moment, said["lines"]) for moment, said in told]
    pots = [(moment, said["pot"]) for moment, said in told]
    volts = [(moment, said["lines"], pin_volts(frames, pin, int(moment.rsplit("@", 1)[1]))) for moment, said in told]
    labels = [(moment, said["labels"]) for moment, said in told]
    first = seen[0][1]
    return [page_steady.in_view(seen, controls=words["controls"]), page_steady.heads([(m, s, None) for m, s in seen], words["automaton"]),
            page_steady.clear(drawn), page_steady.labels_clear(labels), page_steady.tabs_one_row(first["buttons"], [page_steady.TAB_WORDS[k] for k in ("Scheme", "Modules", "System", "Help")]),
            page_steady.flame_number(volts), page_steady.crowded(pots)]


def standing(section):
    """The settings of SECTION that stand on the page by a label of their own: all but the second of a pair, which stands on the line of the first, under the pair's word."""
    seconds = {pair["of"][1] for pair in section.get("pairs", [])}
    return [n for n in section.get("settings", []) if n not in seconds]


def shown(section, name, label):
    """The label the page draws for the setting NAME in SECTION: the first of a pair is called what the pair says where the setting's own label is longer than the column of the first fields, a single row what the section's `labels` say; else the setting's own."""
    for pair in section.get("pairs", []):
        if pair["of"][0] == name and pair.get("first"):
            return pair["first"]["en"]
    named = (section.get("labels") or {}).get(name)
    return named["en"] if named else label[name]


def tree_plan(menu, texts, dictionary):
    """WHAT EACH PAGE OF A MODULE SHOWS, in the menu's words: for each page, the labels of its settings - the menu's for a module's, the page's texts' for the board's -, which of them a choice of another setting governs
    (the board's `shows`: they stand only while that choice is the one set, so they may be absent), and the readings its cards are made of. Nothing in it is written here."""
    label = {}
    for module in menu["modules"].values():
        for name, one in module.get("settings", {}).items():
            label[name] = one["label"]["en"]
    for name, one in texts.get("settings", {}).items():
        label.setdefault(name, one["label"])
    governed = {name for one in dictionary["settings"] for choice in one.get("choices", []) for name in choice.get("shows", [])}
    places = [(page["id"], page["title"]["en"], page["sections"]) for page in menu["tree"]["modules"]["pages"]] + [("system", menu["ui"]["system"]["en"], menu["tree"]["system"]["sections"])]
    return {id_: {"title": title,
                  "labels": [shown(section, n, label) for section in sections for n in standing(section)],
                  "optional": [shown(section, n, label) for section in sections for n in standing(section) if n in governed],
                  "cards": [card for section in sections for card in section.get("cards", [])]}
            for id_, title, sections in places}


def view_of(page):
    """The path of the view that shows PAGE of tree_plan: the system tab, or a module's page on the modules tab."""
    return "System" if page == "system" else "Modules>%s" % page


def in_language(node, lang):
    """NODE with every `{en, ru}` word of it put in LANG - under the key `en`, where the checks below read it: a check that reads the menu's words by `en` reads the page's words, whichever language the page is asked in.
A word with no Russian is its English, as the page says it."""
    if isinstance(node, list):
        return [in_language(x, lang) for x in node]
    if isinstance(node, dict):
        if node and set(node) == {"en", "ru"}:
            return {"en": node.get(lang) or node["en"], "ru": node["ru"]}
        return {k: in_language(v, lang) for k, v in node.items()}
    return node


def texts_in(texts, lang):
    """The table of texts as the page lays it for LANG: its Russian over it, the rest as it is."""
    if lang == "en" or lang not in texts:
        return texts
    over = texts[lang]

    def lay(base, top):
        if not isinstance(base, dict) or not isinstance(top, dict):
            return top
        return dict(base, **{k: lay(base.get(k), v) for k, v in top.items()})
    return lay({k: v for k, v in texts.items() if k != lang}, over)


def lines_of(card):
    return [line.strip() for line in card.get("text", "").split("\n") if line.strip()]


def tree_places(said, plan):
    """The verdict that every setting stands on the page of the module (or the system tab) the menu puts it on, and on no other: SAID is what the browser said of each view (`Modules>fan`, `System`, ...), PLAN what tree_plan says. A label is a whole line of a
    card's text; a setting a choice governs may be absent."""
    out = []
    holders = {}
    for page, one in plan.items():
        for card in said[view_of(page)]["cards"]:
            for line in lines_of(card):
                holders.setdefault(line, set()).add(page)
    for page, one in plan.items():
        have = {line for card in said[view_of(page)]["cards"] for line in lines_of(card)}
        for label in one["labels"]:
            if label not in have and label not in one["optional"]:
                out.append("'%s' is not on the page of '%s'" % (label, one["title"]))
            elsewhere = sorted(plan[other]["title"] for other in holders.get(label, set()) if other != page)
            if elsewhere and label in have:
                out.append("'%s' stands on '%s' and on '%s'" % (label, one["title"], "' and '".join(elsewhere)))
    count = sum(len(one["labels"]) for one in plan.values())
    return ("tree-places", not out, "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "") if out else
            "%d settings, each on its module's page and on no other" % count)


def module_pages(said, menu, dictionary):
    """The verdict that the page of a module holds what the module IS: SAID is what the browser said of each page. Every reading the menu words for the cards of the page is on it (a row of the sockets that has no role is not drawn), each row
    of those cards that has curves has its picture, each that takes a number from a slider has its slider. The settings are tree_places'."""
    out = []
    held = 0
    # a row the menu places in a section of its own (the fan's hands, in the Service) or says a part of the page draws (the scheme's button) stands on no module's page: the Service holds the first, the scheme the second
    placed = {name for section in menu["tree"]["system"]["sections"] for name in section.get("actions", [])}
    for page in menu["tree"]["modules"]["pages"]:
        cards = [card for section in page["sections"] for card in section.get("cards", [])]
        if not cards:
            continue
        view = said["Modules>%s" % page["id"]]
        have = {line for card in view["cards"] for line in lines_of(card)} | {row for card in view["cards"] for row in card["rows"]}
        for name in cards:
            for key, row in menu["modules"][name]["rows"].items():
                if "." in key or row.get("hide") or row.get("apart") or "%s/%s" % (name, key) in placed or re.match(r"^ntc_\d$", key) or key == "none_assigned":
                    continue
                words = row["label"]["en"]
                words = words[:1].upper() + words[1:]
                held += 1
                if words not in have:
                    out.append("'%s' is not on the page of '%s'" % (words, page["title"]["en"]))
        things = [t for t in dictionary["things"] if t["card"] in cards and not t.get("part_of") and "%s/%s" % (t["card"], t["name"]) not in placed]
        # a socket of the temperature module that has no role is not drawn, and with it its picture: the sockets are one picture at least, however many roles are set
        sockets = [t for t in things if t.get("curves") and re.match(r"^ntc_\d$", t["name"])]
        want_curves = sum(1 for t in things if t.get("curves") and t not in sockets) + (1 if sockets else 0)
        want_sliders = sum(1 for t in things if t.get("range"))
        curves = sum(card.get("curves", 0) for card in view["cards"])
        sliders = sum(card.get("sliders", 0) for card in view["cards"])
        if curves < want_curves:
            out.append("'%s' draws %d pictures of the %d rows that have curves" % (page["title"]["en"], curves, want_curves))
        if sliders != want_sliders:
            out.append("'%s' draws %d sliders where the board says %d rows are held by one" % (page["title"]["en"], sliders, want_sliders))
    return ("module-pages", not out, "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "") if out else
            "%d readings, and every curve and slider the board says, on the pages of their modules" % held)


def steps_table(card, section, held):
    """The verdict that the table of power steps is drawn as the menu says and holds what the board holds: CARD is the section's card as the browser said it (`steps`: the rows it drew, each with its step, whether
    it is marked as the one the burner burns on and what its three cells hold), SECTION the menu's table, HELD the board's value of each setting by name. A row for each step the menu lists, in its order; each cell the board's number;
    the marked row the one the board's level names - and none, when it names a level the table has no row for."""
    if card is None:
        return ("steps-table", False, "no section '%s' on the page" % section["title"]["en"])
    table = section["table"]
    rows = card.get("steps", [])
    out = []
    if [r["step"] for r in rows] != [r["step"] for r in table["rows"]]:
        out.append("rows %s, the menu lists %s" % ([r["step"] for r in rows], [r["step"] for r in table["rows"]]))
    for drawn in rows:
        row = next((r for r in table["rows"] if r["step"] == drawn["step"]), None)
        if row is None:
            continue
        if len(drawn["values"]) != len(row["settings"]):
            out.append("row %s holds %d cells, the menu lists %d" % (drawn["step"], len(drawn["values"]), len(row["settings"])))
            continue
        for name, shown in zip(row["settings"], drawn["values"]):
            if shown != held.get(name):
                out.append("row %s: %s shows '%s', the board holds '%s'" % (drawn["step"], name, shown, held.get(name)))
    chosen = [r["step"] for r in rows if r["chosen"]]
    want = [held[table["chosen"]]] if held.get(table["chosen"]) in [r["step"] for r in table["rows"]] else []
    if chosen != want:
        out.append("the marked rows are %s, the board holds the level %s" % (chosen, held.get(table["chosen"])))
    return ("steps-table", not out, "; ".join(out[:6]) if out else "%d rows of %d numbers, each the board's, and the row the burner burns on marked" % (len(rows), len(table["rows"][0]["settings"])))


def reference_complete(help_tab, menu):
    """The verdict that the reference tab says every item of the menu: HELP_TAB is what the browser said of the tab, MENU the menu in the language asked. Every entry of `help` - its name and its line, whole - stands in the text of
    some card of the tab, and the tab has a card for every group. A line the page left out, or cut, is named by its key."""
    entries = menu["help"]["entries"]
    text = "\n".join(card.get("text", "") for card in help_tab["cards"])
    flat = " ".join(text.split())
    out = ["%s: %s" % (key, "name" if " ".join(e["name"]["en"].split()) not in flat else "line") for key, e in entries.items()
           if " ".join(e["name"]["en"].split()) not in flat or " ".join(e["text"]["en"].split()) not in flat]
    return ("reference-complete", not out, "; ".join(out[:6]) + (" (%d more)" % (len(out) - 6) if len(out) > 6 else "") if out else "%d items of the menu, each with its name and its line, in %d cards" % (len(entries), len(help_tab["cards"])))


def run_tree(name, how, frames_file, shots, page, menu):
    """The modules' moments: every page of a module stands still through the automaton's run, each setting stands on the page the menu puts it on and on no other, each page holds what the module is, the power steps are the
    board's, and the reference says every item of the menu."""
    from page_stand import dictionary_of
    texts = texts_in(merged_texts("burner"), page_steady.LANG)
    dictionary = dictionary_of(ROOT, "burner")
    plan = tree_plan(menu, texts, dictionary)
    pages = ["Modules>%s" % p["id"] for p in menu["tree"]["modules"]["pages"]]
    page_steady.TABS = ["Scheme", "Modules"] + pages + ["System", "Help"]
    seen = {tab: [] for tab in page_steady.TABS}
    for moment, second in how["moments"]:
        said = page_steady.look(frames_file, second, shots, "%s_%s" % (name, moment), page)
        for tab in page_steady.TABS:
            seen[tab].append(("%s @%d" % (moment, second), said[tab]))
    first = how["moments"][0]
    opened = page_steady.look(frames_file, first[1], shots, "%s_%s_open" % (name, first[0]), page)
    verdicts = []
    for tab in ["Modules"] + pages + ["System"]:
        verdicts += [("%s/%s" % (tab.replace("Modules>", "").lower(), check), ok, said) for check, ok, said in page_steady.judge(seen[tab])]
    verdicts.append(tree_places(opened, plan))
    verdicts.append(module_pages(opened, menu, dictionary))
    verdicts.append(reference_complete(opened["Help"], menu))
    verdicts += [("help/%s" % check, ok, said) for check, ok, said in page_steady.judge(seen["Help"])]
    held = {name: one.get("value") for name, one in opened["settings"].items()} if isinstance(opened["settings"], dict) else {one["name"]: one.get("value") for one in opened["settings"]}
    for section in menu["tree"]["modules"]["pages"]:
        for part in section["sections"]:
            if "table" in part:
                view = opened["Modules>%s" % section["id"]]
                card = next((c for c in view["cards"] if c["card"].lower() == part["title"]["en"].lower()), None)
                verdicts.append(steps_table(card, part, held))
    verdicts += [("main/%s" % check, ok, said) for check, ok, said in page_steady.judge(seen["Scheme"])]
    ok = True
    for check, passed, words in verdicts:
        print("%-15s %-16s %-24s %s  %s" % (name, "Modules", check, "PASS" if passed else "FAIL", words))
        ok = ok and passed
    return ok


def run(name, shots, page):
    """One module's moments: the verdicts printed, and whether every one passed."""
    how = MODULES[name]
    scenario = os.path.join(HERE, "stand_scenarios", how["scenario"])
    out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", scenario], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("page_steady_modules: the frames were not made:\n" + out.stderr[-1500:])
    work = tempfile.mkdtemp(prefix="page_steady_modules_")
    frames_file = os.path.join(work, "frames.json")
    with open(frames_file, "w") as f:
        f.write(out.stdout)
    if shots:
        os.makedirs(shots, exist_ok=True)
    tab = how.get("tab", "Modules")
    views = how["views"]
    page_steady.TABS = [path for path, _ in views] if views else [tab]
    seen, told, verdicts = {path: [] for path in page_steady.TABS}, [], []
    menu = in_language(merged_menu("burner"), page_steady.LANG)
    page_steady.TAB_WORDS = {"Scheme": menu["ui"]["scheme"]["en"], "Modules": menu["ui"]["modules"]["en"], "System": menu["ui"]["system"]["en"], "Help": menu["ui"]["reference"]["en"],
                             **{one["id"]: one["title"]["en"] for one in menu["tree"]["modules"]["pages"]}}
    page_steady.SAVE = menu["ui"]["save"]["en"]
    words = scheme_words(menu)
    page_steady.VOLT = menu["units"]["V"]["en"]
    page_steady.WORST_FLAME = "3.30 %s" % page_steady.VOLT
    page_steady.WORDS.update({k: words[k] for k in ("starts", "pot", "flame", "stages", "automaton")})
    page_steady.WORST_IGNITER = "%s 59 %s 59 %s" % (words["starts"][0], menu["units"]["min"]["en"], menu["units"]["s"]["en"])
    page_steady.WORST_FLAME_WORD = max(words["flame"], key=len)
    try:
        if tab == "tree":
            return run_tree(name, how, frames_file, shots, page, menu)
        for moment, second in how["moments"]:
            said = page_steady.look(frames_file, second, shots, "%s_%s" % (name, moment), page)
            for path in page_steady.TABS:
                seen[path].append(("%s @%d" % (moment, second), said[path]))
            told.append(("%s @%d" % (moment, second), said))
        for path, cards in views:
            names = [title_of(c) for c in cards]
            verdicts += [("%s/%s" % (path, check) if len(views) > 1 else check, ok, said) for check, ok, said in page_steady.judge(seen[path]) + [one_word(seen[path], names)]]
        if how.get("line"):
            strip = menu_of(how["module"])["rows"]["stage"]["strip"]["cells"]
            verdicts.append(doing_is_code(seen[tab], title_of("Automaton"), len(strip)))
        if tab == "Scheme":
            from page_stand import dictionary_of
            verdicts += scheme_verdicts(seen[tab], told, json.load(open(frames_file))["frames"], dictionary_of(ROOT, "burner"), words)
    finally:
        os.remove(frames_file)
        os.rmdir(work)
    ok = True
    for check, passed, words in verdicts:
        print("%-15s %-16s %-17s %s  %s" % (name, tab, check, "PASS" if passed else "FAIL", words))
        ok = ok and passed
    return ok


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--module", default="all", help="flame, water_and_flue, draught, relays, fan, automaton, scheme, tree or all")
    ap.add_argument("--shots", default=None)
    ap.add_argument("--lang", default="en", choices=["en", "ru"], help="the language the page is asked in: the words the checks read are the menu's in it")
    ap.add_argument("--page", default=None, help="the page files to measure instead of the device's own - a revision taken out of git")
    args = ap.parse_args()
    page_steady.LANG = args.lang
    names = list(MODULES) if args.module == "all" else [args.module]
    ok = [run(name, args.shots, args.page) for name in names]
    sys.exit(0 if all(ok) else 1)


if __name__ == "__main__":
    main()
