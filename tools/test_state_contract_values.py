"""/state of the contract of the document against the board's own dictionary: every address it describes has a value, each value of a
kind and a code its entry allows, and nothing the dictionary has not got."""
import copy

from state_contract import values_check

DICTIONARY = {
    "contract": 10,
    "version": "stragabases3-26.09.24.14+d655537b",
    "things": [
        {"card": "system", "name": "console", "kinds": ["number"], "unit": "consoles"},
        {"card": "relays", "name": "pump", "kinds": ["on"], "can": ["switch"]},
        {"card": "flame", "name": "flame", "kinds": ["word", "unknown"], "words": ["out", "steady"], "reasons": ["not_read"]},
        {"card": "flame", "name": "spread", "part_of": "pin", "kinds": ["number"], "unit": "mV"},
        {"card": "automaton", "name": "stop", "kinds": ["button"], "can": ["press"]},
        {"card": "automaton_fixed", "name": "alarm_c", "kinds": ["number"], "unit": "°C", "fixed": 95},
        {"card": "automaton", "name": "for", "kinds": ["seconds"]},
        {"card": "automaton", "name": "refused", "kinds": ["text"]},
        {"card": "draught", "name": "blast_pressure", "kinds": ["number", "at_least", "unknown"], "unit": "Pa", "reasons": ["own_zero"]},
    ],
}

STATE = {
    "contract": 10,
    "dictionary": "stragabases3-26.09.24.14+d655537b",
    "values": [[0, 1], [1, False], [2, "steady"], [3, None], [6, 52], [7, {"text": "the numbers do not hold"}], [8, {"at_least": 300}]],
}


def test_a_state_that_gives_every_address_a_value_of_its_kind_is_taken():
    assert values_check(DICTIONARY, STATE) == []


def test_every_kind_in_its_shape_is_taken():
    state = copy.deepcopy(STATE)
    state["values"][2] = [2, {"unknown": "not_read"}]
    state["values"][6] = [8, 61.5]
    assert values_check(DICTIONARY, state) == []


def test_an_address_without_a_value_is_refused_by_its_address_and_its_name():
    state = copy.deepcopy(STATE)
    del state["values"][1]
    refused = values_check(DICTIONARY, state)
    assert len(refused) == 1 and "1" in refused[0] and "relays/pump" in refused[0], refused


def test_a_value_the_entry_does_not_allow_is_refused_by_name():
    broken = {
        1: "on",                          # a switch sent as a word
        2: "blinking",                    # a word not of its list
        6: -3,                            # seconds below nought
        7: "the numbers do not hold",     # text without its name
        8: {"unknown": "gone"},           # a reason not of its list
        0: {"number": {"value": 1, "unit": "consoles"}},  # contract 1's shape
    }
    for at, value in broken.items():
        state = copy.deepcopy(STATE)
        state["values"] = [[a, value if a == at else v] for a, v in state["values"]]
        refused = values_check(DICTIONARY, state)
        name = DICTIONARY["things"][at]["name"]
        assert len(refused) == 1 and name in refused[0], (at, value, refused)


def test_an_address_the_dictionary_has_not_and_one_sent_twice_are_refused():
    state = copy.deepcopy(STATE)
    state["values"] += [[42, 1], [1, True]]
    refused = values_check(DICTIONARY, state)
    assert any("42" in r for r in refused) and any("twice" in r and "relays/pump" in r for r in refused), refused


def test_a_state_of_another_dictionary_or_contract_is_refused_before_any_value_is_read():
    for key, value in [("dictionary", "stragabases3-26.09.24.13+00000000"), ("contract", 1), ("contract", 4)]:
        state = copy.deepcopy(STATE)
        state[key] = value
        refused = values_check(DICTIONARY, state)
        assert len(refused) == 1 and key in refused[0], (key, refused)


def test_a_state_of_the_contract_before_is_refused_in_words_naming_both_numbers():
    # a refusal is read by a person at the boiler, so it names the number the check holds and the one it met - not "not 6" with the check at 7
    for met in (6, 5, None):
        state = copy.deepcopy(STATE)
        state["contract"] = met
        refused = values_check(DICTIONARY, state)
        assert len(refused) == 1 and "contract %s" % met in refused[0] and "not 10" in refused[0], (met, refused)
    ok = copy.deepcopy(STATE)
    ok["contract"] = 10
    assert values_check(DICTIONARY, ok) == [], "a state of the contract the check holds was refused"
