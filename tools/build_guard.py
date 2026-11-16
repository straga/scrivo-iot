"""What a run of the tests may not do: change what lies in `build/` of the real tree (6.13, after the flash of 26.10.03.1).

`build/<device>/kit` is what `upload_app.py put` mirrors onto a board, and `build/<device>/<device>.bin` is the image `upload.py` writes. A test that laid a kit with the real tree for its root rewrote the kit with the page of
whatever branch the tree stood on, and on 03.10 that kit went to the boiler: the version in `version.txt` was the same, the page was not. So a run of the tests is held to leave `build/` as it found it: `conftest.py` takes the
sums of every file under it when the run starts and again when it ends, and a run in which they differ fails and says which file.
"""
import hashlib
import os


def sums(root):
    """{path under root/build: sha256 of the file}, for every file under it; an empty answer where there is no build folder."""
    base = os.path.join(root, "build")
    found = {}
    for folder, _, names in os.walk(base):
        for name in names:
            path = os.path.join(folder, name)
            with open(path, "rb") as f:
                found[os.path.relpath(path, base)] = hashlib.sha256(f.read()).hexdigest()
    return found


def changes(before, after):
    """What differs between two answers of `sums`: [(path, "changed" | "added" | "gone")], in path order."""
    out = [(path, "gone") for path in before if path not in after]
    out += [(path, "added") for path in after if path not in before]
    out += [(path, "changed") for path in before if path in after and before[path] != after[path]]
    return sorted(out)
