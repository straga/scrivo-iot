//! /journal answers the end of the journal the board holds in memory and reads no file of the flash for it; `?all`
//! answers the ring on the flash and the lines not written yet (6.6, the burner on 26.09: the whole ring read every
//! time held the loop).

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::time::Duration;

use journal::kept::Tail;

/// A ring of four files of 64 KB, as the board keeps it: numbered lines, the first file oldest.
fn full_ring(run: &common::Running) {
    let mut files = Vec::new();
    for k in 1..=4 {
        let mut bytes = Vec::new();
        let mut n = 0;
        while bytes.len() < 64 * 1024 {
            bytes.extend_from_slice(format!("file {k} line {n:05}\n").as_bytes());
            n += 1;
        }
        files.push((format!("{k}.log"), bytes));
    }
    *run.board.journal.lock().unwrap() = files;
}

/// The body of the answer to PATH, taken off its chunks.
fn body_of(run: &common::Running, path: &str) -> String {
    let mut c = TcpStream::connect(run.http).unwrap();
    c.write_all(format!("GET {path} HTTP/1.1\r\nHost: board\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
    c.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut raw = Vec::new();
    c.read_to_end(&mut raw).unwrap();
    let head = raw.windows(4).position(|w| w == b"\r\n\r\n").expect("no head") + 4;
    let heads = String::from_utf8_lossy(&raw[..head]).to_lowercase();
    assert!(heads.starts_with("http/1.1 200"), "{heads}");
    let mut rest = &raw[head..];
    if !heads.contains("transfer-encoding: chunked") {
        return String::from_utf8_lossy(rest).into_owned();
    }
    let mut body = Vec::new();
    loop {
        let line = rest.windows(2).position(|w| w == b"\r\n").expect("a chunk without its size");
        let size = usize::from_str_radix(std::str::from_utf8(&rest[..line]).unwrap().trim(), 16).unwrap();
        if size == 0 {
            break;
        }
        body.extend_from_slice(&rest[line + 2..line + 2 + size]);
        rest = &rest[line + 2 + size + 2..];
    }
    String::from_utf8(body).unwrap()
}

const WAITING: &str = "a line not written yet\n";

#[test]
fn the_journal_answers_its_end_from_memory_and_reads_no_file() {
    let run = common::start(common::limits(Duration::from_secs(5)));
    full_ring(&run);
    let text: String = (0..3000).map(|n| format!("file 4 line {n:05}\n")).collect();
    *run.board.tail.lock().unwrap() = Tail { left_out: true, text: text.clone() };
    let body = body_of(&run, "/journal");
    assert_eq!(run.board.journal_reads.load(Ordering::Relaxed), 0, "the flash was read for the journal's end");
    let (said, rest) = body.split_once('\n').unwrap();
    assert!(said.contains("/journal?all"), "the answer does not say where the rest is: {said:?}");
    assert_eq!(rest, text);
}

#[test]
fn an_end_that_leaves_nothing_out_is_answered_as_it_is() {
    let run = common::start(common::limits(Duration::from_secs(5)));
    *run.board.tail.lock().unwrap() = Tail { left_out: false, text: "igniter on\nflame seen\n".into() };
    assert_eq!(body_of(&run, "/journal"), "igniter on\nflame seen\n");
}

#[test]
fn the_journal_answers_the_whole_ring_on_the_flash_when_asked() {
    let run = common::start(common::limits(Duration::from_secs(5)));
    full_ring(&run);
    *run.board.tail.lock().unwrap() = Tail { left_out: true, text: "not this\n".into() };
    let body = body_of(&run, "/journal?all");
    let ring: usize = run.board.journal.lock().unwrap().iter().map(|(_, b)| b.len()).sum();
    assert_eq!(body.len(), ring + WAITING.len());
    assert!(body.starts_with("file 1 line 00000\n"));
    assert!(body.ends_with(WAITING));
}

// ---- one reader of the journal at a time ----

/// Asks PATH and reads only the head of the answer: the reader stays, its socket fills, and the board holds its answer.
fn held(run: &common::Running, path: &str) -> TcpStream {
    let mut c = TcpStream::connect(run.http).unwrap();
    c.write_all(format!("GET {path} HTTP/1.1\r\nHost: board\r\n\r\n").as_bytes()).unwrap();
    let mut head = [0u8; 12];
    c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    c.read_exact(&mut head).unwrap();
    assert_eq!(&head, b"HTTP/1.1 200", "the first reader was not answered");
    c
}

/// The status of the answer to PATH on a connection of its own, and how long it took.
fn status_of(run: &common::Running, path: &str) -> (String, Duration) {
    let started = std::time::Instant::now();
    let mut c = TcpStream::connect(run.http).unwrap();
    c.write_all(format!("GET {path} HTTP/1.1\r\nHost: board\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
    c.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut head = [0u8; 12];
    c.read_exact(&mut head).unwrap();
    (String::from_utf8_lossy(&head[9..]).into_owned(), started.elapsed())
}

/// A tail large enough that a reader who takes none of it holds its answer: more than a socket takes on its own.
fn a_tail_that_holds(run: &common::Running) {
    let text: String = (0..20_000).map(|n| format!("file 4 line {n:05}\n")).collect();
    *run.board.tail.lock().unwrap() = Tail { left_out: true, text };
}

#[test]
fn a_second_reader_of_the_journal_is_refused_at_once_while_the_first_reads() {
    let run = common::start(common::limits(Duration::from_secs(30)));
    full_ring(&run);
    a_tail_that_holds(&run);
    let first = held(&run, "/journal");
    for path in ["/journal", "/journal?all"] {
        let (status, took) = status_of(&run, path);
        assert_eq!(status, "503", "{path} beside a reader of the journal was answered {status}");
        assert!(took < Duration::from_millis(500), "{path} was refused after {took:?}");
    }
    drop(first);
}

#[test]
fn a_reader_of_the_journal_after_the_last_one_left_is_answered() {
    let run = common::start(common::limits(Duration::from_secs(30)));
    full_ring(&run);
    a_tail_that_holds(&run);
    drop(held(&run, "/journal"));
    let until = std::time::Instant::now() + Duration::from_secs(3);
    let mut status = String::new();
    while std::time::Instant::now() < until {
        status = status_of(&run, "/journal?all").0;
        if status == "200" {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(status, "200", "the journal is refused after its reader left");
}

// ---- each answer passes its own check and is refused by the other's ----

/// BODY is the end of the journal TAIL answers: its lines, after the word of what is left out when something is.
fn is_the_end(body: &str, tail: &Tail) -> Result<(), String> {
    let lines = match (tail.left_out, body.split_once('\n')) {
        (true, Some((said, rest))) if said.contains("/journal?all") => rest,
        (true, _) => return Err(format!("no word of what is left out: {:?}", &body[..body.len().min(60)])),
        (false, _) => body,
    };
    match lines == tail.text {
        true => Ok(()),
        false => Err(format!("{} bytes of lines where the end has {}", lines.len(), tail.text.len())),
    }
}

/// BODY is the whole ring of RUN, oldest first, and the lines not written yet after it.
fn is_the_whole(body: &str, run: &common::Running) -> Result<(), String> {
    let mut whole: String =
        run.board.journal.lock().unwrap().iter().map(|(_, b)| String::from_utf8_lossy(b).into_owned()).collect();
    whole.push_str(WAITING);
    match body == whole {
        true => Ok(()),
        false => Err(format!("{} bytes where the whole ring and its waiting lines are {}", body.len(), whole.len())),
    }
}

#[test]
fn the_end_and_the_whole_each_pass_their_own_check_and_fail_the_other() {
    let run = common::start(common::limits(Duration::from_secs(5)));
    full_ring(&run);
    let text: String = (0..3000).map(|n| format!("file 4 line {n:05}\n")).collect();
    let tail = Tail { left_out: true, text };
    *run.board.tail.lock().unwrap() = tail.clone();
    let (end, whole) = (body_of(&run, "/journal"), body_of(&run, "/journal?all"));
    let seen = [
        ("/journal", "is the end", is_the_end(&end, &tail)),
        ("/journal", "is the whole", is_the_whole(&end, &run)),
        ("/journal?all", "is the end", is_the_end(&whole, &tail)),
        ("/journal?all", "is the whole", is_the_whole(&whole, &run)),
    ];
    for (asked, check, said) in &seen {
        println!("{asked:13} {check:13} {said:?}");
    }
    assert!(seen[0].2.is_ok() && seen[3].2.is_ok(), "an answer fails its own check: {seen:?}");
    assert!(seen[1].2.is_err() && seen[2].2.is_err(), "an answer passes the other's check: {seen:?}");
}

#[test]
fn a_short_journal_whole_and_a_long_one_by_its_end_each_fail_the_others_check() {
    let short = Tail { left_out: false, text: "igniter on\nflame seen\n".into() };
    let long = Tail { left_out: true, text: (0..3000).map(|n| format!("file 4 line {n:05}\n")).collect() };
    let answer = |tail: &Tail| {
        let run = common::start(common::limits(Duration::from_secs(5)));
        *run.board.tail.lock().unwrap() = tail.clone();
        body_of(&run, "/journal")
    };
    let (of_short, of_long) = (answer(&short), answer(&long));
    let seen = [
        ("short", "as short", is_the_end(&of_short, &short)),
        ("short", "as long", is_the_end(&of_short, &long)),
        ("long", "as short", is_the_end(&of_long, &short)),
        ("long", "as long", is_the_end(&of_long, &long)),
    ];
    for (answered, check, said) in &seen {
        println!("{answered:6} {check:9} {said:?}");
    }
    assert!(seen[0].2.is_ok() && seen[3].2.is_ok(), "an answer fails its own check: {seen:?}");
    assert!(seen[1].2.is_err() && seen[2].2.is_err(), "an answer passes the other's check: {seen:?}");
}
