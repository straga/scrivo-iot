//! An owner's desk when the chip's counter leaps: its limits are timers of embassy-time like any other, and the
//! time driver of the board keeps the leap out of them. The leap adds `MOST` to the board's clock at the most, so the
//! limits here are longer than that.

use core::future::Future;
use core::pin::{pin, Pin};
use core::task::{Context, Poll, Waker};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::Wake;
use std::time::Duration;

use between::{Desk, Refused};

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

fn waker() -> Waker {
    Waker::from(Arc::new(Counted(AtomicU32::new(0))))
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

const LEAP: Duration = Duration::from_secs(3 * 3600);

#[test]
fn a_leap_forward_does_not_refuse_an_asker_inside_its_limit() {
    let _alone = chip::start();
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let waker = waker();
    let mut cx = Context::from_waker(&waker);
    let mut asking = pin!(DESK.asked(1, Duration::from_secs(10)));
    assert!(settle(&mut asking, &mut cx).is_pending());
    chip::pass(Duration::from_millis(100));
    chip::leap_forward(LEAP);
    assert!(settle(&mut asking, &mut cx).is_pending(), "an asker with 9.9 s of its 10 left was refused when the chip leapt three hours");
    // the owner takes it up and answers: the asker has it
    let mut serving = pin!(DESK.served_for(Duration::from_secs(1), |what| what * 10));
    assert!(serving.as_mut().poll(&mut cx).is_pending());
    assert_eq!(settle(&mut asking, &mut cx), Poll::Ready(Ok(10)));
}

#[test]
fn an_asker_whose_limit_has_gone_by_is_refused_as_silent() {
    let _alone = chip::start();
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let waker = waker();
    let mut cx = Context::from_waker(&waker);
    let mut asking = pin!(DESK.asked(1, Duration::from_secs(10)));
    assert!(settle(&mut asking, &mut cx).is_pending());
    chip::pass(Duration::from_secs(9));
    assert!(settle(&mut asking, &mut cx).is_pending(), "refused before its limit");
    chip::pass(Duration::from_secs(2));
    assert_eq!(settle(&mut asking, &mut cx), Poll::Ready(Err(Refused::Silent)), "not refused when its limit had gone by");
}

#[test]
fn the_owners_wait_between_its_rounds_is_not_cut_short_by_a_leap_forward() {
    let _alone = chip::start();
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let waker = waker();
    let mut cx = Context::from_waker(&waker);
    let mut serving = pin!(DESK.served_for(Duration::from_secs(10), |what| what));
    assert!(settle(&mut serving, &mut cx).is_pending());
    chip::pass(Duration::from_millis(100));
    chip::leap_forward(LEAP);
    assert!(settle(&mut serving, &mut cx).is_pending(), "the owner went to its next round after 100 ms of its 10 s");
    // the leap added a slice (100 ms) to the board's clock: the 10 s are out 9.8 s after it
    chip::pass(Duration::from_millis(9700));
    assert!(settle(&mut serving, &mut cx).is_pending(), "the owner came back before its time had gone by");
    chip::pass(Duration::from_millis(300));
    assert_eq!(settle(&mut serving, &mut cx), Poll::Ready(()), "the owner did not come back when its time had gone by");
}

#[test]
fn an_owner_whose_round_has_no_time_left_still_takes_up_what_is_waiting() {
    let _alone = chip::start();
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let waker = waker();
    let mut cx = Context::from_waker(&waker);
    let mut asking = pin!(DESK.asked(1, Duration::from_secs(10)));
    assert!(settle(&mut asking, &mut cx).is_pending());
    // the round overran: nothing is left of the time between rounds, and the request has waited in the queue
    let mut serving = pin!(DESK.served_for(Duration::ZERO, |what| what * 10));
    let _ = serving.as_mut().poll(&mut cx);
    assert_eq!(settle(&mut asking, &mut cx), Poll::Ready(Ok(10)), "the request waiting was not taken up by an owner out of time");
    assert_eq!(settle(&mut serving, &mut cx), Poll::Ready(()), "an owner with no time left did not end its wait");
}

#[test]
fn an_answer_that_is_there_when_the_limit_runs_out_is_given() {
    let _alone = chip::start();
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let waker = waker();
    let mut cx = Context::from_waker(&waker);
    let mut asking = pin!(DESK.asked(1, Duration::from_secs(5)));
    assert!(settle(&mut asking, &mut cx).is_pending());
    let mut serving = pin!(DESK.served_for(Duration::from_secs(10), |what| what * 10));
    assert!(serving.as_mut().poll(&mut cx).is_pending());
    chip::pass(Duration::from_secs(5)); // the limit is out, and the answer has been waiting for the asker to look
    assert_eq!(settle(&mut asking, &mut cx), Poll::Ready(Ok(10)), "an answer already there was refused as silent");
}

#[test]
fn a_leap_back_leaves_an_asker_inside_its_limit_waiting_and_answered() {
    let _alone = chip::start();
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let waker = waker();
    let mut cx = Context::from_waker(&waker);
    let mut asking = pin!(DESK.asked(1, Duration::from_secs(10)));
    assert!(settle(&mut asking, &mut cx).is_pending());
    chip::leap_back(Duration::from_millis(2600));
    assert!(settle(&mut asking, &mut cx).is_pending(), "an asker inside its limit was refused by a leap back");
    let mut serving = pin!(DESK.served_for(Duration::from_secs(1), |what| what * 10));
    assert!(serving.as_mut().poll(&mut cx).is_pending());
    assert_eq!(settle(&mut asking, &mut cx), Poll::Ready(Ok(10)));
}

#[test]
fn a_leap_back_holds_a_limit_for_a_slice_and_no_longer() {
    let _alone = chip::start();
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let waker = waker();
    let mut cx = Context::from_waker(&waker);
    let mut asking = pin!(DESK.asked(1, Duration::from_secs(1)));
    assert!(settle(&mut asking, &mut cx).is_pending());
    // the chip's alarm is set 100 ms ahead and the counter goes back 2.6 s: the alarm would ring that much later, and the
    // beat, which counts by the tick, looks at the clock every slice instead
    chip::leap_back(Duration::from_millis(2600));
    chip::pass(Duration::from_millis(900));
    assert!(settle(&mut asking, &mut cx).is_pending(), "a limit of 1 s was told as gone by 0.9 s after the asking");
    chip::pass(Duration::from_millis(400));
    assert_eq!(settle(&mut asking, &mut cx), Poll::Ready(Err(Refused::Silent)), "the limit was held by the leap back: 1.3 s after the asking it still waited");
}
