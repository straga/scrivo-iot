//! A setting the dictionary lists and no module of the burner keeps is read as its default and refused on save: the page offers it and a person cannot turn it. The module's list of the automaton (`burner_api::automaton::kept`) is
//! what the device registers; the dictionary's list of the card is what the page offers. Held against each other, and against the settings of the card by their own definition (`burner_settings::ALL`).

use burner_api::automaton;

#[test]
fn every_setting_of_the_automaton_card_is_kept_by_its_module_and_none_twice() {
    let kept: Vec<&str> = automaton::kept().iter().map(|s| s.name).collect();
    let unkept: Vec<&str> = burner_settings::ALL.iter().filter(|s| s.card == Some("automaton")).map(|s| s.name).filter(|n| !kept.contains(n)).collect();
    assert_eq!(unkept, Vec::<&str>::new(), "settings of the automaton's card that its module does not register");
    let twice: Vec<&str> = kept.iter().copied().filter(|n| kept.iter().filter(|k| k == &n).count() > 1).collect();
    assert_eq!(twice, Vec::<&str>::new(), "settings the module keeps twice");
}

#[test]
fn the_settings_the_dictionary_lists_for_the_automaton_are_the_ones_its_module_keeps() {
    let served: serde_json::Value = serde_json::from_str(&burner_api::json("kit")).unwrap();
    let listed: Vec<String> = served["settings"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap().to_string()).collect();
    for s in automaton::kept() {
        assert!(listed.iter().any(|n| n == s.name), "{} is kept by the module and not listed in the dictionary", s.name);
    }
}
