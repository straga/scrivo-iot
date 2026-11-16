//! The mains card as the burner's API says it: what the socket the burner is plugged into measures - power, voltage, current - or why there is no number, and the two settings that say where the socket is and how often it is asked.
//! Nothing of how the page draws them. Expected from the stage: the scheme shows `227 V · 4 W`, and a socket that does not answer is `unknown` by a code, never zero.

use burner_api::mains::{self, NOT_READ, OFF, SILENT, UNREADABLE};
use page::dictionary::Value;
use serde_json::Value as Json;

fn served() -> Json {
    serde_json::from_str(&burner_api::json("stragabases3-26.10.01.5")).unwrap()
}

fn card(json: &Json, name: &str) -> Vec<Json> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == name).cloned().collect()
}

#[test]
fn the_card_says_power_voltage_and_current_in_their_units_or_the_reason_there_is_no_number() {
    let rows = card(&served(), "mains");
    for (name, unit, decimals) in [("power", "W", 0), ("voltage", "V", 0), ("current", "A", 2)] {
        let row = rows.iter().find(|t| t["name"] == name).unwrap_or_else(|| panic!("no row {name}"));
        assert_eq!(row["kinds"], serde_json::json!(["number", "unknown"]), "{name}");
        assert_eq!(row["unit"], unit, "{name}");
        assert_eq!(row["decimals"], decimals, "{name}");
        assert_eq!(row["reasons"], serde_json::json!([OFF, NOT_READ, SILENT, UNREADABLE]), "{name}");
    }
}

#[test]
fn the_reasons_are_codes_a_client_tells_in_its_own_words() {
    assert_eq!([OFF, NOT_READ, SILENT, UNREADABLE], ["off", "not_read", "silent", "unreadable"]);
}

#[test]
fn where_the_socket_is_is_a_choice_of_none_and_an_address_that_takes_the_address() {
    let json = served();
    let at = json["settings"].as_array().unwrap().iter().find(|s| s["name"] == "mains_at").cloned().expect("no setting mains_at");
    assert_eq!(at["default"], "off");
    assert_eq!(at["kind"], "choice", "{at}");
    let values: Vec<&str> = at["choices"].as_array().unwrap().iter().map(|c| c["value"].as_str().unwrap()).collect();
    assert_eq!(values, ["off", "address"]);
    assert!(card(&json, "settings").iter().any(|t| t["name"] == "mains_at"), "no row of the settings card");
}

#[test]
fn how_often_is_a_span_of_seconds_with_its_row_on_the_settings_card() {
    let json = served();
    let every = json["settings"].as_array().unwrap().iter().find(|s| s["name"] == "mains_every").cloned().expect("no setting mains_every");
    assert_eq!(every["default"], "5");
    assert!(card(&json, "settings").iter().any(|t| t["name"] == "mains_every"));
}

#[test]
fn the_contract_stays_at_ten_because_a_choice_that_takes_a_word_is_what_a_client_of_ten_already_reads() {
    assert_eq!(page::dictionary::CONTRACT, 10);
    assert_eq!(served()["contract"], 10);
}

#[test]
fn the_card_is_listed_and_stands_last_so_no_address_before_it_moves() {
    let json = served();
    assert!(json["cards"].as_array().unwrap().iter().any(|c| c == "mains"));
    // the soot's rows were laid after the mains': the table as it stood before them
    let all = json["things"].as_array().unwrap();
    let things = &all[..all.len() - burner_api::soot::ENTRIES.len() - dictionary::system::CORES.len()];
    let names: Vec<&str> = things[things.len() - 6..].iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["manual_c", "mains_at", "mains_every", "power", "voltage", "current"]);
    assert_eq!(page::dictionary::check(&burner_api::ALL), Ok(()));
}

#[test]
fn a_reading_is_said_as_three_numbers_and_a_reason_as_three_unknowns_never_as_zeros() {
    let mut put = Vec::new();
    mains::put_values(Ok((4.0, 227.0, 0.067)), &mut |at, v| put.push((at, v)));
    assert_eq!(put, [(mains::at::POWER, Value::Number(4.0)), (mains::at::VOLTAGE, Value::Number(227.0)), (mains::at::CURRENT, Value::Number(0.067))]);
    for why in [OFF, NOT_READ, SILENT, UNREADABLE] {
        let mut put = Vec::new();
        mains::put_values(Err(why), &mut |at, v| put.push((at, v)));
        assert_eq!(put, [(mains::at::POWER, Value::Unknown(why)), (mains::at::VOLTAGE, Value::Unknown(why)), (mains::at::CURRENT, Value::Unknown(why))], "{why}");
    }
}
