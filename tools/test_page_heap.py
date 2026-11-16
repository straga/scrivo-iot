"""The verdicts of tools/page_heap.py on samples made up here: a heap that stays passes, a heap that grows after its
collection is named at the minute it went over, and a measure that did not run is not a pass.

    uv run --no-project --with pytest --with websocket-client pytest tools/test_page_heap.py
"""
import os
import socket
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import page_heap
from page_heap import EVENTS_PER_MINUTE, judge, table

# events a minute at the burner's own pace, 1.8 a second
BRISK = 108


def series(collected, raw=None, events=BRISK):
    """Samples for minutes 0..len-1: the collected sizes, the raw sizes beside them, EVENTS more events each minute."""
    return [{"minute": m, "gc_mb": c, "raw_mb": (raw or collected)[m], "events": events * m} for m, c in enumerate(collected)]


def test_a_heap_that_stays_where_it_was_at_minute_one_passes():
    verdict, words = judge(series([3.0, 5.0, 5.2, 5.1, 5.4, 5.3]), minutes=5)
    assert verdict == "pass", words


def test_a_heap_the_collection_does_not_bring_back_fails_at_the_minute_it_went_over():
    # the page of 30.09: about 19 MB a minute that a collection gave back 2.5 MB of
    collected = [3.0, 5.0, 24.0, 43.0, 62.0, 81.0]
    verdict, words = judge(series(collected), minutes=5)
    assert verdict == "fail" and "minute 2" in words and "24.0 MB" in words, words


def test_growth_the_collection_takes_back_is_not_a_fault_of_the_page():
    # the heap as it stands climbs like a sawtooth; after the collection it is flat
    raw = [3.0, 12.0, 30.0, 8.0, 41.0, 15.0]
    verdict, words = judge(series([3.0, 5.0, 5.1, 5.0, 5.2, 5.1], raw=raw), minutes=5)
    assert verdict == "pass", words


def test_a_rise_within_the_limit_passes_and_one_above_it_fails():
    assert judge(series([3.0, 5.0, 5.9, 5.9]), minutes=3)[0] == "pass"
    assert judge(series([3.0, 5.0, 6.1, 6.1]), minutes=3)[0] == "fail"


def test_fewer_minutes_than_asked_is_not_measured():
    verdict, words = judge(series([3.0, 5.0, 5.1]), minutes=5)
    assert verdict == "not_measured" and "asked for 5 minutes" in words, words


def test_a_stream_that_brought_nothing_is_not_measured_however_flat_the_heap():
    verdict, words = judge(series([3.0, 5.0, 5.0, 5.0], events=EVENTS_PER_MINUTE - 1), minutes=3)
    assert verdict == "not_measured" and "fewer than" in words, words


def test_the_table_has_a_line_for_every_minute():
    lines = table(series([3.0, 5.0, 5.1])).splitlines()
    assert len(lines) == 4 and lines[0].startswith("minute") and lines[3].split()[0] == "2"


def listening():
    """A socket listening on a port of the machine's own: what another run of the tool leaves standing while it measures."""
    held = socket.socket()
    held.bind(("127.0.0.1", 0))
    held.listen(1)
    return held


def test_a_port_someone_listens_on_is_named_and_a_free_one_is_not():
    held = listening()
    try:
        busy = held.getsockname()[1]
        free = socket.socket()
        free.bind(("127.0.0.1", 0))
        gap = free.getsockname()[1]
        free.close()
        assert page_heap.taken([busy, gap]) == [busy]
        assert page_heap.taken([gap]) == []
    finally:
        held.close()


def test_a_port_whose_last_connection_is_winding_down_is_free():
    # the run before this one left the port with a closed connection in TIME_WAIT: a bind refuses it, and nothing listens
    server = listening()
    port = server.getsockname()[1]
    client = socket.socket()
    client.connect(("127.0.0.1", port))
    served, _ = server.accept()
    served.close()  # the side that closes first keeps the port in TIME_WAIT
    client.close()
    server.close()
    assert page_heap.taken([port]) == []


def test_a_run_that_finds_its_port_taken_measures_nothing_and_starts_nothing(monkeypatch, capsys):
    # a second run on the same ports attached to the first one's Chrome and doubled its counts: it must stop, saying so
    held = listening()
    started = []
    monkeypatch.setattr(page_heap.subprocess, "Popen", lambda *a, **k: started.append(a) or (_ for _ in ()).throw(AssertionError("started")))
    monkeypatch.setattr(page_heap, "STAND_PORT", held.getsockname()[1])
    monkeypatch.setattr(sys, "argv", ["page_heap.py", "--minutes", "1"])
    try:
        try:
            page_heap.main()
            code = None
        except SystemExit as gone:
            code = gone.code
    finally:
        held.close()
    assert code == 2, "a taken port is 'not measured' (2), got %r" % (code,)
    assert not started, "it started a process on a port someone holds"
    assert "not measured" in capsys.readouterr().out
