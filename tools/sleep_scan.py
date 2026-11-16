"""Every place the firmware sleeps or waits by a clock, against the list of those judged harmless at a leap of the chip's clock.

    uv run --no-project python tools/sleep_scan.py [--all]

`std::thread::sleep` ends by the chip's clock (esp_timer), not by the tick: IDF's `usleep` loops `vTaskDelay` until that clock has reached the end of the
span, so a leap of it back lengthens the sleep by the leap (between::nap says it). A sleep of the firmware either goes through `between::nap`
or is on the list below with the reason it harms nobody. A site neither on the list nor through the nap stops the scan (exit 1), and so does a
line of the list that no longer matches a site.

The waits of embassy-time (`Timer::after`, `Timer::at`, `with_timeout`, `with_deadline`, the desk's `serve_for`) are not silent either: they are named as covered, because the
board's time driver reads the steady clock (between::steady_time) and its beat sleeps by the tick (between::nap), so a leap of the chip's clock is not in them (10.5, 11.28).
A wait on a notification with no deadline (`wait_any()`) or a look (`wait(0)`) has no clock in it; one with a deadline is a site to be judged.

What it does not see: sleeps inside IDF and inside the crates we do not carry (Wi-Fi, lwIP, drivers), and any wait written under another name than the ones in WORDS and EMBASSY. Tests and examples are not firmware.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
WORDS = re.compile(r"thread::sleep|\busleep\b|FreeRtos::|vTaskDelay|esp_rom_delay_us|Ets::delay|recv_timeout|wait_timeout|park_timeout|xQueueReceive|xSemaphoreTake|TickType::|\.wait\((?!0\))[^)]")
# the waits that need no verdict of their own, with what covers them
COVERED = (
    (re.compile(r"Timer::after|Timer::at\b|with_timeout|with_deadline|Ticker::|\bserved?_for\b"), "embassy"),
    (re.compile(r"\.wait\(0\)|\.wait_any\(\)"), "no clock"),
)

# (file, text of the site) -> why a leap of the chip's clock harms nobody there
HARMLESS = {
    ("devices/burner/src/modules/temperatures.rs", "std::thread::sleep, |line|"): "the pause of module::retry between two tries to start the round: after a start that failed, 2 s, no lock held",
    ("devices/burner/src/hardware/thermocouple.rs", "std::thread::sleep(CONVERSION);"): "once at the chip's start, 250 ms; the first reading is later by the leap",
    ("crates/firmware/src/net.rs", "std::thread::sleep(SCAN_CONNECT_GAP);"): "a pause of the network's task between a scan and a connect; nobody waits on it",
    ("crates/firmware/src/net.rs", "std::thread::sleep(RETRY_PAUSE);"): "a pause of the network's task between laps; nobody waits on it",
    ("crates/firmware/src/net.rs", "std::thread::sleep(Duration::from_millis(1000));"): "before esp_restart: the restart is later by the leap",
    ("crates/firmware/src/net.rs", "std::thread::sleep(Duration::from_secs(1));"): "after esp_restart, which does not return",
    ("crates/firmware/src/fell.rs", "std::thread::sleep(std::time::Duration::from_millis(200));"): "before a fall made on purpose, from the shell",
    ("crates/firmware/src/lib.rs", "std::thread::sleep(std::time::Duration::from_secs(3600));"): "main's idle, nothing runs after it",
    ("crates/firmware/src/board.rs", "std::thread::sleep(std::time::Duration::from_millis(200));"): "before esp_restart asked from the shell",
    ("crates/firmware/src/console.rs", "std::thread::sleep(Duration::from_secs(1));"): "the USB console waits for a seat; it holds nothing",
    ("crates/firmware/src/console.rs", "std::thread::sleep(Duration::from_millis(20));"): "the USB console's poll of an empty line; it holds nothing",
    ("crates/firmware/src/flash_owner.rs", "std::thread::sleep(Duration::from_millis(200));"): "before esp_restart of the planned restart",
    ("devices/bench_s3/src/site.rs", "std::thread::sleep(std::time::Duration::from_millis(7000));"): "the bench device's own probe, not the burner's",
    ("devices/bench_dualr3/src/wall.rs", "std::thread::sleep(switch::DEBOUNCE);"): "the bench device's debounce, not the burner's",
    ("devices/bench_dualr3/src/meter.rs", "std::thread::sleep(Duration::from_millis(1));"): "the bench device's meter setup, not the burner's",
    ("devices/bench_dualr3/src/meter.rs", "std::thread::sleep(EVERY.saturating_sub(asked.elapsed()));"): "the bench device's meter period; a leap makes one period longer",
    ("crates/firmware/src/net.rs", "sntp.wait(LINK_CHECK);"): "firmware::clock::Sntp::wait: a with_timeout of embassy-time, through between::steady_time",
    ("devices/burner/src/hardware/rs485.rs", "TickType::new_millis(100)"): "the UART driver's wait by ticks; no clock of the chip in it",
    ("devices/burner/src/hardware/rs485.rs", "TickType::new_millis(left"): "the UART driver's wait by ticks; the span of the asking is taken by the steady clock (between::clock)",
    ("devices/bench_dualr3/src/meter.rs", "TickType::from(left)"): "the bench device's UART wait by ticks; the span is taken by the steady clock",
    ("crates/between/src/nap.rs", "vTaskDelay(ticks)"): "the nap itself: the tick, no clock of the chip",
    ("crates/between/src/nap.rs", "std::thread::sleep(span);"): "the computer's side of the nap: there is no chip clock to leap",
    ("crates/between/src/steps.rs", "FreeRtos::delay_ms(1);"): "vTaskDelay of one tick: no clock of the chip in it",
    ("crates/between/src/steps.rs", "std::thread::sleep(core::time::Duration::from_millis(1));"): "the computer's side of the step: no chip clock",
    ("crates/firmware/mdns/mdns_networking_socket.c", "vTaskDelay"): "a copy of IDF's mdns: the tick only",
    ("crates/firmware/mdns/mdns.c", "vTaskDelay"): "a copy of IDF's mdns: the tick only",
    ("crates/firmware/mdns/mdns.c", "xQueueReceive"): "a wait on a queue by ticks: the tick only",
    ("crates/firmware/mdns/mdns.c", "xSemaphoreTake"): "a wait on a semaphore by ticks: the tick only",
    ("crates/firmware/mdns/private_include/mdns_private.h", "xSemaphoreTake"): "a wait on a semaphore by ticks: the tick only",
    ("crates/ways_down/ways_down/gesture.c", "vTaskDelay"): "the button's poll by the tick; the hold is counted by esp_timer_get_time and a leap forward could read as a long hold - the recovery image asks again by the gesture's own rules",
}


def sites():
    out = []
    for folder in ("crates", "devices"):
        for path in sorted((ROOT / folder).rglob("*")):
            rel = path.relative_to(ROOT).as_posix()
            if path.suffix not in (".rs", ".c", ".h") or "/tests/" in rel or "/examples/" in rel or "/target/" in rel or "/build/" in rel:
                continue
            for n, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
                stripped = line.strip()
                if stripped.startswith(("//", "*", "/*", "use ")):
                    continue
                if WORDS.search(line):
                    out.append((rel, n, stripped, None))
                else:
                    for pattern, kind in COVERED:
                        if pattern.search(line):
                            out.append((rel, n, stripped, kind))
                            break
    return out


def judged(rel, text):
    """The HARMLESS key a site answers to, or None."""
    for (file, snippet) in HARMLESS:
        if file == rel and snippet in text:
            return (file, snippet)
    return None


def main():
    seen, stray, covered = set(), [], {"embassy": 0, "no clock": 0}
    for rel, n, text, kind in sites():
        if kind:
            covered[kind] += 1
            if "--all" in sys.argv:
                how = "through between::steady_time (10.5, 11.28)" if kind == "embassy" else "no clock in it: a look or a wait with no deadline"
                print(f"covered   {rel}:{n}  {text}\n            {how}")
            continue
        key = judged(rel, text)
        if key:
            seen.add(key)
            if "--all" in sys.argv:
                print(f"harmless  {rel}:{n}  {text}\n            {HARMLESS[key]}")
        else:
            stray.append((rel, n, text))
    stale = [key for key in HARMLESS if key not in seen]
    for rel, n, text in stray:
        print(f"UNJUDGED  {rel}:{n}  {text}")
    for file, snippet in stale:
        print(f"STALE     {file}  {snippet}")
    print(
        f"{covered['embassy']} waits through embassy-time covered by between::steady_time (10.5, 11.28), {covered['no clock']} waits with no clock in them, "
        f"{len(seen)} harmless lines of the list met, {len(stray)} sites unjudged, {len(stale)} lines of the list without a site"
    )
    return 1 if stray or stale else 0


if __name__ == "__main__":
    sys.exit(main())
