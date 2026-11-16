//! The time driver of `embassy-time` on the board's steady clock: `now()` is the steady clock's, and the timers are
//! woken by two things. The chip's alarm (esp_timer) rings at the next timer, no further ahead than a slice, and is as precise
//! as the chip's counter - which leaps. The beat rings every slice by the TICK of FreeRTOS, which a leap of the counter does
//! not move (the watcher of the tick resyncs it, systick.c): it looks at the clock again, wakes what is due and sets the alarm
//! anew if it was lost or is held. So a leap of the chip's counter moves the board's clock by one slice at the most and no
//! timer by more, forward or back, and an alarm that could not be set costs a slice, not a sleep without end.
//!
//! The driver is [`Wheel`], over two functions: the steady clock's reading and the setting of one alarm of the chip, and
//! the beat is started by whoever owns the start of the board ([`Wheel::start_beat`]), who can stop the board when it fails:
//! a timer cannot say that. The board gives the wheel esp_timer and a task asleep by ticks; a row on the computer gives it a
//! chip moved by hand.

use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use core::task::Waker;

use embassy_sync::blocking_mutex::Mutex;

use crate::Lock;

/// The furthest ahead the chip's alarm is set and the beat's period, in microseconds: the most a leap of the chip's
/// counter moves the board's clock by (`clock::PAD`).
pub const SLICE_US: u64 = crate::clock::PAD.as_micros() as u64;

/// How many tasks that wait by `embassy-time` the driver holds at once: 12 on the first device at the peak, 24 with a nested wait
/// each, twice that for tasks to come. The queue has a place per WAKER, not per timer: all the waits
/// of one task are one place. The high mark in `status` says whether this is enough.
pub const WAITERS: usize = 48;

/// How many wakers past the places wait for the next beat to be woken: a waker with no place is woken once a slice, not at
/// every poll.
pub const PARKED: usize = 8;

/// What the driver says of itself, for `status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Waiters {
    /// Places taken now.
    pub held: usize,
    /// The most places taken at once since start.
    pub high: usize,
    /// How many times a waker found no place: it waits for the next beat or, with that full too, is woken at once.
    pub crowded_out: u32,
    /// How many times the chip's alarm could not be set: the beat looks again within a slice.
    pub arm_failed: u32,
    /// Whether the beat runs.
    pub beating: bool,
    /// How many timers were set before the beat ran: a timer that stood first is held by every leap of the chip's counter
    /// back until the beat comes, so the start of the board says it must be none.
    pub early: u32,
}

/// The timers waiting: N places, each a waker and the moment it is woken at, in microseconds of the steady clock, and
/// the PARKED wakers that found none.
pub struct Alarms<const N: usize> {
    places: [Option<(u64, Waker)>; N],
    parked: [Option<Waker>; PARKED],
    high: usize,
    crowded_out: u32,
}

impl<const N: usize> Alarms<N> {
    pub const fn new() -> Self {
        Alarms { places: [const { None }; N], parked: [const { None }; PARKED], high: 0, crowded_out: 0 }
    }

    /// Puts WAKER in to be woken at AT. A waker already there keeps the earlier of the two moments, because a task that
    /// waits on several timers is woken by the first and sets the rest again. With every place taken, the waker with the
    /// latest moment gives its place up and is parked: it is woken by the next beat, looks again and waits as it did -
    /// once a slice however often it is polled, so that a crowd of waiters costs a poll a slice each, not a loop.
    pub fn schedule(&mut self, at: u64, waker: &Waker) {
        if let Some((held_at, _)) = self.places.iter_mut().flatten().find(|(_, held)| held.will_wake(waker)) {
            *held_at = (*held_at).min(at);
            return;
        }
        if let Some(free) = self.places.iter_mut().find(|place| place.is_none()) {
            *free = Some((at, waker.clone()));
            self.high = self.high.max(self.held());
            return;
        }
        self.crowded_out += 1;
        let latest = self.places.iter().enumerate().filter_map(|(i, place)| place.as_ref().map(|(held_at, _)| (i, *held_at))).max_by_key(|(_, held_at)| *held_at);
        match latest {
            Some((i, held_at)) if held_at > at => {
                if let Some((_, crowded)) = self.places[i].replace((at, waker.clone())) {
                    self.park(&crowded);
                }
            }
            _ => self.park(waker),
        }
    }

    /// WAKER waits for the next beat; with the room for that taken too, it is woken at once.
    fn park(&mut self, waker: &Waker) {
        if self.parked.iter().flatten().any(|parked| parked.will_wake(waker)) {
            return;
        }
        match self.parked.iter_mut().find(|place| place.is_none()) {
            Some(free) => *free = Some(waker.clone()),
            None => waker.wake_by_ref(),
        }
    }

    /// The wakers that found no place, for the beat to wake.
    pub fn take_parked(&mut self) -> [Option<Waker>; PARKED] {
        core::mem::replace(&mut self.parked, [const { None }; PARKED])
    }

    /// Wakes and drops every waker due at NOW; the moment of the next one, or `u64::MAX` when none waits.
    pub fn expire(&mut self, now: u64) -> u64 {
        let mut next = u64::MAX;
        for place in &mut self.places {
            match place {
                Some((at, _)) if *at <= now => {
                    if let Some((_, due)) = place.take() {
                        due.wake();
                    }
                }
                Some((at, _)) => next = next.min(*at),
                None => {}
            }
        }
        next
    }

    pub fn held(&self) -> usize {
        self.places.iter().flatten().count()
    }

    pub fn waiters(&self) -> Waiters {
        Waiters { held: self.held(), high: self.high, crowded_out: self.crowded_out, arm_failed: 0, beating: false, early: 0 }
    }
}

impl<const N: usize> Default for Alarms<N> {
    fn default() -> Self {
        Self::new()
    }
}

struct Inner<const N: usize> {
    alarms: Alarms<N>,
    /// The moment of the steady clock the chip's alarm is expected at, 0 when none is set.
    set_for: u64,
    arm_failed: u32,
}

/// The driver: N places for waiting tasks over STEADY (microseconds of the steady clock), ARM (sets the chip's one alarm to
/// ring after the given microseconds, in place of the one set before; it calls [`Wheel::rang`] when it does, and says
/// whether it could be set). The beat, which calls [`Wheel::beat`] every slice, is started by [`Wheel::start_beat`].
pub struct Wheel<const N: usize> {
    inner: Mutex<Lock, RefCell<Inner<N>>>,
    steady: fn() -> u64,
    arm: fn(u64) -> bool,
    beating: AtomicBool,
    early: AtomicU32,
}

impl<const N: usize> Wheel<N> {
    pub const fn new(steady: fn() -> u64, arm: fn(u64) -> bool) -> Self {
        Wheel { inner: Mutex::new(RefCell::new(Inner { alarms: Alarms::new(), set_for: 0, arm_failed: 0 })), steady, arm, beating: AtomicBool::new(false), early: AtomicU32::new(0) }
    }

    /// Starts the beat with START, once: what START says when it fails is given back, and the wheel is not beating then.
    /// Until it runs the wheel is woken by the chip's alarm alone, which a leap of the chip's counter holds. The start is the
    /// caller's and not a timer's, so that a failure stops the board where its start stands instead of being swallowed.
    pub fn start_beat<E>(&self, start: impl FnOnce() -> Result<(), E>) -> Result<(), E> {
        if self.beating.load(Ordering::Acquire) {
            return Ok(());
        }
        let started = start();
        self.beating.store(started.is_ok(), Ordering::Release);
        started
    }

    /// The moment now, in microseconds of the steady clock.
    pub fn now(&self) -> u64 {
        (self.steady)()
    }

    /// WAKER is woken at AT, a moment of the steady clock.
    pub fn schedule_wake(&self, at: u64, waker: &Waker) {
        if !self.beating.load(Ordering::Acquire) {
            self.early.fetch_add(1, Ordering::Relaxed);
        }
        self.inner.lock(|inner| inner.borrow_mut().alarms.schedule(at, waker));
        self.set();
    }

    /// The chip's alarm rang.
    pub fn rang(&self) {
        self.inner.lock(|inner| inner.borrow_mut().set_for = 0);
        self.set();
    }

    /// A slice has gone by on the tick: the wakers that found no place are woken, what is due is woken, and the alarm is
    /// set again if it is not set - lost, or held by a leap back of the chip's counter.
    pub fn beat(&self) {
        let parked = self.inner.lock(|inner| inner.borrow_mut().alarms.take_parked());
        for waker in parked.into_iter().flatten() {
            waker.wake();
        }
        self.set();
    }

    pub fn waiters(&self) -> Waiters {
        self.inner.lock(|inner| {
            let inner = inner.borrow();
            Waiters { arm_failed: inner.arm_failed, beating: self.beating.load(Ordering::Acquire), early: self.early.load(Ordering::Relaxed), ..inner.alarms.waiters() }
        })
    }

    /// Wakes what is due, and sets the chip's alarm for the next timer or a slice ahead, whichever is nearer - unless one
    /// is set for then or sooner. An alarm set for a moment the clock is already past has not rung: a leap back of the chip's
    /// counter holds it, and the beat sets it again. An alarm that could not be set is not counted as set: the next look
    /// sets it again, and the refusal is counted.
    fn set(&self) {
        let now = self.now();
        let after = self.inner.lock(|inner| {
            let mut inner = inner.borrow_mut();
            let next = inner.alarms.expire(now);
            if next == u64::MAX {
                inner.set_for = 0;
                return None;
            }
            let after = (next - now).min(SLICE_US);
            if inner.set_for > now && inner.set_for <= now + after {
                return None;
            }
            Some(after)
        });
        let Some(after) = after else { return };
        let set = (self.arm)(after);
        self.inner.lock(|inner| {
            let mut inner = inner.borrow_mut();
            if set {
                inner.set_for = now + after;
            } else {
                inner.set_for = 0;
                inner.arm_failed += 1;
            }
        });
    }
}

impl<const N: usize> embassy_time_driver::Driver for Wheel<N> {
    fn now(&self) -> u64 {
        Wheel::now(self)
    }

    fn schedule_wake(&self, at: u64, waker: &Waker) {
        Wheel::schedule_wake(self, at, waker)
    }
}

/// The chip's alarm on the board: esp_timer, in the one task of it; the beat: a task of ours asleep by ticks.
#[cfg(target_os = "espidf")]
mod board {
    use esp_idf_svc::timer::{EspTimer, EspTimerService, Task};

    use super::{Wheel, SLICE_US, WAITERS};

    static TIMER: std::sync::Mutex<Option<EspTimer<'static>>> = std::sync::Mutex::new(None);

    /// The stack of the beat's task: it reads the clock, wakes wakers and sets the alarm.
    const BEAT_STACK: usize = 3072;

    /// The steady clock without telling a leap: this runs in the task of esp_timer, whose stack is small.
    fn steady() -> u64 {
        crate::clock::now_quiet().as_micros()
    }

    /// Sets the alarm to ring after AFTER_US; false when esp_timer could not make it or start it.
    fn arm(after_us: u64) -> bool {
        let mut timer = TIMER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if timer.is_none() {
            *timer = EspTimerService::<Task>::new().and_then(|service| service.timer(|| DRIVER.rang())).ok();
        }
        match timer.as_ref() {
            Some(timer) => timer.after(core::time::Duration::from_micros(after_us)).is_ok(),
            None => false,
        }
    }

    embassy_time_driver::time_driver_impl!(static DRIVER: Wheel<WAITERS> = Wheel::new(steady, arm));

    /// Starts the beat: a task asleep for a slice by the tick of FreeRTOS (`crate::nap`; `std::thread::sleep` ends by the chip's clock, and a leap of it
    /// back lengthens the sleep by the leap), which a leap of the chip's counter does not move. The firmware calls it at its start and stops the board when it does not stand
    /// (`firmware::run`): a board whose timers wait on the chip's counter alone is held by every leap.
    pub fn start_beat() -> Result<(), esp_idf_svc::sys::EspError> {
        DRIVER.start_beat(|| {
            round::round("slice", BEAT_STACK, round::Core::Net, || loop {
                crate::nap(core::time::Duration::from_micros(SLICE_US));
                DRIVER.beat();
            })
        })
    }

    /// What the queue of the board's timers holds, for `status`.
    pub fn waiters() -> super::Waiters {
        DRIVER.waiters()
    }
}

#[cfg(target_os = "espidf")]
pub use board::{start_beat, waiters};
