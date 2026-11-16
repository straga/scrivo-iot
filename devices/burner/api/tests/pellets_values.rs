//! What the pellets' module hands the stream: the counts in kilograms, each by its row, the same four in every second. Expected from what a person reads on the card - what was burned today, yesterday, this month, in all.

use burner_api::pellets::put_values;
use page::dictionary::Value;
use std::collections::HashMap;

fn said(kg: [f32; 3]) -> HashMap<&'static str, Value> {
    said_with(kg, Some(kg[0]), true)
}

/// The counts as said when yesterday is YESTERDAY and the count is sure of its day or not.
fn said_with(kg: [f32; 3], yesterday: Option<f32>, day_sure: bool) -> HashMap<&'static str, Value> {
    let mut by_address: HashMap<usize, Value> = HashMap::new();
    put_values(&kg, yesterday, day_sure, &mut |at, value| {
        by_address.insert(at, value);
    });
    let mut out = HashMap::new();
    for name in ["today", "yesterday", "month", "total"] {
        if let Some(value) = page::dictionary::address(&burner_api::ALL, "pellets", name).and_then(|at| by_address.get(&at)) {
            out.insert(name, *value);
        }
    }
    out
}

#[test]
fn the_three_counts_are_said_each_by_its_row() {
    let v = said([1.5, 10.25, 1234.0]);
    assert_eq!(v.get("today"), Some(&Value::Number(1.5)));
    assert_eq!(v.get("month"), Some(&Value::Number(10.25)));
    assert_eq!(v.get("total"), Some(&Value::Number(1234.0)));
}

#[test]
fn a_count_of_nothing_is_a_number_nought_and_not_silence() {
    let v = said([0.0, 0.0, 0.0]);
    assert_eq!(v.len(), 4);
    assert!(v.values().all(|value| *value == Value::Number(0.0)));
}

#[test]
fn a_count_not_sure_of_its_day_says_no_number_for_today_and_the_month_and_the_total_as_it_is() {
    let v = said_with([3.2, 3.2, 5.0], Some(1.0), false);
    assert_eq!(v.get("today"), Some(&Value::Unknown("day_unsure")), "today is not a number the board stands behind");
    assert_eq!(v.get("month"), Some(&Value::Unknown("day_unsure")));
    assert_eq!(v.get("yesterday"), Some(&Value::Unknown("day_unsure")), "a day kept from before is not the day before today until the board knows the day");
    assert_eq!(v.get("total"), Some(&Value::Number(5.0)), "the total is never hidden");
}

#[test]
fn yesterday_is_a_number_beside_the_others_in_kilograms() {
    let v = said_with([1.5, 10.25, 1234.0], Some(3.3), true);
    assert_eq!(v.get("yesterday"), Some(&Value::Number(3.3)));
    assert_eq!(v.get("today"), Some(&Value::Number(1.5)), "the other rows are as they were");
}

#[test]
fn before_the_first_midnight_yesterday_is_unknown_for_a_reason_and_not_nought() {
    let v = said_with([1.5, 10.25, 1234.0], None, true);
    assert_eq!(v.get("yesterday"), Some(&Value::Unknown("none_yet")));
    assert_eq!(v.get("today"), Some(&Value::Number(1.5)));
}

#[test]
fn a_count_sure_of_its_day_says_the_numbers_it_says() {
    let v = said([3.2, 3.2, 5.0]);
    assert!(v.values().all(|value| matches!(value, Value::Number(_))), "{v:?}");
}
