"""tools/page_kit.py: which page a device's kit carries - the engine's page (page/) with the device's addition on top when the device has a page folder, the engine's page alone when it has none.

    uv run --no-project --with pytest pytest tools
"""
import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from page_kit import lay, planned_of  # noqa: E402

REAL_ROOT = os.path.dirname(HERE)

INDEX = ('<html><head><title>{{device}}</title></head><body>\n'
         '<script src="page.js"></script>\n<!--device-->\n<script src="main.js"></script>\n</body></html>\n')


def frame(root, menu=None, texts=None):
    """A small engine page of this test's own, in ROOT/page."""
    folder = os.path.join(root, "page")
    os.makedirs(folder, exist_ok=True)
    files = {"index.html": INDEX, "page.js": "const framework = 1;\n", "main.js": "const mounted = 1;\n", "icon.svg": "<svg/>",
             "menu.json": json.dumps(menu if menu is not None else {"ui": {"save": "Save"}}),
             "texts.json": json.dumps(texts if texts is not None else {"fold": {"show": "show"}})}
    for name, text in files.items():
        with open(os.path.join(folder, name), "w") as f:
            f.write(text)
    return folder


def device(root, name, addon=None):
    """A device with a version; ADDON, when given, is its page folder: {file name: text}."""
    folder = os.path.join(root, "devices", name)
    os.makedirs(folder)
    with open(os.path.join(folder, "version.txt"), "w") as f:
        f.write("stragabases3-26.09.19.1\n")
    if addon is not None:
        os.makedirs(os.path.join(folder, "page"))
        for file, text in addon.items():
            with open(os.path.join(folder, "page", file), "w") as f:
                f.write(text if isinstance(text, str) else json.dumps(text))


def laid(kit, name):
    return open(os.path.join(kit, "page", name)).read()


def refusal_of(root, name, tmp_path):
    with pytest.raises(SystemExit) as refused:
        lay(name, root=root)
    return str(refused.value)


# ---- a device with a page folder carries the engine's page; the folder is what it adds

def test_a_device_with_nothing_to_add_carries_the_engines_page(tmp_path):
    # the empty device: a page folder with nothing in it
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "plain", addon={})
    kit = lay("plain", root=root)
    assert sorted(os.listdir(os.path.join(kit, "page"))) == ["index.html", "index.html.gz"]
    page = laid(kit, "index.html")
    assert "device.js" not in page, "a device that adds nothing asks for no file of its own"
    assert "<title>plain</title>" in page


def test_a_device_with_scripts_adds_them_in_one_file_the_page_loads_between_the_engine_and_its_start(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "burner", addon={"page.json": {"scripts": ["b.js", "a.js"]}, "a.js": "const a = 1;\n", "b.js": "const b = 2;\n"})
    kit = lay("burner", root=root)
    assert sorted(os.listdir(os.path.join(kit, "page"))) == ["index.html", "index.html.gz"], "the addition is inside the one file"
    page = laid(kit, "index.html")
    assert page.index("const framework = 1;") < page.index("const b = 2;") < page.index("const a = 1;") < page.index("const mounted = 1;"), \
        "the order page.json gives, between the engine's page and its start"
    assert "<!--device-->" not in page


def test_the_texts_of_the_device_are_merged_into_the_engines(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root, menu={"ui": {"save": "Save"}, "units": {"s": "s"}}, texts={"fold": {"show": "show"}})
    device(root, "burner", addon={"menu.json": {"ui": {"scheme": "Scheme"}, "modules": {"fan": {"title": "Fan"}}},
                                  "texts.json": {"rows": {"fan/pwm": {"word": "fan"}}}})
    kit = lay("burner", root=root)
    page = laid(kit, "index.html")
    assert embedded(page, "menu.json") == {"ui": {"save": "Save", "scheme": "Scheme"}, "units": {"s": "s"}, "modules": {"fan": {"title": "Fan"}}}
    assert embedded(page, "texts.json") == {"fold": {"show": "show"}, "rows": {"fan/pwm": {"word": "fan"}}}
    from page_kit import merged_menu, merged_texts
    assert merged_menu("burner", root=root) == embedded(page, "menu.json")
    assert merged_texts("burner", root=root) == embedded(page, "texts.json")


def test_a_word_the_device_says_differently_from_the_engine_is_refused_with_its_path(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root, menu={"ui": {"save": "Save"}})
    device(root, "burner", addon={"menu.json": {"ui": {"save": "Keep"}}})
    words = refusal_of(root, "burner", tmp_path)
    assert "ui.save" in words and "Save" in words and "Keep" in words


def test_a_word_the_device_says_as_the_engine_does_is_no_collision(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root, menu={"ui": {"save": "Save"}})
    device(root, "burner", addon={"menu.json": {"ui": {"save": "Save"}}})
    kit = lay("burner", root=root)
    assert embedded(laid(kit, "index.html"), "menu.json") == {"ui": {"save": "Save"}}


def test_a_device_adds_sections_to_a_list_of_sections_the_engine_has_and_the_engines_stay_where_they_are(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root, menu={"tree": {"system": {"sections": [{"id": "time", "settings": ["time"]}, {"id": "board", "board": True}]}}})
    device(root, "burner", addon={"menu.json": {"tree": {"system": {"sections": [{"id": "hands", "group": "service"}, {"id": "fire", "group": "service"}]}}}})
    kit = lay("burner", root=root)
    sections = embedded(laid(kit, "index.html"), "menu.json")["tree"]["system"]["sections"]
    assert [s["id"] for s in sections] == ["time", "board", "hands", "fire"]


def test_a_section_the_device_says_as_the_engine_does_is_no_collision_and_one_it_says_differently_is_refused_with_its_path(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root, menu={"tree": {"system": {"sections": [{"id": "time", "settings": ["time"]}]}}})
    device(root, "burner", addon={"menu.json": {"tree": {"system": {"sections": [{"id": "time", "settings": ["time"]}]}}}})
    assert [s["id"] for s in embedded(laid(lay("burner", root=root), "index.html"), "menu.json")["tree"]["system"]["sections"]] == ["time"]
    root = str(tmp_path / "other" / "scrivo-iot")
    frame(root, menu={"tree": {"system": {"sections": [{"id": "time", "settings": ["time"]}]}}})
    device(root, "burner", addon={"menu.json": {"tree": {"system": {"sections": [{"id": "time", "settings": ["zone"]}]}}}})
    words = refusal_of(root, "burner", tmp_path)
    assert "tree.system.sections" in words and "time" in words


def test_two_lists_that_are_not_lists_of_sections_and_differ_are_still_refused(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root, menu={"ui": {"tabs": ["a", "b"]}})
    device(root, "burner", addon={"menu.json": {"ui": {"tabs": ["a", "c"]}}})
    assert "ui.tabs" in refusal_of(root, "burner", tmp_path)


def test_the_page_a_stand_serves_is_the_page_the_kit_carries_without_the_gz(tmp_path):
    from page_kit import page_into
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "burner", addon={"page.json": {"scripts": ["a.js"]}, "a.js": "const a = 1;\n", "menu.json": {"ui": {"scheme": "Scheme"}}})
    folder = str(tmp_path / "served")
    page_into("burner", folder, root=root)
    kit = lay("burner", root=root)
    assert sorted(os.listdir(folder)) == sorted(n for n in os.listdir(os.path.join(kit, "page")) if not n.endswith(".gz"))
    for name in os.listdir(folder):
        assert open(os.path.join(folder, name)).read() == laid(kit, name), name


def test_a_row_of_the_table_of_texts_the_device_says_differently_is_refused_with_its_path(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root, texts={"rows": {"system/wifi": {"label": "wifi"}}})
    device(root, "burner", addon={"texts.json": {"rows": {"system/wifi": {"label": "network"}}}})
    words = refusal_of(root, "burner", tmp_path)
    assert "rows.system/wifi.label" in words and "wifi" in words and "network" in words and "texts.json" in words


def test_a_row_of_the_table_of_texts_the_device_says_as_the_engine_does_is_no_collision_and_a_new_row_is_added(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root, texts={"rows": {"system/wifi": {"label": "wifi"}}})
    device(root, "burner", addon={"texts.json": {"rows": {"system/wifi": {"label": "wifi"}, "fan/pwm": {"label": "fan"}}}})
    kit = lay("burner", root=root)
    assert embedded(laid(kit, "index.html"), "texts.json") == {"rows": {"system/wifi": {"label": "wifi"}, "fan/pwm": {"label": "fan"}}}


# ---- what a device may not do

def test_a_file_of_the_device_that_is_a_file_of_the_engine_is_refused_in_words(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "burner", addon={"page.json": {"scripts": ["page.js"]}, "page.js": "const mine = 1;\n"})
    words = refusal_of(root, "burner", tmp_path)
    assert "page.js" in words and "engine" in words


def test_a_script_nobody_lists_is_refused_and_a_listed_one_that_is_not_there_is_refused(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "forgot", addon={"extra.js": "const x = 1;\n"})
    assert "extra.js" in refusal_of(root, "forgot", tmp_path)
    device(root, "missing", addon={"page.json": {"scripts": ["gone.js"]}})
    assert "gone.js" in refusal_of(root, "missing", tmp_path)


def test_a_script_the_page_names_twice_is_refused(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "twice", addon={"page.json": {"scripts": ["a.js", "a.js"]}, "a.js": "const a = 1;\n"})
    assert "a.js" in refusal_of(root, "twice", tmp_path)


def test_an_engine_page_that_has_no_place_for_the_device_is_refused_when_the_device_has_scripts(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    folder = frame(root)
    with open(os.path.join(folder, "index.html"), "w") as f:
        f.write("<html></html>")
    device(root, "burner", addon={"page.json": {"scripts": ["a.js"]}, "a.js": "const a = 1;\n"})
    assert "device" in refusal_of(root, "burner", tmp_path)


# ---- a device without a page folder carries the engine's page alone

def test_a_device_without_a_page_folder_carries_the_engines_page_alone(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "bench_dualr3")
    kit = lay("bench_dualr3", root=root)
    page = laid(kit, "index.html")
    assert "const framework = 1;" in page and "const mounted = 1;" in page, "the engine's scripts are in the page"
    assert "<title>bench_dualr3</title>" in page, "the page says whose it is"
    assert "<!--device-->" not in page, "the place for the device's scripts is not left in the page"
    assert embedded(page, "menu.json") == {"ui": {"save": "Save"}}, "the menu is the engine's, untouched"


def test_a_device_that_is_not_there_has_no_kit(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    assert "no_such" in refusal_of(root, "no_such", tmp_path)


def test_the_burner_of_this_project_carries_the_engines_page_with_its_scheme_and_the_sonoff_without(tmp_path):
    """By which file arrived, not by a word written in it: the page is rewritten as the stage goes on, and a
    marker inside it says nothing about where the kit took it from.

    **THE KIT IS LAID IN A ROOT OF THIS TEST'S OWN** - the engine's page and the devices of this project linked into a temporary one -, never with the real tree for the root: `lay` writes `build/<device>/kit`, the folder `upload_app.py put` mirrors onto a board, and a test that wrote it there put the page of the branch the tree stood on into the kit that went to the boiler (03.10). `conftest.py` holds every run to leave `build/` as it found it."""
    root = str(tmp_path / "scrivo-iot")
    os.makedirs(os.path.join(root, "devices"))
    os.symlink(os.path.join(REAL_ROOT, "page"), os.path.join(root, "page"))
    for name in ("burner", "bench_dualr3"):
        os.symlink(os.path.join(REAL_ROOT, "devices", name), os.path.join(root, "devices", name))
    burner = lay("burner", root=root)
    assert burner.startswith(root), "the kit lies in the test's own root"
    assert sorted(os.listdir(os.path.join(burner, "page"))) == ["index.html", "index.html.gz"]
    page = laid(burner, "index.html")
    for part in ("class Page", "class Scheme", "owl", "wire", "dictionary"):
        assert part in page, part
    sonoff = lay("bench_dualr3", root=root)
    sonoff_page = laid(sonoff, "index.html")
    assert "class Scheme" not in sonoff_page, "the Sonoff carries no part of a plant"
    assert sonoff_page != page


# ---- a script that is not JavaScript never reaches a kit

def test_a_script_that_does_not_parse_never_reaches_the_kit(tmp_path):
    # The one this test was written from: a backtick inside an HTML comment of the burner's OWL template ended the
    # template string, and the page stopped being JavaScript. The image builds all the same - to cargo a page file is
    # just a file - so the board would have served a page that never ran.
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "burner", addon={"page.json": {"scripts": ["a.js"]},
                                  "a.js": "const t = xml`<div><!-- a `backtick` in here -->< /div>`;\n"})
    assert "a.js" in refusal_of(root, "burner", tmp_path)


def test_an_engine_script_that_does_not_parse_never_reaches_the_kit(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    folder = frame(root)
    with open(os.path.join(folder, "page.js"), "w") as f:
        f.write("const t = xml`<div><!-- a `backtick` in here -->< /div>`;\n")
    device(root, "plain", addon={})
    assert "page.js" in refusal_of(root, "plain", tmp_path)


def test_scripts_that_parse_are_laid_as_one(tmp_path):
    root = str(tmp_path / "scrivo-iot")
    frame(root)
    device(root, "burner", addon={"page.json": {"scripts": ["a.js"]}, "a.js": "const t = 1;\n"})
    kit = lay("burner", root=root)
    assert "const t = 1;" in laid(kit, "index.html")


def test_the_stand_of_the_dual_carries_the_duals_page_under_its_own_name():
    """The Dual's firmware on the bench S3 is read through the Dual's page - its cards, its words - and a person who opens it
    reads the stand's name in the title, not the Dual's."""
    dual = planned_of("bench_dualr3", REAL_ROOT)
    stand = planned_of("bench_dualr3s3", REAL_ROOT)
    for name in ("menu.json", "texts.json"):
        assert embedded(stand["index.html"][1], name) == embedded(dual["index.html"][1], name), "the stand's %s is not the Dual's" % name
    assert "<title>bench_dualr3s3" in stand["index.html"][1]
    assert "bench_dualr3s3" not in dual["index.html"][1]


# ---- 11.11: the page is ONE file. A browser asks for one address and the board serves one file; a kit of 27 files was
# six connections open at once on a board whose handlers are few (.8_Research).

import gzip
import re


def embedded(page, name):
    """A table of the page (menu.json, texts.json) as the one file carries it: a data element, read as JSON."""
    found = re.search(r'<script type="application/json" id="kit-%s">(.*?)</script>' % re.escape(name), page, re.S)
    assert found, "%s is not inside the page" % name
    return json.loads(found.group(1))


def one_file_root(tmp_path, addon=None, **frame_files):
    root = str(tmp_path / "scrivo-iot")
    folder = frame(root, **frame_files)
    with open(os.path.join(folder, "index.html"), "w") as f:
        f.write('<html><head><title>{{device}}</title><link rel="icon" href="icon.svg">'
                '<link rel="stylesheet" href="app.tw.css"></head><body>\n'
                '<script src="page.js"></script>\n<!--device-->\n<script src="main.js"></script>\n</body></html>\n')
    with open(os.path.join(folder, "app.tw.css"), "w") as f:
        f.write("body { color: red }\n")
    device(root, "burner", addon=addon if addon is not None else {})
    return root


def test_the_kit_carries_one_page_file_and_its_gz(tmp_path):
    root = one_file_root(tmp_path, addon={"page.json": {"scripts": ["a.js"]}, "a.js": "const a = 1;\n", "menu.json": {"ui": {"x": "X"}}})
    kit = lay("burner", root=root)
    assert sorted(os.listdir(os.path.join(kit, "page"))) == ["index.html", "index.html.gz"]


def test_the_gz_is_the_page_gzipped_and_the_same_source_gives_the_same_bytes(tmp_path):
    root = one_file_root(tmp_path)
    kit = lay("burner", root=root)
    page = open(os.path.join(kit, "page", "index.html"), "rb").read()
    first = open(os.path.join(kit, "page", "index.html.gz"), "rb").read()
    assert gzip.decompress(first) == page
    kit = lay("burner", root=root)
    assert open(os.path.join(kit, "page", "index.html.gz"), "rb").read() == first


def test_scripts_style_icon_and_both_tables_stand_inside_the_file_in_the_order_the_page_loads_them(tmp_path):
    root = one_file_root(tmp_path, addon={"page.json": {"scripts": ["b.js", "a.js"]}, "a.js": "const a = 1;\n", "b.js": "const b = 2;\n",
                                          "menu.json": {"ui": {"scheme": "Scheme"}}, "texts.json": {"rows": {"fan/pwm": {"word": "fan"}}}})
    page = laid(lay("burner", root=root), "index.html")
    assert "const framework = 1;" in page and "const mounted = 1;" in page
    assert page.index("const framework = 1;") < page.index("const b = 2;") < page.index("const a = 1;") < page.index("const mounted = 1;")
    assert "body { color: red }" in page and "<style>" in page
    assert re.search(r'<link rel="icon" href="data:image/svg\+xml;base64,[A-Za-z0-9+/=]+">', page)
    assert embedded(page, "menu.json") == {"ui": {"save": "Save", "scheme": "Scheme"}}
    assert embedded(page, "texts.json") == {"fold": {"show": "show"}, "rows": {"fan/pwm": {"word": "fan"}}}
    assert "<!--device-->" not in page
    assert not re.search(r'(src|href)="(?!data:)[^"]*"', page), "the page names no file of its own: %s" % re.findall(r'(?:src|href)="[^"]*"', page)


def test_a_script_each_stays_a_script_of_its_own(tmp_path):
    # a classic script runs when it is reached and a failure of one does not stop the next: the file keeps the pieces apart
    root = one_file_root(tmp_path)
    page = laid(lay("burner", root=root), "index.html")
    assert page.count("<script>") == 2


def test_a_page_that_names_a_file_the_kit_has_not_got_is_refused_in_words(tmp_path):
    root = one_file_root(tmp_path)
    with open(os.path.join(root, "page", "index.html"), "a") as f:
        f.write('<script src="gone.js"></script>\n')
    words = refusal_of(root, "burner", tmp_path)
    assert "gone.js" in words


def test_a_script_that_would_end_its_element_is_refused_and_one_that_only_looks_like_html_is_not(tmp_path):
    root = one_file_root(tmp_path, addon={"page.json": {"scripts": ["a.js"]}, "a.js": 'const a = "</script>";\n'})
    assert "a.js" in refusal_of(root, "burner", tmp_path)
    root = one_file_root(tmp_path / "second", addon={"page.json": {"scripts": ["a.js"]}, "a.js": 'const a = "<div></div>"; const b = 1 < 2;\n'})
    assert "const b = 1 < 2;" in laid(lay("burner", root=root), "index.html")


def test_a_table_that_says_a_script_end_is_carried_whole(tmp_path):
    # a word of a table may hold any characters; inside a data element the angle bracket is written as <, which is the same string to JSON
    root = one_file_root(tmp_path, addon={"texts.json": {"rows": {"x/y": {"word": "</script><b>"}}}})
    page = laid(lay("burner", root=root), "index.html")
    assert page.count("</script>") == page.count("<script"), "the word broke out of its element"
    assert embedded(page, "texts.json")["rows"]["x/y"]["word"] == "</script><b>"


def test_the_page_a_stand_serves_is_the_one_file(tmp_path):
    from page_kit import page_into
    root = one_file_root(tmp_path)
    folder = str(tmp_path / "served")
    page_into("burner", folder, root=root)
    assert os.listdir(folder) == ["index.html"]
    assert open(os.path.join(folder, "index.html")).read() == laid(lay("burner", root=root), "index.html")


@pytest.mark.parametrize("name", ["burner", "bench_dualr3", "bench_dualr3s3", "bench_s3"])
def test_the_real_devices_carry_one_file(name, tmp_path):
    root = str(tmp_path / "scrivo-iot")
    os.makedirs(os.path.join(root, "devices"))
    os.symlink(os.path.join(REAL_ROOT, "page"), os.path.join(root, "page"))
    for device_name in os.listdir(os.path.join(REAL_ROOT, "devices")):
        os.symlink(os.path.join(REAL_ROOT, "devices", device_name), os.path.join(root, "devices", device_name))
    kit = lay(name, root=root)
    assert sorted(os.listdir(os.path.join(kit, "page"))) == ["index.html", "index.html.gz"]
    page = laid(kit, "index.html")
    assert f"<title>{name}" in page
    assert "class Page" in page or "page.js" not in page


def test_the_stand_of_the_dual_on_the_s3_carries_the_page_of_the_dual():
    """bench_dualr3s3 is the Dual's firmware on the S3 bench board, and its page is the Dual's: a menu or a text changed
    for one of them and not the other makes the stand show a page no board has."""
    from page_sources import merged_menu, merged_texts
    assert merged_menu("bench_dualr3s3", REAL_ROOT) == merged_menu("bench_dualr3", REAL_ROOT)
    assert merged_texts("bench_dualr3s3", REAL_ROOT) == merged_texts("bench_dualr3", REAL_ROOT)
