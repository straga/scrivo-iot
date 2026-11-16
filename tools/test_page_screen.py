"""The verdicts of tools/page_screen.py, held on what a browser could answer - answers that are good and answers that are not, each of the kind the owner saw on the boiler on 04.10: a boiler's setpoint written over its number, a norm painted
green on every line, a number with no zone, a fault that is a row in its own card. What is held is what the page drew and painted; the browser is the tool's run."""
import copy

import page_screen as s


def box(left=0, top=0, width=100, height=20):
    return {"left": left, "top": top, "right": left + width, "bottom": top + height, "width": width, "height": height}


SVG = box(14, 60, 347, 270)
INK, OK, WARN, BAD = "rgb(30, 30, 30)", "rgb(10, 140, 60)", "rgb(200, 120, 0)", "rgb(200, 30, 30)"


def said_of(**screen):
    base = {
        "ink": INK, "ok": OK, "warn": WARN, "bad": BAD,
        "levels": [{"level": "ok", "color": INK, "text": "on"}, {"level": "dim", "color": "rgb(150, 150, 150)", "text": "-"}],
        "greens": [],
        "badge": [{"mode": "boiler", "text": "boiler 44 °C −4", "box": box(120, 33, 100, 14), "inSvg": False, "icon": {"box": box(106, 34, 10, 12), "kind": "thermo"}}],
        "header": {"title": box(22, 30, 50, 20), "mode": box(260, 24, 100, 32)},
        "pellets": [
            {"key": "today", "text": "1.2", "box": box(30, 160, 20, 12), "icon": {"box": box(18, 161, 10, 10), "kind": "today"}, "fontPx": 11},
            {"key": "yesterday", "text": "3.3", "box": box(30, 174, 20, 12), "icon": {"box": box(18, 175, 10, 10), "kind": "yesterday"}, "fontPx": 11},
            {"key": "month", "text": "37.6", "box": box(30, 188, 20, 12), "icon": {"box": box(18, 189, 10, 10), "kind": "month"}, "fontPx": 11},
            {"key": "total", "text": "412.4", "box": box(30, 202, 20, 12), "icon": {"box": box(18, 203, 10, 10), "kind": "total"}, "fontPx": 11},
        ],
        "zones": [],
        "attention": None,
        "schemeCard": box(8, 20, 359, 330),
    }
    base.update(screen)
    return {"screen": base, "thermo": {"svg": SVG, "svgTexts": ["hopper", "44.0 °C", "boiler"], "wide": 375}}


def zone(name, mark_at, band=(0.4, 0.6), fill=INK):
    """A zone of a bar 40 px long from x 100: the band as shares of it, the mark's middle at a share."""
    bar = box(100, 120, 40, 3)
    band_box = box(100 + 40 * band[0], 120, 40 * (band[1] - band[0]), 3)
    mark = box(100 + 40 * mark_at - 1, 118, 2, 7)
    return {"name": name, "bar": bar, "band": band_box, "mark": mark, "markFill": fill}


# ---------------------------------------------------------------- the badge

BADGE = {"mode": "boiler", "text": "boiler 44 °C −4"}


def badge_of(**changes):
    """The fixture's badge with CHANGES, as the one badge of an answer."""
    one = copy.deepcopy(said_of()["screen"]["badge"][0])
    for key, value in changes.items():
        one[key] = value
    return said_of(badge=[one])


def test_a_badge_in_the_title_row_with_its_icon_before_the_words_is_in_order():
    ok, words = s.judge_badge(said_of(), BADGE)
    assert ok, words


def test_the_badge_of_each_of_the_four_modes_is_in_order_in_the_title_row():
    # the badge is the same one in the boiler, the return, the air and the power: only its words move
    for mode, text in (("boiler", "boiler 44 °C −4"), ("return", "return 40 °C −2"), ("room", "air 21 °C −1"), ("power", "power 6 kW")):
        said = badge_of(mode=mode, text=text)
        ok, words = s.judge_badge(said, {"mode": mode, "text": text})
        assert ok, "%s: %s" % (mode, words)


def test_a_badge_that_says_other_words_or_is_the_badge_of_another_mode_is_named():
    ok, words = s.judge_badge(said_of(), {"mode": "boiler", "text": "boiler 45 °C −4"})
    assert not ok and "not 'boiler 45 °C −4'" in words
    ok, words = s.judge_badge(said_of(), {"mode": "return", "text": "boiler 44 °C −4"})
    assert not ok and "not of 'return'" in words


def test_no_badge_or_two_are_named():
    assert "0 badges" in s.judge_badge(said_of(badge=[]), BADGE)[1]
    one = said_of()["screen"]["badge"][0]
    assert "2 badges" in s.judge_badge(said_of(badge=[one, one]), BADGE)[1]


def test_a_badge_drawn_inside_the_scheme_is_named_and_so_is_its_text_among_the_texts_of_the_drawing():
    # 04.10-05.10: it stood in the top of the drawing right over the boiler's number and read as its caption; in the air mode it would have lied
    ok, words = s.judge_badge(badge_of(inSvg=True), BADGE)
    assert not ok and "inside the scheme" in words
    said = said_of()
    said["thermo"]["svgTexts"].append("boiler 44 °C −4")
    ok, words = s.judge_badge(said, BADGE)
    assert not ok and "the drawing still says" in words


def test_a_badge_not_in_the_title_row_is_named_over_the_drawing_off_the_line_left_of_the_title_or_under_the_buttons():
    over = badge_of(box=box(120, 62, 100, 14), icon={"box": box(106, 63, 10, 12), "kind": "thermo"})
    ok, words = s.judge_badge(over, BADGE)
    assert not ok and "over the drawing" in words and "line of the title" in words
    left = badge_of(box=box(30, 33, 100, 14), icon={"box": box(16, 34, 10, 12), "kind": "thermo"})
    assert "end of the title" in s.judge_badge(left, BADGE)[1]
    under = badge_of(box=box(200, 33, 100, 14))
    assert "buttons" in s.judge_badge(under, BADGE)[1]


def test_a_page_with_no_title_row_found_does_not_pass_the_badge_by_saying_nothing():
    ok, words = s.judge_badge(said_of(header={}), BADGE)
    assert not ok and "was not judged" in words


def test_a_title_row_with_no_buttons_holds_the_badge_to_the_edge_of_the_card_only():
    said = said_of(header={"title": box(22, 30, 50, 20), "mode": None})
    ok, words = s.judge_badge(said, BADGE)
    assert ok, words


def test_a_badge_with_no_icon_a_tiny_one_or_one_not_before_the_words_is_named():
    bare = badge_of(icon=None)
    assert "no icon" in s.judge_badge(bare, BADGE)[1]
    tiny = badge_of(icon={"box": box(106, 34, 6, 6), "kind": "thermo"})
    assert "under 9" in s.judge_badge(tiny, BADGE)[1]
    after = badge_of(icon={"box": box(225, 34, 10, 12), "kind": "thermo"})
    assert "before the words" in s.judge_badge(after, BADGE)[1]


def test_the_old_setpoint_at_the_boiler_is_named_in_both_languages():
    # 04.10: "set 60 °C −4" stood over the boiler's number and over its name
    for old in ("set 60 °C −4", "уст. 60 °C −4"):
        said = said_of()
        said["thermo"]["svgTexts"].append(old)
        ok, words = s.judge_badge(said, BADGE)
        assert not ok and old in words
    said = said_of()
    said["thermo"]["svgTexts"].append("уст. 51 Па")  # what the fan is asked for: not a setpoint at the boiler
    assert s.judge_badge(said, BADGE)[0]


# ---------------------------------------------------------------- the pellets

TEXTS = ["1.2", "3.3", "37.6", "412.4"]


def test_four_numbers_at_the_hopper_each_beside_its_icon_are_in_order():
    ok, words = s.judge_pellets(said_of(), TEXTS)
    assert ok, words


def test_a_number_that_says_other_than_the_numbers_or_a_word_among_them_is_named():
    ok, words = s.judge_pellets(said_of(), ["1.3", "3.3", "37.6", "412.4"])
    assert not ok and "today says '1.2', not '1.3'" in words
    said = said_of()
    said["screen"]["pellets"][2]["text"] = "37.6 kg"
    assert "says a word" in s.judge_pellets(said, ["1.2", "3.3", "37.6 kg", "412.4"])[1]


def test_numbers_not_at_the_hopper_too_small_or_without_their_icons_are_named():
    far = said_of()
    far["screen"]["pellets"][0]["box"] = box(300, 160, 20, 12)
    assert "does not stand at the hopper" in s.judge_pellets(far, TEXTS)[1]
    small = said_of()
    small["screen"]["pellets"][3]["fontPx"] = 8
    assert "under 10" in s.judge_pellets(small, TEXTS)[1]
    bare = said_of()
    bare["screen"]["pellets"][1]["icon"] = None
    assert "yesterday has no icon" in s.judge_pellets(bare, TEXTS)[1]
    shared = said_of()
    shared["screen"]["pellets"][1]["icon"]["kind"] = "today"
    assert "share an icon" in s.judge_pellets(shared, TEXTS)[1]


def test_pellets_where_the_board_has_none_are_named_and_none_where_none_are_asked_is_in_order():
    assert s.judge_pellets(said_of(pellets=[]), None)[0]
    assert not s.judge_pellets(said_of(), None)[0]
    assert "today, yesterday, month and total were asked" in s.judge_pellets(said_of(pellets=[]), TEXTS)[1]


# ---------------------------------------------------------------- the norm in ink

def test_a_norm_in_the_ink_of_the_title_and_no_green_in_the_cards_is_in_order():
    ok, words = s.judge_norm_in_ink(said_of(), False)
    assert ok, words


def test_a_norm_painted_green_on_a_line_is_named():
    # 04.10: every line in order was green
    ok, words = s.judge_norm_in_ink(said_of(levels=[{"level": "ok", "color": OK, "text": "on"}]), False)
    assert not ok and "'on' in order is painted" in words


def test_green_text_left_in_a_card_is_named():
    ok, words = s.judge_norm_in_ink(said_of(greens=["burning"]), False)
    assert not ok and "green text in the cards" in words


def test_a_warning_and_a_failure_keep_the_colours_of_their_levels_and_a_deviation_not_seen_is_named():
    levels = [{"level": "ok", "color": INK, "text": "on"}, {"level": "warn", "color": WARN, "text": "weak"}, {"level": "bad", "color": BAD, "text": "no-fuel"}]
    assert s.judge_norm_in_ink(said_of(levels=levels), True)[0]
    painted_ink = [{"level": "ok", "color": INK, "text": "on"}, {"level": "bad", "color": INK, "text": "no-fuel"}]
    ok, words = s.judge_norm_in_ink(said_of(levels=painted_ink), True)
    assert not ok and "'no-fuel' at bad" in words
    ok, words = s.judge_norm_in_ink(said_of(), True)
    assert not ok and "none was seen" in words


def test_a_page_with_no_value_in_order_has_judged_nothing_and_does_not_pass():
    ok, words = s.judge_norm_in_ink(said_of(levels=[]), False)
    assert not ok and "nothing was judged" in words


# ---------------------------------------------------------------- the zones

def test_a_mark_in_its_band_and_one_out_of_it_are_each_in_order_where_the_numbers_say_so():
    inside = said_of(zones=[zone("draught", 0.5)])
    assert s.judge_zones(inside, {"draught": {"inside": True, "warn": False}, "water": None, "flame": None})[0]
    outside = said_of(zones=[zone("draught", 0.9, fill=WARN)])
    assert s.judge_zones(outside, {"draught": {"inside": False, "warn": True}, "water": None, "flame": None})[0]


def test_a_mark_on_the_wrong_side_of_its_band_is_named():
    out = said_of(zones=[zone("flame", 0.9)])
    ok, words = s.judge_zones(out, {"draught": None, "water": None, "flame": {"inside": True, "warn": False}})
    assert not ok and "flame: the mark stands out of the band, the numbers say in" in words
    inside = said_of(zones=[zone("flame", 0.5)])
    ok, words = s.judge_zones(inside, {"draught": None, "water": None, "flame": {"inside": False, "warn": False}})
    assert not ok and "stands in the band, the numbers say out" in words


def test_the_warning_colour_goes_only_on_a_mark_that_is_out_while_the_fire_works():
    out_calm = said_of(zones=[zone("flame", 0.9, fill=WARN)])
    ok, words = s.judge_zones(out_calm, {"draught": None, "water": None, "flame": {"inside": False, "warn": False}})
    assert not ok and "in the warning colour, it should be out of" in words
    out_plain = said_of(zones=[zone("flame", 0.9)])
    ok, words = s.judge_zones(out_plain, {"draught": None, "water": None, "flame": {"inside": False, "warn": True}})
    assert not ok and "not in the warning colour, it should be in" in words


def test_a_green_mark_a_zone_that_is_missing_or_one_that_has_no_number_to_stand_on_are_named():
    ok, words = s.judge_zones(said_of(zones=[zone("flame", 0.5, fill=OK)]), {"draught": None, "water": None, "flame": {"inside": True, "warn": False}})
    assert not ok and "the mark is green" in words
    ok, words = s.judge_zones(said_of(), {"draught": None, "water": None, "flame": {"inside": True, "warn": False}})
    assert not ok and "flame has no zone" in words
    ok, words = s.judge_zones(said_of(zones=[zone("water", 0.5)]), {"draught": None, "water": None, "flame": None})
    assert not ok and "water has a zone where there is none to draw" in words


def test_a_bar_or_a_band_too_small_to_see_or_a_band_out_of_its_bar_are_named():
    hair = zone("draught", 0.5, band=(0.5, 0.52))
    assert "under 24 and 3" in s.judge_zones(said_of(zones=[hair]), {"draught": {"inside": True, "warn": False}, "water": None, "flame": None})[1]
    wide = zone("draught", 0.5)
    wide["band"] = box(90, 120, 20, 3)
    assert "the band leaves the bar" in s.judge_zones(said_of(zones=[wide]), {"draught": {"inside": True, "warn": False}, "water": None, "flame": None})[1]
    off = zone("draught", 0.5)
    off["mark"] = box(160, 118, 2, 7)
    assert "the mark leaves the bar" in s.judge_zones(said_of(zones=[off]), {"draught": {"inside": True, "warn": False}, "water": None, "flame": None})[1]


# ---------------------------------------------------------------- the expectations, from the numbers

HELD = {"auto_hold": "boiler", "auto_hold_c": "60", "auto_drop_c": "4", "fan_band": "4", "flame_steady": "1100", "role_ntc_1": "boiler"}


def rows_of(stage="working", pa=51, asked=51, mv=900, water=58, setpoint=None):
    rows = {("automaton", "doing"): {"word": stage}, ("draught", "blast_pressure"): {"number": {"value": pa, "unit": "Pa"}},
            ("automaton", "fan"): {"number": {"value": asked, "unit": "Pa"}}, ("flame", "pin"): {"number": {"value": mv, "unit": "mV"}},
            ("temperatures", "ntc_1"): {"number": {"value": water, "unit": "°C"}}}
    if setpoint is not None:
        rows[("thermostat", "setpoint")] = {"number": {"value": setpoint, "unit": "°C"}}
    return rows


def test_the_zones_the_numbers_make_are_in_or_out_and_warn_only_where_the_fire_works():
    got = s.zones_expect(rows_of(), HELD)
    assert got == {"draught": {"inside": True, "warn": False}, "water": {"inside": True, "warn": False}, "flame": {"inside": True, "warn": False}}
    got = s.zones_expect(rows_of(pa=75, mv=1800, water=40), HELD)
    assert got["draught"] == {"inside": False, "warn": True} and got["flame"] == {"inside": False, "warn": True} and got["water"] == {"inside": False, "warn": False}
    got = s.zones_expect(rows_of(stage="rising", pa=75, mv=3150), HELD)
    assert got["draught"]["warn"] is False and got["flame"]["warn"] is False


def test_a_mark_that_has_not_stood_out_long_enough_is_not_expected_in_the_warning_colour():
    # the page colours a mark after ten seconds out: asked at the first seconds out, it must not be coloured yet
    got = s.zones_expect(rows_of(pa=75, mv=1800), HELD, dwelled=False)
    assert got["draught"] == {"inside": False, "warn": False} and got["flame"] == {"inside": False, "warn": False}
    assert s.zones_expect(rows_of(pa=75, mv=1800), HELD, dwelled=True)["draught"] == {"inside": False, "warn": True}


def test_a_zone_is_none_where_its_numbers_are_not_there():
    assert s.zones_expect(rows_of(), {**HELD, "auto_hold": "return"})["water"] is None
    assert s.zones_expect(rows_of(), {**HELD, "role_ntc_1": "free"})["water"] is None
    rows = rows_of()
    del rows[("automaton", "fan")]
    assert s.zones_expect(rows, HELD)["draught"] is None


def test_the_badge_is_expected_from_the_setpoint_held_now_and_the_step_of_the_rung_by_power():
    menu = {"scheme": {"badge": {"temp": "{what} {to} {unit} −{drop}", "power": "{what} {kw} kW"}, "hold": {"boiler": "boiler", "power": "power"}}, "units": {"°C": "°C"}}
    assert s.badge_expect(rows_of(setpoint=44), menu, "en", HELD, [6, 5, 3]) == {"mode": "boiler", "text": "boiler 44 °C −4"}
    rows = rows_of()
    rows[("thermostat", "rung")] = {"word": "mid"}
    assert s.badge_expect(rows, menu, "en", {**HELD, "auto_hold": "power"}, [6, 5, 3]) == {"mode": "power", "text": "power 5 kW"}


def test_the_badge_is_expected_from_the_setpoint_of_the_frame_and_not_the_one_of_the_settings():
    # a window of the schedule holds eco at 48 while the setting of the comfort setpoint is 60: the badge must say 48
    menu = {"scheme": {"badge": {"temp": "{what} {to} {unit} −{drop}", "power": "{what} {kw} kW"}, "hold": {"boiler": "boiler", "room": "air"}}, "units": {"°C": "°C"}}
    assert s.badge_expect(rows_of(setpoint=48), menu, "en", HELD, [6, 5, 3])["text"] == "boiler 48 °C −4"
    # no setpoint row said: the setting is what the badge holds by
    assert s.badge_expect(rows_of(), menu, "en", HELD, [6, 5, 3])["text"] == "boiler 60 °C −4"
    assert s.badge_expect(rows_of(), menu, "en", {**HELD, "auto_hold": "room"}, [6, 5, 3]) == {"mode": "room", "text": "air 60 °C −4"}


def test_the_pellets_are_expected_each_to_a_tenth():
    rows = {("pellets", "today"): {"number": {"value": 1.234}}, ("pellets", "yesterday"): {"number": {"value": 3.31}}, ("pellets", "month"): {"number": {"value": 37.6}}, ("pellets", "total"): {"number": {"value": 412.4}}}
    assert s.pellets_expect(rows) == ["1.2", "3.3", "37.6", "412.4"]
    small = {("pellets", "today"): {"number": {"value": 0.04}}, ("pellets", "yesterday"): {"number": {"value": 0.3}}, ("pellets", "month"): {"number": {"value": 0.7}}, ("pellets", "total"): {"number": {"value": 0.696}}}
    assert s.pellets_expect(small) == ["0.0", "0.3", "0.7", "0.7"], "a total under a kilo is not '1' beside a day of 0.7"


def test_a_yesterday_the_board_does_not_know_is_expected_as_a_dash_in_its_place():
    rows = {("pellets", "today"): {"number": {"value": 1.2}}, ("pellets", "yesterday"): {"unknown": "none_yet"}, ("pellets", "month"): {"number": {"value": 37.6}}, ("pellets", "total"): {"number": {"value": 412.4}}}
    assert s.pellets_expect(rows) == ["1.2", "-", "37.6", "412.4"]
    assert s.pellets_expect({k: v for k, v in rows.items() if k[1] != "yesterday"}) == ["1.2", "-", "37.6", "412.4"], "a board without the row is a dash too"


# ---------------------------------------------------------------- the card of what asks

ASKING = [{"key": "automaton/doing", "action": "Fill the hopper. Press Stop to acknowledge.", "level": "bad"}]


def card(items, border=BAD, top=360):
    return {"title": "Needs attention", "box": box(8, top, 359, 120), "border": border, "items": items}


ITEM = {"key": "automaton/doing", "what": "fault · no-fuel", "action": "Fill the hopper. Press Stop to acknowledge."}


def test_no_card_while_nothing_asks_and_a_card_with_the_asked_row_and_action_are_in_order():
    assert s.judge_attention(said_of(), [], "Needs attention")[0]
    ok, words = s.judge_attention(said_of(attention=card([ITEM])), ASKING, "Needs attention")
    assert ok, words


def test_a_card_that_stands_with_nothing_to_ask_or_is_missing_where_something_asks_is_named():
    ok, words = s.judge_attention(said_of(attention=card([ITEM])), [], "Needs attention")
    assert not ok and "with nothing to ask" in words
    ok, words = s.judge_attention(said_of(), ASKING, "Needs attention")
    assert not ok and "nothing shows what asks for a person" in words


def test_a_row_without_its_action_or_with_another_one_or_another_list_is_named():
    ok, words = s.judge_attention(said_of(attention=card([{**ITEM, "action": ""}])), ASKING, "Needs attention")
    assert not ok and "the action is ''" in words
    ok, words = s.judge_attention(said_of(attention=card([{**ITEM, "key": "flue/flue_temp"}])), ASKING, "Needs attention")
    assert not ok and "the card lists ['flue/flue_temp']" in words


def test_a_card_not_outlined_in_the_colour_of_the_worst_or_standing_over_the_scheme_is_named():
    ok, words = s.judge_attention(said_of(attention=card([ITEM], border=WARN)), ASKING, "Needs attention")
    assert not ok and "outlined" in words
    ok, words = s.judge_attention(said_of(attention=card([ITEM], top=100)), ASKING, "Needs attention")
    assert not ok and "does not stand under the card of the scheme" in words
    warn_only = [{"key": "automaton/doing", "action": "Check.", "level": "warn"}]
    assert s.judge_attention(said_of(attention=card([{**ITEM, "action": "Check."}], border=WARN)), warn_only, "Needs attention")[0]


def test_a_card_titled_otherwise_is_named():
    ok, words = s.judge_attention(said_of(attention=card([ITEM])), ASKING, "Требует внимания")
    assert not ok and "titled" in words


MENU = {"scheme": {"attention": {"title": "Needs attention", "ack": "Press Stop.", "other": "Check it.",
                                 "why": {"no_fuel": "Fill the hopper."}, "rows": {"thermostat/silent_return": "Check the return sensor.", "automaton/alarm": "Cool the boiler."}}}}
DICT = {"things": [{"card": "automaton", "name": "doing"}, {"card": "thermostat", "name": "silent_return"}, {"card": "automaton", "name": "alarm"}]}


def test_what_asks_is_expected_from_the_codes_and_the_rows_in_the_dictionary_s_order():
    rows = {("automaton", "doing"): {"word": "fault"}, ("thermostat", "silent_return"): {"seconds": 12}, ("automaton", "alarm"): {"on": True}}
    got = s.attention_expect(rows, "no_fuel", MENU, "en", DICT)
    assert [(i["key"], i["action"], i["level"]) for i in got] == [("automaton/doing", "Fill the hopper. Press Stop.", "bad"), ("thermostat/silent_return", "Check the return sensor.", "bad"), ("automaton/alarm", "Cool the boiler.", "bad")]


def test_a_burning_off_with_a_reason_asks_without_the_stop_and_a_quiet_board_asks_for_nothing():
    got = s.attention_expect({("automaton", "doing"): {"word": "burning_off_because"}}, "no_fuel", MENU, "en", DICT)
    assert got == [{"key": "automaton/doing", "action": "Fill the hopper.", "level": "warn"}]
    quiet = {("automaton", "doing"): {"word": "working"}, ("thermostat", "silent_return"): {"unknown": {}}, ("automaton", "alarm"): {"unknown": {}}}
    assert s.attention_expect(quiet, None, MENU, "en", DICT) == []
    assert s.attention_expect({("automaton", "doing"): {"word": "fault"}}, "something_new", MENU, "en", DICT)[0]["action"] == "Check it. Press Stop."


def test_a_page_wider_than_the_phone_is_named():
    assert s.judge_fits(said_of())[0]
    wide = said_of()
    wide["thermo"]["wide"] = 412
    assert not s.judge_fits(wide)[0]


def test_frames_are_patched_by_the_rows_name_in_every_frame():
    import json
    text = json.dumps({"frames": [{"at": 1, "values": [[0, 1.0], [1, 2.0]]}, {"at": 2, "values": [[0, 1.0]]}]})
    dictionary = {"things": [{"card": "a", "name": "x"}, {"card": "a", "name": "y", "part_of": "x"}, {"card": "b", "name": "z"}]}
    got = json.loads(s.patched(text, dictionary, {("b", "z"): 9.0}))
    assert [dict((a, v) for a, v in f["values"])[2] for f in got["frames"]] == [9.0, 9.0]
    assert dict((a, v) for a, v in got["frames"][0]["values"])[0] == 1.0, "what was not named is as it was"
