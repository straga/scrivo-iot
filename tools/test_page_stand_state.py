"""The stand keeps the board's settings and none of its own: every setting in tools/page_stand_values.json is a
setting the burner's dictionary describes.

    uv run --no-project --with pytest pytest tools

WHY THIS IS A TEST AND NOT A COMMENT. The stand answers with a state written once, and a state written once drifts
away from the board in silence: a setting the stand INVENTS is a page judged against something no board has.
"""
import json
import os

from page_stand import dictionary_of, same_dictionary

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def settings_in(dictionary, state):
    things = dictionary["things"]
    return [things[at]["name"] for at, _ in state["values"] if things[at]["card"] == "settings" and not things[at].get("part_of")]


def test_every_setting_of_the_stand_is_one_the_board_describes():
    dictionary = dictionary_of(ROOT)
    described = {s["name"] for s in dictionary["settings"]}
    state = json.load(open(os.path.join(HERE, "page_stand_values.json")))
    kept = settings_in(dictionary, state)
    assert kept and set(kept) <= described, set(kept) - described


def test_the_stand_is_of_the_dictionary_it_is_played_with():
    dictionary = dictionary_of(ROOT)
    state = json.load(open(os.path.join(HERE, "page_stand_values.json")))
    assert state["contract"] == dictionary["contract"] == 10
    assert same_dictionary(state["dictionary"], dictionary["version"]), (state["dictionary"], dictionary["version"])


# WHAT "THE SAME DICTIONARY" IS: the stand's values stand by ADDRESS, so they are of a dictionary as long as its content is the same - the part of the version after the `+`, the hash of the
# content. The kit before the `+` is the image's own label and moves with every version of the burner whatever the dictionary says, so it is no reason to call the values stale.
def test_two_versions_of_one_content_are_the_same_dictionary_whatever_the_kit():
    assert same_dictionary("stragabases3-26.09.27.5+cbf3800a", "stragabases3-26.10.01.6+cbf3800a")


def test_a_version_of_another_content_is_another_dictionary_though_the_kit_is_the_same():
    assert not same_dictionary("stragabases3-26.10.01.6+b7d6b391", "stragabases3-26.10.01.6+cbf3800a")
    assert not same_dictionary("stragabases3-26.10.01.6", "stragabases3-26.10.01.6+cbf3800a"), "a version that says no content is no proof of one"


# ---- the stand of a device that has its own API crate is of that device, and the reference burner is still there for the scenarios ----

def test_the_stand_of_the_burner_is_of_the_burners_own_api_and_the_reference_is_the_first_burner():
    own = dictionary_of(ROOT, "burner")
    # every module of the burner is laid through: the relays, the water's temperatures, the flue, the fan, the draught, the flame, the thermostat and the automaton, the pellets burned and the planned restart; and the dictionary
    # says nothing of how the automaton is drawn - no `machine`
    assert "machine" not in own and {t["card"] for t in own["things"]} == {"system", "settings", "thermostat", "demand", "automaton", "relays", "temperatures", "flue", "fan", "draught", "flame", "pellets", "restart", "mains"}, "the stand shows a burner with other modules"
    reference = dictionary_of(ROOT)
    assert "machine" in reference, "the reference burner of the scenarios lost its automaton"
    # the first burner keeps its own card of fixed numbers, which the new one does not have: the two are not mistaken for each other
    assert "automaton_fixed" in {t["card"] for t in reference["things"]} and "automaton_fixed" not in {t["card"] for t in own["things"]}


def test_the_stand_values_of_the_burner_are_addresses_of_its_own_dictionary():
    from page_stand import state_of
    own = dictionary_of(ROOT, "burner")
    state = state_of(ROOT, "burner")
    assert state["contract"] == own["contract"] == 10
    # the CONTENT of the dictionary, as the two rows above hold it: the kit before the `+` is the image's own label and moves with every version
    assert same_dictionary(state["dictionary"], own["version"]), (state["dictionary"], own["version"], "- `uv run --no-project python tools/stand_state.py burner` writes the values again")
    assert state["values"] and all(0 <= at < len(own["things"]) for at, _ in state["values"])
    # and a device with no values of its own gets the reference state, as before
    assert state_of(ROOT, "no_such_device") == json.load(open(os.path.join(HERE, "page_stand_values.json")))


# ---- a snapshot of another dictionary says nothing by address ----
# The state of the burner was recorded from the board when its dictionary was another one: rows added since stand at other addresses, and the stand showed the smouldering as 0:02 and a feed of 51 s because
# the number of one setting was read at the address of another.

def stale_state(dictionary):
    things = dictionary["things"]
    at = lambda card, name: next(i for i, t in enumerate(things) if t["card"] == card and t["name"] == name and not t.get("part_of"))  # noqa: E731
    return {"contract": 6, "dictionary": "kit+0000stale", "values": [[at("system", "wifi"), "joined"], [at("settings", "auto_smoulder_s"), {"text": "2"}], [at("automaton", "auger"), True]]}, at


def test_a_snapshot_of_another_dictionary_gives_the_stand_the_defaults_of_the_settings_and_none_of_its_other_values():
    from page_stand import fresh_state
    dictionary = dictionary_of(ROOT, "burner")
    state, at = stale_state(dictionary)
    got = dict((a, v) for a, v in fresh_state(dictionary, state)["values"])
    smoulder = next(s for s in dictionary["settings"] if s["name"] == "auto_smoulder_s")
    assert got[at("settings", "auto_smoulder_s")] == {"text": smoulder["default"]}, "the setting was read from a stale address"
    assert got[at("system", "wifi")] == "joined", "the system card stands first at the same addresses in every dictionary and is kept"
    assert at("automaton", "auger") not in got, "a value of another dictionary stood at an address of this one"


def test_every_setting_of_the_dictionary_is_in_the_state_the_stand_starts_from():
    from page_stand import fresh_state
    dictionary = dictionary_of(ROOT, "burner")
    state, _ = stale_state(dictionary)
    named = {dictionary["things"][a]["name"] for a, _ in fresh_state(dictionary, state)["values"] if dictionary["things"][a]["card"] == "settings" and not dictionary["things"][a].get("part_of")}
    assert {s["name"] for s in dictionary["settings"]} <= named


def test_a_snapshot_of_this_dictionary_is_the_state_as_it_is():
    from page_stand import fresh_state
    dictionary = dictionary_of(ROOT, "burner")
    state, at = stale_state(dictionary)
    state["dictionary"] = "kit+" + dictionary["version"].partition("+")[2]
    assert fresh_state(dictionary, state) == state


def test_the_values_of_a_stand_are_written_again_for_the_dictionary_the_image_serves_now():
    from stand_state import rewritten
    dictionary = dictionary_of(ROOT, "burner")
    state, at = stale_state(dictionary)
    got = rewritten(dictionary, state)
    assert same_dictionary(got["dictionary"], dictionary["version"]) and got["contract"] == dictionary["contract"]
    values = dict((a, v) for a, v in got["values"])
    smoulder = next(s for s in dictionary["settings"] if s["name"] == "auto_smoulder_s")
    assert values[at("settings", "auto_smoulder_s")] == {"text": smoulder["default"]}
    assert values[at("system", "wifi")] == "joined"
    assert rewritten(dictionary, got) == got, "writing the values again changed values that were already of this dictionary"
