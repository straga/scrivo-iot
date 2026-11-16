"""The settings a flash would leave behind, taken off the board before it and put back after it.

    uv run --no-project python tools/board_settings.py save HOST FILE [--names a,b]      # before the flash
    uv run --no-project python tools/board_settings.py restore HOST FILE [--dry-run]     # after it

The mode, the setpoint, the hysteresis, the level of power, the levels of the way up to the setpoint and the profiles with the windows stand on the card of the thermostat since contract 6, and a setting's place in the flash is its card's key: **THE FIRMWARE DOES NOT CARRY THE
VALUES FROM ONE KEY TO THE OTHER ON ITS OWN CODE**, so what a person set is taken off the board first and written on again by the way the page saves - one POST /settings. `save` reads the values
from /state of the board that still runs the image before (contract 5, 6, 7, 8, 9 or 10) and writes them down in FILE; `restore` writes the ones that differ onto a board of the contract the client reads (`board_client.CONTRACT`), reads them back and says each. Names
default to the settings of the thermostat's card; --names takes others. A save of a board that held the ROOM (`auto_hold=room`) is carried to the keys of the demand by `translated`: the boiler asked by the room, the old number as `auto_room_c` when it is a room's (6-35), and what has no counterpart is said, not dropped in silence. Exits 1 when a name is missing on the board, the board refuses a value, or a value read back is not the one written.
"""
import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from board_client import CONTRACT, Board, Refused  # noqa: E402

THERMOSTAT = ["auto_hold", "auto_hold_c", "auto_drop_c", "auto_step"]
LEVELS = ["auto_step_mid", "auto_step_small", "auto_mid_c", "auto_small_c"]
LOCK = ["auto_lock_min", "auto_lock_c"]
SCHEDULE = ["auto_profile"] + ["window_%d" % n for n in range(1, 7)]
# who asks the boiler for heat and the targets of the room: a board of the image before has none of them, and one that has them gives them up with the rest
DEMAND = ["auto_asked_by", "auto_room_c", "auto_room_eco_c", "auto_room_gap_c"]
# the setpoints of the WATER by profile of the image before: the profile is a target of the room now, so degrees of water have no counterpart
OF_THE_WATER_BY_PROFILE = ["auto_eco_c", "auto_off_c"]
# the limits of `auto_room_c` (crates/settings): a whole number of degrees
ROOM_TARGET_FROM, ROOM_TARGET_TO = 6, 35


def translated(settings):
    """SETTINGS of a save, written for the board of the contract the client reads: ({name: text}, [line said about what was not carried as it was]).

    The image before held the ROOM by `auto_hold=room`, with the target of the room in `auto_hold_c`; the new image refuses that choice (who asks is the demand's, `auto_asked_by`). So `room` becomes the boiler asked by the room, and the old `auto_hold_c` - degrees of the room - becomes `auto_room_c` and is not written as the water's: a target out of the limits of the room, or not a whole number, is not carried and the default stays. The setpoints of the water by profile have no counterpart and are named skipped. Everything else is as it was."""
    out, said = dict(settings), []
    if out.get("auto_hold") == "room":
        out["auto_hold"], out["auto_asked_by"] = "boiler", "room"
        old = out.pop("auto_hold_c", None)
        if old is None:
            said.append("auto_room_c: not written (the save has no target of the room); the default stays")
        elif re.fullmatch(r"\d+", old) and ROOM_TARGET_FROM <= int(old) <= ROOM_TARGET_TO:
            out["auto_room_c"] = old
            said.append("auto_hold_c = %s was the target of the room: it goes to auto_room_c, and the water's auto_hold_c stays as it stands" % old)
        else:
            said.append("auto_room_c: %r not written (the target of the room is a whole number from %d to %d); the default stays" % (old, ROOM_TARGET_FROM, ROOM_TARGET_TO))
    for name in OF_THE_WATER_BY_PROFILE:
        if name in out:
            said.append("%s: %s not written (degrees of the water by profile have no counterpart: the targets of the room are auto_room_c and auto_room_eco_c)" % (name, out.pop(name)))
    return out, said


def text_of(board, name, state):
    """The text the board keeps for the setting NAME, from STATE; Refused when the board has no such setting or says no text for it."""
    value = board.value("settings", name, state)
    if not isinstance(value, dict) or "text" not in value:
        raise Refused("the board keeps no text for the setting %s" % name)
    return value["text"]


def take(host, names=None):
    """What HOST keeps for NAMES: {"board", "contract", "dictionary", "settings": {name: text}}. A board of contract 5, 6, 7, 8, 9 or 10 is read.

    NAMES default to the four of the thermostat, the levels of the way up to the setpoint, the wait before a light with its floor and the profiles with the six windows - **THE LEVELS, THE WAIT AND THE SCHEDULE ONLY WHERE THE BOARD HAS THEM**: an image from before them never had them, and a person set nothing there. A name given by hand is asked of the board as it stands.
    """
    board = Board(host, contracts=(5, 6, 7, 8, 9, 10))
    state = board.state()
    if names is None:
        has = {s["name"] for s in board.dictionary()["settings"]}
        names = THERMOSTAT + [name for name in LEVELS + LOCK + SCHEDULE + DEMAND if name in has]
    return {"board": host, "contract": board.dictionary()["contract"], "dictionary": board.dictionary()["version"],
            "settings": {name: text_of(board, name, state) for name in names}}


def put(host, taken, dry_run=False, said=None):
    """TAKEN written onto HOST, a board of the contract the client reads (`board_client.CONTRACT`), by the page's own way: [(name, was, now)] of what it changed. TAKEN goes through `translated` first, and what that says is appended to SAID. Refused when the board refuses a value or keeps another one when read back."""
    board = Board(host, contracts=(CONTRACT,))
    state = board.state()
    settings, lines = translated(taken["settings"])
    if said is not None:
        said.extend(lines)
    now = {name: text_of(board, name, state) for name in settings}
    differ = [(name, now[name], text) for name, text in settings.items() if now[name] != text]
    if differ and not dry_run:
        answer = board.save([(name, text) for name, _, text in differ])
        if "error" in answer:
            raise Refused("the board refused: %s" % json.dumps(answer["error"]))
        back = board.state()
        wrong = [(name, text_of(board, name, back), text) for name, _, text in differ if text_of(board, name, back) != text]
        if wrong:
            raise Refused("the board keeps another value than was written: %s" % ", ".join("%s is %s, not %s" % w for w in wrong))
    return differ


def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("what", choices=["save", "restore"])
    ap.add_argument("host")
    ap.add_argument("file")
    ap.add_argument("--names", default=None)
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args(argv)
    try:
        if args.what == "save":
            taken = take(args.host, args.names.split(",") if args.names else None)
            with open(args.file, "w") as f:
                json.dump(taken, f, indent=1)
                f.write("\n")
            for name, text in taken["settings"].items():
                print("%s = %s" % (name, text))
            print("%d settings of %s (contract %s, %s) written to %s" % (len(taken["settings"]), args.host, taken["contract"], taken["dictionary"], args.file))
            return 0
        with open(args.file) as f:
            taken = json.load(f)
        said = []
        done = put(args.host, taken, args.dry_run, said)
        for line in said:
            print(line)
        for name, was, now in done:
            print("%s: %s -> %s%s" % (name, was, now, " (not written: --dry-run)" if args.dry_run else ""))
        print("%d of %d settings %s" % (len(done), len(taken["settings"]), "differ" if args.dry_run else "written and read back; the rest already stood so"))
        return 0
    except Refused as why:
        print("board_settings: %s" % why, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
