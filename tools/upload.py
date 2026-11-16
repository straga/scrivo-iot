"""Puts a device's image on its board the way the recovery page does: signs in, POST /update with the image -
the page's "Write to ota_0 and boot it" - and prints the page's answer. The board must be in factory.

    uv run --no-project python tools/upload.py --device DEVICE --board HOST [--image PATH]

The address has no default: with several devices a default sends an image to the wrong board. The image
defaults to build/<device>/<device>.bin from tools/build.sh.

The client and the credentials are the recovery image's own (scrivo-safe/tools/portal_probe.py, which reads the password from the project's secrets.yaml); the password is never printed.
"""
import argparse
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
from board_env import BoardError, safe_folder  # noqa: E402

# scrivo-safe is found by board_env.safe_folder: $SCRIVO_SAFE, else beside this tree
try:
    sys.path.insert(0, safe_folder(os.path.dirname(HERE), "tools", needs="portal_probe.py"))
except BoardError as why:
    raise SystemExit(f"upload: {why}")
import portal_probe as pp  # noqa: E402


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--device", required=True)
    parser.add_argument("--board", required=True)
    parser.add_argument("--image")
    args = parser.parse_args()
    args.image = args.image or os.path.join(HERE, "..", "build", args.device, args.device + ".bin")

    image = open(args.image, "rb").read()
    board = pp.Board(args.board)
    status, _, body = board.request("GET", "/status", cookie=False)
    running = json.loads(body).get("running") if status == 200 else None
    if running != "factory":
        raise SystemExit("the board at %s answers %s, running %s: it has to be in factory" % (args.board, status, running))
    status, _, _ = pp.sign_in(board, *pp.credentials())
    if not board.cookie:
        raise SystemExit("sign-in refused: %s" % status)
    print("uploading %s, %d bytes, to %s" % (os.path.basename(args.image), len(image), args.board))
    status, _, body = board.request("POST", "/update", image, {"Content-Type": "application/octet-stream"}, timeout=180)
    print("page answered %d: %s" % (status, body.decode(errors="replace").strip()))
    sys.exit(0 if status == 200 else 1)


if __name__ == "__main__":
    main()
