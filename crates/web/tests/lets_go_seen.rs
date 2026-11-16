//! The pair of lets_go.rs: a piece that holds the loop long is seen as a long step - the measure is not blind. Alone in
//! its own test binary: `web::STEPS` is one per program.

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::time::Duration;

const PIECE_US: u64 = 300_000;

#[test]
fn a_piece_that_holds_the_loop_is_seen_as_a_step_at_least_as_long() {
    let run = common::start(common::limits(Duration::from_secs(5)));
    run.board.piece_us.store(PIECE_US, Ordering::Relaxed);
    let mut c = TcpStream::connect(run.http).unwrap();
    c.write_all(b"GET /index.html HTTP/1.1\r\nHost: board\r\nConnection: close\r\n\r\n").unwrap();
    c.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut got = Vec::new();
    c.read_to_end(&mut got).unwrap();
    let (longest_us, at_ms) = web::STEPS.longest();
    println!("the longest step of the loop: {longest_us} us, ended at {at_ms} ms");
    // CPU, and the piece is timed by the clock: a thread taken off the CPU for a moment spends a little less of it
    assert!(longest_us >= PIECE_US as u32 * 9 / 10, "a piece of {PIECE_US} us was seen as a step of {longest_us} us");
}
