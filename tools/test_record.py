"""The burner's run written down by the second: the dictionary once, the values from one stream,
a row a second in a CSV of the day - on the stand of the page, and against a board that does not answer.

    uv run --no-project --with pytest pytest -q tools/test_record.py
"""
import csv
import os
import socket
import sys
import threading
import time

import pytest

from page_stand import Board as Stand, Handler, Threaded
import record
from record import Recorder, cell, columns

DICTIONARY = {
    "contract": 10, "version": "k+1", "board": "B",
    "things": [
        {"card": "automaton", "name": "stage", "kinds": ["word"], "words": ["stop", "lighting"]},
        {"card": "relays", "name": "auger", "kinds": ["on"], "can": ["switch", "toggle"]},
        {"card": "flame", "name": "pin", "kinds": ["number"], "unit": "mV"},
        {"card": "draught", "name": "blast_pressure", "kinds": ["number", "at_least", "unknown"], "unit": "Pa", "reasons": ["own_zero"]},
        {"card": "draught", "name": "spread", "part_of": "blast_pressure", "kinds": ["number"], "unit": "Pa"},
        {"card": "fan", "name": "angle", "kinds": ["number"], "unit": "%"},
    ],
}
STATE = {"values": [[0, "lighting"], [1, True], [2, 412], [3, {"unknown": "own_zero"}], [4, 3], [5, 0]]}


@pytest.fixture
def stand():
    Handler.board = Stand(DICTIONARY, STATE, None, 2)
    Handler.page_dir = "/nonexistent"
    Handler.real = None
    server = Threaded(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    yield "127.0.0.1:%d" % server.server_address[1]
    server.shutdown()


def rows(out):
    files = sorted(out.glob("*.csv"))
    assert files, "no CSV was written in %s" % out
    with open(files[-1], newline="") as f:
        return list(csv.reader(f))


def test_the_columns_are_the_things_of_the_dictionary_by_card_and_name():
    assert columns(DICTIONARY) == [
        "automaton/stage", "relays/auger", "flame/pin", "draught/blast_pressure", "draught/blast_pressure.spread",
        "fan/angle",
    ]


def test_a_value_without_a_number_is_an_empty_field_and_a_zero_is_a_zero():
    assert cell({"unknown": "own_zero"}) == ""
    assert cell(None) == ""
    # the pair: a real zero and a switch that is off are values, not gaps
    assert cell(0) == "0"
    assert cell(False) == "off"
    assert cell(True) == "on"
    assert cell({"at_least": 300}) == ">=300"
    assert cell({"text": "a, b"}) == "a, b"
    assert cell(["auger", "pump"]) == "auger pump"


def test_a_second_of_the_stand_is_a_row_of_its_values(stand, tmp_path):
    recorder = Recorder(stand, tasmota=None, out=tmp_path)
    recorder.start()
    try:
        until = time.time() + 5
        while time.time() < until and not recorder.hears():
            time.sleep(0.05)
        recorder.tick(time.time())
    finally:
        recorder.stop()
    head, last = rows(tmp_path)[0], rows(tmp_path)[-1]
    got = dict(zip(head, last))
    assert got["board"] == "ok", got
    assert got["automaton/stage"] == "lighting" and got["relays/auger"] == "on" and got["flame/pin"] == "412", got
    assert got["draught/blast_pressure"] == "" and got["fan/angle"] == "0", got


def test_a_board_that_does_not_answer_is_a_row_that_says_so_and_the_writing_goes_on(tmp_path):
    silent = socket.socket()
    silent.bind(("127.0.0.1", 0))
    port = silent.getsockname()[1]
    silent.close()  # nothing listens there now
    recorder = Recorder("127.0.0.1:%d" % port, tasmota=None, out=tmp_path)
    recorder.start()
    try:
        time.sleep(0.5)
        recorder.tick(time.time())
        recorder.tick(time.time() + 1)
    finally:
        recorder.stop()
    written = rows(tmp_path)
    assert len(written) == 3, written  # the head and a row each second
    assert all(r[written[0].index("board")].startswith("no answer") for r in written[1:]), written


def test_a_board_of_the_contract_before_is_a_row_that_says_which_contract_it_speaks(stand, tmp_path):
    # a recording read by a dictionary of another contract is a column about another thing - the row says so and writes no value
    Handler.board.dictionary = dict(DICTIONARY, contract=2)
    recorder = Recorder(stand, tasmota=None, out=tmp_path)
    recorder.start()
    try:
        time.sleep(0.5)
        recorder.tick(time.time())
    finally:
        recorder.stop()
    written = rows(tmp_path)
    got = dict(zip(written[0], written[-1]))
    assert got["board"].startswith("no answer") and "contract 2" in got["board"], got
    assert all(v == "" for k, v in got.items() if "/" in k), got


def test_a_count_the_board_said_once_is_written_as_what_it_is_now_every_second_and_a_number_that_is_no_count_as_it_was(tmp_path):
    counting = {"contract": 10, "version": "k+1", "board": "B", "things": [
        {"card": "automaton", "name": "for", "kinds": ["seconds"], "counts": "up"},
        {"card": "automaton", "name": "left", "kinds": ["seconds"], "counts": "down", "clock": True},
        {"card": "flame", "name": "pin", "kinds": ["number"], "unit": "mV"},
    ]}
    recorder = Recorder("unused", tasmota=None, out=tmp_path)
    recorder.dictionary = counting
    recorder.heard_at = 1000.0
    recorder.values = {0: 5, 1: 20, 2: 412}
    recorder.told = {0: 1000.0, 1: 1000.0, 2: 1000.0}
    recorder.tick(1003.0)
    recorder.heard_at = 1029.0  # the stream's beats keep a quiet board heard
    recorder.tick(1030.0)
    written = rows(tmp_path)
    head = written[0]
    got = [dict(zip(head, r)) for r in written[1:]]
    assert (got[0]["automaton/for"], got[0]["automaton/left"], got[0]["flame/pin"]) == ("8", "17", "412")
    assert (got[1]["automaton/for"], got[1]["automaton/left"], got[1]["flame/pin"]) == ("35", "0", "412")


def test_the_csv_of_the_day_goes_to_runs_in_the_current_folder_unless_told_otherwise():
    from record import parser
    assert parser().parse_args(["192.0.2.5"]).out == "runs"
    assert parser().parse_args(["192.0.2.5", "--out", "/data/runs"]).out == "/data/runs"


# ---- the day turns: the file of the days gone is packed ----

def _at(day, clock):
    import datetime
    return datetime.datetime.strptime("%s %s" % (day, clock), "%Y-%m-%d %H:%M:%S").timestamp()


def _packed(recorder):
    t = getattr(recorder, "packing", None)
    if t is not None:
        t.join(10)


def test_when_the_day_turns_the_file_of_the_day_gone_becomes_a_gz_that_reads_back_the_same(tmp_path, monkeypatch):
    monkeypatch.setattr(record, "QUIET_S", 0, raising=False)
    import gzip
    recorder = Recorder("localhost", tasmota=None, out=tmp_path)
    yesterday = recorder.tick(_at("2026-11-02", "23:59:58"))
    recorder.tick(_at("2026-11-02", "23:59:59"))
    written = yesterday.read_bytes()
    today = recorder.tick(_at("2026-11-03", "00:00:00"))
    _packed(recorder)
    assert not yesterday.exists(), "the file of the day gone is still there unpacked"
    with gzip.open(str(yesterday) + ".gz", "rb") as f:
        assert f.read() == written
    assert today.exists() and today.suffix == ".csv"
    assert len(rows(tmp_path)) == 2  # head and the row of the new day, in the plain file


def test_a_day_that_has_not_turned_leaves_its_file_plain(tmp_path, monkeypatch):
    monkeypatch.setattr(record, "QUIET_S", 0, raising=False)
    recorder = Recorder("localhost", tasmota=None, out=tmp_path)
    path = recorder.tick(_at("2026-11-02", "10:00:00"))
    recorder.tick(_at("2026-11-02", "23:59:59"))
    _packed(recorder)
    assert path.exists() and not list(tmp_path.glob("*.gz"))
    assert len(rows(tmp_path)) == 3


def test_a_file_of_a_day_gone_already_packed_is_left_as_it_is(tmp_path, monkeypatch):
    monkeypatch.setattr(record, "QUIET_S", 0, raising=False)
    import gzip
    old = tmp_path / "2026-11-01_localhost_none.csv.gz"
    with gzip.open(old, "wb") as f:
        f.write(b"head\nrow\n")
    before = old.read_bytes()
    recorder = Recorder("localhost", tasmota=None, out=tmp_path)
    recorder.tick(_at("2026-11-02", "00:00:01"))
    recorder.tick(_at("2026-11-03", "00:00:01"))
    _packed(recorder)
    assert old.read_bytes() == before


# ---- an answer the recorder cannot read is a break like any other: it asks again, and the recording rises by itself ----
# The board comes up after a flash and for a moment answers /dictionary with a compressed body, or the socket answers a shape nobody planned. A thread that dies of it leaves a recording that writes `no answer` until someone starts it.

import gzip
import http.server
import json

from record import socket_power


class Flaky(Handler):
    """The stand, with ANSWERS for the paths in the order they are asked: the first ask of a path takes the first of its answers, and what is not listed is the stand's own."""
    answers = {}

    def do_GET(self):
        path = self.path.split("?")[0]
        queue = Flaky.answers.get(path)
        if queue:
            body, kind = queue.pop(0)
            self.send_response(200)
            self.send_header("Content-Type", kind)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        super().do_GET()


@pytest.fixture
def flaky():
    Handler.board = Stand(DICTIONARY, STATE, None, 2)
    Handler.page_dir = "/nonexistent"
    Handler.real = None
    Flaky.answers = {}
    server = Threaded(("127.0.0.1", 0), Flaky)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    yield "127.0.0.1:%d" % server.server_address[1]
    server.shutdown()


GOOD_DICTIONARY = json.dumps(DICTIONARY).encode()
UNREADABLE = {
    "a compressed body": ("/dictionary", gzip.compress(GOOD_DICTIONARY), "application/json"),
    "a dictionary that is no object": ("/dictionary", b"[]", "application/json"),
    "a dictionary with no things": ("/dictionary", json.dumps({"contract": 10, "version": "k+9", "board": "B"}).encode(), "application/json"),
    "a dictionary whose things are not rows": ("/dictionary", json.dumps({"contract": 10, "version": "k+9", "board": "B", "things": [1, 2]}).encode(), "application/json"),
    "a stream event that is no JSON": ("/events", b"data: \xff\xfe\n\n", "text/event-stream"),
    "a stream event whose records are no pairs": ("/events", b"data: 5\n\n", "text/event-stream"),
    "a stream event whose records are pairs of nothing": ("/events", b'data: [[1]]\n\n', "text/event-stream"),
}


@pytest.mark.parametrize("what", sorted(UNREADABLE))
def test_an_answer_that_cannot_be_read_is_a_break_and_the_recording_rises_by_itself(flaky, tmp_path, what):
    path, body, kind = UNREADABLE[what]
    Flaky.answers[path] = [(body, kind)]
    recorder = Recorder(flaky, tasmota=None, out=tmp_path)
    recorder.start()
    try:
        until = time.time() + 12
        while time.time() < until and not recorder.hears():
            time.sleep(0.1)
        assert recorder.hears(), "the board answered well after the unreadable answer and the recorder did not hear it: %s" % recorder.why
        assert recorder.threads[0].is_alive()
        recorder.tick(time.time())
    finally:
        recorder.stop()
    got = dict(zip(rows(tmp_path)[0], rows(tmp_path)[-1]))
    assert got["board"] == "ok" and got["automaton/stage"] == "lighting", got


def test_a_row_is_written_whatever_the_board_has_told_so_far(flaky, tmp_path):
    # the row of a board that has said nothing readable is a row that says so: the second after the bad answer is not a gap and not a crash of the loop that writes
    Flaky.answers["/dictionary"] = [(b"[]", "application/json")] * 3
    recorder = Recorder(flaky, tasmota=None, out=tmp_path)
    recorder.start()
    try:
        time.sleep(0.5)
        recorder.tick(time.time())
    finally:
        recorder.stop()
    got = dict(zip(rows(tmp_path)[0], rows(tmp_path)[-1]))
    assert got["board"].startswith("no answer"), got


class Socket(http.server.BaseHTTPRequestHandler):
    body = b""
    cut = False
    status = 200

    def do_GET(self):
        self.send_response(Socket.status)
        self.send_header("Content-Length", str(len(Socket.body) + (10 if Socket.cut else 0)))
        self.end_headers()
        self.wfile.write(Socket.body)

    def log_message(self, *_):
        pass


@pytest.fixture
def mains():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Socket)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    yield "127.0.0.1:%d" % server.server_address[1]
    server.shutdown()


SOCKET_BAD = {
    "a compressed body": (gzip.compress(b"{}"), False),
    "a list": (b"[]", False),
    "a null": (b"null", False),
    "status with no energy": (b'{"StatusSNS": {}}', False),
    "status that is a list": (b'{"StatusSNS": []}', False),
    "energy that is a list": (b'{"StatusSNS": {"ENERGY": []}}', False),
    "energy that is a number": (b'{"StatusSNS": {"ENERGY": 5}}', False),
    "a body that stops short": (b'{"StatusSNS": {"ENERGY": {"Power": 4', True),
}


@pytest.mark.parametrize("what", sorted(SOCKET_BAD))
def test_a_socket_that_answers_what_cannot_be_read_gives_no_power_and_not_a_crash(mains, what):
    Socket.body, Socket.cut = SOCKET_BAD[what]
    assert socket_power(mains) == (None, None)


def test_a_socket_that_answers_well_after_the_bad_answer_gives_its_power(mains):
    Socket.body, Socket.cut = SOCKET_BAD["energy that is a list"]
    assert socket_power(mains) == (None, None)
    Socket.body, Socket.cut = b'{"StatusSNS": {"ENERGY": {"Power": 4, "Voltage": 227}}}', False
    assert socket_power(mains) == (4, 227)


# ---- a beat of the stream that says nothing readable is no word of the board: its pairs are addresses of the dictionary and values of the forms the board speaks ----

def _beat(text):
    return (("data: %s\n\n" % text).encode(), "text/event-stream")


BAD_BEATS = {
    "an address that is a word": '[["x", 1]]',
    "an address below the first": "[[-1, 1]]",
    "an address one past the last": "[[6, 1]]",
    "an address far past the dictionary": "[[99999, 1]]",
    "an address that is a switch": "[[true, 1]]",
    "an address that is a fraction": "[[1.5, 1]]",
    "an address that is nothing": "[[null, 1]]",
    "an address that is a list": "[[[1], 1]]",
    "a pair of three": "[[1, 2, 3]]",
    "a pair of one": "[[1]]",
    "a value that is not a number": "[[2, NaN]]",
    "a value that is infinite": "[[2, Infinity]]",
    "a value nested deeper than the board speaks": '[[2, {"a": {"b": {"c": {"d": 1}}}}]]',
    "a list of a hundred": "[[2, %s]]" % list(range(100)),
    "a dictionary of twenty keys": "[[2, %s]]" % json.dumps({"k%d" % i: i for i in range(20)}),
    "a list of lists": "[[2, [[1, 2]]]]",
    "a good pair and a bad one": '[[0, "stop"], ["x", 1]]',
}


@pytest.mark.parametrize("what", sorted(BAD_BEATS))
def test_a_beat_with_a_pair_the_dictionary_does_not_have_is_not_the_board_heard_and_the_board_is_asked_again(flaky, tmp_path, what):
    Flaky.answers["/events"] = [_beat(BAD_BEATS[what])]
    recorder = Recorder(flaky, tasmota=None, out=tmp_path)
    recorder.start()
    try:
        time.sleep(1.0)
        assert not recorder.hears(), "the bad beat was taken for the board's word: %s" % BAD_BEATS[what]
        assert recorder.values == {} or "stop" not in recorder.values.values(), "a pair of a beat that is refused is not kept"
        until = time.time() + 12
        while time.time() < until and not recorder.hears():
            time.sleep(0.1)
        assert recorder.hears(), "the board answered well after the bad beat and the recorder did not hear it: %s" % recorder.why
        recorder.tick(time.time())
    finally:
        recorder.stop()
    got = dict(zip(rows(tmp_path)[0], rows(tmp_path)[-1]))
    assert got["board"] == "ok" and got["automaton/stage"] == "lighting", got


def test_the_forms_of_value_the_board_speaks_are_heard_and_written(flaky, tmp_path):
    beat = '[[0, "lighting"], [1, true], [2, 412.5], [3, {"at_least": 300}], [4, [1, 2]], [5, null]]'
    Flaky.answers["/events"] = [_beat(beat)]
    recorder = Recorder(flaky, tasmota=None, out=tmp_path)
    recorder.start()
    try:
        until = time.time() + 6
        while time.time() < until and not recorder.hears():
            time.sleep(0.05)
        assert recorder.hears()
        recorder.tick(time.time())
    finally:
        recorder.stop()
    got = dict(zip(rows(tmp_path)[0], rows(tmp_path)[-1]))
    assert (got["automaton/stage"], got["relays/auger"], got["flame/pin"], got["draught/blast_pressure"], got["draught/blast_pressure.spread"], got["fan/angle"]) == ("lighting", "on", "412.5", ">=300", "1 2", ""), got


def test_the_other_forms_of_a_value_are_heard_as_the_board_says_them(flaky, tmp_path):
    beat = '[[0, "stop"], [3, {"text": "a, b"}], [4, {"unknown": "own_zero"}], [5, 0]]'
    Flaky.answers["/events"] = [_beat(beat)]
    recorder = Recorder(flaky, tasmota=None, out=tmp_path)
    recorder.start()
    try:
        until = time.time() + 6
        while time.time() < until and not recorder.hears():
            time.sleep(0.05)
        recorder.tick(time.time())
    finally:
        recorder.stop()
    got = dict(zip(rows(tmp_path)[0], rows(tmp_path)[-1]))
    assert got["automaton/stage"] == "stop" and got["draught/blast_pressure"] == "a, b" and got["draught/blast_pressure.spread"] == "" and got["fan/angle"] == "0", got


# ---- a file that is still being written is not packed: the quiet window, and a write that comes while the archive is checked ----

def _aged(path, minutes):
    old = time.time() - minutes * 60
    os.utime(path, (old, old))


def test_a_file_of_a_day_gone_that_changed_in_the_last_ten_minutes_is_not_touched(tmp_path):
    recorder = Recorder("localhost", tasmota=None, out=tmp_path)
    yesterday = recorder.tick(_at("2026-11-02", "23:59:58"))
    written = yesterday.read_bytes()
    recorder.tick(_at("2026-11-03", "00:00:00"))
    _packed(recorder)
    assert yesterday.exists() and yesterday.read_bytes() == written, "the file the recorder wrote a moment ago is as it was"
    assert not list(tmp_path.glob("*.gz*")), "no archive, and no half of one"


def test_a_file_of_a_day_gone_that_has_been_quiet_for_ten_minutes_is_packed(tmp_path):
    path = tmp_path / "2026-11-02_localhost_none.csv"
    path.write_bytes(b"time,board\r\n00:00:00,ok\r\n")
    _aged(path, 11)
    assert [p.name for p in record.pack_days_gone(tmp_path, "2026-11-03")] == [path.name + ".gz"]
    assert not path.exists()


def test_a_write_that_comes_after_the_archive_was_checked_and_before_the_file_is_removed_keeps_the_file(tmp_path):
    path = tmp_path / "2026-11-02_localhost_none.csv"
    path.write_bytes(b"time,board\r\n00:00:00,ok\r\n")
    _aged(path, 11)

    def a_row_comes():
        with open(path, "ab") as f:
            f.write(b"00:00:01,ok\r\n")

    assert record.pack_days_gone(tmp_path, "2026-11-03", before_removal=a_row_comes) == []
    assert path.read_bytes().endswith(b"00:00:01,ok\r\n"), "the row that came was not lost with the file"
    assert not list(tmp_path.glob("*.gz*")), "an archive of a file that has changed since is not kept"


def test_the_pack_is_tried_again_when_the_file_has_been_quiet_long_enough(tmp_path, monkeypatch):
    monkeypatch.setattr(record, "QUIET_S", 1.0, raising=False)
    monkeypatch.setattr(record, "PACK_RETRY_S", 0.2, raising=False)
    recorder = Recorder("localhost", tasmota=None, out=tmp_path)
    yesterday = recorder.tick(_at("2026-11-02", "23:59:58"))
    recorder.tick(_at("2026-11-03", "00:00:00"))
    time.sleep(0.3)
    assert yesterday.exists(), "not packed a moment after it was written"
    until = time.time() + 8
    while time.time() < until and yesterday.exists():
        time.sleep(0.1)
    recorder.stop()
    assert not yesterday.exists() and (tmp_path / (yesterday.name + ".gz")).exists(), "packed once it had been quiet"


# ---- the record replaces itself when neither the board nor the socket is heard ----

def test_when_neither_the_board_nor_the_socket_has_been_heard_for_180_s_it_is_time_to_replace_the_process():
    assert record.restart_due(board_at=1000.0, socket_at=1000.0, now=1181.0) is True


@pytest.mark.parametrize("board_at, socket_at, now", [
    (1000.0, 1176.0, 1181.0),  # the board is silent, the socket was heard 5 s ago
    (1176.0, 1000.0, 1181.0),  # the socket is silent, the board was heard 5 s ago
    (1000.0, 1000.0, 1179.0),  # both silent, but not yet 180 s
])
def test_a_process_that_hears_one_of_them_or_is_silent_less_than_180_s_is_not_replaced(board_at, socket_at, now):
    assert record.restart_due(board_at, socket_at, now) is False


def test_a_process_just_started_that_has_heard_nobody_for_179_s_is_not_replaced_and_for_181_s_is():
    recorder = Recorder("127.0.0.1:1", tasmota=None, out=".")
    born = recorder.born
    assert record.restart_due(*recorder.last_heard(), now=born + 179) is False
    assert record.restart_due(*recorder.last_heard(), now=born + 181) is True


def test_without_a_socket_the_decision_is_by_the_board_alone():
    recorder = Recorder("127.0.0.1:1", tasmota=None, out=".")
    recorder.heard_at = recorder.born + 100
    assert record.restart_due(*recorder.last_heard(), now=recorder.born + 270) is False
    assert record.restart_due(*recorder.last_heard(), now=recorder.born + 281) is True


def test_a_socket_that_answers_keeps_the_process_even_when_the_board_is_silent():
    recorder = Recorder("127.0.0.1:1", tasmota="192.0.2.1", out=".")
    recorder.socket_at = recorder.born + 170
    assert record.restart_due(*recorder.last_heard(), now=recorder.born + 181) is False


class Replaced(Exception):
    pass


def run_main(monkeypatch, tmp_path, host, seconds, silent_s, argv_tail=()):
    """Runs main() for about SECONDS ticks with the silence limit silent_s seconds; returns what os.execv was called with, or None, and the output."""
    calls = []

    def execv(path, argv):
        calls.append((path, argv))
        raise Replaced

    ticks = []
    real_sleep = time.sleep

    def sleep(s):
        ticks.append(s)
        if len(ticks) > seconds:
            raise KeyboardInterrupt
        real_sleep(s)

    argv = ["record.py", host, "--out", str(tmp_path)] + list(argv_tail)
    monkeypatch.setattr(sys, "argv", argv)
    monkeypatch.setattr(os, "execv", execv)
    monkeypatch.setattr(record.time, "sleep", sleep)
    monkeypatch.setattr(record, "RESTART_S", silent_s, raising=False)
    try:
        record.main()
    except Replaced:
        pass
    return calls, argv


def dead_address():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return "127.0.0.1:%d" % port


def test_main_replaces_itself_with_the_same_interpreter_and_arguments_after_the_silence(monkeypatch, tmp_path, capsys):
    calls, argv = run_main(monkeypatch, tmp_path, dead_address(), seconds=5, silent_s=2)
    assert calls == [(sys.executable, [sys.executable] + argv)]
    said = [l for l in capsys.readouterr().out.splitlines() if "restarting itself" in l]
    assert len(said) == 1, said
    assert said[0][2] == ":" and said[0][5] == ":"  # HH:MM:SS first
    assert "restarting itself: board and socket silent for 2 s (" in said[0]
    assert "(" in said[0] and said[0].rstrip().endswith(")")


ANSWERS_WITHOUT_READINGS = {
    "200 without energy": (200, b"{}"),
    "200 with an empty energy": (200, b'{"StatusSNS": {"ENERGY": {}}}'),
    "200 that stops short": (200, b'{"StatusSNS": {"ENERGY": {"Power": 4'),
    "404 with a body": (404, b"not here"),
}


@pytest.mark.parametrize("what", sorted(ANSWERS_WITHOUT_READINGS))
def test_main_does_not_replace_itself_while_the_socket_answers_though_it_gives_no_readings_and_the_board_is_silent(monkeypatch, tmp_path, capsys, mains, what):
    monkeypatch.setattr(Socket, "status", ANSWERS_WITHOUT_READINGS[what][0])
    monkeypatch.setattr(Socket, "body", ANSWERS_WITHOUT_READINGS[what][1])
    monkeypatch.setattr(Socket, "cut", what.endswith("short"))
    calls, _ = run_main(monkeypatch, tmp_path, dead_address(), seconds=5, silent_s=2, argv_tail=["--tasmota", mains])
    assert calls == []
    assert "restarting itself" not in capsys.readouterr().out


def test_main_replaces_itself_when_the_socket_does_not_answer_either(monkeypatch, tmp_path):
    calls, argv = run_main(monkeypatch, tmp_path, dead_address(), seconds=5, silent_s=2, argv_tail=["--tasmota", dead_address()])
    assert calls == [(sys.executable, [sys.executable] + argv)]


@pytest.mark.parametrize("status, body", [(404, b"not here"), (200, b'{"not": "a dictionary"}')])
def test_main_does_not_replace_itself_while_the_board_answers_what_is_no_record(monkeypatch, tmp_path, capsys, mains, status, body):
    monkeypatch.setattr(Socket, "status", status)
    monkeypatch.setattr(Socket, "body", body)
    calls, _ = run_main(monkeypatch, tmp_path, mains, seconds=5, silent_s=2)
    assert calls == []
    assert "restarting itself" not in capsys.readouterr().out


@pytest.mark.parametrize("what", sorted(SOCKET_BAD))
def test_a_socket_that_answers_what_cannot_be_read_has_still_answered(mains, what):
    Socket.status = 200
    Socket.body, Socket.cut = SOCKET_BAD[what]
    reached, watts, volts = record.socket_reach(mains)
    assert reached is True and (watts, volts) == (None, None)


def test_a_socket_that_does_not_answer_has_not_answered():
    assert record.socket_reach(dead_address()) == (False, None, None)
