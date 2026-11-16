// The tick of FreeRTOS on the systimer, set up by us in place of ESP-IDF's.
//
// IDF's tick handler catches up with every tick the systimer's counter says it missed, in the interrupt. On the burner
// under fire the counter flipped two bits at once (-2^25-2^23 and +2^37+2^35 of its ticks, 26.09 15:24:50, JTAG): the
// handler set about millions of ticks, both cores stood in the interrupt for seconds, and the second stage of the task
// watchdog reset the board without a note. Ours takes a leap of the counter for what it is: it resyncs to the counter,
// handles one tick, and counts the leap.
#pragma once

#include <stdint.h>

// The leaps of the tick's counter so far, on both cores: how many, and the largest back and forward in ticks (a tick
// is a millisecond at CONFIG_FREERTOS_HZ 1000).
typedef struct {
    uint32_t count;
    uint32_t largest_back;
    uint32_t largest_forward;
    // times the watcher found a core without a tick and set its alarm again (systick_watch)
    uint32_t rearms;
    // the last leap on either core: its size in ticks, and 1 when it went back
    uint32_t last_ticks;
    uint32_t last_was_back;
} systick_leaps_t;

systick_leaps_t systick_leaps(void);

// Starts the watcher of the tick: a timer group's timer, every 10 ms, that sets the alarm of a core again when the core
// has had no tick for two of its looks - after a leap of the counter back, no tick comes to do it. Call once, from a
// task, after the scheduler has started; 0 (ESP_OK) or the driver's error.
int systick_watch(void);
