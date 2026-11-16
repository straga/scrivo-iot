"""Every place that reads, writes or names a file of the board's flash: the machine list a rule about that file is checked against.

    uv run --no-project python tools/flash_file_paths.py /data/relief/state [ROOT]

A file is named by its path in the text or by a `const NAME: &str = "<path>"` of a Rust module; the lines that use that NAME count as the path's own. Each line is sorted by what it does with the file:
read (`vfs::read`, `vfs::read_at`), write (`flash_owner::write`, `vfs::write`), append, remove, or named (anything else that holds the path or its const).
What it cannot see: a path put together at run time (`format!("/data/{}", ..)`), and a file reached through a const another module re-exports; both would need a look at the module that builds them.
"""
import re
import sys
from pathlib import Path

DOES = (
    ("read", re.compile(r"\bvfs::read(_at)?\(")),
    ("write", re.compile(r"\b(flash_owner|vfs)::write\(")),
    ("append", re.compile(r"\b(flash_owner|vfs)::append\(")),
    ("remove", re.compile(r"\b(flash_owner|vfs)::remove\(")),
)
SKIP = {"target", ".git", "node_modules", "build"}
SUFFIXES = {".rs", ".py", ".js", ".sh", ".toml", ".md"}


def sources(root):
    for path in sorted(Path(root).rglob("*")):
        if path.is_file() and path.suffix in SUFFIXES and not SKIP & set(path.parts):
            yield path


def kind_of(line):
    for kind, pattern in DOES:
        if pattern.search(line):
            return kind
    return "named"


def paths_of(flash_path, root):
    """[(file, line number, kind, text)] for every line that holds FLASH_PATH or a const that holds it."""
    found = []
    for path in sources(root):
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        if not any(flash_path in line for line in lines):
            continue
        names = {m.group(1) for line in lines for m in [re.search(r'\bconst\s+(\w+)\s*:\s*&str\s*=\s*"%s"' % re.escape(flash_path), line)] if m}
        uses = [re.compile(r"\b%s\b" % re.escape(name)) for name in sorted(names)]
        for number, line in enumerate(lines, 1):
            if flash_path in line or any(use.search(line) for use in uses):
                found.append((str(path), number, kind_of(line), line.strip()))
    return found


def main(argv):
    if not argv:
        sys.exit(__doc__)
    root = argv[1] if len(argv) > 1 else "."
    rows = paths_of(argv[0], root)
    for file, number, kind, text in rows:
        print("%-7s %s:%d  %s" % (kind, file, number, text[:110]))
    counts = {kind: sum(1 for row in rows if row[2] == kind) for kind in ("read", "write", "append", "remove", "named")}
    print(" ".join("%s=%d" % pair for pair in counts.items()))


if __name__ == "__main__":
    main(sys.argv[1:])
