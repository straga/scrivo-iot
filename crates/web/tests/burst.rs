//! A page's first load is a burst: the browser opens a connection a file, several at once, while the page's stream
//! holds a handler. A connection that finds every handler held waits for one to let go - a kept-alive connection lets
//! go within KEEP_ALIVE_MS - and is refused only when none does within `wait_for_a_handler` (the burner's
//! page stayed blank on 25.09: app.js answered 503 in a first load).

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// A browser's request for a page file, kept alive.
fn file(n: usize) -> String {
    format!("GET /file_{n}.js HTTP/1.1\r\nHost: board\r\n\r\n")
}

#[test]
fn a_burst_of_ten_page_files_beside_an_open_stream_is_answered_whole() {
    let run = common::start(common::limits(Duration::from_secs(5)));
    let _stream = common::stream(run.http);
    std::thread::sleep(Duration::from_millis(200));
    let asked = Instant::now();
    let burst: Vec<_> = (0..10)
        .map(|n| {
            let at = run.http;
            std::thread::spawn(move || {
                let mut s = TcpStream::connect(at).unwrap();
                s.write_all(file(n).as_bytes()).unwrap();
                s.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
                let answer = common::answer_of(&mut s);
                // a browser keeps its connection after the answer: the handler holds it KEEP_ALIVE_MS
                std::thread::sleep(Duration::from_secs(2));
                answer.map(|a| a.lines().next().unwrap_or("").to_string())
            })
        })
        .collect();
    let heads: Vec<Option<String>> = burst.into_iter().map(|t| t.join().unwrap()).collect();
    let refused: Vec<usize> = (0..10).filter(|n| !heads[*n].as_deref().is_some_and(|h| h.starts_with("HTTP/1.1 200"))).collect();
    assert!(refused.is_empty(), "files not answered 200 in a burst beside a stream: {refused:?} - {heads:?}");
    assert!(asked.elapsed() < Duration::from_secs(5), "the burst took {:?}", asked.elapsed());
}

#[test]
fn six_handlers_held_without_a_break_refuse_the_seventh_once_the_wait_is_over() {
    // the pair: a stream and five connections stuck in the middle of their request head hold all six and let none go.
    // The wait is counted from the last handler that moved - the refuser cannot see a connection in the TCP queue
    // until it takes it - so the seventh comes the moment the six are held, and is refused a whole wait later
    let limits = web::Limits { wait_for_a_handler: Duration::from_millis(1500), ..common::limits(Duration::from_secs(8)) };
    let run = common::start(limits);
    let _stream = common::stream(run.http);
    let _five: Vec<TcpStream> = (0..5)
        .map(|_| {
            let mut s = TcpStream::connect(run.http).unwrap();
            s.write_all(b"GET /state HTTP/1.1\r\nHost: bo").unwrap();
            s
        })
        .collect();
    std::thread::sleep(Duration::from_millis(50));
    let asked = Instant::now();
    let mut seventh = TcpStream::connect(run.http).unwrap();
    seventh.write_all(file(7).as_bytes()).unwrap();
    seventh.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut said = String::new();
    let _ = seventh.read_to_string(&mut said);
    let waited = asked.elapsed();
    assert!(said.starts_with("HTTP/1.1 503"), "the seventh while six held without a break was told {said:?}");
    assert!(said.contains("every connection of the board is taken"), "the refusal says nothing: {said:?}");
    assert!(waited >= Duration::from_millis(1350), "refused after {waited:?}, and the wait is 1.5 s");
    assert!(waited < Duration::from_millis(3000), "refused after {waited:?}, long past the wait of 1.5 s");
}
