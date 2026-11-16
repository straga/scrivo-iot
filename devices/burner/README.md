# Burner - by layers

The firmware of a pellet burner on ESP32-S3. The ten modules - `relays`, `temperatures`, `flue`, `fan`, `draught`, `flame`, `pellets`, `thermostat`, `demand`, `automaton` - each have a file of the same name in `src/modules/` and in `api/src/`, and are named in `burner.yaml`.

Open the folder and look like this: four layers lie in four places, and each place is labelled by the header of its files.

| Layer | Where | What is in it | What is never in it |
|---|---|---|---|
| hardware | `src/hardware/` | pins, buses, relays, interrupts, the gate: read a pin, set a pin | the meaning of numbers, words |
| logic | `../../crates/` (`automaton`, `thermostat`, `demand`, `schedule`, `regulator`, `flame`, `draught`, `max6675`, `modbus`, `burner_map`, `dimmer`, `pellets`) | what the readings mean, the automaton, the regulator; tests on the computer | pins, screen, words |
| API | `api/` - a crate of its own, `burner_api` | entities: name, kind, unit, limits, key; values, commands, the stream | cards, order, levels, words, legends |
| interface | `page/` - an addition to the engine's page (`../../page/`): the scheme and the burning (`parts.js`, `scheme.js`, `burning.js`), the texts of the modules (`menu.json`, `texts.json`) | where and how to show: sections, order, levels, the texts in Russian and in English | knowledge of the burner beyond the names of the API |

Besides the layers:

| What | Where | What for |
|---|---|---|
| the description of the burner | `burner.yaml` | the board, the pins of every module, the level that closes the relays; the build reads it and checks the pins against the board's `board.yml` |
| the assembly of a module | `src/modules/` | **the only place where the layers meet**: a module reads the hardware, asks the logic, publishes entities of the API |
| the assembly of the image | `src/main.rs`, `build.rs` | `main.rs` only raises the board's services and the modules; it holds no numbers and no words of its own |

## How to look

```sh
tools/build.sh burner                      # the image: build/burner/burner.bin (it is not laid on a board)
uv run --no-project python tools/page_stand.py --port 8790   # the page on a stand: http://127.0.0.1:8790/
cargo +stable test -p burner_api           # what the API says outward
```
