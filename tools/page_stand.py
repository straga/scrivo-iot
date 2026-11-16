"""A stand-in board for the device page: the real page files, a made-up board behind them, speaking the contract of docs/CONTRACT_Device_Api.md.

    uv run --no-project python tools/page_stand.py [--device burner] [--host 127.0.0.1] [--port 8790] [--state FILE]
                                                   [--log FILE] [--streams 2]

WHY THIS EXISTS. The page is judged by what a person sees and by what it sends, and both questions need a board
answering on the other end. The board itself is occupied - it is the one boiler we have - so everything that does
not need real hardware is asked here: screens at a phone's width and a computer's, in both themes, and what two
pages open at once send while a person works on one of them.

WHAT IT IS NOT. It is not the board and it proves nothing about the board's own code: the pascals here are made
up, the relays click nothing. What it does prove is the page - the markup, the styles, the table of texts and every
request the page decides to send - because those are the page's own and travel unchanged.

It answers the contract of docs/CONTRACT_Device_Api.md as the board does: /dictionary - the burner's own, made by the
burner_api crate (`cargo run -p burner_api --example first_burner_json`) -, /state - every value by address -, /events -
server-sent, what each reader has not got yet, a third reader refused in the board's own words -, /curve/<name>,
and the commands by address: /on/<address>, /off/<address>, /toggle/<address>, /press/<address>, /hold/<address>,
and /settings, /restart, /down/<name>.

EVERY POST IS WRITTEN DOWN, with the time and the body, because that is the record a question about two pages
fighting over a slider is answered from: who sent what, and whether a hand was on it.
"""
import argparse
import atexit
import http.server
import json
import math
import os
import shutil
import subprocess
import tempfile
import threading
import time
import urllib.parse

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# What the board says to a third page asking for the stream (page::BUSY).
BUSY = "the board is busy: two pages are open on it already - close one and reload this one"
# How often a quiet stream gets a beat (page::EVERY_MS): an SSE comment, so a reader that has gone is found.
EVERY_MS = 5000


def every_asked(uri, stream=None):
    """What `?every=` of the request URI asks of the stream, in seconds between two sends; 0 is Live (page::every_asked). Nothing asked, or what is no whole number of seconds up to the most the dictionary says
    (`stream`, as the board serves it), is the pace of everyone."""
    stream = stream or {"every": 5, "most": 60}
    asked = next((pair[len("every="):] for pair in urllib.parse.urlparse(uri).query.split("&") if pair.startswith("every=")), None)
    if asked and asked.isascii() and asked.isdigit() and int(asked) <= stream["most"]:
        return int(asked)
    return stream["every"]


def dictionary_of(root=ROOT, device=None):
    """A burner's dictionary, as its image serves it - made by a crate, never written by hand. With DEVICE that has an API crate of its own
 (devices/<device>/api) it is that crate's; without, the REFERENCE burner's, the first burner of burner_api (first_burner), which the scenarios
    and the comparisons of the new burner with it are made on."""
    own = device and os.path.isfile(os.path.join(root, "devices", device, "api", "Cargo.toml"))
    command = ["cargo", "+stable", "run", "-q", "-p", "%s_api" % device, "--example", "served_json"] if own else \
        ["cargo", "+stable", "run", "-q", "-p", "burner_api", "--example", "first_burner_json"]
    out = subprocess.run(command, cwd=root, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit("page_stand: the dictionary was not made:\n" + out.stderr[-2000:])
    return json.loads(out.stdout)


def same_dictionary(stand_version, dictionary_version):
    """Whether the values of a stand are of this dictionary: they stand by address, so by its CONTENT - the hash after the `+` of the version -, never by the kit before it, which is the image's own
    label and moves with every version of the burner. A version that names no content is no proof of one."""
    mine, theirs = (v.partition("+")[2] for v in (stand_version, dictionary_version))
    return bool(mine) and mine == theirs


def state_of(root, device):
    """The values the stand carries for DEVICE: devices/<device>/stand_values.json when it has one - by the addresses of its own dictionary -, else what its API crate
    makes (`cargo run -p <device>_api --example stand_values`: the values its modules write, by the functions the board writes them with), else the reference burner's
    (tools/page_stand_values.json)."""
    own = os.path.join(root, "devices", device, "stand_values.json")
    if os.path.isfile(own):
        with open(own) as f:
            return json.load(f)
    if os.path.isfile(os.path.join(root, "devices", device, "api", "examples", "stand_values.rs")):
        made = subprocess.run(["cargo", "+stable", "run", "-q", "-p", "%s_api" % device, "--example", "stand_values"], cwd=root, capture_output=True, text=True)
        if made.returncode != 0:
            raise SystemExit("page_stand: the values of %s were not made:\n%s" % (device, made.stderr[-2000:]))
        return json.loads(made.stdout)
    with open(os.path.join(HERE, "page_stand_values.json")) as f:
        return json.load(f)


# the thermostat's presses and the row each sets, and to which word: Start puts its switch to `on`, Stop to `off`. The real thermostat does more (the pause, the stage); the page reads this much
PRESS_SETS = {("thermostat", "start"): ("switch", "on"), ("thermostat", "stop"): ("switch", "off")}


def fresh_state(dictionary, state):
    """STATE as the stand starts from it with DICTIONARY. A snapshot of this dictionary - the hash after the `+` is the same - is the state as it is. A snapshot of another stands by addresses that mean other
    rows now: what it held at the address of one setting was a number of another, and the stand showed a smouldering of 2 s and a feed of 51 s. Of such a snapshot only the system card is kept - it stands first
    at the same addresses in every dictionary - and every setting of the dictionary is its default; the scenario's frames give the rest."""
    if same_dictionary(state.get("dictionary", ""), dictionary.get("version", "")):
        return state
    things = dictionary["things"]
    kept = [[at, value] for at, value in state.get("values", []) if at < len(things) and things[at]["card"] == "system"]
    seeded = [[at, {"text": next((s["default"] for s in dictionary["settings"] if s["name"] == t["name"]), "")}] for at, t in enumerate(things) if t["card"] == "settings" and not t.get("part_of")]
    return {**state, "values": sorted(kept + seeded)}


def with_settings(dictionary, state, settings):
    """STATE with the settings of a scenario written on - {name: text} -, so the page shows the numbers the frames were made with: a scenario that played the burner on three steps and a page that says the one step it is seeded with
    would contradict each other on the same screen. A name that is no setting of DICTIONARY is refused by its name."""
    at = {t["name"]: i for i, t in enumerate(dictionary["things"]) if t["card"] == "settings" and not t.get("part_of")}
    unknown = [name for name in settings if name not in at]
    if unknown:
        raise ValueError("the scenario sets %s, which the dictionary has no setting of" % ", ".join(unknown))
    written = {at[name]: {"text": text} for name, text in settings.items()}
    return {**state, "values": [[address, written.get(address, value)] for address, value in state.get("values", [])]}


class Board:
    """The made-up board: its dictionary, its values by address, who is listening, and the record of what was sent."""

    def __init__(self, dictionary, state, log_path, streams):
        self.dictionary = dictionary
        things = dictionary["things"]
        self.values = [None] * len(things)
        for at, value in state.get("values", []):
            self.values[at] = value
        self.lock = threading.Lock()
        self.changed = threading.Condition(self.lock)
        self.turn = 0
        self.max_streams = streams
        self.open_streams = 0
        self.log_path = log_path
        self.sent = []
        self.started = time.time()
        # how long a second of the board lasts in wall seconds: 1 on a stand that plays in real time, shorter when a scenario is played faster (--pace) - the pace a reader asks and the counts it goes on with are the board's seconds
        self.second = 1.0

    # ---- the record ----

    def wrote(self, what, body):
        """One POST as it arrived. Kept in memory and appended to the log, so a run that is still going can be
        read from outside."""
        line = {"at": round(time.time() - self.started, 3), "what": what, "body": body}
        with self.lock:
            self.sent.append(line)
        text = "%8.3f  %s  %s" % (line["at"], what, body if body is not None else "")
        print(text, flush=True)
        if self.log_path:
            with open(self.log_path, "a") as out:
                out.write(text + "\n")

    # ---- the values ----

    def carried(self, at):
        """Whether the value at AT goes on the wire: a number fixed in the image and a button never do."""
        thing = self.dictionary["things"][at]
        return "fixed" not in thing and thing.get("kinds") != ["button"]

    def records(self):
        """Every value the board sends, [[address, value], ...], in the order of the dictionary."""
        return [[at, v] for at, v in enumerate(self.values) if self.carried(at)]

    def state(self):
        return {"contract": self.dictionary["contract"], "dictionary": self.dictionary["version"], "values": self.records()}

    def announce(self):
        with self.changed:
            self.turn += 1
            self.changed.notify_all()

    def now_s(self):
        """The board's clock in its own seconds."""
        return (time.time() - self.started) / self.second

    def rounded(self, at, value):
        """VALUE of the thing at AT as it goes out: a number to the digits the dictionary says it is worth, as page::rounded - half away from zero, and what rounds to nothing is 0 and not minus 0."""
        digits = self.dictionary["things"][at].get("decimals")
        if digits is None:
            return value
        if isinstance(value, dict) and "at_least" in value:
            return {"at_least": self.rounded(at, value["at_least"])}
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            return value
        step = 10 ** digits
        r = math.floor(abs(value) * step + 0.5) / step
        return 0.0 if r == 0 else float(r if value >= 0 else -r)

    def changed_for(self, sent, now=None):
        """What a reader who has SENT - {address: value as last sent} - has not got: the records to send, and SENT brought up to them. A reader that opens has an empty SENT and gets everything, and one that has
        everything gets nothing - the board's page::write_changed. A value sent only when a reader opens (`once`) is not sent again, however it moves. A number is judged as it goes out, to the digits it is worth;
        a count that goes on by itself (`counts`) is judged against what the reader counts to NOW from what it was told, and said when it is not within a second of that."""
        now = self.now_s() if now is None else now
        out = []
        for at, value in self.records():
            thing = self.dictionary["things"][at]
            value = self.rounded(at, value)
            counts = thing.get("counts")
            if counts and isinstance(value, int) and not isinstance(value, bool):
                told = sent.get(("count", at))
                if told:
                    gone = int(now - told[1] + 0.5)
                    expected = told[0] + gone if counts == "up" else max(0, told[0] - gone)
                    if abs(value - expected) <= 1:
                        continue
                sent[("count", at)] = (value, now)
                sent[at] = value
                out.append([at, value])
                continue
            if counts:
                sent.pop(("count", at), None)
            if at in sent and (sent[at] == value or thing.get("once")):
                continue
            sent[at] = value
            out.append([at, value])
        return out

    def show(self, records):
        """One frame of a scenario: its values in place of what the stand carries; the rest stay as they were."""
        with self.lock:
            for at, value in records:
                self.values[at] = value
        self.announce()

    # ---- the commands, by address ----

    def _thing(self, address):
        """The row at ADDRESS, or the refusal that says why there is none - in the board's words."""
        things = self.dictionary["things"]
        try:
            at = int(address)
        except (TypeError, ValueError):
            return None, None, {"error": "'%s' is no address of this board" % address}
        if not 0 <= at < len(things) or things[at].get("part_of"):
            return None, None, {"error": "no thing of this board has the address %s" % address}
        return at, things[at], None

    def _answer(self, at):
        """What a command answers: the values of its row and of the row's parts."""
        thing = self.dictionary["things"][at]
        parts = [i for i, t in enumerate(self.dictionary["things"])
                 if t["card"] == thing["card"] and t.get("part_of") == thing["name"]]
        return {"values": [[i, self.values[i]] for i in [at] + parts if self.carried(i)]}

    def command(self, what, address, body=None):
        """A command by address, as the board takes it. The values change under the board's lock and the readers are
        told after it: a stream waits on the same lock, and telling it while holding it is waiting on oneself."""
        with self.lock:
            answer = self._command(what, address, body)
        if "values" in answer:
            self.announce()
        return answer

    def _command(self, what, address, body):
        at, thing, refusal = self._thing(address)
        if refusal:
            return refusal
        can = thing.get("can", [])
        needed = {"on": "switch", "off": "switch", "toggle": "toggle", "press": "press", "hold": "hold"}[what]
        if needed not in can:
            return {"error": "%s takes no %s: it takes %s" % (thing["name"], needed, ", ".join(can) or "no command")}
        if what == "on" and (thing["card"], thing["name"]) == ("relays", "auger"):
            # a banned feed is banned to the hand too: the board refuses the auger by hand and says what to press first (crates/automaton, hands.rs) - its words are the board's, said here in the same two phrases
            feed = next((i for i, t in enumerate(self.dictionary["things"]) if t["card"] == "automaton" and t["name"] == "feed" and not t.get("part_of")), None)
            if feed is not None and self.values[feed] is False:
                return {"error": "the feed is banned, and a banned feed is banned to the hand too: press Allow feed, then switch the auger by hand"}
        if what == "press":
            # what the board does with the press that a page can see: the thermostat's Start and Stop set its switch, which is what the page's one button reads
            effect = PRESS_SETS.get((thing["card"], thing["name"]))
            if effect:
                row = next((i for i, t in enumerate(self.dictionary["things"]) if t["card"] == thing["card"] and t["name"] == effect[0] and not t.get("part_of")), None)
                if row is not None:
                    self.values[row] = effect[1]
        if what in ("on", "off"):
            self.values[at] = what == "on"
        elif what == "toggle":
            self.values[at] = not bool(self.values[at])
        elif what == "hold":
            span = thing["range"]
            try:
                value = int(body)
            except (TypeError, ValueError):
                return {"error": "%s takes a whole number" % thing["name"]}
            if value < span["min"] or value > span["max"]:
                return {"error": "%s takes %d to %d; %d is not one of them"
                                 % (thing["name"], span["min"], span["max"], value)}
            self.values[at] = value
            self.values[span["at"]] = value
        return self._answer(at)

    def save(self, body):
        """The tab's save, name=value pairs; the readers are told after the lock, as for a command."""
        with self.lock:
            answer = self._save(body)
        if answer.get("changed"):
            self.announce()
        return answer

    def _save(self, body):
        things = self.dictionary["things"]
        changed = []
        for name, value in urllib.parse.parse_qsl(body, keep_blank_values=True):
            at = next((i for i, t in enumerate(things) if t["card"] == "settings" and t["name"] == name
                       and not t.get("part_of")), None)
            if at is None:
                return {"error": {"code": "no_setting", "name": name}}
            if (self.values[at] or {}).get("text") != value:
                changed.append({"name": name, "value": value})
                self.values[at] = {"text": value}
        return {"changed": changed}


def play(board, frames, start, hold, loop=False, pace=1.0, stop=None):
    """Shows FRAMES from the second START, one every PACE seconds - or holds the frame of START, when HOLD. With LOOP it
    starts over at the first frame after the last, until STOP is set: a measure that runs longer than a scenario."""
    at = next((i for i, f in enumerate(frames) if f["at"] >= start), len(frames) - 1)
    board.show(frames[at]["values"])
    if hold:
        return
    stop = stop or threading.Event()
    while not stop.is_set():
        if at + 1 >= len(frames):
            if not loop:
                return
            at = -1
        if stop.wait(pace):
            return
        at += 1
        board.show(frames[at]["values"])


# ---- the curves ----

def curve(board, name):
    """A curve the way the board answers /curve/<name> (crates/page, `Curve`): `step_ms` and `lines`, each line its unit and points oldest first.

    A line carries its words and its span only for a module the page's menu does not place yet (none now); the flame's, the water's, the flue's and the draught's say
    neither - what a line is called and what it is drawn against are the menu's (the burner's menu.json, merged by tools/page_sources.py).

    The numbers are drawn from a sine with a little noise on it - enough that a line has a shape to read and a scale to be drawn against, which
    is all a picture of a curve is judged on.
    """
    # name: (step_ms, points, middle, swing, lines), a line (unit, words, span, what the pin's millivolts make of it)
    shapes = {
        "draught_fast": (100, 600, 412, 30, [("Pa", None, None, lambda mv: (mv - 250) / 4.2), ("mV", None, None, None)]),
        "draught_slow": (1000, 900, 412, 45, [("Pa", None, None, lambda mv: (mv - 250) / 4.2), ("mV", None, None, None)]),
        "flame_fast": (100, 600, 2480, 60, [("mV", None, None, None)]),
        "flame_slow": (1000, 900, 2480, 90, [("mV", None, None, None)]),
        "flue": (1000, 900, 148.6, 9, [("\u00b0C", None, None, None)]),
        # the fan's three curves, as the board sends them (fan.rs): what was asked, what came out and the angle that held it; the miss and the P and I halves of the move
        "fan_holding": (1000, 300, 51, 6, [("Pa", None, None, lambda v: 51.0), ("Pa", None, None, lambda v: 51 + (v - 51) * 3), ("%", None, None, lambda v: 30 - (v - 51) * 1.1)]),
        "fan_response": (1000, 30, 51, 12, [("Pa", None, None, lambda v: 51.0), ("Pa", None, None, lambda v: 51 + (v - 51) * 3), ("%", None, None, lambda v: 30 - (v - 51) * 1.1)]),
        "fan_regulator": (1000, 300, 51, 6, [("Pa", None, None, lambda v: v - 51), ("% a second", None, None, lambda v: math.sin(v * 0.7) * 0.4),
                                             ("% a second", None, None, lambda v: math.cos(v * 0.9) * 0.2)]),
    }
    # a quarter of an hour of a socket's water, one point every two seconds
    shapes.update({"water_%d" % n: (2000, 450, 60 - 3 * n, 4, [("\u00b0C", None, None, None)]) for n in range(1, 8)})
    if name not in shapes:
        return None
    step_ms, count, middle, swing, wanted = shapes[name]
    turn = board.turn
    points = []
    for i in range(count):
        phase = (i + turn) / 37.0
        v = middle + swing * math.sin(phase) + swing * 0.18 * math.sin(phase * 5.3)
        points.append(round(v, 2))
    lines = []
    for unit, words, span, make in wanted:
        line = {"unit": unit, "points": points if make is None else [round(make(v), 2) for v in points]}
        if words is not None:
            line["words"] = words
        if span is not None:
            line["span"] = list(span)
        lines.append(line)
    return {"step_ms": step_ms, "lines": lines}


class Handler(http.server.SimpleHTTPRequestHandler):
    board = None
    page_dir = None
    # With --board: the real board the page is pointed at. The stand then serves the page and nothing of a board, so a
    # page opened without ?board= is told so in words and never reads the made-up one as if it were real.
    real = None

    def log_message(self, *_):
        pass

    # ---- reading ----

    def do_GET(self):
        path = urllib.parse.urlparse(self.path).path
        board_paths = ("/state", "/events", "/sent", "/dictionary")
        if self.real and (path in board_paths or path.startswith("/curve/")):
            return self.json(404, {"error": f"this stand serves the page only - open it as /?board={self.real}"})
        if path == "/dictionary":
            return self.json(200, self.board.dictionary)
        if path == "/state":
            with self.board.lock:
                return self.json(200, self.board.state())
        if path == "/sent":
            # The record, for a probe standing outside. Not part of the board's contract: the board has no such
            # thing, and nothing the page asks for goes through here.
            with self.board.lock:
                return self.json(200, list(self.board.sent))
        if path == "/events":
            return self.stream()
        if path.startswith("/curve/"):
            drawn = curve(self.board, urllib.parse.unquote(path[len("/curve/"):]))
            if drawn is None:
                return self.json(404, {"error": "no such curve"})
            return self.json(200, drawn)
        return self.file(path)

    def file(self, path):
        name = "index.html" if path in ("/", "") else path.lstrip("/")
        whole = os.path.join(self.page_dir, name)
        if not os.path.isfile(whole) or not os.path.abspath(whole).startswith(self.page_dir):
            self.send_error(404, "no such file on this page")
            return
        kind = {".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json",
                ".svg": "image/svg+xml"}.get(os.path.splitext(name)[1], "application/octet-stream")
        body = open(whole, "rb").read()
        self.send_response(200)
        self.send_header("Content-Type", kind + "; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        self.wfile.write(body)

    def stream(self):
        """The values as server-sent events, each reader getting what it has not got - the board's own way. The board
        holds two at once and refuses the third IN WORDS, which is what the page reads to tell a busy board from a
        board that is gone."""
        with self.board.lock:
            if self.board.open_streams >= self.board.max_streams:
                taken = True
            else:
                taken = False
                self.board.open_streams += 1
        self.board.wrote("/events", "refused - both taken" if taken else "opened %s" % (urllib.parse.urlparse(self.path).query or "no pace asked"))
        if taken:
            self.send_response(503)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.end_headers()
            self.wfile.write(BUSY.encode())
            return
        try:
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            sent, seen = {}, -1
            # THE PACE THIS READER ASKED (`?every=`), as the board keeps it: not more often, a change that comes early is held with the others; Live (0) is every change, for as long as the dictionary says, then the pace of everyone
            stream = self.board.dictionary.get("stream") or {"every": 5, "live_s": 600, "most": 60}
            every, began, second, next_ok = every_asked(self.path, stream), time.time(), self.board.second, 0.0
            while True:
                with self.board.changed:
                    quiet = False
                    if self.board.turn == seen:
                        quiet = not self.board.changed.wait(EVERY_MS / 1000.0)
                if not quiet and time.time() < next_ok:
                    time.sleep(next_ok - time.time())
                with self.board.changed:
                    seen = self.board.turn
                    records = self.board.changed_for(sent)
                if records:
                    self.wfile.write(("data: " + json.dumps(records, separators=(",", ":")) + "\n\n").encode())
                    now = time.time()
                    live = every == 0 and now - began < stream["live_s"] * second
                    next_ok = now + (0 if live else (stream["every"] if every == 0 else every) * second)
                elif quiet:
                    self.wfile.write(b":\n\n")
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError, OSError):
            pass
        finally:
            with self.board.lock:
                self.board.open_streams -= 1
            self.board.wrote("/events", "closed")

    # ---- writing ----

    def do_POST(self):
        if self.real:
            return self.json(404, {"error": f"this stand serves the page only - open it as /?board={self.real}"})
        path = urllib.parse.urlparse(self.path).path
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length).decode() if length else None
        self.board.wrote(path, body)
        for what in ("on", "off", "toggle", "press", "hold"):
            prefix = "/%s/" % what
            if path.startswith(prefix):
                return self.json(200, self.board.command(what, urllib.parse.unquote(path[len(prefix):]), body))
        if path == "/settings":
            return self.json(200, self.board.save(body or ""))
        if path == "/restart":
            return self.json(200, {"going": "restart", "back_in_s": 20})
        if path.startswith("/down/"):
            said = urllib.parse.unquote(path[len("/down/"):])
            if said != self.board.dictionary["board"]:
                return self.json(200, {"error": "the board goes down only when asked by its name, %s; the request said '%s'"
                                                % (self.board.dictionary["board"], said)})
            return self.json(200, {"going": "recovery"})
        return self.json(404, {"error": "no such thing on this board"})

    def json(self, code, body):
        text = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(text)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(text)


class Threaded(http.server.ThreadingHTTPServer):
    daemon_threads = True
    # THE QUEUE OF CONNECTIONS NOT ACCEPTED YET: socketserver's five reset half of what a browser opens in the one moment it loads a page (a dozen files and two streams) - an empty page and
    # ERR_CONNECTION_RESET on the owner's stand
    request_queue_size = 128
    allow_reuse_address = True


def open_server(host, port):
    """The stand's server on HOST:PORT. It is the host that says who may reach it: 127.0.0.1 only this machine, 0.0.0.0 the whole network."""
    return Threaded((host, port), Handler)


def make_parser():
    ap = argparse.ArgumentParser()
    ap.add_argument("--device", default="burner")
    ap.add_argument("--page", default=None,
                    help="a folder of page files to serve instead of the page of the device's kit - a revision taken out of git, "
                         "so a count taken before a change can be taken again afterwards")
    ap.add_argument("--host", default="127.0.0.1",
                    help="the address the stand listens on: 127.0.0.1 - this machine alone, as it always was - or 0.0.0.0 to open it to the network, "
                         "where it has no sign-in and every command it answers is the page's own")
    ap.add_argument("--port", type=int, default=8790)
    ap.add_argument("--state", default=None, help="values by address; the device's own stand_values.json, else the reference burner's")
    ap.add_argument("--dictionary", default=None, help="a dictionary file instead of the one the crate makes")
    ap.add_argument("--log", default=None, help="where every POST is written down as well as printed")
    ap.add_argument("--streams", type=int, default=2, help="streams held at once, as the board holds two")
    ap.add_argument("--board", default=None,
                    help="a real board's address: serve the page alone, pointed at that board by ?board= "
                         "- the made-up board is not raised")
    ap.add_argument("--contract", type=int, default=None,
                    help="the contract number the dictionary says instead of its own - another one is how a page is "
                         "shown a board it cannot read (docs/CONTRACT_Device_Api.md)")
    ap.add_argument("--scenario", default=None,
                    help="frames the automaton's own code wrote (cargo run -p automaton_card --example scenario), "
                         "played one a second over the state")
    ap.add_argument("--settings-of", default=None, help="a scenario file whose `settings` are written on the state the stand starts from: the page then says the numbers the frames were made with")
    ap.add_argument("--from", dest="start", type=int, default=0, help="the second of the scenario to start from")
    ap.add_argument("--at", type=int, default=None, help="hold the frame of this second instead of playing")
    ap.add_argument("--loop", action="store_true", help="play the scenario round again when it ends")
    ap.add_argument("--pace", type=float, default=1.0,
                    help="seconds between two frames; the board's own stream comes about 1.8 times a second, so 0.55 "
                         "puts the page under the same load (the scenario's clocks then run faster than real time)")
    return ap


def main():
    args = make_parser().parse_args()

    if args.page:
        page_dir = os.path.abspath(args.page)
        if not os.path.isdir(page_dir):
            raise SystemExit("page_stand: no page at %s" % page_dir)
    else:
        # the page the kit of the device carries, laid in a folder of its own and taken away when the stand ends: what is served is what the board would serve
        import page_kit
        page_dir = page_kit.page_into(args.device, tempfile.mkdtemp(prefix="page_stand_"))
        atexit.register(shutil.rmtree, page_dir, ignore_errors=True)
    dictionary = json.load(open(args.dictionary)) if args.dictionary else dictionary_of(ROOT, args.device)
    if args.contract is not None:
        dictionary["contract"] = args.contract
    state = fresh_state(dictionary, json.load(open(args.state)) if args.state else state_of(ROOT, args.device))

    if args.settings_of:
        try:
            state = with_settings(dictionary, state, json.load(open(args.settings_of)).get("settings", {}))
        except ValueError as why:
            raise SystemExit("page_stand: %s" % why)
    Handler.board = Board(dictionary, state, args.log, args.streams)
    if args.scenario:
        frames = json.load(open(args.scenario))["frames"]
        hold = args.at is not None
        # a scenario played faster than real time shortens the board's second with it: the pace a reader asks and the counts it goes on with are counted in them
        Handler.board.second = args.pace
        threading.Thread(target=play, args=(Handler.board, frames, args.at if hold else args.start, hold),
                         kwargs={"loop": args.loop, "pace": args.pace}, daemon=True).start()
    Handler.page_dir = page_dir
    Handler.real = args.board
    if args.board:
        print("page_stand: http://%s:%d/?board=%s · %s · the page alone, the board is %s"
              % (args.host, args.port, args.board, page_dir, args.board), flush=True)
    else:
        print("page_stand: http://%s:%d/ · %s · %d streams"
              % (args.host, args.port, page_dir, args.streams), flush=True)
    open_server(args.host, args.port).serve_forever()


if __name__ == "__main__":
    main()
