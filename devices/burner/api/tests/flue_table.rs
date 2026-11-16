//! The flue's temperature as the new burner's API says it, held to the first burner's table (crates/dictionary, `burner`): the same row, unit, reasons
//! for no number and curve.

use serde_json::Value;

fn things_of(json: &str, card: &str) -> Vec<Value> {
    let json: Value = serde_json::from_str(json).unwrap();
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == card).cloned().collect()
}

#[test]
fn the_flue_row_is_the_first_burners_row() {
    let (mut ours, theirs) = (things_of(&burner_api::json("stragabases3-26.10.01.2"), "flue"), things_of(&burner_api::first_burner::json("stragabases3-26.10.01.2"), "flue"));
    assert!(!ours.is_empty(), "the new burner has no row of the flue");
    // how many digits a number is worth and which counts go on by themselves are the stream's, which the first burner does not say
    for t in &mut ours {
        t.as_object_mut().unwrap().remove("decimals");
    }
    assert_eq!(ours, theirs);
}

#[test]
fn the_flue_gives_a_degree_and_three_reasons_for_none() {
    let ours = things_of(&burner_api::json("stragabases3-26.10.01.2"), "flue");
    let [row] = ours.as_slice() else { panic!("the flue is one row, not {}", ours.len()) };
    assert_eq!(row["unit"], "°C");
    assert_eq!(row["reasons"], serde_json::json!(["open", "silent", "not_read"]));
    assert_eq!(row["curves"], serde_json::json!(["flue"]));
}
