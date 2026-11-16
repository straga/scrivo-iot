//! The board's own settings - where the time comes from and the time zone - as the new burner's API says them, held to the first burner's dictionary:
//! the same two settings with the same choices, and no card: they stand on the tab Settings, not on a module's card.

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

fn setting(json: &Value, name: &str) -> Option<Value> {
    json["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).cloned()
}

#[test]
fn the_time_and_the_zone_are_settings_of_the_board_as_the_first_burner_says_them() {
    for name in ["time", "zone"] {
        let ours = setting(&new(), name).unwrap_or_else(|| panic!("no setting {name} in the new burner's dictionary"));
        assert!(ours.get("card").is_none(), "{name} says which card it stands on: {ours}");
        let mut theirs = setting(&reference(), name).unwrap();
        theirs.as_object_mut().unwrap().remove("card");
        assert_eq!(ours, theirs, "{name}");
    }
}

#[test]
fn the_time_offers_three_choices_and_the_zone_five_and_only_the_server_takes_a_word() {
    let choices = |name: &str| setting(&new(), name).unwrap_or_else(|| panic!("no setting {name}"))["choices"].as_array().cloned().unwrap_or_default();
    let time = choices("time");
    assert_eq!(time.iter().map(|c| c["value"].as_str().unwrap()).collect::<Vec<_>>(), ["auto", "internet", "server"]);
    assert_eq!(time.iter().filter(|c| c["takes"] == true).count(), 1);
    assert_eq!(choices("zone").len(), 5);
}

#[test]
fn a_setting_the_board_does_not_have_is_not_in_the_dictionary() {
    assert!(setting(&new(), "no_such_setting").is_none());
}
