//! The burner's API as a whole: the table holds together, the board's own rows come first at the addresses every board gives them, the clock's settings stand as rows of the settings card, and no thing says anything of the screen. What each module adds to the table is held in its own files here (`<module>_table.rs`, `<module>_values.rs`).

use burner_api::{json, ALL, TABLE};
use serde_json::Value;

fn served() -> Value {
    serde_json::from_str(&json("stragabases3-26.10.01.1")).unwrap()
}

#[test]
fn the_table_holds_together() {
    assert_eq!(page::dictionary::check(&ALL), Ok(()));
}

#[test]
fn the_system_card_stands_first_at_the_addresses_every_board_gives_it() {
    let served = served();
    let things = served["things"].as_array().unwrap();
    assert_eq!(things.len(), TABLE.len());
    assert_eq!(things[dictionary::system::WIFI]["name"], "wifi");
    assert_eq!(things[dictionary::system::UPTIME]["name"], "uptime");
}

#[test]
fn the_clock_settings_every_board_keeps_are_rows_of_the_settings_card_with_the_parts_the_time_says() {
    let served = served();
    let rows = served["things"].as_array().unwrap();
    let find = |name: &str, part: Option<&str>| {
        rows.iter().any(|t| t["card"] == "settings" && t["name"] == name && t.get("part_of").and_then(|p| p.as_str()) == part)
    };
    assert!(find("time", None) && find("zone", None));
    for part in ["note", "from", "gateway", "at"] {
        assert!(find(part, Some("time")), "the time setting has no part {part}");
    }
}

#[test]
fn the_dictionary_says_nothing_of_the_screen() {
    // the layer's rule: no card order beyond the list of modules, no level, no word for a person - checked as the keys a thing may carry
    let served = served();
    for t in served["things"].as_array().unwrap() {
        for key in t.as_object().unwrap().keys() {
            assert!(
                ["card", "name", "part_of", "kinds", "unit", "words", "reasons", "can", "curves", "range", "fixed", "once", "clock", "decimals", "counts", "irreversible"].contains(&key.as_str()),
                "{} carries {key}, which is not the board's to say",
                t["name"]
            );
        }
    }
}

// ---- what the stream says of a number and of a count ----

fn sent_things(served: &Value) -> Vec<&Value> {
    served["things"].as_array().unwrap().iter().filter(|t| t.get("fixed").is_none() && t["kinds"] != serde_json::json!(["button"])).collect()
}

#[test]
fn every_number_the_board_sends_says_how_many_digits_it_is_worth_and_never_more_than_two() {
    let served = served();
    let numbers: Vec<&Value> = sent_things(&served).into_iter().filter(|t| t["kinds"].as_array().unwrap().iter().any(|k| k == "number" || k == "at_least")).collect();
    assert!(numbers.len() >= 40, "{} numbers: the table of the burner has lost them", numbers.len());
    let without: Vec<String> = numbers.iter().filter(|t| t.get("decimals").is_none()).map(|t| format!("{}/{}", t["card"], t["name"])).collect();
    assert!(without.is_empty(), "numbers that say nothing of their digits: {without:?}");
    for t in &numbers {
        assert!(t["decimals"].as_u64().unwrap() <= 2, "{}/{} keeps {} digits", t["card"], t["name"], t["decimals"]);
    }
}

#[test]
fn a_reading_in_degrees_keeps_a_tenth_and_a_pressure_or_a_signal_a_whole_number() {
    // what a person turns a knob by: a degree is read in tenths, a pascal or a millivolt in ones - the rule is of the unit, whatever the row is called
    let served = served();
    for t in sent_things(&served) {
        let (Some(unit), Some(digits)) = (t.get("unit").and_then(|u| u.as_str()), t.get("decimals").and_then(|d| d.as_u64())) else { continue };
        if unit == "°C" {
            assert_eq!(digits, 1, "{}/{}", t["card"], t["name"]);
        }
        if unit == "mV" || unit == "bytes" || unit == "ms" {
            assert_eq!(digits, 0, "{}/{} in {unit}", t["card"], t["name"]);
        }
    }
}

#[test]
fn what_counts_on_by_itself_says_which_way_and_a_clock_counts_down() {
    let served = served();
    let counting: Vec<&Value> = sent_things(&served).into_iter().filter(|t| t.get("counts").is_some()).collect();
    assert!(counting.len() >= 10, "{} counts: the clocks and the time a stage lasted", counting.len());
    for t in &counting {
        // a clock whose thing may not have begun says so as an unknown with its reason; a count of seconds it is all the same
        assert!(t["kinds"] == serde_json::json!(["seconds"]) || t["kinds"] == serde_json::json!(["seconds", "unknown"]), "{}/{} counts and is no count of seconds", t["card"], t["name"]);
        assert!(t["counts"] == "up" || t["counts"] == "down", "{}/{}", t["card"], t["name"]);
    }
    for t in sent_things(&served).into_iter().filter(|t| t.get("clock").is_some()) {
        assert_eq!(t["counts"], "down", "{}/{} is a clock to something and does not count down", t["card"], t["name"]);
    }
    assert!(counting.iter().any(|t| t["counts"] == "up"), "nothing counts up: the time a stage has lasted");
}
