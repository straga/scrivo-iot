# scrivo-iot

Firmware in Rust for ESP32 boards, built on ESP-IDF. It includes the firmware of a pellet burner on an ESP32-S3 and of a
Sonoff Dual R3 relay board. Each device serves a web page on port 80 and a shell on USB and on TCP port 23, keeps a
journal, and is updated from a browser through [scrivo-safe](https://github.com/straga/scrivo-safe), the recovery image
that sits in the `factory` partition next to it.

The image built here goes into `ota_0`. It does not flash itself: you upload it on the Firmware tab of the recovery
image, which checks it before writing a byte.

## What is in the tree

```
Cargo.toml            workspace: members crates/* and devices/*, shared dependency versions
sdkconfig.defaults    ESP-IDF settings shared by every board (task stacks); what only one board needs is in boards/<board>/sdkconfig.defaults
boards/<board>/       board.toml: Rust target, MCU, where the board's files live in scrivo-safe
devices/<device>/     the binary of one device: src/, Cargo.toml (names its board), version.txt, build.rs, page/
crates/               shared code: firmware (what every device runs), shell, journal, web, vfs, settings, relay, ...
page/                 the web page of the engine (OWL, Tailwind, light and dark themes)
tools/                build, upload, probes of a live board, tests that run on a computer
tests/                builds every device
docs/                 the contract of a device's API, the name of the main image
```

### Devices

| Device | Board | What it is |
|---|---|---|
| `burner` | `STRAGA_BASE_S3` (ESP32-S3, 8 MB flash, 2 MB PSRAM) | a pellet burner: relays, thermocouples, a flue-gas draught sensor, a fan on a triac, a burn automaton, a page with the scheme of the boiler |
| `bench_s3` | `STRAGA_BASE_S3` | the bench image of the same board: network, rollback confirmation, journal, shell |
| `bench_dualr3` | `SONOFF_DUAL_R3` (ESP32, 4 MB flash, no PSRAM) | Sonoff Dual R3 with its BL0939 energy meter: two relays, wall switches, power readings |
| `bench_dualr3s3` | `STRAGA_BASE_S3` | the Dual R3 firmware running on the S3 board, without PSRAM, on free pins |

The board is what the hardware gives (chip, flash, which pins are wired and which are taken by the system). The device
is what we made of it: which thing sits on which pin, in `[package.metadata.rust_app.pins]` of its `Cargo.toml`.
The build checks every pin against the board's `board.yml` in scrivo-safe and refuses a pin the board does not expose
or that the system uses, naming the pin and the reason.

## Build

You need three things beside this tree:

- **scrivo-safe**, in a folder next to this one: `scrivo-safe/` and `scrivo-iot/` side by side. The partition table and
  the bootloader options of each board live there, one set for every image on the board. If it lies elsewhere, set
  `SCRIVO_SAFE=/path/to/scrivo-safe`.
- **ESP-IDF 5.5.2**, the version scrivo-safe is built with. Point `IDF_PATH` at the checkout (the folder with
  `export.sh`) and `IDF_TOOLS_PATH` at the folder where `install.sh` put the tools.
- **The Xtensa Rust toolchain** from [`espup`](https://github.com/esp-rs/espup), and `ldproxy`. `espup` writes
  `~/export-esp.sh`; the build reads `LIBCLANG_PATH` from it, so run `espup install` first and leave that file in place.
  [uv](https://docs.astral.sh/uv/) runs the Python tools.

```sh
export IDF_PATH=<esp-idf v5.5.2> IDF_TOOLS_PATH=<where install.sh put the tools>
cd scrivo-iot
tools/build.sh bench_s3        # build/bench_s3/bench_s3.bin
tools/build.sh burner          # build/burner/burner.bin
tests/build_devices.sh         # every device, each for the chip of its board, image header checked
```

`cargo build -p <device>` on its own does not work: the target, the MCU and the IDF files come from the board through
`tools/build.sh`. A missing board or board file stops the script, which names what is missing.

The recovery image accepts only an image whose project name equals its `SCRIVO_SAFE_MAIN_PROJECT` (default
`micropython`). Build with the same name on both sides if you change it: `SCRIVO_MAIN_PROJECT=my-firmware
tools/build.sh bench_s3`. See `docs/MAIN_IMAGE_NAME.md`.

## Put it on a board

The board stands in the recovery image (`factory`). Its address is required; there is no default.

```sh
uv run --no-project python tools/upload.py --device bench_s3 --board <board-ip>
nc <board-ip> 23     # journal and shell: status, restart, down, help
```

The files of the page live in the `vfs` partition, not in the image, and are put there by the recovery image:

```sh
uv run --no-project python tools/page_kit.py bench_s3
uv run --no-project --with pyyaml --with requests python tools/upload_kit.py put --kit build/bench_s3/kit --board <board-ip>
```

`put` writes every file of the kit, reads each back, and then removes every file and folder on the board that the kit does not
have, except `/data`. `/data` holds what the board makes itself (energy counts, measurements, logs): `put` neither writes nor
removes anything there, and a kit that carries `/data` is refused. `tools/upload_kit.py verify` checks the board against the kit
the same way. Raise the build number in the device's
`version.txt` before every new upload.

## Tests

```sh
tools/test_host.sh                                              # every row that runs on a computer
uv run --no-project --with pytest --with pyyaml pytest tools   # parsing of a device's board and pins
```

Most of the logic (the burn automaton, the draught regulator, the pellet count, the shell, the journal) is plain Rust
without hardware and is tested on the computer. The tools whose name ends in `_probe` talk to a live board and need its
address.

## License

MIT, see [LICENSE](LICENSE). Code taken from other projects keeps its own terms; where it comes from is in
[CREDITS.md](CREDITS.md).
