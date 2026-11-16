// Counts short starts in a row - power, the reset line, a software reset - from the value stored in nvs; no ESP-IDF
// here, so the host tests it. Which reset reasons count is decided by the caller from esp_reset_reason().
#ifndef STRAGA_RESET_COUNT_H
#define STRAGA_RESET_COUNT_H

#include <stdbool.h>
#include <stdint.h>

typedef struct {
    int32_t starts;     // short starts in a row, this one included when it counts
    bool write;         // store must be written to nvs
    int32_t store;      // the value to write
    bool to_factory;    // this start ends the row: switch boot to factory
} reset_count_start_t;

// stored: the value read from nvs, 0 when the key is absent; a value below 0 or at the limit and above is not a count
// and reads as 0. counted: this start came from power, the reset line or software, not from a watchdog or a panic.
reset_count_start_t reset_count_on_start(int32_t stored, bool counted, int32_t limit);

// The firmware has stayed up past the alive time with stored in nvs: true when 0 must be written, so starts spread
// over minutes or days never add up.
bool reset_count_on_alive(int32_t stored);

#endif
