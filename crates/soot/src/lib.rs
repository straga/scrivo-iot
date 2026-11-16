//! How dirty the boiler is, counted without the hardware: the board hands in, every second, whether the automaton is in
//! its working stage and what the flue and the water read; this crate says what each burn came to, what the last burns
//! say together, and how many grams were burned since the boiler was cleaned - so all of it is tested on the computer.
//!
//! The measure is the flue minus the water in the working stage (measured: it hardly follows the street, 0.06 °C a
//! degree, and little the water, 0.25 °C a degree). A burn is one unbroken run of working seconds. Its number is the median
//! of that difference over one window the same for every burn - the fifth to the fifteenth minute of the working stage -,
//! and only over the seconds on the big rung (measured: the whole burn followed its length and its rung, 4.4 and 3.5 °C over
//! the record; the window does not). A burn that had fewer than ten minutes on the big rung in that window says nothing: the
//! warm-up, the tail and every second on a lower rung do not belong to it. The number of the boiler is the median of the last six burns.
//!
//! The median is taken from a histogram of half-degree bins, so a burn of any length takes the same few hundred bytes and
//! nothing grows with it.
//!
//! The grams since the cleaning follow the total of the pellets (crates/pellets) and not a count of their own: the total is
//! where the grams are counted from the auger, and a reset of the pellets must not take grams from this count.

pub mod past;
mod saving;

use tally::Lines;

pub use tally::{Date, Keeper, Snapshot};

pub use saving::{Pending, Saving, RETRY_MS, SAVE_EVERY_MS};

/// Seconds of a burn that do not count: the fire is still warming the boiler. The window opens after them.
pub const SKIP_S: u32 = 300;
/// The window closes after this second of the burn: it is the fifth to the fifteenth minute.
pub const WINDOW_END_S: u32 = 900;
/// Seconds on the big rung the window must hold for the burn to say anything: all ten minutes of it.
pub const MIN_BIG_S: u32 = 600;
/// Burns kept, the newest last.
pub const KEPT: usize = 8;
/// Burns that make the number of the boiler.
pub const COUNTED: usize = 6;

/// A bin of the histogram is half a degree; the first stands at `LOW_C`.
const BIN_C: f32 = 0.5;
const LOW_C: f32 = -20.0;
const BINS: usize = 256;

const SNAPSHOT: &str = "soot 2";
/// The kind the image before the window wrote: the same lines, but its burns were counted over the whole burn and do not mix with the new ones.
const SNAPSHOT_BEFORE_THE_WINDOW: &str = "soot 1";

/// The count of the soot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Soot {
    /// The flue minus the water of the burn that runs now, one count per bin.
    hist: [u32; BINS],
    /// How many readings the histogram holds.
    samples: u32,
    /// Seconds of the window the burn that runs now has spent on the big rung, whatever the sensors said.
    big_s: u32,
    /// Seconds of the working stage the burn that runs now has lasted; 0 - none runs. Not kept across a restart.
    run_s: u32,
    /// The medians of the burns that ended, as bins, the oldest first; the first `kept` are there.
    burns: [u8; KEPT],
    kept: u8,
    /// The day the boiler was last cleaned, when the board knew it.
    cleaned_on: Option<Date>,
    /// Grams of pellets burned since the cleaning.
    grams: u64,
    /// Where the total of the pellets stood at the last look: the grams since are what it grew by.
    seen_total_g: Option<u64>,
    /// How many times what is kept changed: the board writes the count when this is not what it wrote last.
    pub changes: u64,
    /// How many burns were counted and cleanings made since the start: what moves it cannot be had again, and the board writes it at once. Not kept across a restart.
    pub urgent: u64,
}

impl Default for Soot {
    fn default() -> Soot {
        Soot { hist: [0; BINS], samples: 0, big_s: 0, run_s: 0, burns: [0; KEPT], kept: 0, cleaned_on: None, grams: 0, seen_total_g: None, changes: 0, urgent: 0 }
    }
}

fn bin_of(diff_c: f32) -> usize {
    ((diff_c - LOW_C) / BIN_C).round().clamp(0.0, (BINS - 1) as f32) as usize
}

fn value_of(bin: u8) -> f32 {
    f32::from(bin) * BIN_C + LOW_C
}

impl Soot {
    /// One second: whether the automaton is in its working stage, whether the thermostat holds the big rung, and what the flue and the water read (None - the sensor said nothing).
    /// Only the seconds of the window (after `SKIP_S`, up to `WINDOW_END_S`) on the big rung are taken; the first second that is not working ends the burn that ran.
    pub fn second(&mut self, working: bool, on_big: bool, flue_c: Option<f32>, boiler_c: Option<f32>) {
        if working {
            self.run_s = self.run_s.saturating_add(1);
            if on_big && self.run_s > SKIP_S && self.run_s <= WINDOW_END_S {
                self.big_s += 1;
                if let (Some(flue), Some(boiler)) = (flue_c, boiler_c) {
                    let bin = &mut self.hist[bin_of(flue - boiler)];
                    *bin = bin.saturating_add(1);
                    self.samples += 1;
                }
            }
        } else if self.run_s > 0 {
            self.end_burn();
        }
    }

    fn end_burn(&mut self) {
        if self.big_s >= MIN_BIG_S && self.samples > 0 {
            let median = self.median_bin();
            if usize::from(self.kept) == KEPT {
                self.burns.copy_within(1.., 0);
                self.burns[KEPT - 1] = median;
            } else {
                self.burns[usize::from(self.kept)] = median;
                self.kept += 1;
            }
            self.changes += 1;
            self.urgent += 1;
        }
        self.hist = [0; BINS];
        self.samples = 0;
        self.big_s = 0;
        self.run_s = 0;
    }

    /// The bin that holds the middle reading of the burn that ended; for an even count of readings, the lower of the two in the middle: a bin is half a degree, and the pair is not told apart finer.
    fn median_bin(&self) -> u8 {
        let target = (self.samples - 1) / 2;
        let mut seen = 0u32;
        for (bin, count) in self.hist.iter().enumerate() {
            seen += count;
            if seen > target {
                return bin as u8;
            }
        }
        (BINS - 1) as u8
    }

    /// The median of the last `COUNTED` burns, in degrees; None before the first burn ended.
    pub fn soot_c(&self) -> Option<f32> {
        let n = self.burns_counted();
        if n == 0 {
            return None;
        }
        let kept = usize::from(self.kept);
        let mut last: Vec<f32> = self.burns[kept - n..kept].iter().map(|&b| value_of(b)).collect();
        last.sort_by(|a, b| a.total_cmp(b));
        Some(if n % 2 == 1 { last[n / 2] } else { (last[n / 2 - 1] + last[n / 2]) / 2.0 })
    }

    /// How many burns the number is made of: up to `COUNTED`.
    pub fn burns_counted(&self) -> usize {
        usize::from(self.kept).min(COUNTED)
    }

    /// How many burns are kept: up to `KEPT`.
    pub fn burns_kept(&self) -> usize {
        usize::from(self.kept)
    }

    /// A look at the total of the pellets burned, in grams. The first look only says where the total stands; later ones add what it grew by. A total that fell was zeroed
    /// by a reset of the pellets: nothing is taken from the count, and the next look starts from where the total is.
    pub fn observe_total_g(&mut self, total_g: u64) {
        if let Some(seen) = self.seen_total_g {
            if total_g > seen {
                self.grams += total_g - seen;
                self.changes += 1;
            }
        }
        self.seen_total_g = Some(total_g);
    }

    /// Grams of pellets burned since the cleaning.
    pub fn grams_since_clean(&self) -> u64 {
        self.grams
    }

    /// The boiler was cleaned ON that day (None - the board does not know the date): the grams begin from nought and the burns - the ones that ended and the one that runs - are forgotten, because the number says what THIS boiler does and the burns before the cleaning were
    /// of a dirtier one: left in, they would keep the number dirty - and the card asking for a cleaning - for the four burns it takes to push them out of the last six.
    pub fn cleaned(&mut self, on: Option<Date>) {
        self.grams = 0;
        self.cleaned_on = on;
        self.kept = 0;
        self.burns = [0; KEPT];
        // the burn that runs is forgotten too, and counted anew from now: what it gathered is of the dirty boiler, and the five minutes that do not count and the window after them begin at the cleaning
        self.hist = [0; BINS];
        self.samples = 0;
        self.big_s = 0;
        self.run_s = 0;
        self.changes += 1;
        self.urgent += 1;
    }

    /// The boiler was cleaned ON that day, when the total of the pellets stood at TOTAL_G, and the count was not there for it: the grams since begin from what the total has grown by since, the day is marked, the burns are kept.
    /// Taken only while the count knows no cleaning: a total given by a person is of the epoch it was read in, and the total now may be of another (after `reset pellets`), so a second one could only be told apart from a mistake by luck.
    pub fn cleaned_before(&mut self, on: Date, today: Option<Date>, total_g: u64) -> Result<(), past::Refused> {
        if self.cleaned_on.is_some() {
            return Err(past::Refused::AlreadyKnown);
        }
        let today = today.ok_or(past::Refused::TimeUnknown)?;
        if on > today {
            return Err(past::Refused::FutureDay);
        }
        let seen = self.seen_total_g.ok_or(past::Refused::TotalNotSeen)?;
        if total_g > seen {
            return Err(past::Refused::TotalAboveNow);
        }
        self.cleaned_on = Some(on);
        self.grams = seen - total_g;
        self.changes += 1;
        self.urgent += 1;
        Ok(())
    }

    /// The day of the last cleaning, when it was known.
    pub fn cleaned_on(&self) -> Option<Date> {
        self.cleaned_on
    }

    /// The count as text, for the file that outlives a power cut: a line per field, the name first. The burn that runs is not in it.
    pub fn snapshot(&self) -> String {
        let mut burns = vec![u64::from(self.kept)];
        burns.extend(self.burns.iter().map(|&b| u64::from(b)));
        format!(
            "{SNAPSHOT}\n{}{}{}",
            tally::day_line(self.cleaned_on),
            tally::numbers("grams", &[self.grams, self.seen_total_g.unwrap_or(0), u64::from(self.seen_total_g.is_some())]),
            tally::numbers("burns", &burns),
        )
    }

    /// The count from its snapshot, or why the text is not one: every line in its place, and nothing else. A snapshot of the image before the window gives the grams, the day and the total of the pellets, and not its burns: they were counted over the whole burn.
    pub fn restore(text: &str) -> Result<Soot, String> {
        let before_the_window = text.lines().next() == Some(SNAPSHOT_BEFORE_THE_WINDOW);
        let mut lines = Lines::new(text, if before_the_window { SNAPSHOT_BEFORE_THE_WINDOW } else { SNAPSHOT })?;
        let cleaned_on = lines.day()?;
        let [grams, seen, has_seen] = lines.numbers::<3>("grams")?;
        let burns = lines.numbers::<{ KEPT + 1 }>("burns")?;
        lines.end()?;
        if burns[0] > KEPT as u64 || has_seen > 1 || burns[1..].iter().any(|&b| b >= BINS as u64) {
            return Err("burns: out of range".to_string());
        }
        let mut kept = [0u8; KEPT];
        for (slot, &b) in kept.iter_mut().zip(&burns[1..]) {
            *slot = if before_the_window { 0 } else { b as u8 };
        }
        Ok(Soot { burns: kept, kept: if before_the_window { 0 } else { burns[0] as u8 }, cleaned_on, grams, seen_total_g: (has_seen == 1).then_some(seen), ..Soot::default() })
    }
}
