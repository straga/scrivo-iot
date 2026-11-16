//! Electricity counted from the power the meter reads. No hardware and no clock here: the board hands in each asking
//! of the meter with its monotonic time, and the local time when it knows it; this crate says what was used when, so
//! all of it is tested on the computer.
//!
//! Energy is power integrated over time, as Tasmota does it (xnrg_14_bl09xx.ino), not the meter's CF counters: those
//! start from zero at every power-up and wrap at 2^24. Counts are millijoules (watt-milliseconds) in u64 - no
//! remainder of a step is thrown away, and the count ends after 5 * 10^9 kWh.

/// Outputs of the box counted: L1 and L2. The sum is theirs.
pub const OUTPUTS: usize = 2;
/// The longest step one asking counts: an asking later than this after the one before counts this much, so a stalled
/// thread does not turn one reading into minutes of use.
pub const MAX_STEP_MS: u64 = 2000;

pub use tally::{Date, Keeper, Now, Snapshot, Sums, Watch};
use tally::{Lines, Tally};

/// Where a row of the CSV starts: a date, the minute of the day it starts on, and the offset of that moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub date: Date,
    pub minute: u16,
    pub offset_min: i16,
}

/// A closed stretch of the CSV: what each output used in it, and how long each read negative power.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub slot: Slot,
    pub mj: [u64; OUTPUTS],
    pub negative_ms: [u64; OUTPUTS],
}

/// The count of electricity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Energy {
    /// Today, this month and in all, per output, and what waits for the time (crates/tally).
    pub tally: Tally<OUTPUTS>,
    /// The stretch of the CSV being filled, and what it holds so far.
    pub slot: Option<Slot>,
    pub in_slot: [u64; OUTPUTS],
    /// How long each output read negative power in this stretch.
    pub negative_ms: [u64; OUTPUTS],
    /// When the meter was last asked, on the board's monotonic clock; not kept across a restart.
    pub last_asked_ms: Option<u64>,
}

impl Energy {
    /// One asking of the meter at AT_MS: WATTS when it answered, None when not. An answer counts its power over the
    /// time since the asking before - at most MAX_STEP_MS; a missed asking counts nothing, and neither does the
    /// first. Negative power counts nothing and is timed.
    pub fn poll(&mut self, at_ms: u64, watts: Option<[f32; OUTPUTS]>) {
        let step = self.last_asked_ms.map(|last| at_ms.saturating_sub(last).min(MAX_STEP_MS));
        self.last_asked_ms = Some(at_ms);
        let (Some(step), Some(watts)) = (step, watts) else { return };
        let mut added = [0; OUTPUTS];
        for (n, w) in watts.into_iter().enumerate() {
            if w < 0.0 {
                self.negative_ms[n] += step;
            } else {
                let mj = (f64::from(w) * step as f64).round() as u64;
                added[n] = mj;
            }
        }
        self.tally.add(added);
    }

    /// Places what was counted in its day and stretch at NOW, None while the board does not know the time or does not
    /// believe it (tally::Watch); a stretch of the CSV is EVERY_MIN minutes. The stretch that closed, if one did. Days,
    /// months and stretches turn only forwards: a clock that jumps back by a day or less closes nothing, and what comes
    /// then goes in the day that was; a day or a stretch recorded further ahead than that is put right.
    pub fn at(&mut self, now: Option<Now>, every_min: u16) -> Option<Row> {
        let Some(now) = now else {
            self.tally.untimed();
            return None;
        };
        let before = self.tally.settle_if_timed();
        self.place(before);
        let every = every_min.max(1);
        let start = Slot { date: now.date, minute: now.minute - now.minute % every, offset_min: now.offset_min };
        let closed = match self.slot {
            Some(slot) if (start.date, start.minute) > (slot.date, slot.minute) => {
                let row = Row { slot, mj: self.in_slot, negative_ms: self.negative_ms };
                self.in_slot = [0; OUTPUTS];
                self.negative_ms = [0; OUTPUTS];
                self.slot = Some(start);
                Some(row)
            }
            // a stretch recorded from a time that was false lies further ahead than a day, as a recorded day does (tally): its
            // row would be of a day that never came, so none is written and the open stretch is the real one
            Some(slot) if slot.date.days() - start.date.days() > tally::FUTURE_DAYS => {
                self.in_slot = [0; OUTPUTS];
                self.negative_ms = [0; OUTPUTS];
                self.slot = Some(start);
                None
            }
            Some(_) => None,
            None => {
                self.slot = Some(start);
                None
            }
        };
        let after = self.tally.turn_and_settle(now.date);
        self.place(after);
        closed
    }

    /// What the count placed in a day goes in the open stretch of the CSV too.
    fn place(&mut self, placed: [u64; OUTPUTS]) {
        for n in 0..OUTPUTS {
            self.in_slot[n] += placed[n];
        }
    }

    /// Zeroes today, this month and in all, and what the open stretch and the time not yet known hold. The day and
    /// the stretch stay: a reset is not a new day.
    pub fn reset(&mut self) {
        self.tally.reset();
        self.in_slot = [0; OUTPUTS];
        self.negative_ms = [0; OUTPUTS];
    }

    /// Whether the count is sure of its day (crates/tally): `today` and `month` are numbers the board stands behind only then.
    pub fn day_sure(&self) -> bool {
        self.tally.sure_of_day()
    }

    /// The count as text, for the file that outlives a power cut: a line per field, the name first.
    pub fn snapshot(&self) -> String {
        let slot = self.slot.map_or("-".into(), |s| format!("{} {} {}", s.date.words(), s.minute, s.offset_min));
        let t = &self.tally;
        format!(
            "{SNAPSHOT}\n{}slot {slot}\n{}{}{}{}{}{}",
            tally::day_line(t.day),
            tally::numbers("today", &t.sums.today),
            tally::numbers("month", &t.sums.month),
            tally::numbers("total", &t.sums.total),
            tally::numbers("in_slot", &self.in_slot),
            tally::numbers("negative", &self.negative_ms),
            tally::numbers("pending", &t.pending),
        )
    }

    /// The count from its snapshot, or why the text is not one: every line in its place, and nothing else.
    pub fn restore(text: &str) -> Result<Energy, String> {
        let mut lines = Lines::new(text, SNAPSHOT)?;
        let day = lines.day()?;
        let slot = match lines.field("slot")?.as_slice() {
            ["-"] => None,
            [d, minute, offset] => {
                let minute: u16 = minute.parse().map_err(|_| format!("slot minute '{minute}'"))?;
                let offset_min: i16 = offset.parse().map_err(|_| format!("slot offset '{offset}'"))?;
                if minute >= 24 * 60 {
                    return Err(format!("slot minute {minute}"));
                }
                // no zone of the world is further than 14 hours from UTC
                if offset_min.unsigned_abs() > 14 * 60 {
                    return Err(format!("slot offset {offset_min}"));
                }
                Some(Slot { date: Date::parse(d)?, minute, offset_min })
            }
            other => return Err(format!("slot: {}", other.join(" "))),
        };
        let sums = Sums { today: lines.numbers("today")?, month: lines.numbers("month")?, total: lines.numbers("total")? };
        let (in_slot, negative_ms, pending) = (lines.numbers("in_slot")?, lines.numbers("negative")?, lines.numbers("pending")?);
        lines.end()?;
        Ok(Energy { tally: Tally { sums, day, pending, timed: false, ..Tally::default() }, slot, in_slot, negative_ms, last_asked_ms: None })
    }

    /// The sums of the three rows of [`Energy::words`], in kilowatt-hours, row for row: what a page draws by beside
    /// the words.
    pub fn sums_kwh(&self) -> [(&'static str, f32); 3] {
        // millijoules to kilowatt-hours, the unit the words are in
        let sum = |v: [u64; OUTPUTS]| (v[0] + v[1]) as f64 / 3.6e9;
        let sums = &self.tally.sums;
        [("today", sum(sums.today) as f32), ("month", sum(sums.month) as f32), ("total", sum(sums.total) as f32)]
    }

    /// The same three rows as [`Energy::sums_kwh`], each output's own kilowatt-hours, output for output: what a client says of each output apart.
    pub fn sums_by_output_kwh(&self) -> [(&'static str, [f32; OUTPUTS]); 3] {
        let apart = |v: [u64; OUTPUTS]| v.map(|mj| (mj as f64 / 3.6e9) as f32);
        let sums = &self.tally.sums;
        [("today", apart(sums.today)), ("month", apart(sums.month)), ("total", apart(sums.total))]
    }

    /// The three rows as `words` says them, for where the numbers are told as true - the console's `meter`: a dash for today and
    /// the month while the count is not sure of its day (`day_sure`), for they are the kept numbers of a day the board has not seen;
    /// the total as it is. `words` keeps saying what the count holds, as the log of a start does.
    pub fn words_shown(&self) -> [(&'static str, String); 3] {
        let mut rows = self.words();
        if !self.day_sure() {
            rows[0].1 = "-".into();
            rows[1].1 = "-".into();
        }
        rows
    }

    /// The three rows a person reads - today, month, total - each "L1 0.012 · L2 0.110 · sum 0.122 kWh".
    pub fn words(&self) -> [(&'static str, String); 3] {
        let row = |v: [u64; OUTPUTS]| format!("L1 {} · L2 {} · sum {} kWh", kwh(v[0]), kwh(v[1]), kwh(v[0] + v[1]));
        [("today", row(self.tally.sums.today)), ("month", row(self.tally.sums.month)), ("total", row(self.tally.sums.total))]
    }
}

/// The first line of a snapshot: the format, so that another one is refused rather than misread.
const SNAPSHOT: &str = "energy 1";

/// Millijoules as kilowatt-hours with three decimals - watt-hours, rounded down.
fn kwh(mj: u64) -> String {
    let wh = mj / 3_600_000;
    format!("{}.{:03}", wh / 1000, wh % 1000)
}

/// Millijoules as watt-hours with three decimals - milliwatt-hours, rounded down.
fn wh(mj: u64) -> String {
    let mwh = mj / 3600;
    format!("{}.{:03}", mwh / 1000, mwh % 1000)
}

/// "17:00+03:00" for a minute of the day and an offset.
fn clock(minute: u16, offset_min: i16) -> String {
    let sign = if offset_min < 0 { '-' } else { '+' };
    let off = offset_min.unsigned_abs();
    format!("{:02}:{:02}{sign}{:02}:{:02}", minute / 60, minute % 60, off / 60, off % 60)
}

impl Row {
    /// The row as the CSV has it: "2026-09-19 17:00+03:00,19.412,110.700,130.112" - the start, then Wh of L1, L2 and
    /// the sum.
    pub fn csv(&self) -> String {
        let s = &self.slot;
        format!(
            "{} {},{},{},{}",
            s.date.words(),
            clock(s.minute, s.offset_min),
            wh(self.mj[0]),
            wh(self.mj[1]),
            wh(self.mj[0] + self.mj[1])
        )
    }
}

/// The file of the CSV a stretch goes in: "2026-09.csv".
pub fn file_of(date: Date) -> String {
    format!("{:04}-{:02}.csv", date.year, date.month)
}

/// The first line of every CSV file.
pub const CSV_HEADER: &str = "start,l1_wh,l2_wh,sum_wh";

/// The row that marks a reset at NOW: the counts before it are gone, and the gap in the series shows why.
pub fn reset_row(now: Now) -> String {
    format!("{} {},reset,,", now.date.words(), clock(now.minute, now.offset_min))
}

/// How many closed rows of the CSV wait for a flash that refuses: two days of rows at a row every 15 minutes. Past it the oldest go, and the log of the board says so.
pub const BACKLOG_MAX: usize = 192;

/// Rows of the CSV the flash has not taken yet, in the order they closed. **A ROW IS WRITTEN WHEN THE FLASH TOOK IT**: a row closed in memory is gone from the count (the stretch is open again), so a refused row waits here and goes before the
/// next one - the series has no hole where the flash said no.
#[derive(Debug, Default)]
pub struct Backlog {
    rows: std::collections::VecDeque<(String, String)>,
}

impl Backlog {
    pub const fn new() -> Self {
        Backlog { rows: std::collections::VecDeque::new() }
    }

    /// Adds LINE to the CSV FILE behind what waits, then hands the rows to PUT in their order, each with its file; a row PUT refuses stays with all after it. Ok - nothing waits; Err - how many do, and the
    /// first refusal. Rows over [`BACKLOG_MAX`] let the oldest go; the count of those goes in the Err too.
    pub fn write<E>(&mut self, file: &str, line: &str, mut put: impl FnMut(&str, &str) -> Result<(), E>) -> Result<(), (usize, usize, E)> {
        self.rows.push_back((file.to_string(), line.to_string()));
        let mut dropped = 0;
        while self.rows.len() > BACKLOG_MAX {
            self.rows.pop_front();
            dropped += 1;
        }
        while let Some((file, line)) = self.rows.front() {
            if let Err(why) = put(file, line) {
                return Err((self.rows.len(), dropped, why));
            }
            self.rows.pop_front();
        }
        Ok(())
    }
}
