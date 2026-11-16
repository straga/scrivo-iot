"""The settings a flash would leave behind are taken off the board before it and written back after it: on two stands - the board of the image before, contract 5, with what its owner set, and the board of
the image after, contract 10, with the defaults - the values travel from one to the other by the page's own way of saving, and nothing else on the new board moves.

    uv run --no-project --with pytest pytest -q tools/test_board_settings.py
"""
import contextlib
import io
import json
import threading

import pytest

from board_client import Refused
from board_settings import DEMAND, LEVELS, LOCK, SCHEDULE, THERMOSTAT, put, take
from page_stand import Board, Handler, Threaded

NAMES = THERMOSTAT + ["auto_purge"]
OWNER = {"auto_hold": "return", "auto_hold_c": "47", "auto_drop_c": "6", "auto_step": "8", "auto_purge": "31"}
DEFAULTS = {"auto_hold": "boiler", "auto_hold_c": "60", "auto_drop_c": "4", "auto_step": "6", "auto_purge": "30"}


def dictionary(contract, names=NAMES):
    return {"contract": contract, "version": "k+%d" % contract, "board": "B",
            "things": [{"card": "settings", "name": n, "kinds": ["text"]} for n in names],
            "settings": [{"name": n, "default": DEFAULTS[n]} for n in names]}


def stand(contract, texts, names=NAMES):
    """A board of CONTRACT keeping TEXTS, served over HTTP as the page reaches it; its address and the server to stop."""
    Handler.board = Board(dictionary(contract, names), {"values": [[at, {"text": texts[n]}] for at, n in enumerate(names)]}, None, 2)
    Handler.page_dir = "/nonexistent"
    Handler.real = None
    server = Threaded(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return "127.0.0.1:%d" % server.server_address[1], server, Handler.board


def kept(board, name):
    at = next(i for i, t in enumerate(board.dictionary["things"]) if t["name"] == name)
    return board.values[at]["text"]


def test_the_four_settings_of_the_thermostat_are_taken_off_a_board_of_the_image_before_and_nothing_else():
    host, server, _ = stand(5, OWNER)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert taken["settings"] == {n: OWNER[n] for n in THERMOSTAT}
    assert (taken["contract"], taken["dictionary"]) == (5, "k+5")


def test_a_board_that_keeps_no_such_setting_is_refused_by_the_name_and_not_read_as_empty():
    names = [n for n in NAMES if n != "auto_step"]
    host, server, _ = stand(5, OWNER, names)
    try:
        with pytest.raises(Refused, match="auto_step"):
            take(host)
    finally:
        server.shutdown()


def test_what_was_taken_is_written_onto_the_board_of_the_image_after_by_the_page_s_way_and_read_back():
    old, old_server, _ = stand(5, OWNER)
    try:
        taken = take(old)
    finally:
        old_server.shutdown()
    new, new_server, board = stand(10, DEFAULTS)
    try:
        done = put(new, taken)
        assert sorted(n for n, _, _ in done) == sorted(THERMOSTAT)
        assert {n: kept(board, n) for n in THERMOSTAT} == {n: OWNER[n] for n in THERMOSTAT}
        assert kept(board, "auto_purge") == DEFAULTS["auto_purge"], "a setting that was not taken moved on the new board"
    finally:
        new_server.shutdown()


def test_a_second_restore_writes_nothing_because_every_value_already_stands():
    new, server, board = stand(10, DEFAULTS)
    try:
        taken = {"settings": {n: OWNER[n] for n in THERMOSTAT}}
        assert len(put(new, taken)) == 4
        assert put(new, taken) == []
    finally:
        server.shutdown()


def test_a_dry_run_says_what_would_be_written_and_writes_nothing():
    new, server, board = stand(10, DEFAULTS)
    try:
        done = put(new, {"settings": {n: OWNER[n] for n in THERMOSTAT}}, dry_run=True)
        assert len(done) == 4
        assert {n: kept(board, n) for n in THERMOSTAT} == {n: DEFAULTS[n] for n in THERMOSTAT}
    finally:
        server.shutdown()


def test_a_board_still_of_the_image_before_is_not_written_to():
    old, server, board = stand(5, OWNER)
    try:
        with pytest.raises(Refused, match="contract 5.*contract 10"):
            put(old, {"settings": {"auto_hold": "boiler"}})
        assert kept(board, "auto_hold") == "return"
    finally:
        server.shutdown()


def test_a_board_of_contract_6_is_taken_from_before_the_flash_and_not_written_to_after_it():
    # the silence of the return and the air moved to the thermostat's card, so the image before this one still says contract 6: its settings are read, and it is not written to
    old, server, board = stand(6, OWNER)
    try:
        assert take(old)["settings"], "a board of contract 6 was not read"
        with pytest.raises(Refused, match="contract 6.*contract 10"):
            put(old, {"settings": {"auto_hold": "boiler"}})
    finally:
        server.shutdown()


def test_a_setting_the_new_board_has_not_is_refused_in_the_board_s_own_words():
    names = [n for n in NAMES if n != "auto_step"]
    new, server, _ = stand(10, DEFAULTS, names)
    try:
        with pytest.raises(Refused, match="auto_step"):
            put(new, {"settings": {"auto_step": "8"}})
    finally:
        server.shutdown()


# ---- the levels of the way up to the setpoint stand on the same card and are left behind by a flash the same way ----

LEVELS_OWNER = {"auto_step_mid": "5", "auto_step_small": "3", "auto_mid_c": "10", "auto_small_c": "4"}
LEVELS_DEFAULTS = {"auto_step_mid": "chosen", "auto_step_small": "chosen", "auto_mid_c": "8", "auto_small_c": "3"}
EVERYTHING = THERMOSTAT + LEVELS + ["auto_purge"]
ALL_OWNER = {**OWNER, **LEVELS_OWNER}
ALL_DEFAULTS = {**DEFAULTS, **LEVELS_DEFAULTS}


def dictionary_of(contract, names, defaults):
    return {"contract": contract, "version": "k+%d" % contract, "board": "B",
            "things": [{"card": "settings", "name": n, "kinds": ["text"]} for n in names],
            "settings": [{"name": n, "default": defaults[n]} for n in names]}


def stand_of(contract, texts, names, defaults):
    Handler.board = Board(dictionary_of(contract, names, defaults), {"values": [[at, {"text": texts[n]}] for at, n in enumerate(names)]}, None, 2)
    Handler.page_dir = "/nonexistent"
    Handler.real = None
    server = Threaded(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return "127.0.0.1:%d" % server.server_address[1], server, Handler.board


def test_a_board_that_has_the_levels_gives_them_up_with_the_four_and_by_default():
    host, server, _ = stand_of(10, ALL_OWNER, EVERYTHING, ALL_DEFAULTS)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert taken["settings"] == {n: ALL_OWNER[n] for n in THERMOSTAT + LEVELS}


def test_a_board_of_the_image_before_the_levels_gives_up_the_four_and_is_not_refused_for_lacking_the_levels():
    names = THERMOSTAT + ["auto_purge"]
    host, server, _ = stand_of(10, ALL_OWNER, names, ALL_DEFAULTS)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert taken["settings"] == {n: ALL_OWNER[n] for n in THERMOSTAT}, "the settings the old image never had are not in the file"


def test_the_levels_are_written_back_with_the_four_onto_the_board_that_has_them():
    host, server, board = stand_of(10, ALL_DEFAULTS, EVERYTHING, ALL_DEFAULTS)
    try:
        done = put(host, {"settings": {n: ALL_OWNER[n] for n in THERMOSTAT + LEVELS}})
        assert sorted(n for n, _, _ in done) == sorted(THERMOSTAT + LEVELS)
        assert {n: kept(board, n) for n in THERMOSTAT + LEVELS} == {n: ALL_OWNER[n] for n in THERMOSTAT + LEVELS}
        assert kept(board, "auto_purge") == DEFAULTS["auto_purge"]
    finally:
        server.shutdown()


def test_a_level_the_file_names_and_the_board_has_not_is_still_refused_by_the_name():
    names = THERMOSTAT + ["auto_purge"]
    host, server, _ = stand_of(10, ALL_DEFAULTS, names, ALL_DEFAULTS)
    try:
        with pytest.raises(Refused, match="auto_mid_c"):
            put(host, {"settings": {"auto_mid_c": "10"}})
    finally:
        server.shutdown()


def test_a_name_asked_for_by_hand_that_the_board_has_not_is_refused_even_when_it_is_a_level():
    names = THERMOSTAT + ["auto_purge"]
    host, server, _ = stand_of(10, ALL_OWNER, names, ALL_DEFAULTS)
    try:
        with pytest.raises(Refused, match="auto_mid_c"):
            take(host, ["auto_hold", "auto_mid_c"])
    finally:
        server.shutdown()


# ---- the profiles and the six windows stand on the same card and are left behind by a flash the same way ----

SCHEDULE_OWNER = {"auto_profile": "eco", "window_1": "set:mon-fri 22:00-06:00 eco", "window_2": "set:daily 07:00+3h/20m comfort",
                  "window_3": "off", "window_4": "off", "window_5": "off", "window_6": "off"}
SCHEDULE_DEFAULTS = {"auto_profile": "comfort", **{"window_%d" % n: "off" for n in range(1, 7)}}
WITH_SCHEDULE = THERMOSTAT + LEVELS + SCHEDULE + ["auto_purge"]
SCHEDULED_OWNER = {**ALL_OWNER, **SCHEDULE_OWNER}
SCHEDULED_DEFAULTS = {**ALL_DEFAULTS, **SCHEDULE_DEFAULTS}


def test_a_board_that_has_the_schedule_gives_up_the_profiles_and_the_windows_with_the_rest():
    host, server, _ = stand_of(10, SCHEDULED_OWNER, WITH_SCHEDULE, SCHEDULED_DEFAULTS)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert taken["settings"] == {n: SCHEDULED_OWNER[n] for n in THERMOSTAT + LEVELS + SCHEDULE}


def test_a_board_of_the_image_before_the_schedule_gives_up_what_it_has_and_is_not_refused_for_lacking_it():
    names = THERMOSTAT + LEVELS + ["auto_purge"]
    host, server, _ = stand_of(10, SCHEDULED_OWNER, names, SCHEDULED_DEFAULTS)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert taken["settings"] == {n: SCHEDULED_OWNER[n] for n in THERMOSTAT + LEVELS}, "the windows the old image never had are not in the file"


def test_the_windows_are_written_back_as_they_are_kept_and_the_ones_not_used_stay_so():
    host, server, board = stand_of(10, SCHEDULED_DEFAULTS, WITH_SCHEDULE, SCHEDULED_DEFAULTS)
    try:
        done = put(host, {"settings": {n: SCHEDULED_OWNER[n] for n in SCHEDULE}})
        assert sorted(n for n, _, _ in done) == sorted(["auto_profile", "window_1", "window_2"]), "what already stood so is not written again"
        assert {n: kept(board, n) for n in SCHEDULE} == {n: SCHEDULED_OWNER[n] for n in SCHEDULE}
        assert kept(board, "auto_purge") == DEFAULTS["auto_purge"]
    finally:
        server.shutdown()


def test_a_window_the_file_names_and_the_board_has_not_is_still_refused_by_the_name():
    names = THERMOSTAT + LEVELS + ["auto_purge"]
    host, server, _ = stand_of(10, SCHEDULED_DEFAULTS, names, SCHEDULED_DEFAULTS)
    try:
        with pytest.raises(Refused, match="window_1"):
            put(host, {"settings": {"window_1": "set:daily 22:00-06:00 eco"}})
    finally:
        server.shutdown()


# ---- the wait before a light and the floor that ends it stand on the same card and are left behind by a flash the same way ----

LOCK_OWNER = {"auto_lock_min": "120", "auto_lock_c": "35"}
LOCK_DEFAULTS = {"auto_lock_min": "0", "auto_lock_c": "30"}
WITH_LOCK = THERMOSTAT + LEVELS + LOCK + SCHEDULE + ["auto_purge"]
LOCKED_OWNER = {**SCHEDULED_OWNER, **LOCK_OWNER}
LOCKED_DEFAULTS = {**SCHEDULED_DEFAULTS, **LOCK_DEFAULTS}


def test_a_board_that_has_the_wait_before_a_light_gives_it_up_with_the_rest():
    host, server, _ = stand_of(10, LOCKED_OWNER, WITH_LOCK, LOCKED_DEFAULTS)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert taken["settings"] == {n: LOCKED_OWNER[n] for n in THERMOSTAT + LEVELS + LOCK + SCHEDULE}


def test_a_board_of_the_image_before_the_wait_gives_up_what_it_has_and_is_not_refused_for_lacking_it():
    names = THERMOSTAT + LEVELS + SCHEDULE + ["auto_purge"]
    host, server, _ = stand_of(10, LOCKED_OWNER, names, LOCKED_DEFAULTS)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert taken["settings"] == {n: LOCKED_OWNER[n] for n in THERMOSTAT + LEVELS + SCHEDULE}, "the settings the old image never had are not in the file"


def test_the_wait_and_the_floor_are_written_back_onto_the_board_that_has_them():
    host, server, board = stand_of(10, LOCKED_DEFAULTS, WITH_LOCK, LOCKED_DEFAULTS)
    try:
        done = put(host, {"settings": LOCK_OWNER})
        assert sorted(n for n, _, _ in done) == sorted(LOCK)
        assert {n: kept(board, n) for n in LOCK} == LOCK_OWNER
    finally:
        server.shutdown()


# ---- who asks and the targets of the room are the demand's, the profile and the windows moved to its card, and the setpoints of the water by profile are no more ----

DEMAND_OWNER = {"auto_asked_by": "room", "auto_room_c": "22", "auto_room_eco_c": "18", "auto_room_gap_c": "2"}
DEMAND_DEFAULTS = {"auto_asked_by": "none", "auto_room_c": "21", "auto_room_eco_c": "17", "auto_room_gap_c": "1"}


def test_a_board_that_has_who_asks_gives_it_up_with_the_rest_and_one_that_has_not_is_not_refused_for_lacking_it():
    names = THERMOSTAT + LEVELS + SCHEDULE + DEMAND + ["auto_purge"]
    owner, defaults = {**SCHEDULED_OWNER, **DEMAND_OWNER}, {**SCHEDULED_DEFAULTS, **DEMAND_DEFAULTS}
    host, server, _ = stand_of(10, owner, names, defaults)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert {n: taken["settings"][n] for n in DEMAND} == DEMAND_OWNER, "who asks is left behind by a flash like the rest of the card"
    before, before_server, _ = stand_of(7, SCHEDULED_OWNER, THERMOSTAT + LEVELS + SCHEDULE + ["auto_purge"], SCHEDULED_DEFAULTS)
    try:
        assert not set(take(before)["settings"]) & set(DEMAND), "the image before has no demand, and is not refused for it"
    finally:
        before_server.shutdown()


def test_the_setpoints_of_the_water_by_profile_have_no_counterpart_and_are_not_carried_to_the_new_board():
    # the image before kept auto_eco_c and auto_off_c as setpoints of the WATER by profile; the profile is a target of the room now, and 50 degrees of water carried over would be a room that is never warm
    names = THERMOSTAT + LEVELS + SCHEDULE + ["auto_eco_c", "auto_off_c", "auto_purge"]
    owner, defaults = {**SCHEDULED_OWNER, "auto_eco_c": "46", "auto_off_c": "35"}, {**SCHEDULED_DEFAULTS, "auto_eco_c": "50", "auto_off_c": "30"}
    host, server, _ = stand_of(7, owner, names, defaults)
    try:
        taken = take(host)
    finally:
        server.shutdown()
    assert "auto_eco_c" not in taken["settings"] and "auto_off_c" not in taken["settings"]
    assert taken["settings"]["auto_profile"] == "eco" and taken["settings"]["window_1"].startswith("set:"), "the profile and the windows are carried: they have the same names on the new card"


# ---- a save taken off a board whose burner held the ROOM is carried to the new keys, and what has no counterpart is said ----
# The image before held the room by auto_hold=room with the target of the room in auto_hold_c. The new image refuses that choice: who asks is the demand's now (auto_asked_by) and the burner holds the boiler's water.
# Every row goes the way a person goes: `restore` of a file onto a board of the contract the client reads, and what the board keeps and what the tool says are read.

TRANSLATED_NAMES = THERMOSTAT + ["auto_asked_by", "auto_room_c", "auto_purge"]
TRANSLATED_DEFAULTS = {**DEFAULTS, "auto_asked_by": "none", "auto_room_c": "21"}
OLD_ROOM = {"auto_hold": "room", "auto_hold_c": "23", "auto_drop_c": "6", "auto_step": "8"}


def restored(file_settings, names=TRANSLATED_NAMES, defaults=TRANSLATED_DEFAULTS, extra=()):
    """`restore` of FILE_SETTINGS onto a new board that keeps DEFAULTS: what the board keeps afterwards, what the tool printed and its exit code."""
    import os
    import tempfile
    from board_settings import main
    host, server, board = stand_of(10, defaults, names, defaults)
    path = os.path.join(tempfile.mkdtemp(), "saved.json")
    with open(path, "w") as f:
        json.dump({"settings": file_settings}, f)
    out = io.StringIO()
    err = io.StringIO()
    try:
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = main(["restore", host, path, *extra])
        return {n: kept(board, n) for n in names}, out.getvalue(), err.getvalue(), code
    finally:
        server.shutdown()


def test_a_save_that_held_the_room_is_written_as_the_boiler_asked_by_the_room_with_the_old_number_as_its_target():
    now, out, err, code = restored(OLD_ROOM)
    assert code == 0, err
    assert (now["auto_hold"], now["auto_asked_by"], now["auto_room_c"]) == ("boiler", "room", "23")
    assert (now["auto_drop_c"], now["auto_step"]) == ("6", "8"), "what the room mode shared with the others is carried as it was"
    assert now["auto_hold_c"] == DEFAULTS["auto_hold_c"], "the old number was the target of the ROOM; written as the water's it would be 23 degrees of water"
    assert now["auto_purge"] == DEFAULTS["auto_purge"]
    assert "auto_asked_by: none -> room" in out and "auto_room_c: 21 -> 23" in out, out
    assert "not written" not in out, "the target was in the limits: nothing is left unwritten"


def test_a_save_that_held_the_boiler_is_written_as_it_is_and_brings_no_new_keys():
    now, out, err, code = restored({"auto_hold": "boiler", "auto_hold_c": "55", "auto_drop_c": "6", "auto_step": "8"})
    assert code == 0, err
    assert (now["auto_hold"], now["auto_hold_c"]) == ("boiler", "55")
    assert (now["auto_asked_by"], now["auto_room_c"]) == (TRANSLATED_DEFAULTS["auto_asked_by"], TRANSLATED_DEFAULTS["auto_room_c"])
    assert "auto_asked_by" not in out and "auto_room_c" not in out and "not written" not in out, out


def test_a_target_of_the_room_out_of_the_limits_is_not_written_and_the_line_says_so():
    now, out, err, code = restored({**OLD_ROOM, "auto_hold_c": "60"})
    assert code == 0, err
    assert (now["auto_hold"], now["auto_asked_by"]) == ("boiler", "room"), "the room still asks"
    assert now["auto_room_c"] == TRANSLATED_DEFAULTS["auto_room_c"], "60 degrees of room is not carried; the default stays"
    assert any("auto_room_c" in line and "60" in line and "not written" in line for line in out.splitlines()), out


@pytest.mark.parametrize("old, carried", [("6", True), ("35", True), ("5", False), ("36", False), ("warm", False), ("", False)])
def test_the_limits_of_the_room_target_are_six_to_thirty_five_both_ends_in(old, carried):
    now, out, err, code = restored({**OLD_ROOM, "auto_hold_c": old})
    assert code == 0, err
    assert (now["auto_room_c"] == old) is carried, (old, now["auto_room_c"])
    assert ("not written" in out) is (not carried), out


def test_the_setpoints_of_the_water_by_profile_in_a_file_are_named_skipped_and_the_rest_is_written():
    names = TRANSLATED_NAMES + ["auto_eco_c", "auto_off_c"]
    defaults = {**TRANSLATED_DEFAULTS, "auto_eco_c": "50", "auto_off_c": "30"}
    now, out, err, code = restored({**OLD_ROOM, "auto_eco_c": "46", "auto_off_c": "35"}, names, defaults)
    assert code == 0, err
    assert (now["auto_eco_c"], now["auto_off_c"]) == ("50", "30"), "degrees of water are not the room's"
    assert (now["auto_asked_by"], now["auto_room_c"]) == ("room", "23")
    skipped = [line for line in out.splitlines() if "not written" in line]
    assert any("auto_eco_c" in line for line in skipped) and any("auto_off_c" in line for line in skipped), out


def test_a_board_without_the_demand_keys_still_refuses_a_save_that_held_the_room_by_the_name():
    now, out, err, code = restored(OLD_ROOM, THERMOSTAT + ["auto_purge"], DEFAULTS)
    assert code == 1 and "auto_asked_by" in err, (code, err)
    assert now["auto_hold"] == DEFAULTS["auto_hold"], "nothing was written onto a board that cannot take the whole of it"


def test_a_dry_run_of_a_save_that_held_the_room_says_the_new_keys_and_writes_nothing():
    now, out, err, code = restored({**OLD_ROOM, "auto_hold_c": "60"}, extra=["--dry-run"])
    assert code == 0, err
    assert "auto_asked_by: none -> room" in out and "auto_room_c" in out and "60" in out, out
    assert now == TRANSLATED_DEFAULTS
