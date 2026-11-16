//! The soot's entities as the burner's API says them: a card of numbers - how many degrees the flue stands over the water in the burns, how many burns say it, the kilograms burned since the cleaning, the day of it - and a button for the cleaning; the two edges of the scale as settings.
//! Nothing of how the page draws them, and no word for a person: the board sends numbers and codes.

use serde_json::{json, Value};

fn served() -> Value {
    serde_json::from_str(&burner_api::json("stragabases3-26.11.09.1")).unwrap()
}

fn card(json: &Value, name: &str) -> Vec<Value> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == name).cloned().collect()
}

fn row(rows: &[Value], name: &str) -> Value {
    rows.iter().find(|t| t["name"] == name).cloned().unwrap_or_else(|| panic!("no row {name}"))
}

#[test]
fn the_card_says_degrees_burns_kilograms_and_a_day_each_as_a_number_and_what_may_be_missing_with_its_reason() {
    let rows = card(&served(), "soot");
    let c = row(&rows, "soot_c");
    assert_eq!((c["unit"].clone(), c["kinds"].clone(), c["reasons"].clone()), (json!("°C"), json!(["number", "unknown"]), json!(["no_burns"])), "{c}");
    let burns = row(&rows, "soot_burns");
    assert_eq!(burns["kinds"], json!(["number"]));
    let kg = row(&rows, "kg_since_clean");
    assert_eq!((kg["unit"].clone(), kg["kinds"].clone()), (json!("kg"), json!(["number"])), "{kg}");
    let day = row(&rows, "cleaned_day");
    assert_eq!((day["kinds"].clone(), day["reasons"].clone()), (json!(["number", "unknown"]), json!(["none_yet"])), "{day}");
}

#[test]
fn the_cleaning_is_a_button_that_asks_the_person_first_because_it_zeroes_the_kilograms() {
    let rows = card(&served(), "soot");
    let clean = row(&rows, "clean");
    assert_eq!(clean["kinds"], json!(["button"]));
    assert_eq!(clean["can"], json!(["press"]));
    assert_eq!(clean["irreversible"], json!(true));
}

#[test]
fn the_card_is_one_of_the_cards_the_burner_lists() {
    let cards = served()["cards"].clone();
    assert!(cards.as_array().unwrap().iter().any(|c| c == "soot"), "{cards}");
}

#[test]
fn the_edges_are_settings_of_whole_degrees_with_spans_that_do_not_meet() {
    let json = served();
    let find = |name: &str| json["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).cloned().unwrap_or_else(|| panic!("no setting {name}"));
    let (clean, dirty) = (find("soot_clean_c"), find("soot_dirty_c"));
    assert_eq!((clean["default"].clone(), clean["from"].clone(), clean["to"].clone(), clean["unit"].clone()), ("36".into(), 10.into(), 50.into(), "°C".into()), "{clean}");
    assert_eq!((dirty["default"].clone(), dirty["from"].clone(), dirty["to"].clone()), ("51".into(), 51.into(), 120.into()), "{dirty}");
    let rows = card(&json, "settings");
    for name in ["soot_clean_c", "soot_dirty_c"] {
        assert_eq!(row(&rows, name)["kinds"], json!(["text"]), "{name}");
    }
}

#[test]
fn the_table_holds_together_and_the_rows_laid_on_last_move_no_address_before_them() {
    assert_eq!(page::dictionary::check(&burner_api::ALL), Ok(()));
    let all = burner_api::ALL;
    let is_soot = |e: &page::dictionary::Entry| e.card == "soot" || (e.card == page::SETTINGS && e.name.starts_with("soot_"));
    let first = all.iter().position(is_soot).expect("no soot rows");
    // after the soot's rows stand the board's two rows of the load of its cores, laid on last of all
    let cores = dictionary::system::CORES.len();
    assert!(all[first..all.len() - cores].iter().all(is_soot), "a row of another module stands after the first of the soot: its address moved");
    assert!(all[all.len() - cores..].iter().all(|e| e.card == "system" && e.name.starts_with("core_")));
}
