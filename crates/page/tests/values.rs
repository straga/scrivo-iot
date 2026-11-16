//! /state of the contract the document names: the version of the dictionary and every value by address, nothing else.

use page::dictionary::{Said, Value};
use page::{Values, Wanted};

/// A board of three addresses: a switch, a reading of the card "flame", and one of the system card.
struct Board;

impl Values for Board {
    fn version(&self) -> &str {
        "stragabases3-26.09.24.14+d655537b"
    }
    fn cards(&self) -> &'static [&'static str] {
        &["relays", "flame"]
    }
    fn values(&self, wanted: &Wanted, each: &mut dyn FnMut(usize, Said<'_>)) {
        if wanted.has("system") {
            each(0, Value::Number(48760.0).into());
        }
        if wanted.has("relays") {
            each(1, Value::On(false).into());
        }
        if wanted.has("flame") {
            each(2, Value::Unknown("not_read").into());
            each(3, Value::None.into());
        }
    }
}

fn state() -> String {
    let mut out = Vec::new();
    page::write_values(&mut out, &Board).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn the_state_is_the_version_and_every_value_by_address() {
    assert_eq!(
        state(),
        r#"{"contract":10,"dictionary":"stragabases3-26.09.24.14+d655537b","values":[[0,48760.0],[1,false],[2,{"unknown":"not_read"}],[3,null]]}"#
    );
}

#[test]
fn no_name_no_card_and_no_word_for_a_person_goes_out_with_the_values() {
    let said: serde_json::Value = serde_json::from_str(&state()).unwrap();
    let keys: Vec<&str> = said.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["contract", "dictionary", "values"]);
    for record in said["values"].as_array().unwrap() {
        let pair = record.as_array().expect("a record is a pair");
        assert_eq!(pair.len(), 2);
        assert!(pair[0].is_u64(), "an address is a number: {record}");
    }
}

#[test]
fn the_answer_to_a_command_is_the_values_of_its_row_and_nothing_else_of_its_card() {
    let mut out = Vec::new();
    let cards = Board.cards();
    page::write_values_of(&mut out, &Board, &Wanted::of(cards, &["flame"]), &|at| at == 2).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), r#"{"values":[[2,{"unknown":"not_read"}]]}"#);
    let mut none = Vec::new();
    page::write_values_of(&mut none, &Board, &Wanted::of(cards, &["relays"]), &|at| at == 2).unwrap();
    assert_eq!(String::from_utf8(none).unwrap(), r#"{"values":[]}"#, "the row is of another card");
}
