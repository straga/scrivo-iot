//! The consoles in the loop: on port 23 and in browsers, the shell's answers and the log on both, a place more
//! refused in words; two streams and two browser consoles open leave room for the short requests (six handlers).

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use common::{browser, browser_said, frames, read_raw, typed};

fn telnet(run: &common::Running) -> TcpStream {
    let mut c = TcpStream::connect(run.telnet).unwrap();
    let got = common::read_for(&mut c, Duration::from_millis(500));
    assert!(got.contains("rust> "), "no prompt on port 23: {got:?}");
    c
}

#[test]
fn a_console_on_port_23_answers_its_line_and_carries_the_log() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut c = telnet(&run);
    c.write_all(b"status\r\n").unwrap();
    let got = common::read_for(&mut c, Duration::from_millis(500));
    assert!(got.contains("status done"), "the line was not answered: {got:?}");
    run.log.put("a line of the log");
    let got = common::read_for(&mut c, Duration::from_millis(300));
    assert!(got.contains("a line of the log\r\n"), "the log did not come within 300 ms: {got:?}");
}

#[test]
fn a_third_console_on_port_23_is_refused_in_words_and_the_two_go_on() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut two = [telnet(&run), telnet(&run)];
    let mut third = TcpStream::connect(run.telnet).unwrap();
    let said = common::read_for(&mut third, Duration::from_millis(500));
    assert!(said.contains("busy: both console places are taken"), "the third was told {said:?}");
    for (n, c) in two.iter_mut().enumerate() {
        c.write_all(b"after\r\n").unwrap();
        let got = common::read_for(c, Duration::from_millis(300));
        assert!(got.contains("after done"), "console {n} stopped answering: {got:?}");
    }
}

#[test]
fn a_console_in_a_browser_answers_its_line_and_a_third_is_refused() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut one = browser(&run);
    typed(&mut one, "status");
    let got = frames(&mut one, Duration::from_millis(500));
    assert!(got.contains("status done"), "the line was not answered: {got:?}");
    let _two = browser(&run);
    let (_third, said) = browser_said(&run);
    assert!(said.contains("busy: both browser consoles are open"), "the third was told {said:?}");
}

#[test]
fn with_two_streams_two_browser_consoles_and_a_kept_connection_open_a_request_is_answered_at_once() {
    // what a person's browser holds with the page open: its stream, its console, and the connection it keeps alive
    // between requests - twice, for two people
    let run = common::start(common::limits(Duration::from_millis(500)));
    let _streams = [common::stream(run.http), common::stream(run.http)];
    let _consoles = [browser(&run), browser(&run)];
    let state = "GET /state HTTP/1.1\r\nHost: board\r\n\r\n";
    let mut kept = TcpStream::connect(run.http).unwrap();
    kept.write_all(state.as_bytes()).unwrap();
    kept.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    assert!(common::answer_of(&mut kept).is_some(), "the kept connection was not answered");
    let asked = Instant::now();
    assert!(common::ask(run.http, state, Duration::from_secs(2)).is_some(), "the request beside them was not answered");
    assert!(asked.elapsed() < Duration::from_millis(500), "the request beside them waited {:?}", asked.elapsed());
}

#[test]
fn a_restart_is_answered_first_and_carried_out_after() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let answer = common::ask(run.http, &common::post("/restart"), Duration::from_secs(1)).expect("no answer to /restart");
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert_eq!(run.board.restarts.load(std::sync::atomic::Ordering::Relaxed), 0, "restarted before the answer left");
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(run.board.restarts.load(std::sync::atomic::Ordering::Relaxed), 1, "not restarted after its time");
    let mut buf = [0u8; 1];
    let _ = TcpStream::connect(run.http).map(|mut s| s.read(&mut buf));
}

#[test]
fn a_down_asked_by_the_board_s_name_is_answered_first_and_carried_out_after() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let answer = common::ask(run.http, &common::post("/down/HOST"), Duration::from_secs(1)).expect("no answer to /down");
    assert!(answer.contains("recovery"), "{answer}");
    assert_eq!(run.board.downs.load(std::sync::atomic::Ordering::Relaxed), 0, "went down before the answer left");
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(run.board.downs.load(std::sync::atomic::Ordering::Relaxed), 1, "not down after its time");
}

#[test]
fn a_down_asked_by_another_name_is_refused_and_nothing_is_carried_out() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let answer = common::ask(run.http, &common::post("/down/OTHER"), Duration::from_secs(1)).expect("no answer to /down");
    assert!(answer.contains("goes down only when asked by its name"), "{answer}");
    std::thread::sleep(Duration::from_millis(400));
    let (downs, restarts) = (run.board.downs.load(std::sync::atomic::Ordering::Relaxed), run.board.restarts.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!((downs, restarts), (0, 0), "a refused down was carried out");
}

#[test]
fn a_line_longer_than_the_shell_takes_is_refused_in_words_and_the_console_goes_on() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut one = browser(&run);
    typed(&mut one, &"x".repeat(200));
    let got = frames(&mut one, Duration::from_millis(500));
    assert!(got.contains("line longer than 64 bytes refused"), "the long line was told {got:?}");
    typed(&mut one, "status");
    let got = frames(&mut one, Duration::from_millis(500));
    assert!(got.contains("status done"), "the console did not go on after a long line: {got:?}");
}

#[test]
fn a_frame_no_one_typed_closes_the_console() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut one = browser(&run);
    typed(&mut one, &"x".repeat(2000));
    let mut end = [0u8; 64];
    one.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let mut closed = false;
    for _ in 0..20 {
        match one.read(&mut end) {
            Ok(0) => {
                closed = true;
                break;
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    assert!(closed, "a frame of 2000 bytes did not close the console");
}

#[test]
fn lines_of_the_log_faster_than_a_console_takes_them_lose_the_oldest_and_it_is_told() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut c = telnet(&run);
    for n in 0..200 {
        run.log.put(&format!("line {n}"));
    }
    let got = common::read_for(&mut c, Duration::from_millis(500));
    assert!(got.contains("lines dropped"), "the console was not told of the lines it lost: {got:?}");
    assert!(got.contains("line 199\r\n"), "the newest line did not come: {got:?}");
    // which of the oldest came is a race with the loop taking them while they are put; that the oldest go is held
    // without one by journal's tests of the ring
}

#[test]
fn a_burst_of_the_log_the_feed_holds_all_comes_and_none_is_said_lost() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut c = telnet(&run);
    for n in 0..web::LOG_FEED {
        run.log.put(&format!("line {n}"));
    }
    let got = common::read_for(&mut c, Duration::from_millis(500));
    assert!((0..web::LOG_FEED).all(|n| got.contains(&format!("line {n}\r\n"))), "not every line came: {got:?}");
    assert!(!got.contains("dropped"), "lines were said lost: {got:?}");
}
