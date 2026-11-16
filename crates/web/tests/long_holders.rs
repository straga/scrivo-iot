//! A connection that lasts - a page's stream, a console in a browser - holds a handler all the while it lives, and a board
//! with few handlers (the small plan: three) keeps one back for the short requests: the page's files, /state, a command.
//! Without it a stream and a console left the third handler to whoever came, and a second page's stream took it and
//! left /state waiting until the board refused it.

mod common;

use std::net::TcpStream;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const STATE: &str = "GET /state HTTP/1.1\r\nHost: board\r\n\r\n";

/// The streams of every board in this process share one channel of changes with a few places (page::SUBSCRIBERS): the rows take turns.
static TURN: Mutex<()> = Mutex::new(());

fn start(long_holders: usize) -> (std::sync::MutexGuard<'static, ()>, common::Running) {
    let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let run = common::start(web::Limits { long_holders, ..common::limits(Duration::from_millis(500)) });
    (turn, run)
}

/// What a request for a stream is answered when the board will not hold it: the head of the answer.
fn refused_head(run: &common::Running) -> String {
    let mut s: TcpStream = common::stream(run.http);
    let said = common::read_for(&mut s, Duration::from_millis(800));
    said.lines().next().unwrap_or("").to_string()
}

fn short_request_is_answered_at_once(run: &common::Running) {
    let asked = Instant::now();
    assert!(common::ask(run.http, STATE, Duration::from_secs(2)).is_some(), "a short request was not answered");
    assert!(asked.elapsed() < Duration::from_millis(500), "a short request waited {:?}", asked.elapsed());
}

#[test]
fn a_stream_and_a_console_are_held_when_two_long_ones_are_allowed_and_a_short_request_is_answered_beside_them() {
    let (_turn, run) = start(2);
    let mut stream = common::stream(run.http);
    assert!(common::read_for(&mut stream, Duration::from_millis(500)).starts_with("HTTP/1.1 200"), "the first stream was not held");
    let (_console, said) = common::browser_said(&run);
    assert!(!said.contains("busy"), "the console was refused: {said:?}");
    short_request_is_answered_at_once(&run);
}

#[test]
fn a_second_stream_is_refused_in_words_while_a_stream_and_a_console_hold_the_two_and_the_short_request_is_still_answered() {
    let (_turn, run) = start(2);
    let mut first = common::stream(run.http);
    assert!(common::read_for(&mut first, Duration::from_millis(500)).starts_with("HTTP/1.1 200"));
    let _console = common::browser(&run);
    assert!(refused_head(&run).contains("503"), "a third long connection was held");
    short_request_is_answered_at_once(&run);
}

#[test]
fn a_console_is_refused_in_words_while_two_streams_hold_the_two() {
    let (_turn, run) = start(2);
    let mut one = common::stream(run.http);
    let mut two = common::stream(run.http);
    for s in [&mut one, &mut two] {
        assert!(common::read_for(s, Duration::from_millis(500)).starts_with("HTTP/1.1 200"));
    }
    let (_console, said) = common::browser_said(&run);
    assert!(said.contains("busy"), "the console was held beside two streams: {said:?}");
    short_request_is_answered_at_once(&run);
}

#[test]
fn a_place_given_up_is_taken_again() {
    let (_turn, run) = start(2);
    let mut one = common::stream(run.http);
    let mut two = common::stream(run.http);
    for s in [&mut one, &mut two] {
        assert!(common::read_for(s, Duration::from_millis(500)).starts_with("HTTP/1.1 200"));
    }
    drop(two);
    std::thread::sleep(Duration::from_millis(400));
    let (_console, said) = common::browser_said(&run);
    assert!(!said.contains("busy"), "a place that a stream gave up was not free: {said:?}");
}
