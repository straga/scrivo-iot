"""tools/env.sh: the SDK is the one the builder's environment names (IDF_PATH, IDF_TOOLS_PATH), and scrivo-safe is
the folder beside the tree unless SCRIVO_SAFE says otherwise - no folder of anyone's disk in the script.

    uv run --no-project --with pytest pytest tools/test_env_sh.py
"""
import os
import shutil
import subprocess

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ZSH = shutil.which("zsh")

pytestmark = pytest.mark.skipif(ZSH is None, reason="no zsh on PATH")


def idf(base, version="v5.5.2"):
    """A stand-in IDF: an export.sh that only says it ran, and the version.txt IDF keeps in its root."""
    root = os.path.join(base, "idf_here", "esp-idf")
    os.makedirs(root)
    with open(os.path.join(root, "export.sh"), "w") as f:
        f.write("export FAKE_IDF_ACTIVE=yes\n")
    with open(os.path.join(root, "version.txt"), "w") as f:
        f.write(version + "\n")
    return root


def source_env(tmp_path, **env):
    """Sources tools/env.sh in a shell that has only what the builder gave it; prints what the script leaves."""
    clean = {"PATH": os.environ["PATH"], "HOME": str(tmp_path)}
    clean.update(env)
    script = '. "%s/env.sh" && print -r -- "idf=$FAKE_IDF_ACTIVE safe=$SCRIVO_SAFE path=$IDF_PATH"' % HERE
    return subprocess.run([ZSH, "-c", script], capture_output=True, text=True, env=clean)


def test_the_sdk_comes_from_the_environment_of_the_builder_and_scrivo_safe_from_the_place_of_the_tree(tmp_path):
    path = idf(str(tmp_path))
    said = source_env(tmp_path, IDF_PATH=path, IDF_TOOLS_PATH=str(tmp_path / "tools"))
    assert said.returncode == 0, said.stderr
    assert "idf=yes" in said.stdout and "path=%s" % path in said.stdout, said.stdout
    projects = os.path.dirname(os.path.realpath(os.path.join(HERE, "..")))
    assert "safe=%s" % os.path.join(projects, "scrivo-safe") in said.stdout, said.stdout


def test_scrivo_safe_named_by_the_builder_is_kept(tmp_path):
    said = source_env(tmp_path, IDF_PATH=idf(str(tmp_path)), IDF_TOOLS_PATH=str(tmp_path), SCRIVO_SAFE="/somewhere/scrivo-safe")
    assert said.returncode == 0, said.stderr
    assert "safe=/somewhere/scrivo-safe" in said.stdout, said.stdout


def test_without_an_sdk_in_the_environment_it_says_which_variables_to_set(tmp_path):
    said = source_env(tmp_path)
    assert said.returncode != 0
    assert "IDF_PATH" in said.stderr and "IDF_TOOLS_PATH" in said.stderr, said.stderr
    assert "idf=" not in said.stdout


def test_an_sdk_of_another_version_is_refused_naming_both(tmp_path):
    said = source_env(tmp_path, IDF_PATH=idf(str(tmp_path), version="v5.4.1"), IDF_TOOLS_PATH=str(tmp_path))
    assert said.returncode != 0
    assert "v5.4.1" in said.stderr and "v5.5.2" in said.stderr, said.stderr
