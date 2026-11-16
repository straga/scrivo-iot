//! The planned restart's entities as the burner's API says them: a card of one number - how many minutes ago the board last restarted itself by plan, or why there is no number - and the hour it restarts from as a setting with its limits. Nothing of how the page draws them.

use burner_api::restart::{self, NONE_YET, NO_CLOCK};
use page::dictionary::Value;
use serde_json::Value as Json;

fn served() -> Json {
    serde_json::from_str(&burner_api::json("stragabases3-26.10.01.5")).unwrap()
}

fn card(json: &Json, name: &str) -> Vec<Json> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == name).cloned().collect()
}

#[test]
fn the_card_says_the_minutes_since_the_last_planned_restart_or_the_reason_there_are_none() {
    let rows = card(&served(), "restart");
    let last = rows.iter().find(|t| t["name"] == "last").expect("no row last");
    assert_eq!(last["kinds"], serde_json::json!(["number", "unknown"]));
    assert_eq!(last["unit"], "min");
    assert_eq!(last["reasons"], serde_json::json!([NONE_YET, NO_CLOCK]));
}

#[test]
fn the_hour_is_a_choice_of_off_and_the_hours_from_nought_to_twenty_three_starting_from_three() {
    let json = served();
    let hour = json["settings"].as_array().unwrap().iter().find(|s| s["name"] == "restart_hour").cloned().expect("no setting restart_hour");
    assert_eq!(hour["default"], "3");
    assert_eq!(hour["kind"], "choice", "{hour}");
    let values: Vec<String> = hour["choices"].as_array().expect("a choice lists its values").iter().map(|c| c["value"].as_str().unwrap().to_string()).collect();
    let expected: Vec<String> = std::iter::once("off".to_string()).chain((0..24).map(|h| h.to_string())).collect();
    assert_eq!(values, expected, "off first, then every hour of the day");
    assert!(card(&json, "settings").iter().any(|t| t["name"] == "restart_hour"), "no row of the settings card");
}

#[test]
fn the_contract_stays_at_ten_because_a_client_of_ten_reads_a_choice_it_already_knows() {
    assert_eq!(page::dictionary::CONTRACT, 10);
    assert_eq!(served()["contract"], 10);
}

#[test]
fn the_card_is_listed_and_stands_last_but_the_rows_laid_later_so_no_address_before_it_moves() {
    let json = served();
    assert!(json["cards"].as_array().unwrap().iter().any(|c| c == "restart"));
    // the mains' rows and then the soot's were laid after all of these: the table as it stood before them
    let things = &json["things"].as_array().unwrap()[..json["things"].as_array().unwrap().len() - burner_api::mains::ENTRIES.len() - burner_api::soot::ENTRIES.len() - dictionary::system::CORES.len()];
    // the pellets' yesterday was laid after it, the automaton's silent pin and its relief after that, and the term of the low pressure (a setting) last: the rows of this card kept their addresses
    assert_eq!(things[things.len() - 1]["name"], "manual_c");
    assert_eq!(things[things.len() - 2]["name"], "manual");
    assert_eq!(things[things.len() - 3]["name"], "auto_low_pa_s");
    assert_eq!(things[things.len() - 7]["name"], "silent_fronts");
    assert_eq!(things[things.len() - 8]["name"], "yesterday");
    assert_eq!(things[things.len() - 9]["name"], "last");
    assert_eq!(things[things.len() - 10]["name"], "restart_hour");
    assert_eq!(page::dictionary::check(&burner_api::ALL), Ok(()));
}

#[test]
fn a_planned_restart_is_said_as_minutes_and_none_as_its_reason() {
    let mut put = Vec::new();
    restart::put_value(Ok(90), &mut |at, v| put.push((at, v)));
    restart::put_value(Err(NONE_YET), &mut |at, v| put.push((at, v)));
    restart::put_value(Err(NO_CLOCK), &mut |at, v| put.push((at, v)));
    assert_eq!(put, [(restart::at::LAST, Value::Number(90.0)), (restart::at::LAST, Value::Unknown(NONE_YET)), (restart::at::LAST, Value::Unknown(NO_CLOCK))]);
}
