//! The queue and the alarm of the board's time driver, over a clock and a chip alarm that the rows hold in
//! their hands. The rows with a leap of the chip's counter are in `between_leaps`; here is what the driver does by itself.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Wake, Waker};

use between::steady_time::{Alarms, Wheel, SLICE_US};

struct Counted(AtomicU32);

impl Wake for Counted {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

fn counted() -> (Arc<Counted>, Waker) {
    let c = Arc::new(Counted(AtomicU32::new(0)));
    (c.clone(), Waker::from(c))
}

fn woken(c: &Counted) -> u32 {
    c.0.load(Ordering::Relaxed)
}

static NOW: AtomicU64 = AtomicU64::new(1);
/// The delays the driver asked the chip's alarm for, in order.
static ARMED: Mutex<Vec<u64>> = Mutex::new(Vec::new());
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn steady() -> u64 {
    NOW.load(Ordering::SeqCst)
}

/// Whether the chip's alarm can be set.
static CAN_ARM: AtomicBool = AtomicBool::new(true);

fn arm(after_us: u64) -> bool {
    if !CAN_ARM.load(Ordering::SeqCst) {
        return false;
    }
    ARMED.lock().unwrap().push(after_us);
    true
}

static WHEEL: Wheel<4> = Wheel::new(steady, arm);

fn start() -> std::sync::MutexGuard<'static, ()> {
    let guard = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    // what the rows before left in the queue runs out
    CAN_ARM.store(true, Ordering::SeqCst);
    NOW.fetch_add(10_000_000_000, Ordering::SeqCst);
    WHEEL.rang();
    ARMED.lock().unwrap().clear();
    guard
}

fn armed() -> Vec<u64> {
    ARMED.lock().unwrap().clone()
}

#[test]
fn an_alarm_far_off_is_set_no_further_ahead_than_a_slice() {
    let _alone = start();
    let (_, waker) = counted();
    WHEEL.schedule_wake(steady() + 30_000_000, &waker);
    assert_eq!(armed(), vec![SLICE_US], "a timer 30 s away was set for more than a slice");
}

#[test]
fn an_alarm_near_is_set_for_its_own_moment() {
    let _alone = start();
    let (_, waker) = counted();
    WHEEL.schedule_wake(steady() + 30_000, &waker);
    assert_eq!(armed(), vec![30_000]);
}

#[test]
fn a_timer_is_woken_when_its_moment_has_come_and_not_before() {
    let _alone = start();
    let (count, waker) = counted();
    WHEEL.schedule_wake(steady() + 250_000, &waker);
    NOW.fetch_add(249_999, Ordering::SeqCst);
    WHEEL.rang();
    assert_eq!(woken(&count), 0, "woken one microsecond early");
    NOW.fetch_add(1, Ordering::SeqCst);
    WHEEL.rang();
    assert_eq!(woken(&count), 1, "not woken at its moment");
    WHEEL.rang();
    assert_eq!(woken(&count), 1, "woken twice for one timer");
}

#[test]
fn an_alarm_already_set_for_sooner_is_not_set_again() {
    let _alone = start();
    let (_, near) = counted();
    let (_, far) = counted();
    WHEEL.schedule_wake(steady() + 20_000, &near);
    WHEEL.schedule_wake(steady() + 5_000_000, &far);
    assert_eq!(armed(), vec![20_000], "an alarm for a later timer replaced the one that was nearer");
}

#[test]
fn an_alarm_the_clock_has_passed_without_its_ringing_is_set_again_by_the_next_look() {
    let _alone = start();
    let (_, waker) = counted();
    WHEEL.schedule_wake(steady() + 5_000_000, &waker);
    // the chip's counter went back, the alarm is held: the clock goes past the moment it was set for, and it did not ring
    NOW.fetch_add(SLICE_US + 1, Ordering::SeqCst);
    let (_, other) = counted();
    WHEEL.schedule_wake(steady() + 5_000_000, &other);
    assert_eq!(armed().len(), 2, "no alarm was set again when the first was due and had not rung: {:?}", armed());
}

#[test]
fn a_task_with_two_timers_holds_one_place_and_the_earlier_moment() {
    let mut alarms: Alarms<4> = Alarms::new();
    let (count, waker) = counted();
    alarms.schedule(500, &waker);
    alarms.schedule(300, &waker);
    alarms.schedule(900, &waker);
    assert_eq!(alarms.held(), 1);
    assert_eq!(alarms.expire(299), 300);
    assert_eq!(woken(&count), 0);
    assert_eq!(alarms.expire(300), u64::MAX);
    assert_eq!(woken(&count), 1);
}

#[test]
fn with_every_place_taken_the_waker_due_latest_is_parked_and_woken_by_the_beat() {
    let mut alarms: Alarms<2> = Alarms::new();
    let (first, w1) = counted();
    let (second, w2) = counted();
    let (third, w3) = counted();
    alarms.schedule(100, &w1);
    alarms.schedule(900, &w2);
    alarms.schedule(500, &w3);
    assert_eq!(woken(&first) + woken(&second) + woken(&third), 0, "a waker was woken at once for want of a place");
    for waker in alarms.take_parked().into_iter().flatten() {
        waker.wake();
    }
    assert_eq!(woken(&second), 1, "the waker due latest kept its place");
    assert_eq!(woken(&first) + woken(&third), 0, "a waker due sooner gave its place up");
    let held = alarms.waiters();
    assert_eq!((held.held, held.high, held.crowded_out), (2, 2, 1));
}

#[test]
fn a_newcomer_due_later_than_all_is_parked_itself() {
    let mut alarms: Alarms<1> = Alarms::new();
    let (first, w1) = counted();
    let (second, w2) = counted();
    alarms.schedule(100, &w1);
    alarms.schedule(900, &w2);
    assert_eq!((woken(&first), woken(&second)), (0, 0));
    for waker in alarms.take_parked().into_iter().flatten() {
        waker.wake();
    }
    assert_eq!((woken(&first), woken(&second)), (0, 1));
    assert_eq!(alarms.waiters().crowded_out, 1);
}

#[test]
fn a_crowd_past_the_parked_is_woken_at_once_and_counted() {
    use between::steady_time::PARKED;
    let mut alarms: Alarms<1> = Alarms::new();
    let (_, w0) = counted();
    alarms.schedule(100, &w0);
    let crowd: Vec<_> = (0..PARKED + 2).map(|_| counted()).collect();
    for (_, waker) in &crowd {
        alarms.schedule(900, waker);
    }
    let woken_at_once = crowd.iter().filter(|(count, _)| woken(count) > 0).count();
    assert_eq!(woken_at_once, 2, "the wakers past the room for parking were not the ones woken at once");
    assert_eq!(alarms.waiters().crowded_out as usize, PARKED + 2);
}

#[test]
fn an_alarm_that_could_not_be_set_is_counted_and_set_again_by_the_next_look() {
    let _alone = start();
    let (_, waker) = counted();
    let before = WHEEL.waiters().arm_failed;
    CAN_ARM.store(false, Ordering::SeqCst);
    WHEEL.schedule_wake(steady() + 5_000_000, &waker);
    assert_eq!(WHEEL.waiters().arm_failed, before + 1, "a refused alarm was not counted");
    assert!(armed().is_empty());
    // the chip takes it again; a later timer, a later moment: the alarm for the first is still owed, and set
    CAN_ARM.store(true, Ordering::SeqCst);
    let (_, later) = counted();
    WHEEL.schedule_wake(steady() + 9_000_000, &later);
    assert_eq!(armed(), vec![SLICE_US], "the alarm refused before was not set when the next timer looked");
}

#[test]
fn the_beat_sets_a_refused_alarm_again_without_a_new_timer() {
    let _alone = start();
    let (_, waker) = counted();
    CAN_ARM.store(false, Ordering::SeqCst);
    WHEEL.schedule_wake(steady() + 5_000_000, &waker);
    CAN_ARM.store(true, Ordering::SeqCst);
    WHEEL.beat();
    assert_eq!(armed(), vec![SLICE_US], "the beat did not set the alarm that was refused");
}

#[test]
fn a_crowded_waker_polled_again_and_again_is_not_woken_on_every_poll() {
    let mut alarms: Alarms<2> = Alarms::new();
    let (_, w1) = counted();
    let (_, w2) = counted();
    let (crowded, w3) = counted();
    alarms.schedule(100, &w1);
    alarms.schedule(200, &w2);
    // every place is taken and its timer is the latest: each poll finds no place, and a wake per poll is a loop that spins
    for _ in 0..20 {
        alarms.schedule(900, &w3);
    }
    assert_eq!(woken(&crowded), 0, "a waker with no place was woken {} times in 20 polls", woken(&crowded));
}

#[test]
fn a_timer_does_not_start_the_beat_the_firmware_does() {
    static FRESH: Wheel<4> = Wheel::new(steady, arm);
    let _alone = start();
    let (_, waker) = counted();
    FRESH.schedule_wake(steady() + 50_000, &waker);
    assert!(!FRESH.waiters().beating, "a timer started the beat; its start is the firmware's, which can stop the board when it fails");
}

#[test]
fn a_beat_that_could_not_start_is_given_back_and_the_wheel_is_not_beating() {
    static FRESH: Wheel<4> = Wheel::new(steady, arm);
    let _alone = start();
    let failed = FRESH.start_beat(|| Err::<(), &str>("no memory for the task"));
    assert_eq!(failed, Err("no memory for the task"), "a refused start was swallowed");
    assert!(!FRESH.waiters().beating, "a wheel whose beat did not start says it beats");
    // the start is tried again when asked: a refusal does not shut it
    let mut tried = 0;
    assert_eq!(FRESH.start_beat(|| { tried += 1; Ok::<(), &str>(()) }), Ok(()));
    assert!(FRESH.waiters().beating && tried == 1);
}

#[test]
fn a_beat_that_stands_is_not_started_twice() {
    static FRESH: Wheel<4> = Wheel::new(steady, arm);
    let _alone = start();
    let mut started = 0;
    for _ in 0..3 {
        let _ = FRESH.start_beat(|| { started += 1; Ok::<(), ()>(()) });
    }
    assert_eq!(started, 1, "the beat was started {started} times");
}

#[test]
fn a_timer_set_before_the_beat_is_counted_and_one_set_after_it_is_not() {
    static FRESH: Wheel<4> = Wheel::new(steady, arm);
    let _alone = start();
    let (_, first) = counted();
    let (_, second) = counted();
    FRESH.schedule_wake(steady() + 50_000, &first);
    assert_eq!(FRESH.waiters().early, 1, "a timer set before the beat was not counted");
    FRESH.start_beat(|| Ok::<(), ()>(())).unwrap();
    FRESH.schedule_wake(steady() + 60_000, &second);
    assert_eq!(FRESH.waiters().early, 1, "a timer set after the beat was counted as early");
    assert!(FRESH.waiters().beating);
}

#[test]
fn a_timer_set_while_the_beat_failed_to_start_is_counted_as_early() {
    static FRESH: Wheel<4> = Wheel::new(steady, arm);
    let _alone = start();
    let (_, waker) = counted();
    assert!(FRESH.start_beat(|| Err::<(), &str>("no memory")).is_err());
    FRESH.schedule_wake(steady() + 50_000, &waker);
    let said = FRESH.waiters();
    assert!(!said.beating && said.early == 1, "{said:?}");
}
