"""tools/module_compare.py: what it calls the same and what it calls different, which entities it holds the two burners to - on frames and scenarios made up here, never on the burners' own."""
from module_compare import differences, entities_of, for_the_new_burner, named, without_what_only_the_new_one_reads

DICT = {"things": [
    {"card": "flame", "name": "flame"},
    {"card": "flame", "name": "pin"},
    {"card": "flame", "name": "spread", "part_of": "pin"},
    {"card": "relays", "name": "pump"},
    {"card": "temperatures", "name": "ntc_1"},
    {"card": "temperatures", "name": "role", "part_of": "ntc_1"},
    {"card": "flue", "name": "flue_temp"},
]}
FLAME = ["flame/flame", "flame/pin", "flame/pin.spread"]


def frames(*seconds):
    return [{"values": s} for s in seconds]


def test_a_value_is_read_through_the_dictionary_of_its_own_side():
    # the same words, the entity at another address on the other side
    other = {"things": [{"card": "relays", "name": "pump"}, {"card": "flame", "name": "flame"}]}
    assert named(DICT, frames([[0, "out"], [3, True]]), FLAME) == [{"flame/flame": "out"}]
    assert named(other, frames([[1, "out"], [0, True]]), FLAME) == [{"flame/flame": "out"}]


def test_what_a_second_does_not_mention_stands_as_the_earlier_one_said_it():
    got = named(DICT, frames([[0, "out"], [1, 3150]], [[1, 2000]]), FLAME)
    assert got == [{"flame/flame": "out", "flame/pin": 3150}, {"flame/flame": "out", "flame/pin": 2000}]


def test_only_the_entities_asked_for_are_read_from_a_frame():
    got = named(DICT, frames([[0, "out"], [4, 61.5], [5, "boiler"], [6, 148.5]]), ["temperatures/ntc_1", "flue/flue_temp"])
    assert got == [{"temperatures/ntc_1": 61.5, "flue/flue_temp": 148.5}]


def test_two_sides_that_say_the_same_have_no_difference():
    side = named(DICT, frames([[0, "out"], [1, 3150], [2, 0.0]]), FLAME)
    assert differences(side, side, FLAME) == []


def test_a_second_where_one_entity_differs_is_named_with_both_sayings():
    a = [{"flame/flame": "weak", "flame/pin": 1800}]
    b = [{"flame/flame": "steady", "flame/pin": 1800}]
    assert differences(a, b, FLAME) == [(0, "flame/flame", "weak", "steady")]


def test_a_difference_in_an_entity_nobody_asked_about_is_none():
    a = [{"flame/flame": "weak", "temperatures/ntc_2": 20.0}]
    b = [{"flame/flame": "weak", "temperatures/ntc_2": 99.0}]
    assert differences(a, b, FLAME) == []


def test_an_entity_one_side_never_says_is_a_difference_and_so_are_seconds_one_side_lacks():
    assert differences([{"flame/flame": "out"}], [{}], FLAME)[0][1:] == ("flame/flame", "out", "<nothing>")
    assert differences([{}, {}], [{}], FLAME) == [(1, "seconds", 2, 1)]


# ---- which entities the two burners are held to: those the first burner's stand simulates ----

def test_the_water_is_held_to_the_sockets_whose_roles_the_first_burner_reads_and_the_flue_to_its_one_row():
    roles = {"role_ntc_1": "boiler", "role_ntc_2": "supply", "role_ntc_3": "return", "role_ntc_4": "room", "role_ntc_5": "own:the shed"}
    got = entities_of({"settings": roles}, ["water", "flue"])
    assert got == ["temperatures/ntc_1", "temperatures/ntc_3", "temperatures/ntc_4", "flue/flue_temp"]


def test_a_socket_nobody_gave_a_role_is_no_entity_of_the_water():
    assert entities_of({"settings": {}}, ["water"]) == []
    assert entities_of({}, ["flame"]) == ["flame/flame", "flame/pin", "flame/pin.spread", "flame/pin.edge", "flame/pin.window"]


def test_what_only_the_new_burner_reads_is_taken_out_of_a_scenario_before_either_side_plays_it():
    scenario = {"seconds": 5, "boiler_c": [[0, 1]], "empty_sockets": ["ntc_6"], "bus_silent": [[1, 2]], "flue_open": [[3, 4]], "flue_silent": [[5, 6]], "supply_c": [[0, 2]]}
    assert without_what_only_the_new_one_reads(scenario) == {"seconds": 5, "boiler_c": [[0, 1]], "supply_c": [[0, 2]]}


# ---- the draught: the first burner's stand makes up pascals, the new one's reads millivolts, so the pin is made up from the pascals ----

from module_compare import draught_line, whole


def test_a_pressure_becomes_the_millivolts_a_pin_behind_a_half_divider_would_show():
    # 112.5 Pa: 0.5 V + 112.5/300 of the 4 V span = 2000 mV out of the sensor, half of it on the pin
    assert draught_line([112.5]) == [[0, 1000.0]]
    assert draught_line([0.0]) == [[0, 250.0]]


def test_the_top_of_the_scale_the_stopped_fan_and_nothing_read_are_three_pins_of_their_own():
    got = draught_line([{"at_least": 300.0}, {"unknown": "own_zero"}, None, {"unknown": "not_read"}])
    assert got == [[0, 2400.0], [1, 161.0], [2, None], [3, None]]


def test_a_second_the_first_burner_does_not_mention_has_the_pin_it_had():
    assert draught_line([50.0, "<nothing>", 50.0]) == [[0, 583.33], [1, 583.33], [2, 583.33]]


def test_pascals_are_compared_as_the_card_shows_them_whole_and_codes_as_they_are():
    assert whole(11.6) == 12 and whole(12.4) == 12
    assert whole({"at_least": 300.0}) == {"at_least": 300.0}
    assert whole({"unknown": "own_zero"}) == {"unknown": "own_zero"}
    assert whole("<nothing>") == "<nothing>"


def test_the_draught_is_one_entity_asked_for_by_name():
    assert entities_of({}, ["draught"]) == ["draught/blast_pressure"]


# ---- the relays: the first burner's stand has its automaton close the organs, the new one's machine takes commands, so the commands are made up from the closings ----

from module_compare import relay_commands


def test_a_closing_is_an_on_at_its_second_and_an_opening_an_off():
    assert relay_commands([False, False, True, True, False]) == [[2, "on"], [4, "off"]]


def test_an_organ_closed_from_the_first_second_is_commanded_on_then():
    assert relay_commands([True, True, True]) == [[0, "on"]]


def test_a_second_the_first_burner_does_not_mention_leaves_the_organ_as_it_was():
    assert relay_commands([False, "<nothing>", True, "<nothing>", "<nothing>", False]) == [[2, "on"], [5, "off"]]


def test_an_organ_that_never_closes_is_never_commanded():
    assert relay_commands([False, False]) == []


def test_the_organs_are_held_to_what_the_first_burners_stand_says_of_them():
    assert entities_of({}, ["relays"]) == ["relays/auger", "relays/auger.off_in", "relays/igniter", "relays/igniter.off_in", "relays/pump"]


def test_the_numbers_both_sides_run_on_are_the_ones_the_first_burners_state_keeps():
    from module_compare import state_settings
    d = {"things": [{"card": "settings", "name": "auger_limit"}, {"card": "settings", "name": "auto_igniter"}, {"card": "settings", "name": "role", "part_of": "auger_limit"},
                    {"card": "relays", "name": "auger"}]}
    state = {"values": [[0, {"text": "45"}], [1, {"text": "120"}], [2, "x"], [3, True]]}
    assert state_settings(d, state, ["auger_limit", "auto_igniter", "no_such"]) == {"auger_limit": "45", "auto_igniter": "120"}


# ---- the first burner's pair is the numbers of the step the new burner has chosen ----
PAIR = {"auto_pa": "51", "auto_feed_on_s": "1", "auto_feed_off_s": "24"}


def test_in_a_hold_by_a_temperature_the_pair_the_first_burner_feeds_by_is_the_chosen_steps_numbers_and_no_pair_is_left():
    for hold in ("boiler", "return", "room"):
        given = dict(PAIR, auto_hold=hold, auto_step="8", auto_purge="30")
        got = for_the_new_burner(given)
        assert (got["auto_8_pa"], got["auto_8_on_s"], got["auto_8_off_s"]) == ("51", "1", "24"), hold
        assert not set(PAIR) & set(got), hold
        assert (got["auto_hold"], got["auto_step"], got["auto_purge"]) == (hold, "8", "30"), "what is not the pair moved: %s" % got


def test_in_a_hold_of_power_the_first_burner_burns_by_its_rule_for_the_level_and_not_by_the_pair():
    got = for_the_new_burner(dict(PAIR, auto_hold="power", auto_step="3"))
    assert (got["auto_3_pa"], got["auto_3_on_s"], got["auto_3_off_s"]) == ("11", "2", "43")
    assert not set(PAIR) & set(got)


def test_a_level_nobody_chose_is_the_sixth_and_a_level_the_table_has_not_gets_no_number():
    assert for_the_new_burner(dict(PAIR, auto_hold="boiler"))["auto_6_pa"] == "51"
    got = for_the_new_burner(dict(PAIR, auto_hold="boiler", auto_step="7"))
    assert not [k for k in got if k.startswith("auto_7_")] and not set(PAIR) & set(got)
