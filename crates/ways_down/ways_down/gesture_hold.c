#include "gesture_hold.h"

void gesture_hold_init(gesture_hold_t *h, uint32_t hold_ms, int active) {
    h->hold_ms = hold_ms;
    h->active = active;
    h->pressed = false;
    h->pressed_at = 0;
}

uint32_t gesture_hold_step(gesture_hold_t *h, int level, uint32_t now_ms) {
    if (level == h->active) {
        if (!h->pressed) {
            h->pressed = true;
            h->pressed_at = now_ms;
        }
        return 0;
    }
    if (!h->pressed) {
        return 0;
    }
    h->pressed = false;
    // unsigned subtraction keeps the duration right across a wrap of the clock
    uint32_t held = now_ms - h->pressed_at;
    return held >= h->hold_ms ? held : 0;
}
