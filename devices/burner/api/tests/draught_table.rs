//! The draught's entities as the new burner's API says them, held to the first burner's table (crates/dictionary, `burner`): the same pressure row with its four reasons for no number, the
//! sensor's numbers fixed in the image, the pin under it with its parts and curves, the two arms of the divider as settings - with the part that says where the sensor's own zero lands -,
//! and none of the settings saying which card of the page it stands on.

use serde_json::Value;

/// What the new burner says that the first one does not: how many digits a number is worth and which counts go on by themselves are the stream's, so a held-to-the-first-burner row leaves them out.
fn new() -> Value {
    let mut said: Value = serde_json::from_str(&burner_api::json("stragabases3-26.10.01.3")).unwrap();
    for t in said["things"].as_array_mut().unwrap() {
        let thing = t.as_object_mut().unwrap();
        thing.remove("decimals");
        thing.remove("counts");
    }
    said
}

fn reference() -> Value {
    serde_json::from_str(&burner_api::first_burner::json("stragabases3-26.10.01.3")).unwrap()
}

fn things_of(json: &Value, card: &str) -> Vec<Value> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == card).cloned().collect()
}

#[test]
fn the_draughts_rows_are_the_first_burners_rows() {
    let (ours, theirs) = (things_of(&new(), "draught"), things_of(&reference(), "draught"));
    assert!(!ours.is_empty(), "the new burner has no row of the draught");
    assert_eq!(ours, theirs);
}

#[test]
fn the_dividers_arms_are_rows_of_the_settings_card_with_the_part_that_says_where_the_zero_lands() {
    let (ours, theirs) = (things_of(&new(), "settings"), things_of(&reference(), "settings"));
    let pick = |all: &[Value], name: &str, part_of: Option<&str>| {
        all.iter().find(|t| t["name"] == name && t.get("part_of").and_then(|p| p.as_str()) == part_of).cloned()
    };
    for (name, part_of) in [("div_top", None), ("div_bottom", None), ("zero_at", Some("div_bottom"))] {
        assert!(pick(&ours, name, part_of).is_some(), "no row {name} in the new burner's table");
        assert_eq!(pick(&ours, name, part_of), pick(&theirs, name, part_of), "{name}");
    }
}

#[test]
fn the_arms_say_their_limits_and_defaults_as_the_first_burner_does_and_no_card() {
    let (ours, theirs) = (new(), reference());
    for name in ["div_top", "div_bottom"] {
        let one = |json: &Value| json["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).cloned();
        let (ours, mut theirs) = (one(&ours).unwrap_or_else(|| panic!("no setting {name} in the dictionary")), one(&theirs).unwrap());
        assert!(ours.get("card").is_none(), "{name} says which card of the page it stands on: {ours}");
        theirs.as_object_mut().unwrap().remove("card");
        assert_eq!(ours, theirs, "{name}");
    }
}
