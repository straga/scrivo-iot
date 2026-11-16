//! What the automaton says in the journal: a change of state is an event and goes at `info`; what the organs do inside one state - the auger on and off at every dose, 44 lines in ten minutes of fire - is a detail, and goes at `debug`.
//! The decision, not the wording: the wording is the device's.

use automaton::voice::{draught_line, voice, Voice};
use automaton::{DraughtCounts, Heard, Seen, Silence, State, Why};
use flame::Flame;

#[test]
fn a_change_of_state_is_an_event_whatever_the_organs_say() {
    assert_eq!(voice(Some(State::Stop), State::Purge, false), Voice::Event);
    assert_eq!(voice(Some(State::Working), State::Smouldering, true), Voice::Event);
}

#[test]
fn the_first_second_of_all_is_an_event() {
    assert_eq!(voice(None, State::Stop, false), Voice::Event);
}

#[test]
fn the_organs_changing_inside_one_state_are_a_detail() {
    assert_eq!(voice(Some(State::Working), State::Working, true), Voice::Detail);
}

#[test]
fn nothing_changing_says_nothing() {
    assert_eq!(voice(Some(State::Working), State::Working, false), Voice::Silent);
}

fn seen(draught_pa: Option<f32>, draught_low: bool, draught: Heard) -> Seen {
    Seen { flame: Flame::Steady, boiler_c: Some(40.0), flue_c: Some(150.0), draught_pa, draught_low, gate: 33, silence: Silence { draught, ..Silence::HEARD } }
}

const COUNTS: DraughtCounts = DraughtCounts { held_s: 11.0, no_number_s: 7.0, low_s: 21.0 };

#[test]
fn a_burning_off_by_the_draught_says_what_the_sensor_said_and_for_how_long() {
    let line = draught_line(Why::LowPressure, &seen(None, true, Heard::Speaks), COUNTS, Some(247)).expect("low pressure is a burning off by the draught");
    for must in ["low_pressure", "under its zero", "247 mV", "held 11 s", "no number 7 s", "under zero 21 s"] {
        assert!(line.contains(must), "{must:?} is not in {line:?}");
    }
}

#[test]
fn the_line_tells_a_sensor_that_is_gone_from_one_that_is_silent_and_from_one_that_gave_a_number() {
    let gone = draught_line(Why::NoDraught, &seen(None, false, Heard::Speaks), COUNTS, None).unwrap();
    assert!(gone.contains("no_draught") && gone.contains("no number") && !gone.contains("under its zero") && !gone.contains("silent"), "{gone:?}");
    let silent = draught_line(Why::DraughtSilent, &seen(None, false, Heard::TooLong), COUNTS, None).unwrap();
    assert!(silent.contains("draught_silent") && silent.contains("silent"), "{silent:?}");
    // a burning off whose cause is not what the sensor said in this second: the line says the number that stood, and so shows the cause is elsewhere
    let number = draught_line(Why::NoDraught, &seen(Some(68.0), false, Heard::Speaks), COUNTS, Some(703)).unwrap();
    assert!(number.contains("68 Pa") && number.contains("703 mV"), "{number:?}");
}

#[test]
fn a_burning_off_for_any_other_reason_has_no_line_of_the_draught() {
    for why in [Why::NeverLit(3), Why::FlameLost, Why::NoFuel, Why::Overheated(90.0), Why::NoFronts, Why::FlameSilent] {
        assert_eq!(draught_line(why, &seen(None, true, Heard::Speaks), COUNTS, Some(247)), None, "{why:?}");
    }
}
