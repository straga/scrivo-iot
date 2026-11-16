//! What the thermostat's module hands the stream for a second: whether a person asked for heat, and what the water said of burning in the last order - each by code, the same two addresses in every state.
//! Expected from what a person is told - not asked, asked, asked away; burn, keep, enough, a level, a silent return; and "starting" before the first order - and not from the code that makes the values.

use automaton::{Held, Switch, Water};
use thermostat::Rung;
use burner_api::thermostat::{put_values, Now, Silent};
use page::dictionary::Value;
use std::collections::HashMap;

fn nothing() -> Now {
    Now { switch: Switch::Unasked, from_memory: false, water: None, rung: None, left_c: None, setpoint_c: None, lock_left_s: None, silent: Silent::default(), manual: None }
}

/// What the stream is told for NOW, by the name of the row.
fn said(now: &Now) -> HashMap<&'static str, Value> {
    let mut by_address: HashMap<usize, Value> = HashMap::new();
    put_values(now, &mut |at, value| {
        by_address.insert(at, value);
    });
    ["switch", "switch_from", "heat", "silent_return", "rung", "left", "setpoint", "lock", "manual", "manual_c"]
        .into_iter()
        .filter_map(|name| {
            let at = page::dictionary::address(&burner_api::ALL, "thermostat", name)?;
            Some((name, *by_address.get(&at)?))
        })
        .collect()
}

#[test]
fn every_state_of_the_switch_is_said_by_its_own_code() {
    for (switch, word) in [(Switch::Unasked, "unasked"), (Switch::On, "on"), (Switch::Off, "off")] {
        let v = said(&Now { switch, water: Some(Water::Keep), rung: None, left_c: None, ..nothing() });
        assert_eq!(v.get("switch"), Some(&Value::Word(word)), "{switch:?}");
    }
}

#[test]
fn every_word_of_the_water_is_said_by_its_own_code() {
    for (water, word) in [
        (Water::Burn, "burn"),
        (Water::Keep, "keep"),
        (Water::Enough, "enough"),
        (Water::Level, "level"),
        (Water::Silent(Held::Return), "silent_return"),
    ] {
        let v = said(&Now { switch: Switch::On, water: Some(water), rung: None, left_c: None, ..nothing() });
        assert_eq!(v.get("heat"), Some(&Value::Word(word)), "{water:?}");
    }
}

#[test]
fn before_its_first_order_the_thermostat_is_starting_and_still_says_where_its_switch_stands() {
    let v = said(&Now { switch: Switch::Unasked, water: None, rung: None, left_c: None, ..nothing() });
    assert_eq!(v.get("heat"), Some(&Value::Unknown("starting")));
    assert_eq!(v.get("switch"), Some(&Value::Word("unasked")));
}

#[test]
fn the_same_ten_rows_are_said_in_every_state_and_nothing_else() {
    for now in [Now { switch: Switch::Unasked, water: None, rung: None, left_c: None, ..nothing() }, Now { switch: Switch::On, water: Some(Water::Burn), rung: Some(Rung::Mid), left_c: Some(6.5), ..nothing() }, Now { switch: Switch::Off, water: Some(Water::Silent(Held::Return)), rung: None, left_c: None, ..nothing() }] {
        let v = said(&now);
        let mut names: Vec<&str> = v.keys().copied().collect();
        names.sort();
        assert_eq!(names, ["heat", "left", "lock", "manual", "manual_c", "rung", "setpoint", "silent_return", "switch", "switch_from"]);
    }
}

#[test]
fn every_level_is_said_by_its_own_code_and_no_level_is_none() {
    for (rung, word) in [(Rung::Big, "big"), (Rung::Mid, "mid"), (Rung::Small, "small")] {
        let v = said(&Now { switch: Switch::On, water: Some(Water::Keep), rung: Some(rung), left_c: Some(4.0), ..nothing() });
        assert_eq!(v.get("rung"), Some(&Value::Word(word)), "{rung:?}");
    }
    let v = said(&Now { switch: Switch::On, water: Some(Water::Level), rung: None, left_c: None, ..nothing() });
    assert_eq!(v.get("rung"), Some(&Value::Unknown("none")), "the mode of power has no levels");
}

#[test]
fn the_distance_is_said_as_a_number_of_degrees_and_none_when_the_water_said_nothing() {
    let v = said(&Now { switch: Switch::On, water: Some(Water::Keep), rung: Some(Rung::Big), left_c: Some(12.5), ..nothing() });
    assert_eq!(v.get("left"), Some(&Value::Number(12.5)));
    let v = said(&Now { switch: Switch::On, water: Some(Water::Enough), rung: Some(Rung::Small), left_c: Some(-1.0), ..nothing() });
    assert_eq!(v.get("left"), Some(&Value::Number(-1.0)), "over the setpoint the distance is below nought");
    let v = said(&Now { switch: Switch::On, water: Some(Water::Keep), rung: Some(Rung::Small), left_c: None, ..nothing() });
    assert_eq!(v.get("left"), Some(&Value::Unknown("none")));
}

// ---------------------------------------------------------------- the setpoint of the water

#[test]
fn the_setpoint_is_said_as_a_number_of_degrees_and_none_where_the_mode_holds_no_temperature() {
    let v = said(&Now { setpoint_c: Some(60.0), ..nothing() });
    assert_eq!(v.get("setpoint"), Some(&Value::Number(60.0)));
    let v = said(&Now { setpoint_c: None, ..nothing() });
    assert_eq!(v.get("setpoint"), Some(&Value::Unknown("none")), "the mode of power holds no temperature and so has no setpoint");
}

// ---------------------------------------------------------------- what is left of the wait before a light

#[test]
fn what_is_left_of_the_wait_is_said_in_whole_seconds_rounded_up_so_a_page_never_says_nought_while_one_is_left() {
    let v = said(&Now { lock_left_s: Some(4999.2), ..nothing() });
    assert_eq!(v.get("lock"), Some(&Value::Seconds(5000)));
    let v = said(&Now { lock_left_s: Some(0.4), ..nothing() });
    assert_eq!(v.get("lock"), Some(&Value::Seconds(1)));
}

#[test]
fn where_there_is_no_wait_the_row_is_none_and_the_same_address_is_still_said() {
    let v = said(&Now { lock_left_s: None, ..nothing() });
    assert_eq!(v.get("lock"), Some(&Value::None));
}

#[test]
fn the_return_is_said_by_the_seconds_of_its_silence_and_by_nothing_while_it_speaks() {
    // the silence of the sensor the thermostat holds by stands on its card
    let v = said(&Now { silent: Silent { return_c: Some(12) }, ..nothing() });
    assert_eq!(v.get("silent_return"), Some(&Value::Seconds(12)));
    let v = said(&Now { silent: Silent::default(), ..nothing() });
    assert_eq!(v.get("silent_return"), Some(&Value::None), "a sensor that speaks says nothing");
}

#[test]
fn a_sensor_no_role_is_given_to_is_not_silent_it_is_not_there_and_a_role_given_is_silent_by_the_seconds() {
    assert_eq!(Silent::said(Some(9), true), Some(9));
    assert_eq!(Silent::said(Some(9), false), None, "a role nobody plays is no silence");
    assert_eq!(Silent::said(None, true), None, "a sensor that speaks says nothing");
    assert_eq!(Silent::said(None, false), None);
}

#[test]
fn who_set_the_switch_is_the_memory_after_a_restart_and_a_person_after_a_press() {
    for (switch, from_memory, word) in [(Switch::On, true, "memory"), (Switch::Off, true, "memory"), (Switch::On, false, "person"), (Switch::Off, false, "person")] {
        let v = said(&Now { switch, from_memory, ..nothing() });
        assert_eq!(v.get("switch_from"), Some(&Value::Word(word)), "{switch:?} from memory {from_memory}");
    }
}

#[test]
fn a_switch_nobody_set_says_nobody_did() {
    let v = said(&Now { switch: Switch::Unasked, from_memory: false, ..nothing() });
    assert_eq!(v.get("switch_from"), Some(&Value::Unknown("none")));
}

// ---------------------------------------------------------------- the manual cycle a Light began

#[test]
fn a_manual_cycle_is_said_as_what_it_is_carried_to_and_the_degrees_it_ends_at_and_none_where_there_is_none() {
    use thermostat::{Cycle, Nominal};
    let v = said(&Now { manual: Some(Cycle { nominal: Nominal::Room, to_c: 22.0 }), ..nothing() });
    assert_eq!((v.get("manual"), v.get("manual_c")), (Some(&Value::Word("room")), Some(&Value::Number(22.0))));
    let v = said(&Now { manual: Some(Cycle { nominal: Nominal::Water, to_c: 60.0 }), ..nothing() });
    assert_eq!((v.get("manual"), v.get("manual_c")), (Some(&Value::Word("water")), Some(&Value::Number(60.0))));
    let v = said(&nothing());
    assert_eq!((v.get("manual"), v.get("manual_c")), (Some(&Value::Unknown("none")), Some(&Value::Unknown("none"))), "no cycle: the same two addresses in every state");
}
