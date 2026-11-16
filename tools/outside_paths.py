"""Every relative path in the tree that climbs out of it - the class of place that finds a neighbouring
project by counting directories up from itself, and so works from one place on one disk and from nowhere else.

    python3 tools/outside_paths.py [ROOT]

Walks the files git tracks, takes every run of `../` in them, resolves it against the directory it counts from
and prints the ones that land outside ROOT: file:line, what they climb to, the line. A path that counts from a
variable (`${CMAKE_CURRENT_LIST_DIR}/..`) counts from the file's own directory; one in a board.toml or a tool is
resolved the same way, which is how board_env.py and the tools read them.

What it does NOT see: a path built by joining pieces at run time (`os.path.join(root, "..", x)`) and `..` spelled
without a slash after it. Both are listed by a second pass as `JOINED`, to be read by eye.
"""
import os
import re
import subprocess
import sys

CLIMB = re.compile(r"(?:\.\./)+[\w.\-/]*")
JOINED = re.compile(r"""["']\.\.["']""")


def tracked(root):
    out = subprocess.run(["git", "-C", root, "ls-files"], capture_output=True, text=True, check=True).stdout
    return [os.path.join(root, p) for p in out.splitlines()]


def main(root):
    root = os.path.realpath(root)
    found = 0
    for path in tracked(root):
        try:
            text = open(path, encoding="utf-8").read()
        except (UnicodeDecodeError, FileNotFoundError, IsADirectoryError):
            continue
        base = os.path.dirname(path)
        for number, line in enumerate(text.splitlines(), 1):
            for m in CLIMB.finditer(line):
                lands = os.path.normpath(os.path.join(base, m.group(0)))
                if not (lands + os.sep).startswith(root + os.sep):
                    found += 1
                    print(f"{os.path.relpath(path, root)}:{number}: -> {os.path.relpath(lands, root)} | {line.strip()}")
            if JOINED.search(line):
                print(f"{os.path.relpath(path, root)}:{number}: JOINED | {line.strip()}")
    print(f"{found} climbing out of {root}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), ".."))
