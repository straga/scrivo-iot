"""Tests for tools/sleep_scan.py: the list of the waits of the firmware by a clock.

The scan is run the way a person runs it - as a command, its copy standing in a tree of its own (it finds its tree from where it lies) -
and judged by what is printed and by the exit code: which wait it names, under which verdict.

Run: uv run --no-project python -m unittest tools/test_sleep_scan.py
"""
import os
import re
import subprocess
import sys
import tempfile
import unittest

SCAN = os.path.join(os.path.dirname(__file__), "sleep_scan.py")


def scan(files, *flags):
    """Runs the scan on a tree made of FILES (path -> text); (exit code, output). The copy of the scan has its list of the harmless emptied: that list
    is of the places of the firmware, and a tree made for a row has none of them."""
    with tempfile.TemporaryDirectory() as root:
        os.makedirs(os.path.join(root, "tools"))
        text = open(SCAN, encoding="utf-8").read()
        emptied, n = re.subn(r"^HARMLESS = \{.*?^\}\n", "HARMLESS = {}\n", text, flags=re.S | re.M)
        assert n == 1, "the list of the harmless was not found in the scan"
        with open(os.path.join(root, "tools", "sleep_scan.py"), "w", encoding="utf-8") as f:
            f.write(emptied)
        for path, text in files.items():
            full = os.path.join(root, path)
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8") as f:
                f.write(text)
        run = subprocess.run([sys.executable, os.path.join(root, "tools", "sleep_scan.py"), *flags], capture_output=True, text=True)
        return run.returncode, run.stdout


class WaitsOfEmbassyTime(unittest.TestCase):
    def test_a_timer_and_a_deadline_of_embassy_time_are_named_as_covered_by_the_steady_driver(self):
        code, out = scan(
            {
                "crates/net/src/lib.rs": "async fn a() {\n    Timer::after(time(LET_GO)).await;\n    let _ = embassy_time::with_timeout(t, read()).await;\n}\n",
                "crates/between/src/desk.rs": "fn b() { with_deadline(until, answered).await }\n",
            },
            "--all",
        )
        self.assertEqual(code, 0, out)
        for site in ("crates/net/src/lib.rs:2", "crates/net/src/lib.rs:3", "crates/between/src/desk.rs:1"):
            self.assertIn(f"covered   {site}", out)
        self.assertIn("between::steady_time", out)
        self.assertIn("3 waits through embassy-time covered", out)

    def test_the_summary_counts_them_without_the_flag_too(self):
        code, out = scan({"crates/net/src/lib.rs": "async fn a() { Timer::at(next).await; }\n"})
        self.assertEqual(code, 0, out)
        self.assertIn("1 waits through embassy-time covered", out)
        self.assertNotIn("covered   crates", out)

    def test_a_wait_for_a_notification_with_no_deadline_or_a_look_is_covered_and_one_with_a_deadline_is_not_judged(self):
        code, out = scan({"crates/between/src/steps.rs": "fn a(n: N) { n.wait(0); n.wait_any(); }\n"}, "--all")
        self.assertEqual(code, 0, out)
        self.assertIn("covered   crates/between/src/steps.rs:1", out)
        code, out = scan({"crates/between/src/steps.rs": "fn a(n: N) { n.wait(500); }\n"})
        self.assertEqual(code, 1, out)
        self.assertIn("UNJUDGED  crates/between/src/steps.rs:1", out)

    def test_a_wait_in_a_comment_or_in_a_test_is_not_a_wait_of_the_firmware(self):
        code, out = scan(
            {
                "crates/net/src/lib.rs": "// Timer::after(x) is what this used to be\nfn a() {}\n",
                "crates/net/tests/net.rs": "fn t() { Timer::after(x); std::thread::sleep(y); }\n",
            }
        )
        self.assertEqual(code, 0, out)
        self.assertIn("0 waits through embassy-time covered", out)


class SleepsOfTheChipsClock(unittest.TestCase):
    def test_a_sleep_by_the_clock_of_the_chip_that_is_not_on_the_list_stops_the_scan(self):
        code, out = scan({"crates/net/src/lib.rs": "fn a() { std::thread::sleep(d); Timer::after(t); }\n"})
        self.assertEqual(code, 1, out)
        self.assertIn("UNJUDGED  crates/net/src/lib.rs:1", out)

    def test_a_tree_with_no_waits_passes(self):
        code, out = scan({"crates/net/src/lib.rs": "fn a() {}\n"})
        self.assertEqual(code, 0, out)


if __name__ == "__main__":
    unittest.main()
