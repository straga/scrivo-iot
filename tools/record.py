"""The burner's run written down by the second, on the computer: what the board showed and what was on,
kept in a CSV of the day, so a run can be read afterwards - how the lighting went, how the fire went out.

    uv run --no-project python tools/record.py <board-ip> --tasmota <socket-ip>
    uv run --no-project python tools/record.py <board-ip> --out ~/runs

THE BOARD IS READ THE WAY ITS PAGE READS IT (contract 3, docs/CONTRACT_Device_Api.md): the dictionary once, the values
from one stream /events - never /state asked again and again, which costs the board's heap, and the board has two
places for a stream. A row a second takes the values as the stream last left them: the time, the board's word ("ok",
or "no answer" and why), the socket's power and voltage, then every thing of the dictionary by card and name. A board
that does not answer is a row that says so with its values empty - not a gap, and not the values it last had. A board
that comes back with another dictionary starts a new file: a column read by the wrong dictionary is another thing.

When the day turns, the file of every day gone that has been quiet for ten minutes is packed into a .csv.gz beside it, read
back and compared with the file, and only then is the file removed, if it is found as it was; the file of the day now is not
touched. A pack that does not read back the same, or of a file written meanwhile, leaves the file as it was.

The Tasmota socket is only read (Status 10); nothing is ever sent to the board or to the socket but a GET.
"""
import argparse
import csv
import datetime
import gzip
import http.client
import json
import os
import pathlib
import shutil
import sys
import threading
import time
import urllib.error
import urllib.request

from board_client import Board, Refused, going_on

# A stream says something at least every 5 s (its beat); a board silent longer than this is not answering.
SILENT_S = 8
# Between two tries at a board that is gone.
RETRY_S = 2
# Neither the board nor the socket heard this long: the process replaces itself, because a network lost inside a process is not found again by it.
RESTART_S = 180
# What a row says when the board is heard.
OK = "ok"
# How often the board is asked to send: a row is written a second, so a second is as often as it can be heard to any use - and Live would be held to its ten minutes and fall back in the middle of a record.
ASK_EVERY_S = 1


def columns(dictionary):
    """The things of DICTIONARY by card and name, a part after its row's name: the CSV's columns, in its order."""
    named = []
    for t in dictionary["things"]:
        if t.get("part_of"):
            named.append("%s/%s.%s" % (t["card"], t["part_of"], t["name"]))
        else:
            named.append("%s/%s" % (t["card"], t["name"]))
    return named


def cell(value):
    """VALUE as a field: a number as it came, a switch on or off, a code, a text; no number now - an empty field."""
    if value is None:
        return ""
    if isinstance(value, bool):
        return "on" if value else "off"
    if isinstance(value, (int, float, str)):
        return str(value)
    if isinstance(value, list):
        return " ".join(str(v) for v in value)
    if isinstance(value, dict):
        if "at_least" in value:
            return ">=%s" % value["at_least"]
        if "text" in value:
            return value["text"]
    return ""  # unknown, and whatever this reader does not know: never a zero


# A FILE THAT HAS CHANGED IN THE LAST QUIET_S SECONDS is not packed: the recorder may still be writing it (the day turns at midnight, the last row of the day is written a second before), and a file packed and removed under a write
# loses the rows that come after the archive was made. The pack is tried again every PACK_RETRY_S until the file has been quiet long enough.
QUIET_S = 600
PACK_RETRY_S = 60


def pack_pass(out, today, now=None, before_removal=None):
    """One pass over OUT: packs each CSV whose day (the date its name starts with) is before TODAY (a "YYYY-MM-DD") and that has not changed for QUIET_S seconds into a .csv.gz, and removes the CSV only after the archive reads back byte for
    byte the same AND the file is found as it was when the packing began (size and time of change): a row written meanwhile keeps the file, and the archive of the shorter file is not kept. BEFORE_REMOVAL is called after the check
    and before that last look (a test writes a row there). An archive already there is left alone, and so is its CSV.
    Returns (the archives made, the files left for later because they changed lately)."""
    now = time.time() if now is None else now
    packed, waiting = [], []
    for path in sorted(pathlib.Path(out).glob("*.csv")):
        if path.name[:10] >= today:
            continue
        target = path.with_name(path.name + ".gz")
        part = path.with_name(path.name + ".gz.part")
        if target.exists():
            continue
        try:
            before = path.stat()
            if now - before.st_mtime < QUIET_S:
                waiting.append(path)
                continue
            with open(path, "rb") as src, gzip.open(part, "wb") as dst:
                shutil.copyfileobj(src, dst)
            with open(path, "rb") as src, gzip.open(part, "rb") as back:
                same = True
                while same:
                    a, b = src.read(1 << 20), back.read(1 << 20)
                    same = a == b
                    if not a and not b:
                        break
            if same and before_removal is not None:
                before_removal()
            after = path.stat()
            if not same or (after.st_size, after.st_mtime_ns) != (before.st_size, before.st_mtime_ns):
                part.unlink()
                continue
            os.replace(part, target)
            path.unlink()
            packed.append(target)
        except OSError:
            if part.exists():
                part.unlink()
    return packed, waiting


def pack_days_gone(out, today, now=None, before_removal=None):
    """The archives made by one pass (pack_pass)."""
    return pack_pass(out, today, now, before_removal)[0]


# What an answer of a board or a socket can do to the one who reads it besides not coming: be a body that is no text (a compressed one, as a board does for a moment after a flash: UnicodeDecodeError), no JSON,
# JSON of another shape than the one asked (TypeError, AttributeError, KeyError), or stop short (http.client.HTTPException). All of these are ValueError, LookupError, TypeError, AttributeError or HTTPException.
UNREADABLE = (ValueError, LookupError, TypeError, AttributeError, http.client.HTTPException)


def readable(dictionary):
    """Whether DICTIONARY, as a board said it, can be read by the columns and the stream: an object with a version and things that are rows of a card and a name."""
    return (isinstance(dictionary, dict) and isinstance(dictionary.get("version"), str) and isinstance(dictionary.get("things"), list)
            and all(isinstance(t, dict) and isinstance(t.get("card"), str) and isinstance(t.get("name"), str) for t in dictionary["things"]))


# THE FORMS OF A VALUE the board speaks, as the page reads them: nothing, a switch, a number, a word, a list of those, and an object of a few of them (`at_least`, `text`, `unknown`...). What is more than that - a number that is no
# number, a list of lists, a hundred members, an object deeper than two - is not the board's word.
MAX_MEMBERS = 16
MAX_DEPTH = 2


def form_ok(value, depth=0):
    """Whether VALUE is of a form the board speaks."""
    if value is None or isinstance(value, (bool, str)):
        return True
    if isinstance(value, int):
        return True
    if isinstance(value, float):
        return value == value and value not in (float("inf"), float("-inf"))
    if isinstance(value, list):
        return len(value) <= MAX_MEMBERS and all(not isinstance(v, (list, dict)) and form_ok(v) for v in value)
    if isinstance(value, dict):
        return depth < MAX_DEPTH and len(value) <= MAX_MEMBERS and all(isinstance(k, str) and form_ok(v, depth + 1) for k, v in value.items())
    return False


def pair_ok(pair, size):
    """Whether PAIR is [address, value] of a dictionary of SIZE rows: two members, an address that is a whole number inside the dictionary (a switch is no address), a value of a form the board speaks."""
    return (isinstance(pair, (list, tuple)) and len(pair) == 2 and isinstance(pair[0], int) and not isinstance(pair[0], bool) and 0 <= pair[0] < size and form_ok(pair[1]))


def restart_due(board_at, socket_at, now):
    """Whether the board and the socket have both been silent for RESTART_S at NOW; the times are when each was last heard, or when the process began."""
    return now - board_at >= RESTART_S and now - socket_at >= RESTART_S


def socket_reach(host, timeout=0.8):
    """(answered, watts, volts) of the Tasmota socket at HOST. ANSWERED is whether an HTTP answer came over the network, whatever it said: the network is alive then, and the readings are None when the answer holds none."""
    answered = False
    try:
        with urllib.request.urlopen("http://%s/cm?cmnd=Status%%2010" % host, timeout=timeout) as r:
            answered = True
            energy = json.loads(r.read())["StatusSNS"]["ENERGY"]
        return True, energy.get("Power"), energy.get("Voltage")
    except urllib.error.HTTPError:
        return True, None, None
    except (OSError,) + UNREADABLE:
        return answered, None, None


def socket_power(host, timeout=0.8):
    """(watts, volts) of the Tasmota socket at HOST, or (None, None) when it does not answer or answers what cannot be read."""
    return socket_reach(host, timeout)[1:]


class Recorder:
    """Holds one stream of the board at HOST and writes a row a second into OUT, a file a day and a dictionary."""

    def __init__(self, host, tasmota=None, out="."):
        self.host = host
        self.tasmota = tasmota
        self.out = pathlib.Path(out).expanduser()
        self.lock = threading.Lock()
        self.dictionary = None
        self.values = {}
        # when each value was heard, by this client's clock: a count of seconds is counted on from it
        self.told = {}
        self.heard_at = 0.0
        self.born = time.time()
        self.socket_at = 0.0
        # when the board last answered anything over the network, a refusal or an unreadable answer too
        self.reached_at = 0.0
        self.why = "not yet heard"
        self.power = (None, None)
        self.stopping = threading.Event()
        self.threads = []
        # the day of the last row, and the thread packing the days gone
        self.day = None
        self.packing = None

    # ---- the board ----

    def start(self):
        self.threads = [threading.Thread(target=self._listen, daemon=True)]
        if self.tasmota:
            self.threads.append(threading.Thread(target=self._socket, daemon=True))
        for t in self.threads:
            t.start()

    def stop(self):
        self.stopping.set()

    def hears(self):
        """Whether the board has been heard within SILENT_S."""
        with self.lock:
            return self.dictionary is not None and time.time() - self.heard_at < SILENT_S

    def last_heard(self):
        """(board, socket): when each last answered over the network, whatever it said, or when the process began; without a socket it is never heard, so the board alone decides."""
        with self.lock:
            return max(self.heard_at, self.reached_at, self.born), max(self.socket_at, self.born) if self.tasmota else 0.0

    def _listen(self):
        """The dictionary, then the stream, for as long as the recorder runs; again after every break."""
        while not self.stopping.is_set():
            try:
                board = Board(self.host, timeout=5)
                dictionary = board.dictionary()
                if not readable(dictionary):
                    raise Refused("the board's dictionary cannot be read")
                with self.lock:
                    if self.dictionary is None or self.dictionary["version"] != dictionary["version"]:
                        self.dictionary = dictionary
                    self.values = {}
                    self.told = {}
                board.stream(3600 * 24, self._heard, stop=self.stopping, every=ASK_EVERY_S)
                with self.lock:
                    self.why = "the stream ended"
            except Refused as e:
                with self.lock:
                    self.why = str(e)
                    self.reached_at = time.time()
            except OSError as e:
                with self.lock:
                    self.why = e.__class__.__name__ + (": %s" % e if str(e) else "")
                    if isinstance(e, urllib.error.HTTPError):
                        self.reached_at = time.time()
            except UNREADABLE as e:
                # an answer that cannot be read is a break like a stream that ends: the board is asked again after RETRY_S, and the rows say why meanwhile
                with self.lock:
                    self.reached_at = time.time()
                    self.why = "unreadable answer: " + e.__class__.__name__ + (": %s" % e if str(e) else "")
            self.stopping.wait(RETRY_S)

    def _heard(self, at, _size, records):
        # a beat that carries a pair the dictionary has no address for, or a value of no form the board speaks, is not the board saying its values: it is refused WHOLE, before the board counts as heard and before any of its pairs is kept
        with self.lock:
            size = len(self.dictionary["things"]) if self.dictionary else 0
        records = records or []
        if not isinstance(records, list) or not all(pair_ok(pair, size) for pair in records):
            raise ValueError("a beat of the stream with a pair that is no address and value of this dictionary")
        pairs = [(address, value) for address, value in records]
        with self.lock:
            self.heard_at = at
            for address, value in pairs:
                self.values[address] = value
                self.told[address] = at

    def _pack(self, day):
        """Packs the days gone, and tries again every PACK_RETRY_S for the files that changed too lately to be touched, until none is left or the recorder stops."""
        while not self.stopping.is_set():
            _, waiting = pack_pass(self.out, day)
            if not waiting or self.stopping.wait(PACK_RETRY_S):
                return

    def _socket(self):
        while not self.stopping.is_set():
            answered, watts, volts = socket_reach(self.tasmota)
            with self.lock:
                self.power = (watts, volts)
                if answered:
                    self.socket_at = time.time()
            self.stopping.wait(1)

    def _counted(self, dictionary, now):
        """The values heard, a count of seconds - what the dictionary says goes on by itself - as it is at NOW: the board says it when it jumps, and a row is written every second."""
        values = dict(self.values)
        for at, thing in enumerate(dictionary["things"]):
            way, value = thing.get("counts"), values.get(at)
            if way and isinstance(value, int) and not isinstance(value, bool) and at in self.told:
                values[at] = going_on(way, value, self.told[at], now)
        return values

    # ---- the file ----

    def tick(self, now):
        """Writes the row of the second NOW."""
        with self.lock:
            dictionary = self.dictionary
            heard = dictionary is not None and now - self.heard_at < SILENT_S
            values = self._counted(dictionary, now) if heard else {}
            why = self.why
            watts, volts = self.power
        stamp = datetime.datetime.fromtimestamp(now)
        day = stamp.strftime("%Y-%m-%d")
        if day != self.day:
            self.day = day
            self.packing = threading.Thread(target=self._pack, args=(day,), daemon=True)
            self.packing.start()
        named = columns(dictionary) if dictionary else []
        version = dictionary["version"] if dictionary else "none"
        path = self.out / ("%s_%s_%s.csv" % (stamp.strftime("%Y-%m-%d"), self.host.replace(":", "-"), version.replace("+", "_")))
        head = ["time", "board", "socket_w", "socket_v"] + named
        row = [stamp.strftime("%H:%M:%S"), OK if heard else "no answer: %s" % why, cell(watts), cell(volts)]
        row += [cell(values.get(at)) for at in range(len(named))]
        self.out.mkdir(parents=True, exist_ok=True)
        new = not path.exists()
        with open(path, "a", newline="") as f:
            w = csv.writer(f)
            if new:
                w.writerow(head)
            w.writerow(row)
        return path


def parser():
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("host", help="the board, host[:port]")
    p.add_argument("--tasmota", help="the Tasmota socket the burner is plugged into, read only")
    p.add_argument("--out", default="runs", help="where the CSV of the day goes (default: runs/ in the current folder)")
    return p


def main():
    args = parser().parse_args()
    recorder = Recorder(args.host, args.tasmota, args.out)
    recorder.start()
    print("recording %s into %s, a row a second - Ctrl-C stops" % (args.host, recorder.out), flush=True)
    due = int(time.time()) + 1
    try:
        while True:
            time.sleep(max(0.0, due - time.time()))
            path = recorder.tick(due)
            if due % 60 == 0:
                print("%s  %s" % (time.strftime("%H:%M:%S"), path.name), flush=True)
            due += 1
            if restart_due(*recorder.last_heard(), time.time()):
                print("%s restarting itself: board and socket silent for %d s (%s)" % (time.strftime("%H:%M:%S"), RESTART_S, recorder.why), flush=True)
                os.execv(sys.executable, [sys.executable] + sys.argv)
    except KeyboardInterrupt:
        recorder.stop()


if __name__ == "__main__":
    sys.exit(main())
