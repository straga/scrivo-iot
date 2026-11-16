"""The verdicts of tools/page_target.py, held on answers of the browser that are good and answers that are not.

    uv run --no-project --with pytest pytest tools/test_page_target.py
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import page_target as t  # noqa: E402


def box(left, top, width, height):
    return {"left": left, "right": left + width, "top": top, "bottom": top + height, "width": width, "height": height}


def closed(**more):
    return {"toggle": True, "panel": False, "expanded": "false", "ask": "auto 21.7 / 22 °C", "askBox": box(14, 40, 200, 15), "askLine": 15.0, "cardHeight": 418.0, **more}


def opened(**more):
    base = {
        "panel": True, "expanded": "true", "profile": "комфорт", "value": "21", "unit": "°C",
        "valueBox": box(150, 60, 104, 40), "downBox": box(100, 58, 44, 44), "upBox": box(260, 58, 44, 44),
        "strip": True, "band": box(60, 120, 40, 6), "mark": box(100, 112, 4, 20), "gapValue": "−1", "gapFire": "20 °C",
        "wide": [], "scrollWidth": 375, "cardHeight": 520.0,
    }
    return {**base, **more}


def test_a_closed_line_is_a_button_with_no_panel():
    assert t.judge_line_is_pressable(closed())[0]


def test_a_line_that_is_no_button_fails():
    assert not t.judge_line_is_pressable(closed(toggle=False))[0]


def test_a_panel_drawn_while_closed_fails():
    assert not t.judge_line_is_pressable(closed(panel=True))[0]


def test_a_line_that_wraps_fails():
    assert not t.judge_line_is_pressable(closed(askBox=box(14, 40, 200, 31)))[0]


def test_an_open_panel_with_all_its_parts_passes():
    assert t.judge_panel_opens(opened(), {"profile": "комфорт", "value": "21", "unit": "°C"})[0]


def test_a_wrong_profile_or_number_or_unit_fails_and_says_which():
    expect = {"profile": "комфорт", "value": "21", "unit": "°C"}
    assert "profile" in t.judge_panel_opens(opened(profile="эконом"), expect)[1]
    assert "target" in t.judge_panel_opens(opened(value="22"), expect)[1]
    assert "unit" in t.judge_panel_opens(opened(unit=""), expect)[1]


def test_a_button_under_44_px_fails():
    assert not t.judge_panel_opens(opened(upBox=box(260, 58, 36, 36)), {"profile": "комфорт", "value": "21", "unit": "°C"})[0]


def test_a_panel_wider_than_the_phone_fails():
    assert not t.judge_panel_opens(opened(wide=["BUTTON +"], scrollWidth=390), {"profile": "комфорт", "value": "21", "unit": "°C"})[0]


def test_the_buttons_must_stand_either_side_of_the_number():
    swapped = opened(downBox=box(260, 58, 44, 44), upBox=box(100, 58, 44, 44))
    assert not t.judge_panel_opens(swapped, {"profile": "комфорт", "value": "21", "unit": "°C"})[0]


def test_one_write_with_the_last_number_passes_and_a_second_write_fails():
    assert t.judge_one_write([("auto_room_c", "26")], [("auto_room_c", "26")])[0]
    assert not t.judge_one_write([("auto_room_c", "22"), ("auto_room_c", "26")], [("auto_room_c", "26")])[0]
    assert not t.judge_one_write([], [("auto_room_c", "26")])[0]


def test_a_write_before_the_wait_is_over_fails():
    assert t.judge_nothing_yet([])[0]
    assert not t.judge_nothing_yet([("auto_room_c", "22")])[0]


def test_a_refusal_puts_the_board_s_number_back_and_says_so():
    assert t.judge_back_to_the_board({"value": "26", "said": "no"}, "26", "°C")[0]
    assert not t.judge_back_to_the_board({"value": "27", "said": "no"}, "26", "°C")[0], "the hand's number stayed"
    assert not t.judge_back_to_the_board({"value": "26", "said": ""}, "26", "°C")[0], "nothing said"


def test_the_card_keeps_its_height_to_a_pixel():
    assert t.judge_same_height({"cardHeight": 418.0}, {"cardHeight": 418.4})[0]
    assert not t.judge_same_height({"cardHeight": 418.0}, {"cardHeight": 470.0})[0]


def flow(**more):
    return {"hot": 2, "cold": 3, "manifold": box(188, 209, 20, 14), "texts": [{"text": "28.7 °C", "box": box(109, 186, 40, 12)}, {"text": "27.8 °C", "box": box(234, 186, 40, 12)}], **more}


def test_five_arrows_and_a_manifold_over_no_text_pass():
    assert t.judge_flow(flow())[0]


def test_four_arrows_fail():
    assert not t.judge_flow(flow(cold=2))[0]


def test_a_manifold_over_a_number_fails():
    over = flow(texts=[{"text": "28.7 °C", "box": box(180, 205, 40, 12)}])
    assert not t.judge_flow(over)[0]


def test_no_manifold_fails():
    assert not t.judge_flow(flow(manifold=None))[0]
