//! A task that always has work ready still falls asleep once its step reaches the budget: under load the network's
//! loop is woken by its sockets faster than it runs, and a wake waiting after every poll kept it awake for seconds
//! (6.6, the burner on 26.09 - the task watchdog of core 0).

use std::future::poll_fn;
use std::task::Poll;
use std::time::{Duration, Instant};

use between::{block_on_stepped, Steps};

/// Holds the thread's CPU for FOR, as a read of the board's flash holds its core.
fn busy(for_: Duration) {
    let until = Instant::now() + for_;
    while Instant::now() < until {
        std::hint::spin_loop();
    }
}

/// A future that is ready again at once for WORK - it wakes itself after every poll - and then ends with its polls.
async fn always_ready(work: Duration) -> u32 {
    let until = Instant::now() + work;
    let mut polls = 0;
    poll_fn(|cx| {
        polls += 1;
        if Instant::now() >= until {
            return Poll::Ready(polls);
        }
        busy(Duration::from_micros(100));
        cx.waker().wake_by_ref();
        Poll::Pending
    })
    .await
}

#[test]
fn a_poll_that_waits_asleep_inside_holds_no_core_and_is_no_long_step() {
    let steps = Steps::new();
    let mut waited = false;
    block_on_stepped(
        poll_fn(|_| {
            // a lock another task holds: the task sleeps in the wait, and the core is free
            std::thread::sleep(Duration::from_millis(300));
            waited = true;
            Poll::Ready(())
        }),
        &steps,
        Duration::from_millis(20),
    );
    assert!(waited);
    assert!(steps.longest().0 < 50_000, "300 ms asleep inside a poll were counted as {} us held", steps.longest().0);
}

#[test]
fn a_poll_that_holds_the_core_is_a_step_as_long() {
    let steps = Steps::new();
    block_on_stepped(
        poll_fn(|_| {
            busy(Duration::from_millis(300));
            Poll::Ready(())
        }),
        &steps,
        Duration::from_millis(20),
    );
    assert!(steps.longest().0 >= 290_000, "300 ms of work in one poll were counted as {} us", steps.longest().0);
}

#[test]
fn work_always_ready_is_cut_into_steps_no_longer_than_the_budget() {
    let steps = Steps::new();
    let polls = block_on_stepped(always_ready(Duration::from_millis(300)), &steps, Duration::from_millis(20));
    let (longest_us, _) = steps.longest();
    println!("{polls} polls, the longest step {longest_us} us");
    assert!(polls > 100, "the work ended after {polls} polls");
    assert!(longest_us < 50_000, "a task with work always ready ran {longest_us} us without a sleep");
}

#[test]
fn a_task_that_sleeps_by_itself_is_not_held_up_by_the_budget() {
    let steps = Steps::new();
    let started = Instant::now();
    let slept = block_on_stepped(
        async {
            for _ in 0..5 {
                embassy_time::Timer::after_millis(10).await;
            }
            5
        },
        &steps,
        Duration::from_millis(20),
    );
    assert_eq!(slept, 5);
    assert!(started.elapsed() < Duration::from_millis(200), "five sleeps of 10 ms took {:?}", started.elapsed());
    assert!(steps.longest().0 < 20_000, "a task that sleeps ran {} us at once", steps.longest().0);
}
