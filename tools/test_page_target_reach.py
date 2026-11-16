"""The verdicts of tools/page_target_reach.py, held on answers of the browser that are good and answers that are not.

    uv run --no-project --with pytest pytest tools/test_page_target_reach.py
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import page_target_reach as r  # noqa: E402


def shut(**more):
    return {"toggle": True, "panel": False, "up": False, "down": False, "value": None, "saves": 0, "inputs": 0, "scrollWidth": 375, **more}


def opened(**more):
    return {**shut(), "panel": True, "up": True, "down": True, "value": "21.0", **more}


def test_a_target_that_is_a_button_and_opens_a_panel_passes():
    assert r.judge_opens_by_a_press(shut(), opened())[0]


def test_a_target_with_no_button_fails():
    ok, words = r.judge_opens_by_a_press(shut(toggle=False), shut(toggle=False))
    assert not ok and "no button" in words


def test_a_button_that_draws_no_panel_fails():
    assert not r.judge_opens_by_a_press(shut(), shut())[0]


def test_a_panel_with_one_button_missing_fails():
    assert not r.judge_opens_by_a_press(shut(), opened(down=False))[0]


def test_a_panel_open_with_no_form_beside_it_passes():
    assert r.judge_open_with_no_form(opened(toggle=False))[0]


def test_a_page_with_no_panel_fails():
    ok, words = r.judge_open_with_no_form(shut(toggle=False))
    assert not ok and "no panel" in words


def test_a_panel_beside_a_save_button_or_a_field_fails():
    assert not r.judge_open_with_no_form(opened(toggle=False, saves=1))[0]
    assert not r.judge_open_with_no_form(opened(toggle=False, inputs=2))[0]


def test_a_panel_that_waits_for_a_press_on_a_page_with_no_line_fails():
    assert not r.judge_open_with_no_form(opened(toggle=True))[0]


def test_a_page_wider_than_the_phone_fails():
    assert not r.judge_open_with_no_form(opened(toggle=False, scrollWidth=420))[0]


def test_unread_takes_the_row_out_of_every_frame_and_keeps_the_rest():
    import json

    text = json.dumps({"frames": [{"at": 0, "values": [[3, "cold"], [4, 1]]}, {"at": 1, "values": [[4, 2]]}]})
    frames = json.loads(r.unread(text, 3))["frames"]
    assert frames[0]["values"] == [[4, 1], [3, None]]
    assert frames[1]["values"] == [[4, 2], [3, None]]
