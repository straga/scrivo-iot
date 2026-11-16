//! The fronts of the zero-crossing pin lost and back, told once each. On 24.10 at 23:47:00 `.33` counted 21 and then none, and the next 86 minutes the pin gave
//! a few stray edges a second; at 01:13 the count came back by itself. The board kept no time of either (tests/fixtures/fronts_night_24_10.txt is the record of that night, a second at a
//! time, as runs: "start count seconds"). A detector is alive while it gives what a mains of 40 to 71 Hz gives (`LIVE_CROSSINGS`); the fall below the band is the loss.
//!
//! Over the 452 544 seconds of the record of 21-25.10 a count fell from alive to below the band three times - 21.10 11:51:44, 21.10 14:57:27, 24.10 23:47:00 - and each was a
//! real loss, so a single second below the band is enough to tell.

use dimmer::{Change, Fronts, LIVE_CROSSINGS};

fn seconds_of_the_night() -> Vec<(String, u32)> {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fronts_night_24_10.txt")).unwrap();
    let mut seconds = Vec::new();
    for row in text.lines() {
        let mut parts = row.split(' ');
        let start = parts.next().unwrap();
        let count: i64 = parts.next().unwrap().parse().unwrap();
        let run: u32 = parts.next().unwrap().parse().unwrap();
        let clock: Vec<u32> = start.split(':').map(|n| n.parse().unwrap()).collect();
        let first = clock[0] * 3600 + clock[1] * 60 + clock[2];
        for k in 0..run {
            let at = (first + k) % 86_400;
            seconds.push((format!("{:02}:{:02}:{:02}", at / 3600, at % 3600 / 60, at % 60), count.max(0) as u32));
        }
    }
    seconds
}

#[test]
fn the_night_of_24_10_loses_the_fronts_at_23_47_00_and_has_them_back_at_01_13_37() {
    let mut fronts = Fronts::new();
    let told: Vec<(String, Change)> = seconds_of_the_night().into_iter().filter_map(|(at, up)| fronts.second(up).map(|change| (at, change))).collect();
    assert_eq!(told.len(), 2, "one loss and one return, and not a line for the stray edges between: {told:?}");
    assert_eq!(told[0], ("23:47:00".to_string(), Change::Lost { last_alive: 102 }));
    assert_eq!(told[1], ("01:13:37".to_string(), Change::Returned { silent_s: 5197 }));
}

#[test]
fn a_detector_that_never_gave_a_mains_is_silent_and_says_nothing() {
    let mut fronts = Fronts::new();
    // a bench board with no mains on the module (the stand): not a loss, because nothing was alive to lose
    assert!((0..300).all(|_| fronts.second(0).is_none()));
    assert!((0..300).all(|n| fronts.second(n % 4).is_none()));
}

#[test]
fn a_live_detector_with_the_extra_and_missing_edges_of_a_real_grid_is_never_lost() {
    let mut fronts = Fronts::new();
    for up in [100, 96, 102, 99, 99, 98, 106, 100, 91, 104, 100] {
        assert!(fronts.second(up).is_none(), "{up} is a mains");
    }
}

#[test]
fn the_edge_of_the_band_is_where_the_loss_is_told() {
    let low = *LIVE_CROSSINGS.start();
    let mut fronts = Fronts::new();
    assert!(fronts.second(100).is_none());
    assert!(fronts.second(low).is_none(), "the lowest count a mains gives");
    assert_eq!(fronts.second(low - 1), Some(Change::Lost { last_alive: low }), "one under it");
}

#[test]
fn a_loss_is_told_once_however_long_it_lasts() {
    let mut fronts = Fronts::new();
    fronts.second(100);
    assert!(fronts.second(0).is_some());
    assert!((0..1000).all(|_| fronts.second(0).is_none()));
}

#[test]
fn the_return_is_told_at_the_first_second_of_a_mains_with_how_long_the_pin_was_silent() {
    let mut fronts = Fronts::new();
    fronts.second(100);
    fronts.second(0);
    (0..9).for_each(|_| {
        fronts.second(2);
    });
    // a second that is half a mains (63) is not the return yet
    assert!(fronts.second(63).is_none());
    assert_eq!(fronts.second(99), Some(Change::Returned { silent_s: 11 }));
    assert!(fronts.second(99).is_none(), "told once");
}

#[test]
fn after_a_return_the_next_loss_is_told_again() {
    let mut fronts = Fronts::new();
    fronts.second(100);
    fronts.second(0);
    fronts.second(100);
    assert_eq!(fronts.second(0), Some(Change::Lost { last_alive: 100 }));
}

#[test]
fn how_long_the_pin_has_been_silent_counts_up_while_it_is_and_is_none_otherwise() {
    let mut fronts = Fronts::new();
    assert_eq!(fronts.silent_s(), None, "never alive");
    fronts.second(0);
    assert_eq!(fronts.silent_s(), None, "a pin that never gave a mains has lost none");
    fronts.second(100);
    assert_eq!(fronts.silent_s(), None, "alive");
    fronts.second(0);
    assert_eq!(fronts.silent_s(), Some(1), "the second that told the loss is the first of the silence");
    fronts.second(2);
    fronts.second(0);
    assert_eq!(fronts.silent_s(), Some(3));
    fronts.second(99);
    assert_eq!(fronts.silent_s(), None, "back");
}

#[test]
fn the_night_of_24_10_is_silent_for_5197_seconds_before_the_return_second() {
    let mut fronts = Fronts::new();
    let mut longest = 0;
    for (_, up) in seconds_of_the_night() {
        fronts.second(up);
        longest = longest.max(fronts.silent_s().unwrap_or(0));
    }
    assert_eq!(longest, 5197, "every second from 23:47:00 to 01:13:36 inclusive, 86 minutes 37 seconds, the one before the second that told the return");
}

#[test]
fn how_long_the_pin_has_given_a_mains_counts_up_the_seconds_in_a_row_and_starts_again_after_a_loss() {
    let mut fronts = Fronts::new();
    assert_eq!(fronts.alive_s(), None, "never alive");
    fronts.second(0);
    assert_eq!(fronts.alive_s(), None, "a pin that gave nothing yet has not been alive");
    fronts.second(100);
    assert_eq!(fronts.alive_s(), Some(1), "the first second of a mains is the first of the run");
    (0..58).for_each(|_| {
        fronts.second(97);
    });
    assert_eq!(fronts.alive_s(), Some(59));
    fronts.second(104);
    assert_eq!(fronts.alive_s(), Some(60));
    // a second under the band ends the run, and the silence has no run
    fronts.second(40);
    assert_eq!(fronts.alive_s(), None);
    fronts.second(0);
    assert_eq!(fronts.alive_s(), None);
    // the second that tells the return is the first of a new run
    fronts.second(99);
    assert_eq!(fronts.alive_s(), Some(1));
}

#[test]
fn on_the_record_of_the_night_the_run_after_the_return_is_counted_from_01_13_37() {
    let mut fronts = Fronts::new();
    let mut after_return = None;
    for (at, up) in seconds_of_the_night() {
        fronts.second(up);
        if at == "01:13:37" {
            after_return = fronts.alive_s();
        }
    }
    assert_eq!(after_return, Some(1));
    // the record ends at 01:14:00: 24 seconds from 01:13:37 on
    assert_eq!(fronts.alive_s(), Some(24));
}
