//! How a person sees the soot count: what the burns say after they end, what a cleaning does, what outlives a restart.

use soot::Soot;
use tally::Date;

/// A burn of MINUTES in the working stage with the flue DIFF degrees over the water, then one second out of it.
fn burn(s: &mut Soot, minutes: u32, diff: f32) {
    for _ in 0..minutes * 60 {
        s.second(true, true, Some(60.0 + diff), Some(60.0));
    }
    s.second(false, true, Some(40.0), Some(55.0));
}

#[test]
fn a_burn_of_twenty_minutes_at_forty_over_the_water_says_forty() {
    let mut s = Soot::default();
    burn(&mut s, 20, 40.0);
    assert_eq!(s.soot_c(), Some(40.0));
    assert_eq!(s.burns_counted(), 1);
}

#[test]
fn a_burn_of_ten_minutes_is_not_counted() {
    let mut s = Soot::default();
    burn(&mut s, 10, 40.0);
    assert_eq!(s.soot_c(), None);
    assert_eq!(s.burns_counted(), 0);
}

#[test]
fn fifteen_minutes_count_and_a_second_less_does_not() {
    let mut s = Soot::default();
    for _ in 0..899 {
        s.second(true, true, Some(100.0), Some(60.0));
    }
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), None, "14:59 is short");
    for _ in 0..900 {
        s.second(true, true, Some(100.0), Some(60.0));
    }
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(40.0), "15:00 counts");
}

#[test]
fn the_first_five_minutes_of_a_burn_do_not_count() {
    let mut s = Soot::default();
    for _ in 0..300 {
        s.second(true, true, Some(140.0), Some(60.0)); // 80 over: the warm-up
    }
    for _ in 0..900 {
        s.second(true, true, Some(100.0), Some(60.0)); // 40 over
    }
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(40.0));
}

#[test]
fn a_burn_that_has_not_ended_is_not_in_the_number() {
    let mut s = Soot::default();
    for _ in 0..1800 {
        s.second(true, true, Some(100.0), Some(60.0));
    }
    assert_eq!(s.soot_c(), None);
}

#[test]
fn seconds_without_a_reading_are_skipped_not_counted_as_zero() {
    let mut s = Soot::default();
    for i in 0..1800 {
        if i % 3 == 0 {
            s.second(true, true, None, Some(60.0));
        } else {
            s.second(true, true, Some(100.0), Some(60.0));
        }
    }
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(40.0));
}

#[test]
fn the_number_is_the_median_of_the_last_six_burns() {
    let mut s = Soot::default();
    for diff in [90.0, 90.0, 40.0, 40.0, 40.0, 50.0, 50.0, 50.0] {
        burn(&mut s, 20, diff);
    }
    // the newest six: 40 40 40 50 50 50 -> between the two in the middle
    assert_eq!(s.soot_c(), Some(45.0));
    assert_eq!(s.burns_counted(), 6);
}

#[test]
fn three_burns_say_three() {
    let mut s = Soot::default();
    for diff in [44.0, 40.0, 48.0] {
        burn(&mut s, 20, diff);
    }
    assert_eq!(s.soot_c(), Some(44.0));
    assert_eq!(s.burns_counted(), 3);
}

#[test]
fn a_restart_keeps_eight_burns_in_place() {
    let mut s = Soot::default();
    for diff in [41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 47.0, 48.0, 49.0] {
        burn(&mut s, 20, diff);
    }
    assert_eq!(s.burns_kept(), 8);
    let back = Soot::restore(&s.snapshot()).expect("its own snapshot");
    assert_eq!(back.burns_kept(), 8);
    assert_eq!(back.soot_c(), s.soot_c());
    assert_eq!(back.soot_c(), Some(46.5), "the newest six: 44..49");
}

#[test]
fn a_restart_does_not_make_a_burn_out_of_the_one_that_was_running() {
    let mut s = Soot::default();
    for _ in 0..1800 {
        s.second(true, true, Some(100.0), Some(60.0));
    }
    let mut back = Soot::restore(&s.snapshot()).unwrap();
    back.second(false, true, None, None);
    assert_eq!(back.soot_c(), None);
}

#[test]
fn what_is_not_a_snapshot_is_refused() {
    assert!(Soot::restore("").is_err());
    assert!(Soot::restore("pellets 1\n").is_err());
    let good = Soot::default().snapshot();
    assert!(Soot::restore(&format!("{good}extra\n")).is_err());
    assert!(Soot::restore(&good.replace("burns", "brunes")).is_err());
}

#[test]
fn kilograms_since_the_cleaning_follow_the_total_of_the_pellets() {
    let mut s = Soot::default();
    s.observe_total_g(10_000); // the first look only says where the total stands
    assert_eq!(s.grams_since_clean(), 0);
    s.observe_total_g(10_250);
    s.observe_total_g(10_900);
    assert_eq!(s.grams_since_clean(), 900);
}

#[test]
fn a_reset_of_the_pellets_does_not_take_grams_from_the_count() {
    let mut s = Soot::default();
    s.observe_total_g(10_000);
    s.observe_total_g(10_500);
    s.observe_total_g(0); // the pellets were zeroed
    s.observe_total_g(300);
    assert_eq!(s.grams_since_clean(), 800);
}

#[test]
fn cleaning_zeroes_the_grams_and_marks_the_day_and_forgets_the_burns_of_the_dirty_boiler() {
    let mut s = Soot::default();
    burn(&mut s, 20, 52.0);
    s.observe_total_g(1000);
    s.observe_total_g(6000);
    let day = Date { year: 2026, month: 11, day: 9 };
    s.cleaned(Some(day));
    assert_eq!(s.grams_since_clean(), 0);
    assert_eq!(s.cleaned_on(), Some(day));
    assert_eq!(s.soot_c(), None, "the burns of the boiler before the cleaning say nothing of the one after it");
    assert_eq!((s.burns_counted(), s.burns_kept()), (0, 0));
    s.observe_total_g(6100);
    assert_eq!(s.grams_since_clean(), 100);
    burn(&mut s, 20, 41.0);
    assert_eq!(s.soot_c(), Some(41.0), "the first burn after the cleaning is the whole number: nothing of the old ones is mixed in");
    assert_eq!(s.burns_counted(), 1);
}

#[test]
fn a_cleaning_forgets_the_burns_for_good_and_a_restart_does_not_bring_them_back() {
    let mut s = Soot::default();
    for diff in [55.0, 56.0, 57.0] {
        burn(&mut s, 20, diff);
    }
    s.cleaned(Some(Date { year: 2026, month: 11, day: 9 }));
    let back = Soot::restore(&s.snapshot()).unwrap();
    assert_eq!((back.soot_c(), back.burns_kept()), (None, 0));
}

#[test]
fn the_grams_and_the_day_outlive_a_restart() {
    let mut s = Soot::default();
    s.observe_total_g(1000);
    s.observe_total_g(6000);
    s.cleaned(Some(Date { year: 2026, month: 11, day: 9 }));
    s.observe_total_g(6400);
    let mut back = Soot::restore(&s.snapshot()).unwrap();
    assert_eq!(back.grams_since_clean(), 400);
    assert_eq!(back.cleaned_on(), s.cleaned_on());
    back.observe_total_g(6500);
    assert_eq!(back.grams_since_clean(), 500, "the total kept across the restart is where the next look starts");
}

#[test]
fn what_changes_is_counted_so_the_board_writes_it() {
    let mut s = Soot::default();
    let start = s.changes;
    burn(&mut s, 20, 40.0);
    assert!(s.changes > start, "a burn that ended is news for the flash");
    let after_burn = s.changes;
    s.observe_total_g(500);
    s.observe_total_g(600);
    assert!(s.changes > after_burn, "grams are news too");
    let after_grams = s.changes;
    s.second(false, true, None, None);
    assert_eq!(s.changes, after_grams, "a second that changes nothing is no news");
}

// ---- a cleaning in the middle of a burn: what the burn gathered before it is of the dirty boiler ----

/// SECONDS of the working stage at DIFF degrees over the water, the burn going on.
fn working(s: &mut Soot, seconds: u32, diff: f32) {
    for _ in 0..seconds {
        s.second(true, true, Some(60.0 + diff), Some(60.0));
    }
}

#[test]
fn a_cleaning_in_the_middle_of_a_burn_leaves_the_first_number_to_the_readings_after_it() {
    let mut s = Soot::default();
    working(&mut s, 40 * 60, 60.0); // forty minutes of the dirty boiler
    s.cleaned(Some(Date { year: 2026, month: 11, day: 9 }));
    working(&mut s, 20 * 60, 41.0); // twenty minutes of the clean one
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(41.0), "nothing of the readings before the cleaning is in the number");
    assert_eq!(s.burns_counted(), 1);
}

#[test]
fn the_quarter_of_an_hour_of_a_burn_is_counted_from_the_cleaning_and_not_from_the_start_of_the_burn() {
    let mut s = Soot::default();
    working(&mut s, 40 * 60, 60.0);
    s.cleaned(None);
    working(&mut s, 10 * 60, 41.0); // ten minutes after it: short, though the burn has lasted fifty
    s.second(false, true, None, None);
    assert_eq!((s.soot_c(), s.burns_counted()), (None, 0), "ten minutes of the clean boiler are no burn");
}

#[test]
fn the_first_five_minutes_after_a_cleaning_in_the_middle_of_a_burn_are_not_counted_either() {
    let mut s = Soot::default();
    working(&mut s, 40 * 60, 60.0);
    s.cleaned(None);
    working(&mut s, 5 * 60, 80.0); // the first five minutes after it: the fire is not settled
    working(&mut s, 15 * 60, 41.0);
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(41.0));
}

#[test]
fn a_cleaning_while_the_burner_stands_does_not_touch_the_burn_that_comes_after_it() {
    let mut s = Soot::default();
    burn(&mut s, 20, 52.0);
    s.cleaned(None);
    burn(&mut s, 20, 41.0);
    assert_eq!(s.soot_c(), Some(41.0));
    assert_eq!(s.burns_counted(), 1);
}

// ---- one window for every burn: the fifth to the fifteenth minute, the seconds on the big rung ----

/// SECONDS of the working stage on the big rung or off it, at DIFF degrees over the water; the burn goes on.
fn stretch(s: &mut Soot, seconds: u32, on_big: bool, diff: f32) {
    for _ in 0..seconds {
        s.second(true, on_big, Some(60.0 + diff), Some(60.0));
    }
}

fn minutes(m: u32) -> u32 {
    m * 60
}

#[test]
fn a_long_burn_says_what_its_window_says_and_not_what_its_tail_says() {
    let mut s = Soot::default();
    stretch(&mut s, minutes(5), true, 80.0); // the warm-up
    stretch(&mut s, minutes(10), true, 40.0); // the window
    stretch(&mut s, minutes(20), true, 70.0); // a long burn's tail, still on the big rung
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(40.0));
    assert_eq!(s.burns_counted(), 1);
}

#[test]
fn the_tail_of_a_burn_off_the_big_rung_does_not_change_the_number() {
    let mut s = Soot::default();
    stretch(&mut s, minutes(15), true, 40.0);
    stretch(&mut s, minutes(15), false, 80.0); // the fire went down to mid and small after the window
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(40.0));
    assert_eq!(s.burns_counted(), 1);
}

#[test]
fn a_warm_up_on_a_lower_rung_does_not_matter_when_the_window_is_on_the_big_one() {
    let mut s = Soot::default();
    stretch(&mut s, minutes(5), false, 80.0);
    stretch(&mut s, minutes(10), true, 40.0);
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(40.0));
    assert_eq!(s.burns_counted(), 1);
}

#[test]
fn a_burn_whose_big_rung_ends_in_the_ninth_minute_is_not_counted() {
    let mut s = Soot::default();
    stretch(&mut s, minutes(9), true, 40.0);
    stretch(&mut s, minutes(11), false, 40.0);
    s.second(false, true, None, None);
    assert_eq!((s.soot_c(), s.burns_counted()), (None, 0));
}

#[test]
fn a_single_second_off_the_big_rung_inside_the_window_leaves_the_burn_uncounted() {
    let mut s = Soot::default();
    stretch(&mut s, minutes(5), true, 40.0);
    stretch(&mut s, minutes(5), true, 40.0);
    stretch(&mut s, 1, false, 40.0);
    stretch(&mut s, minutes(5) - 1, true, 40.0);
    stretch(&mut s, minutes(5), true, 40.0);
    s.second(false, true, None, None);
    assert_eq!((s.soot_c(), s.burns_counted()), (None, 0), "599 seconds of the window on big are fewer than ten minutes");
}

#[test]
fn the_seconds_off_the_big_rung_after_the_window_do_not_unmake_a_burn_that_was_on_it_in_the_window() {
    let mut s = Soot::default();
    stretch(&mut s, minutes(15), true, 44.0);
    stretch(&mut s, 1, false, 90.0);
    stretch(&mut s, minutes(5), true, 44.0);
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(44.0));
}

#[test]
fn a_cleaning_in_the_middle_of_a_burn_opens_the_window_anew() {
    let mut s = Soot::default();
    stretch(&mut s, minutes(40), true, 60.0); // the dirty boiler
    s.cleaned(None);
    stretch(&mut s, minutes(5), true, 80.0); // the first five minutes after it
    stretch(&mut s, minutes(10), true, 41.0); // its window
    stretch(&mut s, minutes(20), true, 70.0); // its tail
    s.second(false, true, None, None);
    assert_eq!(s.soot_c(), Some(41.0));
}

// ---- a count written by the image before the window: its burns were counted over the whole burn and do not mix with the new ones ----

/// What the image before the window kept on a board: six burns of 40 to 44 degrees over the water, 29.6 kg since the cleaning of 08.11, the total of the pellets at its last look.
const BEFORE_THE_WINDOW: &str = "soot 1\nday 2026-11-08\ngrams 29600 9000000 1\nburns 6 120 126 130 118 122 128 0 0\n";

#[test]
fn a_count_of_the_image_before_the_window_keeps_the_grams_and_the_day_and_forgets_the_burns() {
    let back = Soot::restore(BEFORE_THE_WINDOW).expect("the image before wrote it, and the new one reads it");
    assert_eq!(back.soot_c(), None, "the burns of another count say nothing of the new one");
    assert_eq!((back.burns_counted(), back.burns_kept()), (0, 0));
    assert_eq!(back.grams_since_clean(), 29_600);
    assert_eq!(back.cleaned_on(), Some(Date { year: 2026, month: 11, day: 8 }));
}

#[test]
fn the_grams_go_on_from_where_the_total_of_the_pellets_stood_when_the_old_count_was_written() {
    let mut back = Soot::restore(BEFORE_THE_WINDOW).unwrap();
    back.observe_total_g(9_000_400);
    assert_eq!(back.grams_since_clean(), 30_000, "the 400 grams burned between the writing and the restart are not lost");
}

#[test]
fn what_a_restored_old_count_says_next_is_a_count_of_the_new_kind() {
    let mut back = Soot::restore(BEFORE_THE_WINDOW).unwrap();
    stretch(&mut back, minutes(5), true, 80.0);
    stretch(&mut back, minutes(10), true, 37.0);
    stretch(&mut back, minutes(5), true, 70.0);
    back.second(false, true, None, None);
    assert_eq!((back.soot_c(), back.burns_counted()), (Some(37.0), 1), "the first burn after it is the whole number: nothing of the old six is mixed in");
    let again = Soot::restore(&back.snapshot()).expect("its own snapshot");
    assert_eq!(again.soot_c(), Some(37.0));
    assert_ne!(back.snapshot(), BEFORE_THE_WINDOW, "what it writes now is not the old kind");
}

#[test]
fn a_snapshot_of_a_kind_nobody_wrote_is_still_refused() {
    assert!(Soot::restore("soot 3\nday -\ngrams 0 0 0\nburns 0 0 0 0 0 0 0 0 0\n").is_err());
    assert!(Soot::restore(&format!("{BEFORE_THE_WINDOW}extra\n")).is_err(), "an old snapshot with a line more is no snapshot");
    assert!(Soot::restore(&BEFORE_THE_WINDOW.replace("grams", "gramm")).is_err());
}
