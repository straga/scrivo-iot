//! What the shared unit costs each round that reads through it: how long the round held the lock of the unit, how long it
//! waited for it, and how long its step was. Counted in whole words and no heap, by the round that reads - one writer to a
//! lane - and read by `status` and the note a silence leaves.
//!
//! THE NUMBERS ARE TAKEN OUTSIDE THE LOCK. [`through`] reads the clock twice after the lock is taken - once on getting it,
//! once before giving it up - and writes the numbers down only when the lock is gone, so that the measure does not
//! lengthen what it measures.
//!
//! No 64-bit word: the chip has no 64-bit atomic. A span is a `u32` of microseconds (71 minutes the most), the mean is a
//! running one, and the minute is six slots of ten seconds.

use core::sync::atomic::{AtomicU32, Ordering};

/// The length of one slot of the minute, in milliseconds.
const SLOT_MS: u64 = 10_000;
/// How many slots make the last minute.
const SLOTS: usize = 6;
/// Over how many measures the mean is a true mean; past it, each new one moves it by this part: a mean over days that
/// still moves when the board changes.
const MEAN_OVER: u32 = 65_536;

/// One quantity of a lane: the most since start, the most within the last minute, the mean since start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spread {
    pub max_us: u32,
    pub minute_max_us: u32,
    /// None before the first measure.
    pub mean_us: Option<u32>,
}

/// What a lane says of itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub held: Spread,
    pub waited: Spread,
    pub step: Spread,
}

struct Slot {
    /// Which ten seconds of the board's clock the slot holds.
    stamp: AtomicU32,
    max: AtomicU32,
}

/// One quantity as the round writes it.
struct Meter {
    max: AtomicU32,
    mean: AtomicU32,
    count: AtomicU32,
    slots: [Slot; SLOTS],
}

impl Meter {
    const fn new() -> Meter {
        Meter { max: AtomicU32::new(0), mean: AtomicU32::new(0), count: AtomicU32::new(0), slots: [const { Slot { stamp: AtomicU32::new(0), max: AtomicU32::new(0) } }; SLOTS] }
    }

    fn record(&self, us: u32, now_ms: u64) {
        if us > self.max.load(Ordering::Relaxed) {
            self.max.store(us, Ordering::Relaxed);
        }
        let count = self.count.load(Ordering::Relaxed).saturating_add(1).min(MEAN_OVER);
        let mean = i64::from(self.mean.load(Ordering::Relaxed));
        let moved = mean + (i64::from(us) - mean) / i64::from(count);
        self.mean.store(moved as u32, Ordering::Relaxed);
        self.count.store(count, Ordering::Relaxed);
        let stamp = (now_ms / SLOT_MS) as u32;
        let slot = &self.slots[stamp as usize % SLOTS];
        if slot.stamp.load(Ordering::Relaxed) == stamp {
            if us > slot.max.load(Ordering::Relaxed) {
                slot.max.store(us, Ordering::Relaxed);
            }
        } else {
            slot.max.store(us, Ordering::Relaxed);
            slot.stamp.store(stamp, Ordering::Relaxed);
        }
    }

    fn spread(&self, now_ms: u64) -> Spread {
        let now = (now_ms / SLOT_MS) as u32;
        let minute_max_us = self
            .slots
            .iter()
            .filter(|slot| now.wrapping_sub(slot.stamp.load(Ordering::Relaxed)) < SLOTS as u32)
            .map(|slot| slot.max.load(Ordering::Relaxed))
            .max()
            .unwrap_or(0);
        let mean_us = (self.count.load(Ordering::Relaxed) > 0).then(|| self.mean.load(Ordering::Relaxed));
        Spread { max_us: self.max.load(Ordering::Relaxed), minute_max_us, mean_us }
    }
}

/// The measures of one round that reads through the unit.
pub struct Lane {
    held: Meter,
    waited: Meter,
    step: Meter,
}

impl Lane {
    pub const fn new() -> Lane {
        Lane { held: Meter::new(), waited: Meter::new(), step: Meter::new() }
    }

    /// The round waited US microseconds for the lock, at NOW_MS of the board's clock.
    pub fn waited(&self, us: u32, now_ms: u64) {
        self.waited.record(us, now_ms);
    }

    /// The round held the lock US microseconds.
    pub fn held(&self, us: u32, now_ms: u64) {
        self.held.record(us, now_ms);
    }

    /// The round took US microseconds from its start to the start of the next.
    pub fn step(&self, us: u32, now_ms: u64) {
        self.step.record(us, now_ms);
    }

    pub fn report(&self, now_ms: u64) -> Report {
        Report { held: self.held.spread(now_ms), waited: self.waited.spread(now_ms), step: self.step.spread(now_ms) }
    }
}

impl Default for Lane {
    fn default() -> Self {
        Self::new()
    }
}

fn us(span: u64) -> u32 {
    span.min(u64::from(u32::MAX)) as u32
}

/// Takes the lock with LOCK, does WORK under it and gives the lock up; the wait and the hold go to LANE after the lock is given up. CLOCK_US reads the board's clock in microseconds.
pub fn through<G, T>(lane: &Lane, clock_us: impl Fn() -> u64, lock: impl FnOnce() -> G, work: impl FnOnce() -> T) -> T {
    let asked = clock_us();
    let guard = lock();
    let got = clock_us();
    let out = work();
    let done = clock_us();
    drop(guard);
    let now_ms = done / 1000;
    lane.waited(us(got.saturating_sub(asked)), now_ms);
    lane.held(us(done.saturating_sub(got)), now_ms);
    out
}

/// The words of one lane for `status` and the journal: numbers with their units, no prose.
pub fn line(name: &str, report: &Report) -> String {
    let mean = |s: &Spread| s.mean_us.map_or(String::from("none"), |m| format!("{m} us"));
    let held = &report.held;
    let (waited, step) = (&report.waited, &report.step);
    format!(
        "{name}: held max {} us mean {}, minute max {} us; waited max {} us, minute max {} us; step max {} us, minute max {} us",
        held.max_us,
        mean(held),
        held.minute_max_us,
        waited.max_us,
        waited.minute_max_us,
        step.max_us,
        step.minute_max_us
    )
}
