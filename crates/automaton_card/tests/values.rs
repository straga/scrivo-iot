//! The automaton's card as values by address: every row and every part of the card each second, a
//! dash where there is nothing to say, and no words.

use automaton::{Asked, Clock, Doing, PumpWhy, State, Why};
use automaton_card::{values, Card, Silent};
use burner_api::first_burner::{at, TABLE};
use page::dictionary::{Said, Value};

fn doing(state: State) -> Doing {
    Doing { state, auger: false, feeding: automaton::Feeding::Stop, igniter: false, fan: Asked::Stop, alarm: false, pump: false, pump_why: PumpWhy::Water(19.0) }
}

fn card<'a>(doing: Option<Doing>, clocks: &'a [Clock]) -> Card<'a> {
    Card { doing, lasted_s: 12, refused: None, clocks, broke_at: None, alarm_c: 95.0, feed_allowed: true, semi: false, silent: Silent::default() }
}

/// The values of CARD, by address; an address sent twice fails here. A text is in [`texts`], not here.
fn sent(card: &Card) -> Vec<Option<Value>> {
    let mut all: Vec<Option<Value>> = vec![None; TABLE.len()];
    let mut twice = Vec::new();
    values(card, &mut |at, said| {
        if all[at].is_some() {
            twice.push(at);
        }
        if let Said::Value(value) = said {
            all[at] = Some(value);
        }
    });
    assert!(twice.is_empty(), "sent twice: {twice:?}");
    all
}

/// The texts of CARD, by address.
fn texts(card: &Card) -> Vec<Option<String>> {
    let mut all = vec![None; TABLE.len()];
    values(card, &mut |at, said| {
        if let Said::Text(text) = said {
            all[at] = Some(text.to_string());
        }
    });
    all
}

/// Every address of the card of the automaton that carries a value: its rows and parts, not its buttons.
fn the_cards_addresses() -> Vec<usize> {
    TABLE.iter().enumerate().filter(|(_, e)| e.card == "automaton" && e.kinds != [page::dictionary::Kind::Button]).map(|(at, _)| at).collect()
}

#[test]
fn every_row_and_part_of_the_card_is_sent_every_time_and_nothing_else() {
    for c in [card(None, &[]), card(Some(doing(State::Stop)), &[]), card(Some(doing(State::Fault(Why::NeverLit(3)))), &[])] {
        let all = sent(&c);
        for at in the_cards_addresses() {
            assert!(all[at].is_some(), "{} was not sent", TABLE[at].name);
        }
        let others: Vec<&str> = (0..TABLE.len()).filter(|at| all[*at].is_some() && !the_cards_addresses().contains(at)).map(|at| TABLE[at].name).collect();
        assert!(others.is_empty(), "sent beyond the card: {others:?}");
    }
}

#[test]
fn before_the_first_second_the_stage_is_starting_and_every_other_row_says_nothing() {
    let all = sent(&card(None, &[]));
    assert_eq!(all[at::STAGE], Some(Value::Unknown("starting")));
    for at in [at::DOING, at::FOR, at::AUGER, at::PUMP_BY, at::ALARM, at::REFUSED] {
        assert_eq!(all[at], Some(Value::None), "{}", TABLE[at].name);
    }
    assert_eq!(all[at::FEED], Some(Value::On(true)), "the feed is a switch of the card, there before the automaton");
    assert_eq!(all[at::SEMI], Some(Value::On(false)), "the mode is a switch of the card, there before the automaton, Auto");
}

#[test]
fn semi_is_the_mode_switch_of_the_card_turned_on() {
    // Auto or Semi, switched from the scheme - on is Semi
    let all = sent(&Card { semi: true, ..card(Some(doing(State::Working)), &[]) });
    assert_eq!(all[at::SEMI], Some(Value::On(true)), "Semi was sent as Auto");
}

#[test]
fn a_fault_names_its_reason_by_code_and_its_count_beside_it() {
    let all = sent(&card(Some(doing(State::Fault(Why::NeverLit(3)))), &[]));
    assert_eq!(all[at::DOING], Some(Value::Word("fault")));
    assert_eq!(all[at::DOING_WHY], Some(Value::Word("never_lit")));
    assert_eq!(all[at::DOING_TRIES], Some(Value::Number(3.0)));
    assert_eq!(all[at::DOING_WATER], Some(Value::None));
    let hot = sent(&card(Some(doing(State::BurningOff(Some(Why::Overheated(91.5))))), &[]));
    assert_eq!(hot[at::DOING], Some(Value::Word("burning_off_because")));
    assert_eq!(hot[at::DOING_WATER], Some(Value::Number(91.5)));
    let plain = sent(&card(Some(doing(State::BurningOff(None))), &[]));
    assert_eq!(plain[at::DOING], Some(Value::Word("burning_off")));
    assert_eq!(plain[at::DOING_WHY], Some(Value::None));
}

#[test]
fn a_working_second_says_its_organs_its_clock_in_whole_seconds_up_and_the_pumps_water() {
    let d = Doing { auger: true, fan: Asked::Hold(51.0), pump: true, pump_why: PumpWhy::Water(47.4), ..doing(State::Working) };
    let clocks = [Clock { what: "auger switches in", left_s: Some(4.2) }, Clock { what: "full step in", left_s: Some(0.0) }];
    let all = sent(&card(Some(d), &clocks));
    assert_eq!(all[at::STAGE], Some(Value::Word("working")));
    assert_eq!(all[at::AUGER], Some(Value::On(true)));
    assert_eq!(all[at::AUTOMATON_FAN], Some(Value::Number(51.0)));
    assert_eq!(all[at::PUMP_BY], Some(Value::Word("water")));
    assert_eq!(all[at::PUMP_BY_WATER], Some(Value::Number(47.4)));
    assert_eq!(all[at::FOR], Some(Value::Seconds(12)));
    let clock = |what| all[burner_api::first_burner::clock_at(what).unwrap()].clone();
    assert_eq!(clock("auger switches in"), Some(Value::Seconds(5)));
    assert_eq!(clock("full step in"), Some(Value::Seconds(0)), "a clock run out stays at nought");
    assert_eq!(clock("dose ends in"), Some(Value::None), "a clock this state does not count");
}

#[test]
fn the_alarm_and_a_refusal_are_there_only_while_they_are_true() {
    let d = Doing { alarm: true, ..doing(State::Working) };
    let c = Card { refused: Some("the numbers do not hold"), broke_at: Some("working"), ..card(Some(d), &[]) };
    let all = sent(&c);
    assert_eq!(all[at::ALARM], Some(Value::On(true)));
    assert_eq!(all[at::ALARM_PAST], Some(Value::Number(95.0)));
    assert_eq!(texts(&c)[at::REFUSED].as_deref(), Some("the numbers do not hold"));
    assert_eq!(all[at::REFUSED], None, "and no value beside the text");
    assert_eq!(all[at::BROKE_AT], Some(Value::Word("working")));
    let quiet = sent(&card(Some(doing(State::Working)), &[]));
    for at in [at::ALARM, at::ALARM_PAST, at::REFUSED, at::BROKE_AT] {
        assert_eq!(quiet[at], Some(Value::None), "{}", TABLE[at].name);
    }
}

#[test]
fn a_silent_sensor_is_on_the_card_in_seconds_by_its_own_row_and_one_that_speaks_is_nothing() {
    // the card of a sensor whose round stopped is published by nobody, so the automaton's card says it
    let silent = Silent { draught: Some(7), water: Some(64), ..Silent::default() };
    let all = sent(&Card { silent, ..card(Some(doing(State::Working)), &[]) });
    assert_eq!(all[at::SILENT_DRAUGHT], Some(Value::Seconds(7)));
    assert_eq!(all[at::SILENT_WATER], Some(Value::Seconds(64)));
    assert_eq!(all[at::SILENT_FLAME], Some(Value::None), "the flame speaks: nothing, sent all the same");
    assert_eq!(all[at::SILENT_FLUE], Some(Value::None));
    let pair = sent(&card(Some(doing(State::Working)), &[]));
    for address in [at::SILENT_DRAUGHT, at::SILENT_FLAME, at::SILENT_FLUE, at::SILENT_WATER] {
        assert_eq!(pair[address], Some(Value::None), "every sensor speaks");
    }
}
