//! The loop sends a file a piece at a time and falls asleep between the pieces: the idle task of its core gets the
//! core, and the task watchdog is fed (6.6, the burner on 26.09 - /journal of 230 KB held the core for seconds). Alone
//! in its own test binary: `web::STEPS` is one per program.

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// A piece of the file read off flash, as long as the board's takes.
const PIECE_US: u64 = 5_000;

#[test]
fn a_file_of_sixty_four_pieces_is_sent_with_the_core_let_go_between_them() {
    let run = common::start(common::limits(Duration::from_secs(5)));
    run.board.piece_us.store(PIECE_US, Ordering::Relaxed);
    let mut c = TcpStream::connect(run.http).unwrap();
    c.write_all(b"GET /big.js HTTP/1.1\r\nHost: board\r\nConnection: close\r\n\r\n").unwrap();
    c.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut got = Vec::new();
    c.read_to_end(&mut got).unwrap();
    assert!(got.len() > common::BIG_FILE, "the file did not come whole: {} bytes", got.len());
    let (longest_us, _) = web::STEPS.longest();
    println!("the longest step of the loop: {longest_us} us, a piece {PIECE_US} us");
    assert!(longest_us < 10 * PIECE_US as u32, "the loop ran {longest_us} us without falling asleep");
}
