Taken from espressif/mdns 1.3.2 (esp-protocols, components/mdns, commit 5d69d3f0ac34da638e8cb99eacb19d5b862afa8b; Apache-2.0).
Left out: examples, tests, idf_component.yml (the component is local, not fetched).

Ours:
- `mdns.c`, `_mdns_start_timer`: `.skip_unhandled_events = true`. The 100 ms timer is periodic and has nothing to catch up on;
  without the flag `timer_process_alarm` (esp_timer.c) calls it once per missed period after a leap of the counter forward,
  and the board falls by the task watchdog (05.10, 10:17 and 10:28).

The version stays at 1.3.2: newer ones use more resources (the recovery image and the MicroPython port hold the same one).
