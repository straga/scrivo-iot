//! Pellets counted from the seconds the auger turns. No hardware and no clock here: the board hands in each asking of
//! the auger's relay with its monotonic time, and the local time when it knows it; this crate says how much was poured
//! and in which day, so all of it is tested on the computer.
//!
//! The grams are the seconds of turning times the rate a person weighed: grams the auger pours in a minute of turning
//! without a stop, a setting. It is a count by the clock, and approximate: a dose of a few seconds weighs less than
//! its seconds times the rate. The count is kept in units of a millisecond times a gram a minute, so no
//! remainder of a step is thrown away; 60 000 units are a gram.
//!
//! Today, this month and in all are crates/tally, the same as the electricity's.

mod saving;

use tally::{Lines, Tally, Yesterday};

pub use saving::{Pending, Saving, YesterdayPending, RETRY_MS, SAVE_EVERY_MS};

pub use tally::{Date, Keeper, Now, Snapshot, Watch};

/// The longest step one asking counts: an asking later than this after the one before counts this much, so a stalled
/// thread does not turn one asking into minutes of pouring. The relays' owner asks every second while the auger turns.
pub const MAX_STEP_MS: u64 = 2000;

/// Units of the count in a gram: a millisecond times a gram a minute, 1000 ms in a second and 60 s in a minute.
pub const UNITS_PER_GRAM: u64 = 60_000;

/// The count of pellets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pellets {
    /// Today, this month and in all, in units, and what waits for the time (crates/tally).
    pub tally: Tally<1>,
    /// When the auger was last asked, on the board's monotonic clock; not kept across a restart.
    pub last_ms: Option<u64>,
    /// The auger turned at the asking before; not kept across a restart.
    pub auger_on: bool,
    /// How many times what is kept changed: the board writes the count when this is not what it wrote last.
    pub changes: u64,
}

impl Pellets {
    /// One asking of the auger's relay at AT_MS: whether it is closed now, and the rate in force, in grams a minute.
    /// What the auger did since the asking before - at most MAX_STEP_MS - is counted at this rate when it turned at
    /// that asking; the first asking counts nothing, and neither does a standing auger.
    pub fn observe(&mut self, at_ms: u64, auger_on: bool, g_per_min: u32) {
        let step = self.last_ms.map(|last| at_ms.saturating_sub(last).min(MAX_STEP_MS));
        if let (Some(step), true) = (step, self.auger_on) {
            let units = step * u64::from(g_per_min);
            if units > 0 {
                self.tally.add([units]);
                self.changes += 1;
            }
        }
        self.last_ms = Some(at_ms);
        self.auger_on = auger_on;
    }

    /// Places what was counted in its day at NOW, None while the board does not know the time.
    pub fn at(&mut self, now: Option<Now>) {
        let key = |t: &Tally<1>| (t.sums, t.day, t.pending, t.yesterday);
        let before = key(&self.tally);
        self.tally.at(now);
        if key(&self.tally) != before {
            self.changes += 1;
        }
    }

    /// Zeroes today, this month, in all and yesterday. The day stays: a reset is not a new day.
    pub fn reset(&mut self) {
        self.tally.reset();
        self.changes += 1;
    }

    /// Whole grams of today, this month and in all, rounded down, row for row.
    pub fn grams(&self) -> [(&'static str, u64); 3] {
        let s = &self.tally.sums;
        [("today", s.today[0] / UNITS_PER_GRAM), ("month", s.month[0] / UNITS_PER_GRAM), ("total", s.total[0] / UNITS_PER_GRAM)]
    }

    /// Kilograms of today, this month and in all, row for row: what the stream carries.
    pub fn kilograms(&self) -> [(&'static str, f32); 3] {
        let kg = |units: u64| (units as f64 / (UNITS_PER_GRAM * 1000) as f64) as f32;
        let s = &self.tally.sums;
        [("today", kg(s.today[0])), ("month", kg(s.month[0])), ("total", kg(s.total[0]))]
    }

    /// Whether the count is sure of its day (crates/tally): `today` and `month` are numbers the board stands behind only then.
    pub fn day_sure(&self) -> bool {
        self.tally.sure_of_day()
    }

    /// Kilograms the day before today came to, when the count is sure of its day and has the day (crates/tally); None otherwise.
    pub fn yesterday_kg(&self) -> Option<f32> {
        self.tally.yesterday_sums().map(|sums| (sums[0] as f64 / (UNITS_PER_GRAM * 1000) as f64) as f32)
    }

    /// The day before today as text for its own file, `/data/pellets/yesterday`: never in the file of the count, for an image that reads only the old
    /// format refuses a count with a line more and counts from nought.
    pub fn yesterday_text(&self) -> Option<String> {
        self.tally.yesterday.map(|day| day.text(YESTERDAY))
    }

    /// Takes the day the file of yesterday holds, or says why the text is not one; the count itself is left as it was.
    pub fn restore_yesterday(&mut self, text: &str) -> Result<(), String> {
        self.tally.yesterday = Some(Yesterday::parse(text, YESTERDAY)?);
        Ok(())
    }

    /// The count as text, for the file that outlives a power cut: a line per field, the name first.
    pub fn snapshot(&self) -> String {
        let t = &self.tally;
        format!(
            "{SNAPSHOT}\n{}{}{}{}{}",
            tally::day_line(t.day),
            tally::numbers("today", &t.sums.today),
            tally::numbers("month", &t.sums.month),
            tally::numbers("total", &t.sums.total),
            tally::numbers("pending", &t.pending),
        )
    }

    /// The count from its snapshot, or why the text is not one: every line in its place, and nothing else.
    pub fn restore(text: &str) -> Result<Pellets, String> {
        let mut lines = Lines::new(text, SNAPSHOT)?;
        let day = lines.day()?;
        let sums = tally::Sums { today: lines.numbers("today")?, month: lines.numbers("month")?, total: lines.numbers("total")? };
        let pending = lines.numbers("pending")?;
        lines.end()?;
        Ok(Pellets { tally: Tally { sums, day, pending, timed: false, ..Tally::default() }, ..Pellets::default() })
    }
}

/// The first line of a snapshot: the format, so that another one is refused rather than misread.
const SNAPSHOT: &str = "pellets 1";

/// The first line of the file of yesterday, a file of its own beside the count's.
const YESTERDAY: &str = "pellets-yesterday 1";
