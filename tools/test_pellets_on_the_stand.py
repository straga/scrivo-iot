"""The pellets burned, on the stand: the new burner's frames (burner_api, example `burner_frames`) carry the three counts of the card `pellets` - kilograms burned today, this month and in all - for a scenario whose auger turns at known
seconds. The scenario is `stand_scenarios/pellets.json`: the clock starts on Monday at 23:00 and runs a minute a second, so midnight stands at second 60; the auger is switched on at 5 and off at 35 (Monday), on at 100 and off at 120 (Tuesday).

Expected from what the auger did and the owner's weighing of 4 g/s (240 g/min), not from the code that counts: 30 s on Monday are 120 g; 20 s on Tuesday are 80 g; a rate of 120 g/min counts half of that; a board that does not know the time counts in all and in no day.

    uv run --no-project --with pytest --with pyyaml --with websocket-client pytest tools/test_pellets_on_the_stand.py
"""
import json
import os
import sys
import tempfile

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

from module_compare import frames_of, named  # noqa: E402
from page_stand import dictionary_of  # noqa: E402

SCENARIO = os.path.join(HERE, "stand_scenarios", "pellets.json")
COLUMNS = ["pellets/today", "pellets/month", "pellets/total", "pellets/yesterday"]


def played(sc):
    with tempfile.TemporaryDirectory() as work:
        path = os.path.join(work, "scenario.json")
        with open(path, "w") as f:
            json.dump(sc, f)
        frames = frames_of(["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "burner_frames", path])
    return named(dictionary_of(ROOT, "burner"), frames, COLUMNS)


def scenario():
    return json.load(open(SCENARIO))


def kg(second):
    """The three counts of a second, in kilograms: today, this month, in all; None where the stand said nothing of one."""
    return [(second.get(c) or {}).get("number") if isinstance(second.get(c), dict) else second.get(c) for c in COLUMNS[:3]]


def yesterday(second):
    """What the card says in the row yesterday: kilograms as a number, or the reason there is none."""
    said = second.get("pellets/yesterday")
    return said.get("unknown") if isinstance(said, dict) and "unknown" in said else said


def grams(second):
    return [None if v is None else round(v * 1000) for v in kg(second)]


@pytest.fixture(scope="module")
def run():
    return played(scenario())


def test_a_monday_burst_of_thirty_seconds_is_a_hundred_and_twenty_grams_in_every_count(run):
    assert grams(run[50]) == [120, 120, 120]


def test_nothing_is_counted_while_the_auger_has_not_turned(run):
    assert grams(run[4]) == [0, 0, 0]


def test_the_count_grows_as_the_auger_turns_and_not_all_at_the_end(run):
    assert grams(run[20])[2] == 60, "15 s of turning at 4 g/s"


def test_at_midnight_today_starts_from_nothing_and_the_month_and_all_keep_what_was_burned(run):
    assert grams(run[70]) == [0, 120, 120]


def test_a_tuesday_burst_of_twenty_seconds_adds_eighty_grams_to_today_and_to_the_rest(run):
    assert grams(run[130]) == [80, 200, 200]


def test_half_the_rate_counts_half_the_pellets():
    sc = scenario()
    sc["settings"] = {"pellet_rate": "120"}
    assert grams(played(sc)[130]) == [40, 100, 100]


def test_a_board_that_does_not_know_the_time_counts_in_all_and_in_no_day():
    sc = scenario()
    del sc["clock"]
    assert grams(played(sc)[130]) == [0, 0, 200]


def test_before_the_first_midnight_yesterday_has_no_number_and_says_why_and_it_is_not_nought(run):
    assert yesterday(run[50]) == "none_yet"
    assert yesterday(run[4]) == "none_yet"


def test_after_midnight_yesterday_is_what_monday_burned_and_stays_that_while_tuesday_burns(run):
    assert round(yesterday(run[70]) * 1000) == 120, "the last second of Monday said 120 g today"
    assert round(yesterday(run[130]) * 1000) == 120, "Tuesday's own 80 g are today's, not yesterday's"
    assert grams(run[130]) == [80, 200, 200]


def test_each_midnight_moves_yesterday_on_and_a_quiet_day_is_nought_not_silence():
    sc = scenario()
    sc["seconds"] = 3000  # midnights stand at seconds 60 (Tuesday), 1500 (Wednesday), 2940 (Thursday)
    done = played(sc)
    assert round(yesterday(done[1530]) * 1000) == 80, "Wednesday: yesterday is Tuesday's 80 g"
    assert round(yesterday(done[2990]) * 1000) == 0, "Thursday: yesterday is Wednesday, on which the auger did not turn"


def test_a_board_that_does_not_know_the_time_has_no_yesterday():
    sc = scenario()
    del sc["clock"]
    assert yesterday(played(sc)[130]) == "none_yet"
