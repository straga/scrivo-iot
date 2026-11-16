//! The burner's dictionary as a client reads it, held to a copy taken from the image of 11.2 (`tests/golden/dictionary.json`): every address, code and
//! setting the page and the clients know the burner by. A change of the shared pieces the burner's table is built from (`dictionary`) that moves an
//! address or says a word otherwise fails here with the first thing that differs, and is a question for whoever owns the burner, not a quiet edit.

use serde_json::Value;

/// The kit the copy was taken with: the dictionary's version is made of it.
const KIT: &str = "stragabases3-26.10.16.1";

#[test]
fn the_dictionary_the_burner_serves_is_the_one_its_clients_were_given() {
    let held: Value = serde_json::from_str(include_str!("golden/dictionary.json")).unwrap();
    let now: Value = serde_json::from_str(&burner_api::json(KIT)).unwrap();
    assert_eq!(held["things"].as_array().unwrap().len(), now["things"].as_array().unwrap().len(), "the table has another length: an address moved");
    for (at, (was, is)) in held["things"].as_array().unwrap().iter().zip(now["things"].as_array().unwrap()).enumerate() {
        assert_eq!(was, is, "address {at} says something else than it did");
    }
    assert_eq!(held, now, "something besides the things differs: the cards, the settings, the stream or the version");
}
