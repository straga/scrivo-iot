// Measures one press of a button from the levels the gesture task samples; no ESP-IDF here, so the host tests it.
#ifndef STRAGA_GESTURE_HOLD_H
#define STRAGA_GESTURE_HOLD_H

#include <stdbool.h>
#include <stdint.h>

typedef struct {
    uint32_t hold_ms;
    int active;
    bool pressed;
    uint32_t pressed_at;
} gesture_hold_t;

void gesture_hold_init(gesture_hold_t *h, uint32_t hold_ms, int active);

// Takes the level sampled at now_ms (a millisecond clock that may wrap); returns how long the button was held when
// this sample is the release that ends a hold of at least hold_ms, otherwise 0.
uint32_t gesture_hold_step(gesture_hold_t *h, int level, uint32_t now_ms);

#endif
