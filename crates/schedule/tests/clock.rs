//! The profile that follows the clock, on rows of wall time: a window opens and closes, a window crosses midnight, a window stands on some days of the week, "every N hours", the
//! time unknown.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: a window picks a PROFILE. A window belongs to the day it STARTS on, so one that crosses midnight on a Friday is still there on Saturday at three, and none begins on the Saturday it
//! does not stand on. Where the board does not know the time the windows say nothing and the main profile stands, and the answer says so by a code of its own.

use schedule::{Profile, Schedule, Source, Span, Wall, Window, WINDOWS};

const ALL: u8 = 0b111_1111;
const WEEK: u8 = 0b001_1111;
const END: u8 = 0b110_0000;
const MON: u8 = 0;
const FRI: u8 = 4;
const SAT: u8 = 5;
const SUN: u8 = 6;
const MON_ONLY: u8 = 0b000_0001;

fn hm(h: u16, m: u16) -> u16 {
    h * 60 + m
}

fn wall(day: u8, h: u16, m: u16) -> Option<Wall> {
    Some(Wall { day, minute: hm(h, m) })
}

fn between(days: u8, from: u16, to: u16, profile: Profile) -> Window {
    Window { days, span: Span::Between { from, to }, profile: Some(profile) }
}

fn every(days: u8, from: u16, every_h: u8, for_min: u16, profile: Profile) -> Window {
    Window { days, span: Span::Every { from, every_h, for_min }, profile: Some(profile) }
}

fn schedule(main: Profile, windows: &[Window]) -> Schedule {
    let mut list = [None; WINDOWS];
    for (at, w) in windows.iter().enumerate() {
        list[at] = Some(*w);
    }
    Schedule { main, windows: list }
}

fn profile_at(s: &Schedule, wall: Option<Wall>) -> Profile {
    s.pick(wall).0
}

// ---------------------------------------------------------------- what stands where no window does

#[test]
fn the_main_profile_stands_where_no_window_does() {
    for main in [Profile::Comfort, Profile::Eco, Profile::Off] {
        assert_eq!(schedule(main, &[]).pick(wall(MON, 12, 0)), (main, Source::Main, None), "{main:?}");
    }
}

// ---------------------------------------------------------------- a window opens and closes

#[test]
fn a_window_opens_at_its_start_and_closes_at_its_end() {
    let s = schedule(Profile::Comfort, &[between(ALL, hm(12, 0), hm(14, 0), Profile::Eco)]);
    assert_eq!(s.pick(wall(MON, 11, 59)), (Profile::Comfort, Source::Main, None));
    assert_eq!(s.pick(wall(MON, 12, 0)), (Profile::Eco, Source::Window(1), Some(hm(14, 0))));
    assert_eq!(profile_at(&s, wall(MON, 13, 59)), Profile::Eco, "the last minute of the window");
    assert_eq!(s.pick(wall(MON, 14, 0)), (Profile::Comfort, Source::Main, None), "its end is the first minute outside");
}

#[test]
fn a_window_of_another_number_says_its_own_number() {
    let s = schedule(Profile::Comfort, &[between(ALL, hm(1, 0), hm(2, 0), Profile::Off), between(ALL, hm(12, 0), hm(14, 0), Profile::Eco)]);
    assert_eq!(s.pick(wall(MON, 13, 0)).1, Source::Window(2));
    assert_eq!(s.pick(wall(MON, 1, 30)).1, Source::Window(1));
}

// ---------------------------------------------------------------- through midnight, by the days of the week

#[test]
fn a_window_through_midnight_belongs_to_the_day_it_starts_on() {
    let s = schedule(Profile::Comfort, &[between(WEEK, hm(22, 0), hm(6, 0), Profile::Eco)]);
    assert_eq!(profile_at(&s, wall(FRI, 23, 0)), Profile::Eco, "Friday evening, the window's own day");
    let sat_night = s.pick(wall(SAT, 3, 0));
    assert_eq!((sat_night.0, sat_night.2), (Profile::Eco, Some(hm(6, 0))), "Saturday at three is still the window that began on Friday");
    assert_eq!(profile_at(&s, wall(SAT, 6, 0)), Profile::Comfort, "closed at six");
    assert_eq!(profile_at(&s, wall(SAT, 23, 0)), Profile::Comfort, "no window begins on a Saturday");
    assert_eq!(profile_at(&s, wall(SUN, 3, 0)), Profile::Comfort, "none began on the Saturday");
    assert_eq!(profile_at(&s, wall(MON, 3, 0)), Profile::Comfort, "none began on the Sunday");
    assert_eq!(profile_at(&s, wall(MON, 23, 0)), Profile::Eco);
    assert_eq!(profile_at(&s, wall(MON + 1, 5, 59)), Profile::Eco);
    assert_eq!(profile_at(&s, wall(MON + 1, 6, 0)), Profile::Comfort);
}

#[test]
fn a_window_stands_on_the_days_it_names_and_on_no_other() {
    let s = schedule(Profile::Eco, &[between(END, hm(8, 0), hm(10, 0), Profile::Comfort)]);
    assert_eq!(profile_at(&s, wall(SAT, 9, 0)), Profile::Comfort);
    assert_eq!(profile_at(&s, wall(SUN, 9, 0)), Profile::Comfort);
    assert_eq!(profile_at(&s, wall(FRI, 9, 0)), Profile::Eco, "Friday is no day of the window");
    assert_eq!(profile_at(&s, wall(MON, 9, 0)), Profile::Eco);
}

// ---------------------------------------------------------------- every N hours

#[test]
fn every_n_hours_lifts_the_profile_for_m_minutes_from_the_time_it_starts_at() {
    let s = schedule(Profile::Eco, &[every(ALL, hm(7, 0), 3, 20, Profile::Comfort)]);
    let on = |h, m| profile_at(&s, wall(MON, h, m));
    assert_eq!(on(6, 59), Profile::Eco, "the last start of the night was at four, and the next is seven");
    assert_eq!(on(7, 0), Profile::Comfort);
    assert_eq!(on(7, 19), Profile::Comfort);
    assert_eq!(on(7, 20), Profile::Eco, "twenty minutes, and the twentieth is over");
    assert_eq!(on(9, 59), Profile::Eco);
    assert_eq!(on(10, 0), Profile::Comfort, "three hours after seven");
    assert_eq!(on(10, 20), Profile::Eco);
    assert_eq!(on(22, 0), Profile::Comfort, "seven, ten, thirteen, sixteen, nineteen, twenty-two");
    assert_eq!(on(22, 19), Profile::Comfort);
    assert_eq!(on(22, 20), Profile::Eco);
    assert_eq!(on(1, 0), Profile::Comfort, "twenty-two and three is one o'clock: the count goes on through the night, to the day's own time of starting");
    assert_eq!(on(4, 0), Profile::Comfort);
    assert_eq!(on(4, 20), Profile::Eco);
    let said = s.pick(wall(MON, 10, 5));
    assert_eq!((said.1, said.2), (Source::Window(1), Some(hm(10, 20))), "the card says when this one ends");
}

#[test]
fn an_every_that_runs_over_midnight_is_still_there_on_the_next_day_and_belongs_to_the_day_it_started_on() {
    let s = schedule(Profile::Eco, &[every(WEEK, hm(23, 30), 24, 60, Profile::Comfort)]);
    assert_eq!(profile_at(&s, wall(FRI, 23, 45)), Profile::Comfort);
    let after = s.pick(wall(SAT, 0, 10));
    assert_eq!((after.0, after.2), (Profile::Comfort, Some(hm(0, 30))));
    assert_eq!(profile_at(&s, wall(SAT, 0, 30)), Profile::Eco);
    assert_eq!(profile_at(&s, wall(SAT, 23, 45)), Profile::Eco, "Saturday is no day of it");
    assert_eq!(profile_at(&s, wall(SUN, 0, 10)), Profile::Eco, "and what began on a Saturday does not exist");
}

// THE COUNT OF AN EVERY STARTS AGAIN EVERY DAY AT ITS OWN TIME: the starts of the day a window belongs to are the time it starts at and every N hours after it, UP TO THE NEXT
// DAY'S OWN TIME - not to midnight, and not on through the days. A count that went on through the days would move the times by the hours that do not divide the day, and the same window would say
// other times on every day of the week; this one says the same times every day, and where N divides 24 the two ways of counting are one.

/// The minutes of the day at which a lifted profile begins on DAY, over a whole day of readings: the starts of the window as the schedule says them.
fn opens(s: &Schedule, day: u8) -> Vec<u16> {
    let at = |minute: u16| profile_at(s, Some(Wall { day, minute })) == Profile::Comfort;
    (0..1440).filter(|m| at(*m) && !at((*m + 1439) % 1440)).collect()
}

#[test]
fn five_hours_from_twenty_hundred_start_at_twenty_one_six_eleven_sixteen_and_twenty_again() {
    let s = schedule(Profile::Eco, &[every(ALL, hm(20, 0), 5, 30, Profile::Comfort)]);
    // the starts on a Tuesday, whichever day of the week it is: 01:00, 06:00, 11:00 and 16:00 are the tail of the Monday's count, 20:00 is the Tuesday's own start
    for day in 0..7 {
        assert_eq!(opens(&s, day), [hm(1, 0), hm(6, 0), hm(11, 0), hm(16, 0), hm(20, 0)], "day {day}");
    }
    let on = |day, h, m| profile_at(&s, wall(day, h, m));
    assert_eq!(on(MON, 20, 0), Profile::Comfort);
    assert_eq!(on(MON, 20, 29), Profile::Comfort);
    assert_eq!(on(MON, 20, 30), Profile::Eco);
    assert_eq!(on(MON + 1, 1, 0), Profile::Comfort, "five hours after twenty is one o'clock of the next day");
    assert_eq!(on(MON + 1, 16, 0), Profile::Comfort);
    assert_eq!(on(MON + 1, 21, 0), Profile::Eco, "twenty-one would be the next start of the count, but the day's own time comes first and starts it again at twenty");
    assert_eq!(on(MON + 1, 20, 0), Profile::Comfort, "the Tuesday's own start");
}

#[test]
fn the_starts_after_midnight_belong_to_the_day_the_count_began_on() {
    let s = schedule(Profile::Eco, &[every(MON_ONLY, hm(20, 0), 5, 30, Profile::Comfort)]);
    let on = |day, h, m| profile_at(&s, wall(day, h, m));
    assert_eq!(on(MON, 20, 0), Profile::Comfort);
    for (h, m) in [(1, 0), (6, 0), (11, 0), (16, 0)] {
        assert_eq!(on(MON + 1, h, m), Profile::Comfort, "Tuesday {h}:{m:02} is the Monday's count");
        assert_eq!(on(MON, h, m), Profile::Eco, "Monday {h}:{m:02}: the Sunday's count is no day of the window");
    }
    assert_eq!(on(MON + 1, 20, 0), Profile::Eco, "Tuesday is no day of the window, and its own start does not come");
    assert_eq!(on(MON + 2, 1, 0), Profile::Eco, "and so its count has no tail on the Wednesday");
    let said = s.pick(wall(MON + 1, 1, 10));
    assert_eq!((said.1, said.2), (Source::Window(1), Some(hm(1, 30))), "the card says when it ends, after midnight too");
}

#[test]
fn hours_that_divide_the_day_say_the_same_times_on_every_day_and_one_that_does_not_still_does() {
    let three = schedule(Profile::Eco, &[every(ALL, hm(7, 0), 3, 20, Profile::Comfort)]);
    let expect: Vec<u16> = (0..8).map(|k| (hm(7, 0) + k * 180) % 1440).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    for day in 0..7 {
        assert_eq!(opens(&three, day), expect, "three hours from seven, day {day}");
    }
    // seven hours from six: six, thirteen, twenty, and three the next day - the ten o'clock a count that went on would give is not there: the day's own six comes first and starts the count again
    let seven = schedule(Profile::Eco, &[every(ALL, hm(6, 0), 7, 20, Profile::Comfort)]);
    for day in 0..7 {
        assert_eq!(opens(&seven, day), [hm(3, 0), hm(6, 0), hm(13, 0), hm(20, 0)], "seven hours, day {day}: the times do not drift through the week");
    }
}

// ---------------------------------------------------------------- the windows among themselves

#[test]
fn where_two_windows_stand_together_the_lower_number_wins() {
    let s = schedule(Profile::Comfort, &[between(ALL, hm(22, 0), hm(6, 0), Profile::Eco), between(ALL, hm(23, 0), hm(5, 0), Profile::Off)]);
    assert_eq!(s.pick(wall(MON, 23, 30)).1, Source::Window(1));
    assert_eq!(profile_at(&s, wall(MON, 23, 30)), Profile::Eco);
}

#[test]
fn a_window_with_no_profile_is_off_the_schedule_whole_and_the_next_one_stands() {
    let mut off = between(ALL, hm(22, 0), hm(6, 0), Profile::Eco);
    off.profile = None;
    let s = schedule(Profile::Comfort, &[off, between(ALL, hm(23, 0), hm(5, 0), Profile::Off)]);
    assert_eq!(s.pick(wall(MON, 22, 30)).1, Source::Main, "the first would have stood, and does not");
    assert_eq!(s.pick(wall(MON, 23, 30)).1, Source::Window(2));
}

// ---------------------------------------------------------------- the time is not known

#[test]
fn where_the_board_does_not_know_the_time_the_main_profile_stands_and_the_answer_says_so() {
    let s = schedule(Profile::Comfort, &[between(ALL, hm(0, 0), hm(23, 59), Profile::Eco)]);
    assert_eq!(s.pick(None), (Profile::Comfort, Source::NoClock, None));
}

#[test]
fn with_no_window_in_use_the_missing_time_is_nothing_to_say() {
    let mut off = between(ALL, hm(0, 0), hm(23, 59), Profile::Eco);
    off.profile = None;
    assert_eq!(schedule(Profile::Eco, &[]).pick(None).1, Source::Main);
    assert_eq!(schedule(Profile::Eco, &[off]).pick(None).1, Source::Main, "a window with no profile is no window");
}

// ---------------------------------------------------------------- the date to the day of the week

#[test]
fn a_date_is_a_day_of_the_week_counted_from_monday() {
    for ((y, m, d), day) in [((2026, 10, 3), SAT), ((2026, 10, 4), SUN), ((2026, 10, 5), MON), ((2024, 2, 29), 3), ((2026, 12, 31), 3), ((2000, 1, 1), SAT), ((1970, 1, 1), 3)] {
        assert_eq!(Wall::of_date(y, m, d, 90), Wall { day, minute: 90 }, "{y}-{m}-{d}");
    }
}
