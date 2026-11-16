"""What a board's /state carries, checked without the page.

    uv run --no-project python tools/state_contract.py HOST       # the contract of docs/CONTRACT_Device_Api.md

`values_check(dictionary, state)` - a /state of the contract of docs/CONTRACT_Device_Api.md against the dictionary the same board serves: its contract
and its version, then every address - each described one with a value its entry allows, nothing foreign. It answers with the refusals, each naming the
thing: [] - the state keeps its contract. Exits non-zero on a refusal.
"""
import json
import os
import sys
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from board_client import CONTRACT  # noqa: E402 - the number the tools read, one for all of them


def _number(x):
    return isinstance(x, (int, float)) and not isinstance(x, bool)


def _who(thing, at):
    row = f"{thing['part_of']}." if thing.get("part_of") else ""
    return f"{thing['card']}/{row}{thing['name']} (address {at})"


def _value_refusal(thing, value):
    """Why VALUE is not one the dictionary's THING may carry, in words; None - it may. The kind is the dictionary's:
    a switch is `true`, a number a number, a code a string of the thing's list (the contract of docs/CONTRACT_Device_Api.md)."""
    kinds = thing.get("kinds", [])
    if value is None:
        return None  # nothing in this state
    if isinstance(value, bool):
        return None if "on" in kinds else f"is not a switch, and came as {json.dumps(value)}"
    if _number(value):
        if "number" in kinds:
            return None
        if "seconds" in kinds and isinstance(value, int) and value >= 0:
            return None
        return f"takes no such number: {json.dumps(value)}"
    if isinstance(value, str):
        if "word" not in kinds:
            return f"takes no code, and came as {json.dumps(value)}"
        return None if value in thing.get("words", []) else f"'{value}' is not of its list {thing.get('words', [])}"
    if isinstance(value, list):
        ok = "words" in kinds and all(isinstance(w, str) and w in thing.get("words", []) for w in value)
        return None if ok else f"a list it cannot carry: {json.dumps(value)}"
    if isinstance(value, dict) and len(value) == 1:
        (kind, inside), = value.items()
        if kind == "at_least" and "at_least" in kinds and _number(inside):
            return None
        if kind == "unknown" and "unknown" in kinds and inside in thing.get("reasons", []):
            return None
        if kind == "text" and "text" in kinds and isinstance(inside, str):
            return None
    return f"a value it cannot carry: {json.dumps(value)}"


def values_check(dictionary, state):
    """The refusals of STATE, a /state of the contract the tools read (`CONTRACT`), against DICTIONARY - the one the same board serves: its contract
    and its version first, then every address - each described one sent once with a value its entry allows, a number
    fixed in the image and a button never. [] - the state keeps the dictionary."""
    if not isinstance(state, dict) or state.get("contract") != CONTRACT:
        return [f"the state keeps contract {state.get('contract') if isinstance(state, dict) else None}, not {CONTRACT}"]
    if state.get("dictionary") != dictionary.get("version"):
        return [f"the state is of dictionary {state.get('dictionary')}, and this one is {dictionary.get('version')}"]
    things = dictionary.get("things", [])
    refused, seen = [], set()
    for record in state.get("values", []):
        at, value = record
        if not isinstance(at, int) or not 0 <= at < len(things):
            refused.append(f"address {at} is no thing of the dictionary")
            continue
        thing = things[at]
        if at in seen:
            refused.append(f"{_who(thing, at)}: sent twice")
            continue
        seen.add(at)
        if "fixed" in thing or thing.get("kinds") == ["button"]:
            refused.append(f"{_who(thing, at)}: carries no value on the wire, and one came")
            continue
        why = _value_refusal(thing, value)
        if why:
            refused.append(f"{_who(thing, at)}: {why}")
    for at, thing in enumerate(things):
        if at not in seen and "fixed" not in thing and thing.get("kinds") != ["button"]:
            refused.append(f"{_who(thing, at)}: no value")
    return refused


def main():
    import argparse

    ap = argparse.ArgumentParser()
    ap.add_argument("host")
    args = ap.parse_args()
    with urllib.request.urlopen(f"http://{args.host}/state", timeout=5) as r:
        state = json.loads(r.read().decode())
    with urllib.request.urlopen(f"http://{args.host}/dictionary", timeout=5) as r:
        dictionary = json.loads(r.read().decode())
    refusals = values_check(dictionary, state)
    print("\n".join(refusals + [f"{len(state.get('values', []))} values, {len(refusals)} refused"]))
    sys.exit(1 if refusals else 0)


if __name__ == "__main__":
    main()
