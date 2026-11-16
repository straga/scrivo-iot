//! What the Dual R3's modules hand the stream, by address: the relays closed or open, the keys' positions and their toggles, the meter's reading or the reason there is none, the windows of
//! packets, the energy counted. Each is checked against what the board's page showed of the same facts: the reading of a lamp on each output, a window with losses, a count not started.

use bench_dualr3_api::{energy, meter, relays, switches, ALL};
use page::dictionary::{self as d, Value};

fn said(put: impl FnOnce(&mut dyn FnMut(usize, Value))) -> Vec<(usize, Value)> {
    let mut out = Vec::new();
    put(&mut |at, value| out.push((at, value)));
    out
}

fn at(card: &str, name: &str) -> usize {
    d::address(&ALL, card, name).unwrap_or_else(|| panic!("no {card}/{name}"))
}

fn part(card: &str, row: &str, name: &str) -> usize {
    d::part(&ALL, at(card, row), name).unwrap_or_else(|| panic!("no {card}/{row}.{name}"))
}

fn value(all: &[(usize, Value)], at: usize) -> Value {
    all.iter().find(|(a, _)| *a == at).unwrap_or_else(|| panic!("nothing said at {at}")).1
}

#[test]
fn a_closed_relay_is_on_and_an_open_one_is_off_and_nothing_else_is_said() {
    let all = said(|put| relays::put_values([true, false], put));
    assert_eq!(all, [(at("relays", "relay1"), Value::On(true)), (at("relays", "relay2"), Value::On(false))]);
}

#[test]
fn a_key_says_where_it_stands_and_how_many_times_it_toggled() {
    let all = said(|put| switches::put_values(&[switches::Key { closed: true, toggles: 3 }, switches::Key { closed: false, toggles: 0 }], put));
    assert_eq!(value(&all, at("switches", "switch1")), Value::On(true));
    assert_eq!(value(&all, part("switches", "switch1", "toggles")), Value::Number(3.0));
    assert_eq!(value(&all, at("switches", "switch2")), Value::On(false));
    assert_eq!(value(&all, part("switches", "switch2", "toggles")), Value::Number(0.0));
    assert_eq!(all.len(), 4);
}

fn packets(accepted: u32, rejected: u32, missed: u32) -> meter::Window {
    meter::Window { accepted, rejected, missed }
}

fn reading() -> meter::Now {
    meter::Now {
        reading: Some(meter::Reading { volts: 237.4, outputs: [meter::Output { amps: 0.5, watts: 110.0 }, meter::Output { amps: 0.0, watts: 0.0 }] }),
        now: packets(822, 0, 0),
        last: None,
    }
}

#[test]
fn a_reading_is_the_voltage_and_each_output_s_power_and_current() {
    let all = said(|put| meter::put_values(&reading(), put));
    assert_eq!(value(&all, at("meter", "voltage")), Value::Number(237.4));
    assert_eq!(value(&all, at("meter", "l1")), Value::Number(110.0));
    assert_eq!(value(&all, part("meter", "l1", "amps")), Value::Number(0.5));
    assert_eq!(value(&all, at("meter", "l2")), Value::Number(0.0));
    assert_eq!(value(&all, part("meter", "l2", "amps")), Value::Number(0.0));
}

#[test]
fn before_the_first_packet_there_is_no_number_and_the_reason_is_said() {
    let all = said(|put| meter::put_values(&meter::Now { reading: None, now: packets(0, 0, 4), last: None }, put));
    for (card, name) in [("meter", "voltage"), ("meter", "l1"), ("meter", "l2")] {
        assert_eq!(value(&all, at(card, name)), Value::Unknown("not_read"), "{name}");
    }
    assert_eq!(value(&all, part("meter", "l1", "amps")), Value::Unknown("not_read"));
    assert_eq!(value(&all, part("meter", "l2", "amps")), Value::Unknown("not_read"));
}

#[test]
fn a_window_without_a_loss_is_ok_and_one_with_a_bad_or_a_lost_packet_is_losing() {
    let ok = said(|put| meter::put_values(&reading(), put));
    assert_eq!(value(&ok, at("meter", "packets")), Value::Word("ok"));
    for (rejected, missed) in [(1, 0), (0, 1), (5, 7)] {
        let now = meter::Now { now: packets(800, rejected, missed), ..reading() };
        let all = said(|put| meter::put_values(&now, put));
        assert_eq!(value(&all, at("meter", "packets")), Value::Word("losing"), "{rejected} bad, {missed} lost");
        assert_eq!(value(&all, part("meter", "packets", "rejected")), Value::Number(rejected as f32));
        assert_eq!(value(&all, part("meter", "packets", "missed")), Value::Number(missed as f32));
    }
    assert_eq!(value(&ok, part("meter", "packets", "accepted")), Value::Number(822.0));
}

#[test]
fn the_last_full_window_is_not_there_until_one_has_filled_and_then_says_how_it_went() {
    let none = said(|put| meter::put_values(&reading(), put));
    assert_eq!(value(&none, at("meter", "last")), Value::Unknown("no_window"));
    let full = meter::Now { last: Some(packets(990, 3, 7)), ..reading() };
    let all = said(|put| meter::put_values(&full, put));
    assert_eq!(value(&all, at("meter", "last")), Value::Word("losing"));
    assert_eq!(value(&all, part("meter", "last", "rejected")), Value::Number(3.0));
    assert_eq!(value(&all, part("meter", "last", "missed")), Value::Number(7.0));
}

#[test]
fn the_energy_says_each_output_apart_and_the_sum_of_both_and_nothing_of_the_reset() {
    let sums = energy::Sums { today: [0.012, 0.110], month: [1.313, 0.503], total: [2.485, 2.084] };
    let all = said(|put| energy::put_values(Some(&sums), true, put));
    let number = |v: Value| match v {
        Value::Number(n) => n,
        other => panic!("{other:?}"),
    };
    assert!((number(value(&all, at("energy", "today"))) - 0.122).abs() < 1e-5);
    assert_eq!(number(value(&all, part("energy", "today", "l1"))), 0.012);
    assert_eq!(number(value(&all, part("energy", "today", "l2"))), 0.110);
    assert!((number(value(&all, at("energy", "month"))) - 1.816).abs() < 1e-5);
    assert!((number(value(&all, at("energy", "total"))) - 4.569).abs() < 1e-5);
    assert!(all.iter().all(|(a, _)| *a != at("energy", "reset")), "a button has no value");
}

#[test]
fn a_count_that_did_not_start_says_so_at_every_row_it_owes() {
    let all = said(|put| energy::put_values(None, true, put));
    for row in ["today", "month", "total"] {
        assert_eq!(value(&all, at("energy", row)), Value::Unknown("not_counting"), "{row}");
        assert_eq!(value(&all, part("energy", row, "l1")), Value::Unknown("not_counting"), "{row}.l1");
    }
}

#[test]
fn a_count_not_sure_of_its_day_says_no_number_for_today_and_the_month_and_each_output_of_them_and_the_total_as_it_is() {
    let sums = energy::Sums { today: [0.012, 0.110], month: [1.313, 0.503], total: [2.485, 2.084] };
    let all = said(|put| energy::put_values(Some(&sums), false, put));
    for row in ["today", "month"] {
        assert_eq!(value(&all, at("energy", row)), Value::Unknown("day_unsure"), "{row}");
        assert_eq!(value(&all, part("energy", row, "l1")), Value::Unknown("day_unsure"), "{row}.l1");
        assert_eq!(value(&all, part("energy", row, "l2")), Value::Unknown("day_unsure"), "{row}.l2");
    }
    assert!(matches!(value(&all, at("energy", "total")), Value::Number(_)), "the total is never hidden");
    assert!(matches!(value(&all, part("energy", "total", "l1")), Value::Number(_)));
}

#[test]
fn a_count_that_did_not_start_says_so_whether_or_not_it_is_sure_of_a_day() {
    for sure in [true, false] {
        let all = said(|put| energy::put_values(None, sure, put));
        for row in ["today", "month", "total"] {
            assert_eq!(value(&all, at("energy", row)), Value::Unknown("not_counting"), "{row} sure {sure}");
        }
    }
}

#[test]
fn every_address_the_modules_write_carries_a_value_in_the_table() {
    let mut written = Vec::new();
    written.extend(said(|put| relays::put_values([false, false], put)));
    written.extend(said(|put| switches::put_values(&[switches::Key { closed: false, toggles: 0 }; 2], put)));
    written.extend(said(|put| meter::put_values(&reading(), put)));
    written.extend(said(|put| energy::put_values(None, true, put)));
    for (address, _) in &written {
        let entry = ALL[*address];
        assert!(entry.fixed.is_none() && !entry.kinds.contains(&d::Kind::Button), "{}/{} is written and carries no value", entry.card, entry.name);
    }
    let mut unique: Vec<usize> = written.iter().map(|(a, _)| *a).collect();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), written.len(), "an address is written twice");
}

fn room(month_kb: u32, all_kb: u32, fs_kb: u32) -> bench_dualr3_api::energy::HistoryRoom {
    bench_dualr3_api::energy::HistoryRoom { month_kb, all_kb, fs_kb }
}

#[test]
fn a_history_that_fits_the_board_says_so_with_the_numbers_it_is_made_of() {
    let all = said(|put| energy::put_setting_values(&room(36, 432, 896), put));
    assert_eq!(value(&all, part("settings", "keep", "note")), Value::Word("fits"));
    assert_eq!(value(&all, part("settings", "keep", "month_kb")), Value::Number(36.0));
    assert_eq!(value(&all, part("settings", "keep", "all_kb")), Value::Number(432.0));
    assert_eq!(value(&all, part("settings", "keep", "fs_kb")), Value::Number(896.0));
    assert_eq!(all.len(), 4);
}

#[test]
fn a_history_larger_than_the_board_is_over_and_a_history_exactly_its_size_is_not() {
    let over = said(|put| energy::put_setting_values(&room(140, 1680, 896), put));
    assert_eq!(value(&over, part("settings", "keep", "note")), Value::Word("over"));
    let exactly = said(|put| energy::put_setting_values(&room(64, 896, 896), put));
    assert_eq!(value(&exactly, part("settings", "keep", "note")), Value::Word("fits"));
}
