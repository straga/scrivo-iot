//! The pieces of a table that every device with a clock takes from here instead of writing again: the rows of the board's clock and station, and what a setting of crates/settings says of itself to a client.
//! What a device adds on top - its cards, its words - is its own API crate's (`devices/<device>/api`).

use dictionary::{board, system};
use page::dictionary::{self as d, Command, Kind, Meta};
use serde_json::Value;

#[test]
fn the_board_table_starts_with_the_system_card_and_holds_the_clock_and_the_station_as_settings() {
    assert_eq!(d::check(&board::BOARD), Ok(()));
    let system = dictionary::system::ROWS;
    assert_eq!(&board::BOARD[..system.len()], &system[..], "the system card stands first, at the addresses every board gives it");
    let time = d::address(&board::BOARD, page::SETTINGS, "time").expect("the clock's source is a row of the settings card");
    let parts: Vec<&str> = ["note", "from", "gateway", "at"].into_iter().filter(|p| d::part(&board::BOARD, time, p).is_some()).collect();
    assert_eq!(parts, ["note", "from", "gateway", "at"]);
    assert!(d::address(&board::BOARD, page::SETTINGS, "zone").is_some());
}

#[test]
fn the_station_s_settings_are_three_rows_of_the_settings_card_for_a_device_to_lay_after_its_own() {
    let names: Vec<&str> = board::NET.iter().map(|e| e.name).collect();
    assert_eq!(names, ["net_point_tries", "net_reset_laps", "net_reboot_laps"]);
    assert!(board::NET.iter().all(|e| e.card == page::SETTINGS && e.part_of.is_none()));
}

/// The dictionary a device with SETTINGS would serve, as a client reads it.
fn served(settings: &[page::dictionary::Setting]) -> Value {
    let meta = Meta { device: "probe", board: "PROBE", kit: "probe-1", cards: &[], settings, machine: None };
    serde_json::from_str(&d::json(&board::BOARD, &meta)).unwrap()
}

fn setting_of(json: &Value, name: &str) -> Value {
    json["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).cloned().unwrap_or_else(|| panic!("no setting {name}"))
}

/// A setting of months, as a device's history keeps one: the engine has none of this kind, so the shape stands here.
const MONTHS: settings::Setting =
    settings::Setting { name: "months", label: "Months kept", default: "12", kind: settings::Kind::Months { from: 1, to: 120 }, level: settings::Level::Work, card: None };

#[test]
fn a_choice_reaches_a_client_with_its_values_and_its_default() {
    let json = served(&[dictionary::setting(&settings::ZONE, &settings::NO_RULES)]);
    let zone = setting_of(&json, "zone");
    assert_eq!(zone["kind"], "choice");
    assert_eq!(zone["default"], "gmt+2");
    let values: Vec<&str> = zone["choices"].as_array().unwrap().iter().map(|c| c["value"].as_str().unwrap()).collect();
    assert_eq!(values, ["gmt+2", "gmt+1", "gmt", "gmt+3", "utc"]);
    assert!(zone.get("card").is_none(), "where a setting stands is the page's menu, the board says only what it is");
}

#[test]
fn a_number_of_months_reaches_a_client_as_months_between_one_and_a_hundred_and_twenty() {
    let json = served(&[dictionary::setting(&MONTHS, &settings::NO_RULES)]);
    let keep = setting_of(&json, "months");
    assert_eq!((keep["kind"].as_str(), keep["from"].as_u64(), keep["to"].as_u64(), keep["default"].as_str()), (Some("months"), Some(1), Some(120), Some("12")));
}

#[test]
fn a_setting_the_device_did_not_hand_over_is_not_in_what_a_client_reads() {
    let json = served(&[dictionary::setting(&MONTHS, &settings::NO_RULES)]);
    assert!(json["settings"].as_array().unwrap().iter().all(|s| s["name"] != "zone"));
}

#[test]
fn the_settings_every_board_keeps_are_the_clock_s_and_the_station_s() {
    let names: Vec<&str> = board::board_settings().iter().map(|s| s.name).collect();
    assert_eq!(names, ["time", "zone", "net_point_tries", "net_reset_laps", "net_reboot_laps"]);
}

#[test]
fn a_table_that_ends_with_the_rows_of_the_cores_stands_them_last_and_one_that_does_not_does_not() {
    let ends = page::dictionary::joined::<{ board::BOARD.len() + system::CORES.len() }>(&board::BOARD, &system::CORES);
    assert!(system::cores_stand_last(&ends));
    assert!(!system::cores_stand_last(&board::BOARD), "a table with no rows of the cores says it has them");
    let reversed = [system::CORES[1], system::CORES[0]];
    assert!(!system::cores_stand_last(&reversed), "the rows are in their order");
    assert!(!system::cores_stand_last(&[]));
}
