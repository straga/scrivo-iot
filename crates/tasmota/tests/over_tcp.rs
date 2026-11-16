//! The whole read over real sockets on the computer: a connect, the request, the answer to the close, and the faults a network has - nobody listening, somebody who never answers, an answer that dribbles in, a head
//! that is not a meter's. The listener is a thread of std; the board's side is edge-nal-std, the stack the page runs on.
//! What the listener received is checked in every case: it is only ever the one request that reads.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use embassy_futures::block_on;
use tasmota::{read, Energy, Fault, REQUEST};

const BODY: &str = r#"{"StatusSNS":{"Time":"2026-11-03T12:19:09","ENERGY":{"Power":62,"Voltage":227,"Current":0.318}}}"#;

fn answer(body: &str) -> Vec<u8> {
    format!("HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}").into_bytes()
}

/// A listener that does BEHAVE with the first connection, and tells what it received through the channel.
fn serve(behave: impl FnOnce(&mut std::net::TcpStream) + Send + 'static) -> (SocketAddr, mpsc::Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let at = listener.local_addr().unwrap();
    let (said, heard) = mpsc::channel();
    thread::spawn(move || {
        let (mut conn, _) = listener.accept().unwrap();
        let mut got = Vec::new();
        let mut buf = [0u8; 256];
        while !got.ends_with(b"\r\n\r\n") {
            match conn.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => got.extend_from_slice(&buf[..n]),
            }
        }
        let _ = said.send(got);
        behave(&mut conn);
    });
    (at, heard)
}

fn ask(at: SocketAddr, within: Duration) -> Result<Energy, Fault> {
    let mut buf = [0u8; 512];
    block_on(read(&edge_nal_std::Stack::new(), at, &mut buf, within))
}

#[test]
fn a_socket_that_answers_gives_its_load_and_has_been_sent_the_one_request() {
    let (at, heard) = serve(|c| {
        c.write_all(&answer(BODY)).unwrap();
    });
    assert_eq!(ask(at, Duration::from_millis(1500)), Ok(Energy { watts: 62.0, volts: 227.0, amps: 0.318 }));
    assert_eq!(heard.recv().unwrap(), REQUEST);
}

#[test]
fn an_answer_that_dribbles_in_is_taken_if_it_ends_within_the_time() {
    let (at, heard) = serve(|c| {
        for piece in answer(BODY).chunks(11) {
            c.write_all(piece).unwrap();
            c.flush().unwrap();
            thread::sleep(Duration::from_millis(15));
        }
    });
    assert_eq!(ask(at, Duration::from_millis(1500)).map(|e| e.watts), Ok(62.0));
    assert_eq!(heard.recv().unwrap(), REQUEST);
}

#[test]
fn a_socket_that_never_answers_is_silent_after_the_time_and_not_after_longer() {
    let (at, heard) = serve(|_c| thread::sleep(Duration::from_secs(3)));
    let began = Instant::now();
    assert_eq!(ask(at, Duration::from_millis(300)), Err(Fault::Silent));
    let took = began.elapsed();
    assert!(took >= Duration::from_millis(280) && took < Duration::from_millis(900), "gave up after {took:?}");
    assert_eq!(heard.recv().unwrap(), REQUEST);
}

#[test]
fn an_answer_that_is_late_in_the_middle_is_silence_as_well() {
    let (at, _heard) = serve(|c| {
        let all = answer(BODY);
        c.write_all(&all[..20]).unwrap();
        thread::sleep(Duration::from_secs(3));
    });
    assert_eq!(ask(at, Duration::from_millis(300)), Err(Fault::Silent));
}

#[test]
fn nobody_listening_is_unreachable_at_once_and_not_after_the_time() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let at = listener.local_addr().unwrap();
    drop(listener);
    let began = Instant::now();
    assert_eq!(ask(at, Duration::from_millis(1500)), Err(Fault::Unreachable));
    assert!(began.elapsed() < Duration::from_millis(500));
}

#[test]
fn a_page_that_is_not_a_meter_is_unreadable_and_not_a_reading() {
    let (at, _heard) = serve(|c| {
        c.write_all(&answer(r#"{"StatusSNS":{"Time":"2026-11-03T12:19:09"}}"#)).unwrap();
    });
    assert_eq!(ask(at, Duration::from_millis(1500)), Err(Fault::NoEnergy));
}

#[test]
fn a_server_that_refuses_the_request_is_unreadable_with_its_status() {
    let (at, _heard) = serve(|c| {
        c.write_all(b"HTTP/1.0 404 Not Found\r\nConnection: close\r\n\r\nno such thing").unwrap();
    });
    assert_eq!(ask(at, Duration::from_millis(1500)), Err(Fault::Refused(404)));
}

#[test]
fn the_next_read_after_a_fault_is_a_fresh_one() {
    let (at, _h) = serve(|_c| thread::sleep(Duration::from_secs(2)));
    assert_eq!(ask(at, Duration::from_millis(200)), Err(Fault::Silent));
    let (at, _h) = serve(|c| {
        c.write_all(&answer(BODY)).unwrap();
    });
    assert_eq!(ask(at, Duration::from_millis(1500)).map(|e| e.watts), Ok(62.0));
}
