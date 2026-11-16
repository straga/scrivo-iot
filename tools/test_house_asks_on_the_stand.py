"""The house asks the boiler for heat, by the stand: the new burner's own thermostat, demand and automaton (burner_api, example `burner_frames`) play the room against its target, the schedule against the
clock, and a room that goes silent - and the cards say what the house asks and why while the fire follows. Expected from the scenario's own lines and clock - the word a room line makes is worked out here from the stage's
rule (warm at the target or over it, cold at the target less the hysteresis or under it, the word before between the two), and the second a wall time comes from the start and the rate - and from what the stage says each word
does to the fire: warm is the ordinary way down, cold is the water's own band, Stop is stronger than both. Not from the code that makes the frames.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_house_asks_on_the_stand.py
"""
import copy
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from module_compare import frames_of, named  # noqa: E402
from page_stand import dictionary_of  # noqa: E402

SCENARIOS = os.path.join(HERE, "stand_scenarios")
TARGET, GAP = 21.0, 1.0
NONE = {"unknown": "none"}
COLUMNS = ["demand/asked", "demand/why", "demand/silent_room", "demand/profile", "demand/source", "demand/source.window", "demand/source.until", "demand/target",
           "thermostat/heat", "thermostat/switch", "automaton/stage"]


def played(scenario):
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "scenario.json")
        with open(path, "w") as f:
            json.dump(scenario, f)
        frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
    return named(dictionary_of(ROOT, "burner"), frames, COLUMNS)


def scenario(name):
    return json.load(open(os.path.join(SCENARIOS, name)))


def line_at(points, t):
    """The value of a scenario's line at second T: straight between the points, none from a null point to the next."""
    for (t0, v0), (t1, v1) in zip(points, points[1:]):
        if t0 <= t <= t1:
            if v0 is None:
                return None
            return v0 if v1 is None or t1 == t0 else v0 + (v1 - v0) * (t - t0) / (t1 - t0)
    return points[-1][1]


def words_of_the_room(points, seconds):
    """What the room says each second by the stage's rule: warm at the target or over it, cold at the target less the hysteresis or under it, what it said before between the two; before the room has been read once, no heat is asked; a room that spoke and fell silent is cold."""
    cold, out = None, []
    for t in range(seconds):
        r = line_at(points, t)
        if r is None:
            if cold is None:
                out.append(("warm", "not_read"))
                continue
            cold = True
            out.append(("cold", "source_silent"))
            continue
        if cold is None:
            cold = False
        if r >= TARGET:
            cold = False
        elif r <= TARGET - GAP:
            cold = True
        out.append(("cold", "below_target") if cold else ("warm", "target_reached"))
    return out


def stages(run, a, b):
    return {s["automaton/stage"] for s in run[a:b]}


# ---------------------------------------------------------------- the room against its target

def test_the_house_says_what_the_room_says_second_by_second_and_the_card_says_why():
    sc = scenario("house_by_the_room.json")
    run = played(sc)
    want = words_of_the_room(sc["room_c"], sc["seconds"])
    wrong = [t for t, s in enumerate(run) if (s["demand/asked"], s["demand/why"]) != want[t]]
    assert not wrong, "the house does not say the room's word at seconds %s" % wrong[:10]
    assert {(s["demand/profile"], s["demand/source"], s["demand/target"]) for s in run} == {("comfort", "main", TARGET)}


def test_a_warm_room_sends_the_working_fire_down_to_smouldering_whatever_the_water_and_the_fire_burns_out_by_its_own_time():
    sc = scenario("house_by_the_room.json")
    run = played(sc)
    assert run[198]["automaton/stage"] == "working", "the fire did not work before the room was warm"
    assert line_at(sc["boiler_c"], 205) < 60.0 - 4.0, "the water is under the line it lights at: this is the case the word of the house is for"
    smoulder = next(t for t, s in enumerate(run) if s["automaton/stage"] == "smouldering")
    assert 200 <= smoulder <= 202, "smouldering at %d, the room was warm at 200" % smoulder
    assert run[smoulder]["thermostat/heat"] == "enough"
    between = stages(run, smoulder, 396)
    assert "burning_off" in between and "purge" not in between and "kindling" not in between, "the fire was not left to burn out by itself: %s" % sorted(between)
    assert run[399]["automaton/stage"] == "stop"


def test_a_room_that_falls_under_its_target_less_the_hysteresis_lights_the_stopped_fire_by_the_water():
    run = played(scenario("house_by_the_room.json"))
    assert run[399]["thermostat/switch"] == "on" and run[399]["automaton/stage"] == "stop"
    lit = next(t for t, s in enumerate(run) if t >= 400 and s["automaton/stage"] == "purge")
    assert lit - 400 <= 3, "lit %d s after the room fell to 19" % (lit - 400)
    assert run[lit]["thermostat/heat"] == "burn"


def test_nobody_asking_leaves_the_fire_to_the_water_whatever_the_room_does():
    sc = copy.deepcopy(scenario("house_by_the_room.json"))
    del sc["settings"]["auto_asked_by"]
    run = played(sc)
    assert all((s["demand/asked"], s["demand/why"], s["demand/profile"], s["demand/target"]) == (NONE, NONE, NONE, NONE) for s in run), "nobody asks, and the card says none of it"
    assert "smouldering" not in stages(run, 160, 640), "a warm room moved a fire nobody asked it of"
    assert {s["thermostat/heat"] for s in run[170:640]} == {"burn"}, "the water is under the line the whole run"


def test_stop_is_stronger_than_a_cold_house_and_no_word_lights_a_fire_a_person_has_stopped():
    sc = copy.deepcopy(scenario("house_by_the_room.json"))
    sc["press"] = [[2, "start"], [250, "stop"]]
    run = played(sc)
    assert run[255]["thermostat/switch"] == "off"
    assert run[450]["demand/asked"] == "cold" and run[450]["thermostat/heat"] == "burn", "the room is cold and the water under the line from 400 s"
    assert "purge" not in stages(run, 255, 640) and "kindling" not in stages(run, 255, 640), "a cold house lit a fire a person had stopped"


# ---------------------------------------------------------------- the schedule against the clock

START_MIN, RATE = 12 * 60, 3


def minute_of_day(t):
    return (START_MIN + t * RATE) % 1440


def in_window(t):
    """Window 1, 22:00-06:00 every day, at second T of the scenario: worked out from the clock, as the stage says a window through midnight is."""
    m = minute_of_day(t)
    return m >= 22 * 60 or m < 6 * 60


def test_the_schedule_says_cold_exactly_while_the_wall_clock_is_in_a_window_that_burns_and_warm_outside_it():
    run = played(scenario("house_by_the_schedule.json"))
    wrong = [t for t, s in enumerate(run) if (s["demand/asked"], s["demand/why"]) != (("cold", "scheduled_on") if in_window(t) else ("warm", "scheduled_off"))]
    assert not wrong, "the house does not say the clock's word at seconds %s" % wrong[:10]
    opened = next(t for t in range(640) if in_window(t))
    assert run[opened - 1]["demand/asked"] == "warm" and run[opened]["demand/asked"] == "cold"
    closed = next(t for t in range(opened, 640) if not in_window(t))
    assert run[closed - 1]["demand/asked"] == "cold" and run[closed]["demand/asked"] == "warm"


def test_the_card_says_what_chose_the_profile_the_window_and_the_minute_it_ends_at_and_no_target_of_a_room():
    run = played(scenario("house_by_the_schedule.json"))
    for t, s in enumerate(run):
        assert s["demand/target"] == NONE, "the schedule alone holds no room: %d" % t
        if in_window(t):
            assert (s["demand/profile"], s["demand/source"], s["demand/source.window"], s["demand/source.until"]) == ("comfort", "window", 1.0, 360.0), t
        else:
            assert (s["demand/profile"], s["demand/source"]) == ("off", "main"), t
            assert s["demand/source.window"] == NONE and s["demand/source.until"] == NONE, t


def test_a_warm_house_sends_the_fire_a_start_lit_down_the_second_it_works_a_window_that_opens_makes_it_work_again_and_one_that_closes_sends_it_down_to_a_stop():
    run = played(scenario("house_by_the_schedule.json"))
    working = next(t for t, s in enumerate(run) if s["automaton/stage"] == "working")
    assert run[working + 1]["automaton/stage"] == "smouldering", "the house was warm and the working fire stayed working"
    opened = next(t for t in range(640) if in_window(t))
    worked = next(t for t, s in enumerate(run) if t >= opened and s["automaton/stage"] == "working")
    assert worked - opened <= 2, "the smouldering fire worked again %d s after the window opened" % (worked - opened)
    closed = next(t for t in range(opened, 640) if not in_window(t))
    down = next(t for t, s in enumerate(run) if t >= closed and s["automaton/stage"] == "smouldering")
    assert down - closed <= 2, "the fire went down %d s after the window closed" % (down - closed)
    after = stages(run, down, 640)
    assert "burning_off" in after and run[639]["automaton/stage"] == "stop", "the fire was not left to burn out by itself: %s" % sorted(after)
    assert "purge" not in stages(run, working + 1, 640), "the house lit a fire again"


def test_a_board_that_does_not_know_the_time_stands_on_the_main_profile_and_says_so_and_the_window_changes_nothing():
    sc = copy.deepcopy(scenario("house_by_the_schedule.json"))
    del sc["clock"]
    run = played(sc)
    assert {(s["demand/asked"], s["demand/why"], s["demand/profile"], s["demand/source"]) for s in run} == {("warm", "scheduled_off", "off", "no_clock")}
    assert "working" not in stages(run, 165, 640), "a window the board cannot follow made the fire work again"


def test_with_the_main_profile_burning_and_no_window_the_schedule_says_cold_all_day():
    sc = copy.deepcopy(scenario("house_by_the_schedule.json"))
    sc["settings"]["auto_profile"] = "comfort"
    del sc["settings"]["window_1"]
    run = played(sc)
    assert {(s["demand/asked"], s["demand/why"], s["demand/source"]) for s in run} == {("cold", "scheduled_on", "main")}
    assert "smouldering" not in stages(run, 0, 640), "a house that is cold sent a working fire down"


def test_stop_in_an_open_window_wins_and_a_window_that_opens_after_it_lights_no_fire():
    sc = copy.deepcopy(scenario("house_by_the_schedule.json"))
    sc["press"] = [[2, "start"], [120, "stop"]]
    run = played(sc)
    assert run[125]["thermostat/switch"] == "off"
    opened = next(t for t in range(640) if in_window(t))
    assert run[opened + 5]["demand/asked"] == "cold"
    assert "purge" not in stages(run, 125, 640) and "kindling" not in stages(run, 125, 640), "a window lit a fire a person had stopped"


def test_every_n_hours_asks_for_m_minutes_from_the_time_it_starts_at_by_the_clock_of_the_run():
    # the count starts again every day at its own time and goes on through the night: six hours from thirteen are 13:00, 19:00, 01:00 and 07:00, the same times every day
    sc = copy.deepcopy(scenario("house_by_the_schedule.json"))
    sc["settings"]["window_1"] = "set:daily 13:00+6h/30m comfort"
    run = played(sc)
    for t, s in enumerate(run):
        minute = (START_MIN + t * RATE) % 1440
        lifted = (minute - 13 * 60) % 360 < 30
        assert s["demand/profile"] == ("comfort" if lifted else "off"), (t, minute)
        assert s["demand/asked"] == ("cold" if lifted else "warm"), (t, minute)
    opened = [t for t in range(1, len(run)) if run[t]["demand/asked"] == "cold" and run[t - 1]["demand/asked"] == "warm"]
    assert sorted({minute_of_day(t) for t in opened}) == [60, 420, 780, 1140], "the starts of the day: 01:00, 07:00, 13:00, 19:00"


# ---------------------------------------------------------------- a source that goes silent

def test_a_room_that_says_nothing_is_cold_because_its_source_is_silent_and_the_fire_works_again_by_the_water():
    sc = scenario("house_room_silent.json")
    run = played(sc)
    want = words_of_the_room(sc["room_c"], sc["seconds"])
    wrong = [t for t, s in enumerate(run) if (s["demand/asked"], s["demand/why"]) != want[t]]
    assert not wrong, "the house does not say the room's word at seconds %s" % wrong[:10]
    assert run[259]["automaton/stage"] == "smouldering" and run[262]["automaton/stage"] == "working", "the silent room did not make the fire work again"
    assert run[300]["thermostat/heat"] == "burn" and line_at(sc["boiler_c"], 300) < 60.0 - 4.0
    assert run[430]["demand/asked"] == "warm" and run[435]["automaton/stage"] == "smouldering", "the room answered and the house was not warm again"


def test_the_card_counts_the_silence_of_the_room_up_while_it_lasts_and_says_nothing_before_and_after():
    run = played(scenario("house_room_silent.json"))
    assert run[255]["demand/silent_room"] is None, "the room speaks and the card says no silence"
    seconds = [s["demand/silent_room"] for s in run[268:420]]
    assert all(isinstance(v, int) for v in seconds), "the room is silent past three periods of its round and the card says by how long"
    assert seconds == sorted(seconds) and seconds[-1] > seconds[0], "the silence counts up"
    assert run[430]["demand/silent_room"] is None, "the room answers and the card says no silence"


def test_a_silent_room_under_a_profile_that_holds_nothing_asks_for_nothing():
    sc = copy.deepcopy(scenario("house_room_silent.json"))
    sc["settings"]["auto_profile"] = "off"
    run = played(sc)
    assert {(s["demand/asked"], s["demand/why"]) for s in run[10:640]} == {("warm", "profile_off")}, "the person said off, and a silent sensor asks for nothing"
    assert all(s["demand/target"] == NONE for s in run), "a profile that holds nothing has no target"


def test_a_room_that_asks_on_a_board_where_no_socket_plays_the_room_is_never_silent_it_is_not_there():
    sc = copy.deepcopy(scenario("house_room_silent.json"))
    del sc["settings"]["role_ntc_4"]
    run = played(sc)
    assert all(not isinstance(s["demand/silent_room"], int) for s in run), "a role nobody plays is no silence"


def test_a_role_given_to_another_socket_is_the_role_played_and_the_card_counts_its_silence_all_the_same():
    sc = copy.deepcopy(scenario("house_room_silent.json"))
    sc["settings"].pop("role_ntc_4")
    sc["settings"]["role_ntc_6"] = "room"
    run = played(sc)
    assert [s["demand/silent_room"] for s in run[268:420]] and all(isinstance(s["demand/silent_room"], int) for s in run[268:420])
