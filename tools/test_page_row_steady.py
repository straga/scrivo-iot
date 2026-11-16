"""A row of a card keeps its height whatever number it says: the value stands on one line, digits of one width. A value that does not fit beside its label stands whole on a line of its own under it, and nothing
is cut - so a countdown does not move the card under a finger once a second, and a long value loses none of its parts.

    uv run --no-project --with pytest --with websocket-client pytest tools/test_page_row_steady.py

What is held is what a browser draws: the stand (tools/page_stand.py) serves the real page files, a headless Chrome opens the Modules tab and the
test puts values into a row's value cell the way the board's stream does, at the widths of phones. Needs Chrome (`CHROME_BIN`, or Chrome of macOS):
without it the tests are skipped, which is a run that did not happen, not a green one.
"""
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

import pytest

pytest.importorskip("websocket")

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

from chrome_page import CHROME, Page  # noqa: E402

pytestmark = pytest.mark.skipif(not os.path.exists(CHROME), reason="no Chrome to draw the page")

# a clock the way the page writes a pause: the narrow digits (1) and the wide ones (0, 4) of one length
NARROW, WIDE = "можно разжечь через 1:11:11", "можно разжечь через 1:00:00"
# widths from a small phone to a large one, in the step the wrap used to flip in
WIDTHS = list(range(340, 521, 2))

ROW = ("(function(){for (const r of document.querySelectorAll('section > div')) {"
       " const s = r.querySelectorAll(':scope > span'); if (s.length > 1) return r } })()")


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture(scope="module")
def page():
    port, cdp = free_port(), free_port()
    stand = subprocess.Popen([sys.executable, os.path.join(HERE, "page_stand.py"), "--port", str(port)],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    chrome = subprocess.Popen([CHROME, "--headless=new", "--remote-debugging-port=%d" % cdp, "--user-data-dir=" + tempfile.mkdtemp(),
                               "--no-first-run", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        targets = None
        for _ in range(60):
            try:
                targets = json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % cdp))
                urllib.request.urlopen("http://127.0.0.1:%d/state" % port)
                break
            except OSError:
                time.sleep(0.25)
        assert targets, "Chrome did not come up"
        p = Page(next(t["webSocketDebuggerUrl"] for t in targets if t["type"] == "page"))
        p.call("Page.enable")
        p.call("Page.navigate", url="http://127.0.0.1:%d/" % port)
        p.settle(2.5)
        p.js("(function(){for (const b of document.querySelectorAll('nav button')) if (b.textContent.trim() === 'Модули') b.click()})()")
        p.settle(0.5)
        # the first module on the list: any card with rows will do, the row is the thing held
        p.js("(function(){for (const b of document.querySelectorAll('button')) if (!b.closest('nav') && b.textContent.trim() === 'Котёл') b.click()})()")
        p.settle(0.8)
        yield p
    finally:
        chrome.terminate()
        stand.terminate()


def height_with(page, width, text):
    page.call("Emulation.setDeviceMetricsOverride", width=width, height=900, deviceScaleFactor=1, mobile=True)
    page.settle(0.1)
    return page.js("(function(){const r = %s; r.querySelectorAll(':scope > span')[1].textContent = %s; return r.getBoundingClientRect().height})()"
                   % (ROW, json.dumps(text)))


def test_the_row_is_as_high_for_a_narrow_clock_as_for_a_wide_one_at_every_phone_width(page):
    moved = [(w, height_with(page, w, NARROW), height_with(page, w, WIDE)) for w in WIDTHS]
    moved = [m for m in moved if m[1] != m[2]]
    assert not moved, "a clock of narrow digits and one of wide digits make rows of different height at widths (width, narrow, wide): %s" % moved[:5]


def test_digits_are_of_one_width_so_a_clock_does_not_breathe(page):
    page.call("Emulation.setDeviceMetricsOverride", width=375, height=900, deviceScaleFactor=1, mobile=True)
    widths = []
    for text in ("1:11:11", "0:00:00"):
        widths.append(page.js("(function(){const c = %s.querySelectorAll(':scope > span')[1]; c.textContent = %s;"
                              " const r = document.createRange(); r.selectNodeContents(c); return r.getBoundingClientRect().width})()"
                              % (ROW, json.dumps(text))))
    assert widths[0] == widths[1], "'1:11:11' is %s px and '0:00:00' is %s px" % tuple(widths)


def fit_of(page, width, text):
    """How a row of the card draws TEXT at WIDTH: its height, whether the value is cut, and whether it stands under the label (below its bottom edge) or beside it."""
    page.call("Emulation.setDeviceMetricsOverride", width=width, height=900, deviceScaleFactor=1, mobile=True)
    page.settle(0.1)
    return page.js("(function(){const r = %s; const [label, value] = r.querySelectorAll(':scope > span'); value.textContent = %s;"
                   " return {height: r.getBoundingClientRect().height, cut: value.scrollWidth > value.clientWidth,"
                   " under: value.getBoundingClientRect().top >= label.getBoundingClientRect().bottom - 1}})()" % (ROW, json.dumps(text)))


# a value of a meter of the bench: it fits a line of the card, and does not fit beside a label
LONG_FITS = "L1 0.012 \u00b7 L2 0.110 \u00b7 0.122 \u043a\u0412\u0442\u00b7\u0447"
LONG_FITS_OTHER = "L1 1.313 \u00b7 L2 0.503 \u00b7 1.816 \u043a\u0412\u0442\u00b7\u0447"


def test_a_value_that_does_not_fit_beside_the_label_stands_whole_on_a_line_of_its_own(page):
    one_line = fit_of(page, 375, "1:11:11")
    assert not one_line["under"], "a short value stands beside its label"
    seen = fit_of(page, 375, LONG_FITS)
    assert seen["under"] and not seen["cut"], seen
    assert seen["height"] > one_line["height"], "the row has the line of the value besides the line of the label"


def test_the_row_of_a_long_value_is_as_high_whatever_its_digits(page):
    # the pair of the one above: the height of a row that has gone to two lines does not follow the digits either
    heights = {(w, fit_of(page, w, LONG_FITS)["height"], fit_of(page, w, LONG_FITS_OTHER)["height"]) for w in (340, 375, 414)}
    assert all(a == b for _, a, b in heights), heights


def test_a_value_longer_than_a_line_is_cut_on_its_line_and_the_page_does_not_grow_sideways(page):
    # bad data: what cannot stand even alone on a line is cut at the edge of the card, never pushes the page wide
    long = "значение настолько длинное, что ни в какую строку карточки на телефоне оно не поместится никак"
    seen = fit_of(page, 375, long)
    assert seen["cut"] and seen["under"], seen
    assert page.js("document.documentElement.scrollWidth") <= 375
