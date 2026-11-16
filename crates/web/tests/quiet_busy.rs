//! The pair of tests/quiet.rs: the same measure over a loop kept busy. Its own binary, for the same reason.

mod common;

use std::time::{Duration, Instant};

/// The CPU time this process has used so far, user and system.
fn cpu() -> Duration {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    let t = |tv: libc::timeval| Duration::from_secs(tv.tv_sec as u64) + Duration::from_micros(tv.tv_usec as u64);
    t(usage.ru_utime) + t(usage.ru_stime)
}

#[test]
fn a_loop_kept_busy_is_seen_by_this_measure() {
    // the pair: the same measure over a loop that has work - changes announced as fast as it takes them - reads
    // well over the bound, so a quiet reading is the loop's and not a measure that cannot see anything
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut s = common::stream(run.http);
    common::read_for(&mut s, Duration::from_millis(300));
    let reader = std::thread::spawn(move || common::read_for(&mut s, Duration::from_millis(1200)));
    let (wall, before) = (Instant::now(), cpu());
    while wall.elapsed() < Duration::from_secs(1) {
        run.board.set(0, wall.elapsed().as_micros() as f32);
        web::announce("flame");
    }
    let used = cpu() - before;
    reader.join().unwrap();
    assert!(used > Duration::from_millis(100), "a busy loop measured {used:?} of CPU in a second");
}
