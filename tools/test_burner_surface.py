"""The list of what a head can touch on the burner stands on the code: a word the code has gets a class or the run stops, and a class held for a word the code has not got stops it too.

    uv run --no-project --with pytest pytest tools/test_burner_surface.py
"""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import burner_surface  # noqa: E402


def test_every_word_of_the_burners_modules_and_the_shell_has_a_class():
    verbs, takes = burner_surface.verbs_and_takes()
    rows = burner_surface.console_rows(verbs, takes)
    classes = {klass for _, klass, _, _ in rows}
    assert classes <= {burner_surface.READ, burner_surface.SET, burner_surface.ACT}
    words = {form.split()[0] for form, *_ in rows}
    # what the live board lists in `help`: the shell's eleven, and the five of the modules
    assert {"status", "restart", "down", "toggle", "press", "on", "off", "fell", "log", "settings", "set", "meter", "dim", "zc", "reset", "cleaned"} <= words


def test_a_word_a_module_adds_with_no_class_stops_the_run(monkeypatch):
    verbs, takes = burner_surface.verbs_and_takes()
    with pytest.raises(SystemExit) as stopped:
        burner_surface.console_rows(verbs + [("frobnicate", "fan")], takes)
    assert stopped.value.code == 2


def test_a_class_held_for_a_word_the_code_has_not_got_stops_the_run(monkeypatch):
    monkeypatch.setitem(burner_surface.FORMS, "vanished <x>", (burner_surface.READ, "was removed"))
    verbs, takes = burner_surface.verbs_and_takes()
    with pytest.raises(SystemExit):
        burner_surface.console_rows(verbs, takes)


def test_every_route_of_the_page_has_a_class_and_the_way_is_the_codes():
    rows = {name: klass for name, klass, _, _ in burner_surface.route_rows()}
    assert rows["POST /settings"] == burner_surface.SET
    assert rows["GET /state"] == burner_surface.READ
    assert rows["POST /down/*"] == burner_surface.ACT
    assert len(rows) == 19
