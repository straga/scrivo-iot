//! How busy each core is, from what a person can read off a clock and the idle tasks: a core that spent 4.5 s of a 5 s
//! window in its idle task was busy a tenth of it.

use core_load::{Load, Reading, WINDOW_US};

const S: u32 = 1_000_000;

fn at(wall: u32, idle0: u32, idle1: u32) -> Reading {
    Reading { wall_us: wall, idle_us: [idle0, idle1] }
}

#[test]
fn a_window_of_five_seconds_gives_each_core_the_share_it_was_not_idle() {
    let mut load = Load::new();
    load.feed(at(0, 0, 0));
    // core 0 idle 4.5 s of the 5, core 1 not idle at all
    load.feed(at(WINDOW_US, 4_500_000, 0));
    assert_eq!(load.percents(), [Some(10), Some(100)]);
}

#[test]
fn one_core_busy_and_the_other_free_are_two_numbers_and_not_their_average() {
    let mut load = Load::new();
    load.feed(at(0, 0, 0));
    load.feed(at(5 * S, 250_000, 4_750_000));
    // the mean would say 50: the busy core would be hidden by the free one
    assert_eq!(load.percents(), [Some(95), Some(5)]);
}

#[test]
fn nothing_is_said_before_the_first_window_is_over() {
    let mut load = Load::new();
    assert_eq!(load.percents(), [None, None]);
    load.feed(at(100, 0, 0));
    assert_eq!(load.percents(), [None, None], "one reading has nothing to be compared with");
    load.feed(at(100 + 4 * S, 3 * S, 3 * S));
    assert_eq!(load.percents(), [None, None], "four seconds is not a window yet");
}

#[test]
fn a_reading_inside_the_window_does_not_move_it_and_the_window_is_counted_from_its_start() {
    let mut load = Load::new();
    load.feed(at(0, 0, 0));
    load.feed(at(S, S, S));
    load.feed(at(5 * S, 4 * S, 5 * S));
    // idle 4 s of 5 on core 0, not the last second's: 20 % busy
    assert_eq!(load.percents(), [Some(20), Some(0)]);
}

#[test]
fn the_counters_going_round_the_end_of_their_range_are_read_as_going_on() {
    // both are microseconds in 32 bits, and go round every 71 minutes
    let base = u32::MAX - 2 * S;
    let mut load = Load::new();
    load.feed(at(base, u32::MAX - 3 * S, u32::MAX - 5 * S));
    load.feed(at(base.wrapping_add(5 * S), (u32::MAX - 3 * S).wrapping_add(4 * S), (u32::MAX - 5 * S).wrapping_add(5 * S)));
    assert_eq!(load.percents(), [Some(20), Some(0)]);
}

#[test]
fn an_idle_counter_ahead_of_the_clock_is_no_load_and_not_a_negative_one() {
    // the two are read a moment apart, and the idle task may have run on past the wall reading
    let mut load = Load::new();
    load.feed(at(0, 0, 0));
    load.feed(at(5 * S, 5 * S + 300, 5 * S + 9_000));
    assert_eq!(load.percents(), [Some(0), Some(0)]);
}

#[test]
fn the_share_is_rounded_to_the_nearest_per_cent() {
    let mut load = Load::new();
    load.feed(at(0, 0, 0));
    // 4.7 s idle of 5: 6 % busy; 4.9999 s: well under a half per cent
    load.feed(at(5 * S, 4_700_000, 4_999_900));
    assert_eq!(load.percents(), [Some(6), Some(0)]);
}

#[test]
fn the_next_window_starts_where_the_last_one_ended() {
    let mut load = Load::new();
    load.feed(at(0, 0, 0));
    load.feed(at(5 * S, 5 * S, 5 * S));
    assert_eq!(load.percents(), [Some(0), Some(0)]);
    load.feed(at(10 * S, 5 * S + 2_500_000, 5 * S));
    assert_eq!(load.percents(), [Some(50), Some(100)]);
}

#[test]
fn a_window_that_ran_long_is_counted_as_long_as_it_was() {
    // a pass that was late: the clock says twenty seconds
    let mut load = Load::new();
    load.feed(at(0, 0, 0));
    load.feed(at(20 * S, 18 * S, 20 * S));
    assert_eq!(load.percents(), [Some(10), Some(0)]);
}
