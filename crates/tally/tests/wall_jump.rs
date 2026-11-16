//! A jump of the WALL clock - the time of day moves while the chip's counter does not - is no day of the count unless a server's
//! answer came with it. 23.10: the time of day is the counter's plus an offset, and it moves for two reasons only: SNTP set it
//! (an answer, counted), or something else moved it. An answer that corrects the time is a legal jump; a jump with no answer is a
//! leap that nobody has put right, and the count must not stand in its day. Expected from what a person reading "yesterday" would
//! say - the day before today as it really was - not from the code that judges the time.
//!
//! The board is played second by second like tests/jump.rs; here the chip's counter never leaps (`answered` stays true all along),
//! only the time of day does.

use tally::{Date, Now, Tally, Watch, Yesterday};

const DAY_MS: i64 = 86_400_000;

fn date(year: u16, month: u8, day: u8) -> Date {
    Date { year, month, day }
}

/// The date of a day number since 1970-01-01.
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

fn local(utc_ms: i64, offset_min: i16) -> Now {
    let ms = utc_ms + i64::from(offset_min) * 60_000;
    Now { date: civil(ms.div_euclid(DAY_MS)), minute: (ms.rem_euclid(DAY_MS) / 60_000) as u16, offset_min }
}

struct Board {
    steady_ms: i64,
    start_utc_ms: i64,
    /// What the time of day has moved by, besides the steady clock.
    shift_ms: i64,
    /// How many answers of a server there have been.
    answers: u32,
    tally: Tally<1>,
    watch: Watch,
}

impl Board {
    /// A board that starts at LOCAL hour and minute on 2026-10-10 (UTC+3), the count already knowing the day before: 7 units on the 9th.
    fn starting(hour: u16, minute: u16) -> Board {
        let utc = date(2026, 10, 10).days() * DAY_MS + (i64::from(hour) * 60 + i64::from(minute)) * 60_000 - 180 * 60_000;
        let mut tally = Tally::<1>::default();
        tally.yesterday = Some(Yesterday { date: date(2026, 10, 9), sums: [7] });
        Board { steady_ms: 0, start_utc_ms: utc, shift_ms: 0, answers: 0, tally, watch: Watch::new() }
    }

    /// One second: a unit counted, the clock read, the count placed. The answers are read before the time, as the board reads them.
    fn second(&mut self) {
        self.steady_ms += 1000;
        self.tally.add([1]);
        let answers = self.answers;
        let wall = Some(local(self.start_utc_ms + self.steady_ms + self.shift_ms, 180));
        let believed = self.watch.believe(wall, self.steady_ms as u64, true, answers);
        self.tally.at(believed);
    }

    fn seconds(&mut self, n: u32) {
        for _ in 0..n {
            self.second();
        }
    }

    /// The time of day moved by MS with nobody answering.
    fn jump(&mut self, ms: i64) {
        self.shift_ms += ms;
    }

    /// A server answered, and what it says is the time of day as it stands now.
    fn server_answers_with_the_time_as_it_is(&mut self) {
        self.answers += 1;
    }

    /// A server answered, and what it says is the real time: the time of day goes back to it.
    fn server_corrects(&mut self) {
        self.shift_ms = 0;
        self.answers += 1;
    }
}

/// A board that has run an hour of its real day (the 10th) and is believed.
fn settled() -> Board {
    let mut b = Board::starting(12, 0);
    b.seconds(3600);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)));
    assert_eq!(b.tally.sums.today, [3600]);
    b
}

#[test]
fn a_jump_of_a_day_forward_with_no_answer_changes_neither_today_nor_yesterday() {
    let mut b = settled();
    b.jump(DAY_MS);
    b.seconds(600);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)), "the count does not stand in the day of a jump");
    assert_eq!(b.tally.yesterday, Some(Yesterday { date: date(2026, 10, 9), sums: [7] }), "yesterday is the 9th, as it was");
    assert_eq!(b.tally.sums.today[0] + b.tally.pending[0], 4200, "what was counted is today's or waits for the time: nothing is lost, nothing is moved");
}

#[test]
fn a_jump_of_three_days_with_no_answer_leaves_the_kept_yesterday_whole() {
    let mut b = settled();
    b.jump(3 * DAY_MS);
    b.seconds(600);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)));
    assert_eq!(b.tally.yesterday, Some(Yesterday { date: date(2026, 10, 9), sums: [7] }), "three days of a false calendar do not wipe the kept day");
}

#[test]
fn a_jump_back_with_no_answer_closes_nothing() {
    let mut b = settled();
    b.jump(-3 * DAY_MS);
    b.seconds(600);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)), "the day does not go back with the clock");
    assert!(b.tally.sums.today[0] + b.tally.pending[0] >= 3600, "today is not zeroed by a day that never came");
    assert_eq!(b.tally.yesterday, Some(Yesterday { date: date(2026, 10, 9), sums: [7] }));
}

#[test]
fn a_real_midnight_with_no_jump_makes_yesterday_of_the_day_that_ended() {
    // 23:30 local, run through midnight: 1800 s of the 10th, then the 11th
    let mut b = Board::starting(23, 30);
    b.seconds(1860);
    assert_eq!(b.tally.day, Some(date(2026, 10, 11)));
    assert_eq!(b.tally.yesterday, Some(Yesterday { date: date(2026, 10, 10), sums: [1800] }), "what burned by midnight");
}

#[test]
fn a_jump_that_a_server_answers_is_taken_and_the_day_turns_by_it() {
    let mut b = settled();
    b.jump(DAY_MS);
    b.seconds(100);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)), "not yet: nobody has answered");
    b.server_answers_with_the_time_as_it_is();
    b.seconds(30);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)), "the answer starts a minute of quiet, as every correction of the time does");
    b.seconds(40);
    assert_eq!(b.tally.day, Some(date(2026, 10, 11)), "a server says it is the 11th");
    // what was counted while the time was not believed waited for it and lands in the day the time names: the count cannot tell which day it was counted in
    assert_eq!(b.tally.yesterday, Some(Yesterday { date: date(2026, 10, 10), sums: [3600] }), "the 10th as the count had placed it before the jump");
    assert_eq!(b.tally.sums.today[0] + b.tally.pending[0], 170, "the seconds that waited are today's");
}

#[test]
fn a_jump_that_a_server_corrects_back_is_never_taken() {
    let mut b = settled();
    b.jump(DAY_MS);
    b.seconds(100);
    b.server_corrects();
    b.seconds(200);
    assert_eq!(b.tally.day, Some(date(2026, 10, 10)));
    assert_eq!(b.tally.yesterday, Some(Yesterday { date: date(2026, 10, 9), sums: [7] }));
}

#[test]
fn an_answer_that_changes_nothing_blanks_nothing() {
    // the hourly answer of a server that finds the time right: no minute of "not sure" for the card
    let mut b = settled();
    b.server_answers_with_the_time_as_it_is();
    b.seconds(5);
    assert!(b.tally.sure_of_day(), "the count stays sure of its day");
}
