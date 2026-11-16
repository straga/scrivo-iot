//! The fan's entities as the new burner's API says them, held to the first burner's table (crates/dictionary, `burner`): the same rows, the same sliders and commands, the same ten settings with the same limits and
//! no card. Three things are different on purpose, and each is held here as what it is now: why the gate stopped is a CODE, not a sentence; the numbers that say what turning a knob does stand under the
//! knob's own setting, not under the holding, and are worked out in crates/regulator, not on the page; and no setting names the card it stands on.

use serde_json::Value;

/// What the new burner says that the first one does not: how many digits a number is worth and which counts go on by themselves are the stream's, so a held-to-the-first-burner row leaves them out.
fn new() -> Value {
    let mut said: Value = serde_json::from_str(&burner_api::json("stragabases3-26.10.01.5")).unwrap();
    for t in said["things"].as_array_mut().unwrap() {
        let thing = t.as_object_mut().unwrap();
        thing.remove("decimals");
        thing.remove("counts");
    }
    said
}

fn reference() -> Value {
    serde_json::from_str(&burner_api::first_burner::json("stragabases3-26.10.01.5")).unwrap()
}

fn things_of(json: &Value, card: &str) -> Vec<Value> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == card).cloned().collect()
}

fn named<'a>(things: &'a [Value], name: &str, part_of: Option<&str>) -> Option<&'a Value> {
    things.iter().find(|t| t["name"] == name && t.get("part_of").and_then(|p| p.as_str()) == part_of)
}

/// The names of the holding's parts the first burner kept for the page to work the knobs' lines out of: they stand under the knobs now.
const MOVED: [&str; 4] = ["steepness", "answer_s", "answer_dip", "answer_jump"];

/// A row without the table address its slider stands at: that number is where the slider is in ONE table, and the two tables are not the same length before the fan.
fn without_slider_address(mut one: Value) -> Value {
    if let Some(range) = one.get_mut("range").and_then(|r| r.as_object_mut()) {
        range.remove("at");
    }
    one
}

#[test]
fn the_fans_rows_are_the_first_burners_rows_but_for_the_numbers_of_the_knobs_and_the_words_of_a_refusal() {
    let (ours, theirs) = (things_of(&new(), "fan"), things_of(&reference(), "fan"));
    assert!(!ours.is_empty(), "the new burner has no row of the fan");
    let skipped = |t: &&Value| !MOVED.contains(&t["name"].as_str().unwrap()) && !["gate_why", "stop_why"].contains(&t["name"].as_str().unwrap());
    let kept: Vec<Value> = theirs.iter().filter(skipped).cloned().map(without_slider_address).collect();
    let ours_kept: Vec<Value> = ours.iter().filter(|t| !["gate_why", "stop_why"].contains(&t["name"].as_str().unwrap())).cloned().map(without_slider_address).collect();
    assert_eq!(ours_kept, kept);
}

#[test]
fn a_slider_stands_at_the_part_of_its_own_row_that_says_where_it_is() {
    let json = new();
    let all = json["things"].as_array().unwrap();
    for row in ["draught", "gate"] {
        let one = all.iter().find(|t| t["card"] == "fan" && t["name"] == row && t.get("part_of").is_none()).unwrap();
        let at = one["range"]["at"].as_u64().unwrap() as usize;
        assert_eq!((all[at]["name"].as_str(), all[at]["part_of"].as_str(), all[at]["card"].as_str()), (Some("slider"), Some(row), Some("fan")), "{row}");
    }
}

#[test]
fn why_the_gate_stopped_is_a_code_of_five_not_a_sentence() {
    let ours = things_of(&new(), "fan");
    for name in ["gate_why", "stop_why"] {
        let one = named(&ours, name, Some("holding")).unwrap_or_else(|| panic!("no part {name} of the holding"));
        assert_eq!(one["kinds"], serde_json::json!(["word"]), "{name} still carries a sentence");
        assert_eq!(one["words"], serde_json::json!(["mains_unknown", "mains_wrong", "over_scale", "no_gate", "gate_refused"]), "{name}");
    }
    let theirs = things_of(&reference(), "fan");
    assert_eq!(named(&theirs, "gate_why", Some("holding")).unwrap()["kinds"], serde_json::json!(["text"]), "the first burner says it in words: this row guards what changed");
}

#[test]
fn the_knobs_numbers_stand_under_the_knobs_and_not_under_the_holding() {
    let (ours, theirs) = (things_of(&new(), "fan"), things_of(&reference(), "fan"));
    for name in MOVED {
        assert!(named(&theirs, name, Some("holding")).is_some(), "{name}: the first burner has it");
        assert!(named(&ours, name, Some("holding")).is_none(), "{name} is still a part of the holding");
    }
    let settings = things_of(&new(), "settings");
    for (row, parts) in [
        ("up_kp", &["moves", "miss"][..]),
        ("up_ki", &["moves", "miss"]),
        ("down_kp", &["moves", "miss"]),
        ("down_ki", &["moves", "miss"]),
        ("fan_step", &["from", "to", "takes"]),
        ("fan_band", &["band", "jump", "back_in", "dip"]),
        ("fan_band_share", &["band", "near", "up", "down"]),
    ] {
        for part in parts {
            assert!(named(&settings, part, Some(row)).is_some(), "no part {part} under the setting {row}");
        }
    }
    // the sizes a line is said for are the logic's, fixed in the image, and the page does not choose them
    assert_eq!(named(&settings, "miss", Some("up_ki")).unwrap()["fixed"], 10.0);
    assert_eq!(named(&settings, "near", Some("fan_band_share")).unwrap()["fixed"], 3.0);
    assert_eq!(named(&settings, "jump", Some("fan_band")).unwrap()["fixed"], 20.0);
}

#[test]
fn the_ten_settings_of_the_fan_are_the_first_burners_with_no_card_and_no_effect() {
    let find = |json: &Value, name: &str| json["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).cloned();
    for name in ["draught_pa", "fan_min", "fan_max", "fan_step", "up_kp", "up_ki", "down_kp", "down_ki", "fan_band", "fan_band_share"] {
        let mut ours = find(&new(), name).unwrap_or_else(|| panic!("no setting {name} in the new burner's dictionary"));
        let mut theirs = find(&reference(), name).unwrap();
        assert!(ours.get("card").is_none(), "{name} says which card of the page it stands on: {ours}");
        assert!(ours.get("effect").is_none(), "{name} says what its line is worked out of: the page works out nothing now");
        for key in ["card", "effect"] {
            theirs.as_object_mut().unwrap().remove(key);
            ours.as_object_mut().unwrap().remove(key);
        }
        assert_eq!(ours, theirs, "{name}");
    }
}

#[test]
fn every_row_of_the_fan_has_its_place_in_the_table_the_build_checks() {
    page::dictionary::check(&burner_api::ALL).expect("the burner's table does not hold together");
}

#[test]
fn the_fan_stands_between_the_flue_and_the_draught_in_the_cards_order() {
    let cards: Vec<String> = new()["cards"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect();
    let at = |name: &str| cards.iter().position(|c| c == name).unwrap_or_else(|| panic!("no card {name}"));
    assert!(at("flue") < at("fan") && at("fan") < at("draught"), "{cards:?}");
}
