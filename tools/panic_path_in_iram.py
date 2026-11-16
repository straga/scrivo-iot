"""The way a fall writes its note touches no flash: read off the built ELF, not trusted from the source.

    . tools/env.sh && uv run --no-project python tools/panic_path_in_iram.py [ELF] [--also FUNCTION ...]

A fall while the flash chip erases finds code and rodata in flash unreadable, and a note written from there is not
written (6.6, the burner on 26.09). What is checked, for every function of the way (WAY below, and --also):
- it lies in IRAM, by nm;
- every direct call it makes lands in IRAM, and every address it loads as a literal (l32r) lies outside flash - the
  code at 0x42000000.. and the rodata at 0x3c000000.. -, by objdump;
- every entry of IDF's tables of reasons (panic_arch.c) points inside, since the note copies the one it is given.
What it cannot see: an address the way is handed at run time - the reason, the details of an abort. That one the
note copies only when it lies inside (panic_note.c, keep_reason) and keeps the address of otherwise.

Prints one line per function and FAIL for each address in flash; exit 1 when there is any.
"""
import re
import subprocess
import sys

WAY = [
    "__wrap_esp_panic_handler",
    "write_note",
    "keep_reason",
    "walk",
    "walk_other",
    "copy_text",
    "reseal",
    "sum_of",
    "esp_panic_handler",
    "esp_backtrace_get_next_frame",
]
TABLES = ["reason$1", "pseudo_reason$0"]
FLASH = [(0x42000000, 0x44000000), (0x3C000000, 0x3E000000)]
IRAM = (0x40370000, 0x403E0000)


def in_flash(address):
    return any(low <= address < high for low, high in FLASH)


def run(*command):
    return subprocess.run(command, capture_output=True, text=True, check=True).stdout


def main():
    args = sys.argv[1:]
    elf = "target/xtensa-esp32s3-espidf/release/burner"
    also = []
    if args and not args[0].startswith("--"):
        elf = args.pop(0)
    if args[:1] == ["--also"]:
        also = args[1:]
    symbols = {}
    for line in run("xtensa-esp32s3-elf-nm", "-S", elf).splitlines():
        parts = line.split()
        if len(parts) == 4:
            symbols.setdefault(parts[3], (int(parts[0], 16), int(parts[1], 16)))
    failed = 0
    for name in WAY + also:
        if name not in symbols:
            print(f"{name:32} not in the image (inlined into its caller)")
            continue
        start, size = symbols[name]
        where = "IRAM" if IRAM[0] <= start < IRAM[1] else ("FLASH" if in_flash(start) else f"{start:#x}")
        bad = [] if where == "IRAM" else [f"the function itself at {start:#010x}"]
        listing = run("xtensa-esp32s3-elf-objdump", "-d", f"--start-address={start:#x}",
                      f"--stop-address={start + size:#x}", elf)
        for line in listing.splitlines():
            call = re.search(r"\bcall(?:0|4|8|12)\s+([0-9a-f]+)", line)
            literal = re.search(r"\bl32r\s+\w+,\s*[0-9a-f]+\s*\(([0-9a-f]+)\)", line)
            for found, what in ((call, "calls"), (literal, "loads")):
                if found and in_flash(int(found.group(1), 16)):
                    bad.append(f"{what} {int(found.group(1), 16):#010x}: {line.strip()}")
        print(f"{name:32} {where:6} {start:#010x}" + ("" if not bad else "  FAIL"))
        for one in bad:
            print(f"    FAIL {one}")
        failed += len(bad)
    for table in TABLES:
        if table not in symbols:
            print(f"{table:32} not in the image")
            continue
        start, size = symbols[table]
        dump = run("xtensa-esp32s3-elf-objdump", "-s", f"--start-address={start:#x}",
                   f"--stop-address={start + size:#x}", elf)
        words = []
        for line in dump.splitlines():
            m = re.match(r"^\s*[0-9a-f]{8}((?:\s[0-9a-f]{8}){1,4})", line)
            if m:
                words += [int.from_bytes(bytes.fromhex(w), "little") for w in m.group(1).split()]
        flash = [w for w in words if in_flash(w)]
        print(f"{table:32} {len(words)} texts, in flash {len(flash)}" + ("  FAIL" if flash else ""))
        failed += len(flash)
    print("PASS" if failed == 0 else f"FAIL: {failed} addresses in flash")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
