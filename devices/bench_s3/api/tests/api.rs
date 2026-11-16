//! The bench S3's dictionary as a client reads it: what every board has - the system card, the clock and the station's settings - and the /events probe's row that says in ms when it last changed - in the table of every image, written by one built with the probe. Expected from what the bench's page showed (`bench_s3`, contract 1: a thing "probe" with a number in ms) and from the contract.

use bench_s3_api::{json, ALL, TABLE};
use serde_json::Value;

fn served() -> Value {
    serde_json::from_str(&json("stragabases3-26.10.08.1")).unwrap()
}

#[test]
fn the_table_holds_together_and_is_the_table_the_dictionary_serves() {
    assert_eq!(page::dictionary::check(&ALL), Ok(()));
    assert_eq!(served()["things"].as_array().unwrap().len(), TABLE.len());
}

#[test]
fn the_dictionary_names_the_device_and_the_board_it_runs_on() {
    let served = served();
    assert_eq!((served["device"].as_str(), served["board"].as_str(), served["contract"].as_u64()), (Some("bench_s3"), Some("STRAGA_BASE_S3"), Some(9)));
}

#[test]
fn the_system_card_stands_first_and_the_clock_and_the_station_are_settings() {
    let served = served();
    let things = served["things"].as_array().unwrap();
    assert_eq!(things[dictionary::system::WIFI]["name"], "wifi");
    let settings: Vec<&str> = served["settings"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(settings, ["time", "zone", "net_point_tries", "net_reset_laps", "net_reboot_laps"]);
}

#[test]
fn the_probe_is_one_row_that_says_in_milliseconds_when_it_last_changed_and_is_never_commanded() {
    let served = served();
    assert_eq!(served["cards"], serde_json::json!(["system", "settings", "other", "probe"]));
    let probe = served["things"].as_array().unwrap().iter().find(|t| t["card"] == "probe").expect("no row of the probe");
    assert_eq!((probe["name"].as_str(), probe["unit"].as_str(), probe["kinds"].clone()), (Some("changed_at"), Some("ms"), serde_json::json!(["number"])));
    assert!(probe.get("can").is_none(), "the probe is read, never commanded");
}

#[test]
fn what_the_probe_is_told_stands_at_the_address_its_row_has() {
    let mut written = Vec::new();
    bench_s3_api::probe::put_value(1234, &mut |at, value| written.push((at, value)));
    assert_eq!(written, [(bench_s3_api::probe::AT, page::dictionary::Value::Number(1234.0))]);
    assert_eq!(TABLE[bench_s3_api::probe::AT].name, "changed_at");
}

#[test]
fn a_thing_the_board_has_not_is_not_in_the_dictionary() {
    let all: Vec<String> = served()["things"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    for name in ["relay1", "voltage", "flue_temp"] {
        assert!(!all.contains(&name.to_string()), "{name}");
    }
}
