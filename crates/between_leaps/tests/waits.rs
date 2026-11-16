//! Every wait of the board when the chip's counter leaps. A wait is a timer of embassy-time, and the board's
//! timers must not take a leap of the counter as time gone by: not the network's, not a sensor's, not a desk's. The rows
//! ask for what a task sees - a timer that runs out, a clock that goes back, a waiter woken - and move the chip's
//! counter by the helpers of `chip`, the only place that knows how it is moved.

use core::future::Future;
use core::pin::{pin, Pin};
use core::task::{Context, Poll, Waker};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::Wake;
use std::time::Duration;

use between::Desk;
use embassy_time::{Instant, Timer};

mod common;
use common as chip;

struct Counted(AtomicU32);

impl Wake for Counted {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

/// Polls up to a few times: a timer of embassy-time answers "not yet" at its first look, whatever its span.
fn settle<T>(fut: &mut Pin<&mut impl Future<Output = T>>, cx: &mut Context<'_>) -> Poll<T> {
    for _ in 0..3 {
        if let Poll::Ready(out) = fut.as_mut().poll(cx) {
            return Poll::Ready(out);
        }
    }
    Poll::Pending
}

#[test]
fn a_wait_beside_the_desk_is_not_run_out_by_a_leap_forward() {
    let _alone = chip::start();
    let waker = Waker::from(Arc::new(Counted(AtomicU32::new(0))));
    let mut cx = Context::from_waker(&waker);
    let mut waiting = pin!(Timer::after(embassy_time::Duration::from_secs(30)));
    assert!(settle(&mut waiting, &mut cx).is_pending());
    chip::pass(Duration::from_millis(100));
    chip::leap_forward(Duration::from_secs(3 * 3600));
    assert!(settle(&mut waiting, &mut cx).is_pending(), "a wait of 30 s ran out when the chip's counter leapt three hours");
}

#[test]
fn a_wait_beside_the_desk_runs_out_when_its_time_has_gone_by() {
    let _alone = chip::start();
    let waker = Waker::from(Arc::new(Counted(AtomicU32::new(0))));
    let mut cx = Context::from_waker(&waker);
    let mut waiting = pin!(Timer::after(embassy_time::Duration::from_secs(30)));
    assert!(settle(&mut waiting, &mut cx).is_pending());
    chip::pass(Duration::from_secs(29));
    assert!(settle(&mut waiting, &mut cx).is_pending(), "a wait of 30 s ran out after 29");
    chip::pass(Duration::from_secs(2));
    assert!(settle(&mut waiting, &mut cx).is_ready(), "a wait of 30 s did not run out after 31");
}

#[test]
fn the_clock_of_embassy_time_does_not_go_back_with_the_chips_counter() {
    let _alone = chip::start();
    chip::pass(Duration::from_millis(100));
    let began = Instant::now();
    chip::leap_back(Duration::from_millis(2600));
    let span = std::panic::catch_unwind(|| Instant::now().duration_since(began));
    assert!(span.is_ok(), "a span taken across a leap back of the chip's counter panics");
}

#[test]
fn the_clock_of_embassy_time_goes_with_time_that_goes_by() {
    let _alone = chip::start();
    let began = Instant::now();
    chip::pass(Duration::from_millis(250));
    let span = Instant::now().duration_since(began);
    assert_eq!(span.as_millis(), 250);
}

/// A task of its own, with its own waker, holding a desk's owner waiting for its round: what each of the board's tasks is.
struct Task {
    woken: Arc<Counted>,
    waker: Waker,
    owner: Pin<Box<dyn Future<Output = ()>>>,
}

fn owners_at_their_own_desks(many: usize) -> Vec<Task> {
    (0..many)
        .map(|_| {
            let desk: &'static Desk<u32, u32, 4, 2> = Box::leak(Box::new(Desk::new()));
            let woken = Arc::new(Counted(AtomicU32::new(0)));
            Task { waker: Waker::from(woken.clone()), woken, owner: Box::pin(desk.served_for(Duration::from_secs(1), |what| what)) }
        })
        .collect()
}

fn poll_all(tasks: &mut [Task]) {
    for task in tasks {
        let mut cx = Context::from_waker(&task.waker);
        assert!(task.owner.as_mut().poll(&mut cx).is_pending());
    }
}

fn woken(tasks: &[Task]) -> u32 {
    tasks.iter().map(|task| task.woken.0.swap(0, Ordering::Relaxed)).sum()
}

#[test]
fn forty_waiters_without_a_leap_do_not_wake_each_other() {
    let _alone = chip::start();
    let mut tasks = owners_at_their_own_desks(40);
    poll_all(&mut tasks);
    woken(&tasks);
    // polled until nothing wakes them: a leap that never came must not keep them going
    for round in 0..20 {
        poll_all(&mut tasks);
        assert_eq!(woken(&tasks), 0, "40 waiters woke each other at round {round} with no leap: a livelock");
    }
}

#[test]
fn forty_waiters_are_all_woken_when_their_second_has_gone_by() {
    let _alone = chip::start();
    let mut tasks = owners_at_their_own_desks(40);
    poll_all(&mut tasks);
    woken(&tasks);
    chip::pass(Duration::from_millis(1100));
    let silent: Vec<usize> = (0..tasks.len()).filter(|&i| tasks[i].woken.0.load(Ordering::Relaxed) == 0).collect();
    assert!(silent.is_empty(), "waiters {silent:?} of 40 were not woken when their second went by");
}

#[test]
fn waiters_past_the_places_are_counted_and_the_queue_holds_the_rest() {
    let _alone = chip::start();
    let before = chip::waiters();
    let mut tasks = owners_at_their_own_desks(between::steady_time::WAITERS + 8);
    // each is polled once: the 8 past the places are woken at once, as many times as they find no place
    for task in &mut tasks {
        let mut cx = Context::from_waker(&task.waker);
        let _ = task.owner.as_mut().poll(&mut cx);
    }
    let after = chip::waiters();
    assert!(after.crowded_out >= before.crowded_out + 8, "waiters past the places were not counted: {after:?}");
    assert_eq!(after.high, between::steady_time::WAITERS, "the queue did not fill its places");
}

#[test]
fn a_wait_ends_within_a_slice_of_its_time_after_a_leap_back() {
    let _alone = chip::start();
    let waker = Waker::from(Arc::new(Counted(AtomicU32::new(0))));
    let mut cx = Context::from_waker(&waker);
    let mut waiting = pin!(Timer::after(embassy_time::Duration::from_secs(1)));
    assert!(settle(&mut waiting, &mut cx).is_pending());
    chip::pass(Duration::from_millis(100));
    chip::leap_back(Duration::from_millis(2600));
    chip::pass(Duration::from_millis(800));
    assert!(settle(&mut waiting, &mut cx).is_pending(), "a wait of 1 s ran out 0.9 s after it began");
    chip::pass(Duration::from_millis(400));
    assert!(settle(&mut waiting, &mut cx).is_ready(), "a wait of 1 s was held by a leap back: 1.3 s after it began it was still waiting");
}

#[test]
fn a_leap_forward_adds_no_more_than_a_slice_to_the_clock() {
    let _alone = chip::start();
    chip::pass(Duration::from_millis(100));
    let began = Instant::now();
    chip::leap_forward(Duration::from_secs(3 * 3600));
    let span = Instant::now().duration_since(began);
    assert!(span <= embassy_time::Duration::from_millis(100), "a leap of three hours moved the clock of embassy-time by {span:?}");
}

#[test]
fn a_leap_forward_refuses_none_of_the_askers_with_limits_of_two_and_three_seconds() {
    let _alone = chip::start();
    static FAN: Desk<u32, u32, 4, 2> = Desk::new();
    static AUTOMATON: Desk<u32, u32, 4, 2> = Desk::new();
    static OTHER: Desk<u32, u32, 4, 2> = Desk::new();
    let waker = Waker::from(Arc::new(Counted(AtomicU32::new(0))));
    let mut cx = Context::from_waker(&waker);
    let mut fan = pin!(FAN.asked(1, Duration::from_secs(2)));
    let mut automaton = pin!(AUTOMATON.asked(1, Duration::from_secs(3)));
    let mut other = pin!(OTHER.asked(1, Duration::from_secs(2)));
    assert!(settle(&mut fan, &mut cx).is_pending() && settle(&mut automaton, &mut cx).is_pending() && settle(&mut other, &mut cx).is_pending());
    chip::pass(Duration::from_millis(100));
    chip::leap_forward(Duration::from_secs(3 * 3600));
    assert!(settle(&mut fan, &mut cx).is_pending(), "the fan's asker (2 s) was refused by a leap forward");
    assert!(settle(&mut automaton, &mut cx).is_pending(), "the automaton's asker (3 s) was refused by a leap forward");
    assert!(settle(&mut other, &mut cx).is_pending(), "an asker of 2 s was refused by a leap forward");
    // from there the limits run as they were given: 2 s are out two seconds after the asking
    chip::pass(Duration::from_millis(2000));
    assert!(settle(&mut fan, &mut cx).is_ready(), "the fan's asker did not run out after its 2 s");
    assert!(settle(&mut automaton, &mut cx).is_pending(), "the automaton's asker (3 s) ran out after 2.1 s");
    chip::pass(Duration::from_millis(1000));
    assert!(settle(&mut automaton, &mut cx).is_ready(), "the automaton's asker did not run out after its 3 s");
}

#[test]
fn a_timer_is_woken_when_the_chips_alarm_could_not_be_set() {
    let _alone = chip::start();
    let counted = Arc::new(Counted(AtomicU32::new(0)));
    let waker = Waker::from(counted.clone());
    let mut cx = Context::from_waker(&waker);
    chip::lose_alarms(true);
    let mut waiting = pin!(Timer::after(embassy_time::Duration::from_secs(1)));
    assert!(settle(&mut waiting, &mut cx).is_pending());
    counted.0.store(0, Ordering::Relaxed);
    chip::pass(Duration::from_millis(1300));
    // woken, not polled: a task asleep is looked at by nobody, and a timer polled by hand is ready whatever woke it
    assert!(counted.0.load(Ordering::Relaxed) > 0, "a wait of 1 s was left asleep by an alarm that could not be set");
}
