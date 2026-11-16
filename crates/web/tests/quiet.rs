//! The loop in quiet must sleep, not spin: a page may sit open for hours with nothing to send. Alone in its own test
//! binary, so no other test burns the process's time while it is measured.

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
fn a_loop_with_two_streams_and_a_console_open_in_quiet_takes_almost_no_cpu() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut streams = [common::stream(run.http), common::stream(run.http)];
    for s in &mut streams {
        common::read_for(s, Duration::from_millis(300));
    }
    let mut console = std::net::TcpStream::connect(run.telnet).unwrap();
    common::read_for(&mut console, Duration::from_millis(300));
    let (wall, before) = (Instant::now(), cpu());
    std::thread::sleep(Duration::from_secs(1));
    let used = cpu() - before;
    println!("the loop in quiet used {used:?} of CPU in {:?}", wall.elapsed());
    assert!(used < Duration::from_millis(100), "the loop in quiet used {used:?} of CPU in {:?}", wall.elapsed());
}
