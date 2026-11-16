//! A sensor by the modules' contract: what it reads once, and nothing of the round around it - as ESPHome's
//! `PollingComponent::update` reads and `Sensor::publish_state` does the rest in one order for every sensor.
//!
//! **THE ROUND WAS WRITTEN FOUR TIMES.** Four sensors of the first device each kept their own
//! "last value" under a mutex, their own curve, their own measured step and their own "announce when what the card
//! shows has changed" - in four spellings, one of which forgot the step for a while. The order is
//! written here once: keep the reading, record it in the curves, and say whether the card has news.
//!
//! What runs the round - the task, the period, the announcing - is the firmware's (`module::polling`); what is kept
//! between rounds is here, so it is tested without a board.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// One sensor: how to read it once, what its curves keep of a reading, what its card shows of one.
pub trait Sensor: Send + 'static {
    /// One round's reading.
    type Reading: Copy + PartialEq + Send;
    /// What the sensor draws from: rings, a pair of them, a ring per channel.
    type Curves: Send;
    /// What the card shows of a reading. Two readings that show the same are the same news, and the card is not
    /// announced for the second: a breathing sensor differs on every round in digits nobody sees.
    type Shown: PartialEq + Send;

    /// Reads the sensor once. None - there was nothing to read this round (every sample refused), which is not
    /// a reading of zero.
    fn read(&mut self) -> Option<Self::Reading>;

    /// Puts a reading into the curves.
    fn record(curves: &mut Self::Curves, reading: &Self::Reading);

    /// What the card shows of the last reading, or of none.
    fn shown(reading: Option<&Self::Reading>) -> Self::Shown;

    /// Says in the log what the card now shows, when it has changed. Nothing by default.
    fn told(&mut self, _reading: Option<&Self::Reading>) {}
}

/// What a sensor keeps between rounds: its last reading, its curves, and the step its rounds actually take.
pub struct Kept<S: Sensor> {
    last: Mutex<Option<S::Reading>>,
    /// What the card showed after the last round, as it was worked out THEN. Worked out again from the last
    /// reading it would pass through today's settings, and a setting that changes what the card shows - a
    /// threshold of a detector, a resistor of a divider - would change it with nobody told.
    /// None - nothing has been shown yet, which is what `shown(None)` says.
    shown: Mutex<Option<S::Shown>>,
    curves: Mutex<S::Curves>,
    step_ms: AtomicU32,
}

impl<S: Sensor> Kept<S> {
    /// Nothing read yet; CURVES empty; the step the rounds are asked for, until one is measured.
    pub const fn new(curves: S::Curves, step_ms: u32) -> Self {
        Self { last: Mutex::new(None), shown: Mutex::new(None), curves: Mutex::new(curves), step_ms: AtomicU32::new(step_ms) }
    }

    /// One round's READING: kept as the last, recorded in the curves when there is one, and answers whether what
    /// the card shows has changed.
    pub fn take(&self, reading: Option<S::Reading>) -> bool {
        if let Some(reading) = &reading {
            S::record(&mut self.curves.lock().unwrap_or_else(|poisoned| poisoned.into_inner()), reading);
        }
        let now = S::shown(reading.as_ref());
        *self.last.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = reading;
        let mut shown = self.shown.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let news = match shown.as_ref() {
            Some(before) => *before != now,
            None => S::shown(None) != now,
        };
        *shown = Some(now);
        news
    }

    /// The last reading; None until the first, and after a round that read nothing.
    pub fn last(&self) -> Option<S::Reading> {
        *self.last.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Looks at the curves.
    pub fn curves<T>(&self, look: impl FnOnce(&S::Curves) -> T) -> T {
        look(&self.curves.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    /// The step the rounds take, in milliseconds.
    pub fn step_ms(&self) -> u32 {
        self.step_ms.load(Ordering::Relaxed)
    }

    /// The step as measured. Zero is not a step - a curve drawn on it has every point at one moment - and is not
    /// taken.
    pub fn measured(&self, step_ms: u32) {
        if step_ms > 0 {
            self.step_ms.store(step_ms, Ordering::Relaxed);
        }
    }
}

/// How the rounds of one sensor are going: how many, how long they took, and how many samples the converter
/// refused.
///
/// A PERIOD NAMED IN THE CODE IS NOT A PERIOD THE BOARD KEEPS. Between "read every 100 ms" and "a round takes
/// 100 ms" lies everything this stage is about: the other sensor's turn at the unit, the network, the page. So
/// the rounds are counted and the time they really took is said, a line every hundred of them.
pub struct Tally {
    rounds: u32,
    since: between::clock::Moment,
}

impl Default for Tally {
    fn default() -> Self {
        Self::new()
    }
}

impl Tally {
    /// Over how many rounds the step is measured. At ten rounds a second that is ten seconds of them, which
    /// is long enough for one slow round not to move the answer.
    pub const EVERY: u32 = 100;

    pub fn new() -> Self {
        Self { rounds: 0, since: between::clock::now() }
    }

    /// Counts one round; every EVERY rounds answers the step the rounds ACTUALLY took, in milliseconds -
    /// which is what a curve has to be drawn on, because a round asked for every 100 ms comes every 109.
    ///
    /// It used to answer a line for the log as well - rounds, seconds, reads refused - written to prove that
    /// the queue at the converter kept up. It did; the line went, and the refusals are warned about per round
    /// where they happen (in the sensors' own modules, under devices/).
    pub fn round(&mut self) -> Option<u32> {
        self.rounds += 1;
        if self.rounds < Self::EVERY {
            return None;
        }
        let step = (self.since.elapsed().as_millis() as u32).div_ceil(self.rounds);
        *self = Self::new();
        Some(step)
    }
}
