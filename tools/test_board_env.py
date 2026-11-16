"""tools/board_env.py on small trees laid out like scrivo-iot next to scrivo-safe, and on the real tree.

    uv run --no-project --with pytest pytest tools
"""
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from board_env import BoardError, safe_folder, scrivo_safe, resolve  # noqa: E402

REAL_ROOT = os.path.dirname(HERE)

S3_SDKCONFIG = """CONFIG_IDF_TARGET="esp32s3"
CONFIG_ESPTOOLPY_FLASHSIZE_8MB=y
CONFIG_ESPTOOLPY_FLASHSIZE="8MB"
CONFIG_ESPTOOLPY_FLASHMODE_QIO=y
CONFIG_ESPTOOLPY_FLASHFREQ_80M=y
CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="boards/STRAGA_BASE_S3/partitions.csv"
"""
S3_TABLE = "factory,  app,  factory, ,        1536K,\nota_0,    app,  ota_0,   ,        3072K,\n"


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)


def tree(tmp_path, board_toml=None, device_toml=None, sdkconfig=S3_SDKCONFIG, table=S3_TABLE, common="CONFIG_SPIRAM=y\n"):
    scrivo_safe = tmp_path / "scrivo-safe" / "boards" / "STRAGA_BASE_S3"
    if sdkconfig is not None:
        write(str(scrivo_safe / "bootloader.sdkconfig"), sdkconfig)
    if table is not None:
        write(str(scrivo_safe / "partitions.csv"), table)
    root = tmp_path / "scrivo-iot"
    # the default board says psram = true, and the common options ask for it: the two have to agree (board_env.resolve)
    write(str(root / "sdkconfig.defaults"), "CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192\n" + common)
    write(str(root / "boards" / "STRAGA_BASE_S3" / "board.toml"), board_toml if board_toml is not None else (
        'target = "xtensa-esp32s3-espidf"\nmcu = "esp32s3"\nsafe = "boards/STRAGA_BASE_S3"\npsram = true\n'))
    write(str(root / "devices" / "bench_s3" / "Cargo.toml"), device_toml if device_toml is not None else (
        '[package]\nname = "bench_s3"\nversion = "0.1.0"\n\n[package.metadata.rust_app]\nboard = "STRAGA_BASE_S3"\n'))
    write(str(root / "devices" / "bench_s3" / "version.txt"), "stragabases3-26.09.19.5\n")
    return str(root)


@pytest.fixture(autouse=True)
def no_scrivo_safe_variable(monkeypatch):
    """Every test starts without SCRIVO_SAFE, whatever the shell it runs in has sourced: the trees here are laid out
    next to their scrivo-safe, and a test that wants the variable sets it."""
    monkeypatch.delenv("SCRIVO_SAFE", raising=False)


def copy_away(tmp_path):
    """A scrivo-safe, and a copy of scrivo-iot somewhere else with nothing beside it - as a copy of the tree under
    a worktree of git stands. Answers the copy and the scrivo-safe."""
    root = tree(tmp_path / "home")
    safe = str(tmp_path / "home" / "scrivo-safe")
    copy = tmp_path / "worktree" / "scrivo-iot_some-branch"
    os.makedirs(str(copy.parent))
    os.rename(root, str(copy))
    return str(copy), safe


def test_a_copy_of_the_tree_finds_scrivo_safe_where_the_variable_names_it(tmp_path, monkeypatch):
    copy, safe = copy_away(tmp_path)
    monkeypatch.setenv("SCRIVO_SAFE", safe)
    env = resolve(copy, "bench_s3")
    board = os.path.join(os.path.realpath(safe), "boards", "STRAGA_BASE_S3")
    assert env["PARTITIONS"] == os.path.join(board, "partitions.csv")
    assert env["ESP_IDF_SDKCONFIG_DEFAULTS"].split(";")[0] == os.path.join(board, "bootloader.sdkconfig")


def test_a_copy_without_the_variable_looks_beside_itself_and_says_what_to_set(tmp_path):
    copy, _ = copy_away(tmp_path)
    with pytest.raises(BoardError, match="SCRIVO_SAFE"):
        resolve(copy, "bench_s3")


def test_the_variable_naming_an_empty_place_is_refused_in_words_naming_it(tmp_path, monkeypatch):
    root = tree(tmp_path)
    empty = tmp_path / "nothing_here"
    os.makedirs(str(empty))
    monkeypatch.setenv("SCRIVO_SAFE", str(empty))
    with pytest.raises(BoardError, match="SCRIVO_SAFE.*nothing_here"):
        resolve(root, "bench_s3")


def test_without_the_variable_scrivo_safe_is_the_trees_neighbour(tmp_path):
    root = tree(tmp_path)
    assert scrivo_safe(root) == os.path.realpath(str(tmp_path / "scrivo-safe"))
    assert resolve(root, "bench_s3")["PARTITIONS"].startswith(os.path.realpath(str(tmp_path / "scrivo-safe")))


def test_a_tool_in_a_copy_finds_a_file_of_scrivo_safe_where_the_variable_names_it(tmp_path, monkeypatch):
    copy, safe = copy_away(tmp_path)
    write(os.path.join(safe, "tools", "portal_probe.py"), "")
    monkeypatch.setenv("SCRIVO_SAFE", safe)
    found = safe_folder(copy, "tools", needs="portal_probe.py")
    assert found == os.path.join(os.path.realpath(safe), "tools")


def test_a_tool_without_the_variable_finds_the_neighbour_of_its_tree(tmp_path):
    root = tree(tmp_path)
    write(str(tmp_path / "scrivo-safe" / "tools" / "portal_probe.py"), "")
    found = safe_folder(root, "tools", needs="portal_probe.py")
    assert found == os.path.join(os.path.realpath(str(tmp_path)), "scrivo-safe", "tools")


def test_a_neighbour_that_is_not_there_is_refused_in_words_naming_the_file_and_the_variable(tmp_path, monkeypatch):
    root = tree(tmp_path)
    empty = tmp_path / "nothing_here"
    os.makedirs(str(empty))
    monkeypatch.setenv("SCRIVO_SAFE", str(empty))
    with pytest.raises(BoardError, match=r"portal_probe\.py.*SCRIVO_SAFE names .*nothing_here"):
        safe_folder(root, "tools", needs="portal_probe.py")


def test_a_copy_without_the_variable_is_told_to_set_it(tmp_path):
    copy, _ = copy_away(tmp_path)
    with pytest.raises(BoardError, match="SCRIVO_SAFE is not set"):
        safe_folder(copy, "tools", needs="portal_probe.py")


def test_device_gets_target_mcu_and_files_of_its_board(tmp_path):
    root = tree(tmp_path)
    env = resolve(root, "bench_s3")
    scrivo_safe = os.path.realpath(str(tmp_path / "scrivo-safe"))
    assert env["TARGET"] == "xtensa-esp32s3-espidf"
    assert env["MCU"] == "esp32s3"
    assert env["ESP_IDF_SYS_ROOT_CRATE"] == "bench_s3"
    assert env["ESP_IDF_SDKCONFIG_DEFAULTS"] == ";".join([
        os.path.join(scrivo_safe, "boards", "STRAGA_BASE_S3", "bootloader.sdkconfig"),
        os.path.join(os.path.realpath(root), "sdkconfig.defaults")])
    assert env["ESP_IDF_GLOB_BOARD_BASE"] == scrivo_safe
    assert env["ESP_IDF_GLOB_BOARD_TABLE"] == "boards/STRAGA_BASE_S3/partitions.csv"
    assert env["PARTITIONS"] == os.path.join(scrivo_safe, "boards", "STRAGA_BASE_S3", "partitions.csv")
    assert (env["FLASH_SIZE"], env["FLASH_FREQ"], env["FLASH_MODE"]) == ("8MB", "80m", "dio")


def test_a_board_that_has_options_of_its_own_gets_them_after_the_common_ones(tmp_path):
    root = tree(tmp_path)
    write(os.path.join(root, "boards", "STRAGA_BASE_S3", "sdkconfig.defaults"), "CONFIG_SPIRAM=y\n")
    env = resolve(root, "bench_s3")
    files = env["ESP_IDF_SDKCONFIG_DEFAULTS"].split(";")
    assert files[-2:] == [os.path.join(os.path.realpath(root), "sdkconfig.defaults"),
                          os.path.join(os.path.realpath(root), "boards", "STRAGA_BASE_S3", "sdkconfig.defaults")]


def _options_of(env):
    """What the build is handed for a device: the lines of every defaults file, in the order IDF reads them."""
    lines = []
    for path in env["ESP_IDF_SDKCONFIG_DEFAULTS"].split(";"):
        lines += [line.strip() for line in open(path)]
    return lines


@pytest.mark.parametrize("device", ["bench_s3", "burner"])
def test_a_board_with_psram_is_built_with_it_switched_on(device):
    assert "CONFIG_SPIRAM=y" in _options_of(resolve(REAL_ROOT, device))


def test_a_board_without_psram_is_not_built_with_it_switched_on():
    """The Dual's chip has none: an image that asks IDF for the RAM aborts at the start and falls back to the recovery image."""
    assert not [line for line in _options_of(resolve(REAL_ROOT, "bench_dualr3")) if line.startswith("CONFIG_SPIRAM")]


def test_the_s3_built_without_psram_is_handed_no_psram_option_and_the_rest_of_its_board_is_the_same():
    """The Dual's memory is the chip's alone, and the S3 on the bench stands in for it: the same chip family, the same
    flash and table, the RAM off. The variant is a build's property, not an edit of a shared file."""
    plain = resolve(REAL_ROOT, "bench_s3")
    bare = resolve(REAL_ROOT, "bench_s3", variant="no_psram")
    assert "CONFIG_SPIRAM=y" not in _options_of(bare)
    assert not [line for line in _options_of(bare) if line.startswith("CONFIG_SPIRAM") and line.endswith("=y")]
    assert "CONFIG_SPIRAM=y" in _options_of(plain)
    for key in ("TARGET", "MCU", "PARTITIONS", "FLASH_SIZE", "FLASH_FREQ", "FLASH_MODE", "ESP_IDF_GLOB_BOARD_TABLE"):
        assert bare[key] == plain[key]


def test_a_variant_the_board_has_no_file_for_is_refused_in_words_naming_the_file():
    with pytest.raises(BoardError, match="sdkconfig.no_psram"):
        resolve(REAL_ROOT, "bench_dualr3", variant="no_psram")


def test_a_variant_that_is_no_name_is_refused(tmp_path):
    root = tree(tmp_path)
    with pytest.raises(BoardError, match="variant"):
        resolve(root, "bench_s3", variant="../x")


@pytest.mark.parametrize("device, word", [("bench_s3", "1"), ("burner", "1"), ("bench_dualr3", "0")])
def test_the_build_is_told_whether_the_board_of_the_device_has_psram(device, word):
    assert resolve(REAL_ROOT, device)["STRAGA_PSRAM"] == word


def test_the_s3_built_without_its_psram_is_told_it_has_none():
    assert resolve(REAL_ROOT, "bench_s3", variant="no_psram")["STRAGA_PSRAM"] == "0"


@pytest.mark.parametrize("device, variant", [("bench_s3", None), ("burner", None), ("bench_dualr3", None), ("bench_s3", "no_psram")])
def test_what_the_build_is_told_of_the_psram_is_what_its_sdkconfig_asks_for(device, variant):
    """The size of the heap profile follows the word, the chip follows the options: two truths about one fact must not part."""
    env = resolve(REAL_ROOT, device, variant=variant)
    asked = "CONFIG_SPIRAM=y" in _options_of(env)
    assert (env["STRAGA_PSRAM"] == "1") == asked


def test_a_board_that_does_not_say_whether_it_has_psram_is_refused_in_words(tmp_path):
    root = tree(tmp_path, board_toml='target = "xtensa-esp32s3-espidf"\nmcu = "esp32s3"\nsafe = "boards/STRAGA_BASE_S3"\n')
    with pytest.raises(BoardError, match="psram"):
        resolve(root, "bench_s3")


@pytest.mark.parametrize("line", ['psram = "yes"', "psram = 1"])
def test_a_psram_that_is_no_true_or_false_is_refused(tmp_path, line):
    root = tree(tmp_path, board_toml='target = "xtensa-esp32s3-espidf"\nmcu = "esp32s3"\nsafe = "boards/STRAGA_BASE_S3"\n%s\n' % line)
    with pytest.raises(BoardError, match="psram"):
        resolve(root, "bench_s3")


def test_unknown_device_is_refused(tmp_path):
    root = tree(tmp_path)
    with pytest.raises(BoardError, match="no device 'gate'"):
        resolve(root, "gate")


def test_device_without_board_is_refused(tmp_path):
    root = tree(tmp_path, device_toml='[package]\nname = "bench_s3"\nversion = "0.1.0"\n')
    with pytest.raises(BoardError, match="bench_s3.*names no board"):
        resolve(root, "bench_s3")


def test_device_naming_a_board_that_is_not_here_is_refused(tmp_path):
    root = tree(tmp_path, device_toml=(
        '[package]\nname = "bench_s3"\n\n[package.metadata.rust_app]\nboard = "ESP32_NOPE"\n'))
    with pytest.raises(BoardError, match="ESP32_NOPE.*board.toml"):
        resolve(root, "bench_s3")


def test_board_without_target_is_refused(tmp_path):
    root = tree(tmp_path, board_toml='mcu = "esp32s3"\nsafe = "boards/STRAGA_BASE_S3"\n')
    with pytest.raises(BoardError, match="STRAGA_BASE_S3.*target"):
        resolve(root, "bench_s3")


def test_board_naming_its_folder_by_a_key_that_is_not_safe_is_refused(tmp_path):
    root = tree(tmp_path, board_toml='target = "xtensa-esp32s3-espidf"\nmcu = "esp32s3"\n'
                                     'a_device = "boards/STRAGA_BASE_S3"\n')
    with pytest.raises(BoardError, match="STRAGA_BASE_S3.*no safe"):
        resolve(root, "bench_s3")


def test_board_whose_sdkconfig_is_missing_is_refused(tmp_path):
    root = tree(tmp_path, sdkconfig=None)
    with pytest.raises(BoardError, match="bootloader.sdkconfig"):
        resolve(root, "bench_s3")


def test_board_whose_table_is_missing_is_refused(tmp_path):
    root = tree(tmp_path, table=None)
    with pytest.raises(BoardError, match="partitions.csv"):
        resolve(root, "bench_s3")


def test_sdkconfig_naming_another_table_is_refused(tmp_path):
    root = tree(tmp_path, sdkconfig=S3_SDKCONFIG.replace("boards/STRAGA_BASE_S3/", "boards/OTHER/"))
    with pytest.raises(BoardError, match="boards/OTHER/partitions.csv"):
        resolve(root, "bench_s3")


def test_sdkconfig_of_another_chip_is_refused(tmp_path):
    root = tree(tmp_path, sdkconfig=S3_SDKCONFIG.replace('"esp32s3"', '"esp32"'))
    with pytest.raises(BoardError, match="esp32.*esp32s3"):
        resolve(root, "bench_s3")


@pytest.mark.parametrize("device, target, mcu, size", [
    ("bench_s3", "xtensa-esp32s3-espidf", "esp32s3", "8MB"),
    ("bench_dualr3", "xtensa-esp32-espidf", "esp32", "4MB"),
    ("burner", "xtensa-esp32s3-espidf", "esp32s3", "8MB"),
])
def test_devices_of_this_project_resolve(device, target, mcu, size):
    env = resolve(REAL_ROOT, device)
    assert (env["TARGET"], env["MCU"], env["FLASH_SIZE"]) == (target, mcu, size)
    assert os.path.exists(env["PARTITIONS"])


def test_the_button_of_the_board_comes_from_its_board_yml(tmp_path):
    root = tree(tmp_path)
    write(str(tmp_path / "scrivo-safe" / "boards" / "STRAGA_BASE_S3" / "board.yml"),
          "# the board\ngesture:\n  pin: GPIO0\n  level: low\n\nstatus_led:\n  pin: GPIO48\n")
    env = resolve(root, "bench_s3")
    assert (env["STRAGA_GESTURE_PIN"], env["STRAGA_GESTURE_ACTIVE"]) == ("0", "0")


def test_a_button_pressed_high_is_active_one(tmp_path):
    root = tree(tmp_path)
    write(str(tmp_path / "scrivo-safe" / "boards" / "STRAGA_BASE_S3" / "board.yml"),
          "gesture:\n  pin: GPIO9\n  level: high\n")
    env = resolve(root, "bench_s3")
    assert (env["STRAGA_GESTURE_PIN"], env["STRAGA_GESTURE_ACTIVE"]) == ("9", "1")


def test_a_board_without_a_button_gives_no_pin(tmp_path):
    # the ways-down component refuses to build without it, so a device that uses it on such a board stops there
    root = tree(tmp_path)
    write(str(tmp_path / "scrivo-safe" / "boards" / "STRAGA_BASE_S3" / "board.yml"), "status_led:\n  pin: GPIO48\n")
    env = resolve(root, "bench_s3")
    assert "STRAGA_GESTURE_PIN" not in env and "STRAGA_GESTURE_ACTIVE" not in env


def test_a_button_field_of_another_form_is_refused(tmp_path):
    root = tree(tmp_path)
    write(str(tmp_path / "scrivo-safe" / "boards" / "STRAGA_BASE_S3" / "board.yml"), "gesture:\n  pin: 0\n  level: low\n")
    with pytest.raises(BoardError, match="gesture"):
        resolve(root, "bench_s3")


def test_the_s3_of_this_project_has_its_button_on_gpio0():
    env = resolve(REAL_ROOT, "bench_s3")
    assert (env["STRAGA_GESTURE_PIN"], env["STRAGA_GESTURE_ACTIVE"]) == ("0", "0")


SONOFF_LIKE_YML = """gesture:
  pin: GPIO0
  level: low

pins:
  GPIO0: button, input with pull-up
  GPIO13: status LED, output, lit on low        # a comment
  GPIO14: relay, output - L1 on the box
  GPIO33: switch input, pull-up - S1 on the box

status_led:
  platform: gpio
  pin: GPIO13
  inverted: true
"""

DEVICE_HEAD = '[package]\nname = "bench_s3"\nversion = "0.1.0"\n\n[package.metadata.rust_app]\nboard = "STRAGA_BASE_S3"\n'


def pinned(tmp_path, pins, board_yml=SONOFF_LIKE_YML):
    root = tree(tmp_path, device_toml=DEVICE_HEAD + "\n[package.metadata.rust_app.pins]\n" + pins)
    if board_yml is not None:
        write(str(tmp_path / "scrivo-safe" / "boards" / "STRAGA_BASE_S3" / "board.yml"), board_yml)
    return root


def test_the_pins_a_device_names_reach_its_build(tmp_path):
    root = pinned(tmp_path, 'relay1 = "GPIO14"\nswitch1 = "GPIO33"\n')
    env = resolve(root, "bench_s3")
    assert (env["STRAGA_PIN_RELAY1"], env["STRAGA_PIN_SWITCH1"]) == ("14", "33")


def test_a_device_naming_no_pins_gets_none(tmp_path):
    root = pinned(tmp_path, "")
    assert not [k for k in resolve(root, "bench_s3") if k.startswith("STRAGA_PIN_")]


def test_a_pin_the_board_does_not_bring_out_is_refused(tmp_path):
    root = pinned(tmp_path, 'relay1 = "GPIO27"\n')
    with pytest.raises(BoardError, match="relay1.*GPIO27.*not brought out"):
        resolve(root, "bench_s3")


def test_a_pin_of_the_button_is_refused(tmp_path):
    root = pinned(tmp_path, 'switch1 = "GPIO0"\n')
    with pytest.raises(BoardError, match="switch1.*GPIO0.*held by the system.*gesture"):
        resolve(root, "bench_s3")


def test_a_pin_of_the_status_led_is_refused(tmp_path):
    root = pinned(tmp_path, 'relay1 = "GPIO13"\n')
    with pytest.raises(BoardError, match="relay1.*GPIO13.*held by the system.*status_led"):
        resolve(root, "bench_s3")


def test_two_things_on_one_pin_are_refused(tmp_path):
    root = pinned(tmp_path, 'relay1 = "GPIO14"\nrelay2 = "GPIO14"\n')
    with pytest.raises(BoardError, match="GPIO14.*relay1.*relay2"):
        resolve(root, "bench_s3")


@pytest.mark.parametrize("pin", ['"14"', '"gpio14"', '"GPIO"', "14"])
def test_a_pin_of_another_form_is_refused(tmp_path, pin):
    root = pinned(tmp_path, "relay1 = %s\n" % pin)
    with pytest.raises(BoardError, match="relay1.*GPIO<n>"):
        resolve(root, "bench_s3")


@pytest.mark.parametrize("name", ['"Relay1"', '"relay-1"', '"1relay"'])
def test_a_name_that_is_no_identifier_is_refused(tmp_path, name):
    root = pinned(tmp_path, '%s = "GPIO14"\n' % name)
    with pytest.raises(BoardError, match="not lowercase"):
        resolve(root, "bench_s3")


def test_pins_on_a_board_without_board_yml_are_refused(tmp_path):
    root = pinned(tmp_path, 'relay1 = "GPIO14"\n', board_yml=None)
    with pytest.raises(BoardError, match="relay1.*board.yml"):
        resolve(root, "bench_s3")


def test_pins_on_a_board_yml_without_pins_are_refused(tmp_path):
    root = pinned(tmp_path, 'relay1 = "GPIO14"\n', board_yml="gesture:\n  pin: GPIO0\n  level: low\n")
    with pytest.raises(BoardError, match="relay1.*GPIO14.*not brought out"):
        resolve(root, "bench_s3")


def test_the_status_led_comes_from_its_board_yml(tmp_path):
    root = pinned(tmp_path, "")
    env = resolve(root, "bench_s3")
    assert (env["STRAGA_STATUS_LED_PIN"], env["STRAGA_STATUS_LED_LIT"]) == ("13", "0")


def test_a_status_led_lit_on_high_is_lit_one(tmp_path):
    root = pinned(tmp_path, "", board_yml="status_led:\n  pin: GPIO48\n")
    env = resolve(root, "bench_s3")
    assert (env["STRAGA_STATUS_LED_PIN"], env["STRAGA_STATUS_LED_LIT"]) == ("48", "1")


def test_a_status_led_field_of_another_form_is_refused(tmp_path):
    root = pinned(tmp_path, "", board_yml="status_led:\n  platform: gpio\n  inverted: true\n")
    with pytest.raises(BoardError, match="status_led"):
        resolve(root, "bench_s3")


def test_the_sonoff_of_this_project_gets_the_pins_of_its_box():
    env = resolve(REAL_ROOT, "bench_dualr3")
    assert {k: v for k, v in env.items() if k.startswith("STRAGA_PIN_")} == {
        "STRAGA_PIN_RELAY1": "14", "STRAGA_PIN_RELAY2": "27", "STRAGA_PIN_SWITCH1": "33", "STRAGA_PIN_SWITCH2": "32",
        "STRAGA_PIN_METER_TX": "25", "STRAGA_PIN_METER_RX": "26"}
    assert (env["STRAGA_STATUS_LED_PIN"], env["STRAGA_STATUS_LED_LIT"]) == ("13", "0")


def described(tmp_path, settings):
    return tree(tmp_path, device_toml=(
        '[package]\nname = "bench_s3"\n\n[package.metadata.rust_app]\nboard = "STRAGA_BASE_S3"\n\n'
        '[package.metadata.rust_app.settings]\n' + settings))


def test_the_settings_a_device_describes_reach_its_build_a_line_each(tmp_path):
    root = described(tmp_path, 'time = "internet"\nzone = "utc"\n')
    assert resolve(root, "bench_s3")["STRAGA_SETTINGS"] == "time=internet\nzone=utc"


def test_a_device_describing_no_settings_gets_an_empty_list(tmp_path):
    assert resolve(tree(tmp_path), "bench_s3")["STRAGA_SETTINGS"] == ""


@pytest.mark.parametrize("line", ['Zone = "utc"\n', 'keep-months = "12"\n', 'keep = 12\n', 'zone = "utc\\ntime=x"\n'])
def test_a_setting_that_is_no_name_or_no_single_line_of_text_is_refused(tmp_path, line):
    root = described(tmp_path, line)
    with pytest.raises(BoardError, match="setting"):
        resolve(root, "bench_s3")


def relaying(tmp_path, close_on):
    return tree(tmp_path, device_toml=DEVICE_HEAD + close_on)


@pytest.mark.parametrize("word, low", [("low", "1"), ("high", "0")])
def test_the_level_that_closes_the_relays_reaches_the_build(tmp_path, word, low):
    root = relaying(tmp_path, 'relays_close_on = "%s"\n' % word)
    assert resolve(root, "bench_s3")["STRAGA_RELAYS_CLOSE_ON_LOW"] == low


def test_a_device_that_names_no_level_of_its_relays_gets_none(tmp_path):
    assert "STRAGA_RELAYS_CLOSE_ON_LOW" not in resolve(tree(tmp_path), "bench_s3")


@pytest.mark.parametrize("line", ['relays_close_on = "Low"\n', 'relays_close_on = "inverted"\n',
                                  'relays_close_on = true\n', 'relays_close_on = 0\n'])
def test_a_level_of_the_relays_that_is_neither_low_nor_high_is_refused(tmp_path, line):
    root = relaying(tmp_path, line)
    with pytest.raises(BoardError, match="relays_close_on.*low or high"):
        resolve(root, "bench_s3")


def test_the_relays_of_the_sonoff_close_on_high():
    assert resolve(REAL_ROOT, "bench_dualr3")["STRAGA_RELAYS_CLOSE_ON_LOW"] == "0"


def test_the_burner_gets_its_relays_its_bus_and_its_thermocouple_on_the_pins_of_the_bench():
    # The Modbus bus took GPIO16 and GPIO17 (bench, 20.09.2026: the modules answer on no other pair), so aux_power
    # moved off 16 onto 21 - nothing is wired to the relays yet. The MAX6675 sits on GPIO38-40, where
    # the owner wired it; the garage burner had the same chip on 40/41/42, and that stays a fact about that board.
    # The phase dimmer took the free pair left after it, GPIO41 and GPIO42: the zero-crossing detector talks in,
    # the triac's gate listens out.
    env = resolve(REAL_ROOT, "burner")
    assert {k: v for k, v in env.items() if k.startswith("STRAGA_PIN_")} == {
        "STRAGA_PIN_AUGER": "13", "STRAGA_PIN_IGNITER": "14", "STRAGA_PIN_PUMP": "15",
        "STRAGA_PIN_AUX_POWER": "21", "STRAGA_PIN_BUS_TX": "17", "STRAGA_PIN_BUS_RX": "16",
        "STRAGA_PIN_FLUE_SCK": "38", "STRAGA_PIN_FLUE_CS": "39", "STRAGA_PIN_FLUE_SO": "40",
        "STRAGA_PIN_ZC": "41", "STRAGA_PIN_DIM": "42",
        # the two analog pins, named by burner.yaml and so checked against the board's board.yml
        "STRAGA_PIN_DRAUGHT_PIN": "4", "STRAGA_PIN_FLAME_PIN": "7"}
    assert env["STRAGA_RELAYS_CLOSE_ON_LOW"] == "1"


def burner_on_real_board(tmp_path, pins):
    """The burner's description with PINS, against the real board.yml of STRAGA_BASE_S3."""
    root = pinned(tmp_path, pins, board_yml=None)
    real = os.path.join(scrivo_safe(REAL_ROOT), "boards", "STRAGA_BASE_S3", "board.yml")
    write(str(tmp_path / "scrivo-safe" / "boards" / "STRAGA_BASE_S3" / "board.yml"), open(real).read())
    return root


def test_a_relay_of_the_burner_on_the_button_of_the_s3_is_refused_in_words(tmp_path):
    root = burner_on_real_board(tmp_path, 'auger = "GPIO0"\nigniter = "GPIO14"\n')
    with pytest.raises(BoardError, match="auger on GPIO0 - held by the system on board STRAGA_BASE_S3: gesture"):
        resolve(root, "bench_s3")


def test_a_relay_of_the_burner_on_the_led_of_the_s3_is_refused_in_words(tmp_path):
    root = burner_on_real_board(tmp_path, 'pump = "GPIO48"\n')
    with pytest.raises(BoardError, match="pump on GPIO48 - held by the system on board STRAGA_BASE_S3: status_led"):
        resolve(root, "bench_s3")


def test_the_flue_thermocouple_of_the_burner_reaches_its_build():
    """The three wires of the MAX6675 as the burner's own description names them.

    Its build.rs asks for all three by name, so a pin dropped from the description stops the build - but it stops
    it with a panic out of a build script, which nobody reads twice. Here the same loss is one red test.
    """
    env = resolve(REAL_ROOT, "burner")
    assert (env["STRAGA_PIN_FLUE_SCK"], env["STRAGA_PIN_FLUE_CS"], env["STRAGA_PIN_FLUE_SO"]) == ("38", "39", "40")


def test_the_wires_of_the_flue_are_not_shared_with_anything_else_of_the_burner():
    """The pair to it: three wires of their own, and no other part of this device on any of them."""
    env = resolve(REAL_ROOT, "burner")
    pins = [(name, value) for name, value in env.items() if name.startswith("STRAGA_PIN_")]
    taken = [value for name, value in pins]
    assert len(set(taken)) == len(taken), "two things of the burner sit on one pin: %s" % sorted(pins)


# ---- a device described by <device>.yaml: the modules it has, and the pins each of them is wired to ----

YAML_HEAD = "device: bench_s3\nboard: STRAGA_BASE_S3\n"


def yaml_device(tmp_path, text, board_yml=SONOFF_LIKE_YML, cargo=None):
    """A device whose description is devices/bench_s3/bench_s3.yaml: its Cargo.toml carries the package and no description."""
    root = tree(tmp_path, device_toml=cargo if cargo is not None else '[package]\nname = "bench_s3"\nversion = "0.1.0"\n')
    write(os.path.join(root, "devices", "bench_s3", "bench_s3.yaml"), text)
    if board_yml is not None:
        write(str(tmp_path / "scrivo-safe" / "boards" / "STRAGA_BASE_S3" / "board.yml"), board_yml)
    return root


def test_a_device_described_by_its_yaml_gets_its_board_its_pins_its_relay_level_and_its_settings(tmp_path):
    root = yaml_device(tmp_path, YAML_HEAD + "relays_close_on: low\nmodules:\n  relays:\n    relay1: GPIO14\n  inputs:\n    switch1: GPIO33\n"
                     "settings:\n  poll_every: '5'\n")
    env = resolve(root, "bench_s3")
    assert env["TARGET"] == "xtensa-esp32s3-espidf"
    assert (env["STRAGA_PIN_RELAY1"], env["STRAGA_PIN_SWITCH1"]) == ("14", "33")
    assert env["STRAGA_RELAYS_CLOSE_ON_LOW"] == "1"
    assert env["STRAGA_SETTINGS"] == "poll_every=5"


def test_a_module_with_no_pins_is_a_module_and_asks_for_none(tmp_path):
    env = resolve(yaml_device(tmp_path, YAML_HEAD + "modules:\n  automaton: {}\n"), "bench_s3")
    assert not [k for k in env if k.startswith("STRAGA_PIN_")]


def test_a_pin_in_a_yaml_the_board_does_not_bring_out_is_refused_in_words(tmp_path):
    root = yaml_device(tmp_path, YAML_HEAD + "modules:\n  relays:\n    relay1: GPIO5\n")
    with pytest.raises(BoardError, match="relay1 on GPIO5 - not brought out on board STRAGA_BASE_S3"):
        resolve(root, "bench_s3")


def test_a_pin_name_two_modules_both_use_is_refused(tmp_path):
    root = yaml_device(tmp_path, YAML_HEAD + "modules:\n  relays:\n    pin: GPIO14\n  inputs:\n    pin: GPIO33\n")
    with pytest.raises(BoardError, match="pin.*relays.*inputs"):
        resolve(root, "bench_s3")


def test_a_device_described_in_its_cargo_toml_and_its_yaml_is_refused(tmp_path):
    root = yaml_device(tmp_path, YAML_HEAD, cargo=DEVICE_HEAD)
    with pytest.raises(BoardError, match="described twice"):
        resolve(root, "bench_s3")


def test_a_yaml_that_names_no_board_is_refused(tmp_path):
    with pytest.raises(BoardError, match=r"bench_s3\.yaml names no board"):
        resolve(yaml_device(tmp_path, "device: bench_s3\nmodules: {}\n"), "bench_s3")


def test_a_yaml_that_names_another_device_is_refused(tmp_path):
    with pytest.raises(BoardError, match="device: burner"):
        resolve(yaml_device(tmp_path, "device: burner\nboard: STRAGA_BASE_S3\n"), "bench_s3")


# ---- the name a device gives itself on the network is its description's, not the code's ----

def test_the_name_of_a_device_reaches_its_build_as_its_description_names_it(tmp_path):
    env = resolve(yaml_device(tmp_path, YAML_HEAD + "modules:\n  automaton: {}\n"), "bench_s3")
    assert env["STRAGA_DEVICE"] == "bench_s3"


def test_the_burner_is_called_burner_in_its_build():
    assert resolve(REAL_ROOT, "burner")["STRAGA_DEVICE"] == "burner"


# --- the word of the board and the options of the build must agree ------------------------------------

def _board_toml(psram):
    return ('target = "xtensa-esp32s3-espidf"\nmcu = "esp32s3"\nsafe = "boards/STRAGA_BASE_S3"\npsram = %s\n'
            % ("true" if psram else "false"))


def test_a_board_with_psram_whose_options_ask_for_it_is_built(tmp_path):
    root = tree(tmp_path, board_toml=_board_toml(True), common="CONFIG_SPIRAM=y\n")
    assert resolve(root, "bench_s3")["STRAGA_PSRAM"] == "1"


def test_a_board_without_psram_whose_options_do_not_ask_for_it_is_built(tmp_path):
    root = tree(tmp_path, board_toml=_board_toml(False), common="CONFIG_SPIRAM_MODE_QUAD=y\n")
    assert resolve(root, "bench_s3")["STRAGA_PSRAM"] == "0"


def test_a_board_with_psram_that_no_option_asks_for_is_refused_in_words_saying_what_where_and_how_to_mend(tmp_path):
    """The big profile is for a heap with the RAM behind it; an image built without the RAM would take it and fall short."""
    root = tree(tmp_path, board_toml=_board_toml(True), common="CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192\n")
    with pytest.raises(BoardError) as why:
        resolve(root, "bench_s3")
    text = str(why.value)
    assert "psram = true" in text and "CONFIG_SPIRAM=y" in text and "STRAGA_BASE_S3" in text and "sdkconfig" in text


def test_a_board_without_psram_whose_options_ask_for_it_is_refused_naming_the_file_and_the_line(tmp_path):
    """An image that asks IDF for RAM the chip lacks aborts at the start and falls back to the recovery image."""
    root = tree(tmp_path, board_toml=_board_toml(False), common="CONFIG_SPIRAM=y\n")  # after the helper's own first line
    with pytest.raises(BoardError) as why:
        resolve(root, "bench_s3")
    text = str(why.value)
    assert "psram = false" in text and "CONFIG_SPIRAM=y" in text
    assert os.path.join(os.path.realpath(root), "sdkconfig.defaults") in text and "line 2" in text


def test_the_options_of_the_board_itself_are_read_after_the_common_ones_and_have_the_last_word(tmp_path):
    root = tree(tmp_path, board_toml=_board_toml(False), common="CONFIG_SPIRAM=y\n")
    write(os.path.join(root, "boards", "STRAGA_BASE_S3", "sdkconfig.defaults"), "# CONFIG_SPIRAM is not set\n")
    assert resolve(root, "bench_s3")["STRAGA_PSRAM"] == "0"


def test_a_board_with_psram_whose_own_options_switch_it_off_after_the_common_ones_is_refused(tmp_path):
    root = tree(tmp_path, board_toml=_board_toml(True), common="CONFIG_SPIRAM=y\n")
    write(os.path.join(root, "boards", "STRAGA_BASE_S3", "sdkconfig.defaults"), "# CONFIG_SPIRAM is not set\n")
    with pytest.raises(BoardError, match="CONFIG_SPIRAM"):
        resolve(root, "bench_s3")


def test_the_variant_without_psram_of_a_board_with_psram_is_built_when_its_options_switch_the_ram_off(tmp_path):
    root = tree(tmp_path, board_toml=_board_toml(True), common="CONFIG_SPIRAM=y\n")
    write(os.path.join(root, "boards", "STRAGA_BASE_S3", "sdkconfig.no_psram"), "# CONFIG_SPIRAM is not set\n")
    assert resolve(root, "bench_s3", variant="no_psram")["STRAGA_PSRAM"] == "0"


def test_the_variant_without_psram_whose_options_still_ask_for_the_ram_is_refused_naming_the_variant_file(tmp_path):
    root = tree(tmp_path, board_toml=_board_toml(True), common="CONFIG_SPIRAM=y\n")
    write(os.path.join(root, "boards", "STRAGA_BASE_S3", "sdkconfig.no_psram"), "CONFIG_SPIRAM=y\n")
    with pytest.raises(BoardError) as why:
        resolve(root, "bench_s3", variant="no_psram")
    assert "sdkconfig.no_psram" in str(why.value) and "no_psram" in str(why.value)


# --- a device that carries another's modules is called by its own name and board -----------------------

@pytest.mark.parametrize("device, board", [("bench_dualr3", "SONOFF_DUAL_R3"), ("bench_dualr3s3", "STRAGA_BASE_S3"),
                                           ("bench_s3", "STRAGA_BASE_S3"), ("burner", "STRAGA_BASE_S3")])
def test_the_build_is_told_the_board_of_the_device_it_builds(device, board):
    """The Dual's modules run on the bench S3 as the stand: its dictionary says the board it stands on, not the Dual's."""
    assert resolve(REAL_ROOT, device)["STRAGA_BOARD"] == board


def test_the_stand_of_the_dual_and_the_dual_are_called_each_by_its_name():
    assert resolve(REAL_ROOT, "bench_dualr3")["STRAGA_DEVICE"] == "bench_dualr3"
    assert resolve(REAL_ROOT, "bench_dualr3s3")["STRAGA_DEVICE"] == "bench_dualr3s3"
