"""A client of the contract of the document without the page: it takes the dictionary, reads the values by card and
code, commands by code through the address the dictionary gives, and refuses an answer of another contract or of
another dictionary before it reads a single value.

    uv run --no-project --with pytest pytest -q tools/test_board_client.py
"""
import threading
import time

import pytest

import page_stand
from board_client import Board, Refused, is_on, number, word
from page_stand import Board as Stand, Handler, Threaded

DICTIONARY = {
    "contract": 10, "version": "k+1", "board": "B",
    "things": [
        {"card": "relays", "name": "pump", "kinds": ["on"], "can": ["switch", "toggle"]},
        {"card": "draught", "name": "blast_pressure", "kinds": ["number", "at_least", "unknown"], "unit": "Pa", "reasons": ["own_zero"]},
        {"card": "automaton", "name": "stage", "kinds": ["word"], "words": ["stop", "purge"]},
        {"card": "draught", "name": "pin", "kinds": ["number"], "unit": "mV"},
        {"card": "draught", "name": "spread", "part_of": "pin", "kinds": ["number"], "unit": "mV"},
    ],
}


@pytest.fixture
def served():
    Handler.board = Stand(DICTIONARY, {"values": [[0, False], [1, 31.5], [2, "stop"], [3, 412], [4, 16]]}, None, 2)
    Handler.page_dir = "/nonexistent"
    Handler.real = None
    server = Threaded(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    yield "127.0.0.1:%d" % server.server_address[1]
    server.shutdown()


def test_a_value_is_read_by_its_card_and_code_through_the_address_the_dictionary_gives(served):
    board = Board(served)
    assert board.address("relays", "pump") == 0
    assert board.address("draught", "pin", "spread") == 4
    now = board.state()
    assert is_on(board.value("relays", "pump", now)) is False
    assert number(board.value("draught", "blast_pressure", now)) == 31.5
    assert word(board.value("automaton", "stage", now)) == "stop"
    assert number(board.value("draught", "pin", now, part="spread")) == 16


def test_a_command_by_code_goes_to_the_address_and_answers_the_values_it_left(served):
    board = Board(served)
    assert board.command("on", "relays", "pump") == {"values": [[0, True]]}
    assert is_on(board.value("relays", "pump")) is True


def test_a_code_the_dictionary_has_not_is_refused_by_its_name(served):
    with pytest.raises(Refused, match="relays/fan"):
        Board(served).address("relays", "fan")


def test_an_answer_of_another_contract_or_another_dictionary_is_refused_before_a_value_is_read(served):
    board = Board(served)
    board.dictionary()
    Handler.board.dictionary = dict(DICTIONARY, version="k+2")
    with pytest.raises(Refused, match="k\\+2"):
        board.state()
    Handler.board.dictionary = dict(DICTIONARY, contract=1)
    with pytest.raises(Refused, match="contract 1"):
        Board(served).state()


def test_a_dictionary_of_the_contract_before_this_one_is_refused_in_words_naming_both(served):
    # the automaton, the clocks and the choices' bearings came with contract 3, and the stream sends less than it did with contract 4; a client reading them from a board that says 2 reads a
    # dictionary without them and takes it for one that has none
    Handler.board.dictionary = dict(DICTIONARY, contract=2)
    with pytest.raises(Refused, match=r"contract 2.*contract 10"):
        Board(served).dictionary()


def test_a_board_of_contract_4_is_refused_naming_both_numbers_because_its_save_answered_in_words(served):
    # the answer of /settings and the row `refused` changed their shape - a client or a page of the other side of that must be told, not shown the raw
    Handler.board.dictionary = dict(DICTIONARY, contract=4)
    with pytest.raises(Refused, match=r"contract 4.*contract 10"):
        Board(served).dictionary()


def test_the_readers_say_nothing_of_a_value_of_another_kind():
    assert number({"at_least": 300}) is None and number(True) is None and number(None) is None
    assert is_on(1) is None and is_on("on") is None
    assert word({"unknown": "own_zero"}) is None and word(3) is None


assert page_stand.EVERY_MS


# ---- what a client asks of the stream and what it counts on by itself ----

from board_client import going_on  # noqa: E402


def test_a_count_the_board_said_once_goes_on_up_or_down_to_the_nearest_second_and_not_below_nought():
    assert going_on("up", 10, 100.0, 104.4) == 14
    assert going_on("up", 10, 100.0, 100.4) == 10, "less than half a second has not gone"
    assert going_on("up", 10, 100.0, 100.6) == 11
    assert going_on("down", 3, 100.0, 102.0) == 1
    assert going_on("down", 3, 100.0, 160.0) == 0


def test_a_stream_asks_for_the_pace_it_wants_and_for_none_when_it_wants_the_pace_of_everyone(served):
    # a stream the client left is found by the stand when it next writes, five seconds on: room for all three meanwhile
    Handler.board.max_streams = 10
    for every, said in [(1, "every=1"), (0, "every=0"), (None, "no pace asked")]:
        before = len(Handler.board.sent)
        Board(served).stream(0.3, lambda *a: None, every=every)
        time.sleep(0.2)
        asked = [one["body"] for one in Handler.board.sent[before:] if one["what"] == "/events"]
        assert any(body.endswith(said) for body in asked), (every, asked)
