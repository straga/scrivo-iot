"""Turns the addresses the board printed after a fall into files and lines.

    uv run --no-project python tools/where_fell.py DEVICE 0x42009abc 0x4200912c ...

The board says where it fell as addresses, because the names live in the ELF on this computer and not on the
board. This asks that ELF. It must be the one the running image was built from: an address named against
another build points at whatever happens to sit there, and reads exactly as confident.

Prints one line per address, nearest the fall first. An address the ELF does not know comes out as itself with
"not in this build" beside it - which is the answer, and usually means the image on the board is not this one.
"""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def elf(device):
    path = os.path.join(ROOT, "target", target(device), "release", device)
    if not os.path.isfile(path):
        raise SystemExit("no ELF for %s at %s: build it with tools/build.sh %s" % (device, path, device))
    return path


def target(device):
    """The device's Rust target, from its board - the same answer tools/build.sh gets."""
    out = subprocess.run([sys.executable, os.path.join(HERE, "board_env.py"), device],
                         capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit(out.stderr.strip() or out.stdout.strip())
    for line in out.stdout.splitlines():
        if line.startswith("export TARGET="):
            return line.split("=", 1)[1].strip("'\"")
    raise SystemExit("board_env.py named no TARGET for %s" % device)


def addr2line(device, addresses):
    tool = "xtensa-%s-elf-addr2line" % target(device).split("-")[1]
    command = [tool, "-pfiaC", "-e", elf(device)] + addresses
    try:
        out = subprocess.run(command, capture_output=True, text=True)
    except FileNotFoundError:
        raise SystemExit("%s is not on PATH: . tools/env.sh first" % tool)
    if out.returncode != 0:
        raise SystemExit(out.stderr.strip() or "%s failed" % tool)
    return out.stdout


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        sys.exit(__doc__)
    device, addresses = args[0], args[1:]
    for address in addresses:
        if not address.lower().startswith("0x"):
            sys.exit("'%s' is not an address: write them as the board printed them, 0x42009abc" % address)
    said = addr2line(device, addresses)
    for line in said.splitlines():
        # addr2line answers "?? ??:0" for an address outside the build; saying so is the answer, not a silence.
        print(line if "??:0" not in line else "%s - not in this build" % line)


if __name__ == "__main__":
    main()
