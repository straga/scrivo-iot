"""A run of the tests leaves `build/` of the real tree as it found it (tools/build_guard.py): the sums of its files are taken when the run starts and when it ends, and a run in which they differ fails, naming the files."""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_guard  # noqa: E402

ROOT = os.path.dirname(HERE)


def pytest_sessionstart(session):
    session.config._build_before = build_guard.sums(ROOT)


def pytest_sessionfinish(session, exitstatus):
    before = getattr(session.config, "_build_before", None)
    if before is None:
        return
    moved = build_guard.changes(before, build_guard.sums(ROOT))
    if moved:
        lines = "\n".join("  build/%s: %s" % (path, how) for path, how in moved[:20])
        sys.stderr.write("\nBUILD GUARD: the run changed what lies in build/ of the real tree (%d files):\n%s\n" % (len(moved), lines))
        session.exitstatus = pytest.ExitCode.TESTS_FAILED
