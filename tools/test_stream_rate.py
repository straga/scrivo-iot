"""tools/stream_rate.py: it counts the events a reader of a stand's stream gets, the first (the whole board) apart, and the records in them - on the stand of the page, a run played faster than it happened."""
import threading
import time

import pytest

from page_stand import Board as Stand, Handler, Threaded
from stream_rate import measure

DICTIONARY = {
    "contract": 4, "version": "k+1", "board": "B",
    "stream": {"every": 5, "live_s": 600, "most": 60},
    "things": [
        {"card": "c", "name": "a", "kinds": ["number"], "unit": "u"},
        {"card": "c", "name": "b", "kinds": ["number"], "unit": "u"},
    ],
}
SECOND = 0.05


@pytest.fixture
def stand():
    Handler.board = Stand(DICTIONARY, {"values": [[0, 0], [1, 0]]}, None, 4)
    Handler.board.second = SECOND
    Handler.page_dir = "/nonexistent"
    Handler.real = None
    server = Threaded(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    yield "127.0.0.1:%d" % server.server_address[1]
    server.shutdown()


def play(changes, every):
    """A value that changes CHANGES times, EVERY wall seconds apart, on the stand of the test."""
    def run():
        for n in range(1, changes + 1):
            time.sleep(every)
            Handler.board.show([[0, n * 1.0]])
    threading.Thread(target=run, daemon=True).start()


def test_a_reader_that_asks_every_change_counts_every_change_and_the_first_event_is_no_change(stand):
    play(20, 0.03)
    got = measure(stand, "?every=0", 1.0, SECOND)
    assert got["events"] >= 17, got
    assert got["records"] == got["events"], "one value moved at a time: a record in each event"


def test_a_reader_that_asks_a_pace_counts_fewer_events_of_the_same_run_and_the_last_value_still_arrives(stand):
    play(20, 0.03)
    got = measure(stand, "?every=2", 1.0, SECOND)  # a pace of two board seconds: 0.1 s here
    assert 3 <= got["events"] <= 9, got


def test_the_rates_are_counted_in_seconds_of_the_board_and_not_of_the_wall(stand):
    play(10, 0.05)
    got = measure(stand, "?every=0", 0.7, SECOND)
    assert abs(got["events_per_s"] * got["board_seconds"] - got["events"]) < 0.5
    assert got["board_seconds"] > 10, "0.7 wall seconds are fourteen board seconds of 0.05"
