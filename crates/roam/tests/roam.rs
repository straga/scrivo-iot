//! When the station looks for a stronger access point and which one it takes: judged every EVERY_S, a scan
//! only when the signal is under WEAK_BELOW, a move only to the same network's other access point stronger by GAIN, and
//! no more looking after TRIES failed moves in a row until the next join. The numbers are the rows' own.

use roam::{Found, Roam, Rules};

const R: Rules = Rules { weak_below: -70, every_s: 300.0, gain: 10, tries: 3 };
const OWN: [u8; 6] = [0xdc, 0x2c, 0x6e, 0x7f, 0x39, 0x4f];
const NEAR: [u8; 6] = [0xdc, 0x2c, 0x6e, 0x7f, 0x39, 0x50];

/// SECONDS of one-second stretches at RSSI: the second a scan was asked for, from 1, or None.
fn asked(roam: &mut Roam, rssi: i8, seconds: u32) -> Option<u32> {
    (1..=seconds).find(|_| roam.second(rssi, 1.0, &R))
}

fn ap(ssid: &str, bssid: [u8; 6], channel: u8, rssi: i8) -> Found {
    Found { ssid: ssid.into(), bssid, channel, rssi }
}

#[test]
fn a_weak_signal_is_scanned_for_once_its_period_is_over_and_not_before() {
    assert_eq!(asked(&mut Roam::new(), -91, 600), Some(300), "a signal of -91 over 600 s");
}

#[test]
fn a_signal_at_or_over_the_line_is_never_scanned_for() {
    // the pair: -70 is not under -70, -64 is the burner's working signal
    for rssi in [-70, -64] {
        assert_eq!(asked(&mut Roam::new(), rssi, 1200), None, "a signal of {rssi} was scanned for");
    }
}

#[test]
fn the_period_counts_from_the_last_judgement_whatever_it_said() {
    // good for the first period, weak from then on: the next judgement is a period later, not at once
    let mut roam = Roam::new();
    assert_eq!(asked(&mut roam, -60, 300), None);
    assert_eq!(asked(&mut roam, -91, 300), Some(300), "the weak signal was not judged a period after the good one");
}

#[test]
fn the_strongest_other_access_point_of_the_network_stronger_by_the_gain_is_chosen() {
    let found = [ap("GZ103", OWN, 13, -91), ap("GZ103", NEAR, 1, -51), ap("GZ103", [1; 6], 6, -70)];
    let to = Roam::new().choose("GZ103", OWN, -91, &found, &R);
    assert_eq!(to, Some(ap("GZ103", NEAR, 1, -51)));
}

#[test]
fn an_access_point_stronger_by_less_than_the_gain_is_not_moved_to() {
    // the pair of the gain: +9 stays, +10 moves
    let roam = Roam::new();
    assert_eq!(roam.choose("GZ103", OWN, -80, &[ap("GZ103", NEAR, 1, -71)], &R), None, "+9 dB was moved to");
    assert!(roam.choose("GZ103", OWN, -80, &[ap("GZ103", NEAR, 1, -70)], &R).is_some(), "+10 dB was not moved to");
}

#[test]
fn the_own_access_point_and_another_network_are_never_chosen() {
    let found = [ap("GZ103", OWN, 13, -40), ap("TP-LINK_4542", NEAR, 1, -40)];
    assert_eq!(Roam::new().choose("GZ103", OWN, -91, &found, &R), None);
}

#[test]
fn three_failed_moves_in_a_row_stop_the_looking_until_the_next_join() {
    let mut roam = Roam::new();
    for _ in 0..3 {
        assert!(asked(&mut roam, -91, 300).is_some(), "a scan was not asked for before the third failure");
        roam.moved(false);
    }
    assert_eq!(asked(&mut roam, -91, 1200), None, "the station looked on after three failed moves");
    assert_eq!(asked(&mut Roam::new(), -91, 300), Some(300), "a new join did not look again");
}

#[test]
fn a_move_that_worked_clears_the_failures() {
    // the pair: two failures, a move that worked, two more failures - still looking
    let mut roam = Roam::new();
    for ok in [false, false, true, false, false] {
        assert!(asked(&mut roam, -91, 300).is_some(), "a scan was not asked for");
        roam.moved(ok);
    }
    assert!(asked(&mut roam, -91, 300).is_some(), "a move that worked did not clear the failures");
}
