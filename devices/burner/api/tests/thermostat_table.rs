//! The thermostat's entities - the water layer, which the page calls the boiler - as the new burner's API says them: a card of its own with its rows - the switch, what the water said, the level and the distance, the setpoint, the two buttons - and the settings the burner is held by; and the automaton's card without what moved.
//! Expected from what the card is for and from what the first burner's table (crates/dictionary, `burner`) did not have: the first burner has no thermostat, and keeps the buttons and the four settings on its automaton's card.

use serde_json::Value;

fn new() -> Value {
    serde_json::from_str(&burner_api::json("stragabases3-26.10.01.5")).unwrap()
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
fn the_thermostat_has_a_card_of_thirteen_rows_and_nothing_else_of_its_own() {
    let rows = rows_of(&new(), "thermostat");
    // the two of the manual cycle stand last in the burner's table, after the rows that were laid later before them, and so after the buttons
    assert_eq!(names(&rows), ["switch", "switch_from", "heat", "silent_return", "rung", "left", "setpoint", "lock", "start", "stop", "light", "manual", "manual_c"]);
}

#[test]
fn the_silence_of_the_held_sensor_is_seconds_that_count_up_and_take_no_command() {
    // said beside the reason the thermostat gives for not asking for heat, as the silence of the other sensors is said on the automaton's card
    let (rows, automaton) = (rows_of(&new(), "thermostat"), rows_of(&new(), "automaton"));
    let like = named(&automaton, "silent_water");
    let one = named(&rows, "silent_return");
    assert_eq!((&one["kinds"], one.get("counts"), one.get("clock"), one.get("can")), (&like["kinds"], like.get("counts"), like.get("clock"), like.get("can")), "silent_return is not said as the silence of the water is");
}

#[test]
fn the_automatons_card_says_nothing_of_the_sensors_the_thermostat_holds_by() {
    // one place for a silence, the card of whoever decides by the sensor; the automaton is silent about the return as it decides nothing by it
    let automaton = names(&rows_of(&new(), "automaton"));
    assert!(!automaton.contains(&"silent_return".to_string()), "the automaton's card still says silent_return");
    for name in ["silent_draught", "silent_flame", "silent_flue", "silent_water"] {
        assert!(automaton.contains(&name.to_string()), "the automaton's card lost {name}");
    }
}

#[test]
fn what_is_left_of_the_wait_before_a_light_is_a_clock_that_counts_down_in_seconds() {
    // the same kind of row as the automaton's clocks, so a page can say the nearest one that runs and count on from what the board says
    let rows = rows_of(&new(), "thermostat");
    let lock = named(&rows, "lock");
    assert_eq!(lock["kinds"], serde_json::json!(["seconds"]));
    assert_eq!(lock["clock"], true);
    assert_eq!(lock["counts"], "down");
    assert!(lock.get("can").is_none(), "it takes no command");
}

#[test]
fn the_setpoint_is_degrees_and_says_why_it_has_none() {
    let rows = rows_of(&new(), "thermostat");
    let setpoint = named(&rows, "setpoint");
    assert_eq!((setpoint["kinds"].clone(), setpoint["unit"].clone(), setpoint["reasons"].clone()), (serde_json::json!(["number", "unknown"]), serde_json::json!("°C"), serde_json::json!(["none"])));
}

#[test]
fn the_rung_is_a_word_of_the_levels_and_the_left_is_degrees_and_each_says_why_it_has_none() {
    let rows = rows_of(&new(), "thermostat");
    let rung = named(&rows, "rung");
    assert_eq!(rung["kinds"], serde_json::json!(["word", "unknown"]));
    assert_eq!(rung["words"], serde_json::json!(["big", "mid", "small"]));
    assert_eq!(rung["reasons"], serde_json::json!(["none"]));
    let left = named(&rows, "left");
    assert_eq!(left["kinds"], serde_json::json!(["number", "unknown"]));
    assert_eq!(left["unit"], "°C");
    assert_eq!(left["reasons"], serde_json::json!(["none"]));
}

#[test]
fn the_switch_and_the_heat_are_words_of_the_lists_the_automaton_crate_names() {
    let rows = rows_of(&new(), "thermostat");
    assert_eq!(named(&rows, "switch")["kinds"], serde_json::json!(["word"]));
    assert_eq!(named(&rows, "switch")["words"], serde_json::json!(["unasked", "on", "off"]));
    assert_eq!(named(&rows, "heat")["kinds"], serde_json::json!(["word", "unknown"]));
    assert_eq!(named(&rows, "heat")["words"], serde_json::json!(["burn", "keep", "enough", "level", "silent_return"]));
    assert_eq!(named(&rows, "heat")["reasons"], serde_json::json!(["starting"]));
}

#[test]
fn the_three_buttons_are_pressed_and_nothing_else_is() {
    let rows = rows_of(&new(), "thermostat");
    for name in ["start", "stop", "light"] {
        let row = named(&rows, name);
        assert_eq!(row["kinds"], serde_json::json!(["button"]), "{name}");
        assert_eq!(row["can"], serde_json::json!(["press"]), "{name}");
    }
    for name in ["switch", "heat", "rung", "left", "setpoint", "lock"] {
        assert!(named(&rows, name).get("can").is_none(), "{name} takes a command");
    }
}

#[test]
fn what_moved_to_the_thermostat_is_no_longer_on_the_automatons_card() {
    let rows = rows_of(&new(), "automaton");
    for name in ["start", "stop", "wanted"] {
        assert!(!names(&rows).contains(&name.to_string()), "{name} is still on the automaton's card");
    }
    // the pair of each: what stayed with the automaton is the rest of its card
    for name in ["stage", "doing", "organs", "refused", "feed", "semi"] {
        assert!(names(&rows).contains(&name.to_string()), "{name} left the automaton's card");
    }
}

#[test]
fn each_of_the_ten_settings_is_listed_once_and_the_dictionary_says_what_each_takes() {
    let json = new();
    let settings = json["settings"].as_array().expect("the dictionary lists the settings");
    for name in [
        "auto_hold", "auto_hold_c", "auto_drop_c", "auto_step", "auto_step_mid", "auto_step_small", "auto_mid_c", "auto_small_c", "auto_lock_min", "auto_lock_c",
    ] {
        let found = settings.iter().filter(|s| s["name"] == name).count();
        assert_eq!(found, 1, "{name} is listed {found} times");
        let rows = json["things"].as_array().unwrap().iter().filter(|t| t["card"] == "settings" && t["name"] == name).count();
        assert_eq!(rows, 1, "{name} has {rows} rows in the settings card");
    }
}

#[test]
fn who_set_the_switch_is_a_word_of_a_person_or_the_memory_and_says_why_it_has_none() {
    let rows = rows_of(&new(), "thermostat");
    let from = named(&rows, "switch_from");
    assert_eq!(from["kinds"], serde_json::json!(["word", "unknown"]));
    assert_eq!(from["words"], serde_json::json!(["person", "memory"]));
    assert_eq!(from["reasons"], serde_json::json!(["none"]));
    assert!(from["can"].as_array().map_or(true, |can| can.is_empty()), "it is read, not pressed");
}
