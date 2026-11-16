"""A client of a board of the contract `CONTRACT` below (docs/CONTRACT_Device_Api.md), for every tool that reads or commands the burner
without its page: the dictionary once, then values by card and code, commands by code through the address the
dictionary gives, and the stream.

    board = Board("<board-ip>")
    number(board.value("draught", "blast_pressure"))        # 31.5, or None when there is no number now
    board.command("on", "relays", "pump")                     # {"values": [[68, true]]} or {"error": "..."}

AN ANSWER OF ANOTHER CONTRACT, OR OF ANOTHER DICTIONARY THAN THE ONE HELD, IS REFUSED BEFORE A VALUE IS READ: the same
address means another thing there, and a number read by the wrong dictionary is a number about something else.
"""
import http.client
import json
import socket
import time
import urllib.parse
import urllib.request

CONTRACT = 10


class Refused(Exception):
    """The board answers what this client cannot read, or has not got what was asked - in words."""


def _get(url, timeout):
    with urllib.request.urlopen(url, timeout=timeout) as r:
        return json.loads(r.read())


class Board:
    def __init__(self, host, timeout=10, contracts=None):
        """CONTRACTS - the numbers this client reads, this module's own when none is given. A tool that carries a board across an image that raised the number (the settings
        taken before a flash and put back after it, tools/board_settings.py) reads both."""
        self.host = host
        self.timeout = timeout
        self.contracts = tuple(contracts) if contracts else (CONTRACT,)
        self._dictionary = None

    def reads(self):
        """The numbers this client reads, for a refusal in words."""
        return "/".join(str(n) for n in self.contracts)

    def url(self, path):
        return "http://%s%s" % (self.host, path)

    def dictionary(self):
        """The board's dictionary, taken once; one of another contract is refused."""
        if self._dictionary is None:
            d = _get(self.url("/dictionary"), self.timeout)
            if d.get("contract") not in self.contracts:
                raise Refused("the board speaks contract %s and this client reads contract %s" % (d.get("contract"), self.reads()))
            self._dictionary = d
        return self._dictionary

    def address(self, card, name, part=None):
        """The address of the row NAME of CARD - or of its part PART; refused by name when the dictionary has not got it."""
        for at, t in enumerate(self.dictionary()["things"]):
            if t["card"] != card:
                continue
            if part is None and t["name"] == name and not t.get("part_of"):
                return at
            if part is not None and t.get("part_of") == name and t["name"] == part:
                return at
        raise Refused("the board's dictionary has no %s/%s%s" % (card, name, "." + part if part else ""))

    def state(self):
        """Every value now, {address: value}, from /state: its contract and its dictionary checked first."""
        said = _get(self.url("/state"), self.timeout)
        if said.get("contract") not in self.contracts:
            raise Refused("the board answers contract %s and this client reads contract %s" % (said.get("contract"), self.reads()))
        held = self.dictionary()["version"]
        if said.get("dictionary") != held:
            raise Refused("the board answers by dictionary %s and this client holds %s - take the dictionary again"
                          % (said.get("dictionary"), held))
        return {at: value for at, value in said.get("values", [])}

    def value(self, card, name, state=None, part=None):
        """The value of the row NAME of CARD, or of its part PART, in STATE - now, when none is given."""
        state = self.state() if state is None else state
        return state.get(self.address(card, name, part))

    def command(self, way, card, name, body=None):
        """WAY - on, off, toggle, press, hold - to the row NAME of CARD, BODY the number a hold carries: the board's
        answer, {"values": [...]} or {"error": "..."}."""
        data = b"" if body is None else str(body).encode()
        request = urllib.request.Request(self.url("/%s/%d" % (way, self.address(card, name))), data=data, method="POST")
        with urllib.request.urlopen(request, timeout=self.timeout) as r:
            return json.loads(r.read())

    def save(self, pairs):
        """PAIRS - [(name, text)] - written the way the page's Save writes them: one POST /settings, every pair escaped as a browser escapes a form. The board's answer, {"changed": [{"name", "value"}]} or {"error": ...}."""
        body = "&".join("%s=%s" % (urllib.parse.quote(n, safe="-_.!~*'()"), urllib.parse.quote(v, safe="-_.!~*'()")) for n, v in pairs)
        request = urllib.request.Request(self.url("/settings"), data=body.encode(), method="POST")
        with urllib.request.urlopen(request, timeout=self.timeout) as r:
            return json.loads(r.read())

    def stream(self, seconds, each, stop=None, every=None):
        """Reads /events for SECONDS, or until the event STOP is set, handing EACH (time, bytes, records) - records None
        for a beat. EVERY is the pace asked of the board, in seconds between two sends: 0 - every change, None - the pace
        of everyone (the board's own, 5 s). Answers the bytes of body read."""
        host, _, port = self.host.partition(":")
        # longer than a beat: a quiet stream says something every 5 s, and a read that times out spoils the connection
        conn = http.client.HTTPConnection(host, int(port or 80), timeout=8)
        conn.request("GET", "/events" if every is None else "/events?every=%d" % every)
        resp = conn.getresponse()
        if resp.status != 200:
            raise Refused("the stream answered %d: %s" % (resp.status, resp.read().decode(errors="replace")))
        until, buf, read = time.time() + seconds, b"", 0
        try:
            while time.time() < until and not (stop is not None and stop.is_set()):
                try:
                    chunk = resp.read1(4096)
                except (socket.timeout, TimeoutError):
                    break
                if not chunk:
                    break
                read += len(chunk)
                buf += chunk
                while b"\n\n" in buf:
                    one, buf = buf.split(b"\n\n", 1)
                    for line in one.split(b"\n"):
                        if line.startswith(b"data: "):
                            each(time.time(), len(line) + 2, json.loads(line[6:]))
                        elif line.startswith(b":"):
                            each(time.time(), len(line) + 2, None)
        finally:
            conn.close()
        return read


def going_on(way, value, told_at, now):
    """What a count of seconds the board said once - VALUE at TOLD_AT, by the clock of this client - is at NOW: up gains the seconds gone, to the nearest, down loses them to nothing (the board says a count when it
    jumps and not every second, and a client counts it on: docs/CONTRACT_Device_Api.md, `counts`)."""
    gone = int(now - told_at + 0.5)
    return value + gone if way == "up" else max(0, value - gone)


def number(value):
    """VALUE as a number: a reading; None for the top of a scale, no number now, a switch, a code."""
    return value if isinstance(value, (int, float)) and not isinstance(value, bool) else None


def is_on(value):
    """Whether VALUE is on; None when it says nothing of on or off."""
    return value if isinstance(value, bool) else None


def word(value):
    """The code VALUE is; None when it is no code."""
    return value if isinstance(value, str) else None
