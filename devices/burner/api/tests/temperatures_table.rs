//! The temperatures of the water as the new burner's API says them, held to the first burner's table (crates/dictionary, `burner`): the same channels,
//! roles, curves, bus rows, kinds, units, words and reasons, and the settings of the bus - the role of each socket, the module's address, the pace
//! of the round - with the same limits and choices and, new, none of them saying which card of the page it stands on.

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

fn things_of(json: &Value, card: &str) -> Vec<Value> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == card).cloned().collect()
}

const SETTINGS: [&str; 9] =
    ["role_ntc_1", "role_ntc_2", "role_ntc_3", "role_ntc_4", "role_ntc_5", "role_ntc_6", "role_ntc_7", "ntc_address", "poll_every"];

#[test]
fn the_temperatures_rows_are_the_first_burners_rows() {
    let (ours, theirs) = (things_of(&new(), "temperatures"), things_of(&reference(), "temperatures"));
    assert!(!ours.is_empty(), "the new burner has no row of the temperatures");
    assert_eq!(ours, theirs);
}

#[test]
fn the_bus_and_the_sockets_settings_are_rows_of_the_settings_card_as_in_the_first_burner() {
    let (ours, theirs) = (things_of(&new(), "settings"), things_of(&reference(), "settings"));
    let pick = |all: &[Value], name: &str| all.iter().find(|t| t["name"] == name && t.get("part_of").is_none()).cloned();
    for name in SETTINGS {
        assert!(pick(&ours, name).is_some(), "no setting {name} in the new burner's table");
        assert_eq!(pick(&ours, name), pick(&theirs, name), "{name}");
    }
}

#[test]
fn the_settings_say_their_choices_limits_and_defaults_as_the_first_burner_does_and_no_card() {
    let (ours, theirs) = (new(), reference());
    for name in SETTINGS {
        let one = |json: &Value| json["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).cloned();
        let (mut ours, mut theirs) = (one(&ours).unwrap_or_else(|| panic!("no setting {name} in the dictionary")), one(&theirs).unwrap());
        assert!(ours.get("card").is_none(), "{name} says which card of the page it stands on: {ours}");
        theirs.as_object_mut().unwrap().remove("card");
        assert_eq!(ours, theirs, "{name}");
    }
}

#[test]
fn a_socket_can_be_given_one_of_seven_roles_and_a_name_of_ones_own() {
    let json = new();
    let role = json["settings"].as_array().unwrap().iter().find(|s| s["name"] == "role_ntc_1").cloned().expect("no setting role_ntc_1");
    let values: Vec<&str> = role["choices"].as_array().expect("the role has no choices").iter().map(|c| c["value"].as_str().unwrap()).collect();
    assert_eq!(values, ["free", "boiler", "supply", "return", "flue", "hopper", "room", "own"]);
}
