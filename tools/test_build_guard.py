"""tools/build_guard.py: the sums of what lies in `build/`, and what differs between two of them - the part of a run's guard that can be held to rows (the hook that fails a run is conftest.py, and what it does to the real
tree is shown by running the tests of the kit before and after the fix)."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from build_guard import changes, sums  # noqa: E402


def lay(root, files):
    for path, text in files.items():
        full = os.path.join(root, "build", path)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, "w") as f:
            f.write(text)


def test_a_tree_with_no_build_folder_has_no_sums(tmp_path):
    assert sums(str(tmp_path)) == {}


def test_every_file_under_build_has_a_sum_by_its_path_under_it(tmp_path):
    lay(str(tmp_path), {"burner/kit/version.txt": "a", "burner/kit/page/app.js": "b", "scrivo-iot.bin": "c"})
    found = sums(str(tmp_path))
    assert sorted(found) == ["burner/kit/page/app.js", "burner/kit/version.txt", "scrivo-iot.bin"]
    assert len(set(found.values())) == 3


def test_what_is_the_same_is_no_change(tmp_path):
    lay(str(tmp_path), {"burner/kit/version.txt": "a", "burner/kit/page/app.js": "b"})
    assert changes(sums(str(tmp_path)), sums(str(tmp_path))) == []


def test_a_file_with_other_bytes_one_that_is_new_and_one_that_went_are_each_found(tmp_path):
    lay(str(tmp_path), {"burner/kit/version.txt": "a", "burner/kit/page/app.js": "b", "burner/kit/page/old.js": "c"})
    before = sums(str(tmp_path))
    lay(str(tmp_path), {"burner/kit/page/app.js": "B", "burner/kit/page/new.js": "d"})
    os.remove(os.path.join(str(tmp_path), "build", "burner/kit/page/old.js"))
    assert changes(before, sums(str(tmp_path))) == [("burner/kit/page/app.js", "changed"), ("burner/kit/page/new.js", "added"), ("burner/kit/page/old.js", "gone")]


def test_a_kit_laid_from_the_same_page_is_the_same_kit_so_laying_one_in_a_copy_changes_nothing_here(tmp_path):
    lay(str(tmp_path / "real"), {"burner/kit/page/app.js": "the page"})
    before = sums(str(tmp_path / "real"))
    lay(str(tmp_path / "copy"), {"burner/kit/page/app.js": "another page"})
    assert changes(before, sums(str(tmp_path / "real"))) == []
