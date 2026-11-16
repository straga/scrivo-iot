//! The page can say every refusal the board can send: each code of a save's answer and of the automaton's door has a phrase in the page's table, in both languages, and no phrase asks for a place its answer
//! does not carry. Held against the real answers - `Refusal::wire` and `page::save` - and the real table, `page/menu.json`; a phrase's wording is the page's and is not looked at.

use std::collections::BTreeSet;

use serde_json::Value;
use settings::Refusal;

/// The menu as the burner's page carries it: the engine's (`page/menu.json`) with the burner's own (`devices/burner/page/menu.json`) laid over it, as
/// `tools/page_sources.py` merges them.
fn menu() -> Value {
    let read = |path: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(format!("{}/../{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()).unwrap() };
    fn laid(base: &mut Value, over: Value) {
        match (base, over) {
            (Value::Object(base), Value::Object(over)) => {
                for (key, value) in over {
                    match base.get_mut(&key) {
                        Some(held) => laid(held, value),
                        None => {
                            base.insert(key, value);
                        }
                    }
                }
            }
            (base, over) => *base = over,
        }
    }
    let mut menu = read("../../page/menu.json");
    laid(&mut menu, read("page/menu.json"));
    menu
}

/// The `{places}` of a phrase.
fn places(phrase: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = phrase;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else { break };
        out.insert(rest[open + 1..open + close].split(':').next().unwrap().to_string());
        rest = &rest[open + close + 1..];
    }
    out
}

/// One refusal of each kind a save can meet at the settings, as the board says it.
fn the_settings_refusals() -> Vec<Refusal> {
    vec![
        Refusal::NoSetting { name: "x".into() },
        Refusal::NotMonths { name: "keep", text: "0".into(), from: 1, to: 120 },
        Refusal::NotSeconds { name: "auger_limit", text: "0".into(), from: 10, to: 600 },
        Refusal::NotWhole { name: "auto_hold_c", text: "0".into(), from: 5, to: 85 },
        Refusal::NoneOf { name: "zone", text: "mars".into(), choices: vec!["utc"], own: None },
        Refusal::NoneOf { name: "time", text: "x y".into(), choices: vec!["auto", "server"], own: Some("server") },
        Refusal::NoSocket { name: "auto_hold", role: "room", said: "the boiler room".into() },
        Refusal::NotKept,
        Refusal::NotWritten { detail: "nvs: no free pages".into() },
        Refusal::Busy,
        Refusal::Silent { within_s: 3 },
    ]
}

#[test]
fn every_refusal_of_a_save_has_a_phrase_in_both_languages_that_asks_only_for_what_the_answer_carries() {
    let menu = menu();
    let mut carried: Vec<(String, BTreeSet<String>)> = the_settings_refusals()
        .iter()
        .map(|r| {
            let wire = r.wire();
            let mut keys: BTreeSet<String> = wire.as_object().unwrap().keys().cloned().collect();
            // the page fills `label` from the name, and `takes` from the choice that takes a word of its own
            if keys.contains("name") {
                keys.insert("label".into());
            }
            if keys.contains("own") {
                keys.insert("takes".into());
            }
            let code = if wire.get("own").is_some() { "none_of_own".to_string() } else { r.code().to_string() };
            (code, keys)
        })
        .collect();
    // the answers the page's contract gives before the board is asked
    for said in [page::refusal::no_setting("wifi"), page::refusal::value_too_long("zone", page::MAX_VALUE + 1), page::refusal::nothing_to_save(), page::refusal::form_too_long(page::MAX_VALUE)] {
        let error = said["error"].as_object().unwrap_or_else(|| panic!("not a refusal by a code: {said}"));
        let mut keys: BTreeSet<String> = error.keys().cloned().collect();
        if keys.contains("name") {
            keys.insert("label".into());
        }
        carried.push((error["code"].as_str().unwrap().to_string(), keys));
    }
    let table = &menu["ui"]["refusals"];
    for (code, keys) in &carried {
        for lang in ["en", "ru"] {
            let phrase = table[code][lang].as_str().unwrap_or_else(|| panic!("the page has no {lang} phrase for the code {code}"));
            let asked = places(phrase);
            let missing: Vec<&String> = asked.iter().filter(|p| !keys.contains(*p)).collect();
            assert!(missing.is_empty(), "{lang} phrase of {code} asks for {missing:?}, which the answer does not carry ({keys:?})");
        }
    }
    assert!(table["unknown"]["en"].is_string() && table["unknown"]["ru"].is_string(), "the page has no phrase for a code it does not know");
    assert_eq!(places(table["unknown"]["en"].as_str().unwrap()), BTreeSet::from(["code".to_string()]));
}

#[test]
fn every_code_the_doors_refusal_can_carry_has_a_phrase_in_both_languages_with_only_the_places_the_card_carries() {
    let menu = menu();
    let says = menu["modules"]["automaton"]["rows"]["refused"]["says"].as_array().expect("the refusal row says nothing by code");
    let carried: BTreeSet<String> = ["field", "value", "than", "limit"].iter().map(|k| k.to_string()).collect();
    for code in automaton::REFUSALS {
        let line = says.iter().find(|s| s[0] == code).unwrap_or_else(|| panic!("the page says nothing for the code {code}"));
        for lang in ["en", "ru"] {
            let phrase = line[1][lang].as_str().unwrap_or_else(|| panic!("no {lang} phrase for {code}"));
            let asked = places(phrase);
            assert!(asked.is_subset(&carried), "{lang} phrase of {code} asks for {:?}", asked.difference(&carried).collect::<Vec<_>>());
        }
    }
    for line in says {
        assert!(automaton::REFUSALS.contains(&line[0].as_str().unwrap()), "the page says something for {} and the door has no such code", line[0]);
    }
}
