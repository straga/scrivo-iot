//! The values a stand of a device starts from (`tools/page_stand.py`): the board's own card as a joined board has it, every setting at its default, and the device's values on top -
//! written by the same functions the board writes them with, so the stand cannot say what the dictionary does not have.

use dictionary::{board, stand, system};
use page::dictionary::{Entry, Kind, Said, Value};

static TABLE: [Entry; board::BOARD.len() + 1] = page::dictionary::joined(&board::BOARD, &[Entry::row("relays", "relay1", &[Kind::On])]);

fn state(put: &dyn Fn(&mut dyn FnMut(usize, Said<'_>))) -> serde_json::Value {
    serde_json::from_str(&stand::state_json(&TABLE, &settings::ENGINE, "probe-1+abcd", put)).unwrap()
}

fn at(values: &serde_json::Value, address: usize) -> Option<serde_json::Value> {
    values["values"].as_array().unwrap().iter().find(|v| v[0] == address).map(|v| v[1].clone())
}

#[test]
fn a_stand_starts_as_a_board_that_has_joined_its_network() {
    let state = state(&|_| {});
    assert_eq!(state["contract"], 10);
    assert_eq!(state["dictionary"], "probe-1+abcd");
    assert_eq!(at(&state, system::WIFI), Some(serde_json::json!("joined")));
    assert!(at(&state, system::WIFI_IP).unwrap()["text"].is_string());
}

#[test]
fn every_setting_of_the_table_stands_at_its_default_as_a_text() {
    let state = state(&|_| {});
    let time = page::dictionary::address(&TABLE, page::SETTINGS, "time").unwrap();
    assert_eq!(at(&state, time), Some(serde_json::json!({"text": "auto"})));
    let zone = page::dictionary::address(&TABLE, page::SETTINGS, "zone").unwrap();
    assert_eq!(at(&state, zone), Some(serde_json::json!({"text": "gmt+2"})));
}

#[test]
fn the_device_s_own_values_stand_where_it_puts_them() {
    let relay = page::dictionary::address(&TABLE, "relays", "relay1").unwrap();
    let state = state(&|put| put(relay, Said::Value(Value::On(true))));
    assert_eq!(at(&state, relay), Some(serde_json::json!(true)));
}

#[test]
fn nothing_stands_at_an_address_the_table_does_not_have_or_that_carries_no_value() {
    let state = state(&|put| put(TABLE.len() + 5, Said::Value(Value::On(true))));
    let values = state["values"].as_array().unwrap();
    assert!(values.iter().all(|v| v[0].as_u64().unwrap() < TABLE.len() as u64), "an address beyond the table was written down");
}
