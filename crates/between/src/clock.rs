//! The board's steady clock: time that never goes back and never leaps. On a device under load the chip's
//! clock went back 2.6 s and forward about three hours at the switching of its power loads (26.09); `embassy-time`
//! panics on a clock that went back, and every span taken across a leap forward reads as hours gone by. `embassy-time`
//! reads this clock now (`steady_time`), so no wait of the board sees the leap.

use core::time::Duration;

/// A moment of a steady clock: microseconds of it since it started.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Moment(u64);

impl Moment {
    /// The span from EARLIER to this moment; zero when EARLIER is not before it.
    pub fn since(self, earlier: Moment) -> Duration {
        Duration::from_micros(self.0.saturating_sub(earlier.0))
    }

    pub fn as_micros(self) -> u64 {
        self.0
    }

    pub fn as_millis(self) -> u64 {
        self.0 / 1000
    }
}

impl core::ops::Add<Duration> for Moment {
    type Output = Moment;
    fn add(self, span: Duration) -> Moment {
        Moment(self.0 + span.as_micros() as u64)
    }
}

/// The leaps a steady clock has taken out of its source: how many, and the largest back and forward.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Jumps {
    pub count: u32,
    pub largest_back_ms: u64,
    pub largest_forward_ms: u64,
}

/// A leap of the source the steady clock took: back or forward, and by how much, in milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leap {
    pub back: bool,
    pub ms: u64,
}

/// A steady clock over SOURCE - microseconds of a clock that may leap. Every reading adds the span since the last one,
/// but no more than MOST: a source gone back adds nothing, one gone forward past MOST adds PAD; either is a jump.
pub struct Steady {
    source: fn() -> u64,
    most: Duration,
    pad: Duration,
    state: std::sync::Mutex<State>,
    on_leap: std::sync::OnceLock<fn(Leap)>,
}

#[derive(Default)]
struct State {
    last: Option<u64>,
    // the source's reading at the first reading of the clock: the steady clock counts from it, the chip from its start
    first: u64,
    now: u64,
    jumps: Jumps,
    // the last leaps forward: the moment of the clock after each and what it added, in microseconds (`padded_between`)
    pads: [(u64, u64); PADS],
    pads_taken: usize,
    // the last leap that no reader has told yet: a reading that must not say a leap (`now_quiet`) leaves it for the next one
    untold: Option<Leap>,
}

/// How many leaps forward `padded_between` remembers: a silence is judged within a minute, and four leaps in a minute
/// is a storm that the count in `status` shows anyway.
const PADS: usize = 4;

impl Steady {
    /// A clock that takes a leap forward as MOST of time gone by: what a reader that looks at it once in MOST can say.
    pub const fn new(source: fn() -> u64, most: Duration) -> Self {
        Self::with_pad(source, most, most)
    }

    /// A clock that takes a leap forward as PAD of time gone by, a span past MOST being a leap: for a clock that is read
    /// at least once in PAD whatever else happens, so that a leap adds no more than the time that went by unseen.
    pub const fn with_pad(source: fn() -> u64, most: Duration, pad: Duration) -> Self {
        Steady { source, most, pad, state: std::sync::Mutex::new(State { last: None, first: 0, now: 0, jumps: Jumps { count: 0, largest_back_ms: 0, largest_forward_ms: 0 }, pads: [(0, 0); PADS], pads_taken: 0, untold: None }), on_leap: std::sync::OnceLock::new() }
    }

    /// What to do at each leap of the source; set once, the first call counts.
    pub fn set_on_leap(&self, said: fn(Leap)) {
        let _ = self.on_leap.set(said);
    }

    /// The moment now.
    pub fn now(&self) -> Moment {
        let (moment, leap) = self.read(true);
        // said after the lock is let go: the line takes its stamp from this clock
        if let Some(leap) = leap {
            if let Some(said) = self.on_leap.get() {
                said(leap);
            }
        }
        moment
    }

    /// The moment now, with a leap not said: for a reader whose stack is too small for a line of the log, the task of
    /// esp_timer that the time driver runs in. The leap is said by the next `now`.
    pub fn now_quiet(&self) -> Moment {
        self.read(false).0
    }

    /// The moment now, and the leap to say with it when SAY.
    fn read(&self, say: bool) -> (Moment, Option<Leap>) {
        // the chip's clock is read under the lock: read before it, a reader that takes the lock second brings a moment
        // older than the one already there, and the clock counts a leap back that never was (33 in 2.5 min on `.30`)
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let raw = (self.source)();
        let Some(last) = state.last.replace(raw) else {
            state.first = raw;
            return (Moment(state.now), None);
        };
        let most = self.most.as_micros() as u64;
        let pad_us = self.pad.as_micros() as u64;
        let mut leap = None;
        let mut pad = None;
        let step = match raw.checked_sub(last) {
            Some(step) if step <= most => step,
            Some(span) => {
                state.jumps.count += 1;
                state.jumps.largest_forward_ms = state.jumps.largest_forward_ms.max(span / 1000);
                leap = Some(Leap { back: false, ms: span / 1000 });
                pad = Some(pad_us);
                pad_us
            }
            None => {
                state.jumps.count += 1;
                state.jumps.largest_back_ms = state.jumps.largest_back_ms.max((last - raw) / 1000);
                leap = Some(Leap { back: true, ms: (last - raw) / 1000 });
                0
            }
        };
        state.now += step;
        if leap.is_some() {
            state.untold = leap;
        }
        let leap = if say { state.untold.take() } else { None };
        if let Some(by) = pad {
            let at = state.now;
            let slot = state.pads_taken % PADS;
            state.pads[slot] = (at, by);
            state.pads_taken += 1;
        }
        (Moment(state.now), leap)
    }

    /// The time since the chip started, in milliseconds: its clock at the first reading of this one and the steady time
    /// since, so that a leap of the chip's clock is not in it.
    pub fn uptime_ms(&self) -> u64 {
        let now = self.now();
        let first = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).first;
        (first + now.as_micros()) / 1000
    }

    /// How much of the span from FROM to TO is the leaps of the source and not time gone by: each leap forward added
    /// PAD to the clock whatever it really was, so a silence measured across one is that much too long.
    pub fn padded_between(&self, from: Moment, to: Moment) -> Duration {
        let state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let taken = state.pads_taken.min(PADS);
        let padded: u64 = state.pads[..taken].iter().filter(|(at, _)| *at > from.0 && *at <= to.0).map(|(_, by)| *by).sum();
        Duration::from_micros(padded)
    }

    /// The leaps taken so far.
    pub fn jumps(&self) -> Jumps {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).jumps
    }
}

/// The longest a reading of the board's clock may add to it, a span of more than that being a leap of the chip's clock and
/// not time gone by. The time driver reads the board's clock once a slice by the tick of FreeRTOS (`steady_time`), so a span
/// of two seconds is a stall of the board and not a leap that is yet to be told from time.
pub const MOST: Duration = Duration::from_secs(2);

/// What a leap of the chip's clock forward adds to the board's: one slice of the time driver's beat, the time that can have
/// gone by unseen between two readings of the clock. A timer, a limit and a span measured across a leap are out by this
/// at the most, not by the hours it was.
pub const PAD: Duration = Duration::from_millis(100);

/// The chip's clock in microseconds since start: esp_timer, which `std::time` reads too.
#[cfg(target_os = "espidf")]
fn chip_us() -> u64 {
    unsafe { esp_idf_svc::sys::esp_timer_get_time() as u64 }
}

/// The host's clock in microseconds since the first reading.
#[cfg(not(target_os = "espidf"))]
fn chip_us() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_micros() as u64
}

static BOARD: Steady = Steady::with_pad(chip_us, MOST, PAD);

/// What to do at each leap of the board's clock; set once.
pub fn on_leap(said: fn(Leap)) {
    BOARD.set_on_leap(said);
}

/// The moment now by the board's steady clock: what every span of time on the board is taken by.
pub fn now() -> Moment {
    BOARD.now()
}

/// The same without saying a leap of the chip's counter, for the time driver's alarm, which runs in a task of a small stack.
pub fn now_quiet() -> Moment {
    BOARD.now_quiet()
}

/// The time since the chip started by the board's clock, in milliseconds; counted from the first reading of the clock,
/// which the board makes at its start.
pub fn uptime_ms() -> u64 {
    BOARD.uptime_ms()
}

/// What of the span from FROM to TO is leaps of the chip's counter, by the board's clock (see `Steady::padded_between`).
pub fn padded_between(from: Moment, to: Moment) -> Duration {
    BOARD.padded_between(from, to)
}

/// The leaps the board's clock has taken out of the chip's, for `status`.
pub fn jumps() -> Jumps {
    BOARD.jumps()
}

impl Moment {
    /// The span from this moment to now by the board's steady clock.
    pub fn elapsed(self) -> Duration {
        now().since(self)
    }
}

/// The line of `status`: how many leaps, and the largest back and forward.
impl core::fmt::Display for Jumps {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "clock jumps: {}, largest back {} ms, forward {} ms",
            self.count, self.largest_back_ms, self.largest_forward_ms
        )
    }
}
