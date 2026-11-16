"""Writes the values a device's stand starts from again, for the dictionary its image serves now.

    uv run --no-project python tools/stand_state.py burner

The values stand by ADDRESS, and an address means what the dictionary of the day made it: a row added since moves the rows after it, and the stand showed a smouldering of 2 s because the number of one setting was read at
the address of another. The state of a device is therefore of one dictionary - `same_dictionary`, the content hash after the `+` of its version - and `tests/test_page_stand_state.py` holds it. When the dictionary
moves the row goes red and this is the way back to green: the system card is kept - it stands first at the same addresses in every dictionary -, every setting is its default, and the frames of a scenario give the rest.

It does not record anything from a board: a state recorded off the boiler is `record.py`'s, and this one is made of what the dictionary itself says.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from page_stand import dictionary_of, fresh_state  # noqa: E402


def rewritten(dictionary, state):
    """STATE made of DICTIONARY and what of it still holds: its label and contract the dictionary's, its values `page_stand.fresh_state`'s - the whole of it when it is already of this dictionary."""
    return {**fresh_state(dictionary, state), "contract": dictionary["contract"], "dictionary": dictionary["version"]}


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: stand_state.py DEVICE  (a device with devices/DEVICE/stand_values.json)")
    device = sys.argv[1]
    path = os.path.join(ROOT, "devices", device, "stand_values.json")
    if not os.path.isfile(path):
        raise SystemExit("stand_state: %s has no stand_values.json" % device)
    dictionary = dictionary_of(ROOT, device)
    state = json.load(open(path))
    new = rewritten(dictionary, state)
    with open(path, "w") as f:
        json.dump(new, f)
        f.write("\n")
    print("%s: %d values, dictionary %s" % (path, len(new["values"]), new["dictionary"]))


if __name__ == "__main__":
    main()
