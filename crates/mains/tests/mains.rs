//! What the burner does with its socket's reading, on the computer: the address the setting names, and the journal's line when an organ switched - the power before and the power after. The words of the line are for a person
//! reading the journal; what the tests hold is that every switch is named with its direction, that "before" and "after" are the readings on either side of it, and that a reading that is missing is said missing and never as zero.

use mains::{address, Watch};
use tasmota::{Energy, Fault};

fn load(watts: f32) -> Result<Energy, Fault> {
    Ok(Energy { watts, volts: 227.0, amps: watts / 227.0 })
}

fn names(relay: usize) -> &'static str {
    ["auger", "igniter", "pump", "fan"].get(relay).copied().unwrap_or("relay")
}

#[test]
fn the_address_the_setting_keeps_is_the_sockets_address_at_the_port_of_http() {
    assert_eq!(address("address:192.0.2.10"), Some("192.0.2.10:80".parse().unwrap()));
    assert_eq!(address("address:192.0.2.5"), Some("192.0.2.5:80".parse().unwrap()));
}

#[test]
fn no_socket_is_off_and_what_is_no_address_is_none_too() {
    for kept in ["off", "", "address:", "address:socket.local", "192.0.2.10", "address:300.1.1.1", "address:192.0.2.10:80"] {
        assert_eq!(address(kept), None, "{kept:?}");
    }
}

#[test]
fn a_round_with_no_switch_says_nothing_however_the_power_moves() {
    let mut watch = Watch::new();
    assert_eq!(watch.round(&load(3.0), &[], 0, names), None);
    assert_eq!(watch.round(&load(120.0), &[], 0, names), None);
    assert_eq!(watch.round(&Err(Fault::Silent), &[], 0, names), None);
}

#[test]
fn a_switch_is_told_with_the_power_before_it_and_the_power_after_it() {
    let mut watch = Watch::new();
    watch.round(&load(3.0), &[], 0, names);
    let line = watch.round(&load(62.0), &[(3, true)], 0, names).expect("a switch with no line");
    assert!(line.contains("fan on"), "{line}");
    assert!(line.contains("3 W before"), "{line}");
    assert!(line.contains("62 W after"), "{line}");
}

#[test]
fn the_power_before_the_next_switch_is_the_power_after_the_last() {
    let mut watch = Watch::new();
    watch.round(&load(3.0), &[], 0, names);
    watch.round(&load(62.0), &[(3, true)], 0, names);
    let line = watch.round(&load(80.0), &[(1, true)], 0, names).unwrap();
    assert!(line.contains("igniter on") && line.contains("62 W before") && line.contains("80 W after"), "{line}");
}

#[test]
fn several_switches_in_one_round_are_all_named_in_order_and_with_their_direction() {
    let mut watch = Watch::new();
    watch.round(&load(3.0), &[], 0, names);
    let line = watch.round(&load(75.0), &[(3, true), (1, true), (0, false)], 0, names).unwrap();
    let at = |word: &str| line.find(word).unwrap_or_else(|| panic!("{word} not in {line}"));
    assert!(at("fan on") < at("igniter on") && at("igniter on") < at("auger off"), "{line}");
}

#[test]
fn the_first_round_has_no_power_before_it_and_says_unknown_and_not_zero() {
    let mut watch = Watch::new();
    let line = watch.round(&load(62.0), &[(3, true)], 0, names).unwrap();
    assert!(line.contains("unknown before"), "{line}");
    assert!(!line.contains("0 W before"), "{line}");
}

#[test]
fn a_socket_that_did_not_answer_before_gives_an_unknown_before_and_not_the_reading_from_longer_ago() {
    let mut watch = Watch::new();
    watch.round(&load(3.0), &[], 0, names);
    watch.round(&Err(Fault::Silent), &[], 0, names);
    let line = watch.round(&load(62.0), &[(3, true)], 0, names).unwrap();
    assert!(line.contains("unknown before"), "{line}");
}

#[test]
fn a_socket_that_does_not_answer_after_the_switch_is_told_with_its_reason() {
    let mut watch = Watch::new();
    watch.round(&load(3.0), &[], 0, names);
    let line = watch.round(&Err(Fault::Silent), &[(3, true)], 0, names).expect("a switch while the socket is silent has no line");
    assert!(line.contains("fan on") && line.contains("3 W before") && line.contains("unknown after") && line.contains("silent"), "{line}");
    let line = watch.round(&Err(Fault::NoEnergy), &[(3, false)], 0, names).unwrap();
    assert!(line.contains("unreadable"), "{line}");
}

#[test]
fn switches_the_notes_could_not_hold_are_counted_in_the_line() {
    let mut watch = Watch::new();
    watch.round(&load(3.0), &[], 0, names);
    let line = watch.round(&load(62.0), &[(0, true)], 4, names).unwrap();
    assert!(line.contains("4 more"), "{line}");
}

#[test]
fn a_relay_the_names_do_not_know_is_still_told_by_its_number() {
    let mut watch = Watch::new();
    watch.round(&load(3.0), &[], 0, names);
    let line = watch.round(&load(62.0), &[(9, true)], 0, |_| "").unwrap();
    assert!(line.contains("relay 9 on"), "{line}");
}
