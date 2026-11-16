"""Lists the public items of crates/page/src/lib.rs and, for each, the files outside that file and outside crates/page/tests that name it: the machine inventory the removal of contract 1 was made from.

    uv run --no-project python tools/old_contract_users.py [--names NAME,NAME]

An item is any `pub fn|struct|enum|trait|const|static|type` at the top level of the file. A file names it when the word stands there as a whole word, comments and strings included - the inventory over-counts
on purpose, and a hit that is only a word in a comment is read and dropped by a person. What it does not see: a name built at run time, and an item reached through a `use ... as` rename.
"""
import argparse
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
LIB = "crates/page/src/lib.rs"
ITEM = re.compile(r"^pub (?:const )?(?:fn|struct|enum|trait|const|static|type) ([A-Za-z_][A-Za-z0-9_]*)", re.M)


def items(root=ROOT):
    """The names of the public items of LIB, in the order they stand."""
    return ITEM.findall(open(os.path.join(root, LIB), encoding="utf-8").read())


def users(name, root=ROOT):
    """The files outside LIB and outside crates/page/tests that name NAME as a whole word, with the count of lines in each."""
    out = subprocess.run(["git", "grep", "-cw", "--", name, "--", "*.rs", "*.py", "*.js", "*.md", ":!" + LIB, ":!crates/page/tests"], cwd=root, capture_output=True, text=True).stdout
    return sorted((line.rsplit(":", 1)[0], int(line.rsplit(":", 1)[1])) for line in out.splitlines() if line)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--names", default=None, help="only these items")
    args = ap.parse_args()
    wanted = args.names.split(",") if args.names else None
    for name in items():
        if wanted and name not in wanted:
            continue
        found = users(name)
        print("%-28s %3d files" % (name, len(found)))
        for path, n in found:
            print("    %-62s %d" % (path, n))


if __name__ == "__main__":
    main()
