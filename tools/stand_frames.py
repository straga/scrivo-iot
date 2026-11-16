"""The frames of a scenario for the tools that drive the page on its stand (page_steady, page_scheme): made by the
automaton's own code (`cargo run -p automaton_card --example scenario`) on the stand's state, and read by the names the
page gives the rows, so a moment is picked by what it IS - the stage, an organ on, a sensor silent.

A frame of contract 3 carries values by address; `named` lays them out by card and by the page's own name of each row
(texts.json of the burner's page: the engine's with the burner's laid over it, tools/page_sources.py), each in the shape a page of contract 1 read - {"on": ...}, {"seconds": ...}, and a
row with nothing to say, or no number now, as {"unknown": {}}.
"""
import json
import os
import subprocess

from page_sources import merged_texts

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
STATE = os.path.join(HERE, "page_stand_values.json")


def frames_of(scenario, root=ROOT, state=STATE):
    """The frames SCENARIO comes to on the stand's STATE, each carrying every value the stand then holds."""
    out = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "automaton_card", "--example", "scenario", "--",
                          state, scenario], cwd=root, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("the scenario did not run:\n" + out.stderr[-2000:])
    held = dict((at, v) for at, v in json.load(open(state))["values"])
    frames = []
    for frame in json.loads(out.stdout)["frames"]:
        held.update((at, v) for at, v in frame["values"])
        frames.append({"at": frame["at"], "values": sorted(held.items())})
    return frames


def drawn(thing, value):
    """VALUE of THING in the shape a page of contract 1 read."""
    kinds = thing.get("kinds", [])
    if value is None:
        return {"unknown": {}}
    if isinstance(value, bool):
        return {"on": value}
    if isinstance(value, (int, float)):
        return {"number": {"value": value, "unit": thing.get("unit", "")}} if "number" in kinds else {"seconds": value}
    if isinstance(value, str):
        return {"word": value}
    if isinstance(value, list):
        return {"words": value}
    if "at_least" in value:
        return {"at_least": {"value": value["at_least"], "unit": thing.get("unit", "")}}
    if "text" in value:
        return {"text": value["text"]}
    return {"unknown": {}}


def named(dictionary, frame, texts=None):
    """The rows of FRAME by (card, the page's name of the row), each value as `drawn` makes it."""
    texts = texts or merged_texts("burner")
    rows = texts.get("rows", {})
    out = {}
    values = dict((at, v) for at, v in frame["values"])
    for at, thing in enumerate(dictionary["things"]):
        if thing.get("part_of"):
            continue
        label = rows.get("%s/%s" % (thing["card"], thing["name"]), {}).get("label", thing["name"])
        if "{" in label:
            label = thing["name"]
        value = thing["fixed"] if "fixed" in thing else values.get(at)
        out[(thing["card"], label)] = drawn(thing, value)
    return out


# WHAT THE PAGE DRAWS, made in the browser by the page's own dictionary.js from the stand's dictionary, the page's
# texts and the stand's /state - the same code and data the page draws with, so a check of what the page SHOULD show
# is not a second reading of the values written in Python. FIELD is the part of it wanted: things, settings.
def drawn_js(field):
    return """(function(){
  function get(u) { const x = new XMLHttpRequest(); x.open('GET', u, false); x.send(); return JSON.parse(x.responseText); }
  // the page is one file with its tables inside it (`kit-<name>`, tools/page_kit.py); a page served as separate files is asked for them by name
  function table(name) { const e = document.getElementById('kit-' + name); return e ? JSON.parse(e.textContent) : get(name); }
  const d = get('/dictionary'), t = table('texts.json'), s = get('/state');
  const v = [];
  for (const [a, x] of s.values) v[a] = x;
  return JSON.stringify(dictionary.state(d, t, v).%s);
})()""" % field


THINGS_JS = drawn_js("things")
