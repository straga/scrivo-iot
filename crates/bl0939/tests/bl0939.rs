//! The BL0939's packet, from packets taken off a live Sonoff Dual R3 by Tasmota (xnrg_14_bl09xx.ino, the comment of
//! Bl09XXDecode3940, "Sample from BL0939 (dual channel)") with the numbers Tasmota decoded next to them.

use bl0939::{checksum, parse, Counts, Outcome, Reading, Receiver, Refused, Tally, INIT, PACKET_LEN, WINDOW};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
}

/// Tasmota's four live packets and what it read in them: U, I A/B, P A/B.
const LIVE: [(&str, u32, [u32; 2], [i32; 2]); 4] = [
    ("55 82 03 00 00 00 00 1E 15 01 65 80 3E E5 C6 00 00 00 00 50 B1 00 00 00 00 00 00 00 F9 01 00 FE 03 00 D2", 4096101, [0, 70942], [0, 45392]),
    ("55 E6 02 00 00 00 00 37 15 01 0F 83 3E F4 C7 00 00 00 00 69 B1 00 00 00 00 01 00 00 FA 01 00 FE 03 00 7E", 4096783, [0, 70967], [0, 45417]),
    ("55 29 03 00 00 00 00 27 15 01 3A 86 3E AF C8 00 00 00 00 67 B1 00 00 00 00 01 00 00 FA 01 00 FE 03 00 62", 4097594, [0, 70951], [0, 45415]),
    ("55 04 03 00 00 00 00 D6 14 01 7D 8E 3E 25 C7 00 00 00 00 53 B1 00 00 00 00 01 00 00 F9 01 00 FE 03 00 2E", 4099709, [0, 70870], [0, 45395]),
];

fn live(n: usize) -> Vec<u8> {
    hex(LIVE[n].0)
}

/// A live packet with other bytes at AT and its checksum made right again.
fn edited(at: usize, bytes: &[u8]) -> Vec<u8> {
    let mut p = live(0);
    p[at..at + bytes.len()].copy_from_slice(bytes);
    p[PACKET_LEN - 1] = checksum(&p[..PACKET_LEN - 1]);
    p
}

#[test]
fn live_packets_give_the_numbers_tasmota_read() {
    for (n, (_, u, i, p)) in LIVE.iter().enumerate() {
        let raw = parse(&live(n)).unwrap();
        assert_eq!((raw.v_rms, [raw.ia_rms, raw.ib_rms], [raw.a_watt, raw.b_watt]), (*u, *i, *p), "packet {n}");
    }
}

#[test]
fn the_checksum_of_a_live_packet_is_the_one_it_carries() {
    for n in 0..LIVE.len() {
        let p = live(n);
        assert_eq!(checksum(&p[..PACKET_LEN - 1]), p[PACKET_LEN - 1], "packet {n}");
    }
}

#[test]
fn a_live_packet_in_units_is_mains_voltage_and_channel_b_loaded() {
    let r = Reading::from(parse(&live(0)).unwrap());
    assert!((r.volts - 238.72).abs() < 0.01, "{r:?}");
    assert_eq!(r.amps[0], 0.0);
    assert!((r.amps[1] - 0.2667).abs() < 0.0001, "{r:?}");
    assert!((r.watts[1] - 63.66).abs() < 0.01, "{r:?}");
}

#[test]
fn a_negative_power_keeps_its_sign() {
    // -21 in 24 bits, low byte first, in the power field of channel A (bytes 16-18)
    let raw = parse(&edited(16, &[0xEB, 0xFF, 0xFF])).unwrap();
    assert_eq!(raw.a_watt, -21);
    assert!(Reading::from(raw).watts[0] < 0.0);
}

#[test]
fn a_packet_with_a_broken_checksum_is_refused() {
    let mut p = live(0);
    p[10] ^= 0x01;
    assert!(matches!(parse(&p), Err(Refused::Checksum { carried: 0xD2, .. })));
}

#[test]
fn a_packet_without_the_header_is_refused() {
    let mut p = edited(0, &[0x54]);
    p[PACKET_LEN - 1] = checksum(&p[..PACKET_LEN - 1]);
    assert_eq!(parse(&p), Err(Refused::Header(0x54)));
}

#[test]
fn a_short_or_long_packet_is_refused() {
    assert_eq!(parse(&live(0)[..34]), Err(Refused::Length(34)));
    let mut long = live(0);
    long.push(0);
    assert_eq!(parse(&long), Err(Refused::Length(36)));
}

#[test]
fn the_setup_writes_carry_their_own_checksums() {
    for row in INIT {
        let sum = row[..5].iter().fold(0u8, |s, b| s.wrapping_add(*b));
        assert_eq!(sum ^ 0xFF, row[5], "{row:02X?}");
    }
}

/// One asking: ask, the bytes in the pieces given, and what it came to.
fn asking(rx: &mut Receiver, pieces: &[&[u8]]) -> Outcome {
    rx.ask();
    for piece in pieces {
        rx.feed(piece);
    }
    rx.outcome()
}

fn v_rms(o: Outcome) -> Option<u32> {
    match o {
        Outcome::Accepted(raw) => Some(raw.v_rms),
        _ => None,
    }
}

#[test]
fn the_receiver_takes_a_packet_in_pieces() {
    let mut rx = Receiver::default();
    let p = live(1);
    assert_eq!(v_rms(asking(&mut rx, &[&p[..10], &p[10..30], &p[30..]])), Some(4096783));
}

#[test]
fn the_receiver_finds_a_packet_behind_a_stray_header() {
    let mut bytes = vec![0x55]; // a stray header, then the real packet: the first window fails its checksum
    bytes.extend(live(2));
    assert_eq!(v_rms(asking(&mut Receiver::default(), &[&bytes])), Some(4097594));
}

#[test]
fn the_receiver_skips_junk_before_the_first_header() {
    let mut bytes = vec![0x00, 0x13, 0xFF];
    bytes.extend(live(3));
    assert_eq!(v_rms(asking(&mut Receiver::default(), &[&bytes])), Some(4099709));
}

#[test]
fn a_packet_with_0x55_inside_its_data_is_taken() {
    // 0x55 in the voltage field: a header's value where the data is, and the packet is still one packet
    let p = edited(10, &[0x55, 0x80, 0x3E]);
    assert_eq!(v_rms(asking(&mut Receiver::default(), &[&p])), Some(0x3E8055));
}

#[test]
fn a_cut_packet_with_0x55_inside_does_not_hide_the_whole_one_behind_it() {
    // the cut packet carries 0x55 inside its data; the resync tries it as a header, fails, and reaches the real one
    let cut = edited(10, &[0x55, 0x80, 0x3E]);
    let mut bytes = cut[..20].to_vec();
    bytes.extend(live(1));
    assert_eq!(v_rms(asking(&mut Receiver::default(), &[&bytes])), Some(4096783));
}

#[test]
fn a_short_answer_is_missed() {
    assert_eq!(asking(&mut Receiver::default(), &[&live(0)[..34]]), Outcome::Missed);
}

#[test]
fn no_answer_is_missed() {
    assert_eq!(asking(&mut Receiver::default(), &[]), Outcome::Missed);
}

#[test]
fn one_broken_whole_packet_is_rejected_and_not_missed() {
    let mut broken = live(0);
    broken[20] ^= 0x10;
    let mut counts = Counts::default();
    counts.count(&asking(&mut Receiver::default(), &[&broken]));
    assert_eq!(counts.now, Tally { accepted: 0, rejected: 1, missed: 0 });
}

#[test]
fn a_cut_answer_then_a_whole_one_is_one_missed_and_one_accepted() {
    let mut rx = Receiver::default();
    let mut counts = Counts::default();
    counts.count(&asking(&mut rx, &[&live(0)[..20]]));
    let second = asking(&mut rx, &[&live(1)]);
    assert_eq!(v_rms(second), Some(4096783));
    counts.count(&second);
    assert_eq!(counts.now, Tally { accepted: 1, rejected: 0, missed: 1 });
}

#[test]
fn a_broken_packet_then_a_whole_one_in_the_next_asking_is_one_rejected_and_one_accepted() {
    let mut rx = Receiver::default();
    let mut counts = Counts::default();
    let mut broken = live(0);
    broken[20] ^= 0x10;
    counts.count(&asking(&mut rx, &[&broken]));
    counts.count(&asking(&mut rx, &[&live(1)]));
    assert_eq!(counts.now, Tally { accepted: 1, rejected: 1, missed: 0 });
}

#[test]
fn the_window_starts_afresh_after_its_last_asking_and_keeps_the_full_one() {
    let mut counts = Counts::default();
    let accepted = Outcome::Accepted(parse(&live(0)).unwrap());
    for n in 0..WINDOW {
        counts.count(if n == 0 { &Outcome::Missed } else if n == 1 { &Outcome::Rejected } else { &accepted });
    }
    assert_eq!(counts.now, Tally::default());
    assert_eq!(counts.last, Some(Tally { accepted: WINDOW - 2, rejected: 1, missed: 1 }));
    counts.count(&Outcome::Missed);
    assert_eq!(counts.now, Tally { accepted: 0, rejected: 0, missed: 1 });
    assert_eq!(counts.last.map(|t| t.askings()), Some(WINDOW));
}

#[test]
fn a_window_not_yet_full_has_no_last_one() {
    let mut counts = Counts::default();
    for _ in 0..WINDOW - 1 {
        counts.count(&Outcome::Missed);
    }
    assert_eq!(counts.now.missed, WINDOW - 1);
    assert_eq!(counts.last, None);
}

#[test]
fn the_tail_of_a_cut_answer_is_not_finished_by_the_next_asking() {
    // the first asking gets the head of packet A, the second only its end: kept across askings, the two halves
    // would make an old packet pass for a new answer
    let mut rx = Receiver::default();
    let a = live(0);
    assert_eq!(asking(&mut rx, &[&a[..20]]), Outcome::Missed);
    assert_eq!(asking(&mut rx, &[&a[20..]]), Outcome::Missed);
}

const SECOND: std::time::Duration = std::time::Duration::from_secs(1);

#[test]
fn the_counts_read_as_askings_bad_and_lost_and_say_when_the_first_window_is_there() {
    let mut counts = Counts::default();
    assert_eq!(counts.words(SECOND), ("0/1000 · 0 bad · 0 lost".into(), "in 17 min".into()));
    counts.count(&Outcome::Rejected);
    counts.count(&Outcome::Missed);
    counts.count(&Outcome::Accepted(parse(&live(0)).unwrap()));
    assert_eq!(counts.words(SECOND).0, "3/1000 · 1 bad · 1 lost");
}

#[test]
fn the_last_window_reads_as_its_bad_and_lost_of_all() {
    let mut counts = Counts::default();
    let accepted = Outcome::Accepted(parse(&live(0)).unwrap());
    for n in 0..WINDOW {
        counts.count(if n == 0 { &Outcome::Rejected } else { &accepted });
    }
    counts.count(&Outcome::Missed);
    assert_eq!(counts.words(SECOND), ("1/1000 · 0 bad · 1 lost".into(), "1 bad · 0 lost of 1000".into()));
}

#[test]
fn the_wait_for_the_first_window_rounds_up_to_a_minute() {
    let mut counts = Counts::default();
    for _ in 0..WINDOW - 1 {
        counts.count(&Outcome::Missed);
    }
    assert_eq!(counts.words(SECOND).1, "in 1 min");
}
