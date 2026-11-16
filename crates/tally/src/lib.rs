//! A count with its day, its month and its total, for whatever a board counts: electricity (crates/energy), pellets
//! (crates/pellets). No hardware and no clock here: the board hands in what was counted and the local time when it
//! knows it, and this crate says which day and month it belongs to, so all of it is tested on the computer.
//!
//! What is counted while the board does not know the time is in the total at once and waits; it is placed in a day
//! when the time comes. Days and months turn only forwards: a clock that jumps back closes nothing.
//!
//! THE TIME HANDED IN IS THE TIME BELIEVED. A jump of the clock - the chip's counter leapt and the time of day with it -
//! must not become a day of the count, for days turn only forwards and a day from the future would stand until the real
//! date caught up with it (06.10: a day of 2027, then 2028; "today" equalled "this month" for days). [`Watch`] judges the
//! time before the count sees it, and a day recorded from a time that was false is put right (`turn_and_settle`).
//!
//! The count lives on the board as text that outlives a power cut ([`Lines`] reads it, [`numbers`] writes a line of it);
//! [`Keeper`] writes the texts in the order they were taken.

/// A local date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

/// The local time when the board knows it: the date, the minute of the day, and the offset from UTC in minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Now {
    pub date: Date,
    pub minute: u16,
    pub offset_min: i16,
}

impl Date {
    /// "2026-09-19".
    pub fn words(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// A date from its words, or why not.
    pub fn parse(text: &str) -> Result<Date, String> {
        let bad = || format!("not a date: '{text}'");
        let mut parts = text.split('-');
        let (Some(y), Some(m), Some(d), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
            return Err(bad());
        };
        let date = Date { year: y.parse().map_err(|_| bad())?, month: m.parse().map_err(|_| bad())?, day: d.parse().map_err(|_| bad())? };
        if !(1..=12).contains(&date.month) || !(1..=date.days_in_month()).contains(&date.day) {
            return Err(bad());
        }
        Ok(date)
    }

    /// Days since 1970-01-01 (the civil calendar), so two dates can be told apart by how many days lie between them.
    pub fn days(&self) -> i64 {
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let yoe = year.rem_euclid(400);
        let month = i64::from(self.month);
        let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Days in this date's month, February of a leap year with 29.
    fn days_in_month(&self) -> u8 {
        match self.month {
            4 | 6 | 9 | 11 => 30,
            2 if self.year % 4 == 0 && (self.year % 100 != 0 || self.year % 400 == 0) => 29,
            2 => 28,
            _ => 31,
        }
    }
}

impl Now {
    /// The instant in milliseconds since 1970-01-01 UTC, to the minute: the local time less its offset, so a change of
    /// the zone or of summer time, which moves the local time and not the instant, moves nothing here.
    pub fn utc_ms(&self) -> i64 {
        ((self.date.days() * 1440 + i64::from(self.minute)) - i64::from(self.offset_min)) * 60_000
    }
}

/// How many days ahead of the believed date a recorded day may stand and be left alone: the date goes back by a day at
/// most when the zone changes or summer time ends, and days turn only forwards, so the real date catches up on its own.
pub const FUTURE_DAYS: i64 = 1;

/// How long the time of day has to hold steady, in milliseconds, before it is believed.
pub const STILL_MS: u64 = 60_000;

/// How far, in milliseconds, the time of day may move against the steady clock between two readings and still be no
/// leap: the time comes to the minute, so two readings differ by a minute on their own.
pub const LEAP_MS: i64 = 120_000;

/// Whether the time of day is to be believed.
///
/// The time of day is believed when a server has answered since the chip's counter last leapt - or it has not leapt - AND
/// the time of day has held steady against the board's steady clock for [`STILL_MS`]. Held steady: between two readings it
/// moved by what the steady clock moved, within [`LEAP_MS`]. A leap breaks the hold at once, whichever way it goes; the
/// answer of a server that corrects it breaks the hold again, and a minute of quiet after it is what makes the time
/// believed. The answer starts that minute by itself, whatever the leap measured: the steady clock takes a leap of more
/// than two seconds for one, a leap smaller than [`LEAP_MS`] is no break of the hold for the watch alone, and the minute
/// would then have aged while the server was silent. Both conditions are needed: a server's answer says the time was put right, not that it stays; the quiet says
/// it stays, not that it is the real one - a leap that no server answers settles into a time that has held for any
/// length, and is still false.
///
/// Compared on the instant (UTC), so a change of the zone or of summer time, which moves the local time by an hour
/// while the instant goes on, is no leap.
#[derive(Clone, Debug, Default)]
pub struct Watch {
    /// The time of day less the steady clock at the last reading, in milliseconds; None while the time is not known.
    offset_ms: Option<i64>,
    /// The steady moment from which the offset has held.
    held_since_ms: u64,
    /// A server had answered since the last leap at the last reading.
    answered: bool,
    /// How many answers of a server there had been at the last reading; None before the first.
    answers: Option<u32>,
    /// The time of day leapt and no answer of a server came with it or after it: it is a leap nobody has put right.
    unexplained: bool,
}

impl Watch {
    pub const fn new() -> Self {
        Watch { offset_ms: None, held_since_ms: 0, answered: true, answers: None, unexplained: false }
    }

    /// The time of day WALL as the board reads it, None while it does not know it, judged at STEADY_MS of the steady clock;
    /// ANSWERED is whether a server has answered since the chip's counter last leapt; ANSWERS is how many answers of a server there have
    /// been since the board started, read BEFORE the time of day: the count moves after the time is set. Some - the time to hand to
    /// the count - only when it is believed.
    ///
    /// The time of day moves for two reasons only: a server's answer set it, or something else did. A move of more than [`LEAP_MS`]
    /// against the steady clock that comes in the reading where the count of answers grew is the answer's correction, and a minute of
    /// quiet makes it believed. A move with no answer is a leap nobody has put right (23.10: the wall clock leapt a day while the chip's
    /// counter stood, and the count would have stood in that day): it is not believed, whatever has held, until an answer comes - and
    /// then the answer starts the minute, as every correction does. An answer that moves nothing changes nothing.
    pub fn believe(&mut self, wall: Option<Now>, steady_ms: u64, answered: bool, answers: u32) -> Option<Now> {
        let answer_came = self.answers.is_some_and(|seen| seen != answers);
        self.answers = Some(answers);
        // the answer of a server after a leap: the time was put right just now, and a minute of quiet begins from it
        if answered && !self.answered {
            self.held_since_ms = steady_ms;
        }
        self.answered = answered;
        // the answer a leap of the time of day waited for
        if answer_came && self.unexplained {
            self.unexplained = false;
            self.held_since_ms = steady_ms;
        }
        let Some(now) = wall else {
            self.offset_ms = None;
            return None;
        };
        let offset = now.utc_ms() - steady_ms as i64;
        match self.offset_ms {
            Some(last) if (offset - last).abs() <= LEAP_MS => {}
            // a leap of the time of day: the hold begins again, and it was no correction unless an answer came with it
            Some(_) => {
                self.held_since_ms = steady_ms;
                self.unexplained = !answer_came;
            }
            // the first reading of a time: the hold begins
            None => self.held_since_ms = steady_ms,
        }
        self.offset_ms = Some(offset);
        (answered && !self.unexplained && steady_ms.saturating_sub(self.held_since_ms) >= STILL_MS).then_some(now)
    }
}

/// Today, this month and in all, per output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sums<const N: usize> {
    pub today: [u64; N],
    pub month: [u64; N],
    pub total: [u64; N],
}

impl<const N: usize> Default for Sums<N> {
    fn default() -> Self {
        Sums { today: [0; N], month: [0; N], total: [0; N] }
    }
}

/// What a day came to, and which day it was: the count of the day that turned, kept when the next one began.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Yesterday<const N: usize> {
    pub date: Date,
    pub sums: [u64; N],
}

impl<const N: usize> Yesterday<N> {
    /// The day as text for a file of its own, the first line HEADER: the date, then what the day came to.
    pub fn text(&self, header: &str) -> String {
        format!("{header}\n{}{}", day_line(Some(self.date)), numbers("sums", &self.sums))
    }

    /// The day from its text, or why the text is not one of the kind HEADER names.
    pub fn parse(text: &str, header: &str) -> Result<Yesterday<N>, String> {
        let mut lines = Lines::new(text, header)?;
        let date = lines.day()?.ok_or("day: no day")?;
        let sums = lines.numbers("sums")?;
        lines.end()?;
        Ok(Yesterday { date, sums })
    }
}

/// The count of N outputs of one thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tally<const N: usize> {
    pub sums: Sums<N>,
    /// What the day before the day `today` belongs to came to; None while the board has not seen a day turn.
    pub yesterday: Option<Yesterday<N>>,
    /// The day `today` belongs to; None until the board first knew the time.
    pub day: Option<Date>,
    /// Counted while the board did not know the time: in `total` already, placed in a day when the time comes.
    pub pending: [u64; N],
    /// The board knew the time when the count was last placed: what came since belongs to the day that was, not to
    /// the one a turn opens. Not kept across a restart.
    pub timed: bool,
}

impl<const N: usize> Default for Tally<N> {
    fn default() -> Self {
        Tally { sums: Sums::default(), yesterday: None, day: None, pending: [0; N], timed: false }
    }
}

impl<const N: usize> Tally<N> {
    /// Counts V: in all at once, in a day when the time is known.
    pub fn add(&mut self, v: [u64; N]) {
        for n in 0..N {
            self.sums.total[n] += v[n];
            self.pending[n] += v[n];
        }
    }

    /// Whether the count is sure of its day: the board has been handed a believed time since it started (see [`Watch`]) and has
    /// not lost it since. Until then `today` and `month` are the kept numbers of a day the board has not seen, or a day that may be
    /// no day at all, and a reader that draws them says so: the total is a number always, for nothing is ever taken from it.
    pub fn sure_of_day(&self) -> bool {
        self.timed
    }

    /// What the day before today came to, when the count is sure of its day and the kept day is the one before it.
    pub fn yesterday_sums(&self) -> Option<[u64; N]> {
        let (kept, day) = (self.yesterday?, self.day?);
        (self.timed && kept.date.days() + 1 == day.days()).then_some(kept.sums)
    }

    /// The board does not know the time now.
    pub fn untimed(&mut self) {
        self.timed = false;
    }

    /// Places what waits in the day that was, when the board knew the time at the last placing; what it placed, or
    /// nothing. Called before a turn is looked at, so what came before it stays in the day before it.
    pub fn settle_if_timed(&mut self) -> [u64; N] {
        if self.timed {
            self.settle()
        } else {
            [0; N]
        }
    }

    /// Turns the day to DATE when it is a later one - the month too, when the month is another - and places what
    /// waits in the day that now stands; what it placed. A day that turns one day forward leaves what it came to as
    /// yesterday; a turn over more days leaves none, for the board did not see the day before the new one. DATE is the date of a time that is believed ([`Watch`]), so a
    /// day recorded further ahead than a day (a zone, summer time) is a day from a time that was false: it is put right,
    /// today starts from nought - what lies under a day that never came is no day's - and the month and the total stay,
    /// for the month is a short count and not a false one.
    pub fn turn_and_settle(&mut self, date: Date) -> [u64; N] {
        self.timed = true;
        match self.day {
            Some(day) if date > day => {
                // what the day came to is yesterday only when the day that turned is the one right before the new one
                self.yesterday = (date.days() - day.days() == 1).then_some(Yesterday { date: day, sums: self.sums.today });
                self.sums.today = [0; N];
                if (date.year, date.month) != (day.year, day.month) {
                    self.sums.month = [0; N];
                }
                self.day = Some(date);
            }
            Some(day) if day.days() - date.days() > FUTURE_DAYS => {
                self.sums.today = [0; N];
                self.day = Some(date);
            }
            Some(_) => {}
            None => self.day = Some(date),
        }
        self.settle()
    }

    /// The count at NOW: what came before it in the day that was, the turn of the day, what came while the time was
    /// not known in the day that stands. None - the board does not know the time.
    pub fn at(&mut self, now: Option<Now>) {
        let Some(now) = now else {
            self.untimed();
            return;
        };
        self.settle_if_timed();
        self.turn_and_settle(now.date);
    }

    /// Zeroes today, this month, in all, what waits and yesterday: that was a day of the count that is no more. The day stays: a reset is not a new day.
    pub fn reset(&mut self) {
        self.sums = Sums::default();
        self.pending = [0; N];
        self.yesterday = None;
    }

    fn settle(&mut self) -> [u64; N] {
        let placed = std::mem::replace(&mut self.pending, [0; N]);
        for n in 0..N {
            self.sums.today[n] += placed[n];
            self.sums.month[n] += placed[n];
        }
        placed
    }
}

/// A line of a snapshot: the NAME, then VALUES separated by spaces.
pub fn numbers(name: &str, values: &[u64]) -> String {
    let words: Vec<String> = values.iter().map(u64::to_string).collect();
    format!("{name} {}\n", words.join(" "))
}

/// The line "day": the date, or "-" before the board knew the time.
pub fn day_line(day: Option<Date>) -> String {
    format!("day {}\n", day.map_or("-".into(), |d| d.words()))
}

/// A snapshot read line by line, every line in its place and nothing else.
pub struct Lines<'a> {
    lines: std::str::Lines<'a>,
}

impl<'a> Lines<'a> {
    /// The reader of TEXT, or why it is not a snapshot of the kind HEADER names: another format is refused rather
    /// than misread.
    pub fn new(text: &'a str, header: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        if lines.next() != Some(header) {
            return Err(format!("not a snapshot: the first line is not '{header}'"));
        }
        Ok(Lines { lines })
    }

    /// The words after NAME on the next line.
    pub fn field(&mut self, name: &str) -> Result<Vec<&'a str>, String> {
        let line = self.lines.next().ok_or(format!("no line {name}"))?;
        let rest = line.strip_prefix(name).and_then(|r| r.strip_prefix(' ')).ok_or(format!("'{line}' where {name} stands"))?;
        Ok(rest.split(' ').collect())
    }

    /// The line "day".
    pub fn day(&mut self) -> Result<Option<Date>, String> {
        match self.field("day")?.as_slice() {
            ["-"] => Ok(None),
            [d] => Ok(Some(Date::parse(d)?)),
            other => Err(format!("day: {}", other.join(" "))),
        }
    }

    /// The line NAME with N whole numbers.
    pub fn numbers<const N: usize>(&mut self, name: &str) -> Result<[u64; N], String> {
        let words = self.field(name)?;
        if words.len() != N {
            return Err(format!("{name}: {}", words.join(" ")));
        }
        let mut out = [0; N];
        for (slot, word) in out.iter_mut().zip(&words) {
            *slot = word.parse().map_err(|_| format!("{name}: '{word}'"))?;
        }
        Ok(out)
    }

    /// Nothing is left after the last line.
    pub fn end(mut self) -> Result<(), String> {
        match self.lines.next() {
            Some(extra) => Err(format!("'{extra}' after the last line")),
            None => Ok(()),
        }
    }
}

/// A snapshot of a count and its place in the order snapshots were taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub generation: u64,
    pub text: String,
}

/// Keeps the snapshots of one count in the order they were taken: a snapshot taken before another never lands after
/// it. The count is snapshotted under its own lock and written after the lock is let go, so a reset that snapshots
/// and writes in between would otherwise be written over by the older snapshot, and come back after a restart. The
/// numbers are in Mutex, not atomics: the ESP32 has no 64-bit atomics.
#[derive(Debug, Default)]
pub struct Keeper {
    taken: std::sync::Mutex<u64>,
    /// The number of the snapshot last written; writing holds it, so writes go one at a time, in order.
    written: std::sync::Mutex<u64>,
}

impl Keeper {
    pub const fn new() -> Self {
        Keeper { taken: std::sync::Mutex::new(0), written: std::sync::Mutex::new(0) }
    }

    /// TEXT numbered after every snapshot taken before it.
    pub fn take(&self, text: String) -> Snapshot {
        let mut taken = self.taken.lock().unwrap();
        *taken += 1;
        Snapshot { generation: *taken, text }
    }

    /// Writes SNAPSHOT with PUT unless one taken after it is written already: Ok(false) then. A write that fails
    /// leaves the last written as it was.
    pub fn write<E>(&self, snapshot: Snapshot, put: impl FnOnce(&str) -> Result<(), E>) -> Result<bool, E> {
        let mut written = self.written.lock().unwrap();
        if snapshot.generation <= *written {
            return Ok(false);
        }
        put(&snapshot.text)?;
        *written = snapshot.generation;
        Ok(true)
    }

    /// As [`Keeper::write`], but None at once when another write holds the keeper: the caller that must not wait - the owner of the flash, whose desk answers the very write that holds it - tries again at its next pass.
    pub fn try_write<E>(&self, snapshot: Snapshot, put: impl FnOnce(&str) -> Result<(), E>) -> Option<Result<bool, E>> {
        let mut written = self.written.try_lock().ok()?;
        if snapshot.generation <= *written {
            return Some(Ok(false));
        }
        if let Err(why) = put(&snapshot.text) {
            return Some(Err(why));
        }
        *written = snapshot.generation;
        Some(Ok(true))
    }
}
