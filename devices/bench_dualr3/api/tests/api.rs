//! The Sonoff Dual R3's dictionary as a client reads it: the rows the page of this board always had - two relays, two wall keys, the meter's voltage, current and power of each output,
//! the packets it counted, the energy of today, the month and in all - now as numbers and codes, and the energy's reset as the one command that cannot be taken back. Expected from what
//! the board's page showed a person (the state of the first Dual R3 firmware, `workspace/11.3/before/dual_state_before.json`) and from the contract, not from the code that makes the table.

use bench_dualr3_api::{json, json_for, ALL, TABLE};
use serde_json::Value;

const KIT: &str = "sonoffdualr3-26.10.08.1";

fn served() -> Value {
    serde_json::from_str(&json(KIT)).unwrap()
}

fn things_of(card: &str) -> Vec<Value> {
    served()["things"].as_array().unwrap().iter().filter(|t| t["card"] == card).cloned().collect()
}

fn rows_of(card: &str) -> Vec<String> {
    things_of(card).iter().filter(|t| t.get("part_of").is_none()).map(|t| t["name"].as_str().unwrap().to_string()).collect()
}

fn parts_of(card: &str, row: &str) -> Vec<Value> {
    things_of(card).iter().filter(|t| t["part_of"] == row).cloned().collect()
}

#[test]
fn the_table_holds_together_and_is_the_table_the_dictionary_serves() {
    assert_eq!(page::dictionary::check(&ALL), Ok(()));
    assert_eq!(served()["things"].as_array().unwrap().len(), TABLE.len());
}

#[test]
fn the_system_card_stands_first_at_the_addresses_every_board_gives_it() {
    let served = served();
    let things = served["things"].as_array().unwrap();
    assert_eq!(things[dictionary::system::WIFI]["name"], "wifi");
    assert_eq!(things[dictionary::system::UPTIME]["name"], "uptime");
}

#[test]
fn the_dictionary_names_the_device_and_the_board_and_keeps_the_contract_of_the_burner() {
    let served = served();
    assert_eq!(served["device"], "bench_dualr3");
    assert_eq!(served["board"], "SONOFF_DUAL_R3");
    assert_eq!(served["contract"], 9);
    assert_eq!(served["cards"], serde_json::json!(["system", "settings", "other", "relays", "switches", "meter", "energy"]));
}

#[test]
fn the_two_relays_take_on_off_and_a_toggle() {
    assert_eq!(rows_of("relays"), ["relay1", "relay2"]);
    for relay in things_of("relays") {
        assert_eq!(relay["kinds"], serde_json::json!(["on"]));
        assert_eq!(relay["can"], serde_json::json!(["switch", "toggle"]), "{}", relay["name"]);
    }
}

#[test]
fn the_two_wall_keys_are_read_and_never_commanded_and_each_counts_its_toggles() {
    assert_eq!(rows_of("switches"), ["switch1", "switch2"]);
    for key in things_of("switches").iter().filter(|t| t.get("part_of").is_none()) {
        assert!(key.get("can").is_none(), "a key on the wall is commanded by a hand: {key}");
        let parts = parts_of("switches", key["name"].as_str().unwrap());
        assert_eq!(parts.iter().map(|p| p["name"].as_str().unwrap()).collect::<Vec<_>>(), ["toggles"]);
        assert_eq!(parts[0]["unit"], "toggles");
    }
}

#[test]
fn the_meter_says_the_voltage_and_for_each_output_the_power_with_its_current_apart() {
    assert_eq!(rows_of("meter"), ["voltage", "l1", "l2", "packets", "last"]);
    let meter = things_of("meter");
    let one = |name: &str| meter.iter().find(|t| t["name"] == name && t.get("part_of").is_none()).unwrap().clone();
    assert_eq!(one("voltage")["unit"], "V");
    for output in ["l1", "l2"] {
        assert_eq!(one(output)["unit"], "W", "{output}");
        let amps = parts_of("meter", output);
        assert_eq!((amps.len(), amps[0]["name"].as_str(), amps[0]["unit"].as_str()), (1, Some("amps"), Some("A")), "{output}");
    }
}

#[test]
fn a_reading_the_meter_did_not_give_has_a_reason_to_say() {
    let meter = things_of("meter");
    for name in ["voltage", "l1", "l2"] {
        let row = meter.iter().find(|t| t["name"] == name && t.get("part_of").is_none()).unwrap();
        assert!(row["kinds"].as_array().unwrap().contains(&Value::from("unknown")), "{name} cannot say it has no number");
        assert_eq!(row["reasons"], serde_json::json!(["not_read"]), "{name}");
    }
}

#[test]
fn the_packets_are_counted_over_a_window_of_the_size_the_meter_counts_in_and_two_windows_are_told() {
    let packets = parts_of("meter", "packets");
    assert_eq!(packets.iter().map(|p| p["name"].as_str().unwrap()).collect::<Vec<_>>(), ["accepted", "rejected", "missed", "window"]);
    let window = packets.iter().find(|p| p["name"] == "window").unwrap();
    assert_eq!(window["fixed"].as_f64(), Some(f64::from(bl0939::WINDOW)), "the window the page says is the window the meter counts in");
    let last = parts_of("meter", "last");
    assert_eq!(last.iter().map(|p| p["name"].as_str().unwrap()).collect::<Vec<_>>(), ["rejected", "missed", "window"]);
    let last_window = last.iter().find(|p| p["name"] == "window").unwrap();
    assert_eq!(last_window["fixed"].as_f64(), Some(f64::from(bl0939::WINDOW)), "the last window is told against the same size, by the row itself");
    let rows = things_of("meter");
    let last_row = rows.iter().find(|t| t["name"] == "last" && t.get("part_of").is_none()).unwrap();
    assert_eq!(last_row["reasons"], serde_json::json!(["no_window"]));
}

#[test]
fn the_energy_of_today_the_month_and_in_all_is_a_sum_in_kilowatt_hours_with_each_output_apart() {
    assert_eq!(rows_of("energy"), ["today", "month", "total", "reset"]);
    for row in ["today", "month", "total"] {
        let energy = things_of("energy");
        let one = energy.iter().find(|t| t["name"] == row && t.get("part_of").is_none()).unwrap();
        assert_eq!(one["unit"], "kWh", "{row}");
        // today and the month have a second reason: the count is not sure of its day; the total never has
        let reasons = if row == "total" { serde_json::json!(["not_counting"]) } else { serde_json::json!(["not_counting", "day_unsure"]) };
        assert_eq!(one["reasons"], reasons, "{row}");
        let parts = parts_of("energy", row);
        assert_eq!(parts.iter().map(|p| p["name"].as_str().unwrap()).collect::<Vec<_>>(), ["l1", "l2"], "{row}");
        assert!(parts.iter().all(|p| p["unit"] == "kWh"));
    }
}

#[test]
fn the_energys_reset_is_a_press_a_person_is_asked_about_first() {
    let energy = things_of("energy");
    let reset = energy.iter().find(|t| t["name"] == "reset").unwrap();
    assert_eq!(reset["kinds"], serde_json::json!(["button"]));
    assert_eq!(reset["can"], serde_json::json!(["press"]));
    assert_eq!(reset["irreversible"], true);
    assert!(energy.iter().filter(|t| t["name"] != "reset").all(|t| t.get("irreversible").is_none()), "only the reset cannot be taken back");
}

#[test]
fn the_history_of_the_energy_is_set_by_two_settings_of_the_board() {
    let served = served();
    let rows: Vec<&str> = served["things"].as_array().unwrap().iter().filter(|t| t["card"] == "settings" && t.get("part_of").is_none()).map(|t| t["name"].as_str().unwrap()).collect();
    for name in ["time", "zone", "rows", "keep", "net_point_tries", "net_reset_laps", "net_reboot_laps"] {
        assert!(rows.contains(&name), "no row {name} in the settings card: {rows:?}");
    }
    let settings: Vec<&str> = served["settings"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    for name in ["time", "zone", "rows", "keep", "net_point_tries", "net_reset_laps", "net_reboot_laps"] {
        assert!(settings.contains(&name), "the dictionary does not say what {name} takes: {settings:?}");
    }
}

#[test]
fn the_dictionary_carries_no_screen_and_no_phrase() {
    for thing in served()["things"].as_array().unwrap() {
        for key in thing.as_object().unwrap().keys() {
            assert!(["card", "name", "part_of", "kinds", "unit", "words", "reasons", "can", "curves", "fixed", "once", "clock", "decimals", "counts", "range", "irreversible"].contains(&key.as_str()), "{key} in {thing}");
        }
    }
}

#[test]
fn a_thing_the_board_has_not_is_not_in_the_dictionary() {
    let all: Vec<String> = served()["things"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
    for name in ["relay3", "auger", "flue_temp"] {
        assert!(!all.contains(&name.to_string()), "{name}");
    }
}

#[test]
fn the_months_kept_say_how_much_room_they_take_in_kilobytes_and_whether_it_fits_the_board() {
    let parts = parts_of("settings", "keep");
    assert_eq!(parts.iter().map(|p| p["name"].as_str().unwrap()).collect::<Vec<_>>(), ["note", "month_kb", "all_kb", "fs_kb"]);
    assert_eq!(parts[0]["words"], serde_json::json!(["fits", "over"]));
    for part in &parts[1..] {
        assert_eq!(part["unit"], "KB", "{}", part["name"]);
    }
    assert!(parts_of("settings", "rows").is_empty(), "the minutes between rows say nothing of themselves");
}


#[test]
fn the_dual_is_named_as_the_dual_and_the_stand_as_the_stand() {
    // the Dual's own build says its own names: nothing in what its page reads changes
    assert_eq!(json_for(KIT, "bench_dualr3", "SONOFF_DUAL_R3"), json(KIT));
    let own = served();
    assert_eq!((own["device"].as_str(), own["board"].as_str()), (Some("bench_dualr3"), Some("SONOFF_DUAL_R3")));
    // the same modules on the bench S3 are called what the stand is called, on the board it stands on
    let stand: Value = serde_json::from_str(&json_for(KIT, "bench_dualr3s3", "STRAGA_BASE_S3")).unwrap();
    assert_eq!((stand["device"].as_str(), stand["board"].as_str()), (Some("bench_dualr3s3"), Some("STRAGA_BASE_S3")));
    // and nothing else of the dictionary differs: the rows, the cards, the settings
    for key in ["things", "cards", "settings", "contract"] {
        assert_eq!(stand[key], own[key], "{key} must not depend on the name");
    }
}
