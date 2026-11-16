"""Headless Chrome driven over the DevTools protocol: the browser to start (`CHROME`), the port it listens on (`PORT`)
and `Page`, one tab spoken to with `call`, `js`, `settle` and `shot`. The page tools of this folder open the device's page
with it.

Taken from scrivo-safe's tools/board/page_check.py (class Page, CHROME, PORT); from here on it is this tree's own.

CHROME is the browser the tools start: $CHROME_BIN when it is set, and otherwise Chrome of macOS. Needs the
`websocket-client` package: `uv run --no-project --with websocket-client python tools/<tool>.py`.
"""
import base64
import json
import os
import time

import websocket

CHROME = os.environ.get("CHROME_BIN") or "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
PORT = 9333


class Page:
    def __init__(self, ws_url):
        self.ws = websocket.create_connection(ws_url, suppress_origin=True, timeout=30)
        self.next_id = 0
        self.requests = []

    def call(self, method, **params):
        self.next_id += 1
        if os.environ.get("TRACE"):
            print("  cdp", method, {k: v for k, v in params.items() if k != "expression"} or params.get("expression", "")[:60], flush=True)
        self.ws.send(json.dumps({"id": self.next_id, "method": method, "params": params}))
        while True:
            msg = json.loads(self.ws.recv())
            if msg.get("method") == "Network.requestWillBeSent":
                self.requests.append(msg["params"]["request"]["url"])
            if msg.get("id") == self.next_id:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result", {})

    def js(self, expr):
        r = self.call("Runtime.evaluate", expression=expr, awaitPromise=True, returnByValue=True)
        return r.get("result", {}).get("value")

    def settle(self, seconds=2.5):
        end = time.time() + seconds
        while time.time() < end:
            self.js("1")
            time.sleep(0.2)

    def shot(self, path, full=False):
        r = self.call("Page.captureScreenshot", format="png", captureBeyondViewport=full)
        open(path, "wb").write(base64.b64decode(r["data"]))
        print(f"screen {os.path.basename(path)}")
