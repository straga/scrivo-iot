"""Tests for tools/hold_exits.py: the exits of a hold, set against the list of them.

The scan is run as a command, its copy standing in a tree made for the row (it finds its tree from where it lies); the HoldButton of the rows is the page's own class, taken from page/page.js and
altered in the row where the row needs a missing handler, an extra one, or a second place that holds. Judged by the exit code and by the lines printed.

Run: uv run --no-project python -m unittest tools/test_hold_exits.py
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCAN = os.path.join(HERE, "hold_exits.py")
PAGE = os.path.join(os.path.dirname(HERE), "page", "page.js")


def real_class():
    text = open(PAGE, encoding="utf-8").read()
    start = text.index("class HoldButton")
    end = text.index("\n}\n", start) + 3
    return text[start:end]


def scan(page_js, parts_js="", *flags):
    with tempfile.TemporaryDirectory() as root:
        for path, text in (("tools/hold_exits.py", open(SCAN, encoding="utf-8").read()), ("page/page.js", page_js), ("devices/x/page/parts.js", parts_js)):
            full = os.path.join(root, path)
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8") as f:
                f.write(text)
        run = subprocess.run([sys.executable, os.path.join(root, "tools", "hold_exits.py"), *flags], capture_output=True, text=True)
        return run.returncode, run.stdout


class ExitsOfAHold(unittest.TestCase):
    def test_the_page_s_own_holdbutton_has_an_event_for_every_exit_and_none_that_is_not_one(self):
        code, out = scan(real_class())
        self.assertEqual(code, 0, out)
        self.assertIn("0 findings", out)

    def test_a_holdbutton_that_does_not_hear_the_page_hidden_says_which_exit_has_nothing(self):
        page = real_class().replace('document[how]("visibilitychange", this.hidden);', "")
        code, out = scan(page)
        self.assertEqual(code, 1, out)
        self.assertIn("NO EXIT   the page is hidden", out)

    def test_a_holdbutton_with_an_event_that_no_exit_asks_for_says_which(self):
        page = real_class().replace('window[how]("pagehide", this.gone);', 'window[how]("pagehide", this.gone);\n        window[how]("scroll", this.gone);')
        code, out = scan(page)
        self.assertEqual(code, 1, out)
        self.assertIn("NO LINE   window scroll", out)

    def test_another_class_that_takes_a_hand_down_and_keeps_a_timer_is_named_as_a_second_holder(self):
        other = 'class Other extends Component {\n    static template = xml`<div t-on-pointerdown="down"/>`;\n    down() { setTimeout(() => 1, 1000); }\n}\n'
        code, out = scan(real_class(), other)
        self.assertEqual(code, 1, out)
        self.assertIn("HOLDER?", out)
        self.assertIn("class Other", out)

    def test_a_class_that_takes_a_hand_down_and_keeps_no_timer_is_no_holder(self):
        other = 'class Plain extends Component {\n    static template = xml`<div t-on-pointerdown="down"/>`;\n    down() { return 1; }\n}\n'
        code, out = scan(real_class(), other)
        self.assertEqual(code, 0, out)

    def test_what_a_hold_does_at_its_end_is_said_to_be_reached_by_the_hold_alone_and_a_second_caller_is_named(self):
        host = 'class Host extends Component {\n    static template = xml`<HoldButton onHold="() => this.go()"/>`;\n    go() { return 1; }\n}\n'
        code, out = scan(real_class(), host)
        self.assertEqual(code, 0, out)
        self.assertIn("class Host -> go: reached by the hold alone", out)
        host_with_other = host.replace("go() { return 1; }", "go() { return 1; }\n    again() { this.go(); }")
        code, out = scan(real_class(), host_with_other)
        self.assertEqual(code, 1, out)
        self.assertIn("ALSO CALLED  go of class Host", out)

    def test_a_page_with_no_holdbutton_stops_the_scan(self):
        code, out = scan("class Else extends Component {\n}\n")
        self.assertEqual(code, 1, out)
        self.assertIn("MISSING   class HoldButton", out)


if __name__ == "__main__":
    unittest.main()
