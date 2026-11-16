"""tools/page_css.py: the stylesheet every device's page links is built from the engine's style folder, over the engine's page and the scripts of the devices' page folders.

    uv run --no-project --with pytest pytest tools

The build is proved by what comes out, not by its exit code: Tailwind writes only the classes it FOUND, so a
config pointed at the wrong files ends happily with a stylesheet the page cannot use.
"""
import os
import re
import shutil
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from page_css import TAILWIND, build  # noqa: E402

REAL_ROOT = os.path.dirname(HERE)
# A class is written in two places, and the second one is where a page quietly loses its look: `class="..."` is
# plain, while a class that depends on the state lives inside t-att-class as a STRING LITERAL among JavaScript.
# Splitting that attribute on spaces yields `props.mono` and `===`, which are not classes and never will be in the
# stylesheet - so the literals are taken out of it instead.
CLASS = re.compile(r'(?<!t-att-)(?<!t-attf-)class="([^"]*)"')
CONDITIONAL = re.compile(r't-att-class="([^"]*)"')
LITERAL = re.compile(r"'([^']*)'")
# A literal that is being COMPARED against is a value, not a class: `props.level === 'ok'` names a level the board
# sends, and there is no .ok in any stylesheet.
COMPARED = re.compile(r"[!=]==?\s*'[^']*'")


def classes_of(*files):
    """Every Tailwind class the page files ask for - written plainly, and written under a condition."""
    found = set()
    for path in files:
        text = open(path).read()
        for written in CLASS.findall(text):
            found.update(written.split())
        for expression in CONDITIONAL.findall(text):
            for literal in LITERAL.findall(COMPARED.sub("", expression)):
                found.update(literal.split())
    return found


def pages_of(root):
    """The files a device page is made of that carry classes: the engine's index and script, the burner's parts."""
    return [os.path.join(root, "page", "index.html"), os.path.join(root, "page", "page.js"),
            os.path.join(root, "devices", "burner", "page", "parts.js")]


def copy_of_tree(tmp_path):
    """The engine's page and the burner's page folder as they are in this tree, copied where a build may write."""
    root = str(tmp_path / "scrivo-iot")
    os.makedirs(os.path.join(root, "devices", "burner"))
    shutil.copytree(os.path.join(REAL_ROOT, "page"), os.path.join(root, "page"))
    shutil.copytree(os.path.join(REAL_ROOT, "devices", "burner", "page"), os.path.join(root, "devices", "burner", "page"))
    return root


needs_tailwind = pytest.mark.skipif(not os.path.isfile(TAILWIND), reason="bun install --cwd tools/page_css")


@needs_tailwind
def test_every_class_the_page_asks_for_is_in_the_stylesheet_built_for_it(tmp_path):
    root = copy_of_tree(tmp_path)
    path, _, after = build(root=root)
    style = open(path).read()
    assert after > 0
    missing = [c for c in classes_of(*pages_of(root)) if escaped(c) not in style]
    assert missing == [], "built from the wrong files: %s" % missing


def escaped(name):
    """The class NAME as a stylesheet spells it: [ ] . / : are escaped there, as Tailwind writes them."""
    return "." + re.sub(r"([\[\]./:%])", r"\\\1", name)


@needs_tailwind
def test_a_class_only_a_device_script_asks_for_is_in_the_stylesheet(tmp_path):
    """The devices' scripts are among what is read: a class a device's own part uses must not depend on the engine using it too."""
    root = copy_of_tree(tmp_path)
    with open(os.path.join(root, "devices", "burner", "page", "parts.js"), "a") as f:
        f.write('const only_here = xml`<div class="bg-rose-500"/>`;\n')
    path, _, _ = build(root=root)
    assert escaped("bg-rose-500") in open(path).read()


@needs_tailwind
def test_the_tokens_of_the_dark_theme_come_with_it(tmp_path):
    # the colours are imported by tailwind.css, and without them every class of the page draws an empty variable
    root = copy_of_tree(tmp_path)
    path, _, _ = build(root=root)
    style = open(path).read()
    assert "--bg:" in style and "--acc-soft:" in style
    assert "data-t=d" in style, "the dark theme did not come"


@needs_tailwind
def test_a_class_no_page_uses_is_not_written(tmp_path):
    # the other half: Tailwind writes what the page asks for and nothing else, which is why the file is small
    root = copy_of_tree(tmp_path)
    path, _, _ = build(root=root)
    assert escaped("bg-rose-500") not in open(path).read()


def test_a_tree_without_a_style_folder_is_refused(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    os.makedirs(os.path.join(root, "page"))
    with pytest.raises(SystemExit) as refusal:
        build(root=root)
    assert "no style folder" in str(refusal.value)


def test_a_style_folder_without_its_entry_point_is_refused(tmp_path):
    root = copy_of_tree(tmp_path)
    os.remove(os.path.join(root, "page", "style", "tailwind.css"))
    with pytest.raises(SystemExit) as refusal:
        build(root=root)
    assert "tailwind.css" in str(refusal.value)


def test_a_tailwind_that_is_not_installed_says_how_to_install_it(tmp_path):
    root = copy_of_tree(tmp_path)
    with pytest.raises(SystemExit) as refusal:
        build(root=root, tailwind=str(tmp_path / "no-such-tailwind"))
    assert "bun install" in str(refusal.value)


def test_a_board_without_bun_is_told_to_install_it(tmp_path):
    root = copy_of_tree(tmp_path)
    with pytest.raises(SystemExit) as refusal:
        build(root=root, bun=str(tmp_path / "no-such-bun"))
    assert "bun" in str(refusal.value) and "no-such-bun" in str(refusal.value), str(refusal.value)


@needs_tailwind
def test_a_style_tailwind_cannot_read_is_refused_in_words(tmp_path):
    root = copy_of_tree(tmp_path)
    with open(os.path.join(root, "page", "style", "tailwind.config.js"), "w") as f:
        f.write("module.exports = { this is not javascript }\n")
    with pytest.raises(SystemExit) as refusal:
        build(root=root)
    assert "refused the style" in str(refusal.value)


def test_the_stylesheet_in_the_tree_is_the_one_this_builds(tmp_path):
    """What lies in the engine's page is what this command produces - not a file copied in from somewhere else."""
    if not os.path.isfile(TAILWIND):
        pytest.skip("bun install --cwd tools/page_css")
    root = copy_of_tree(tmp_path)
    built, _, _ = build(root=root)
    in_the_tree = os.path.join(REAL_ROOT, "page", "app.tw.css")
    assert open(built).read() == open(in_the_tree).read(), "page/app.tw.css is out of date"
