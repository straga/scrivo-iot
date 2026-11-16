//! Six handlers, the memory for them set at start: a seventh connection while all six hold one waits for one to let
//! go - a kept-alive connection lets go within KEEP_ALIVE_MS - and breaks none of the six. Refused it is only when the six
//! hold on without a break for the whole `wait_for_a_handler` (tests/burst.rs).

mod common;

use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

const STATE: &str = "GET /state HTTP/1.1\r\nHost: board\r\n\r\n";

/// A connection that asked once and holds its handler, kept alive.
fn held(run: &common::Running) -> TcpStream {
    let mut s = TcpStream::connect(run.http).unwrap();
    s.write_all(STATE.as_bytes()).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    assert!(common::answer_of(&mut s).is_some(), "one of the six was not answered");
    s
}

#[test]
fn a_seventh_connection_while_six_are_kept_alive_is_answered_once_one_lets_go_and_breaks_none_of_the_six() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut six: Vec<TcpStream> = (0..web::HANDLERS).map(|_| held(&run)).collect();

    let asked = Instant::now();
    let mut seventh = TcpStream::connect(run.http).unwrap();
    seventh.write_all(STATE.as_bytes()).unwrap();
    seventh.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
    let answer = common::answer_of(&mut seventh).unwrap_or_default();
    assert!(answer.starts_with("HTTP/1.1 200"), "a seventh while six were kept alive was told {answer:?}");
    assert!(asked.elapsed() < Duration::from_millis(1500), "the seventh waited {:?}, and a kept connection lets go within 1 s", asked.elapsed());

    // the six went quiet for their keep-alive and were let go; each asks again on a connection of its own
    for (n, s) in six.iter_mut().enumerate() {
        let _ = s.write_all(STATE.as_bytes());
        let _ = common::answer_of(s);
        assert!(common::ask(run.http, STATE, Duration::from_secs(2)).is_some(), "connection {n} of the six was not answered again");
    }
}

#[test]
fn with_a_handler_free_a_new_connection_is_answered_at_once() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let _five: Vec<TcpStream> = (0..web::HANDLERS - 1).map(|_| held(&run)).collect();
    let asked = Instant::now();
    assert!(common::ask(run.http, STATE, Duration::from_secs(1)).is_some(), "the sixth was not answered");
    assert!(asked.elapsed() < Duration::from_millis(300), "the sixth waited {:?}", asked.elapsed());
}
