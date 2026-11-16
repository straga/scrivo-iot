"""Makes the board a mirror of a built kit through the recovery page, and says whether it still is one.

  upload_kit.py put    --kit build/DEVICE/kit --board HOST
      refuses a folder without /version.txt or one carrying /data; writes every file of the kit at its board path
      and reads each back, then takes off the board every file and directory outside /data the kit does not have and
      lists the board again.
  upload_kit.py verify --kit build/DEVICE/kit --board HOST
      reads every kit file back and fails on one missing, one with other bytes, or anything beyond the kit outside
      /data.
  upload_kit.py fetch  --out DIR --board HOST
      saves every file the board holds under DIR at its board path, with DIR/sha256.json.

The tree is the source and the board its mirror, everywhere but /data: what the board makes itself - the energy
counted, rows of measurements, logs - lies there, has no copy on the computer, and a mirror neither writes nor takes it
off. Outside /data nothing on the board is kept or brought back, and what a board needs goes into the kit of its
device. The page and its sign-in belong to the recovery image, so the client is
scrivo-safe's tools/portal_probe.py imported from the neighbour project, not a copy of it.

Taken from the MicroPython project's upload client when that project stopped being a part of this one: `put`, `verify` and `fetch` are the same, the seeding of a network through the page's
/wifi is left behind - a Rust board keeps its networks in nvs, and a kit never writes them.
"""
import hashlib
import json
import os
import sys
import urllib.parse

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from board_env import BoardError, safe_folder  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def portal_tools(root=ROOT):
    """Where the recovery page's client lives: scrivo-safe/tools, found by board_env.safe_folder - $SCRIVO_SAFE, else
    the neighbour of the tree. Not there - the refusal names the file and where scrivo-safe was looked for."""
    try:
        return safe_folder(root, "tools", needs="portal_probe.py")
    except BoardError as why:
        raise OSError(str(why))


# What the board makes itself; a kit carries none of it, and a mirror leaves it where it is.
DATA = "/data"


def _is_data(board_path):
    return board_path == DATA or board_path.startswith(DATA + "/")


def _kit_files(kit):
    files = {}
    for dirpath, _, filenames in os.walk(kit, followlinks=True):
        for name in filenames:
            path = os.path.join(dirpath, name)
            with open(path, "rb") as f:
                files["/" + os.path.relpath(path, kit)] = f.read()
    return files


def _board_listing(board):
    """Returns (files, directories) the board holds, as paths."""
    status, _, body = board.request("GET", "/files")
    if status != 200:
        raise OSError("GET /files: %s %s" % (status, body[:60]))
    entries = json.loads(body)["entries"]
    return [e["path"] for e in entries if not e.get("dir")], [e["path"] for e in entries if e.get("dir")]


def _board_files(board):
    return _board_listing(board)[0]


def _kit_dirs(files):
    """Every directory the kit's files lie in."""
    dirs = set()
    for board_path in files:
        parts = board_path.strip("/").split("/")[:-1]
        dirs.update("/" + "/".join(parts[:i + 1]) for i in range(len(parts)))
    return dirs


def _beyond_kit(board, files):
    """Files and directories on the board that the kit does not have, directories deepest first; /data is the
    board's own and never beyond the kit."""
    board_files, board_dirs = _board_listing(board)
    board_files = [p for p in board_files if not _is_data(p)]
    board_dirs = [p for p in board_dirs if not _is_data(p)]
    extra_dirs = sorted(set(board_dirs) - _kit_dirs(files), key=lambda d: (-d.count("/"), d))
    return sorted(set(board_files) - set(files)), extra_dirs


def put(board, kit):
    """Makes the board a mirror of a built kit: the tree is the source, the board holds the kit and nothing else but
    its own /data, which is neither written nor taken off; a kit carrying /data is refused before anything is written.
    A folder without /version.txt is no built kit, and nothing on the board is touched. Every kit file is written and
    read back first; only when all of them landed are the files and directories beyond the kit taken off, and the
    board is listed again to confirm it. Nothing is read off the board to be kept: what the tree lacks is not wanted."""
    files = _kit_files(kit) if os.path.isdir(kit) else {}
    if not files:
        print("kit NOT uploaded: no files in kit %s" % kit, file=sys.stderr)
        return 1
    if "/version.txt" not in files:
        print("kit NOT uploaded: %s has no /version.txt, so it is no built kit, and a mirror of it would wipe the board"
              % kit, file=sys.stderr)
        return 1
    carried = sorted(p for p in files if _is_data(p))
    if carried:
        print("kit NOT uploaded: it carries %s - %s is what the board makes itself, and a kit writes nothing there"
              % (", ".join(carried), DATA), file=sys.stderr)
        return 1
    failed = 0
    for board_path, data in sorted(files.items()):
        digest = hashlib.sha256(data).hexdigest()
        query = "?path=" + urllib.parse.quote(board_path)
        status, _, body = board.request("POST", "/file" + query, data, {"Content-Type": "application/octet-stream"})
        if status != 200:
            print("refused %s: %s %s" % (board_path, status, body[:60]), file=sys.stderr)
            failed += 1
            continue
        status, _, back = board.request("GET", "/file" + query)
        if status != 200 or hashlib.sha256(back).hexdigest() != digest:
            print("read back differs %s: %s, %d bytes" % (board_path, status, len(back)), file=sys.stderr)
            failed += 1
            continue
        print("  %-24s %6d bytes  sha256 %s  written, read back equal" % (board_path, len(data), digest[:16]))
    if failed:
        print("kit NOT uploaded whole: %d of %d file(s) failed; nothing was taken off the board" % (failed, len(files)),
              file=sys.stderr)
        return 1
    extra_files, extra_dirs = _beyond_kit(board, files)
    for board_path in extra_files + extra_dirs:
        status, _, body = board.request("POST", "/file/delete?path=" + urllib.parse.quote(board_path), b"")
        if status != 200:
            print("not taken off %s: %s %s" % (board_path, status, body[:60]), file=sys.stderr)
        else:
            print("  %-24s taken off the board: not in the kit" % board_path)
    left_files, left_dirs = _beyond_kit(board, files)
    missing = sorted(set(files) - set(_board_files(board)))
    if left_files or left_dirs or missing:
        for board_path in left_files + left_dirs:
            print("still on the board, not in the kit: %s" % board_path, file=sys.stderr)
        for board_path in missing:
            print("missing on the board after the upload: %s" % board_path, file=sys.stderr)
        print("board is NOT a mirror of the kit", file=sys.stderr)
        return 1
    print("%d files uploaded and read back equal, %d path(s) taken off: the board holds the kit and nothing else"
          % (len(files), len(extra_files) + len(extra_dirs)))
    return 0


def verify(board, kit):
    """Reads every kit file back from the board and names the ones missing or with other bytes, and every file or
    directory the board holds beyond the kit; writes nothing."""
    files = _kit_files(kit) if os.path.isdir(kit) else {}
    if not files:
        print("kit NOT verified: no files in kit %s" % kit, file=sys.stderr)
        return 1
    failed = 0
    for board_path, data in sorted(files.items()):
        digest = hashlib.sha256(data).hexdigest()
        status, _, back = board.request("GET", "/file?path=" + urllib.parse.quote(board_path))
        if status != 200:
            print("missing on the board %s: %s" % (board_path, status), file=sys.stderr)
            failed += 1
        elif hashlib.sha256(back).hexdigest() != digest:
            print("other bytes on the board %s: %d bytes, the kit has %d" % (board_path, len(back), len(data)),
                  file=sys.stderr)
            failed += 1
        else:
            print("  %-24s %6d bytes  sha256 %s  equal" % (board_path, len(data), digest[:16]))
    extra_files, extra_dirs = _beyond_kit(board, files)
    for board_path in extra_files + extra_dirs:
        print("on the board, not in the kit: %s" % board_path, file=sys.stderr)
    if failed or extra_files or extra_dirs:
        print("board does NOT hold the kit: %d of %d file(s) differ, %d path(s) beyond it"
              % (failed, len(files), len(extra_files) + len(extra_dirs)), file=sys.stderr)
        return 1
    print("board holds the kit: %d files equal" % len(files))
    return 0


def fetch(board, out_dir):
    sums, failed = {}, 0
    for board_path in sorted(_board_files(board)):
        status, _, data = board.request("GET", "/file?path=" + urllib.parse.quote(board_path))
        if status != 200:
            print("not saved %s: %s %s" % (board_path, status, data[:60]), file=sys.stderr)
            failed += 1
            continue
        dest = os.path.join(out_dir, board_path.lstrip("/"))
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "wb") as f:
            f.write(data)
        sums[board_path] = hashlib.sha256(data).hexdigest()
        print("  %-24s %6d bytes  sha256 %s" % (board_path, len(data), sums[board_path][:16]))
    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, "sha256.json"), "w") as f:
        json.dump(sums, f, indent=1, sort_keys=True)
    if failed:
        print("board files NOT saved whole: %d failed" % failed, file=sys.stderr)
        return 1
    print("%d files saved to %s" % (len(sums), out_dir))
    return 0


def _connect(host):
    """Signs in to the recovery page with the client that belongs to it."""
    sys.path.insert(0, portal_tools())
    import portal_probe as pp
    board = pp.Board(host)
    pp.sign_in(board, *pp.credentials())
    return board


def main(argv, connect=_connect):
    if "--board" not in argv or argv.index("--board") + 1 >= len(argv):
        print("usage: upload_kit.py put|verify --kit DIR | fetch --out DIR --board HOST (the board to connect to is required)",
              file=sys.stderr)
        return 1
    board = connect(argv[argv.index("--board") + 1])
    try:
        if argv and argv[0] == "put" and "--kit" in argv:
            return put(board, argv[argv.index("--kit") + 1])
        if argv and argv[0] == "verify" and "--kit" in argv:
            return verify(board, argv[argv.index("--kit") + 1])
        if argv and argv[0] == "fetch" and "--out" in argv:
            return fetch(board, argv[argv.index("--out") + 1])
        print("usage: upload_kit.py put|verify --kit DIR | fetch --out DIR --board HOST", file=sys.stderr)
        return 1
    finally:
        board.request("POST", "/logout", b"")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
