//! A failure that repeats is said when it begins and then once in a while with its count: the board that cannot join its network tried every ~74 s and wrote a line for each,
//! 14 of them in the tail of the journal. What is held is the policy, on the seconds the caller counts; the lines are the caller's.

use journal::Repeats;

const EVERY_5_MIN: u32 = 300;

#[test]
fn the_first_failure_is_said_at_once_with_its_count_of_one() {
    let mut fails = Repeats::new(EVERY_5_MIN);
    assert_eq!(fails.fail(1000).map(|said| (said.attempts, said.unsaid)), Some((1, 0)));
}

#[test]
fn the_failures_that_follow_inside_the_period_are_counted_and_not_said() {
    let mut fails = Repeats::new(EVERY_5_MIN);
    fails.fail(1000);
    for at in [1074, 1148, 1222, 1296] {
        assert!(fails.fail(at).is_none(), "a failure at {at} s said again inside five minutes");
    }
}

#[test]
fn the_period_after_the_last_line_says_it_again_with_how_many_attempts_there_were_and_how_many_went_unsaid() {
    let mut fails = Repeats::new(EVERY_5_MIN);
    fails.fail(1000);
    for at in [1074, 1148, 1222, 1296] {
        fails.fail(at);
    }
    let said = fails.fail(1370).expect("six attempts and 370 s since the line");
    assert_eq!((said.attempts, said.unsaid), (6, 4));
}

#[test]
fn a_line_is_said_once_in_a_period_however_often_it_fails() {
    // every 74 s for an hour: 49 attempts over 3552 s
    let mut fails = Repeats::new(EVERY_5_MIN);
    let lines: Vec<u32> = (0..49).map(|n| 1000 + n * 74).filter(|at| fails.fail(u64::from(*at)).is_some()).collect();
    assert!(lines.windows(2).all(|pair| pair[1] - pair[0] >= EVERY_5_MIN), "two lines closer than five minutes: {lines:?}");
    // 49 tries wrote 49 lines; the period allows the first and one in every 300 s after it
    assert!(lines.len() <= 1 + 3552 / EVERY_5_MIN as usize, "{} lines in an hour of failing every 74 s", lines.len());
    assert!(lines.len() * 4 < 49, "{} lines for 49 attempts", lines.len());
}

#[test]
fn a_success_ends_the_run_and_the_next_failure_is_said_at_once_as_the_first() {
    let mut fails = Repeats::new(EVERY_5_MIN);
    fails.fail(1000);
    fails.fail(1074);
    fails.ok();
    let said = fails.fail(1100).expect("the first failure after a success");
    assert_eq!((said.attempts, said.unsaid), (1, 0));
}

#[test]
fn a_clock_that_stands_still_or_goes_back_says_nothing_more_rather_than_everything() {
    let mut fails = Repeats::new(EVERY_5_MIN);
    fails.fail(1000);
    assert!(fails.fail(1000).is_none());
    assert!(fails.fail(400).is_none());
}

#[test]
fn a_network_that_cannot_be_joined_for_an_hour_says_ten_lines_of_forty_nine_tries() {
    // 04.10: tries every 74 s; the line comes at the first try that stands five minutes or more after the last line - the fifth after it, 370 s - so at tries 1, 6, 11 ... 46
    let mut fails = Repeats::new(EVERY_5_MIN);
    let lines = (0..49u32).filter(|n| fails.fail(u64::from(1000 + *n * 74)).is_some()).count();
    assert_eq!(lines, 10);
}

// ---- a count for each network ----
// One count for the whole list of saved networks said the tries of B under the name of A: with two networks out of reach each line carried the number of both.

use journal::Each;

#[test]
fn each_network_counts_its_own_tries_and_a_line_for_one_carries_no_try_of_the_other() {
    // A at 1000, 1074, ...; B 37 s after each: six tries of each over 370 s
    let mut fails = Each::new(EVERY_5_MIN, 2);
    let mut lines_of_a = Vec::new();
    for n in 0..6u64 {
        if let Some(said) = fails.fail(0, 1000 + n * 74) {
            lines_of_a.push((said.attempts, said.unsaid));
        }
        fails.fail(1, 1037 + n * 74);
    }
    assert_eq!(lines_of_a, vec![(1, 0), (6, 4)], "A's lines count A's tries only");
}

#[test]
fn the_first_failure_of_each_network_is_said_at_once() {
    let mut fails = Each::new(EVERY_5_MIN, 2);
    assert_eq!(fails.fail(0, 1000).map(|said| said.attempts), Some(1));
    assert_eq!(fails.fail(1, 1037).map(|said| said.attempts), Some(1), "B's first, though A has just been said");
    assert!(fails.fail(0, 1074).is_none());
    assert!(fails.fail(1, 1111).is_none());
}

#[test]
fn a_join_ends_every_run_and_the_next_failure_of_any_network_is_the_first_again() {
    let mut fails = Each::new(EVERY_5_MIN, 2);
    fails.fail(0, 1000);
    fails.fail(1, 1037);
    assert!(!fails.quiet());
    fails.ok();
    assert!(fails.quiet());
    assert_eq!(fails.fail(1, 1100).map(|said| said.attempts), Some(1));
}

#[test]
fn a_network_the_list_does_not_have_says_nothing_and_does_not_fall() {
    let mut fails = Each::new(EVERY_5_MIN, 2);
    assert!(fails.fail(5, 1000).is_none());
    assert!(fails.quiet());
}

// ---- a count for each access point ----
// One count for every access point said the breaks of B under the name of A, and a join at any of them cleared the run of the rest.

use journal::Keyed;

const A: [u8; 6] = [0xdc, 0x2c, 0x6e, 0x7f, 0x39, 0x4f];
const B: [u8; 6] = [0xdc, 0x2c, 0x6e, 0x7f, 0x39, 0x50];

#[test]
fn breaks_at_two_points_are_not_added_into_one_count() {
    // A breaks at 1000, 1074, ...; B 37 s after each: six breaks of each over 370 s
    let mut breaks: Keyed<[u8; 6], 4> = Keyed::new(EVERY_5_MIN);
    let mut lines_of_a = Vec::new();
    let mut lines_of_b = Vec::new();
    for n in 0..6u64 {
        if let Some(said) = breaks.fail(A, 1000 + n * 74) {
            lines_of_a.push((said.attempts, said.unsaid));
        }
        if let Some(said) = breaks.fail(B, 1037 + n * 74) {
            lines_of_b.push((said.attempts, said.unsaid));
        }
    }
    assert_eq!(lines_of_a, vec![(1, 0), (6, 4)], "A's lines count A's breaks only");
    assert_eq!(lines_of_b, vec![(1, 0), (6, 4)], "B's lines count B's breaks only");
}

#[test]
fn the_first_break_at_each_point_is_said_at_once() {
    let mut breaks: Keyed<[u8; 6], 4> = Keyed::new(EVERY_5_MIN);
    assert_eq!(breaks.fail(A, 1000).map(|said| said.attempts), Some(1));
    assert_eq!(breaks.fail(B, 1037).map(|said| said.attempts), Some(1), "B's first, though A has just been said");
    assert!(breaks.fail(A, 1074).is_none());
}

#[test]
fn a_join_at_one_point_does_not_clear_the_count_of_another() {
    let mut breaks: Keyed<[u8; 6], 4> = Keyed::new(EVERY_5_MIN);
    breaks.fail(A, 1000);
    breaks.fail(B, 1037);
    breaks.fail(B, 1111);
    breaks.ok(&A);
    assert_eq!(breaks.fail(A, 1148).map(|said| said.attempts), Some(1), "A's run is over: its next break is the first");
    breaks.fail(B, 1185);
    breaks.fail(B, 1259);
    breaks.fail(B, 1333);
    let said = breaks.fail(B, 1407).expect("B's sixth break stands 370 s after its first line");
    assert_eq!((said.attempts, said.unsaid), (6, 4), "B's run went on through the join at A");
}

#[test]
fn a_key_past_the_capacity_takes_the_place_of_the_oldest_and_does_not_fall() {
    let mut breaks: Keyed<u8, 2> = Keyed::new(EVERY_5_MIN);
    for (n, key) in [1u8, 2, 3].into_iter().enumerate() {
        assert_eq!(breaks.fail(key, 1000 + n as u64).map(|said| said.attempts), Some(1));
    }
    assert!(breaks.fail(3, 1010).is_none(), "3 is held: its second break is counted");
    assert_eq!(breaks.fail(1, 1020).map(|said| said.attempts), Some(1), "1 was forgotten for 3: it begins again");
}

#[test]
fn a_join_at_a_point_not_known_ends_the_run_of_every_point() {
    let mut breaks: Keyed<[u8; 6], 4> = Keyed::new(EVERY_5_MIN);
    breaks.fail(A, 1000);
    breaks.fail(B, 1037);
    breaks.ok_all();
    assert_eq!(breaks.fail(A, 1074).map(|said| said.attempts), Some(1));
    assert_eq!(breaks.fail(B, 1111).map(|said| said.attempts), Some(1));
}
