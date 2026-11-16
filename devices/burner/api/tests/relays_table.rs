//! The relays' entities as the new burner's API says them, held to the first burner's table (crates/dictionary, `burner`): the same four organs as switches that take on, off and a toggle,
//! the seconds a limit leaves as a part of the auger and of the igniter only, and the auger's limit as a setting with its limits and default - and, new, with no card.

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

#[test]
fn the_relays_rows_are_the_first_burners_rows() {
    let (ours, theirs) = (things_of(&new(), "relays"), things_of(&reference(), "relays"));
    assert!(!ours.is_empty(), "the new burner has no row of the relays");
    assert_eq!(ours, theirs);
}

#[test]
fn every_organ_takes_the_commands_a_switch_takes() {
    let ours = things_of(&new(), "relays");
    let organs: Vec<&Value> = ours.iter().filter(|t| t.get("part_of").is_none()).collect();
    assert_eq!(organs.iter().map(|t| t["name"].as_str().unwrap()).collect::<Vec<_>>(), ["auger", "igniter", "pump", "aux_power"]);
    for organ in organs {
        assert_eq!(organ["can"], serde_json::json!(["switch", "toggle"]), "{}", organ["name"]);
    }
}

#[test]
fn only_the_auger_and_the_igniter_say_how_long_their_limit_leaves() {
    let parts: Vec<(String, String)> = things_of(&new(), "relays")
        .iter()
        .filter_map(|t| t.get("part_of").map(|p| (p.as_str().unwrap().to_string(), t["name"].as_str().unwrap().to_string())))
        .collect();
    assert_eq!(parts, [("auger".to_string(), "off_in".to_string()), ("igniter".to_string(), "off_in".to_string())]);
}

#[test]
fn the_augers_limit_is_a_row_of_the_settings_card_and_says_its_limits_and_default_as_the_first_burner_does_and_no_card() {
    let (ours, theirs) = (things_of(&new(), "settings"), things_of(&reference(), "settings"));
    let pick = |all: &[Value]| all.iter().find(|t| t["name"] == "auger_limit" && t.get("part_of").is_none()).cloned();
    assert!(pick(&ours).is_some(), "no row auger_limit in the new burner's table");
    assert_eq!(pick(&ours), pick(&theirs));
    let one = |json: &Value| json["settings"].as_array().unwrap().iter().find(|s| s["name"] == "auger_limit").cloned();
    let (ours, mut theirs) = (one(&new()).expect("no setting auger_limit in the dictionary"), one(&reference()).unwrap());
    assert!(ours.get("card").is_none(), "auger_limit says which card of the page it stands on: {ours}");
    theirs.as_object_mut().unwrap().remove("card");
    assert_eq!(ours, theirs);
}
