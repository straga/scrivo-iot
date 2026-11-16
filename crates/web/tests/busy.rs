//! The owner of commands held by a long order: the streams go on, and a command from the page is refused `Busy` at
//! once instead of waiting the long order out. A short order is waited for.

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// A console on port 23 with its prompt read.
fn console(run: &common::Running) -> TcpStream {
    let mut c = TcpStream::connect(run.telnet).unwrap();
    let got = common::read_for(&mut c, Duration::from_millis(500));
    assert!(got.contains("rust> "), "no prompt on port 23: {got:?}");
    c
}

#[test]
fn a_command_behind_a_long_order_is_refused_at_once_and_the_stream_goes_on() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut stream = common::stream(run.http);
    common::read_for(&mut stream, Duration::from_millis(300));
    let mut c = console(&run);

    c.write_all(b"long\r\n").unwrap();
    std::thread::sleep(Duration::from_millis(700));
    let asked = Instant::now();
    let answer = common::ask(run.http, &common::post(&format!("/toggle/{}", common::PUMP)), Duration::from_secs(5));
    let took = asked.elapsed();
    let answer = answer.expect("the command got no answer at all");
    assert!(answer.contains("busy"), "not refused as busy: {answer}");
    assert!(took < Duration::from_secs(1), "the refusal took {took:?} behind a long order");

    run.board.set(2, 9.0);
    web::announce("flame");
    let heard = common::read_for(&mut stream, Duration::from_millis(300));
    assert!(heard.contains("[[2,9.0]]"), "the stream stood while the long order ran: {heard:?}");

    let mut rest = [0u8; 256];
    c.set_read_timeout(Some(common::LONG)).unwrap();
    let n = c.read(&mut rest).unwrap_or(0);
    assert!(String::from_utf8_lossy(&rest[..n]).contains("long done"), "the long order itself did not end");
}

#[test]
fn a_command_behind_a_short_order_waits_for_it_and_is_carried_out() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut c = console(&run);
    c.write_all(b"short\r\n").unwrap();
    std::thread::sleep(Duration::from_millis(20));
    let answer = common::ask(run.http, &common::post(&format!("/toggle/{}", common::PUMP)), Duration::from_secs(5));
    let answer = answer.expect("the command got no answer at all");
    assert!(answer.contains(&format!("[[{},true]]", common::PUMP)), "the command was not carried out: {answer}");
}
