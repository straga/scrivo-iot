"""Tests for tools/upload_kit.py: a built kit goes onto the board through the recovery page file by file,
every file is read back and compared, and the board is made its mirror - what it holds beyond the kit is taken
off, a folder that is no built kit touches nothing, and nothing is taken off a board the kit did not reach whole.

Run: uv run --no-project python -m unittest tools/test_upload_kit.py
"""
import contextlib
import hashlib
import io
import json
import os
import sys
import tempfile
import unittest
import urllib.parse
from unittest import mock

sys.path.insert(0, os.path.dirname(__file__))
import upload_kit  # noqa: E402


class FakeBoard:
    """Answers the recovery page's file calls from a dict {board path: bytes}."""

    def __init__(self, files=None, refuse=(), corrupt=(), dirs=()):
        self.files = dict(files or {})
        self.refuse = set(refuse)
        self.corrupt = set(corrupt)
        # directories stay when their files go, as on the board's littlefs
        self.dirs = set(dirs) | {d for p in self.files for d in self._parents(p)}
        self.calls = []

    @staticmethod
    def _parents(path):
        parts = path.strip("/").split("/")[:-1]
        return ["/" + "/".join(parts[:i + 1]) for i in range(len(parts))]

    def request(self, method, path, body=None, headers=None, cookie=True):
        self.calls.append((method, path))
        route, _, query = path.partition("?")
        target = urllib.parse.parse_qs(query).get("path", [None])[0]
        if method == "POST" and route == "/file":
            if target in self.refuse:
                return 500, {}, b"write failed"
            self.files[target] = bytes(body)
            self.dirs.update(self._parents(target))
            return 200, {}, b"saved"
        if method == "POST" and route == "/file/delete":
            if target in self.files:
                del self.files[target]
                return 200, {}, b"Deleted.\n"
            if target in self.dirs:
                if any(p.startswith(target + "/") for p in list(self.files) + list(self.dirs)):
                    return 400, {}, b"the directory is not empty"
                self.dirs.discard(target)
                return 200, {}, b"Deleted.\n"
            return 404, {}, b"no such file"
        if method == "GET" and route == "/file":
            if target not in self.files:
                return 404, {}, b"no such file"
            data = self.files[target]
            return 200, {}, (data + b"!" if target in self.corrupt else data)
        if method == "GET" and route == "/files":
            entries = [{"path": d, "dir": True, "size": 0} for d in sorted(self.dirs)]
            entries += [{"path": p, "dir": False, "size": len(d)} for p, d in sorted(self.files.items())]
            return 200, {}, json.dumps({"entries": entries, "fs": {}}).encode()
        return 404, {}, b""


def make_kit(files):
    root = tempfile.mkdtemp()
    for board_path, data in files.items():
        path = os.path.join(root, board_path.lstrip("/"))
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)
    return root


def quiet(fn, *args):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = fn(*args)
    return code, out.getvalue(), err.getvalue()


KIT = {"/boot.py": b"boot\n", "/lib/a.py": b"A = 1\n", "/version.txt": b"dev-26.09.14.1\n"}


class Put(unittest.TestCase):
    def test_every_kit_file_lands_at_its_board_path_and_reads_back_equal(self):
        board = FakeBoard()
        code, out, err = quiet(upload_kit.put, board, make_kit(KIT))
        self.assertEqual(code, 0, err)
        self.assertEqual(board.files, KIT)
        for path, data in KIT.items():
            self.assertIn(path, out)
            self.assertIn(hashlib.sha256(data).hexdigest()[:16], out)

    def test_links_in_the_kit_upload_the_bytes_they_point_to(self):
        src = make_kit({"/real.py": b"REAL\n"})
        kit = make_kit({"/version.txt": KIT["/version.txt"]})
        os.symlink(os.path.join(src, "real.py"), os.path.join(kit, "main.py"))
        board = FakeBoard()
        self.assertEqual(quiet(upload_kit.put, board, kit)[0], 0)
        self.assertEqual(board.files, {"/main.py": b"REAL\n", "/version.txt": KIT["/version.txt"]})

    def test_a_refused_write_fails_the_upload_and_is_named(self):
        code, _, err = quiet(upload_kit.put, FakeBoard(refuse={"/lib/a.py"}), make_kit(KIT))
        self.assertNotEqual(code, 0)
        self.assertIn("/lib/a.py", err)

    def test_a_file_that_reads_back_different_fails_the_upload_and_is_named(self):
        code, _, err = quiet(upload_kit.put, FakeBoard(corrupt={"/boot.py"}), make_kit(KIT))
        self.assertNotEqual(code, 0)
        self.assertIn("/boot.py", err)

    def test_a_file_outside_the_kit_goes_off_the_board(self):
        board = FakeBoard(files={"/probe.txt": b"left by a probe\n", "/lib/boot_counter.py": b"C = 1\n"})
        code, out, err = quiet(upload_kit.put, board, make_kit(KIT))
        self.assertEqual(code, 0, err)
        self.assertEqual(board.files, KIT)
        self.assertIn("/probe.txt", out)
        self.assertIn("/lib/boot_counter.py", out)

    def test_switching_a_module_to_the_other_form_takes_the_old_form_off_the_board(self):
        # negative data: a file of an older form of the same name left behind is what a reader of the folder would take
        on_board = dict(KIT, **{"/lib/a.mpy": b"M\x06old"})
        del on_board["/lib/a.py"]
        board = FakeBoard(files=on_board)
        self.assertEqual(quiet(upload_kit.put, board, make_kit(KIT))[0], 0)
        self.assertNotIn("/lib/a.mpy", board.files)
        self.assertEqual(board.files["/lib/a.py"], KIT["/lib/a.py"])

    def test_a_directory_left_empty_goes_off_the_board_and_the_kit_ones_stay(self):
        board = FakeBoard(files={"/old/x.py": b"X\n"}, dirs={"/empty"})
        self.assertEqual(quiet(upload_kit.put, board, make_kit(KIT))[0], 0)
        self.assertEqual(board.dirs, {"/lib"})

    def test_a_kit_without_its_build_stamp_touches_nothing_on_the_board(self):
        # negative data: a path to the wrong folder must not become a mirror that wipes the board
        board = FakeBoard(files={"/main.py": b"main\n", "/lib/b.mpy": b"M\x06"})
        kit = dict(KIT)
        del kit["/version.txt"]
        code, _, err = quiet(upload_kit.put, board, make_kit(kit))
        self.assertNotEqual(code, 0)
        self.assertIn("version.txt", err)
        self.assertEqual([c for c in board.calls if c[0] != "GET"], [])
        self.assertEqual(board.files, {"/main.py": b"main\n", "/lib/b.mpy": b"M\x06"})

    def test_a_kit_file_that_fails_to_land_leaves_everything_else_on_the_board(self):
        # negative data: nothing is taken off a board the kit did not reach whole
        board = FakeBoard(files={"/probe.txt": b"p\n"}, refuse={"/lib/a.py"})
        code, _, err = quiet(upload_kit.put, board, make_kit(KIT))
        self.assertNotEqual(code, 0)
        self.assertIn("/lib/a.py", err)
        self.assertEqual(board.files["/probe.txt"], b"p\n")
        self.assertEqual([c for c in board.calls if c[1].startswith("/file/delete")], [])

    def test_an_empty_or_missing_kit_uploads_nothing(self):
        for kit in (tempfile.mkdtemp(), os.path.join(tempfile.mkdtemp(), "absent")):
            board = FakeBoard()
            code, _, err = quiet(upload_kit.put, board, kit)
            self.assertNotEqual(code, 0)
            self.assertIn("kit", err)
            self.assertEqual(board.files, {})


DATA = {"/data/energy/state": b"energy 1\n", "/data/energy/2026-09.csv": b"start,l1_wh,l2_wh,sum_wh\n"}


class PutLeavesTheBoardsData(unittest.TestCase):
    """What the board makes itself lies in /data and has no copy on the computer: a mirror of the kit leaves it."""

    def test_the_data_stays_and_a_stray_file_beyond_it_goes(self):
        board = FakeBoard(files=dict(DATA, **{"/probe.txt": b"left by a probe\n"}))
        code, out, err = quiet(upload_kit.put, board, make_kit(KIT))
        self.assertEqual(code, 0, err)
        self.assertEqual(board.files, dict(KIT, **DATA))
        self.assertIn("/probe.txt", out)
        self.assertTrue({"/data", "/data/energy"} <= board.dirs, board.dirs)

    def test_a_kit_file_outside_the_data_is_still_written_over(self):
        # negative data: the rule for /data is no reason to leave an old kit file standing
        board = FakeBoard(files=dict(DATA, **{"/boot.py": b"old boot\n"}))
        self.assertEqual(quiet(upload_kit.put, board, make_kit(KIT))[0], 0)
        self.assertEqual(board.files["/boot.py"], KIT["/boot.py"])

    def test_a_kit_that_carries_data_touches_nothing_on_the_board(self):
        # negative data: a kit may not write over what the board counted
        board = FakeBoard(files=dict(DATA))
        code, _, err = quiet(upload_kit.put, board, make_kit(dict(KIT, **{"/data/energy/state": b"energy 1\nzero\n"})))
        self.assertNotEqual(code, 0)
        self.assertIn("/data", err)
        self.assertEqual([c for c in board.calls if c[0] != "GET"], [])
        self.assertEqual(board.files, DATA)

    def test_verify_does_not_count_the_data_as_beyond_the_kit(self):
        board = FakeBoard(files=dict(KIT, **DATA))
        code, _, err = quiet(upload_kit.verify, board, make_kit(KIT))
        self.assertEqual(code, 0, err)


class Verify(unittest.TestCase):
    def test_board_holding_the_kit_byte_for_byte_matches(self):
        code, out, err = quiet(upload_kit.verify, FakeBoard(files=KIT), make_kit(KIT))
        self.assertEqual(code, 0, err)
        self.assertIn("3 files", out)

    def test_a_file_with_other_bytes_on_the_board_is_named(self):
        files = dict(KIT, **{"/lib/a.py": b"A = 2\n"})
        code, _, err = quiet(upload_kit.verify, FakeBoard(files=files), make_kit(KIT))
        self.assertNotEqual(code, 0)
        self.assertIn("/lib/a.py", err)

    def test_a_kit_file_missing_on_the_board_is_named(self):
        files = dict(KIT)
        del files["/version.txt"]
        code, _, err = quiet(upload_kit.verify, FakeBoard(files=files), make_kit(KIT))
        self.assertNotEqual(code, 0)
        self.assertIn("/version.txt", err)

    def test_a_folder_outside_the_kit_fails_verify(self):
        # negative data: an empty /lib/a folder is found before /lib/a.mpy when a module is imported
        board = FakeBoard(files=KIT, dirs={"/lib/a"})
        code, _, err = quiet(upload_kit.verify, board, make_kit(KIT))
        self.assertNotEqual(code, 0)
        self.assertIn("/lib/a", err)

    def test_verify_writes_nothing_and_a_file_outside_the_kit_fails_it(self):
        board = FakeBoard(files=dict(KIT, **{"/probe.txt": b"x\n"}))
        code, _, err = quiet(upload_kit.verify, board, make_kit(KIT))
        self.assertNotEqual(code, 0)
        self.assertIn("/probe.txt", err)
        self.assertEqual([c for c in board.calls if c[0] != "GET"], [])


class Command(unittest.TestCase):
    """The command line a person types reaches the same functions the tests above call."""

    def run_main(self, argv, board, host="192.0.2.5"):
        return quiet(lambda: upload_kit.main(argv + ["--board", host], connect=lambda host: board))

    def test_verify_put_and_fetch_are_reachable_from_the_command_line(self):
        kit = make_kit(KIT)
        self.assertEqual(self.run_main(["put", "--kit", kit], FakeBoard())[0], 0)
        code, out, err = self.run_main(["verify", "--kit", kit], FakeBoard(files=KIT))
        self.assertEqual(code, 0, err)
        self.assertIn("board holds the kit", out)
        code, _, err = self.run_main(["fetch", "--out", tempfile.mkdtemp()], FakeBoard(files=KIT))
        self.assertEqual(code, 0, err)

    def test_the_board_named_on_the_command_line_is_the_one_connected_to(self):
        asked = []
        quiet(lambda: upload_kit.main(["verify", "--kit", make_kit(KIT), "--board", "board.example"],
                                      connect=lambda host: asked.append(host) or FakeBoard(files=KIT)))
        self.assertEqual(asked, ["board.example"])

    def test_without_a_board_nothing_is_connected_to_and_the_usage_says_so(self):
        asked = []
        code, _, err = quiet(lambda: upload_kit.main(["verify", "--kit", make_kit(KIT)],
                                                     connect=lambda host: asked.append(host) or FakeBoard(files=KIT)))
        self.assertNotEqual(code, 0)
        self.assertEqual(asked, [])
        self.assertIn("--board", err)

    def test_seed_is_no_command_of_this_client(self):
        code, _, err = self.run_main(["seed", "--device", "dev"], FakeBoard())
        self.assertNotEqual(code, 0)
        self.assertIn("usage", err)

    def test_unknown_subcommand_is_refused_with_usage(self):
        code, _, err = self.run_main(["compare", "--kit", make_kit(KIT)], FakeBoard())
        self.assertNotEqual(code, 0)
        self.assertIn("usage", err)


class Fetch(unittest.TestCase):
    def test_every_board_file_is_saved_under_its_path_with_its_sha256(self):
        files = {"/boot.py": b"boot\n", "/lib/aiorepl.py": b"R\n"}
        out_dir = os.path.join(tempfile.mkdtemp(), "copy")
        code, out, err = quiet(upload_kit.fetch, FakeBoard(files=files), out_dir)
        self.assertEqual(code, 0, err)
        for path, data in files.items():
            with open(os.path.join(out_dir, path.lstrip("/")), "rb") as f:
                self.assertEqual(f.read(), data)
        with open(os.path.join(out_dir, "sha256.json")) as f:
            self.assertEqual(json.load(f), {p: hashlib.sha256(d).hexdigest() for p, d in files.items()})

    def test_a_file_that_cannot_be_downloaded_fails_the_copy(self):
        class Lossy(FakeBoard):
            def request(self, method, path, body=None, headers=None, cookie=True):
                if method == "GET" and path.startswith("/file?"):
                    return 500, {}, b"read failed"
                return super().request(method, path, body, headers, cookie)

        code, _, err = quiet(upload_kit.fetch, Lossy(files={"/boot.py": b"b\n"}), tempfile.mkdtemp())
        self.assertNotEqual(code, 0)
        self.assertIn("/boot.py", err)


class WhereThePortalClientLives(unittest.TestCase):
    """The client of the recovery page belongs to the neighbour project scrivo-safe, which lies OUTSIDE this
    repository. Where it is found must not depend on where this tree happens to lie on the disk: the same
    command runs in the working tree and in a copy of it, and a path counted upwards from this file lands
    in a folder that does not exist in the copy. The tool then fails at the sign-in, naming nothing a person
    can act on."""

    def laid(self, base):
        os.makedirs(os.path.join(base, "scrivo-safe", "tools"))
        with open(os.path.join(base, "scrivo-safe", "tools", "portal_probe.py"), "w"):
            pass
        os.makedirs(os.path.join(base, "scrivo-iot"))
        return os.path.join(base, "scrivo-iot"), os.path.realpath(os.path.join(base, "scrivo-safe", "tools"))

    def test_the_folder_is_taken_from_the_environment_when_it_is_named(self):
        with tempfile.TemporaryDirectory() as base:
            _, tools = self.laid(base)
            with mock.patch.dict(os.environ, {"SCRIVO_SAFE": os.path.dirname(tools)}):
                self.assertEqual(upload_kit.portal_tools(os.path.join(base, "nowhere", "scrivo-iot")), tools)

    def test_a_named_folder_without_the_client_is_refused_by_name(self):
        with tempfile.TemporaryDirectory() as base:
            with mock.patch.dict(os.environ, {"SCRIVO_SAFE": base}):
                with self.assertRaises(OSError) as raised:
                    upload_kit.portal_tools(base)
        self.assertIn("portal_probe.py", str(raised.exception))
        self.assertIn("SCRIVO_SAFE names %s" % base, str(raised.exception))

    def test_without_the_environment_the_neighbour_of_the_tree_is_taken(self):
        with tempfile.TemporaryDirectory() as base:
            root, tools = self.laid(base)
            with mock.patch.dict(os.environ, {}):
                os.environ.pop("SCRIVO_SAFE", None)
                self.assertEqual(upload_kit.portal_tools(root), tools)

    def test_with_neither_the_refusal_says_what_to_set(self):
        with tempfile.TemporaryDirectory() as base:
            root = os.path.join(base, "scrivo-iot")
            os.makedirs(root)
            with mock.patch.dict(os.environ, {}):
                os.environ.pop("SCRIVO_SAFE", None)
                with self.assertRaises(OSError) as raised:
                    upload_kit.portal_tools(root)
        self.assertIn("SCRIVO_SAFE is not set", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
