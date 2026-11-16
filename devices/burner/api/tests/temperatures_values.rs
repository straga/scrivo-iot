//! What the temperatures' module hands the stream for what the bus said: a number, a socket with nothing in it, a module that did not answer - three things,
//! three codes - the role each socket plays, whether any does, and the state of the bus. Expected from the modules' frames (iot/burner/docs/01_REF_Modbus_Modules.md:
//! tenths of a degree, 0xF555 for an empty socket) and from the first burner's board, which says the same of the same readings - not from the code that makes them.
//!
//! The frames go through the logic crates the module goes through (crates/burner_map, crates/modbus) as the module does, and then through the API: what is held
//! here is the way from the bytes on the wire to the code on the stream, everything but the UART.

use burner_api::temperatures::{at, put_values};
use burner_map::{Readings, COUNT, MAP, UNREAD};
use modbus::{Counts, Outcome, Receiver, Reading, Tally};
use page::dictionary::{Said, Value};

#[derive(Debug, PartialEq)]
enum Got {
    Is(Value),
    Text(String),
}

const FREE: [&str; COUNT] = ["free"; COUNT];

fn said(readings: &Readings, roles: &[&str; COUNT], counts: &Counts) -> Vec<(usize, Got)> {
    let mut out = Vec::new();
    put_values(readings, roles, counts, &mut |at, said| {
        out.push((at, match said {
            Said::Value(value) => Got::Is(value),
            Said::Text(text) => Got::Text(text.to_string()),
        }))
    });
    out
}

fn get<'a>(said: &'a [(usize, Got)], at: usize) -> &'a Got {
    &said.iter().find(|(a, _)| *a == at).unwrap_or_else(|| panic!("nothing said at {at}: {said:?}")).1
}

fn value(said: &[(usize, Got)], at: usize) -> Value {
    match get(said, at) {
        Got::Is(value) => *value,
        other => panic!("a text at {at}: {other:?}"),
    }
}

/// The frame a module sends for a reading of its registers: its address, the function, the byte count, the registers, the CRC low byte first.
fn reply(addr: u8, registers: &[u16]) -> Vec<u8> {
    let mut frame = vec![addr, 0x03, (registers.len() * 2) as u8];
    for r in registers {
        frame.extend(r.to_be_bytes());
    }
    frame.extend(modbus::crc16(&frame).to_le_bytes());
    frame
}

/// A round as the module makes it: every asking the map needs, answered by BUS - the bytes that come back for an asking, none for a silent module.
fn round(address: u8, bus: impl Fn(u8) -> Option<Vec<u8>>) -> (Readings, Counts) {
    let channels = burner_map::channels(&MAP, |_| address);
    let mut built = [modbus::Ask::read(0, 0, 0); burner_map::MAX_ASKS];
    let made = burner_map::asks(&channels, &mut built);
    let (mut read, mut counts, mut rx) = (UNREAD, Counts::default(), Receiver::default());
    for ask in &built[..made] {
        rx.ask(*ask);
        if let Some(bytes) = bus(ask.addr) {
            rx.feed(&bytes);
        }
        counts.count(rx.outcome());
        if rx.outcome() != Outcome::Accepted {
            continue;
        }
        for (slot, channel) in read.iter_mut().zip(&channels) {
            match modbus::reading(channel, ask, rx.values()) {
                Reading::Unknown => {}
                said => *slot = said,
            }
        }
    }
    (read, counts)
}

#[test]
fn a_number_a_socket_with_nothing_in_it_and_a_module_that_did_not_answer_are_three_different_things() {
    let mut read = UNREAD;
    read[0] = Reading::Is(61.5);
    read[1] = Reading::Empty;
    read[2] = Reading::Unknown;
    let s = said(&read, &FREE, &Counts::default());
    assert_eq!(value(&s, at::CHANNEL[0]), Value::Number(61.5));
    assert_eq!(value(&s, at::CHANNEL[1]), Value::Unknown("no_sensor"));
    assert_eq!(value(&s, at::CHANNEL[2]), Value::Unknown("not_read"));
}

#[test]
fn every_socket_has_its_own_address_and_its_own_role_part() {
    let all: std::collections::BTreeSet<usize> = at::CHANNEL.iter().chain(&at::CHANNEL_ROLE).copied().collect();
    assert_eq!(all.len(), 2 * COUNT);
    let s = said(&UNREAD, &FREE, &Counts::default());
    for a in at::CHANNEL {
        assert!(s.iter().any(|(x, _)| *x == a), "the socket at {a} says nothing");
    }
}

#[test]
fn a_role_of_the_list_goes_by_its_code_and_one_of_ones_own_by_the_words_that_were_written() {
    let mut roles = FREE;
    roles[0] = "boiler";
    roles[1] = "own:hopper floor";
    let s = said(&UNREAD, &roles, &Counts::default());
    assert_eq!(value(&s, at::CHANNEL_ROLE[0]), Value::Word("boiler"));
    assert_eq!(get(&s, at::CHANNEL_ROLE[1]), &Got::Text("hopper floor".into()));
    assert_eq!(value(&s, at::CHANNEL_ROLE[2]), Value::Word("free"));
}

#[test]
fn the_card_says_so_while_every_socket_is_free_and_stops_when_one_is_given_a_role() {
    let s = said(&UNREAD, &FREE, &Counts::default());
    assert_eq!(value(&s, at::NONE_ASSIGNED), Value::On(true));
    let mut roles = FREE;
    roles[4] = "return";
    assert_eq!(value(&said(&UNREAD, &roles, &Counts::default()), at::NONE_ASSIGNED), Value::On(false));
    roles = FREE;
    roles[6] = "own:the shed";
    assert_eq!(value(&said(&UNREAD, &roles, &Counts::default()), at::NONE_ASSIGNED), Value::On(false));
}

#[test]
fn the_bus_is_ok_while_every_asking_was_answered_and_losing_when_one_was_missed_broken_or_foreign() {
    let now = |tally: Tally| Counts { now: tally, last: None };
    let bus = |counts: Counts| value(&said(&UNREAD, &FREE, &counts), at::BUS);
    assert_eq!(bus(now(Tally { accepted: 50, ..Tally::default() })), Value::Word("ok"));
    for lost in [Tally { accepted: 50, missed: 1, ..Tally::default() }, Tally { accepted: 50, broken: 1, ..Tally::default() }, Tally { accepted: 50, foreign: 1, ..Tally::default() }] {
        assert_eq!(bus(now(lost)), Value::Word("losing"), "{lost:?}");
    }
}

#[test]
fn the_bus_counts_what_was_read_and_what_was_not_and_the_last_full_window_only_once_there_is_one() {
    let tally = Tally { accepted: 40, missed: 5, broken: 3, foreign: 2, exception: 0 };
    let s = said(&UNREAD, &FREE, &Counts { now: tally, last: None });
    assert_eq!(
        [at::BUS_READ, at::BUS_SILENT, at::BUS_BROKEN, at::BUS_FOREIGN].map(|a| value(&s, a)),
        [Value::Number(40.0), Value::Number(5.0), Value::Number(3.0), Value::Number(2.0)]
    );
    assert_eq!(value(&s, at::BUS_LAST_SILENT), Value::None);
    let closed = said(&UNREAD, &FREE, &Counts { now: Tally::default(), last: Some(Tally { missed: 7, accepted: 993, ..Tally::default() }) });
    assert_eq!(value(&closed, at::BUS_LAST_SILENT), Value::Number(7.0));
}

#[test]
fn seven_registers_off_the_wire_become_seven_codes_with_the_empty_socket_not_a_temperature() {
    // 20.7, 61.5, an empty socket, -23.5 (0xFF15), 0.0, 99.9 and 100.0 degrees, in tenths
    let registers = [0x00CF, 0x0267, 0xF555, 0xFF15, 0x0000, 0x03E7, 0x03E8];
    let (read, counts) = round(2, |addr| (addr == 2).then(|| reply(2, &registers)));
    assert_eq!(counts.now, Tally { accepted: 1, ..Tally::default() });
    let s = said(&read, &FREE, &counts);
    let expected = [
        Value::Number(20.7),
        Value::Number(61.5),
        Value::Unknown("no_sensor"),
        Value::Number(-23.5),
        Value::Number(0.0),
        Value::Number(99.9),
        Value::Number(100.0),
    ];
    for (n, want) in expected.into_iter().enumerate() {
        assert_eq!(value(&s, at::CHANNEL[n]), want, "socket {}", n + 1);
    }
    assert_eq!(value(&s, at::BUS), Value::Word("ok"));
}

#[test]
fn a_module_that_is_silent_leaves_every_socket_unread_and_the_bus_losing() {
    let (read, counts) = round(2, |_| None);
    let s = said(&read, &FREE, &counts);
    for a in at::CHANNEL {
        assert_eq!(value(&s, a), Value::Unknown("not_read"), "at {a}");
    }
    assert_eq!(value(&s, at::BUS), Value::Word("losing"));
    assert_eq!(value(&s, at::BUS_SILENT), Value::Number(1.0));
}

#[test]
fn another_module_answering_in_its_place_says_nothing_of_our_sockets() {
    // the PTA9B01 can be set to send by itself: its frame lands in the middle of an asking addressed to the NT18B07
    let (read, counts) = round(2, |_| Some(reply(3, &[0x00CF; 7])));
    let s = said(&read, &FREE, &counts);
    assert_eq!(value(&s, at::CHANNEL[0]), Value::Unknown("not_read"));
    assert_eq!(value(&s, at::BUS_FOREIGN), Value::Number(1.0));
    assert_eq!(value(&s, at::BUS), Value::Word("losing"));
}

#[test]
fn a_frame_with_a_wrong_crc_is_not_a_reading() {
    let mut frame = reply(2, &[0x0267; 7]);
    *frame.last_mut().unwrap() ^= 0xFF;
    let (read, counts) = round(2, |_| Some(frame.clone()));
    assert_eq!(value(&said(&read, &FREE, &counts), at::CHANNEL[1]), Value::Unknown("not_read"));
    assert_eq!(counts.now.broken, 1);
}
