//! The chip of the rows: a counter moved by hand, one alarm on it, and the time driver of the board over both, registered
//! as the driver of embassy-time for the process. The rows move the counter as the chip moves it - time goes by, it
//! leaps forward, it leaps back - and look at what the tasks of the board see.

use core::time::Duration;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use between::clock::{Steady, MOST, PAD};
use between::steady_time::{Wheel, WAITERS};

/// The chip's counter, microseconds. It starts above zero so that a leap back has room.
static RAW: AtomicU64 = AtomicU64::new(1_000_000_000);

/// Where the chip's one alarm is set, as a reading of its counter.
static ALARM: Mutex<Option<u64>> = Mutex::new(None);

/// The rows are run one at a time: the driver is one for the whole process.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Whether the chip's alarm refuses to be set: the call to esp_timer failed.
static LOSE_ALARMS: AtomicBool = AtomicBool::new(false);

fn raw() -> u64 {
    RAW.load(Ordering::SeqCst)
}

/// The tick of the board, microseconds: it goes with time and does not leap with the chip's counter.
static TICK: AtomicU64 = AtomicU64::new(0);

/// Whether the beat has been started: the firmware starts it at its start, and so does `start` of the rows.
static BEATING: AtomicBool = AtomicBool::new(false);

static STEADY: Steady = Steady::with_pad(raw, MOST, PAD);

fn steady() -> u64 {
    STEADY.now_quiet().as_micros()
}

fn arm(after_us: u64) -> bool {
    if LOSE_ALARMS.load(Ordering::SeqCst) {
        return false;
    }
    *ALARM.lock().unwrap() = Some(raw() + after_us);
    true
}

embassy_time_driver::time_driver_impl!(static DRIVER: Wheel<WAITERS> = Wheel::new(steady, arm));

pub fn start() -> std::sync::MutexGuard<'static, ()> {
    let guard = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *ALARM.lock().unwrap() = None;
    LOSE_ALARMS.store(false, Ordering::SeqCst);
    // the firmware starts the beat at its start, before any wait; the rows do the same
    DRIVER.start_beat(|| {
        BEATING.store(true, Ordering::SeqCst);
        Ok::<(), ()>(())
    })
    .unwrap();
    // what the rows before left in the queue, their futures dropped, runs out: a minute is longer than any row waits
    pass(Duration::from_secs(60));
    guard
}

/// The alarm rings if the counter has reached it.
fn ring_if_due() {
    let due = {
        let mut alarm = ALARM.lock().unwrap();
        match *alarm {
            Some(at) if at <= raw() => alarm.take().is_some(),
            _ => false,
        }
    };
    if due {
        DRIVER.rang();
    }
}

/// Time goes by on the chip: its counter and the tick go on, its alarm rings where it stands, and the beat rings at every
/// slice of the tick, once it has been started.
pub fn pass(span: Duration) {
    let slice = PAD.as_micros() as u64;
    let mut left = span.as_micros() as u64;
    while left > 0 {
        let to_alarm = ALARM.lock().unwrap().map(|at| at.saturating_sub(raw()));
        let to_beat = slice - TICK.load(Ordering::SeqCst) % slice;
        let step = left.min(to_beat).min(to_alarm.unwrap_or(u64::MAX));
        RAW.fetch_add(step, Ordering::SeqCst);
        TICK.fetch_add(step, Ordering::SeqCst);
        left -= step;
        ring_if_due();
        if TICK.load(Ordering::SeqCst) % slice == 0 && BEATING.load(Ordering::SeqCst) {
            DRIVER.beat();
        }
    }
}

/// The chip's counter leaps forward by SPAN: an alarm it passes rings at once.
pub fn leap_forward(span: Duration) {
    RAW.fetch_add(span.as_micros() as u64, Ordering::SeqCst);
    ring_if_due();
}

/// The chip's counter leaps back by SPAN: its alarm is held for as long.
pub fn leap_back(span: Duration) {
    RAW.fetch_sub(span.as_micros() as u64, Ordering::SeqCst);
}

/// What the driver's queue holds.
pub fn waiters() -> between::steady_time::Waiters {
    DRIVER.waiters()
}

/// The chip's alarm cannot be set from now on (or can again): the creation or the start of the esp_timer failed.
pub fn lose_alarms(lose: bool) {
    LOSE_ALARMS.store(lose, Ordering::SeqCst);
}
