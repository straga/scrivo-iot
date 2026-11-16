//! A cleaning the board was not there for: the day of it and the total of the pellets when it was made, given by a person after the fact.
//! Expected from what a person means: the kilograms since the cleaning are what the pellets counted since, the day is the day told, the burns of the boiler that was cleaned are still its own, and a day that has not come or a total the pellets have not reached is refused with a code, touching nothing.

use soot::past::{parse, Refused};
use soot::Soot;
use tally::Date;

const NOV_8: Date = Date { year: 2026, month: 11, day: 8 };
const NOV_16: Date = Date { year: 2026, month: 11, day: 16 };
const NOV_17: Date = Date { year: 2026, month: 11, day: 17 };

/// A count that has looked at a total of 117.2 kg of pellets burned.
fn looking_at_117_kg() -> Soot {
    let mut s = Soot::default();
    s.observe_total_g(117_200);
    s
}

fn burn(s: &mut Soot, diff: f32) {
    for _ in 0..20 * 60 {
        s.second(true, true, Some(60.0 + diff), Some(60.0));
    }
    s.second(false, true, None, None);
}

// ---- what a past cleaning does ----

#[test]
fn a_cleaning_of_08_11_at_82_4_kg_marks_the_day_and_counts_the_kilograms_since() {
    let mut s = looking_at_117_kg();
    assert_eq!(s.cleaned_before(NOV_8, Some(NOV_16), 82_400), Ok(()));
    assert_eq!(s.cleaned_on(), Some(NOV_8));
    assert_eq!(s.grams_since_clean(), 34_800);
}

#[test]
fn the_kilograms_go_on_growing_with_the_pellets_after_it() {
    let mut s = looking_at_117_kg();
    s.cleaned_before(NOV_8, Some(NOV_16), 82_400).unwrap();
    s.observe_total_g(117_500);
    assert_eq!(s.grams_since_clean(), 35_100);
}

#[test]
fn the_burns_of_the_boiler_that_was_cleaned_stay_in_the_number() {
    let mut s = looking_at_117_kg();
    burn(&mut s, 40.0);
    s.cleaned_before(NOV_8, Some(NOV_16), 82_400).unwrap();
    assert_eq!((s.soot_c(), s.burns_counted()), (Some(40.0), 1), "a cleaning that was before the burns forgets none of them");
}

#[test]
fn today_is_a_day_that_has_come_and_a_total_equal_to_the_one_now_says_nothing_since() {
    let mut s = looking_at_117_kg();
    assert_eq!(s.cleaned_before(NOV_16, Some(NOV_16), 117_200), Ok(()));
    assert_eq!((s.cleaned_on(), s.grams_since_clean()), (Some(NOV_16), 0));
}

#[test]
fn it_is_news_for_the_flash_so_the_board_writes_it_at_once() {
    let mut s = looking_at_117_kg();
    let (changes, urgent) = (s.changes, s.urgent);
    s.cleaned_before(NOV_8, Some(NOV_16), 82_400).unwrap();
    assert!(s.changes > changes && s.urgent > urgent);
}

#[test]
fn the_day_and_the_kilograms_outlive_a_restart() {
    let mut s = looking_at_117_kg();
    s.cleaned_before(NOV_8, Some(NOV_16), 82_400).unwrap();
    let mut back = Soot::restore(&s.snapshot()).expect("its own snapshot");
    assert_eq!((back.cleaned_on(), back.grams_since_clean()), (Some(NOV_8), 34_800));
    back.observe_total_g(117_300);
    assert_eq!(back.grams_since_clean(), 34_900, "the total kept across the restart is where the next look starts");
}

// ---- what it refuses, and that a refusal touches nothing ----

fn refused(today: Option<Date>, day: Date, total_g: u64, mut s: Soot) -> Refused {
    let (cleaned_on, grams, changes) = (s.cleaned_on(), s.grams_since_clean(), s.changes);
    let why = s.cleaned_before(day, today, total_g).expect_err("it was to be refused");
    assert_eq!((s.cleaned_on(), s.grams_since_clean(), s.changes), (cleaned_on, grams, changes), "a refusal changed the count");
    why
}

#[test]
fn a_day_later_than_today_is_refused() {
    assert_eq!(refused(Some(NOV_16), NOV_17, 82_400, looking_at_117_kg()), Refused::FutureDay);
}

#[test]
fn a_total_above_the_one_now_is_refused() {
    assert_eq!(refused(Some(NOV_16), NOV_8, 118_000, looking_at_117_kg()), Refused::TotalAboveNow);
}

#[test]
fn a_board_that_has_not_looked_at_the_pellets_cannot_tell_the_grams_and_refuses() {
    assert_eq!(refused(Some(NOV_16), NOV_8, 82_400, Soot::default()), Refused::TotalNotSeen);
}

#[test]
fn a_board_that_does_not_know_the_date_cannot_tell_a_future_day_and_refuses() {
    assert_eq!(refused(None, NOV_8, 82_400, looking_at_117_kg()), Refused::TimeUnknown);
}

#[test]
fn every_refusal_has_a_code_and_words() {
    let all = [
        (Refused::Usage, "usage"),
        (Refused::BadDay, "bad_day"),
        (Refused::BadKg, "bad_kg"),
        (Refused::TimeUnknown, "time_unknown"),
        (Refused::FutureDay, "future_day"),
        (Refused::TotalNotSeen, "total_not_seen"),
        (Refused::TotalAboveNow, "total_above_now"),
        (Refused::AlreadyKnown, "already_known"),
    ];
    for (why, code) in all {
        assert_eq!(why.code(), code);
        assert!(!why.words().is_empty() && !why.words().contains(code), "{code}: the words say it to a person, not the code again");
    }
}

// ---- the words of the console ----

#[test]
fn a_day_and_kilograms_are_read_as_a_day_and_grams() {
    assert_eq!(parse(&["2026-11-08", "82.4"]), Ok((NOV_8, 82_400)));
    assert_eq!(parse(&["2026-11-08", "82"]), Ok((NOV_8, 82_000)));
    assert_eq!(parse(&["2026-11-08", "82.45"]), Ok((NOV_8, 82_450)));
    assert_eq!(parse(&["2026-11-08", "0.001"]), Ok((NOV_8, 1)));
}

#[test]
fn words_that_are_not_a_day_and_kilograms_are_refused_by_what_is_wrong() {
    assert_eq!(parse(&[]), Err(Refused::Usage));
    assert_eq!(parse(&["2026-11-08"]), Err(Refused::Usage));
    assert_eq!(parse(&["2026-11-08", "82.4", "kg"]), Err(Refused::Usage));
    assert_eq!(parse(&["08.11", "82.4"]), Err(Refused::BadDay));
    assert_eq!(parse(&["2026-13-01", "82.4"]), Err(Refused::BadDay));
    for bad in ["82,4", "-1", "abc", "", ".5", "82.", "82.4567", "1e3"] {
        assert_eq!(parse(&["2026-11-08", bad]), Err(Refused::BadKg), "'{bad}'");
    }
}

// ---- a past cleaning is given once: to a board that knows no cleaning ----

#[test]
fn the_first_past_cleaning_of_a_board_that_knows_none_is_taken() {
    let mut s = looking_at_117_kg();
    assert_eq!(s.cleaned_on(), None);
    assert_eq!(s.cleaned_before(NOV_8, Some(NOV_16), 82_400), Ok(()));
}

#[test]
fn a_cleaning_the_button_marked_without_a_day_is_not_a_known_one() {
    // the board did not know the time when the button was pressed: the card says no day, and the first past cleaning is still to be given
    let mut s = looking_at_117_kg();
    s.cleaned(None);
    assert_eq!(s.cleaned_before(NOV_8, Some(NOV_16), 82_400), Ok(()));
}

#[test]
fn the_same_words_given_again_are_refused_and_the_count_stays() {
    let mut s = looking_at_117_kg();
    s.cleaned_before(NOV_8, Some(NOV_16), 82_400).unwrap();
    assert_eq!(refused(Some(NOV_16), NOV_8, 82_400, s), Refused::AlreadyKnown);
}

#[test]
fn a_cleaning_given_after_a_reset_of_the_pellets_is_refused_and_the_grams_stay() {
    // the review's case: the total the person means is of the epoch before the reset, and the total now is of the one after it - 82.4 kg against 0.3
    let mut s = looking_at_117_kg();
    s.cleaned_before(NOV_8, Some(NOV_16), 82_400).unwrap();
    s.observe_total_g(300); // the pellets were zeroed and 300 grams burned since
    assert_eq!(s.grams_since_clean(), 34_800, "a reset of the pellets takes no grams from the count");
    assert_eq!(refused(Some(NOV_16), NOV_8, 82_400, s.clone()), Refused::AlreadyKnown, "a total of the old epoch above the new one");
    assert_eq!(refused(Some(NOV_16), NOV_8, 100, s), Refused::AlreadyKnown, "a total of the old epoch that happens to stand under the new one would give grams of no epoch at all");
}

#[test]
fn a_cleaning_the_button_marked_with_its_day_is_known_and_a_past_one_is_refused() {
    let mut s = looking_at_117_kg();
    s.cleaned(Some(NOV_16));
    assert_eq!(refused(Some(NOV_16), NOV_8, 82_400, s), Refused::AlreadyKnown);
}
