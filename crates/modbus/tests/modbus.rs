//! The frames of the burner's bus, taken off the bench on 20.09.2026 and kept whole in
//! scrivo-iot/runs/burner/2.1/bus_probe.hex: PTA9B01 on address 1, NT18B07 on address 2, both at 9600 8N1. A made-up
//! frame proves that the code agrees with itself; these were said by the modules.
//!
//! The requests are checked against two libraries we do not depend on - rmodbus (rmodbus,
//! src/client.rs, test_rtu_gen_get_holdings) and modbus-core (modbus-core, src/codec/rtu/client.rs,
//! encode_write_single_register_request) - so both ends of every asking are held against somebody else.
//!
//! A refusal that the bench did not produce is made from a live frame by editing it: the bad case then differs from
//! the good one in one named thing, and in nothing else.

use modbus::{crc16, parse, Ask, Counts, Outcome, Receiver, Refused, Tally, MAX_VALUES, WINDOW};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
}

/// The frame with its CRC made right again, however its bytes were edited.
fn sealed(head: &[u8]) -> Vec<u8> {
    let mut frame = head.to_vec();
    frame.extend(crc16(head).to_le_bytes());
    frame
}

/// Takes the answer apart, values and all.
fn taken(ask: &Ask, frame: &[u8]) -> Result<Vec<u16>, Refused> {
    let mut out = [0u16; MAX_VALUES];
    parse(ask, frame, &mut out).map(|n| out[..n].to_vec())
}

// The live frames. Each is the whole answer as it came off the wire.

/// PTA9B01, address 1: temperature and resistance (registers 0x0000-0x0001).
const PTA_TWO: &str = "01 03 04 FF 15 03 89 1A B5";
/// PTA9B01, address 1: all seven of its registers.
const PTA_ALL: &str = "01 03 0E FF 15 03 89 00 01 00 03 00 0A 00 00 00 00 07 B4";
/// NT18B07, address 2: its seven channels, four sensors and three empty sockets.
const NT_SEVEN: &str = "02 03 0E 00 CF 00 CF 00 CF 00 CF F5 55 F5 55 F5 55 54 1F";
/// NT18B07 answering from address 1, before the two were told apart: its address register says 1.
const NT_ADDRESS: &str = "01 03 02 00 01 79 84";
/// The answer to a write: the request said back, word for word.
const WRITTEN: &str = "01 06 00 FE 00 02 69 FB";
/// PTA9B01 throwing a frame out by itself while another address was being asked.
const AUTOSENT: &str = "01 03 02 FF 15 38 7B";
/// An answer whose first byte the converter ate: 01 came back as 07, the CRC is the right frame's.
const FIRST_BYTE_EATEN: &str = "07 03 02 00 CE 39 D0";

// --- the request: ours against theirs ---

#[test]
fn the_request_to_read_is_the_frame_rmodbus_builds() {
    // rmodbus, src/client.rs: generate_get_holdings(0x0000, 0x0002) of slave 1 - and the same frame went to the
    // PTA9B01 on the bench, which answered PTA_TWO to it
    assert_eq!(Ask::read(1, 0x0000, 2).frame().to_vec(), hex("01 03 00 00 00 02 C4 0B"));
}

#[test]
fn the_request_to_write_is_the_frame_modbus_core_builds() {
    // modbus-core, src/codec/rtu/client.rs: WriteSingleRegister(0x2222, 0xABCD) of slave 0x12
    assert_eq!(Ask::write(0x12, 0x2222, 0xABCD).frame().to_vec(), hex("12 06 22 22 AB CD 9F BE"));
}

#[test]
fn the_requests_of_the_burner_are_the_ones_the_modules_answered() {
    assert_eq!(Ask::read(2, 0x0000, 7).frame().to_vec(), hex("02 03 00 00 00 07 04 3B"));
    assert_eq!(Ask::write(1, 0x00FE, 2).frame().to_vec(), hex("01 06 00 FE 00 02 69 FB"));
    assert_eq!(Ask::write(1, 0x0006, 0).frame().to_vec(), hex("01 06 00 06 00 00 69 CB"));
}

#[test]
fn an_answer_is_as_long_as_the_asking_says() {
    assert_eq!(Ask::read(1, 0x0000, 2).reply_len(), hex(PTA_TWO).len());
    assert_eq!(Ask::read(2, 0x0000, 7).reply_len(), hex(NT_SEVEN).len());
    assert_eq!(Ask::write(1, 0x00FE, 2).reply_len(), hex(WRITTEN).len());
}

// --- the answers the modules gave ---

#[test]
fn the_pta9b01_answers_its_temperature_and_its_resistance() {
    // -23.5 C and 90.5 ohm: a thermocouple was on its terminals, and it measures resistance
    assert_eq!(taken(&Ask::read(1, 0x0000, 2), &hex(PTA_TWO)), Ok(vec![0xFF15, 0x0389]));
}

#[test]
fn the_pta9b01_answers_all_seven_of_its_registers() {
    let all = taken(&Ask::read(1, 0x0000, 7), &hex(PTA_ALL)).unwrap();
    assert_eq!(all.len(), 7);
    assert_eq!(all[2], 1); // its address on the bus
    assert_eq!(all[3], 3); // the code for 9600
    assert_eq!(all[6], 0); // autosend, turned off from the console
}

#[test]
fn the_nt18b07_answers_seven_channels_at_once() {
    // four sensors at 20.7 C and three empty sockets; what 0xF555 means is the sensor map's job, not ours
    let ask = Ask::read(2, 0x0000, 7);
    assert_eq!(
        taken(&ask, &hex(NT_SEVEN)),
        Ok(vec![0x00CF, 0x00CF, 0x00CF, 0x00CF, 0xF555, 0xF555, 0xF555])
    );
}

#[test]
fn a_write_is_answered_by_its_own_echo() {
    // the frame that moved the NT18B07 off address 1 - and the answer to it is the request word for word, which is
    // why a receiver that drops what looks like its own frame would lose this answer
    assert_eq!(taken(&Ask::write(1, 0x00FE, 2), &hex(WRITTEN)), Ok(vec![2]));
}

// --- what the bus does besides answering ---

#[test]
fn a_frame_from_another_slave_is_refused_and_not_called_broken() {
    // live: the PTA9B01 had autosend on and threw this out while address 0x12 was being asked
    let ask = Ask::read(0x12, 0x0000, 1);
    assert_eq!(taken(&ask, &hex(AUTOSENT)), Err(Refused::Foreign { asked: 0x12, answered: 1 }));
}

#[test]
fn a_frame_from_another_slave_is_foreign_whatever_its_length() {
    // Where a frame ends is said by the frame - the byte count it carries - not by the asking that was sent. A
    // foreign answer is almost never as long as ours: judge it by our length and the address is never reached.
    //
    // Shorter: seven channels awaited (19 bytes), and the autosent frame of the PTA9B01 is seven bytes long.
    assert_eq!(
        taken(&Ask::read(2, 0x0000, 7), &hex(AUTOSENT)),
        Err(Refused::Foreign { asked: 2, answered: 1 })
    );
    // Longer: two registers awaited (9 bytes), and the NT18B07 answers its seven channels, 19 bytes.
    assert_eq!(
        taken(&Ask::read(1, 0x0000, 2), &hex(NT_SEVEN)),
        Err(Refused::Foreign { asked: 1, answered: 2 })
    );
    // And the NT18B07 answering from address 1 while address 2 is asked: seven bytes against nine awaited.
    assert_eq!(
        taken(&Ask::read(2, 0x00FE, 1), &hex(NT_ADDRESS)),
        Err(Refused::Foreign { asked: 2, answered: 1 })
    );
}

#[test]
fn an_answer_with_its_first_byte_eaten_is_refused() {
    // live: the converter still held the line and 01 came back as 07. The CRC travels with the frame and is
    // computed over the whole of it, the address byte included, so a byte lost on the wire is caught here as a
    // broken checksum - not as a foreign slave. What tells them apart is that a foreign frame's CRC holds.
    let ask = Ask::read(1, 0x0000, 1);
    let frame = hex(FIRST_BYTE_EATEN);
    assert_eq!(
        taken(&ask, &frame),
        Err(Refused::Crc { carried: 0xD039, computed: crc16(&frame[..5]) })
    );
    // and the frame it was before the byte was eaten is taken without a word
    assert_eq!(taken(&ask, &hex("01 03 02 00 CE 39 D0")), Ok(vec![0x00CE]));
}

#[test]
fn a_frame_with_a_broken_crc_is_refused() {
    let mut frame = hex(PTA_TWO);
    frame[4] ^= 0x01; // one bit of the temperature, the CRC left as it was
    assert_eq!(
        taken(&Ask::read(1, 0x0000, 2), &frame),
        Err(Refused::Crc { carried: 0xB51A, computed: crc16(&frame[..7]) })
    );
}

#[test]
fn a_frame_shorter_than_a_frame_is_refused() {
    assert_eq!(taken(&Ask::read(1, 0x0000, 2), &hex(PTA_TWO)[..3]), Err(Refused::Short(3)));
}

#[test]
fn an_answer_cut_short_is_refused_by_its_length() {
    let frame = hex(PTA_TWO);
    assert_eq!(
        taken(&Ask::read(1, 0x0000, 2), &frame[..8]),
        Err(Refused::Length { awaited: 9, came: 8 })
    );
}

#[test]
fn a_slave_that_refuses_names_its_code() {
    // neither module ever refused on the bench, so this frame is built, not taken: address 1, READ with the refusal
    // bit, and 0x02 - the code for an address the module has not
    assert_eq!(taken(&Ask::read(1, 0x0000, 2), &sealed(&hex("01 83 02"))), Err(Refused::Exception(0x02)));
}

#[test]
fn an_answer_to_another_function_is_refused() {
    let mut frame = hex(PTA_TWO);
    frame[1] = 0x04; // input registers instead of holding ones
    let frame = sealed(&frame[..frame.len() - 2]);
    assert_eq!(taken(&Ask::read(1, 0x0000, 2), &frame), Err(Refused::Function { asked: 3, answered: 4 }));
}

#[test]
fn a_byte_count_that_is_not_the_bytes_carried_is_refused() {
    let mut frame = hex(PTA_TWO);
    frame[2] = 0x06; // says six bytes of data and carries four
    let frame = sealed(&frame[..frame.len() - 2]);
    assert_eq!(taken(&Ask::read(1, 0x0000, 2), &frame), Err(Refused::Count { said: 6, carried: 4 }));
}

#[test]
fn an_echo_of_another_value_is_refused() {
    let mut frame = hex(WRITTEN);
    frame[5] = 0x03; // asked to write 2 into 0x00FE, the slave says it wrote 3
    let frame = sealed(&frame[..frame.len() - 2]);
    assert_eq!(taken(&Ask::write(1, 0x00FE, 2), &frame), Err(Refused::Echo { reg: 0x00FE, value: 3 }));
}

// --- the receiver: bytes as the UART hands them over ---

/// One asking: the frame to send, the bytes in the pieces given, and what it came to.
fn asking(rx: &mut Receiver, ask: Ask, pieces: &[&[u8]]) -> Outcome {
    rx.ask(ask);
    for piece in pieces {
        rx.feed(piece);
    }
    rx.outcome()
}

#[test]
fn the_receiver_takes_an_answer_in_pieces() {
    let frame = hex(NT_SEVEN);
    let mut rx = Receiver::default();
    let out = asking(&mut rx, Ask::read(2, 0x0000, 7), &[&frame[..1], &frame[1..12], &frame[12..]]);
    assert_eq!(out, Outcome::Accepted);
    assert_eq!(rx.values(), [0x00CF, 0x00CF, 0x00CF, 0x00CF, 0xF555, 0xF555, 0xF555]);
}

#[test]
fn the_receiver_takes_the_answer_to_a_write() {
    let mut rx = Receiver::default();
    assert_eq!(asking(&mut rx, Ask::write(1, 0x00FE, 2), &[&hex(WRITTEN)]), Outcome::Accepted);
    assert_eq!(rx.values(), [2]);
}

#[test]
fn the_receiver_does_not_wait_out_an_answer_that_is_a_refusal() {
    // the asking awaits nine bytes and a refusal is five: waiting for the four that never come would be reported as
    // silence, and the code the slave named would be lost
    let mut rx = Receiver::default();
    let refusal = sealed(&hex("01 83 02"));
    assert_eq!(asking(&mut rx, Ask::read(1, 0x0000, 2), &[&refusal]), Outcome::Exception(0x02));
    assert!(rx.values().is_empty());
}

#[test]
fn a_silent_address_is_missed() {
    // live: address 9, where nothing sits - the board sent 09 03 00 00 00 01 85 42 and nothing came back
    let mut rx = Receiver::default();
    assert_eq!(asking(&mut rx, Ask::read(9, 0x0000, 1), &[]), Outcome::Missed);
    assert_eq!(asking(&mut rx, Ask::read(1, 0x0000, 2), &[&hex(PTA_TWO)[..3]]), Outcome::Missed);
}

#[test]
fn a_broken_answer_is_broken_and_a_foreign_one_is_foreign() {
    let mut rx = Receiver::default();
    let ask = Ask::read(1, 0x0000, 2);
    let mut broken = hex(PTA_TWO);
    broken[5] ^= 0x10;
    assert_eq!(asking(&mut rx, ask, &[&broken]), Outcome::Broken);
    // the autosent frame of the PTA9B01, landing in an asking addressed elsewhere
    assert_eq!(asking(&mut rx, Ask::read(0x12, 0x0000, 1), &[&hex(AUTOSENT)]), Outcome::Foreign);
}

#[test]
fn the_receiver_calls_a_foreign_frame_foreign_whatever_its_length() {
    // the defect this pair guards: the receiver used to wait out ask.reply_len() bytes, so a foreign frame of
    // another length never reached the verdict at all and the asking timed out as silence - and silence is what we
    // read as a broken sensor
    let mut rx = Receiver::default();
    assert_eq!(asking(&mut rx, Ask::read(2, 0x0000, 7), &[&hex(AUTOSENT)]), Outcome::Foreign);
    assert_eq!(asking(&mut rx, Ask::read(1, 0x0000, 2), &[&hex(NT_SEVEN)]), Outcome::Foreign);
    // in pieces, the way the UART hands them over
    let frame = hex(NT_SEVEN);
    assert_eq!(
        asking(&mut rx, Ask::read(1, 0x0000, 2), &[&frame[..4], &frame[4..]]),
        Outcome::Foreign
    );
}

#[test]
fn a_count_byte_bigger_than_anything_that_can_come_is_broken_at_once() {
    // The byte count is a CLAIM made by somebody else's frame, not a fact. 0xFF claims 260 bytes where the receiver
    // can hold MAX_REPLY; waiting for them means waiting out the whole timeout and calling it silence - the very
    // class we had just fixed. It has to be refused the moment the claim is read.
    let ask = Ask::read(1, 0x0000, 2);
    let claimed = sealed(&hex("01 03 FF 00 CE"));

    // whole
    let mut rx = Receiver::default();
    assert_eq!(asking(&mut rx, ask, &[&claimed]), Outcome::Broken);

    // and byte by byte: the verdict falls on the third byte, not on the last one that fits
    let mut rx = Receiver::default();
    rx.ask(ask);
    for (n, byte) in claimed.iter().enumerate() {
        let whole = rx.feed(&[*byte]);
        if n == 2 {
            assert!(whole, "the claim of 260 bytes was read and not acted on: {n} bytes in");
            assert_eq!(rx.outcome(), Outcome::Broken);
        }
    }
    assert_eq!(rx.outcome(), Outcome::Broken);
    assert!(rx.values().is_empty());
}

#[test]
fn more_registers_than_the_room_given_are_not_written_past_it() {
    // The other place where something from outside measures how much we write: ask.count. A frame long enough to
    // carry a hundred registers must not be taken apart into room for eight - and what comes back must say how
    // many were really written, or the caller slices past the end.
    let ask = Ask::read(1, 0x0000, 100);
    let mut body = hex("01 03 C8");
    body.extend(std::iter::repeat_n(0x00, 200));
    let frame = sealed(&body);
    let mut out = [0u16; MAX_VALUES];
    let taken = parse(&ask, &frame, &mut out).expect("a whole frame of our own slave");
    assert!(taken <= out.len(), "said it wrote {taken} registers into room for {}", out.len());
}

#[test]
fn the_tail_of_a_late_answer_does_not_count_towards_the_next_asking() {
    let ask = Ask::read(1, 0x0000, 2);
    let frame = hex(PTA_TWO);
    let mut rx = Receiver::default();
    assert_eq!(asking(&mut rx, ask, &[&frame[..5]]), Outcome::Missed);
    // the rest of it arrives during the next asking, and four bytes are not an answer
    assert_eq!(asking(&mut rx, ask, &[&frame[5..]]), Outcome::Missed);
}

#[test]
fn what_came_after_a_whole_answer_is_left_alone() {
    let ask = Ask::read(1, 0x0000, 2);
    let mut frame = hex(PTA_TWO);
    frame.extend(hex("01 03 02 FF 15 38 7B")); // an autosent frame arriving right behind the answer
    let mut rx = Receiver::default();
    assert_eq!(asking(&mut rx, ask, &[&frame]), Outcome::Accepted);
    assert_eq!(rx.values(), [0xFF15, 0x0389]);
}

// --- the counts ---

#[test]
fn every_outcome_grows_its_own_count() {
    let mut counts = Counts::default();
    for outcome in [Outcome::Accepted, Outcome::Accepted, Outcome::Exception(2), Outcome::Broken, Outcome::Foreign] {
        counts.count(outcome);
    }
    counts.count(Outcome::Missed);
    assert_eq!(counts.now, Tally { accepted: 2, exception: 1, broken: 1, foreign: 1, missed: 1 });
    assert_eq!(counts.now.askings(), 6);
    assert_eq!(counts.last, None);
}

#[test]
fn the_window_starts_afresh_after_its_last_asking_and_keeps_the_full_one() {
    let mut counts = Counts::default();
    for _ in 0..WINDOW {
        counts.count(Outcome::Accepted);
    }
    assert_eq!(counts.now, Tally::default());
    assert_eq!(counts.last.map(|t| t.accepted), Some(WINDOW));
    counts.count(Outcome::Missed);
    assert_eq!(counts.now.missed, 1);
    assert_eq!(counts.last.map(|t| t.accepted), Some(WINDOW));
}
