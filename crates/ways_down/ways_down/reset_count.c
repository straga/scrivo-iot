#include "reset_count.h"

// A stored value outside 0..limit-1 was never written by the count - the start that reaches the limit writes 0 - so
// it is not a count: flash garbage or another key's leftover must not send the board down.
static int32_t stored_count(int32_t stored, int32_t limit) {
    return stored >= 0 && stored < limit ? stored : 0;
}

reset_count_start_t reset_count_on_start(int32_t stored, bool counted, int32_t limit) {
    reset_count_start_t r = {stored_count(stored, limit), false, 0, false};
    if (!counted) {
        return r;
    }
    r.starts += 1;
    r.write = true;
    if (r.starts >= limit) {
        // cleared before leaving, so the first start after coming back from factory counts from one again
        r.store = 0;
        r.to_factory = true;
    } else {
        r.store = r.starts;
    }
    return r;
}

bool reset_count_on_alive(int32_t stored) {
    return stored != 0;
}
