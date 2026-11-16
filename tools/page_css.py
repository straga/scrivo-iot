"""Builds the stylesheet every device's page links, page/app.tw.css, from page/style/.

    uv run --no-project python tools/page_css.py

The style folder holds what a person writes - tokens.css with the colours, tailwind.css pulling them in, and
tailwind.config.js carrying them into Tailwind by name and naming the files Tailwind reads: the engine's page and the
scripts of every device's page folder (devices/*/page/*.js). What comes out is one minified file the page links, and
nothing else: it is built here rather than kept by hand, because Tailwind writes only the classes the page
actually uses, and a page whose classes changed needs the stylesheet built again. One stylesheet for every device:
a device's own part asks for a class, and the file that carries it is the same for the device that has none.

Tailwind itself comes from tools/page_css/, one copy for the whole project:

    bun install --cwd tools/page_css

Tailwind is run by bun, not by node: it is quicker, and it is what this machine installs page tools with.

Build the kit after this (tools/page_kit.py <device>): the kit carries the file this wrote.
"""
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
TAILWIND = os.path.join(HERE, "page_css", "node_modules", ".bin", "tailwindcss")
BUN = shutil.which("bun")

IN = "tailwind.css"
CONFIG = "tailwind.config.js"
OUT = os.path.join("..", "app.tw.css")


def build(root=ROOT, tailwind=TAILWIND, bun=BUN):
    """Builds page/app.tw.css of ROOT from its page/style folder and returns (path, bytes before, bytes after); a
    tree without a style folder or an entry point, or a Tailwind that is not installed, is an error in words."""
    style = os.path.join(root, "page", "style")
    if not os.path.isdir(style):
        raise SystemExit("page_css: no style folder at %s" % style)
    for name in (IN, CONFIG):
        if not os.path.isfile(os.path.join(style, name)):
            raise SystemExit("page_css: %s has no %s" % (style, name))
    if not bun or not os.path.isfile(bun):
        raise SystemExit("page_css: bun is not found at %s - install bun (brew install oven-sh/bun/bun): Tailwind is run by it"
                         % (bun or "any path on PATH"))
    if not os.path.isfile(tailwind):
        raise SystemExit("page_css: Tailwind is not installed at %s - run: bun install --cwd %s"
                         % (tailwind, os.path.join(ROOT, "tools", "page_css")))
    out = os.path.join(style, OUT)
    before = os.path.getsize(out) if os.path.isfile(out) else 0
    done = subprocess.run([bun, tailwind, "-c", CONFIG, "-i", IN, "-o", OUT, "--minify"],
                          cwd=style, capture_output=True, text=True)
    if done.returncode != 0:
        raise SystemExit("page_css: Tailwind refused the style of %s:\n%s" % (style, done.stderr.strip()))
    return os.path.normpath(out), before, os.path.getsize(out)


def main():
    if len(sys.argv) != 1:
        raise SystemExit(__doc__)
    path, before, after = build()
    print("%s  %d -> %d bytes" % (os.path.relpath(path, ROOT), before, after))


if __name__ == "__main__":
    main()
