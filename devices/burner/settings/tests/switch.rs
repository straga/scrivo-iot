//! A choice of two or three is a switch: short words on it, one press sets it, and a line under it says what the set
//! one means now: a choice of sentences with a Save under it "is not how a page is
//! made".

use settings::{switch_refusal, Choice, Kind, Setting, SWITCH_UP_TO};
use burner_settings::{ALL};

#[test]
fn every_choice_of_up_to_three_on_the_board_is_short_words_and_says_what_each_means() {
    let refused: Vec<String> = ALL.iter().filter_map(|s| switch_refusal(s)).collect();
    assert!(refused.is_empty(), "{refused:#?}");
}

const SENTENCES: [Choice; 2] = [
    Choice { value: "idle", words: "idle - turns the fan, only shows what it would do", takes: None, checks: None, means: Some("x") },
    Choice { value: "live", words: "live", takes: None, checks: None, means: Some("the auger and the igniter run") },
];
const NO_MEANING: [Choice; 2] = [
    Choice { value: "on", words: "on", takes: None, checks: None, means: Some("it runs") },
    Choice { value: "off", words: "off", takes: None, checks: None, means: None },
];
const FIVE: [Choice; 5] = [
    Choice { value: "a", words: "a long sentence of a list", takes: None, checks: None, means: None },
    Choice { value: "b", words: "b", takes: None, checks: None, means: None },
    Choice { value: "c", words: "c", takes: None, checks: None, means: None },
    Choice { value: "d", words: "d", takes: None, checks: None, means: None },
    Choice { value: "e", words: "e", takes: None, checks: None, means: None },
];

fn setting(label: &'static str, choices: &'static [Choice]) -> Setting {
    Setting { name: "mode", label, default: "idle", kind: Kind::Choice(choices), level: settings::Level::Work, card: None }
}

#[test]
fn a_choice_worded_as_a_sentence_is_refused_naming_the_setting_and_the_choice() {
    let got = switch_refusal(&setting("Automaton runs", &SENTENCES)).unwrap_or_default();
    assert!(got.contains("Automaton runs") && got.contains("idle"), "{got:?}");
}

#[test]
fn a_choice_that_does_not_say_what_it_means_is_refused_naming_it() {
    let got = switch_refusal(&setting("Pump", &NO_MEANING)).unwrap_or_default();
    assert!(got.contains("Pump") && got.contains("off"), "{got:?}");
}

#[test]
fn a_choice_of_a_list_is_no_switch_and_its_words_are_its_own() {
    assert!(FIVE.len() > SWITCH_UP_TO);
    assert_eq!(switch_refusal(&setting("Zone", &FIVE)), None);
}
