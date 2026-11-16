//! What the lanes of the unit say: the most and the mean of the time a round holds the shared lock, the most it waits for
//! it and the most its step takes - since the start and within the last minute. Expected from the numbers fed in.

use std::cell::{Cell, RefCell};

use analog::gauge::{line, through, Lane, Report};

const MINUTE_MS: u64 = 60_000;

#[test]
fn the_most_and_the_mean_of_what_a_round_held_are_said() {
    let lane = Lane::new();
    for us in [10_000, 20_000, 15_000] {
        lane.held(us, 1_000);
    }
    let held = lane.report(1_000).held;
    assert_eq!(held.max_us, 20_000);
    assert_eq!(held.mean_us, Some(15_000));
}

#[test]
fn a_lane_that_measured_nothing_has_no_mean_and_zero_for_the_most() {
    let report = Lane::new().report(0);
    assert_eq!(report, Report::default());
    assert_eq!(report.held.mean_us, None);
}

#[test]
fn the_wait_and_the_step_keep_their_most_apart_from_the_hold() {
    let lane = Lane::new();
    lane.waited(3_000, 10);
    lane.waited(1_000, 20);
    lane.step(130_000, 30);
    lane.held(21_000, 40);
    let report = lane.report(50);
    assert_eq!((report.waited.max_us, report.step.max_us, report.held.max_us), (3_000, 130_000, 21_000));
}

#[test]
fn what_left_the_last_minute_stays_in_the_most_since_start_and_goes_from_the_minute() {
    let lane = Lane::new();
    lane.waited(500_000, 0);
    lane.waited(2_000, 40_000);
    let inside = lane.report(MINUTE_MS - 1).waited;
    assert_eq!((inside.max_us, inside.minute_max_us), (500_000, 500_000), "49 seconds on: still in the minute");
    let outside = lane.report(MINUTE_MS + 20_000).waited;
    assert_eq!(outside.max_us, 500_000, "the most since start does not forget");
    assert_eq!(outside.minute_max_us, 2_000, "the minute holds what came after the big one");
}

#[test]
fn a_minute_of_quiet_empties_the_minute_and_not_the_most_since_start() {
    let lane = Lane::new();
    lane.step(900_000, 5_000);
    let later = lane.report(5_000 + 2 * MINUTE_MS).step;
    assert_eq!((later.max_us, later.minute_max_us), (900_000, 0));
}

#[test]
fn two_lanes_do_not_share_a_number() {
    let (draught, flame) = (Lane::new(), Lane::new());
    draught.waited(8_000, 100);
    flame.waited(100, 100);
    assert_eq!(draught.report(100).waited.max_us, 8_000);
    assert_eq!(flame.report(100).waited.max_us, 100);
}

#[test]
fn the_mean_survives_days_of_holds_without_wrapping() {
    let lane = Lane::new();
    // a million holds of 20 ms is 20 000 s of them: more than a 32-bit sum of microseconds takes
    for n in 0..1_000_000u64 {
        lane.held(20_000, n);
    }
    assert_eq!(lane.report(1_000_000).held.mean_us, Some(20_000));
}

#[test]
fn the_lock_is_held_for_the_work_and_two_reads_of_the_clock_and_the_numbers_are_taken_after_it_is_given_up() {
    let events = RefCell::new(Vec::new());
    let now = Cell::new(1_000_000u64);
    let clock = || {
        events.borrow_mut().push("clock");
        now.get()
    };
    struct Guard<'a>(&'a RefCell<Vec<&'static str>>);
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            self.0.borrow_mut().push("drop");
        }
    }
    let lane = Lane::new();
    let out = through(
        &lane,
        clock,
        || {
            events.borrow_mut().push("lock");
            now.set(now.get() + 7_000);
            Guard(&events)
        },
        || {
            events.borrow_mut().push("work");
            now.set(now.get() + 12_000);
            42
        },
    );
    assert_eq!(out, 42, "what the work gave comes back");
    assert_eq!(*events.borrow(), ["clock", "lock", "clock", "work", "clock", "drop"], "nothing but the work and two reads of the clock between the lock and its giving up");
    let report = lane.report(now.get() / 1000);
    assert_eq!((report.waited.max_us, report.held.max_us), (7_000, 12_000));
}

#[test]
fn a_lane_is_said_in_numbers_with_units_and_the_name_of_the_round() {
    let lane = Lane::new();
    lane.held(21_400, 10);
    lane.held(7_000, 20);
    lane.waited(40_000, 30);
    lane.step(130_000, 40);
    let said = line("draught", &lane.report(50));
    assert_eq!(
        said,
        "draught: held max 21400 us mean 14200 us, minute max 21400 us; waited max 40000 us, minute max 40000 us; step max 130000 us, minute max 130000 us"
    );
}
