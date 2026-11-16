//! The console words that modules add: which are listed, who answers a word two modules use, what a word that takes a number or asks first says.

use module::{say, Module, Modules, Verb};

struct Pellets;
struct Energy;
struct Silent;

fn pellets_reset(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    match words {
        ["pellets", rest @ ..] => Some(say::confirmed(rest).map(|yes| line(if yes { "pellets zeroed" } else { "reset pellets zeroes the pellets; to do it: reset pellets yes" }))),
        _ => None,
    }
}

fn energy_reset(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    match words {
        ["energy", rest @ ..] => Some(say::confirmed(rest).map(|yes| line(if yes { "energy zeroed" } else { "reset energy zeroes the energy; to do it: reset energy yes" }))),
        _ => None,
    }
}

static PELLETS_VERBS: [Verb; 1] = [Verb { name: "reset", help: "Zero the pellets counted", run: pellets_reset }];
static ENERGY_VERBS: [Verb; 1] = [Verb { name: "reset", help: "Zero the energy counted", run: energy_reset }];

impl Module for Pellets {
    fn card(&self) -> &'static str {
        "pellets"
    }

    fn verbs(&self) -> &'static [Verb] {
        &PELLETS_VERBS
    }
}

impl Module for Energy {
    fn card(&self) -> &'static str {
        "energy"
    }

    fn verbs(&self) -> &'static [Verb] {
        &ENERGY_VERBS
    }

    fn takes(&self) -> &'static [&'static str] {
        &["press"]
    }
}

impl Module for Silent {
    fn card(&self) -> &'static str {
        "silent"
    }
}

static BURNER: [&dyn Module; 2] = [&Silent, &Pellets];
static BOTH: [&dyn Module; 2] = [&Pellets, &Energy];

fn ask(modules: &Modules, name: &str, words: &[&str]) -> (Option<Result<(), String>>, Vec<String>) {
    let mut said = Vec::new();
    let result = module::verb(modules, name, words, &mut |l| said.push(l.to_string()));
    (result, said)
}

#[test]
fn a_word_is_listed_once_with_the_help_of_the_first_module_that_has_it() {
    let names: Vec<(&str, &str)> = module::verbs(&BOTH).iter().map(|v| (v.name, v.help)).collect();
    assert_eq!(names, [("reset", "Zero the pellets counted")]);
    assert!(module::verbs(&[&Silent]).is_empty(), "a module with no word lists none");
}

#[test]
fn the_module_that_takes_the_words_answers_and_the_one_that_does_not_is_passed() {
    assert_eq!(ask(&BOTH, "reset", &["energy", "yes"]), (Some(Ok(())), vec!["energy zeroed".to_string()]));
    assert_eq!(ask(&BOTH, "reset", &["pellets", "yes"]), (Some(Ok(())), vec!["pellets zeroed".to_string()]));
}

#[test]
fn a_word_nobody_takes_the_words_of_is_none_and_so_is_a_word_nobody_has() {
    assert_eq!(ask(&BURNER, "reset", &["energy"]).0, None, "the burner counts no energy");
    assert_eq!(ask(&BURNER, "reset", &[]).0, None);
    assert_eq!(ask(&BURNER, "dim", &["40"]).0, None, "nothing here holds a fan");
}

#[test]
fn a_module_that_takes_the_words_and_refuses_them_has_the_last_word() {
    let (result, said) = ask(&BURNER, "reset", &["pellets", "maybe"]);
    assert_eq!(result, Some(Err("'maybe' is not yes; nothing zeroed".to_string())));
    assert!(said.is_empty(), "nothing was zeroed and nothing said: {said:?}");
}

#[test]
fn a_word_that_asks_first_asks_and_does_nothing_until_it_is_told_yes() {
    let (result, said) = ask(&BURNER, "reset", &["pellets"]);
    assert_eq!(result, Some(Ok(())));
    assert_eq!(said, ["reset pellets zeroes the pellets; to do it: reset pellets yes"]);
}

#[test]
fn the_shells_optional_words_are_taken_only_where_a_module_carries_them_out() {
    assert!(module::takes(&BOTH, "press"));
    assert!(!module::takes(&BOTH, "toggle"));
    assert!(!module::takes(&BURNER, "press"));
}

#[test]
fn one_number_is_read_whole_and_anything_else_is_refused_in_words() {
    assert_eq!(say::one_number("dim", &["40"], "dim 40"), Ok(40));
    assert_eq!(say::one_number("dim", &["0"], "dim 40"), Ok(0));
    for bad in [&[][..], &["half"][..], &["-5"][..], &["4.5"][..], &["1", "2"][..]] {
        let why = say::one_number("dim", bad, "dim 40, dim 0 to stop it").unwrap_err();
        assert!(why.contains("dim 40, dim 0 to stop it"), "{bad:?}: {why}");
    }
}
