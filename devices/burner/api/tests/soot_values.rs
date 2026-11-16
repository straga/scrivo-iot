//! What the soot's module hands the stream: the number of the boiler and the burns behind it, the kilograms since the cleaning, the day of it. Expected from what a person reads on the card.

use burner_api::soot::put_values;
use page::dictionary::Value;
use std::collections::HashMap;

fn said(soot_c: Option<f32>, burns: usize, grams: u64, cleaned_day: Option<i64>) -> HashMap<&'static str, Value> {
    let mut by_address: HashMap<usize, Value> = HashMap::new();
    put_values(soot_c, burns, grams, cleaned_day, &mut |at, value| {
        by_address.insert(at, value);
    });
    let mut out = HashMap::new();
    for name in ["soot_c", "soot_burns", "kg_since_clean", "cleaned_day"] {
        if let Some(at) = page::dictionary::address(&burner_api::ALL, "soot", name) {
            if let Some(value) = by_address.get(&at) {
                out.insert(name, *value);
            }
        }
    }
    out
}

#[test]
fn the_numbers_are_said_each_by_its_row() {
    let v = said(Some(44.5), 6, 12_340, Some(20_401));
    assert_eq!(v.get("soot_c"), Some(&Value::Number(44.5)));
    assert_eq!(v.get("soot_burns"), Some(&Value::Number(6.0)));
    assert_eq!(v.get("kg_since_clean"), Some(&Value::Number(12.34)), "grams go out as kilograms");
    assert_eq!(v.get("cleaned_day"), Some(&Value::Number(20_401.0)));
}

#[test]
fn before_the_first_burn_ended_there_is_no_number_for_a_reason_and_not_nought() {
    let v = said(None, 0, 0, None);
    assert_eq!(v.get("soot_c"), Some(&Value::Unknown("no_burns")));
    assert_eq!(v.get("soot_burns"), Some(&Value::Number(0.0)));
    assert_eq!(v.get("kg_since_clean"), Some(&Value::Number(0.0)), "nothing burned since is a count of nothing");
    assert_eq!(v.get("cleaned_day"), Some(&Value::Unknown("none_yet")));
}

#[test]
fn all_four_rows_are_said_every_second() {
    assert_eq!(said(Some(40.0), 1, 0, None).len(), 4);
    assert_eq!(said(None, 0, 0, None).len(), 4);
}

#[test]
fn a_cleaning_the_flash_did_not_take_is_refused_by_a_code_one_word_long_and_not_by_a_sentence() {
    // the page says the words (page/menu.json, ui.refusals; tools/test_page_save_words.js holds the menu to this code): the board sends a code
    assert_eq!(burner_api::soot::NOT_WRITTEN, "clean_not_written");
    assert!(burner_api::soot::NOT_WRITTEN.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
}
