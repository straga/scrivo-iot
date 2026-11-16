"""The page does not eat the tab: the memory of an open page, measured on the stand, once a minute.

    uv run --no-project --with websocket-client python tools/page_heap.py [--minutes 10] [--scenario steady]
                                                                          [--pace 0.55] [--out FILE]
    uv run --no-project --with websocket-client python tools/page_heap.py --board <board-ip> --minutes 30

Raises tools/page_stand.py on a scenario played round again at the burner's own pace (or, with --board, points the page
at a real board: reads only, nothing is pressed), opens the page in headless Chrome, leaves it alone, and once a minute
takes the JS heap twice: as it stands, and after the browser has collected garbage. Growth that survives the collection
is what a page holds on to; growth without it is what a page has not yet let go of, and the two look the same until
the collection is asked for.

The verdict is one line, by what the collected heap did from the first minute on:

  pass          it stayed within LIMIT_MB of minute 1 at every minute after it
  fail          it went above, and the minute and the size say where
  not measured  fewer minutes than asked, or the stream brought nothing - nothing is said of the heap

With --snapshots 1,4 --snapshot-dir DIR it also writes the heap at those minutes, after a collection, for tools/heap_path.py.

Exits 0 on pass, 1 on fail, 2 on not measured - also when the ports it needs are held by another run, which it neither joins nor stops.
Stops the stand and Chrome whatever happens.
"""
import argparse
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

CHROME_PORT = 9416
STAND_PORT = 8798
# how far the collected heap may go above its size at minute 1
LIMIT_MB = 1.0
# the events of the stream a minute must bring for the measure to mean anything: the stand plays about 1.8 a second
EVENTS_PER_MINUTE = 30

# Counts what the stream brings, without a line of the page's own: it wraps EventSource before the page loads.
COUNT_EVENTS = """(function () {
  window.__events = 0;
  const Real = window.EventSource;
  window.EventSource = function (url, init) {
    const source = new Real(url, init);
    source.addEventListener('message', () => { window.__events += 1; });
    return source;
  };
  window.EventSource.prototype = Real.prototype;
})();"""


def taken(ports):
    """The ones of PORTS that something listens on this machine, in order. Asked by connecting: a port whose last connection is still winding down
    (the run before this one, a minute ago) refuses a bind and is free all the same."""
    held = []
    for port in ports:
        probe = socket.socket()
        probe.settimeout(1.0)
        try:
            if probe.connect_ex(("127.0.0.1", port)) == 0:
                held.append(port)
        finally:
            probe.close()
    return held


def judge(samples, minutes, limit_mb=LIMIT_MB):
    """The verdict on SAMPLES, [{minute, raw_mb, gc_mb, events}], one for minute 0 (the load) and one for each minute
    after it, MINUTES of them asked for: (verdict, words), the verdict pass, fail or not_measured."""
    by = {s["minute"]: s for s in samples}
    if any(m not in by for m in range(minutes + 1)):
        got = sorted(by)
        return "not_measured", "asked for %d minutes, got minutes %s" % (minutes, got)
    still = [m for m in range(1, minutes + 1) if by[m]["events"] - by[m - 1]["events"] < EVENTS_PER_MINUTE]
    if still:
        return "not_measured", "the stream brought fewer than %d events in minute %s" % (EVENTS_PER_MINUTE, still[0])
    first = by[1]["gc_mb"]
    over = [m for m in range(2, minutes + 1) if by[m]["gc_mb"] - first > limit_mb]
    rate = (by[minutes]["events"] - by[0]["events"]) / (60.0 * minutes)
    if over:
        m = over[0]
        return "fail", "after collection the heap is %.1f MB at minute %d against %.1f MB at minute 1 (+%.1f, limit %.1f); %.1f events a second" % (
            by[m]["gc_mb"], m, first, by[m]["gc_mb"] - first, limit_mb, rate)
    peak = max(by[m]["gc_mb"] for m in range(1, minutes + 1))
    return "pass", "after collection the heap stays within %.1f MB of minute 1 (%.1f MB, peak %.1f) over %d minutes; %.1f events a second" % (
        limit_mb, first, peak, minutes, rate)


def table(samples):
    """The samples as the table a report carries: minute, the heap as it stands, after collection, events so far."""
    rows = ["minute  heap MB  collected MB  events"]
    rows += ["%6d  %7.2f  %12.2f  %6d" % (s["minute"], s["raw_mb"], s["gc_mb"], s["events"]) for s in samples]
    return "\n".join(rows)


def collecting(page_class):
    """PAGE_CLASS that also keeps the chunks of a heap snapshot, which the browser sends as events before it answers."""

    class Collecting(page_class):
        chunks = []

        def call(self, method, **params):
            self.next_id += 1
            self.ws.send(json.dumps({"id": self.next_id, "method": method, "params": params}))
            while True:
                msg = json.loads(self.ws.recv())
                if msg.get("method") == "HeapProfiler.addHeapSnapshotChunk":
                    self.chunks.append(msg["params"]["chunk"])
                if msg.get("id") == self.next_id:
                    if "error" in msg:
                        raise RuntimeError(f"{method}: {msg['error']}")
                    return msg.get("result", {})

    return Collecting


def snapshot(p, path):
    """The heap of the page P written to PATH, after a collection, so what is in it is what the page holds."""
    p.call("HeapProfiler.collectGarbage")
    p.chunks = []
    p.ws.settimeout(600)
    try:
        p.call("HeapProfiler.takeHeapSnapshot", reportProgress=False)
    finally:
        p.ws.settimeout(30)
    with open(path, "w") as f:
        f.write("".join(p.chunks))
    p.chunks = []
    print("snapshot %s, %.1f MB" % (path, os.path.getsize(path) / 1048576.0), flush=True)


def sample(p, minute):
    """One sample of the page P: the heap as it stands, then after the browser collected garbage."""
    raw = p.call("Runtime.getHeapUsage")["usedSize"]
    p.call("HeapProfiler.collectGarbage")
    collected = p.call("Runtime.getHeapUsage")["usedSize"]
    return {"minute": minute, "raw_mb": raw / 1048576.0, "gc_mb": collected / 1048576.0,
            "events": int(p.js("window.__events") or 0)}


def measure(url, minutes, chrome_binary, page_class, settle=2.0, snapshots=(), snapshot_dir=None):
    """Opens URL in headless Chrome and samples it for MINUTES minutes: [sample]. Stops Chrome whatever happens."""
    profile = tempfile.mkdtemp(prefix="page_heap_")
    chrome = subprocess.Popen([chrome_binary, "--headless=new", f"--remote-debugging-port={CHROME_PORT}",
                               f"--user-data-dir={profile}", "--no-first-run", "about:blank"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        targets = None
        for _ in range(75):
            try:
                targets = json.load(urllib.request.urlopen(f"http://127.0.0.1:{CHROME_PORT}/json"))
                break
            except OSError:
                time.sleep(0.2)
        if targets is None:
            raise SystemExit("page_heap: Chrome did not come up")
        p = collecting(page_class)(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Runtime.enable")
        p.call("HeapProfiler.enable")
        p.call("Emulation.setDeviceMetricsOverride", width=375, height=667, deviceScaleFactor=2, mobile=True)
        p.call("Page.addScriptToEvaluateOnNewDocument", source=COUNT_EVENTS)
        p.call("Page.navigate", url=url)
        p.settle(settle)
        samples = [sample(p, 0)]
        began = time.time()
        for minute in range(1, minutes + 1):
            # the clock, not a count of sleeps: a sample takes its own time, and a minute is a minute
            time.sleep(max(0.0, began + 60.0 * minute - time.time()))
            samples.append(sample(p, minute))
            print("minute %d: %.2f MB, %.2f MB collected, %d events" % (
                minute, samples[-1]["raw_mb"], samples[-1]["gc_mb"], samples[-1]["events"]), flush=True)
            if minute in snapshots:
                snapshot(p, os.path.join(snapshot_dir, "heap_minute_%d.heapsnapshot" % minute))
        return samples
    finally:
        chrome.terminate()
        try:
            chrome.wait(timeout=10)
        except subprocess.TimeoutExpired:
            chrome.kill()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--minutes", type=int, default=10)
    ap.add_argument("--scenario", default="steady", help="a scenario of tools/stand_scenarios, played round again")
    ap.add_argument("--pace", type=float, default=0.55,
                    help="seconds between the stand's frames: 0.55 is the burner's own 1.8 events a second")
    ap.add_argument("--page", default=None, help="the page files to serve instead of the device's own (a revision out of git)")
    ap.add_argument("--board", default=None, help="a real board's address: the page is pointed at it, nothing is pressed")
    ap.add_argument("--out", default=None, help="where the samples are written, as JSON")
    ap.add_argument("--snapshots", default="", help="minutes to take a heap snapshot at, comma separated")
    ap.add_argument("--snapshot-dir", default=".", help="where the snapshots are written")
    args = ap.parse_args()

    # A run that finds its ports held is not a second run: Chrome would attach to the first one's, the page would be loaded twice and the events
    # counted twice, and the numbers would be neither's.
    busy = taken([CHROME_PORT, STAND_PORT])
    if busy:
        print("not measured: port %s is held by another run or process - wait for it, or stop it if it is a leftover of yours" % ", ".join(map(str, busy)))
        sys.exit(2)

    from chrome_page import CHROME, Page

    sys.path.insert(0, HERE)
    stand_args = [sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(STAND_PORT)]
    if args.page:
        stand_args += ["--page", args.page]
    if args.board:
        stand_args += ["--board", args.board]
        url = f"http://127.0.0.1:{STAND_PORT}/?board={args.board}"
    else:
        from stand_frames import frames_of

        scenario = os.path.join(HERE, "stand_scenarios", args.scenario + ".json")
        frames = tempfile.NamedTemporaryFile("w", suffix=".json", prefix="page_heap_frames_", delete=False)
        json.dump({"frames": frames_of(scenario, ROOT)}, frames)
        frames.close()
        stand_args += ["--scenario", frames.name, "--loop", "--pace", str(args.pace)]
        url = f"http://127.0.0.1:{STAND_PORT}/"
    stand = subprocess.Popen(stand_args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(75):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{STAND_PORT}/state" if not args.board else
                                       f"http://127.0.0.1:{STAND_PORT}/", timeout=1)
                break
            except OSError:
                time.sleep(0.2)
        samples = measure(url, args.minutes, CHROME, Page,
                          snapshots={int(m) for m in args.snapshots.split(",") if m}, snapshot_dir=args.snapshot_dir)
    finally:
        stand.terminate()
        try:
            stand.wait(timeout=10)
        except subprocess.TimeoutExpired:
            stand.kill()
    verdict, words = judge(samples, args.minutes)
    print(table(samples))
    print("%s: %s" % (verdict, words))
    if args.out:
        json.dump({"verdict": verdict, "words": words, "samples": samples}, open(args.out, "w"), indent=1)
    sys.exit({"pass": 0, "fail": 1, "not_measured": 2}[verdict])


if __name__ == "__main__":
    main()
