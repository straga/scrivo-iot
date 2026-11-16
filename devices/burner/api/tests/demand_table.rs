//! The demand's entities as the new burner's API says them: a card of its own with its rows - what the house says and why, how long the room's sensor has been silent, the profile that stands with what chose it, the target of the room - and
//! the eleven settings of who asks, the targets and the profiles by the clock. Expected from what the card is for: the thermostat's card kept the profile and the silence of the room until this stage, and keeps them no more.

use serde_json::Value;

fn new() -> Value {
    serde_json::from_str(&burner_api::json("stragabases3-26.10.05.1")).unwrap()
}

fn rows_of(json: &Value, card: &str) -> Vec<Value> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == card && t.get("part_of").is_none()).cloned().collect()
}

fn names(rows: &[Value]) -> Vec<String> {
    rows.iter().map(|t| t["name"].as_str().unwrap().to_string()).collect()
}

fn named<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter().find(|t| t["name"] == name).unwrap_or_else(|| panic!("no row {name}"))
}

#[test]
fn the_demand_has_a_card_of_six_rows_and_takes_no_command() {
    let rows = rows_of(&new(), "demand");
    assert_eq!(names(&rows), ["asked", "why", "silent_room", "profile", "source", "target"]);
    for row in &rows {
        assert!(row.get("can").is_none(), "{} takes a command: a person asks the boiler by the buttons of the thermostat", row["name"]);
    }
}

#[test]
fn what_the_house_says_is_a_word_and_a_reason_and_each_says_why_it_has_none() {
    let rows = rows_of(&new(), "demand");
    let asked = named(&rows, "asked");
    assert_eq!((asked["kinds"].clone(), asked["words"].clone(), asked["reasons"].clone()), (serde_json::json!(["word", "unknown"]), serde_json::json!(["cold", "warm"]), serde_json::json!(["none"])));
    let why = named(&rows, "why");
    assert_eq!((why["kinds"].clone(), why["words"].clone(), why["reasons"].clone()), (serde_json::json!(["word", "unknown"]), serde_json::json!(["below_target", "target_reached", "profile_off", "scheduled_on", "scheduled_off", "source_silent", "not_read", "starting"]), serde_json::json!(["none"])));
}

#[test]
fn the_silence_of_the_room_is_seconds_that_count_up_and_take_no_command() {
    // said as the silence of the other sensors is, and now on the card of whoever asks by the room
    let (rows, automaton) = (rows_of(&new(), "demand"), rows_of(&new(), "automaton"));
    let like = named(&automaton, "silent_water");
    let one = named(&rows, "silent_room");
    assert_eq!((&one["kinds"], one.get("counts"), one.get("clock"), one.get("can")), (&like["kinds"], like.get("counts"), like.get("clock"), like.get("can")));
}

#[test]
fn the_profile_that_stands_is_said_by_codes_and_numbers_and_each_says_why_it_has_none() {
    // the profile, what chose it, the number of the window and the minute of the day it ends at
    let rows = rows_of(&new(), "demand");
    let profile = named(&rows, "profile");
    assert_eq!((profile["kinds"].clone(), profile["words"].clone(), profile["reasons"].clone()), (serde_json::json!(["word", "unknown"]), serde_json::json!(["comfort", "eco", "off"]), serde_json::json!(["none"])));
    let source = named(&rows, "source");
    assert_eq!((source["kinds"].clone(), source["words"].clone(), source["reasons"].clone()), (serde_json::json!(["word", "unknown"]), serde_json::json!(["main", "window", "no_clock"]), serde_json::json!(["none"])));
    let parts: Vec<Value> = new()["things"].as_array().unwrap().iter().filter(|t| t["card"] == "demand" && t["part_of"] == "source").cloned().collect();
    assert_eq!(names(&parts), ["window", "until"]);
    let until = named(&parts, "until");
    assert_eq!((until["kinds"].clone(), until["unit"].clone(), until["reasons"].clone()), (serde_json::json!(["number", "unknown"]), serde_json::json!("min"), serde_json::json!(["none"])));
}

#[test]
fn the_target_of_the_room_is_degrees_and_says_why_it_has_none() {
    let rows = rows_of(&new(), "demand");
    let target = named(&rows, "target");
    assert_eq!((target["kinds"].clone(), target["unit"].clone(), target["reasons"].clone()), (serde_json::json!(["number", "unknown"]), serde_json::json!("°C"), serde_json::json!(["none"])));
}

#[test]
fn the_thermostat_no_longer_says_what_moved_to_the_demand() {
    let thermostat = names(&rows_of(&new(), "thermostat"));
    for name in ["profile", "source", "silent_room"] {
        assert!(!thermostat.contains(&name.to_string()), "the thermostat's card still says {name}");
    }
}

#[test]
fn each_of_the_eleven_settings_is_listed_once_and_who_asks_says_what_each_choice_brings() {
    let json = new();
    let settings = json["settings"].as_array().expect("the dictionary lists the settings");
    for name in ["auto_asked_by", "auto_room_c", "auto_room_eco_c", "auto_room_gap_c", "auto_profile", "window_1", "window_2", "window_3", "window_4", "window_5", "window_6"] {
        let found = settings.iter().filter(|s| s["name"] == name).count();
        assert_eq!(found, 1, "{name} is listed {found} times");
        let rows = json["things"].as_array().unwrap().iter().filter(|t| t["card"] == "settings" && t["name"] == name).count();
        assert_eq!(rows, 1, "{name} has {rows} rows in the settings card");
    }
    for gone in ["auto_eco_c", "auto_off_c"] {
        assert!(settings.iter().all(|s| s["name"] != gone), "{gone} is still listed: it was a setpoint of the water by profile");
    }
    let asked_by = settings.iter().find(|s| s["name"] == "auto_asked_by").unwrap();
    let choices = asked_by["choices"].as_array().or_else(|| asked_by["variants"].as_array()).expect("a choice lists its variants");
    let shows = |value: &str| -> Vec<String> {
        choices.iter().find(|c| c["value"] == value).unwrap_or_else(|| panic!("no choice {value}"))["shows"].as_array().map_or(Vec::new(), |all| all.iter().map(|s| s.as_str().unwrap().to_string()).collect())
    };
    assert!(shows("none").is_empty(), "nobody asking brings nothing");
    assert!(shows("room").contains(&"auto_room_c".to_string()) && shows("room").contains(&"auto_profile".to_string()));
    assert!(shows("schedule").contains(&"auto_profile".to_string()) && !shows("schedule").contains(&"auto_room_c".to_string()), "the schedule alone has no target of a room");
}
