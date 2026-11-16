//! A connection that went quiet between its requests does not hold a handler of the page (on the burner):
//! one browser keeps up to six connections to a board, and kept for 10 s each they left a second viewer - a phone - a
//! refusal. A stream and a console are no such quiet: they are answers that last, and the limit does not touch them.

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const STATE: &str = "GET /state HTTP/1.1\r\nHost: board\r\n\r\n";

/// A connection of a tab: asked once, answered, and kept by the browser.
fn asked_once(run: &common::Running) -> TcpStream {
    let mut s = TcpStream::connect(run.http).unwrap();
    s.write_all(STATE.as_bytes()).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    assert!(common::answer_of(&mut s).is_some(), "a tab's connection was not answered");
    s
}

#[test]
fn two_tabs_that_went_quiet_leave_a_third_viewer_an_answer() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    // two tabs, three connections each: every handler of the page asked once and then quiet
    let _tabs: Vec<TcpStream> = (0..web::HANDLERS).map(|_| asked_once(&run)).collect();
    std::thread::sleep(Duration::from_secs(2));
    let said = common::ask(run.http, STATE, Duration::from_secs(2)).unwrap_or_default();
    assert!(said.starts_with("HTTP/1.1 200"), "a third viewer, two seconds after the tabs went quiet, was told {said:?}");
}

#[test]
fn a_stream_in_quiet_outlives_the_limit_of_a_quiet_connection() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut stream = common::stream(run.http);
    common::read_for(&mut stream, Duration::from_millis(300));
    std::thread::sleep(Duration::from_secs(3));
    // nothing changed on the board: what keeps coming is the stream's beat
    let later = common::read_for(&mut stream, Duration::from_secs(3));
    assert!(!later.is_empty(), "the stream was let go in its quiet");
    let mut end = [0u8; 1];
    stream.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    assert!(!matches!(stream.read(&mut end), Ok(0)), "the stream was closed");
}

#[test]
fn a_console_in_a_browser_quiet_for_a_while_still_answers() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut console = common::browser(&run);
    std::thread::sleep(Duration::from_secs(3));
    common::typed(&mut console, "status");
    let said = common::frames(&mut console, Duration::from_secs(1));
    assert!(said.contains("status done"), "a console quiet for 3 s did not answer: {said:?}");
}
