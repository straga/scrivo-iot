"""The stand answers over HTTP the way a page reaches it: a command by address comes back with the values it left, and
a reader of the stream gets the change - through the server, not by calling the stand's methods past it.

    uv run --no-project --with pytest pytest -q tools/test_page_stand_http.py
"""
import json
import threading
import urllib.request

import page_stand
from page_stand import Board, Handler, Threaded
from test_page_stand_stream import DICTIONARY


def serve():
    Handler.board = Board(DICTIONARY, {"values": [[0, False], [1, 21.5], [2, 1000]]}, None, 2)
    Handler.page_dir = "/nonexistent"
    Handler.real = None
    server = Threaded(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, "http://127.0.0.1:%d" % server.server_address[1]


def post(url, body=b""):
    with urllib.request.urlopen(urllib.request.Request(url, data=body, method="POST"), timeout=3) as r:
        return json.loads(r.read())


def test_a_command_by_address_is_answered_with_the_values_it_left():
    server, base = serve()
    try:
        assert post(base + "/on/0") == {"values": [[0, True]]}
        with urllib.request.urlopen(base + "/state", timeout=3) as r:
            assert json.loads(r.read())["values"][0] == [0, True]
    finally:
        server.shutdown()


def test_a_command_the_thing_does_not_take_is_refused_in_words_over_http_too():
    server, base = serve()
    try:
        said = post(base + "/press/0")
        assert "error" in said and "sw" in said["error"], said
    finally:
        server.shutdown()


def test_a_reader_of_the_stream_gets_a_command_as_one_record():
    server, base = serve()
    try:
        stream = urllib.request.urlopen(base + "/events", timeout=6)
        first = stream.readline() + stream.readline()
        assert first.startswith(b"data: [[0,false],[1,21.5],[2,1000]]"), first
        post(base + "/on/0")
        second = stream.readline()
        assert second == b"data: [[0,true]]\n", second
        stream.close()
    finally:
        server.shutdown()


assert page_stand.EVERY_MS > 0


# ---- who may reach the stand: itself by default, the network only when asked (--host) ----

def test_the_stand_listens_to_itself_unless_asked_otherwise():
    assert page_stand.make_parser().parse_args([]).host == "127.0.0.1"
    assert page_stand.make_parser().parse_args(["--host", "0.0.0.0"]).host == "0.0.0.0"


def test_the_server_stands_on_the_address_it_is_given():
    mine = page_stand.open_server("127.0.0.1", 0)
    every = page_stand.open_server("0.0.0.0", 0)
    try:
        assert mine.server_address[0] == "127.0.0.1"
        assert every.server_address[0] == "0.0.0.0"
    finally:
        mine.server_close()
        every.server_close()


def test_a_page_that_opens_many_connections_at_once_gets_every_one_answered():
    # a browser opens a dozen connections at the moment it loads a page, and the stand with a queue of five reset half of them - the owner saw an empty page and "Failed to load resource: ERR_CONNECTION_RESET"
    server, base = serve()
    failed = []

    def get():
        try:
            with urllib.request.urlopen(base + "/state", timeout=5) as r:
                r.read()
        except OSError as why:
            failed.append(str(why))

    try:
        for _ in range(5):
            crowd = [threading.Thread(target=get) for _ in range(60)]
            for t in crowd:
                t.start()
            for t in crowd:
                t.join()
        assert not failed, "%d of 300 answers were reset or refused, the first: %s" % (len(failed), failed[0])
    finally:
        server.shutdown()
