//! What "yesterday" says of a count: the day that turned, from midnight to midnight by the local time, kept when the next day began.
//! Expected from what a person reading a card would say - the number of the day before today, nothing before the board has seen a day turn,
//! nothing when it is not sure of its day - not from the code that keeps it.

use tally::{Date, Now, Tally, Yesterday};

const HEADER: &str = "pellets-yesterday 1";

fn date(year: u16, month: u8, day: u8) -> Date {
    Date { year, month, day }
}

fn now(d: Date) -> Option<Now> {
    Some(Now { date: d, minute: 12 * 60, offset_min: 120 })
}

/// A count that burned AMOUNT on the day D, seen by the board all day.
fn burned(d: Date, amount: u64) -> Tally<1> {
    let mut t = Tally::<1>::default();
    t.at(now(d));
    t.add([amount]);
    t.at(now(d));
    t
}

#[test]
fn a_day_that_turned_forward_is_yesterday_with_what_it_came_to() {
    let mut t = burned(date(2026, 10, 22), 33);
    assert_eq!(t.yesterday_sums(), None, "no day has turned yet: nothing to say, not nought");
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), Some([33]));
    assert_eq!(t.sums.today, [0]);
    assert_eq!(t.yesterday, Some(Yesterday { date: date(2026, 10, 22), sums: [33] }));
}

#[test]
fn what_came_before_midnight_but_was_placed_after_it_is_in_the_day_that_was() {
    let mut t = burned(date(2026, 10, 22), 33);
    t.add([7]);
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), Some([40]));
    assert_eq!(t.sums.today, [0]);
}

#[test]
fn a_quiet_day_is_nought_not_silence() {
    let mut t = burned(date(2026, 10, 22), 0);
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), Some([0]));
}

#[test]
fn the_same_day_asked_again_changes_nothing() {
    let mut t = burned(date(2026, 10, 22), 33);
    t.at(now(date(2026, 10, 23)));
    t.add([5]);
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), Some([33]));
}

#[test]
fn a_day_turns_again_and_yesterday_is_the_one_that_just_ended() {
    let mut t = burned(date(2026, 10, 22), 33);
    t.at(now(date(2026, 10, 23)));
    t.add([5]);
    t.at(now(date(2026, 10, 23)));
    t.at(now(date(2026, 10, 24)));
    assert_eq!(t.yesterday_sums(), Some([5]));
}

#[test]
fn the_last_day_of_a_month_is_yesterday_on_the_first() {
    let mut t = burned(date(2026, 10, 31), 12);
    t.at(now(date(2026, 11, 1)));
    assert_eq!(t.yesterday_sums(), Some([12]));
    assert_eq!(t.sums.month, [0]);
}

#[test]
fn a_date_that_went_back_or_far_ahead_leaves_yesterday_alone() {
    let mut t = burned(date(2026, 10, 22), 33);
    t.at(now(date(2026, 10, 23)));
    t.add([5]);
    t.at(now(date(2026, 10, 23)));
    // back by a day (a zone, summer time): the count is on the 23rd still, yesterday is the 22nd still
    t.at(now(date(2026, 10, 22)));
    assert_eq!(t.yesterday_sums(), Some([33]));
    // a day of the future put right (a jump of the clock that a watch let by): yesterday is not what the false day held
    let mut far = burned(date(2028, 2, 27), 9);
    far.yesterday = Some(Yesterday { date: date(2028, 2, 26), sums: [4] });
    far.at(now(date(2026, 10, 23)));
    assert_eq!(far.yesterday_sums(), None, "the kept yesterday is a day of a false calendar");
}

#[test]
fn more_than_a_day_gone_the_board_does_not_know_what_yesterday_held() {
    let mut t = burned(date(2026, 10, 20), 33);
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), None);
    assert_eq!(t.sums.today, [0]);
}

#[test]
fn yesterday_is_not_said_while_the_count_is_not_sure_of_its_day() {
    let mut t = burned(date(2026, 10, 22), 33);
    t.at(now(date(2026, 10, 23)));
    t.at(None);
    assert_eq!(t.yesterday_sums(), None, "the time is lost: the day it stands in is not known");
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), Some([33]));
}

#[test]
fn a_kept_yesterday_is_said_only_while_it_is_the_day_before_the_count_s_day() {
    let mut t = Tally::<1>::default();
    t.day = Some(date(2026, 10, 23));
    t.yesterday = Some(Yesterday { date: date(2026, 10, 22), sums: [33] });
    assert_eq!(t.yesterday_sums(), None, "not sure of the day yet");
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), Some([33]));
    t.yesterday = Some(Yesterday { date: date(2026, 10, 20), sums: [33] });
    assert_eq!(t.yesterday_sums(), None, "the 20th is not the day before the 23rd");
}

#[test]
fn a_reset_forgets_yesterday_for_it_was_a_day_of_a_count_that_is_no_more() {
    let mut t = burned(date(2026, 10, 22), 33);
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), Some([33]));
    t.reset();
    assert_eq!(t.yesterday_sums(), None);
    assert_eq!(t.yesterday, None);
    assert_eq!(t.day, Some(date(2026, 10, 23)), "a reset is not a new day");
}

#[test]
fn the_day_that_turns_after_a_reset_is_yesterday_with_what_was_counted_since() {
    let mut t = burned(date(2026, 10, 22), 33);
    t.at(now(date(2026, 10, 23)));
    t.reset();
    t.add([5]);
    t.at(now(date(2026, 10, 23)));
    assert_eq!(t.yesterday_sums(), None, "nothing turned since the reset");
    t.at(now(date(2026, 10, 24)));
    assert_eq!(t.yesterday_sums(), Some([5]), "the 23rd as the count has it since the reset, not the 33 of the count before it");
}

#[test]
fn a_day_is_kept_as_text_and_comes_back() {
    let y = Yesterday::<2> { date: date(2026, 10, 22), sums: [197_000_000, 5] };
    let text = y.text(HEADER);
    assert_eq!(text, "pellets-yesterday 1\nday 2026-10-22\nsums 197000000 5\n");
    assert_eq!(Yesterday::<2>::parse(&text, HEADER), Ok(y));
}

#[test]
fn text_that_is_not_a_kept_day_is_refused() {
    let good = "pellets-yesterday 1\nday 2026-10-22\nsums 7\n";
    assert!(Yesterday::<1>::parse(good, HEADER).is_ok());
    let bad = [
        "".to_string(),
        good.replacen("pellets-yesterday 1", "pellets 1", 1),
        good.replacen("day 2026-10-22", "day -", 1),
        good.replacen("day 2026-10-22", "day 2026-02-30", 1),
        good.replacen("sums 7", "sums lots", 1),
        good.replacen("sums 7", "sums 7 8", 1),
        format!("{good}sums 7\n"),
        "pellets-yesterday 1\nday 2026-10-22\n".to_string(),
    ];
    for text in bad {
        assert!(Yesterday::<1>::parse(&text, HEADER).is_err(), "{text:?}");
    }
}
