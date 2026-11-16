"""How many events a stream sends, counted by a reader.

    uv run --no-project python tools/stream_rate.py HOST[:PORT] [--every N | --live | --default] --seconds WALL [--second S]

Opens /events on a board or a stand asking the pace it is told (`--every N` seconds, `--live` every change, `--default` nothing: the pace the board keeps for everyone), reads for WALL seconds, and says in one line of
JSON what came: the events after the first (the first is the whole board, which every reader gets once), the records in them, the bytes, and the rates per BOARD second - the seconds of the board being SECOND wall seconds
each, as a stand that plays a recording faster than it happened keeps them (`tools/page_stand.py --pace`).

What it is for: the claim of 9.16 is a number - events per second a stream sends on a standing burner and on a working one - and it is counted by a reader of the stream, as a page would, and not read off the board's own
bookkeeping.
"""
import argparse
import http.client
import json
import socket
import time


def measure(host, query, wall, second=1.0):
    """Reads /events of HOST asking QUERY (`?every=0`, or nothing) for WALL seconds. Answers {events, records, bytes, board_seconds, events_per_s, records_per_s}: the events after the first, counted a board second at
    a time - a board second being SECOND wall seconds. A beat is no event."""
    name, _, port = host.partition(":")
    conn = http.client.HTTPConnection(name, int(port or 80), timeout=8)
    conn.request("GET", "/events" + query)
    resp = conn.getresponse()
    if resp.status != 200:
        raise SystemExit("the stream answered %d: %s" % (resp.status, resp.read().decode(errors="replace")))
    events = records = size = 0
    first = True
    began, until, buf = time.time(), time.time() + wall, b""
    try:
        while time.time() < until:
            try:
                chunk = resp.read1(4096)
            except (socket.timeout, TimeoutError):
                continue
            if not chunk:
                break
            buf += chunk
            while b"\n\n" in buf:
                one, buf = buf.split(b"\n\n", 1)
                for line in one.split(b"\n"):
                    if not line.startswith(b"data: "):
                        continue
                    if first:
                        first = False
                        continue
                    events += 1
                    records += len(json.loads(line[6:]))
                    size += len(line) + 2
    finally:
        conn.close()
    board_seconds = (time.time() - began) / second
    return {"events": events, "records": records, "bytes": size, "board_seconds": round(board_seconds, 1),
            "events_per_s": round(events / board_seconds, 3), "records_per_s": round(records / board_seconds, 3)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("host")
    pace = ap.add_mutually_exclusive_group()
    pace.add_argument("--every", type=int)
    pace.add_argument("--live", action="store_true")
    pace.add_argument("--default", action="store_true")
    ap.add_argument("--seconds", type=float, required=True, help="wall seconds to read for")
    ap.add_argument("--second", type=float, default=1.0, help="wall seconds in a second of the board (a stand played at --pace P: P)")
    args = ap.parse_args()
    query = "?every=0" if args.live else "?every=%d" % args.every if args.every is not None else ""
    print(json.dumps(measure(args.host, query, args.seconds, args.second)))


if __name__ == "__main__":
    main()
