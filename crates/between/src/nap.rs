//! The board's sleep: a task asleep by the TICK of FreeRTOS, and by nothing else.
//!
//! `std::thread::sleep` is not that. On the chip it is `usleep`, and from one tick up IDF's `usleep` calls `vTaskDelay` in a loop until
//! the chip's clock (esp_timer) has reached the end of the span (esp-idf components/newlib/src/time.c, `usleep`). A leap of that clock
//! back by L, which the board's power loads make, puts the end L further off, and the sleep that was running lasts L longer: 696 ms
//! on the burner, 20.10, with the lock of the ADC in the sleeper's hands. A sleep by the tick does not read that clock; a leap of the
//! tick itself is `systick`'s, and costs the watcher's 20 ms or so.
//!
//! Whoever sleeps on the board for a span that matters - holding a lock, or the beat of the timers - sleeps here.

use core::time::Duration;

/// The ticks that make a nap of at least SPAN, at HZ ticks a second: SPAN rounded up, and one more, because `vTaskDelay(n)` wakes between
/// n-1 and n ticks after the call (the tick's interrupt is not in step with the call). Nothing asked for is no tick; anything else is two at the least.
pub fn ticks(span: Duration, hz: u32) -> u32 {
    if span.is_zero() {
        return 0;
    }
    let ns = span.as_nanos();
    let tick_ns = 1_000_000_000u128 / u128::from(hz);
    let whole = ns.div_ceil(tick_ns) + 1;
    u32::try_from(whole).unwrap_or(u32::MAX)
}

/// Sleeps for at least SPAN by the tick of FreeRTOS.
#[cfg(target_os = "espidf")]
pub fn nap(span: Duration) {
    let ticks = ticks(span, esp_idf_svc::sys::CONFIG_FREERTOS_HZ);
    if ticks > 0 {
        unsafe { esp_idf_svc::sys::vTaskDelay(ticks) };
    }
}

/// On the computer there is no tick but the thread's own sleep, and no chip clock to leap.
#[cfg(not(target_os = "espidf"))]
pub fn nap(span: Duration) {
    std::thread::sleep(span);
}
