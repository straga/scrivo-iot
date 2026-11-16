//! A reader that takes nothing is let go by the write limit, and the loop does not stand still behind it: the other
//! reader gets every change meanwhile.

mod common;

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Changes the board for DURING, every EVERY, each change a number and forty texts of a hundred bytes; the last number.
fn keep_changing(run: &common::Running, during: Duration, every: Duration) -> f32 {
    let until = Instant::now() + during;
    let mut n = 0.0;
    while Instant::now() < until {
        n += 1.0;
        run.board.set(0, n);
        run.board.texts(&format!("{n:>100}"));
        web::announce("flame");
        std::thread::sleep(every);
    }
    n
}

/// Reads S in its own thread until DONE, every piece stamped with when it came.
fn reader(mut s: std::net::TcpStream, pace: Option<Duration>, done: Arc<AtomicBool>) -> Arc<Mutex<Vec<(Instant, String)>>> {
    let got = Arc::new(Mutex::new(Vec::new()));
    let into = got.clone();
    std::thread::spawn(move || {
        s.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
        let mut buf = [0u8; 16384];
        while !done.load(Ordering::Relaxed) {
            match s.read(&mut buf) {
                Ok(0) => {
                    into.lock().unwrap().push((Instant::now(), "<closed>".to_string()));
                    return;
                }
                Ok(n) => into.lock().unwrap().push((Instant::now(), String::from_utf8_lossy(&buf[..n]).into_owned())),
                Err(_) => {}
            }
            if let Some(pace) = pace {
                std::thread::sleep(pace);
            }
        }
    });
    got
}

fn longest_gap(got: &[(Instant, String)], from: Instant, to: Instant) -> Duration {
    let mut stamps: Vec<Instant> = got.iter().map(|(t, _)| *t).filter(|t| *t >= from && *t <= to).collect();
    stamps.insert(0, from);
    stamps.push(to);
    stamps.windows(2).map(|w| w[1] - w[0]).max().unwrap()
}

#[test]
fn a_reader_that_takes_nothing_is_let_go_and_the_other_gets_every_change_meanwhile() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut idle = common::stream(run.http);
    let done = Arc::new(AtomicBool::new(false));
    let fast = reader(common::stream(run.http), None, done.clone());
    std::thread::sleep(Duration::from_millis(200));

    let from = Instant::now();
    // far faster than any socket takes it: the loop always has an event to write
    let last = keep_changing(&run, Duration::from_secs(3), Duration::from_millis(2));
    let to = Instant::now();
    std::thread::sleep(Duration::from_millis(500));
    done.store(true, Ordering::Relaxed);

    let got = fast.lock().unwrap();
    let text: String = got.iter().map(|(_, t)| t.as_str()).collect();
    assert!(text.contains(&format!("[0,{last:.1}]")), "the reader that reads never got the last change, {last}");
    let gap = longest_gap(&got, from, to);
    assert!(gap < Duration::from_millis(400), "the reader that reads waited {gap:?} for an event while the other took nothing");

    // what the idle one was sent is in its socket; after it, the loop has closed it
    let rest = common::read_for(&mut idle, Duration::from_secs(5));
    let mut end = [0u8; 1];
    idle.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    assert!(
        matches!(idle.read(&mut end), Ok(0)),
        "the reader that took nothing is still held after {} bytes read back",
        rest.len()
    );
}

#[test]
fn a_reader_that_reads_slowly_within_the_limit_is_not_let_go() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let done = Arc::new(AtomicBool::new(false));
    let slow = reader(common::stream(run.http), Some(Duration::from_millis(50)), done.clone());
    std::thread::sleep(Duration::from_millis(200));

    // the board's own pace, ten times a second (the draught's and the flame's rounds)
    let last = keep_changing(&run, Duration::from_secs(2), Duration::from_millis(100));
    // what the socket's buffers hold on the way takes this reader a while to get through
    let wanted = format!("[0,{last:.1}]");
    let until = Instant::now() + Duration::from_secs(5);
    let seen = || slow.lock().unwrap().iter().map(|(_, t)| t.clone()).collect::<String>().contains(&wanted);
    while !seen() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(50));
    }
    done.store(true, Ordering::Relaxed);

    let got = slow.lock().unwrap();
    assert!(!got.iter().any(|(_, t)| t == "<closed>"), "a reader within the limit was let go");
    let text: String = got.iter().map(|(_, t)| t.as_str()).collect();
    assert!(text.contains(&wanted), "the slow reader never got the last change, {last}");
}
