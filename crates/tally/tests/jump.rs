//! A count that lives through a jump of the clock. On 06.10 the time of day jumped to 2027-11-17 for about five
//! seconds; the count placed itself in that day, days turn only forwards, and "today" equalled "this month" from then on.
//! Expected from what a person reading the page would say: the day is the real day, today starts from nought in a new
//! day, the total never goes down - not from the code that does it.
//!
//! The board is played second by second: the steady clock runs, the time of day is the steady clock plus a shift that
//! a leap changes, the count is added to once a second, and the watch says whether the time is to be believed.

use tally::{Date, Now, Tally, Watch, LEAP_MS, STILL_MS};

const DAY_MS: i64 = 86_400_000;
/// 2026-10-10 12:00 UTC.
fn noon() -> i64 {
    Date { year: 2026, month: 10, day: 10 }.days() * DAY_MS + 12 * 3_600_000
}

fn date(year: u16, month: u8, day: u8) -> Date {
    Date { year, month, day }
}

/// The date of a day number since 1970-01-01 (the civil calendar, as the board's libc counts it).
fn civil(days: i64) -> Date {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    Date { year: (yoe + era * 400 + i64::from(m <= 2)) as u16, month: m as u8, day: d as u8 }
}

/// The local time of an instant given in milliseconds since the epoch, at OFFSET_MIN minutes east of UTC.
fn local(utc_ms: i64, offset_min: i16) -> Now {
    let ms = utc_ms + i64::from(offset_min) * 60_000;
    let days = ms.div_euclid(DAY_MS);
    let minute = (ms.rem_euclid(DAY_MS) / 60_000) as u16;
    Now { date: civil(days), minute, offset_min }
}

/// A board with a count of one thing, one unit counted every second.
struct Board {
    /// The steady clock, milliseconds.
    steady_ms: i64,
    /// Where the board started, UTC.
    start_utc_ms: i64,
    /// What a leap added to the time of day.
    shift_ms: i64,
    offset_min: i16,
    /// A server answered since the chip's counter last leapt.
    answered: bool,
    /// How many answers of a server there have been since the board started.
    answers: u32,
    tally: Tally<1>,
    watch: Watch,
    counted: u64,
}

impl Board {
    fn starting(utc_ms: i64) -> Board {
        Board { steady_ms: 0, start_utc_ms: utc_ms, shift_ms: 0, offset_min: 180, answered: true, answers: 0, tally: Tally::default(), watch: Watch::new(), counted: 0 }
    }

    /// The real instant, UTC.
    fn real_utc_ms(&self) -> i64 {
        self.start_utc_ms + self.steady_ms
    }

    fn real_date(&self) -> Date {
        local(self.real_utc_ms(), self.offset_min).date
    }

    /// One second: the count is added to, the clock is read, the count is placed.
    fn second(&mut self) -> Option<Now> {
        self.steady_ms += 1000;
        self.tally.add([1]);
        self.counted += 1;
        let wall = Some(local(self.real_utc_ms() + self.shift_ms, self.offset_min));
        let believed = self.watch.believe(wall, self.steady_ms as u64, self.answered, self.answers);
        self.tally.at(believed);
        believed
    }

    fn seconds(&mut self, n: u32) {
        for _ in 0..n {
            self.second();
        }
    }

    /// Like `seconds`, and the day the count stands in is never a day after the real one.
    fn seconds_never_in_the_future(&mut self, n: u32) {
        for _ in 0..n {
            self.second();
            if let Some(day) = self.tally.day {
                assert!(day <= self.real_date(), "the count stands in {day:?}, real {:?}", self.real_date());
            }
        }
    }

    /// The chip's counter leapt: the time of day moved by MS, and no server has answered since.
    fn leap(&mut self, ms: i64) {
        self.shift_ms += ms;
        self.answered = false;
    }

    /// A server answered: the time of day is the real one again.
    fn server_answers(&mut self) {
        self.shift_ms = 0;
        self.answered = true;
        self.answers += 1;
    }
}

#[test]
fn a_leap_forward_of_a_year_that_a_server_corrects_leaves_the_count_in_its_real_day_and_loses_nothing() {
    let mut b = Board::starting(noon());
    b.seconds_never_in_the_future(120);
    assert_eq!(b.tally.sums.today, [120], "the time is believed after a minute, and what waited is in today");
    b.leap(365 * DAY_MS);
    b.seconds_never_in_the_future(5);
    b.server_answers();
    // the server has answered, but the time has not held for a minute yet
    b.seconds_never_in_the_future(59);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)));
    b.seconds_never_in_the_future(2);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)));
    assert_eq!((b.tally.sums.today, b.tally.sums.month, b.tally.sums.total), ([b.counted], [b.counted], [b.counted]), "all of it was burned today");
    assert_eq!(b.tally.pending, [0]);
}

#[test]
fn a_leap_forward_that_no_server_corrects_stays_out_of_the_days_however_long_it_stands() {
    let mut b = Board::starting(noon());
    b.seconds_never_in_the_future(120);
    let before = b.counted;
    b.leap(365 * DAY_MS);
    // ten minutes of the shifted time, steady, and nobody answering: it is held for a minute and more, and still not believed
    b.seconds_never_in_the_future(600);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)), "the day did not move");
    assert_eq!(b.tally.sums.today, [before], "what was counted since waits, it is not in a day");
    assert_eq!(b.tally.pending, [600]);
    assert_eq!(b.tally.sums.total, [b.counted], "and it is in all");
    // the server answers at last: what waited lands in the real day
    b.server_answers();
    b.seconds_never_in_the_future(61);
    assert_eq!((b.tally.sums.today, b.tally.sums.total, b.tally.pending), ([b.counted], [b.counted], [0]));
}

#[test]
fn a_leap_back_is_treated_as_a_leap_forward_is() {
    let mut b = Board::starting(noon());
    b.seconds_never_in_the_future(120);
    b.leap(-400 * DAY_MS);
    b.seconds_never_in_the_future(300);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)));
    b.server_answers();
    b.seconds_never_in_the_future(61);
    assert_eq!((b.tally.sums.today, b.tally.sums.total), ([b.counted], [b.counted]));
}

#[test]
fn an_ordinary_midnight_turns_the_day_as_before_with_the_month_kept() {
    // 23:58 local, 10.10: two minutes to midnight
    let mut b = Board::starting(Date { year: 2026, month: 10, day: 10 }.days() * DAY_MS + 20 * 3_600_000 + 58 * 60_000);
    b.seconds_never_in_the_future(150);
    assert_eq!(b.tally.day, Some(date(2026, 10, 11)), "the day turned at midnight");
    // the second that showed the new date belongs to the day before it: the 30 after it are today's
    assert_eq!(b.tally.sums.today, [30]);
    assert_eq!((b.tally.sums.month, b.tally.sums.total), ([150], [150]), "the month goes on and so does the total");
}

#[test]
fn an_ordinary_month_end_zeroes_the_month_and_keeps_the_total() {
    let mut b = Board::starting(Date { year: 2026, month: 10, day: 31 }.days() * DAY_MS + 20 * 3_600_000 + 58 * 60_000);
    b.seconds_never_in_the_future(150);
    assert_eq!(b.tally.day, Some(date(2026, 11, 1)));
    assert_eq!((b.tally.sums.today, b.tally.sums.month, b.tally.sums.total), ([30], [30], [150]));
}

#[test]
fn a_time_of_day_is_not_believed_before_a_minute_has_held_and_not_without_a_server_after_a_leap() {
    let mut w = Watch::new();
    let now = |s: i64| Some(local(noon() + s * 1000, 180));
    assert_eq!(w.believe(now(0), 0, true, 0), None, "the first reading has held for nothing yet");
    assert_eq!(w.believe(now(30), 30_000, true, 0), None);
    assert_eq!(w.believe(now(STILL_MS as i64 / 1000), STILL_MS, true, 0), now(STILL_MS as i64 / 1000), "a minute held, and no leap since the start");
    assert_eq!(w.believe(now(90), 90_000, false, 0), None, "a leap and no answer since: not believed, however long the time has held");
    assert_eq!(w.believe(now(900), 900_000, false, 0), None);
    assert_eq!(w.believe(now(901), 901_000, true, 1), None, "the answer starts a minute of its own, however long the time held before it");
    assert_eq!(w.believe(now(960), 960_000, true, 1), None);
    assert_eq!(w.believe(now(961), 961_000, true, 1), now(961), "a minute after the answer");
}

#[test]
fn a_time_that_is_lost_is_asked_to_hold_for_a_minute_again() {
    let mut w = Watch::new();
    let now = |s: i64| Some(local(noon() + s * 1000, 180));
    w.believe(now(0), 0, true, 0);
    assert!(w.believe(now(61), 61_000, true, 0).is_some());
    assert_eq!(w.believe(None, 62_000, true, 0), None);
    assert_eq!(w.believe(now(63), 63_000, true, 0), None, "the time came back: a minute again");
    assert!(w.believe(now(124), 124_000, true, 0).is_some());
}

#[test]
fn a_change_of_the_zone_or_of_summer_time_is_no_leap_because_the_instant_did_not_move() {
    let mut b = Board::starting(noon());
    b.seconds_never_in_the_future(120);
    // summer time ends: the local time goes back an hour, the instant goes on
    b.offset_min = 120;
    for _ in 0..10 {
        assert!(b.second().is_some(), "the time stopped being believed at a change of the zone");
    }
    assert_eq!(b.tally.sums.today, [b.counted], "all of it still counted in the day");
}

#[test]
fn a_step_of_the_time_smaller_than_the_tolerance_is_not_a_leap() {
    let mut w = Watch::new();
    let now = |s: i64, shift: i64| Some(local(noon() + s * 1000 + shift, 180));
    w.believe(now(0, 0), 0, true, 0);
    assert!(w.believe(now(61, 0), 61_000, true, 0).is_some());
    assert!(w.believe(now(62, LEAP_MS / 2), 62_000, true, 0).is_some(), "the time moved by less than the tolerance: still believed");
    assert_eq!(w.believe(now(63, 3 * LEAP_MS), 63_000, true, 0), None, "three tolerances is a leap");
}

#[test]
fn a_day_recorded_from_the_future_is_put_right_when_the_time_is_believed_and_the_total_is_not_touched() {
    // what the boiler held on 09.10: the day of a leap, today equal to the month, the total of all burning
    let mut t = Tally::<1>::default();
    t.day = Some(date(2028, 2, 27));
    t.sums.today = [192_555_360];
    t.sums.month = [192_555_360];
    t.sums.total = [301_081_440];
    t.at(Some(local(noon(), 180)));
    assert_eq!(t.day, Some(date(2026, 10, 10)), "the real day");
    assert_eq!(t.sums.today, [0], "today starts from nought: what is under a day that never came is no day's");
    assert_eq!(t.sums.month, [192_555_360], "the month is a short count, not a false one: it stays");
    assert_eq!(t.sums.total, [301_081_440], "the total is never touched");
}

#[test]
fn what_waited_for_the_time_lands_in_the_real_day_when_a_recorded_future_day_is_put_right() {
    let mut t = Tally::<1>::default();
    t.day = Some(date(2028, 2, 27));
    t.sums = tally::Sums { today: [100], month: [100], total: [100 + 7] };
    t.pending = [7];
    t.at(Some(local(noon(), 180)));
    assert_eq!((t.sums.today, t.sums.month, t.sums.total, t.pending), ([7], [107], [107], [0]));
}

#[test]
fn a_recorded_day_a_day_ahead_is_left_as_it_was_and_two_days_ahead_is_put_right() {
    let at = |recorded: Date| {
        let mut t = Tally::<1>::default();
        t.day = Some(recorded);
        t.sums = tally::Sums { today: [5], month: [9], total: [20] };
        t.at(Some(local(noon(), 180)));
        t
    };
    let one = at(date(2026, 10, 11));
    assert_eq!((one.day, one.sums.today), (Some(date(2026, 10, 11)), [5]), "a day ahead is a zone or a summer time: closes nothing");
    let two = at(date(2026, 10, 12));
    assert_eq!((two.day, two.sums.today, two.sums.month, two.sums.total), (Some(date(2026, 10, 10)), [0], [9], [20]));
}

#[test]
fn a_recorded_day_that_is_the_real_day_or_an_earlier_one_is_not_a_future_day() {
    let mut same = Tally::<1>::default();
    same.day = Some(date(2026, 10, 10));
    same.sums = tally::Sums { today: [5], month: [9], total: [20] };
    same.at(Some(local(noon(), 180)));
    assert_eq!((same.day, same.sums.today), (Some(date(2026, 10, 10)), [5]));
    let mut earlier = Tally::<1>::default();
    earlier.day = Some(date(2026, 10, 9));
    earlier.sums = tally::Sums { today: [5], month: [9], total: [20] };
    earlier.at(Some(local(noon(), 180)));
    assert_eq!((earlier.day, earlier.sums.today, earlier.sums.month), (Some(date(2026, 10, 10)), [0], [9]), "a day behind turns forward, as it always did");
}

#[test]
fn the_total_never_goes_down_through_a_leap_and_what_puts_it_right() {
    let mut b = Board::starting(noon());
    let mut last = 0;
    let mut check = |b: &Board| {
        assert!(b.tally.sums.total[0] >= last, "the total went down");
        last = b.tally.sums.total[0];
    };
    for _ in 0..120 {
        b.second();
        check(&b);
    }
    b.leap(365 * DAY_MS);
    for _ in 0..600 {
        b.second();
        check(&b);
    }
    b.server_answers();
    for _ in 0..61 {
        b.second();
        check(&b);
    }
    assert_eq!(b.tally.sums.total, [b.counted]);
}

#[test]
fn a_leap_the_clock_told_but_smaller_than_the_tolerance_restarts_the_minute_when_the_server_answers() {
    // a leap of 30 s: the steady clock tells it (it takes more than 2 s of leap), the watch alone takes it for the time's own tremor
    assert!(30_000 < LEAP_MS);
    let mut w = Watch::new();
    let now = |s: i64, shift: i64| Some(local(noon() + s * 1000 + shift, 180));
    assert_eq!(w.believe(now(0, 0), 0, true, 0), None);
    assert!(w.believe(now(70, 0), 70_000, true, 0).is_some(), "a minute held before the leap");
    assert_eq!(w.believe(now(71, 30_000), 71_000, false, 0), None, "the leap, and no server has answered");
    for s in 72_i64..200 {
        assert_eq!(w.believe(now(s, 30_000), (s * 1000) as u64, false, 0), None, "no answer: not believed, however long the time holds");
    }
    // the server answers: the time is put right, and the minute begins again from the answer
    assert_eq!(w.believe(now(200, 0), 200_000, true, 1), None, "the answer itself is not a minute held");
    assert_eq!(w.believe(now(259, 0), 259_000, true, 1), None, "59 s after the answer");
    assert!(w.believe(now(260, 0), 260_000, true, 1).is_some(), "60 s after the answer");
}

#[test]
fn what_is_counted_after_such_a_leap_waits_for_the_new_minute_and_then_lands_in_today() {
    let mut b = Board::starting(noon());
    b.seconds_never_in_the_future(120);
    b.leap(30_000);
    b.seconds_never_in_the_future(100);
    b.server_answers();
    let placed = b.tally.sums.today;
    // the minute runs from the first reading after the answer
    b.seconds_never_in_the_future(60);
    assert_eq!(b.tally.sums.today, placed, "nothing was placed in the minute after the answer");
    b.seconds_never_in_the_future(1);
    assert_eq!((b.tally.sums.today, b.tally.pending), ([b.counted], [0]), "after the minute all of it lands in today");
}

#[test]
fn an_answer_that_was_never_missed_does_not_restart_the_minute() {
    // no leap, the flag never went down: the minute runs from the first reading, as it did
    let mut w = Watch::new();
    let now = |s: i64| Some(local(noon() + s * 1000, 180));
    w.believe(now(0), 0, true, 0);
    assert!(w.believe(now(60), 60_000, true, 0).is_some());
    assert!(w.believe(now(61), 61_000, true, 0).is_some());
}

#[test]
fn a_count_is_not_sure_of_its_day_until_the_time_is_believed_and_is_not_while_it_is_lost() {
    let mut b = Board::starting(noon());
    assert!(!b.tally.sure_of_day(), "a count that has not met a believed time is not sure of its day");
    b.seconds(59);
    assert!(!b.tally.sure_of_day(), "a minute has not held yet");
    b.seconds(2);
    assert!(b.tally.sure_of_day(), "the time is believed: the count is sure");
    // a leap: the time is not believed, and the count is not sure of the day until it is again
    b.leap(365 * DAY_MS);
    b.seconds(1);
    assert!(!b.tally.sure_of_day());
    b.seconds(600);
    assert!(!b.tally.sure_of_day(), "no server has answered: however long the shifted time stands");
    b.server_answers();
    b.seconds(60);
    assert!(!b.tally.sure_of_day(), "a minute is not over after the answer");
    b.seconds(2);
    assert!(b.tally.sure_of_day(), "put right and held for a minute");
}

#[test]
fn a_count_with_a_kept_day_is_not_sure_of_it_until_the_time_is_believed() {
    // what restore gives: the day and the sums of the text, no time met since the start
    let mut t = Tally::<1>::default();
    t.day = Some(date(2028, 2, 27));
    t.sums = tally::Sums { today: [192_555_360], month: [192_555_360], total: [301_081_440] };
    assert!(!t.sure_of_day(), "the numbers are the kept ones: not a day the board has seen");
    t.at(None);
    assert!(!t.sure_of_day(), "no time yet");
    t.at(Some(local(noon(), 180)));
    assert!(t.sure_of_day(), "a believed time puts the day right and the count is sure of it");
    assert_eq!(t.sums.today, [0]);
}
