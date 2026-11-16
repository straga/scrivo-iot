"""tools/chrome_page.py, the Chrome driving of the page tools: `Page` over a stand-in socket, `CHROME` from the
environment, and every page tool imported with no scrivo-safe anywhere.

    uv run --no-project --with pytest --with websocket-client pytest tools/test_chrome_page.py
"""
import base64
import glob
import json
import os
import subprocess
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

pytest.importorskip("websocket")
import chrome_page  # noqa: E402


class Socket:
    """A DevTools socket: answers each command with the result `answers` gives for its method, after the events."""

    def __init__(self, answers, events=()):
        self.answers = answers
        self.queue = [json.dumps(e) for e in events]
        self.sent = []

    def send(self, text):
        sent = json.loads(text)
        self.sent.append(sent)
        reply = self.answers[sent["method"]]
        self.queue.append(json.dumps({"id": sent["id"], **reply}))

    def recv(self):
        return self.queue.pop(0)


def page_over(monkeypatch, socket):
    monkeypatch.setattr(chrome_page.websocket, "create_connection", lambda *a, **k: socket)
    return chrome_page.Page("ws://x")


def test_js_returns_the_value_the_page_computed(monkeypatch):
    socket = Socket({"Runtime.evaluate": {"result": {"result": {"value": 42}}}})
    assert page_over(monkeypatch, socket).js("6*7") == 42
    assert socket.sent[0]["params"]["expression"] == "6*7"
    assert socket.sent[0]["params"]["returnByValue"] is True


def test_requests_the_page_made_before_the_answer_are_kept(monkeypatch):
    event = {"method": "Network.requestWillBeSent", "params": {"request": {"url": "http://board/state"}}}
    socket = Socket({"Runtime.evaluate": {"result": {"result": {"value": 1}}}}, events=[event])
    page = page_over(monkeypatch, socket)
    page.js("1")
    assert page.requests == ["http://board/state"]


def test_an_error_answer_is_raised_naming_the_method(monkeypatch):
    socket = Socket({"Page.navigate": {"error": {"message": "no such page"}}})
    with pytest.raises(RuntimeError, match="Page.navigate.*no such page"):
        page_over(monkeypatch, socket).call("Page.navigate", url="http://x")


def test_shot_writes_the_picture_the_browser_sent(monkeypatch, tmp_path):
    png = b"\x89PNG-not-really"
    socket = Socket({"Page.captureScreenshot": {"result": {"data": base64.b64encode(png).decode()}}})
    out = tmp_path / "a.png"
    page_over(monkeypatch, socket).shot(str(out))
    assert out.read_bytes() == png


def run_python(code, **env):
    clean = {"PATH": os.environ["PATH"], "HOME": "/nonexistent"}
    clean.update(env)
    return subprocess.run([sys.executable, "-I", "-c", code], capture_output=True, text=True, env=clean, cwd=HERE)


def test_chrome_is_the_browser_the_environment_names():
    said = run_python("import sys; sys.path.insert(0, '.'); import chrome_page; print(chrome_page.CHROME)", CHROME_BIN="/opt/chrome/bin/chrome")
    assert said.stdout.strip() == "/opt/chrome/bin/chrome", said.stderr


def test_chrome_is_the_chrome_of_macos_when_the_environment_names_none():
    said = run_python("import sys; sys.path.insert(0, '.'); import chrome_page; print(chrome_page.CHROME)")
    assert said.stdout.strip() == "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", said.stderr


TOOLS = sorted(os.path.basename(p)[:-3] for p in glob.glob(os.path.join(HERE, "page_*.py"))
               if "chrome_page" in open(p, encoding="utf-8").read())


def test_the_tools_that_drive_chrome_are_found():
    assert TOOLS, "no page tool names chrome_page: the import rows below would run on nothing"


@pytest.mark.parametrize("tool", TOOLS)
def test_a_page_tool_imports_with_no_scrivo_safe_anywhere(tool):
    code = "import sys; sys.path.insert(0, '.'); import %s" % tool
    said = run_python(code, SCRIVO_SAFE="/nonexistent/scrivo-safe")
    assert said.returncode == 0, said.stderr[-600:]
