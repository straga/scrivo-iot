//! What a count of one thing does over time, whatever the thing is: it adds to today, this month and in all; it turns
//! with the local calendar; what is counted before the time is known waits and lands in the day the time names. Expected
//! from what a person reading "today" and "this month" would say - not from the code that does it.

use tally::{Date, Keeper, Now, Tally};

fn date(year: u16, month: u8, day: u8) -> Date {
    Date { year, month, day }
}

fn now(d: Date) -> Option<Now> {
    Some(Now { date: d, minute: 12 * 60, offset_min: 120 })
}

#[test]
fn what_is_added_is_in_all_at_once_and_in_the_day_when_the_time_is_known() {
    let mut t = Tally::<1>::default();
    t.add([5]);
    assert_eq!((t.sums.total, t.sums.today, t.sums.month), ([5], [0], [0]), "no day yet: in all only");
    t.at(now(date(2026, 10, 4)));
    assert_eq!((t.sums.total, t.sums.today, t.sums.month), ([5], [5], [5]));
    assert_eq!(t.pending, [0]);
    assert_eq!(t.day, Some(date(2026, 10, 4)));
}

#[test]
fn a_new_day_zeroes_today_and_keeps_the_month() {
    let mut t = Tally::<1>::default();
    t.at(now(date(2026, 10, 4)));
    t.add([5]);
    t.at(now(date(2026, 10, 4)));
    t.at(now(date(2026, 10, 5)));
    t.add([2]);
    t.at(now(date(2026, 10, 5)));
    assert_eq!((t.sums.today, t.sums.month, t.sums.total), ([2], [7], [7]));
}

#[test]
fn a_new_month_zeroes_the_month_and_the_same_month_of_the_next_year_does_too() {
    let mut t = Tally::<1>::default();
    t.at(now(date(2026, 10, 31)));
    t.add([5]);
    t.at(now(date(2026, 10, 31)));
    t.at(now(date(2026, 11, 1)));
    assert_eq!((t.sums.today, t.sums.month, t.sums.total), ([0], [0], [5]));
    t.add([3]);
    t.at(now(date(2026, 11, 1)));
    t.at(now(date(2027, 11, 1)).map(|mut n| {
        n.date.day = 2;
        n
    }));
    assert_eq!((t.sums.today, t.sums.month, t.sums.total), ([0], [0], [8]));
}

#[test]
fn what_came_before_a_turn_belongs_to_the_day_that_was() {
    let mut t = Tally::<1>::default();
    t.at(now(date(2026, 10, 4)));
    t.add([5]);
    // counted at 23:59 but placed after midnight: the board saw the time again only now
    let placed = t.settle_if_timed();
    assert_eq!(placed, [5]);
    t.at(now(date(2026, 10, 5)));
    assert_eq!((t.sums.today, t.sums.month), ([0], [5]), "the five went to the fourth");
}

#[test]
fn a_clock_that_jumps_back_closes_no_day_and_takes_nothing_away() {
    let mut t = Tally::<1>::default();
    t.at(now(date(2026, 10, 5)));
    t.add([4]);
    t.at(now(date(2026, 10, 5)));
    // a day back: a zone or summer time. More than that is a day recorded from a time that was false (tests/jump.rs).
    t.at(now(date(2026, 10, 4)));
    assert_eq!(t.day, Some(date(2026, 10, 5)));
    assert_eq!(t.sums.today, [4]);
}

#[test]
fn a_time_lost_and_found_places_what_waited_in_the_day_found() {
    let mut t = Tally::<1>::default();
    t.at(now(date(2026, 10, 4)));
    t.at(None);
    t.add([6]);
    assert_eq!(t.sums.today, [0]);
    t.at(now(date(2026, 10, 4)));
    assert_eq!((t.sums.today, t.sums.month, t.sums.total), ([6], [6], [6]));
}

#[test]
fn a_reset_zeroes_the_sums_and_what_waits_but_the_day_stays() {
    let mut t = Tally::<1>::default();
    t.at(now(date(2026, 10, 4)));
    t.add([5]);
    t.at(now(date(2026, 10, 4)));
    t.add([3]);
    t.reset();
    assert_eq!((t.sums.today, t.sums.month, t.sums.total, t.pending), ([0], [0], [0], [0]));
    assert_eq!(t.day, Some(date(2026, 10, 4)), "a reset is not a new day");
}

#[test]
fn several_outputs_are_counted_side_by_side() {
    let mut t = Tally::<2>::default();
    t.add([1, 2]);
    t.at(now(date(2026, 10, 4)));
    assert_eq!((t.sums.today, t.sums.total), ([1, 2], [1, 2]));
}

#[test]
fn a_write_that_finds_the_keeper_busy_does_not_wait_and_says_so() {
    let keeper = Keeper::new();
    let (first, second) = (keeper.take("a".into()), keeper.take("b".into()));
    let mut nested = None;
    keeper
        .write(first, |_| {
            // the keeper is held by this write: another that would have to wait does not
            nested = Some(keeper.try_write(second.clone(), |_| Ok::<(), ()>(())));
            Ok::<(), ()>(())
        })
        .unwrap();
    assert_eq!(nested, Some(None), "a write that found the keeper busy waited or went through");
}

#[test]
fn a_write_that_finds_the_keeper_free_is_written_like_any_other() {
    let keeper = Keeper::new();
    let (first, second) = (keeper.take("a".into()), keeper.take("b".into()));
    let mut held = String::new();
    assert_eq!(keeper.try_write(second, |t| -> Result<(), ()> { held = t.to_string(); Ok(()) }), Some(Ok(true)));
    assert_eq!(held, "b");
    // an older snapshot than the one written is passed over, as with write
    assert_eq!(keeper.try_write(first, |_| -> Result<(), ()> { panic!("an older snapshot was written") }), Some(Ok(false)));
}

#[test]
fn a_write_the_flash_refused_through_try_write_leaves_the_last_written_as_it_was() {
    let keeper = Keeper::new();
    let (first, second) = (keeper.take("a".into()), keeper.take("b".into()));
    assert_eq!(keeper.try_write(first, |_| Err::<(), &str>("no")), Some(Err("no")));
    assert_eq!(keeper.try_write(second, |_| Ok::<(), &str>(())), Some(Ok(true)));
}
