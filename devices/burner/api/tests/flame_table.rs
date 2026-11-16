//! The flame's entities as the new burner's API says them, held to the first burner's table (crates/dictionary, `burner`): the same rows, kinds,
//! units, words, reasons, curves and fixed numbers, and the two thresholds as settings with their limits - and, new, none of them with a card.

use serde_json::Value;

/// What the new burner says that the first one does not: how many digits a number is worth and which counts go on by themselves are the stream's, so a held-to-the-first-burner row leaves them out.
fn new() -> Value {
    let mut said: Value = serde_json::from_str(&burner_api::json("stragabases3-26.10.01.2")).unwrap();
    for t in said["things"].as_array_mut().unwrap() {
        let thing = t.as_object_mut().unwrap();
        thing.remove("decimals");
        thing.remove("counts");
    }
    said
}

fn reference() -> Value {
    serde_json::from_str(&burner_api::first_burner::json("stragabases3-26.10.01.2")).unwrap()
}

/// The things of CARD, without what only the position in the table says: a thing is its card, name, part, kinds, unit, words, reasons, curves, fixed.
fn things_of(json: &Value, card: &str) -> Vec<Value> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == card).cloned().collect()
}

#[test]
fn the_flames_rows_are_the_first_burners_rows() {
    let (ours, theirs) = (things_of(&new(), "flame"), things_of(&reference(), "flame"));
    assert!(!ours.is_empty(), "the new burner has no row of the flame");
    assert_eq!(ours, theirs);
}

#[test]
fn the_two_thresholds_are_rows_of_the_settings_card_with_the_part_that_says_their_order() {
    let (ours, theirs) = (things_of(&new(), "settings"), things_of(&reference(), "settings"));
    let pick = |all: &[Value], name: &str| all.iter().find(|t| t["name"] == name).cloned();
    for name in ["flame_lit", "flame_steady"] {
        assert!(pick(&ours, name).is_some(), "no setting {name}");
        assert_eq!(pick(&ours, name), pick(&theirs, name), "{name}");
    }
    let order = |all: &[Value]| all.iter().find(|t| t["name"] == "order" && t["part_of"] == "flame_steady").cloned();
    assert!(order(&ours).is_some(), "the order of the thresholds is not a part of flame_steady");
    assert_eq!(order(&ours), order(&theirs));
}

#[test]
fn the_thresholds_say_their_limits_and_defaults_as_the_first_burner_does_and_no_card() {
    let (ours, theirs) = (new(), reference());
    for name in ["flame_lit", "flame_steady"] {
        let one = |json: &Value| json["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).cloned();
        let (mut ours, mut theirs) = (one(&ours).unwrap_or_else(|| panic!("no setting {name} in the dictionary")), one(&theirs).unwrap());
        assert!(ours.get("card").is_none(), "{name} says which card of the page it stands on: {ours}");
        theirs.as_object_mut().unwrap().remove("card");
        ours.as_object_mut().unwrap().remove("card");
        assert_eq!(ours, theirs, "{name}");
    }
}

#[test]
fn the_modules_laid_through_are_the_cards_of_the_burner_and_nothing_else_of_it_is_there() {
    let json = new();
    assert_eq!(json["cards"], serde_json::json!(["system", "settings", "other", "thermostat", "demand", "automaton", "relays", "temperatures", "flue", "fan", "draught", "flame", "pellets", "restart", "mains", "soot"]));
    let cards: std::collections::BTreeSet<&str> = json["things"].as_array().unwrap().iter().map(|t| t["card"].as_str().unwrap()).collect();
    assert_eq!(cards, ["automaton", "demand", "draught", "fan", "flame", "flue", "mains", "pellets", "relays", "restart", "settings", "soot", "system", "temperatures", "thermostat"].into_iter().collect());
    assert!(json.get("machine").is_none());
}
