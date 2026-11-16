//! What a Tasmota socket's answer says: watts, volts and amps taken out of the body of `Status 10`, and the answers that say none of them. The bodies are the socket's own, read from a Tasmota 8.4.0 socket, not made up here.

use tasmota::{body_of, parse, Energy, Fault};

const LIVE: &str = r#"{"StatusSNS":{"Time":"2026-11-03T12:19:09","ENERGY":{"TotalStartTime":"2020-11-13T00:00:00","Total":451.545,"Yesterday":0.183,"Today":0.200,"Power":3,"ApparentPower":14,"ReactivePower":13,"Factor":0.21,"Voltage":227,"Current":0.060}}}"#;

const LIVE_HEAD: &str = "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-cache, no-store, must-revalidate\r\nPragma: no-cache\r\nExpires: -1\r\nConnection: close\r\n\r\n";

#[test]
fn the_socket_s_own_answer_gives_its_watts_volts_and_amps() {
    assert_eq!(parse(LIVE.as_bytes()), Ok(Energy { watts: 3.0, volts: 227.0, amps: 0.06 }));
}

#[test]
fn a_load_is_read_as_the_number_the_socket_said() {
    let body = LIVE.replace(r#""Power":3"#, r#""Power":62"#).replace(r#""Current":0.060"#, r#""Current":0.318"#);
    assert_eq!(parse(body.as_bytes()), Ok(Energy { watts: 62.0, volts: 227.0, amps: 0.318 }));
}

#[test]
fn a_socket_switched_off_says_zero_and_zero_is_a_reading() {
    let body = LIVE.replace(r#""Power":3"#, r#""Power":0"#).replace(r#""Voltage":227"#, r#""Voltage":0"#).replace(r#""Current":0.060"#, r#""Current":0.000"#);
    assert_eq!(parse(body.as_bytes()), Ok(Energy { watts: 0.0, volts: 0.0, amps: 0.0 }));
}

#[test]
fn watts_with_a_fraction_are_kept_with_it() {
    let body = LIVE.replace(r#""Power":3"#, r#""Power":3.5"#);
    assert_eq!(parse(body.as_bytes()).unwrap().watts, 3.5);
}

#[test]
fn a_socket_without_a_meter_is_not_a_reading_of_zero() {
    let body = r#"{"StatusSNS":{"Time":"2026-11-03T12:19:09"}}"#;
    assert_eq!(parse(body.as_bytes()), Err(Fault::NoEnergy));
}

#[test]
fn an_energy_block_that_lacks_a_quantity_is_not_a_reading_of_zero() {
    for lacking in ["Power", "Voltage", "Current"] {
        let body = LIVE.replace(&format!(r#""{lacking}":"#), r#""Other":"#);
        assert_eq!(parse(body.as_bytes()), Err(Fault::NoEnergy), "without {lacking}");
    }
}

#[test]
fn nothing_at_all_is_no_answer() {
    assert_eq!(parse(b""), Err(Fault::Closed));
}

#[test]
fn text_that_is_not_json_is_refused() {
    assert_eq!(parse(b"<html>captive portal</html>"), Err(Fault::NotJson));
}

#[test]
fn an_answer_cut_short_is_refused_and_not_read_as_far_as_it_goes() {
    let cut = &LIVE.as_bytes()[..LIVE.len() - 30];
    assert_eq!(parse(cut), Err(Fault::NotJson));
}

#[test]
fn a_quantity_that_is_not_a_number_is_refused() {
    let body = LIVE.replace(r#""Power":3"#, r#""Power":"3""#);
    assert_eq!(parse(body.as_bytes()), Err(Fault::NoEnergy));
}

#[test]
fn the_body_is_what_follows_the_blank_line_of_an_answer_that_says_ok() {
    let answer = format!("{LIVE_HEAD}{LIVE}");
    assert_eq!(body_of(answer.as_bytes()), Ok(LIVE.as_bytes()));
}

#[test]
fn an_answer_that_does_not_say_ok_is_refused_with_the_status_it_said() {
    assert_eq!(body_of(b"HTTP/1.0 404 Not Found\r\nConnection: close\r\n\r\nnope"), Err(Fault::Refused(404)));
    assert_eq!(body_of(b"HTTP/1.1 401 Unauthorized\r\n\r\n"), Err(Fault::Refused(401)));
}

#[test]
fn an_answer_whose_head_has_not_ended_has_no_body() {
    assert_eq!(body_of(b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n"), Err(Fault::NotJson));
}

#[test]
fn an_answer_in_chunks_is_refused_and_not_misread_with_its_sizes_inside() {
    let answer = format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\ne9\r\n{LIVE}\r\n0\r\n\r\n");
    assert_eq!(body_of(answer.as_bytes()), Err(Fault::Chunked));
}

#[test]
fn a_line_that_is_not_http_at_all_is_refused() {
    assert_eq!(body_of(b"SSH-2.0-OpenSSH_9.0\r\n\r\n"), Err(Fault::NotJson));
}

#[test]
fn every_fault_has_a_reason_a_client_can_tell_a_person_by_code() {
    use Fault::*;
    // the quiet ones: no one is there; the unreadable ones: someone is there and it is not a meter
    for f in [Silent, Unreachable, Closed] {
        assert_eq!(f.reason(), "silent", "{f:?}");
    }
    for f in [TooLong, Refused(404), Chunked, NotJson, NoEnergy] {
        assert_eq!(f.reason(), "unreadable", "{f:?}");
    }
}
