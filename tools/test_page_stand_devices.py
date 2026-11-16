"""The stand of a device that has an API crate and no hand-written values (the Dual R3, the bench S3) starts from what the crate makes: its dictionary, and the values its modules write
by the same functions the board does (`cargo run -p <device>_api --example stand_values`).

    uv run --no-project --with pytest pytest tools/test_page_stand_devices.py

WHY A TEST. A stand's values stand by address, so values written for another table say other things at the same addresses: the stand then shows the page a board no one has. They are asked for
the dictionary the stand is played with, and what is shown is the device's own.
"""
import os

import pytest

from page_stand import dictionary_of, fresh_state, same_dictionary, state_of

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

DEVICES = ["bench_dualr3", "bench_s3"]


@pytest.mark.parametrize("device", DEVICES)
def test_the_values_of_a_device_are_of_the_dictionary_the_stand_is_played_with(device):
    dictionary = dictionary_of(ROOT, device)
    state = state_of(ROOT, device)
    assert state["contract"] == dictionary["contract"] == 10
    assert same_dictionary(state["dictionary"], dictionary["version"]), (state["dictionary"], dictionary["version"])


@pytest.mark.parametrize("device", DEVICES)
def test_a_device_s_values_survive_the_stand_taking_them(device):
    dictionary = dictionary_of(ROOT, device)
    kept = fresh_state(dictionary, state_of(ROOT, device))
    things = dictionary["things"]
    cards = {things[at]["card"] for at, _ in kept["values"]}
    assert cards - {"system", "settings"}, "nothing of the device's own is left: the stand took the values for another table's"


def test_the_dual_s_relay_and_its_energy_stand_where_the_dictionary_says():
    dictionary = dictionary_of(ROOT, "bench_dualr3")
    things = dictionary["things"]
    by_name = {(t["card"], t["name"], t.get("part_of")): at for at, t in enumerate(things)}
    values = dict((at, v) for at, v in state_of(ROOT, "bench_dualr3")["values"])
    assert values[by_name[("relays", "relay1", None)]] is True
    assert values[by_name[("relays", "relay2", None)]] is False
    assert values[by_name[("meter", "voltage", None)]] == pytest.approx(237.4, abs=0.01)
    assert values[by_name[("energy", "today", None)]] == pytest.approx(0.122, abs=1e-3)
    assert ("energy", "reset", None) in by_name and by_name[("energy", "reset", None)] not in values, "a button has no value"


def test_a_device_without_values_of_its_own_stands_on_the_reference_ones():
    state = state_of(ROOT, "no_such_device")
    assert state["contract"] == 10 and state["values"]
