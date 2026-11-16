//! A future that is not the page's, run by the page's loop beside its own: what the burner reads its socket by. It is another's work in the loop's one task, so what it must never do is
//! the page's harm: take the loop down when it ends, or make the page wait while it waits for its own network.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use embassy_time::Timer;

const STATE: &str = "GET /state HTTP/1.1\r\nHost: board\r\n\r\n";

static BEATS: AtomicUsize = AtomicUsize::new(0);
static SLOW_ASKS: AtomicUsize = AtomicUsize::new(0);
static ENDED: AtomicUsize = AtomicUsize::new(0);

fn beating() -> web::outside::Errand {
    web::outside::boxed(async {
        loop {
            Timer::after(embassy_time::Duration::from_millis(10)).await;
            BEATS.fetch_add(1, Ordering::SeqCst);
        }
    })
}

/// A read of a socket that does not answer: it waits two seconds for each, as the board's read waits for its limit.
fn waiting_for_a_socket() -> web::outside::Errand {
    web::outside::boxed(async {
        loop {
            SLOW_ASKS.fetch_add(1, Ordering::SeqCst);
            Timer::after(embassy_time::Duration::from_secs(2)).await;
        }
    })
}

fn ending_at_once() -> web::outside::Errand {
    web::outside::boxed(async {
        ENDED.fetch_add(1, Ordering::SeqCst);
    })
}

fn state(run: &common::Running) -> (String, Duration) {
    let began = Instant::now();
    let said = common::ask(run.http, STATE, Duration::from_secs(2)).unwrap_or_default();
    (said, began.elapsed())
}

#[test]
fn errands_run_beside_the_page_and_the_page_is_never_made_to_wait_for_them_nor_taken_down_by_one_that_ends() {
    web::outside::add(beating).unwrap();
    web::outside::add(waiting_for_a_socket).unwrap();
    web::outside::add(ending_at_once).unwrap();
    let run = common::start(common::limits(Duration::from_millis(500)));
    std::thread::sleep(Duration::from_millis(300));

    // they run
    assert!(BEATS.load(Ordering::SeqCst) >= 10, "the beating errand ran {} times in 300 ms", BEATS.load(Ordering::SeqCst));
    assert_eq!(SLOW_ASKS.load(Ordering::SeqCst), 1, "the waiting errand was started once and is in its wait");
    assert_eq!(ENDED.load(Ordering::SeqCst), 1, "the errand that ends was run once");

    // the page is answered at its own pace while one errand waits two seconds for a socket
    for _ in 0..5 {
        let (said, took) = state(&run);
        assert!(said.starts_with("HTTP/1.1 200"), "the page was told {said:?}");
        assert!(took < Duration::from_millis(200), "the page waited {took:?} while an errand waited for its socket");
    }

    // the errand that ended did not end the loop: the page answers a second after, and the others still run
    let before = BEATS.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(1100));
    assert!(BEATS.load(Ordering::SeqCst) > before + 50, "the beating stopped");
    assert!(state(&run).0.starts_with("HTTP/1.1 200"));
    assert_eq!(ENDED.load(Ordering::SeqCst), 1, "the errand that ended was started again");
    assert!(SLOW_ASKS.load(Ordering::SeqCst) >= 1);

    // an errand added when the loop already runs is refused in words, not lost in silence
    assert_eq!(web::outside::add(beating), Err(web::outside::Late));
}
