"""The verdicts of tools/page_reason_line.py, held on what a browser could answer - answers that are good and answers that are not, each of the kind the owner saw on the boiler on 31.10 ("not the way people write it, a thermostat looks like
this"). What is held is the shape of the line, never its words: a word is the menu's and a person changes it."""
import page_reason_line as r


def said_of(text, width=120, height=16, line_height=16, svg=("21.5 °C", "52 °C", "комната")):
    return {"thermo": {"ask": {"word": "x", "text": text, "box": {"right": 14 + width}, "ink": {"width": width, "height": height}, "lineHeight": line_height}, "svgTexts": list(svg)}}


def no_line():
    return {"thermo": {"ask": None, "svgTexts": []}}


# ---------------------------------------------------------------- one row


def test_a_line_that_stands_on_one_row_is_in_order_and_a_wrapped_phrase_is_named():
    ok, words = r.judge_one_row(said_of("✋ 21.5 → 22 °C"))
    assert ok, words
    ok, words = r.judge_one_row(said_of("ручной цикл · горит, пока комната не дойдёт до 22 °C · комната 21.5 °C", width=340, height=48))
    assert not ok and "wraps" in words


def test_a_line_wider_than_the_phone_is_named_though_it_is_one_row():
    ok, words = r.judge_one_row(said_of("x", width=420))
    assert not ok and "375" in words


def test_no_line_where_one_is_asked_is_named():
    assert not r.judge_one_row(no_line())[0]


# ---------------------------------------------------------------- few words


def test_numbers_units_clocks_and_signs_are_not_words_and_a_phrase_is_counted_by_its_words():
    for line in ("✋ 21.5 → 22 °C", "авто 19 → 22 °C", "авто 21.5 / 22 °C", "авто 19 / 22 °C · ждёт", "стоп"):
        ok, words = r.judge_few_words(said_of(line))
        assert ok, (line, words)
    for line in ("ждёт · комната 19 °C, цель 22 °C · пауза ещё 3:00 · раньше, если вода ниже 45 °C", "выключено кнопкой Стоп · включит Старт"):
        ok, words = r.judge_few_words(said_of(line))
        assert not ok and "words" in words, (line, words)


def test_the_english_line_is_counted_the_same_way():
    assert r.judge_few_words(said_of("auto 19 / 22 °C · waits"))[0]
    assert not r.judge_few_words(said_of("manual cycle · burning until the room reaches 22 °C · room 21.5 °C"))[0]


# ---------------------------------------------------------------- numbers


def test_the_target_and_the_readings_of_the_drawing_are_in_order_and_a_number_of_a_setting_is_named():
    ours = {"ours": {"22"}, "pause": False}
    assert r.judge_numbers(said_of("✋ 21.5 → 22 °C"), ours)[0]
    ok, words = r.judge_numbers(said_of("авто 19 / 22 °C · вода < 45 °C", svg=("21.5 °C", "52 °C")), ours)
    assert not ok and "45" in words and "19" in words


def test_a_clock_is_in_order_where_the_card_counts_a_pause_and_named_where_it_does_not():
    line = said_of("авто 19 / 22 °C · ⏸ 3:00", svg=("19.0 °C",))
    assert r.judge_numbers(line, {"ours": {"22"}, "pause": True})[0]
    ok, words = r.judge_numbers(line, {"ours": {"22"}, "pause": False})
    assert not ok and "3:00" in words and "no pause" in words


def test_a_reading_the_drawing_writes_with_a_decimal_is_the_same_number_as_the_line_writes_whole():
    assert r.judge_numbers(said_of("авто 18 → 21 °C", svg=("18.0 °C",)), {"ours": {"21"}, "pause": False})[0]
    assert not r.judge_numbers(said_of("авто 18 → 21 °C", svg=("18.5 °C",)), {"ours": {"21"}, "pause": False})[0]


def test_ours_reads_the_target_the_end_of_a_cycle_and_the_setpoint_as_the_page_writes_them():
    rows = {("demand", "target"): {"number": {"value": 22.0}}, ("thermostat", "manual_c"): {"number": {"value": 60.0}}, ("thermostat", "setpoint"): {"number": {"value": 57.5}}}
    assert r.ours(rows) == {"22", "60", "57.5"}
    assert r.ours({}) == set()


# ---------------------------------------------------------------- a number said again


def test_the_reading_and_the_target_each_once_are_in_order_and_a_number_said_again_is_named():
    ours = {"ours": {"21"}, "pause": True}
    assert r.judge_numbers(said_of("авто 19 / 21 °C · ⏸ 3:00", svg=("19.0 °C",)), ours)[0]
    assert r.judge_numbers(said_of("авто 21 / 21 °C", svg=("21.0 °C",)), {"ours": {"21"}, "pause": False})[0], "a reading that has reached the target says the same number twice, and it is two numbers"
    ok, words = r.judge_numbers(said_of("авто 19 / 21 °C · 19 °C", svg=("19.0 °C",)), {"ours": {"21"}, "pause": False})
    assert not ok and "3 numbers" in words and "said again" in words, words
    ok, words = r.judge_numbers(said_of("авто 19 / 21 °C · ⏸ 3:00 · ⏸ 2:00", svg=("19.0 °C",)), ours)
    assert not ok and "more than once" in words, words


# ---------------------------------------------------------------- the sensor "now" is read off


def test_the_first_number_is_the_reading_of_the_sensor_the_case_is_about_and_another_sensors_number_is_named():
    assert r.judge_now(said_of("✋ вода 31.1 → 60 °C"), {"now": 31.1})[0]
    ok, words = r.judge_now(said_of("✋ вода 37.6 → 60 °C"), {"now": 31.1})
    assert not ok and "37.6" in words and "31.1" in words, words


def test_a_sensor_that_reads_nothing_is_a_dash_and_a_number_there_is_named():
    assert r.judge_now(said_of("✋ вода - → 60 °C"), {"now": None})[0]
    ok, words = r.judge_now(said_of("✋ вода 37.6 → 60 °C"), {"now": None})
    assert not ok and "nothing" in words, words
    ok, words = r.judge_now(said_of("✋ вода - → 60 °C"), {"now": 31.1})
    assert not ok, words


def test_a_line_that_says_no_sign_has_no_reading_to_judge_and_is_named_where_one_is_asked():
    ok, words = r.judge_now(said_of("стоп"), {"now": 20.0})
    assert not ok and "no reading" in words


def test_role_reading_finds_the_sensor_by_the_role_the_board_says_and_not_by_its_place():
    rows = {("temperatures", "ntc_1"): {"number": {"value": 37.6}}, ("temperatures", "ntc_3"): {"number": {"value": 31.12}}, ("temperatures", "ntc_4"): {"unknown": {}}}
    settings = {"role_ntc_1": "boiler", "role_ntc_3": "return", "role_ntc_4": "room"}
    assert r.role_reading(rows, settings, "boiler") == 37.6
    assert abs(r.role_reading(rows, settings, "return") - 31.12) < 1e-9
    assert r.role_reading(rows, settings, "room") is None, "a sensor that reads nothing"
    assert r.role_reading(rows, settings, "air") is None, "a role nobody plays"
    assert r.now_of("held", rows, {**settings, "auto_hold": "return"}, {"auto_hold": "boiler"}) == r.role_reading(rows, settings, "return")
    assert r.now_of("held", rows, settings, {"auto_hold": "boiler"}) == 37.6, "the setting not set by the scenario is the default's"
    assert r.now_of("held", rows, {**settings, "auto_hold": "power"}, {}) is None, "the mode of power holds no sensor"


# ---------------------------------------------------------------- every text of `scheme/ask` is measured, and every place a number is read off is paired


def test_every_text_of_the_menus_ask_is_a_case_of_the_browser_run():
    import json
    import os

    menu = json.load(open(os.path.join(r.ROOT, "devices", "burner", "page", "menu.json")))["scheme"]["ask"]
    keys = set()
    for name, node in menu.items():
        if isinstance(node, dict) and "en" not in node:
            keys |= {"%s/%s" % (name, one) for one in node}
        else:
            keys.add(name)
    assert keys == r.ASK_TEXTS, "a text of the menu the run does not ask, or one the run asks the menu has not: %s" % (keys ^ r.ASK_TEXTS)
    said_by_cases = {key for case in r.CASES for key in case[5]}
    assert said_by_cases == r.ASK_TEXTS, "a text no case carries, or a key of a case that is no text: %s" % (said_by_cases ^ r.ASK_TEXTS)


def test_the_places_a_number_is_read_off_are_the_ones_named_and_each_has_its_pair_of_tests():
    import reason_sources_sweep as sweep_place

    expected = [
        ("degrees", "n"),  # the helper `reading`: the number of the place, said whole or with a decimal
        ("reading", 'kind === "water" ? heldPlace(settings) : "room"'),  # selected by the VALUE of the row `thermostat/manual` (room or water) and, for water, of the setting `auto_hold`; pairs: test_page_scheme.js "a cycle of the water says...", "a held sensor that is silent...", "a cycle of the room says..."
        ("degrees", "ends.value"),  # the row `thermostat/manual_c`, said only with the cycle's kind; pair: "a cycle of the room says what the burner burns for"
        ("reading", '"room"'),  # the room's reading where the row `demand/target` is a number; pairs: "the reasons that are not numbers...", "a room that reads nothing is a dash"
        ("degrees", "n.value"),  # the row `demand/target`
    ]
    found = [(call, expression) for call, expression, _ in sweep_place.sites(open(sweep_place.SCHEME).read())]
    assert found == expected, "a place a number is read off came in or went out of askOf: name its selecting value and its pair of tests, then change the list: %s" % found


def test_a_line_that_carries_the_words_of_its_case_is_in_order_and_one_of_another_state_is_named():
    assert r.judge_says(said_of("профиль выкл"), {"says": ["профиль выкл"]})[0]
    ok, words = r.judge_says(said_of("стоп"), {"says": ["профиль выкл"]})
    assert not ok and "профиль выкл" in words
    assert not r.judge_says(no_line(), {"says": ["x"]})[0]


def test_the_fixed_words_of_a_text_stand_before_its_first_place():
    menu = {"scheme": {"ask": {"cold": {"ru": "авто {now} → {to} {unit}"}, "why": {"profile_off": {"ru": "профиль выкл"}}, "manual": {"water": {"ru": "✋ вода {now} → {to}"}}}}}
    assert r.fixed_words(menu, ["cold", "why/profile_off", "manual/water"], "ru") == ["авто", "профиль выкл", "✋ вода"]
