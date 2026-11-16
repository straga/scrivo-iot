//! How busy each core is, counted from what the kernel keeps for its idle tasks (`core_load` does the arithmetic).
//!
//! The kernel counts the time every task has run (CONFIG_FREERTOS_GENERATE_RUN_TIME_STATS, by esp_timer); a core that has run its
//! idle task for all of a window was not busy at all. **IT IS COUNTED IN A TASK THAT IS AWAKE ALREADY** - the flash's owner, once a
//! pass - and not in one of its own: a task costs a stack out of the internal heap, and a count of a few words needs none.

use std::sync::Mutex;

use esp_idf_svc::sys;

static LOAD: Mutex<core_load::Load> = Mutex::new(core_load::Load::new());

/// Reads the clock and the idle tasks' counters, once a pass of the flash's owner.
pub(crate) fn pass() {
    let reading = core_load::Reading {
        // the same timer the run time counters are made by; 32 bits of microseconds, which go round as theirs do
        wall_us: unsafe { sys::esp_timer_get_time() } as u32,
        idle_us: [0, 1].map(|core| unsafe { sys::ulTaskGetIdleRunTimeCounterForCore(core) }),
    };
    LOAD.lock().unwrap().feed(reading);
}

/// The load of each core over the last window, in per cent; None before the first window is over.
pub fn percents() -> [Option<u8>; core_load::CORES] {
    LOAD.lock().unwrap().percents()
}
