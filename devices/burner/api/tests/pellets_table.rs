//! The pellets' entities as the burner's API says them: a card of four numbers - what the burner burned today, yesterday, this month and in all, in kilograms - and the rate the auger pours as a setting with its limits. Nothing of how the page draws them.
//! The first burner has no such card, so it is held to what a person who weighs a boiler's fuel would expect, not to the first burner's table.

use serde_json::Value;

fn served() -> Value {
    serde_json::from_str(&burner_api::json("stragabases3-26.10.01.5")).unwrap()
}

fn card(json: &Value, name: &str) -> Vec<Value> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == name).cloned().collect()
}

#[test]
fn the_card_says_kilograms_of_today_the_month_and_all_the_time_a_number_and_for_today_and_the_month_the_reason_there_is_none() {
    let rows = card(&served(), "pellets");
    let mut said: Vec<(String, String, Value)> = rows
        .iter()
        .filter(|t| t["name"] != "pellet_rate")
        .map(|t| (t["name"].as_str().unwrap().to_string(), t["unit"].as_str().unwrap_or("").to_string(), t["kinds"].clone()))
        .collect();
    said.sort_by(|a, b| a.0.cmp(&b.0));
    let number = serde_json::json!(["number"]);
    let of_a_day = serde_json::json!(["number", "unknown"]);
    assert_eq!(
        said,
        [
            ("month".to_string(), "kg".to_string(), of_a_day.clone()),
            ("today".to_string(), "kg".to_string(), of_a_day.clone()),
            ("total".to_string(), "kg".to_string(), number),
            ("yesterday".to_string(), "kg".to_string(), of_a_day)
        ]
    );
}

#[test]
fn the_card_is_one_of_the_cards_the_burner_lists() {
    let cards = served()["cards"].clone();
    assert!(cards.as_array().unwrap().iter().any(|c| c == "pellets"), "{cards}");
}

#[test]
fn the_rate_is_a_setting_of_whole_grams_a_minute_from_sixty_to_six_hundred_starting_from_two_hundred_and_forty() {
    let json = served();
    let rate = json["settings"].as_array().unwrap().iter().find(|s| s["name"] == "pellet_rate").cloned().expect("no setting pellet_rate");
    assert_eq!(rate["default"], "240");
    // a whole number is served as the board's other whole numbers are: its kind, its span, the unit beside it
    assert_eq!((rate["kind"].clone(), rate["from"].clone(), rate["to"].clone(), rate["unit"].clone()), ("whole".into(), 60.into(), 600.into(), "g/min".into()), "{rate}");
    let rows = card(&json, "settings");
    let row = rows.iter().find(|t| t["name"] == "pellet_rate").expect("no row of the settings card for pellet_rate");
    assert_eq!(row["kinds"], serde_json::json!(["text"]));
}

#[test]
fn the_table_still_holds_together_with_the_pellets_in_it() {
    assert_eq!(page::dictionary::check(&burner_api::ALL), Ok(()));
}

#[test]
fn today_and_the_month_may_be_unknown_with_the_reason_a_count_not_sure_of_its_day_gives_and_the_total_may_not() {
    let rows = card(&served(), "pellets");
    let find = |name: &str| rows.iter().find(|t| t["name"] == name).unwrap().clone();
    for name in ["today", "month"] {
        let row = find(name);
        assert_eq!(row["kinds"], serde_json::json!(["number", "unknown"]), "{name}");
        assert_eq!(row["reasons"], serde_json::json!(["day_unsure"]), "{name}");
    }
    assert_eq!(find("total")["kinds"], serde_json::json!(["number"]));
}

#[test]
fn yesterday_may_be_unknown_for_the_two_reasons_there_is_no_number_not_sure_of_the_day_and_no_day_has_turned_yet() {
    let rows = card(&served(), "pellets");
    let row = rows.iter().find(|t| t["name"] == "yesterday").expect("no row yesterday");
    assert_eq!(row["kinds"], serde_json::json!(["number", "unknown"]));
    assert_eq!(row["reasons"], serde_json::json!(["day_unsure", "none_yet"]));
}
