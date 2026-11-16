//! What the board answers to a save: what changed as the name and the value kept, and a refusal as a code with the field and the numbers it stands on. Never a sentence: the label of a setting, the words of
//! a choice and the phrase of a refusal are the page's own table, in the language the page speaks. Expected from what a person must be told and from what the page has to know to say it - not from the code that builds the answer.

use burner_settings::RULES;
use std::collections::HashMap;

use serde_json::{json, Value};
use settings::{change_all, seed, Setting, Store};
use burner_settings::{ALL, AUTO_DOSE, AUTO_HOLD_C};

#[derive(Default)]
struct Map(HashMap<String, String>);

impl Store for Map {
    fn get(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.0.insert(name.into(), value.into());
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<bool, String> {
        Ok(self.0.remove(name).is_some())
    }
}

/// A store whose every write fails.
struct Full(Map);

impl Store for Full {
    fn get(&self, name: &str) -> Option<String> {
        self.0.get(name)
    }
    fn put(&mut self, _: &str, _: &str) -> Result<(), String> {
        Err("nvs: no free pages".into())
    }
    fn remove(&mut self, name: &str) -> Result<bool, String> {
        self.0.remove(name)
    }
}

fn seeded() -> Map {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    store
}

fn span(s: &Setting) -> (u32, u32) {
    match s.kind {
        settings::Kind::Whole { from, to, .. } | settings::Kind::Seconds { from, to } | settings::Kind::Tenths { from, to, .. } | settings::Kind::Months { from, to } => (from, to),
        _ => panic!("{} has no span", s.name),
    }
}

fn refused(pairs: &[(&str, &str)]) -> Value {
    change_all(&ALL, &RULES, &mut seeded(), pairs).expect_err("the save was taken").wire()
}

#[test]
fn what_a_save_changed_is_each_name_with_the_value_kept_and_nothing_a_person_reads() {
    let mut store = seeded();
    let moved = change_all(&ALL, &RULES, &mut store, &[("time", "internet"), ("zone", "gmt+2"), ("net_reset_laps", "5"), ("auto_hold_c", "45")]).unwrap();
    let wire: Vec<Value> = moved.iter().map(|m| m.wire()).collect();
    assert_eq!(
        wire,
        [json!({"name": "time", "value": "internet"}), json!({"name": "net_reset_laps", "value": "5"}), json!({"name": "auto_hold_c", "value": "45"})],
        "zone stayed as it was and is not said"
    );
    for line in wire {
        let text = line.to_string();
        assert!(!text.contains("Hold until") && !text.contains("Time comes from"), "the label of a setting is the page's: {text}");
    }
}

#[test]
fn a_save_that_changes_nothing_says_nothing() {
    let mut store = seeded();
    assert!(change_all(&ALL, &RULES, &mut store, &[("zone", "gmt+2")]).unwrap().is_empty());
}

#[test]
fn a_number_outside_its_span_or_no_number_is_refused_by_the_code_of_its_kind_with_the_span() {
    let (from, to) = span(&AUTO_HOLD_C);
    for text in ["abc", "", "4", "86", "45.5"] {
        assert_eq!(refused(&[("auto_hold_c", text)]), json!({"code": "not_whole", "name": "auto_hold_c", "text": text, "from": from, "to": to}), "{text:?}");
    }
    // a time of the auger is kept in tenths of a second and refused in seconds, as a person types it
    assert_eq!(refused(&[("auto_dose", "0")]), json!({"code": "not_tenths", "name": "auto_dose", "text": "0", "from": "0.1", "to": "120", "unit": "s"}));
}

#[test]
fn a_number_inside_its_span_is_taken_at_both_ends() {
    let (from, to) = span(&AUTO_HOLD_C);
    for n in [from, to] {
        let moved = change_all(&ALL, &RULES, &mut seeded(), &[("auto_hold_c", &n.to_string())]).unwrap();
        assert_eq!(moved[0].wire(), json!({"name": "auto_hold_c", "value": n.to_string()}));
    }
}

#[test]
fn a_word_that_is_none_of_the_choices_is_refused_with_the_choices_by_their_values() {
    assert_eq!(
        refused(&[("zone", "mars")]),
        json!({"code": "none_of", "name": "zone", "text": "mars", "choices": ["gmt+2", "gmt+1", "gmt", "gmt+3", "utc"]})
    );
}

#[test]
fn a_choice_that_takes_a_word_of_its_own_says_which_one_in_its_refusal() {
    // "server" takes a host: a name with a space is no host, and the page says what a server takes from its own table
    let said = refused(&[("time", "server:not a host")]);
    assert_eq!(said["code"], "none_of", "{said}");
    assert_eq!(said["own"], "server", "{said}");
    assert_eq!(said["text"], "server:not a host", "{said}");
    // a choice that takes a word but has no rule of its own is refused for a bare word the same way
    let said = refused(&[("role_ntc_1", "boilr")]);
    assert_eq!((said["code"].clone(), said["own"].clone()), (json!("none_of"), json!("own")), "{said}");
    // and one that takes no word says none
    assert!(refused(&[("zone", "mars")]).get("own").is_none());
}

#[test]
fn a_name_the_board_has_not_is_refused_by_the_name() {
    assert_eq!(refused(&[("wifi", "x")]), json!({"code": "no_setting", "name": "wifi"}));
}

#[test]
fn a_mode_held_by_a_sensor_no_socket_plays_is_refused_with_the_role_it_lacks() {
    assert_eq!(refused(&[("auto_hold", "return")]), json!({"code": "no_socket", "name": "auto_hold", "role": "return"}));
}

#[test]
fn a_thermostat_that_asks_by_a_room_no_socket_plays_is_refused_with_the_role_it_lacks_and_the_room_is_no_mode_to_hold() {
    assert_eq!(refused(&[("auto_asked_by", "room")]), json!({"code": "no_socket", "name": "auto_asked_by", "role": "room"}));
    assert_eq!(refused(&[("auto_hold", "room")])["code"], "none_of", "the room was a mode of the water once and is the thermostat's now");
}

#[test]
fn a_save_the_store_refuses_is_a_code_alone_and_the_drivers_text_is_not_in_the_answer_and_nothing_is_stored() {
    let mut store = Full(seeded());
    let why = change_all(&ALL, &RULES, &mut store, &[("time", "internet")]).expect_err("the store refused every write");
    assert_eq!(why.wire(), json!({"code": "not_written"}));
    assert!(!why.wire().to_string().contains("nvs"), "the driver's own words are in the answer: {}", why.wire());
    // the words of the driver stay for the log, which a person reads off a terminal
    assert!(why.to_string().contains("no free pages"), "{why}");
}

#[test]
fn one_bad_value_in_a_save_names_the_bad_one_and_stores_nothing() {
    let mut store = seeded();
    let said = change_all(&ALL, &RULES, &mut store, &[("time", "internet"), ("net_reset_laps", "0")]).expect_err("a bad value was taken").wire();
    assert_eq!(said["name"], "net_reset_laps");
    assert_eq!(said["code"], "not_whole");
    assert_eq!(settings::value(&settings::TIME, &store), "auto", "the good one was stored beside the bad one");
}
