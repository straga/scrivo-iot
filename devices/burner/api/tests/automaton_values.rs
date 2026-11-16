//! What the automaton's module hands the stream for a second: the stage and what it does by code, the attempt, the reason and what the reason carries, where it broke, how long the state has lasted, the clocks
//! counting down, the organs, the feed and the mode, the pump and why, what the fan is asked, the alarm, the silence of the sensors, the door's refusal of a number. Expected from what the automaton does and what a person
//! must be told - a clock never reads nought while its thing still runs, a row with nothing to say in this state says nothing and stays where it is, the switches always say where they stand - and from the first burner's
//! card, which says the same; not from the code that makes the values.

use automaton::{Asked, Clock, Doing, PumpWhy, Refused, State, Why};
use burner_api::automaton::{put_values, Now, ReliefNow, Silent};
use page::dictionary::{Said, Value};
use std::collections::HashMap;

/// Where the table keeps the row NAME of the card, or its part PART; None for one the table has not.
fn address(name: &str, part: Option<&str>) -> Option<usize> {
    let all = &burner_api::ALL;
    let row = page::dictionary::address(all, "automaton", name)?;
    match part {
        None => Some(row),
        Some(part) => page::dictionary::part(all, row, part),
    }
}

/// What the stream is told for NOW, by the name of the row (or `row.part`), and nothing for a name the second does not mention.
fn said<'a>(now: &Now<'a>) -> HashMap<String, Said<'a>> {
    let mut by_address: HashMap<usize, Said<'a>> = HashMap::new();
    put_values(now, &mut |at, said| {
        by_address.insert(at, said);
    });
    let mut out = HashMap::new();
    for (name, part) in NAMES {
        if let Some(said) = address(name, *part).and_then(|at| by_address.get(&at)) {
            out.insert(part.map_or(name.to_string(), |p| format!("{name}.{p}")), *said);
        }
    }
    out
}

/// Every row and part of the card that carries a value.
const NAMES: &[(&str, Option<&str>)] = &[
    ("stage", None),
    ("doing", None),
    ("doing", Some("attempt")),
    ("doing", Some("why")),
    ("doing", Some("tries")),
    ("doing", Some("water")),
    ("broke_at", None),
    ("silent_draught", None),
    ("silent_flame", None),
    ("silent_flue", None),
    ("silent_water", None),
    ("silent_fronts", None),
    ("relief_in", None),
    ("relief_today", None),
    ("relief_limit", None),
    ("for", None),
    ("lighting_in", None),
    ("dose_ends_in", None),
    ("igniter_off_in", None),
    ("attempt_ends_in", None),
    ("fed_after_flame_for", None),
    ("full_step_in", None),
    ("auger_switches_in", None),
    ("relight_in", None),
    ("no_relight_burning_off_in", None),
    ("burning_off_in", None),
    ("burning_out_ends_in", None),
    ("blowing_clear_ends_in", None),
    ("auger", None),
    ("feed", None),
    ("semi", None),
    ("igniter", None),
    ("pump", None),
    ("pump_by", None),
    ("pump_by", Some("water")),
    ("fan", None),
    ("alarm", None),
    ("alarm", Some("past")),
    ("organs", None),
    ("refused", None),
    ("refused", Some("field")),
    ("refused", Some("value")),
    ("refused", Some("than")),
    ("refused", Some("limit")),
];

fn doing(state: State) -> Doing {
    Doing { state, auger: false, feeding: automaton::Feeding::Stop, igniter: false, fan: Asked::Stop, alarm: false, pump: false, pump_why: PumpWhy::Water(40.0) }
}

fn now(doing: Option<Doing>) -> Now<'static> {
    Now { doing, lasted_s: 0, refused: None, clocks: &[], broke_at: None, feed_allowed: true, semi: false, silent: Silent::default(), relief: ReliefNow::default(), alarm_c: 95.0 }
}

fn value(said: &HashMap<String, Said<'_>>, name: &str) -> Option<Value> {
    match said.get(name)? {
        Said::Value(v) => Some(*v),
        Said::Text(_) => None,
    }
}

#[test]
fn before_its_first_second_the_automaton_is_starting_and_says_nothing_else_but_where_its_switches_stand() {
    let v = said(&now(None));
    assert_eq!(value(&v, "stage"), Some(Value::Unknown("starting")));
    for name in ["doing", "doing.attempt", "doing.why", "broke_at", "for", "auger", "igniter", "pump", "pump_by", "fan", "alarm", "lighting_in", "burning_off_in"] {
        assert_eq!(value(&v, name), Some(Value::None), "{name}");
    }
    assert_eq!((value(&v, "feed"), value(&v, "semi")), (Some(Value::On(true)), Some(Value::On(false))));
    assert_eq!(value(&v, "organs"), Some(Value::Words(&["auger", "igniter", "pump", "fan"])));
}

#[test]
fn a_kindling_says_its_attempt_how_long_it_has_lasted_and_the_clock_of_its_igniter_in_whole_seconds_up() {
    let clocks = [Clock { what: "igniter off in", left_s: Some(6.2) }, Clock { what: "attempt ends in", left_s: Some(41.0) }];
    let mut n = now(Some(Doing { igniter: true, fan: Asked::Hold(30.0), ..doing(State::Kindling(2)) }));
    n.lasted_s = 12;
    n.clocks = &clocks;
    let v = said(&n);
    assert_eq!((value(&v, "stage"), value(&v, "doing"), value(&v, "doing.attempt")), (Some(Value::Word("kindling")), Some(Value::Word("kindling")), Some(Value::Number(2.0))));
    assert_eq!(value(&v, "for"), Some(Value::Seconds(12)));
    // 6.2 s left is seven seconds on the card: a clock never reads nought while its thing is running
    assert_eq!((value(&v, "igniter_off_in"), value(&v, "attempt_ends_in")), (Some(Value::Seconds(7)), Some(Value::Seconds(41))));
    assert_eq!(value(&v, "burning_off_in"), Some(Value::None), "a clock of another state says nothing");
    assert_eq!((value(&v, "igniter"), value(&v, "auger"), value(&v, "fan")), (Some(Value::On(true)), Some(Value::On(false)), Some(Value::Number(30.0))));
}

#[test]
fn a_clock_that_has_run_out_reads_nought_only_when_it_is_nought() {
    for (left, seconds) in [(0.0, 0), (0.01, 1), (0.99, 1), (1.0, 1), (1.01, 2)] {
        let clocks = [Clock { what: "dose ends in", left_s: Some(left) }];
        let mut n = now(Some(doing(State::Kindling(1))));
        n.clocks = &clocks;
        assert_eq!(value(&said(&n), "dose_ends_in"), Some(Value::Seconds(seconds)), "{left} s left");
    }
}

#[test]
fn a_clock_that_has_not_begun_is_said_unknown_with_its_reason_and_one_that_runs_in_seconds() {
    for (what, code) in [("blowing clear ends in", "blowing_clear_ends_in"), ("fed after watching the flame for", "fed_after_flame_for"), ("full step in", "full_step_in")] {
        let waiting = [Clock { what, left_s: None }];
        let mut n = now(Some(doing(State::BurningOff(None))));
        n.clocks = &waiting;
        assert_eq!(value(&said(&n), code), Some(Value::Unknown("not_begun")), "{what}: the clock has not begun and says a number");
        let running = [Clock { what, left_s: Some(6.2) }];
        n.clocks = &running;
        assert_eq!(value(&said(&n), code), Some(Value::Seconds(7)), "{what}: the clock runs and says no number");
    }
}

#[test]
fn a_clock_the_state_does_not_have_says_nothing_and_not_that_it_has_not_begun() {
    let n = now(Some(doing(State::Working)));
    for code in ["blowing_clear_ends_in", "fed_after_flame_for", "full_step_in"] {
        assert_eq!(value(&said(&n), code), Some(Value::None), "{code}");
    }
}

#[test]
fn the_fan_the_automaton_asks_for_is_a_number_of_pascals_or_the_word_of_what_it_does_without_one() {
    for (asked, said_as) in [
        (Asked::Hold(51.0), Value::Number(51.0)),
        (Asked::Purge, Value::Word("full")),
        (Asked::Stop, Value::Word("off")),
        (Asked::ByHand, Value::Word("by_hand")),
    ] {
        let v = said(&now(Some(Doing { fan: asked, ..doing(State::Working) })));
        assert_eq!(value(&v, "fan"), Some(said_as), "{asked:?}");
    }
}

#[test]
fn a_burning_off_with_a_reason_is_said_as_such_and_carries_what_the_reason_carries() {
    for (why, code, tries, water) in [
        (Why::NeverLit(3), "never_lit", Some(3.0), None),
        (Why::FlameNotBack(2), "flame_not_back", Some(2.0), None),
        (Why::Overheated(97.0), "overheated", None, Some(97.0)),
        (Why::NoFuel, "no_fuel", None, None),
        (Why::DraughtSilent, "draught_silent", None, None),
        (Why::NoFronts, "no_fronts", None, None),
    ] {
        let mut n = now(Some(doing(State::BurningOff(Some(why)))));
        n.broke_at = Some("working");
        let v = said(&n);
        assert_eq!((value(&v, "stage"), value(&v, "doing")), (Some(Value::Word("burning_off")), Some(Value::Word("burning_off_because"))), "{code}");
        assert_eq!(value(&v, "doing.why"), Some(Value::Word(code)));
        assert_eq!(value(&v, "doing.tries"), Some(tries.map_or(Value::None, Value::Number)), "{code}");
        assert_eq!(value(&v, "doing.water"), Some(water.map_or(Value::None, Value::Number)), "{code}");
        assert_eq!(value(&v, "broke_at"), Some(Value::Word("working")));
    }
}

#[test]
fn a_burning_off_with_no_reason_is_a_burning_off_and_a_fault_says_what_broke() {
    let v = said(&now(Some(doing(State::BurningOff(None)))));
    assert_eq!((value(&v, "doing"), value(&v, "doing.why")), (Some(Value::Word("burning_off")), Some(Value::None)));
    let v = said(&now(Some(doing(State::Fault(Why::Unconfigured)))));
    assert_eq!((value(&v, "stage"), value(&v, "doing"), value(&v, "doing.why")), (Some(Value::Word("fault")), Some(Value::Word("fault")), Some(Value::Word("unconfigured"))));
}

#[test]
fn the_pump_says_why_it_runs_and_the_water_that_decided_it() {
    for (why, code, water) in [(PumpWhy::Water(66.0), "water", Some(66.0)), (PumpWhy::Overheated(90.0), "overheated", Some(90.0)), (PumpWhy::NoNumber, "no_number", None)] {
        let v = said(&now(Some(Doing { pump: true, pump_why: why, ..doing(State::Working) })));
        assert_eq!((value(&v, "pump"), value(&v, "pump_by")), (Some(Value::On(true)), Some(Value::Word(code))), "{code}");
        assert_eq!(value(&v, "pump_by.water"), Some(water.map_or(Value::None, Value::Number)), "{code}");
    }
}

#[test]
fn the_alarm_is_said_with_the_temperature_it_is_past_and_is_silent_otherwise() {
    let v = said(&now(Some(Doing { alarm: true, ..doing(State::Working) })));
    assert_eq!((value(&v, "alarm"), value(&v, "alarm.past")), (Some(Value::On(true)), Some(Value::Number(95.0))));
    let v = said(&now(Some(doing(State::Working))));
    assert_eq!((value(&v, "alarm"), value(&v, "alarm.past")), (Some(Value::None), Some(Value::None)));
}

#[test]
fn the_alarm_says_the_water_the_owner_set_it_for_and_not_a_number_of_the_image() {
    for set in [71.0, 90.0, 99.0] {
        let v = said(&Now { alarm_c: set, ..now(Some(Doing { alarm: true, ..doing(State::Working) })) });
        assert_eq!(value(&v, "alarm.past"), Some(Value::Number(set)), "an alarm set for {set} °C");
    }
}

#[test]
fn the_feed_and_the_mode_say_where_they_stand_in_every_state_and_before_the_first_second() {
    for state in [None, Some(State::Stop), Some(State::Working), Some(State::Fault(Why::NoFuel))] {
        let mut n = now(state.map(doing));
        n.feed_allowed = false;
        n.semi = true;
        let v = said(&n);
        assert_eq!((value(&v, "feed"), value(&v, "semi")), (Some(Value::On(false)), Some(Value::On(true))), "{state:?}");
    }
}

#[test]
fn a_sensor_that_has_gone_quiet_is_said_by_the_seconds_it_has_been_silent_and_one_that_speaks_says_nothing() {
    let mut n = now(Some(doing(State::Working)));
    n.silent = Silent { draught: None, flame: Some(7), flue: None, water: Some(31), fronts: None };
    let v = said(&n);
    assert_eq!(
        (value(&v, "silent_draught"), value(&v, "silent_flame"), value(&v, "silent_flue"), value(&v, "silent_water")),
        (Some(Value::None), Some(Value::Seconds(7)), Some(Value::None), Some(Value::Seconds(31)))
    );
}

#[test]
fn a_pin_that_gives_no_fronts_is_said_by_the_seconds_it_has_been_silent_and_one_that_gives_them_says_nothing() {
    let mut n = now(Some(doing(State::BurningOff(Some(Why::NoFronts)))));
    assert_eq!(value(&said(&n), "silent_fronts"), Some(Value::None), "it gives them");
    n.silent = Silent { fronts: Some(86), ..Silent::default() };
    let v = said(&n);
    assert_eq!(value(&v, "silent_fronts"), Some(Value::Seconds(86)));
    assert_eq!(value(&v, "doing.why"), Some(Value::Word("no_fronts")), "and the cause is named by the pin");
}

#[test]
fn the_rows_laid_later_stand_last_in_the_table_in_their_order_so_that_no_address_before_them_moves() {
    // the mains' rows, then the soot's, then the board's two rows of the load of its cores were laid after all of these: the table as it stood before them
    let all = &burner_api::ALL[..burner_api::ALL.len() - burner_api::mains::ENTRIES.len() - burner_api::soot::ENTRIES.len() - dictionary::system::CORES.len()];
    let late = ["silent_fronts", "relief_in", "relief_today", "relief_limit"];
    // and after them the term of the low pressure, a setting of the settings card, and the thermostat's two rows of the manual cycle: the last rows of the table
    for (n, name) in late.iter().enumerate() {
        let at = page::dictionary::address(all, "automaton", name).expect("the row");
        assert_eq!(at, all.len() - late.len() - 3 + n, "{name}: a row laid in the middle of a card moves every address after it");
    }
    assert_eq!(page::dictionary::address(all, "settings", "auto_low_pa_s"), Some(all.len() - 3), "the term of the low pressure stands after the automaton's late rows");
    assert_eq!(page::dictionary::address(all, "thermostat", "manual"), Some(all.len() - 2), "the manual cycle's two rows stand last");
    assert_eq!(page::dictionary::address(all, "thermostat", "manual_c"), Some(all.len() - 1));
}

#[test]
fn what_a_person_is_told_while_a_fault_waits_is_the_seconds_owed_the_faults_let_go_today_and_how_many_a_day_there_are() {
    let mut n = now(Some(doing(State::Fault(Why::NoFronts))));
    let v = said(&n);
    assert_eq!((value(&v, "relief_in"), value(&v, "relief_today"), value(&v, "relief_limit")), (Some(Value::None), Some(Value::None), Some(Value::Number(automaton::RELIEF_PER_DAY as f32))), "nothing owed and no day known: the limit alone");
    n.relief = ReliefNow { left_s: Some(37), today: Some(2) };
    let v = said(&n);
    assert_eq!((value(&v, "relief_in"), value(&v, "relief_today")), (Some(Value::Seconds(37)), Some(Value::Number(2.0))));
}

#[test]
fn the_doors_refusal_of_a_number_is_a_code_with_the_field_and_the_numbers_and_nothing_when_there_is_none() {
    // an order of two numbers: the code, the field, the number that stood there, the field it was held against and its number
    let order = Refused { field: "burn_off_s".into(), code: "not_below", value: Some(10.0), than: Some("purge_after_s"), limit: Some(20.0) };
    let mut n = now(Some(doing(State::Stop)));
    n.refused = Some(&order);
    let v = said(&n);
    assert_eq!(value(&v, "refused"), Some(Value::Word("not_below")));
    assert_eq!(v.get("refused.field"), Some(&Said::Text("burn_off_s")));
    assert_eq!(value(&v, "refused.value"), Some(Value::Number(10.0)));
    assert_eq!(v.get("refused.than"), Some(&Said::Text("purge_after_s")));
    assert_eq!(value(&v, "refused.limit"), Some(Value::Number(20.0)));
    // one number alone holds against nothing: no field to compare with, no number of it
    let alone = Refused { field: "tries".into(), code: "no_tries", value: Some(0.0), than: None, limit: None };
    n.refused = Some(&alone);
    let v = said(&n);
    assert_eq!((value(&v, "refused"), v.get("refused.field")), (Some(Value::Word("no_tries")), Some(&Said::Text("tries"))));
    assert_eq!((value(&v, "refused.value"), value(&v, "refused.than"), value(&v, "refused.limit")), (Some(Value::Number(0.0)), Some(Value::None), Some(Value::None)));
    // a number that is no number says no number
    let nan = Refused { field: "rise_s".into(), code: "not_a_number", value: None, than: None, limit: None };
    n.refused = Some(&nan);
    assert_eq!(value(&said(&n), "refused.value"), Some(Value::None));
    // nothing refused: every part says nothing, and the row too
    let v = said(&now(Some(doing(State::Stop))));
    for name in ["refused", "refused.field", "refused.value", "refused.than", "refused.limit"] {
        assert_eq!(value(&v, name), Some(Value::None), "{name}");
    }
}

#[test]
fn every_code_of_the_doors_refusals_is_a_word_the_card_lists() {
    let row = burner_api::ALL.iter().find(|t| t.name == "refused").expect("the card has a row `refused`");
    for code in automaton::REFUSALS {
        assert!(row.words.contains(&code), "the card does not list the code {code}");
    }
}

#[test]
fn every_state_says_the_same_addresses_and_the_card_never_changes_shape_with_the_stage() {
    let states = [None, Some(State::Stop), Some(State::Purge), Some(State::Kindling(1)), Some(State::Rising), Some(State::Working), Some(State::Smouldering), Some(State::BurningOff(None)), Some(State::Fault(Why::NoFuel))];
    let reference: Vec<usize> = {
        let mut at = Vec::new();
        put_values(&now(None), &mut |a, _| at.push(a));
        at.sort();
        at
    };
    assert!(reference.len() >= 36, "the first second says only {} addresses", reference.len());
    for state in states {
        let mut at = Vec::new();
        put_values(&now(state.map(doing)), &mut |a, _| at.push(a));
        at.sort();
        assert_eq!(at, reference, "{state:?}");
    }
}
