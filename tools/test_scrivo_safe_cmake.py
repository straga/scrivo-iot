"""tools/scrivo_safe.cmake on small layouts: the tree with scrivo-safe beside it, and a copy of the tree elsewhere -
with and without SCRIVO_SAFE - run by `cmake -P`, the way an IDF component of the tree includes it.

    uv run --no-project --with pytest pytest tools/test_scrivo_safe_cmake.py
"""
import os
import shutil
import subprocess

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
CMAKE = shutil.which("cmake")

pytestmark = pytest.mark.skipif(CMAKE is None, reason="no cmake on PATH")

# What a component does with it: find scrivo-safe, name what it takes from there, say where it looked.
PROBE = """include(${CMAKE_CURRENT_LIST_DIR}/../../../tools/scrivo_safe.cmake)
scrivo_safe_dir(scrivo_safe)
scrivo_needs(probe "${scrivo_safe}/main/app_fs.c" "${scrivo_safe}/third_party/littlefs/lfs2.c")
message(STATUS "scrivo_safe=${scrivo_safe}")
"""


def write(path, text=""):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)


def scrivo_safe(at):
    """A scrivo-safe with the files the vfs component takes from it."""
    write(os.path.join(at, "main", "app_fs.c"))
    write(os.path.join(at, "third_party", "littlefs", "lfs2.c"))
    return at


def tree(at):
    """The tree's tools/scrivo_safe.cmake and a component that includes it, laid at AT."""
    write(os.path.join(at, "tools", "scrivo_safe.cmake"), open(os.path.join(HERE, "scrivo_safe.cmake")).read())
    component = os.path.join(at, "crates", "vfs", "app_fs", "probe.cmake")
    write(component, PROBE)
    return component


def run(component, env_value, extra=None):
    env = {k: v for k, v in os.environ.items() if k != "SCRIVO_SAFE"}
    if env_value is not None:
        env["SCRIVO_SAFE"] = env_value
    env.update(extra or {})
    said = subprocess.run([CMAKE, "-P", component], capture_output=True, text=True, env=env)
    # CMake wraps a long message over lines of its own choosing; the words are what is checked
    said.stderr = " ".join(said.stderr.split())
    return said


def test_the_tree_beside_scrivo_safe_finds_it_without_the_variable(tmp_path):
    safe = scrivo_safe(str(tmp_path / "scrivo-safe"))
    said = run(tree(str(tmp_path / "scrivo-iot")), None)
    assert said.returncode == 0, said.stderr
    assert "scrivo_safe=%s" % os.path.realpath(safe) in said.stdout + said.stderr


def test_a_copy_elsewhere_finds_scrivo_safe_by_the_variable(tmp_path):
    safe = scrivo_safe(str(tmp_path / "scrivo-safe"))
    said = run(tree(str(tmp_path / "worktree" / "scrivo-iot_some-branch")), safe)
    assert said.returncode == 0, said.stderr
    assert "scrivo_safe=%s" % safe in said.stdout + said.stderr


def test_a_copy_elsewhere_without_the_variable_is_refused_saying_what_to_set(tmp_path):
    scrivo_safe(str(tmp_path / "scrivo-safe"))
    said = run(tree(str(tmp_path / "worktree" / "scrivo-iot_some-branch")), None)
    assert said.returncode != 0
    assert "probe:" in said.stderr and "SCRIVO_SAFE is not set" in said.stderr, said.stderr


def test_the_variable_naming_an_empty_place_is_refused_naming_the_file_and_the_place(tmp_path):
    empty = str(tmp_path / "nothing_here")
    os.makedirs(empty)
    scrivo_safe(str(tmp_path / "scrivo-safe"))
    said = run(tree(str(tmp_path / "scrivo-iot")), empty)
    assert said.returncode != 0
    assert "app_fs.c is not there" in said.stderr, said.stderr
    assert "SCRIVO_SAFE names %s" % empty in said.stderr, said.stderr


# The vfs component itself, run with the IDF calls stubbed: what it asks of scrivo-safe, and of nothing else.
VFS_STUBS = """function(idf_component_register)
  message(STATUS "sources=${ARGN}")
endfunction()
function(target_compile_definitions)
endfunction()
function(set_source_files_properties)
endfunction()
set(COMPONENT_LIB stub)
include("%s")
"""


def run_vfs(tmp_path, variable):
    """The real crates/vfs/app_fs/CMakeLists.txt in a tree laid at tmp_path, with IDF's calls stubbed."""
    at = str(tmp_path / "scrivo-iot")
    write(os.path.join(at, "tools", "scrivo_safe.cmake"), open(os.path.join(HERE, "scrivo_safe.cmake")).read())
    real = os.path.join(HERE, "..", "crates", "vfs", "app_fs", "CMakeLists.txt")
    write(os.path.join(at, "crates", "vfs", "app_fs", "CMakeLists.txt"), open(real).read())
    script = os.path.join(at, "stubbed.cmake")
    write(script, VFS_STUBS % os.path.join(at, "crates", "vfs", "app_fs", "CMakeLists.txt"))
    return run(script, variable)


def scrivo_safe_with_littlefs(base):
    for name in ("main/app_fs.c", "main/app_fs_tree.c", "third_party/littlefs/lfs2.c", "third_party/littlefs/lfs2_util.c"):
        write(os.path.join(base, "scrivo-safe", *name.split("/")))
    return os.path.join(base, "scrivo-safe")


def test_the_vfs_component_builds_from_scrivo_safe_alone(tmp_path):
    safe = scrivo_safe_with_littlefs(str(tmp_path))
    said = run_vfs(tmp_path, safe)
    assert said.returncode == 0, said.stderr
    sources = said.stdout + said.stderr
    assert "scrivo-safe/third_party/littlefs/lfs2.c" in sources
    assert "opt/" not in sources


def test_the_vfs_component_without_littlefs_in_scrivo_safe_is_refused_naming_the_file(tmp_path):
    safe = scrivo_safe_with_littlefs(str(tmp_path))
    os.remove(os.path.join(safe, "third_party", "littlefs", "lfs2.c"))
    said = run_vfs(tmp_path, safe)
    assert said.returncode != 0
    assert "lfs2.c is not there" in said.stderr, said.stderr


GESTURE_FILES = ("gesture.c", "gesture.h", "gesture_hold.c", "gesture_hold.h", "reset_count.c", "reset_count.h")
BUTTON = {"STRAGA_GESTURE_PIN": "0", "STRAGA_GESTURE_ACTIVE": "0"}


def run_ways_down(tmp_path, drop=None):
    """The real crates/ways_down/ways_down/CMakeLists.txt in a tree laid at tmp_path with a scrivo-safe that has nothing
    in it, IDF's calls stubbed; DROP is a file of the component taken away first."""
    at = str(tmp_path / "scrivo-iot")
    write(os.path.join(at, "tools", "scrivo_safe.cmake"), open(os.path.join(HERE, "scrivo_safe.cmake")).read())
    component = os.path.join(HERE, "..", "crates", "ways_down", "ways_down")
    for name in os.listdir(component):
        if name != drop:
            write(os.path.join(at, "crates", "ways_down", "ways_down", name), open(os.path.join(component, name)).read())
    script = os.path.join(at, "stubbed.cmake")
    write(script, VFS_STUBS % os.path.join(at, "crates", "ways_down", "ways_down", "CMakeLists.txt"))
    empty = str(tmp_path / "scrivo-safe")
    os.makedirs(empty)
    return run(script, empty, BUTTON)


def test_the_ways_down_component_builds_from_its_own_folder_with_an_empty_scrivo_safe(tmp_path):
    said = run_ways_down(tmp_path)
    assert said.returncode == 0, said.stderr
    sources = said.stdout + said.stderr
    assert "crates/ways_down/ways_down/gesture.c" in sources
    assert "scrivo-safe" not in sources and "opt/" not in sources


def test_the_ways_down_component_without_one_of_its_files_is_refused(tmp_path):
    said = run_ways_down(tmp_path, drop="reset_count.c")
    assert said.returncode != 0
