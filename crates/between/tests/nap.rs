//! How many ticks of the board a nap is: at least as long as asked, never none for something asked.

use core::time::Duration;

use between::nap::ticks;

#[test]
fn a_nap_is_asked_for_a_tick_more_than_its_span_so_that_it_is_never_shorter() {
    // vTaskDelay(n) wakes between n-1 and n ticks after the call: the span asked for is the least it may last
    assert_eq!(ticks(Duration::from_millis(1), 1000), 2);
    assert_eq!(ticks(Duration::from_millis(100), 1000), 101);
    assert_eq!(ticks(Duration::from_millis(1), 100), 2);
    assert_eq!(ticks(Duration::from_millis(25), 100), 4);
}

#[test]
fn a_span_that_is_not_whole_ticks_is_rounded_up() {
    assert_eq!(ticks(Duration::from_micros(1_500), 1000), 3);
    assert_eq!(ticks(Duration::from_micros(1), 1000), 2);
}

#[test]
fn nothing_asked_for_is_no_nap_and_a_huge_span_does_not_wrap() {
    assert_eq!(ticks(Duration::ZERO, 1000), 0);
    assert_eq!(ticks(Duration::from_secs(u64::MAX / 2), 1000), u32::MAX);
}
