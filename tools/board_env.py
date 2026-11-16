"""What a device is built with, from its board: prints the variables tools/build.sh exports.

    python3 tools/board_env.py DEVICE [--root DIR] [--variant NAME]

The device names its board in its description: devices/<device>/<device>.yaml (`board:`, with the pins of each of its `modules`), or, for the
devices not moved yet, devices/<device>/Cargo.toml, [package.metadata.rust_app] board. The board,
boards/<board>/board.toml, gives the Rust target and the MCU, and points at the board's folder in scrivo-safe:
its bootloader.sdkconfig and partitions.csv are the ones every image of that board is built with. scrivo-safe is
$SCRIVO_SAFE, or the folder beside the tree (`scrivo_safe` below). Flash size,
frequency and mode for the image header come from that sdkconfig. Anything missing is an error that names it.

What hangs on which pin is the device's word, [package.metadata.rust_app.pins] in its Cargo.toml (name = "GPIO<n>",
as the box names them); the build checks every pin against the board's board.yml: a pin that is not
a key of its pins is not brought out, and the pins of its gesture and status_led are held by the system. Each pin
reaches the device's build.rs as STRAGA_PIN_<NAME>.

Which level closes the device's relays, relays_close_on = "low" or "high" of [package.metadata.rust_app], reaches
its build.rs as STRAGA_RELAYS_CLOSE_ON_LOW, 1 or 0.

What the device's settings start from, [package.metadata.rust_app.settings] (name = "value"), reaches its build.rs
as STRAGA_SETTINGS, a line each, name=value.
"""
import os
import re
import shlex
import sys
import tomllib


class BoardError(Exception):
    pass


def _toml(path, what):
    if not os.path.isfile(path):
        raise BoardError("%s: %s is missing" % (what, path))
    with open(path, "rb") as f:
        try:
            return tomllib.load(f)
        except tomllib.TOMLDecodeError as e:
            raise BoardError("%s: %s is not valid TOML: %s" % (what, path, e))


def _sdkconfig(path):
    values = {}
    for line in open(path, encoding="utf-8"):
        m = re.match(r"^(CONFIG_\w+)=(.*)$", line.strip())
        if m:
            values[m.group(1)] = m.group(2).strip('"')
    return values


def _spiram_in(files):
    """What the options of a build say of the external RAM, the files read in the order IDF reads them - the last word
    wins: (True or False, the file and the 1-based line that said it), or (False, None) when no file says anything.
    Only CONFIG_SPIRAM itself counts: CONFIG_SPIRAM_MODE_QUAD and the like are settings of the RAM, not the switch."""
    said = (False, None)
    for path in files:
        with open(path, encoding="utf-8") as f:
            for number, line in enumerate(f, 1):
                line = line.strip()
                m = re.match(r"^CONFIG_SPIRAM=(\w+)$", line)
                if m:
                    said = (m.group(1) == "y", (path, number))
                elif line == "# CONFIG_SPIRAM is not set":
                    said = (False, (path, number))
    return said


def scrivo_safe(root):
    """The folder of scrivo-safe, the recovery image whose board folders and tools this tree builds with:
    $SCRIVO_SAFE when the variable is set (tools/env.sh sets it), and otherwise the neighbour of the tree,
    <tree>/../scrivo-safe.

    THE NEIGHBOUR IS RIGHT ONLY FOR A TREE THAT LIES NEXT TO scrivo-safe. A copy of the tree for a branch lies
    elsewhere, with nothing beside it, and needs SCRIVO_SAFE."""
    named = os.environ.get("SCRIVO_SAFE")
    if named:
        return os.path.realpath(named)
    return os.path.realpath(os.path.join(root, "..", "scrivo-safe"))


def safe_folder(root, *inside, needs):
    """The folder INSIDE scrivo-safe that holds the file NEEDS - for a tool of this tree that takes code from the
    recovery image (its portal client). Found the way `scrivo_safe` finds it; not there - refused in words naming
    the file and where scrivo-safe was taken from."""
    folder = os.path.join(scrivo_safe(root), *inside)
    if not os.path.isfile(os.path.join(folder, needs)):
        raise BoardError("%s is not there; %s" % (os.path.join(folder, needs), _where_from(root)))
    return folder


def _where_from(root):
    """Which of the two ways found scrivo-safe, in words for a refusal."""
    named = os.environ.get("SCRIVO_SAFE")
    if named:
        return "SCRIVO_SAFE names %s as scrivo-safe" % named
    return ("SCRIVO_SAFE is not set, so scrivo-safe was looked for beside %s - put scrivo-safe next to the tree or "
            "set SCRIVO_SAFE to its folder (tools/env.sh)" % os.path.realpath(root))


def resolve(root, device, variant=None):
    """The build's variables for DEVICE. VARIANT is a property of this build, not of the device: boards/<board>/sdkconfig.<variant>
    stands in for the board's own sdkconfig.defaults (`no_psram` - the S3 on the bench built without its RAM, in the
    conditions of the Sonoff Dual's chip)."""
    if variant is not None and not re.fullmatch(r"[a-z][a-z0-9_]*", variant):
        raise BoardError("variant '%s' is not lowercase letters, digits and _" % variant)
    root = os.path.realpath(root)
    device_dir = os.path.join(root, "devices", device)
    if not os.path.isdir(device_dir):
        raise BoardError("no device '%s': %s is not a folder" % (device, device_dir))
    meta = _description(device_dir, device)
    board = meta["board"]

    board_dir = os.path.join(root, "boards", board)
    spec = _toml(os.path.join(board_dir, "board.toml"), "board %s of device %s" % (board, device))
    for key in ("target", "mcu", "safe"):
        if not spec.get(key):
            raise BoardError("board %s: board.toml has no %s" % (board, key))
    # Whether the board has the external RAM: the size of what the image keeps in its heap follows it (crates/plan), and an
    # image that asks for RAM the chip lacks aborts at the start. A board that does not say is refused: the big sizes are the
    # ones for a board with PSRAM, and a board that forgot the word would take them and fall.
    if "psram" not in spec:
        raise BoardError("board %s: board.toml does not say whether the board has psram: psram = true or false" % board)
    if not isinstance(spec["psram"], bool):
        raise BoardError("board %s: psram in board.toml is %r - write true or false" % (board, spec["psram"]))

    # safe is a path inside scrivo-safe, not one counted from this file: see `scrivo_safe`
    safe_dir = os.path.realpath(os.path.join(scrivo_safe(root), spec["safe"]))
    sdkconfig = os.path.join(safe_dir, "bootloader.sdkconfig")
    table = os.path.join(safe_dir, "partitions.csv")
    for path in (sdkconfig, table):
        if not os.path.isfile(path):
            raise BoardError("board %s: %s is missing; %s" % (board, path, _where_from(root)))
    common = os.path.join(root, "sdkconfig.defaults")
    if not os.path.isfile(common):
        raise BoardError("%s is missing" % common)

    # What only this board has - the RAM it carries, say - lives beside its board.toml and is read after the common
    # options: an option every board got would be asked of the boards that lack the part.
    if variant is None:
        own = [os.path.join(board_dir, "sdkconfig.defaults")]
        own = [path for path in own if os.path.isfile(path)]
    else:
        own = [os.path.join(board_dir, "sdkconfig.%s" % variant)]
        if not os.path.isfile(own[0]):
            raise BoardError("board %s has no variant '%s': %s is missing" % (board, variant, own[0]))

    # The word of the board and the options of the build are two truths about one fact (the heap profile follows the word,
    # the chip follows the options), and they must not part: a profile for a heap with RAM on an image without it falls
    # short, and an image that asks IDF for RAM the chip lacks aborts at the start. The variant `no_psram` is the S3 built
    # without its RAM on purpose: for it the options must switch the RAM off.
    builds_with_ram = spec["psram"] and variant != "no_psram"
    asked, where = _spiram_in([sdkconfig, common] + own)
    toml = os.path.join(board_dir, "board.toml")
    if builds_with_ram and not asked:
        last = "the last word on CONFIG_SPIRAM is %s line %d" % where if where else "none of the options mentions CONFIG_SPIRAM"
        raise BoardError(
            "board %s: board.toml says psram = true, but the options of the build do not switch the RAM on (%s). "
            "Put CONFIG_SPIRAM=y in %s, or write psram = false in %s"
            % (board, last, own[0] if own else os.path.join(board_dir, "sdkconfig.defaults"), toml))
    if asked and not builds_with_ram:
        said = ("board.toml says psram = false" if not spec["psram"]
                else "the variant %s builds the board without its RAM" % variant)
        mend = ("Take the line out of %s, or write psram = true in %s" % (where[0], toml) if not spec["psram"]
                else "Take the line out of %s" % where[0])
        raise BoardError(
            "board %s: %s, but %s line %d says CONFIG_SPIRAM=y: an image that asks for RAM the chip lacks aborts at the start. %s"
            % (board, said, where[0], where[1], mend))

    config = _sdkconfig(sdkconfig)
    chip = config.get("CONFIG_IDF_TARGET")
    if chip != spec["mcu"]:
        raise BoardError("board %s: %s is for %s, board.toml says mcu %s" % (board, sdkconfig, chip, spec["mcu"]))
    # The sdkconfig names the table relative to the C project; esp-idf-sys copies it there from scrivo-safe.
    glob_base = os.path.dirname(os.path.dirname(safe_dir))
    table_name = config.get("CONFIG_PARTITION_TABLE_CUSTOM_FILENAME", "")
    if os.path.realpath(os.path.join(glob_base, table_name)) != os.path.realpath(table):
        raise BoardError("board %s: %s names %s, not the board's %s" % (board, sdkconfig, table_name, table))
    size = config.get("CONFIG_ESPTOOLPY_FLASHSIZE")
    freq = next((k.rsplit("_", 1)[1].lower() for k, v in config.items()
                 if k.startswith("CONFIG_ESPTOOLPY_FLASHFREQ_") and v == "y"), None)
    if not size or not freq:
        raise BoardError("board %s: %s gives no flash size or frequency" % (board, sdkconfig))

    env = {
        "DEVICE_DIR": device_dir,
        "TARGET": spec["target"],
        "MCU": spec["mcu"],
        "ESP_IDF_SYS_ROOT_CRATE": device,
        # 1 when the image is built for a heap with external RAM behind it, 0 when its internal RAM is the whole heap: the
        # variant `no_psram` is the S3 built without its RAM, in the conditions of the Dual's chip
        "STRAGA_PSRAM": "1" if spec["psram"] and variant != "no_psram" else "0",
        # what the image calls itself on the network: the device's name, which its description says (`device:` of <device>.yaml), is read by the device's build as `env!`
        "STRAGA_DEVICE": device,
        # the board the device stands on, by the name of its boards/<board>: a stand that runs another device's modules is called what it is, on the board it is
        "STRAGA_BOARD": board,
        "ESP_IDF_SDKCONFIG_DEFAULTS": ";".join([sdkconfig, common] + own),
        "ESP_IDF_GLOB_BOARD_BASE": glob_base,
        "ESP_IDF_GLOB_BOARD_TABLE": table_name,
        "PARTITIONS": table,
        "FLASH_SIZE": size,
        "FLASH_FREQ": freq,
        # An IDF app image carries dio for any quad mode: the bootloader switches the flash itself.
        "FLASH_MODE": "dout" if config.get("CONFIG_ESPTOOLPY_FLASHMODE_DOUT") == "y" else "dio",
    }
    board_yml = os.path.join(safe_dir, "board.yml")
    env.update(_gesture(board, board_yml))
    env.update(_status_led(board, board_yml))
    env.update(_device_pins(device, board, board_yml, meta["pins"]))
    env.update(_relays_close_on(device, meta))
    env["STRAGA_SETTINGS"] = _device_settings(device, meta["settings"])
    return env


def _description(device_dir, device):
    """What the device says of itself: {board, pins, settings, relays_close_on}. Either `<device>.yaml` in its folder - the board, the
    level that closes its relays, its settings, and its `modules`, each with the pins it is wired to - or, for the devices
    not moved yet, [package.metadata.rust_app] of its Cargo.toml. Both is refused: a description in two places is two truths.

    The yaml's pins are `modules: <module>: <name>: GPIO<n>`, and a pin is named by its <name> alone, as the Cargo.toml's are, so the
    name two modules both use is refused."""
    manifest = _toml(os.path.join(device_dir, "Cargo.toml"), "device %s" % device)
    cargo = manifest.get("package", {}).get("metadata", {}).get("rust_app", {})
    path = os.path.join(device_dir, "%s.yaml" % device)
    if not os.path.isfile(path):
        if not cargo.get("board"):
            raise BoardError("device %s names no board: [package.metadata.rust_app] board in %s/Cargo.toml" % (device, device_dir))
        return {"board": cargo["board"], "pins": cargo.get("pins", {}), "settings": cargo.get("settings", {}),
                **({"relays_close_on": cargo["relays_close_on"]} if "relays_close_on" in cargo else {})}
    if cargo:
        raise BoardError("device %s is described twice: [package.metadata.rust_app] of its Cargo.toml and %s - keep the yaml" % (device, path))
    import yaml  # only a device with a yaml needs it: tools/build.sh asks uv for it

    try:
        with open(path, encoding="utf-8") as f:
            text = yaml.safe_load(f)
    except yaml.YAMLError as why:
        raise BoardError("device %s: %s is not valid YAML: %s" % (device, path, why))
    if not isinstance(text, dict):
        raise BoardError("device %s: %s holds no mapping" % (device, path))
    if text.get("device") != device:
        raise BoardError("%s says device: %s, and it lies in the folder of %s" % (path, text.get("device"), device))
    if not text.get("board"):
        raise BoardError("device %s: %s names no board: board: <name> at the top of it" % (device, path))
    pins, owner = {}, {}
    for module, own in (text.get("modules") or {}).items():
        for name, pin in (own or {}).items():
            if name in pins:
                raise BoardError("device %s: the pin name %s is used by two modules, %s and %s" % (device, name, owner[name], module))
            pins[name], owner[name] = pin, module
    meta = {"board": text["board"], "pins": pins, "settings": {k: v for k, v in (text.get("settings") or {}).items()}}
    if "relays_close_on" in text:
        meta["relays_close_on"] = text["relays_close_on"]
    return meta


def _relays_close_on(device, meta):
    """Which level closes the device's relays, relays_close_on = "low" or "high" of [package.metadata.rust_app]:
    STRAGA_RELAYS_CLOSE_ON_LOW = 1 or 0 for build.rs, which writes the open level to a relay's pin before the pin is
    an output (crates/relay open_pins). A device that names none gets nothing; its build.rs asks for it if it has
    relays."""
    if "relays_close_on" not in meta:
        return {}
    word = meta["relays_close_on"]
    if word not in ("low", "high"):
        raise BoardError("device %s: relays_close_on = %r - write low or high: the level of a pin that closes its relay"
                         % (device, word))
    return {"STRAGA_RELAYS_CLOSE_ON_LOW": "1" if word == "low" else "0"}


def _device_settings(device, settings):
    """What the device's settings start from on a board whose nvs has none, [package.metadata.rust_app.settings]:
    a line each, name=value, for build.rs, which checks each against crates/settings. Here only the form: a name of
    lowercase letters, digits and _, a value of text on one line."""
    lines = []
    for name, value in settings.items():
        if not re.fullmatch(r"[a-z][a-z0-9_]*", name):
            raise BoardError("device %s: setting name '%s' is not lowercase letters, digits and _" % (device, name))
        if not isinstance(value, str) or "\n" in value or "\r" in value:
            raise BoardError("device %s: setting %s = %r is not text on one line" % (device, name, value))
        lines.append("%s=%s" % (name, value))
    return "\n".join(lines)


def _board_text(board_yml):
    return "\n" + open(board_yml, encoding="utf-8").read() if os.path.isfile(board_yml) else None


def _status_led(board, board_yml):
    """The board's LED, from the status_led field of its board.yml: the pin and 1 when it is lit on high (inverted:
    true is lit on low). A board with no such field gives nothing; a field without a pin is an error."""
    text = _board_text(board_yml)
    if text is None or not re.search(r"\nstatus_led:", text):
        return {}
    block = re.search(r"\nstatus_led:[ \t]*\n((?:[ \t]+.*\n?)*)", text).group(1)
    m = re.search(r"^[ \t]+pin:[ \t]*GPIO([0-9]+)[ \t]*$", block, re.M)
    if not m:
        raise BoardError("board %s: the status_led field of %s has no pin: GPIO<n>" % (board, board_yml))
    inverted = re.search(r"^[ \t]+inverted:[ \t]*true[ \t]*$", block, re.M)
    return {"STRAGA_STATUS_LED_PIN": m.group(1), "STRAGA_STATUS_LED_LIT": "0" if inverted else "1"}


def _device_pins(device, board, board_yml, pins):
    """The pins the device names, checked against its board: brought out (a key of pins in board.yml) and not
    held by the system (the pins of gesture and status_led). Gives STRAGA_PIN_<NAME> = the pin number for each."""
    if not pins:
        return {}
    text = _board_text(board_yml)
    if text is None:
        raise BoardError("device %s names pins (%s), and board %s has no %s to check them against"
                         % (device, ", ".join(pins), board, board_yml))
    pins_block = re.search(r"\npins:[ \t]*\n((?:[ \t]+.*\n?)*)", text)
    brought_out = set(re.findall(r"^[ \t]+GPIO([0-9]+):", pins_block.group(1), re.M)) if pins_block else set()
    held = {}
    if "STRAGA_GESTURE_PIN" in (g := _gesture(board, board_yml)):
        held[g["STRAGA_GESTURE_PIN"]] = "gesture"
    if "STRAGA_STATUS_LED_PIN" in (led := _status_led(board, board_yml)):
        held[led["STRAGA_STATUS_LED_PIN"]] = "status_led"

    env, taken = {}, {}
    for name, pin in pins.items():
        if not re.fullmatch(r"[a-z][a-z0-9_]*", name):
            raise BoardError("device %s: pin name '%s' is not lowercase letters, digits and _" % (device, name))
        m = re.fullmatch(r"GPIO([0-9]+)", pin) if isinstance(pin, str) else None
        if not m:
            raise BoardError("device %s: %s = %r is not of the form GPIO<n>" % (device, name, pin))
        number = str(int(m.group(1)))
        if number not in brought_out:
            raise BoardError("device %s: %s on %s - not brought out on board %s (no key GPIO%s in pins of %s)"
                             % (device, name, pin, board, number, board_yml))
        if number in held:
            raise BoardError("device %s: %s on %s - held by the system on board %s: %s of %s"
                             % (device, name, pin, board, held[number], board_yml))
        if number in taken:
            raise BoardError("device %s: GPIO%s is named twice: %s and %s" % (device, number, taken[number], name))
        taken[number] = name
        env["STRAGA_PIN_" + name.upper()] = number
    return env


def _gesture(board, board_yml):
    """The button of the ways down (crates/ways_down), from the gesture field of the board's board.yml read with the
    same pattern as the C of the ways down reads it (crates/ways_down): the pin and 1 when pressed is high. A board
    with no such field gives nothing; a field of another form is an error."""
    if not os.path.isfile(board_yml):
        return {}
    text = "\n" + open(board_yml, encoding="utf-8").read()
    if not re.search(r"\ngesture:", text):
        return {}
    m = re.search(r"\ngesture:[ \t]*\n[ \t]+pin:[ \t]*GPIO([0-9]+)[ \t]*\n[ \t]+level:[ \t]*(low|high)", text)
    if not m:
        raise BoardError("board %s: the gesture field of %s is not of the form pin: GPIO<n>, level: low|high"
                         % (board, board_yml))
    return {"STRAGA_GESTURE_PIN": m.group(1), "STRAGA_GESTURE_ACTIVE": "1" if m.group(2) == "high" else "0"}


def main():
    args = sys.argv[1:]
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    variant = None
    if "--variant" in args:
        i = args.index("--variant")
        variant = args[i + 1]
        del args[i:i + 2]
    if "--root" in args:
        i = args.index("--root")
        root = args[i + 1]
        del args[i:i + 2]
    if len(args) != 1:
        sys.exit(__doc__)
    try:
        env = resolve(root, args[0], variant)
    except BoardError as e:
        sys.exit("board_env: %s" % e)
    for key, value in env.items():
        print("export %s=%s" % (key, shlex.quote(value)))


if __name__ == "__main__":
    main()
