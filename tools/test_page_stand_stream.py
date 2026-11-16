"""The stand's stream, as the board's: a reader that opens gets every value, a
quiet board sends nothing, one value changed is one record, a second reader mid-work gets everything, a value sent
when a reader opens is not sent again.

    uv run --no-project --with pytest pytest -q tools/test_page_stand_stream.py
"""
from page_stand import Board

DICTIONARY = {
    "contract": 5, "version": "k+1", "board": "B",
    "things": [
        {"card": "c", "name": "sw", "kinds": ["on"], "can": ["switch", "toggle"]},
        {"card": "c", "name": "t", "kinds": ["number"], "unit": "°C"},
        {"card": "system", "name": "uptime", "kinds": ["number"], "unit": "ms", "once": True},
        {"card": "c", "name": "k", "kinds": ["number"], "unit": "s", "fixed": 20},
        {"card": "c", "name": "go", "kinds": ["button"], "can": ["press"]},
    ],
}


def board():
    return Board(DICTIONARY, {"values": [[0, False], [1, 21.5], [2, 1000]]}, None, 2)


def test_a_reader_that_opens_gets_every_value_the_board_sends_and_nothing_fixed_or_pressed():
    assert board().changed_for({}) == [[0, False], [1, 21.5], [2, 1000]]


def test_a_quiet_board_sends_nothing():
    b, sent = board(), {}
    b.changed_for(sent)
    assert b.changed_for(sent) == []


def test_one_value_changed_is_one_record():
    b, sent = board(), {}
    b.changed_for(sent)
    b.show([[1, 22.0]])
    assert b.changed_for(sent) == [[1, 22.0]]


def test_a_second_reader_mid_work_gets_every_value_and_the_first_only_what_changed():
    b, first = board(), {}
    b.changed_for(first)
    b.show([[0, True]])
    assert b.changed_for({}) == [[0, True], [1, 21.5], [2, 1000]]
    assert b.changed_for(first) == [[0, True]]


def test_a_value_sent_when_a_reader_opens_is_not_sent_again_when_it_moves():
    b, sent = board(), {}
    b.changed_for(sent)
    b.show([[2, 6000]])
    assert b.changed_for(sent) == []


# ---- the stream sends what a reader needs: the digits a number is worth, the counts that go on by themselves, the pace a reader asks ----

from page_stand import every_asked

NEEDS = {
    "contract": 5, "version": "k+2", "board": "B",
    "stream": {"every": 5, "live_s": 600, "most": 60},
    "things": [
        {"card": "c", "name": "angle", "kinds": ["number"], "unit": "%", "decimals": 0},
        {"card": "c", "name": "water", "kinds": ["number"], "unit": "°C", "decimals": 1},
        {"card": "c", "name": "raw", "kinds": ["number"], "unit": "mV"},
        {"card": "a", "name": "for", "kinds": ["seconds"], "counts": "up"},
        {"card": "a", "name": "left", "kinds": ["seconds"], "counts": "down", "clock": True},
        {"card": "a", "name": "silent", "kinds": ["seconds"]},
    ],
}


def needs(values):
    return Board(NEEDS, {"values": values}, None, 2)


def test_a_number_goes_out_with_the_digits_the_dictionary_says():
    b = needs([[0, 17.149593], [1, 98.26], [2, 460.123]])
    got = dict(b.changed_for({}, 0))
    assert (got[0], got[1], got[2]) == (17.0, 98.3, 460.123), "a number with no digits said goes as it is"


def test_a_change_under_the_digits_a_number_is_worth_sends_nothing():
    b, sent = needs([[0, 17.149593], [1, 98.26]]), {}
    b.changed_for(sent, 0)
    b.show([[0, 17.2], [1, 98.24]])
    assert dict(b.changed_for(sent, 1)) == {1: 98.2}, "17.2 is still 17, and 98.24 is 98.2 against 98.3"
    b.show([[0, 18.4]])
    assert dict(b.changed_for(sent, 2)) == {0: 18.0}


def test_a_number_that_rounds_to_nothing_is_zero_and_not_minus_zero():
    b = needs([[1, -0.04]])
    assert dict(b.changed_for({}, 0))[1] == 0.0 and str(dict(b.changed_for({}, 0))[1]) == "0.0"


def test_a_count_that_goes_on_as_the_reader_counts_it_is_not_sent_again_and_one_that_jumps_is():
    b, sent = needs([[3, 10], [4, 40]]), {}
    first = dict(b.changed_for(sent, 0))
    assert first[3] == 10 and first[4] == 40, "a reader that opens gets the counts"
    for second in range(1, 31):
        b.show([[3, 10 + second], [4, 40 - second]])
        assert 3 not in dict(b.changed_for(sent, second)) and 4 not in dict(b.changed_for(sent, second)), "second %d" % second
    b.show([[3, 0], [4, 25]])
    assert dict(b.changed_for(sent, 31)) == {3: 0, 4: 25}, "a stage that begins, a new target: said"


def test_a_count_down_that_reached_nothing_stays_unsaid_and_a_count_that_loses_its_number_is_said():
    b, sent = needs([[4, 3]]), {}
    b.changed_for(sent, 0)
    for second in range(1, 20):
        b.show([[4, max(0, 3 - second)]])
        assert 4 not in dict(b.changed_for(sent, second)), "second %d" % second
    b.show([[4, None]])
    assert dict(b.changed_for(sent, 20)) == {4: None}


def test_seconds_that_are_no_count_go_out_whenever_they_change():
    b, sent = needs([[5, 0]]), {}
    b.changed_for(sent, 0)
    for second in range(1, 6):
        b.show([[5, second]])
        assert dict(b.changed_for(sent, second)) == {5: second}


def test_a_reader_asks_its_pace_in_whole_seconds_and_zero_is_live():
    assert (every_asked("/events?every=1"), every_asked("/events?every=30"), every_asked("/events?every=0"), every_asked("/events?x=1&every=10&y=2")) == (1, 30, 0, 10)


def test_a_reader_that_asks_nothing_or_nonsense_gets_the_pace_of_everyone():
    for uri in ["/events", "/events?", "/events?every=", "/events?every=abc", "/events?every=-3", "/events?every=2.5", "/events?every=61", "/events?every=99999999999"]:
        assert every_asked(uri) == 5, uri
    assert every_asked("/events?every=60") == 60
