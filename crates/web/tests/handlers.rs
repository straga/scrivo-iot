//! What holds the page's handlers, as `status` says it (on the burner: /state went unanswered for 15 s
//! while the console answered). Alone in its binary: the counts are the process's, and a test beside it would add its
//! own connections.

mod common;

use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// web::handlers() once it says WANTED, or what it said last after two seconds.
fn settled(wanted: web::Handlers) -> web::Handlers {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        let now = web::handlers();
        if now == wanted || Instant::now() > until {
            return now;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_stream_and_a_kept_connection_are_told_apart_and_let_go_when_they_close() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut stream = common::stream(run.http);
    common::read_for(&mut stream, Duration::from_millis(300));
    let mut kept = TcpStream::connect(run.http).unwrap();
    kept.write_all(b"GET /state HTTP/1.1\r\nHost: board\r\n\r\n").unwrap();
    kept.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    assert!(common::answer_of(&mut kept).is_some(), "the kept connection was not answered");

    let both = web::Handlers { held: 2, streams: 1, browser_consoles: 0, answering: 0, kept: 1 };
    assert_eq!(settled(both), both, "a stream and a connection kept between its requests");

    // the pair: closed, they hold nothing
    drop(stream);
    drop(kept);
    let none = web::Handlers { held: 0, streams: 0, browser_consoles: 0, answering: 0, kept: 0 };
    assert_eq!(settled(none), none, "closed connections still counted");
}
