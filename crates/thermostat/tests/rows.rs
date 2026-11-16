//! What the thermostat orders on rows of readings and presses, and no board anywhere: every word of the water has a row at its
//! edge and one on the far side of it, and the switch has its own.
//!
//! **THE EXPECTATIONS ARE THE BURNER'S, NOT THE CODE'S**: each row says what the fire did before the decision was taken out of it
//! (crates/automaton tests/rows.rs, the water's rows) - burn at or under the setpoint less the hysteresis, nothing inside it,
//! enough at the setpoint, a level and no water in the mode that holds none, and the held sensor's silence as the sensor's.

use automaton::{Feed, Heard, Presses, Step, Switch, Water, Held};
use thermostat::{Fire, Hold, Lockout, Press, Temperatures, Thermostat, Tuning};

const WANT: f32 = 60.0;
const DROP: f32 = 4.0;

fn step() -> Step {
    Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } }
}

fn power() -> Step {
    Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } }
}

fn tuning(hold: Hold) -> Tuning {
    Tuning { hold, want_c: WANT, hysteresis_c: DROP, step: step(), mid: step(), small: step(), mid_c: 8.0, small_c: 3.0, power: power(), lockout: Lockout::default() }
}

fn sound(hold: Hold) -> thermostat::Sound {
    tuning(hold).sound(88.0).expect("the numbers are sound")
}

/// Every sensor speaking, the boiler at C and the other two far from anything a row asks.
fn at(boiler_c: f32) -> Temperatures {
    Temperatures { boiler_c: Some(boiler_c), return_c: Some(30.0), return_heard: Heard::Speaks, asked: None, room_c: None, room_target_c: None, fire: Fire::default() }
}

fn water(hold: Hold, readings: Temperatures) -> Water {
    Thermostat::new().order(&readings, &sound(hold)).water
}

// ---------------------------------------------------------------- the water, by the boiler

#[test]
fn water_at_the_setpoint_less_the_hysteresis_is_burn() {
    assert_eq!(water(Hold::Boiler, at(WANT - DROP)), Water::Burn, "56 is the line itself: the fire lights on it");
    assert_eq!(water(Hold::Boiler, at(30.0)), Water::Burn);
}

#[test]
fn water_just_over_the_line_is_inside_the_hysteresis_and_orders_nothing() {
    assert_eq!(water(Hold::Boiler, at(WANT - DROP + 0.1)), Water::Keep);
    assert_eq!(water(Hold::Boiler, at(58.0)), Water::Keep);
    assert_eq!(water(Hold::Boiler, at(WANT - 0.1)), Water::Keep);
}

#[test]
fn water_at_the_setpoint_is_enough() {
    assert_eq!(water(Hold::Boiler, at(WANT)), Water::Enough, "60 is the line itself: the fire goes down on it");
    assert_eq!(water(Hold::Boiler, at(75.0)), Water::Enough);
}

#[test]
fn a_boiler_nobody_can_read_orders_nothing_and_is_never_taken_for_cold() {
    let blind = Temperatures { boiler_c: None, ..at(0.0) };
    assert_eq!(water(Hold::Boiler, blind), Water::Keep, "a missing temperature is not a cold one, and not a hot one either");
}

// ---------------------------------------------------------------- the water, by the return

#[test]
fn the_return_is_held_by_its_own_reading_and_the_boiler_is_not_asked() {
    let readings = Temperatures { return_c: Some(50.0), ..at(90.0) };
    assert_eq!(water(Hold::Return, readings), Water::Burn, "the boiler is far past the setpoint and the mode does not look at it");
    assert_eq!(water(Hold::Return, Temperatures { return_c: Some(WANT), ..at(10.0) }), Water::Enough);
    assert_eq!(water(Hold::Return, Temperatures { return_c: Some(58.0), ..at(10.0) }), Water::Keep);
}

#[test]
fn a_held_sensor_that_says_nothing_orders_nothing() {
    assert_eq!(water(Hold::Return, Temperatures { return_c: None, return_heard: Heard::Silent, ..at(10.0) }), Water::Keep);
}

// ---------------------------------------------------------------- the silence of the held sensor

#[test]
fn a_return_silent_too_long_is_the_returns_silence_while_the_burner_holds_the_return() {
    let readings = Temperatures { return_c: None, return_heard: Heard::TooLong, ..at(40.0) };
    assert_eq!(water(Hold::Return, readings), Water::Silent(Held::Return));
}

#[test]
fn a_sensor_the_mode_does_not_hold_is_not_the_modes_business_however_long_it_is_silent() {
    let gone = Temperatures { return_c: None, return_heard: Heard::TooLong, ..at(70.0) };
    assert_eq!(water(Hold::Boiler, gone), Water::Enough, "the boiler's own water decides, and the silent return is not asked");
    assert_eq!(water(Hold::Power, gone), Water::Level);
}

// ---------------------------------------------------------------- the mode that holds no temperature

#[test]
fn the_mode_of_power_orders_the_level_whatever_the_water_says() {
    for boiler_c in [Some(10.0), Some(58.0), Some(75.0), None] {
        assert_eq!(water(Hold::Power, Temperatures { boiler_c, ..at(0.0) }), Water::Level, "the water is {boiler_c:?} and no part of it");
    }
}

// ---------------------------------------------------------------- the level

#[test]
fn the_level_is_the_table_step_in_a_mode_by_a_sensor_and_the_chosen_power_in_the_mode_of_power() {
    for hold in [Hold::Boiler, Hold::Return] {
        assert_eq!(Thermostat::new().order(&at(50.0), &sound(hold)).step, step(), "{hold:?}");
    }
    assert_eq!(Thermostat::new().order(&at(50.0), &sound(Hold::Power)).step, power());
}

// ---------------------------------------------------------------- the switch and the presses

#[test]
fn a_thermostat_that_has_just_come_up_has_not_been_asked_anything() {
    let mut thermostat = Thermostat::new();
    let order = thermostat.order(&at(50.0), &sound(Hold::Boiler));
    assert_eq!(order.switch, Switch::Unasked);
    assert_eq!(order.presses, Presses { start: 0, stop: 0, light: 0 });
}

#[test]
fn start_asks_and_stop_withdraws_and_each_press_is_counted() {
    let mut thermostat = Thermostat::new();
    let t = sound(Hold::Boiler);
    thermostat.press(Press::Start);
    let started = thermostat.order(&at(50.0), &t);
    assert_eq!((started.switch, started.presses), (Switch::On, Presses { start: 1, stop: 0, light: 0 }));
    thermostat.press(Press::Stop);
    let stopped = thermostat.order(&at(50.0), &t);
    assert_eq!((stopped.switch, stopped.presses), (Switch::Off, Presses { start: 1, stop: 1, light: 0 }));
    thermostat.press(Press::Start);
    let again = thermostat.order(&at(50.0), &t);
    assert_eq!((again.switch, again.presses), (Switch::On, Presses { start: 2, stop: 1, light: 0 }));
}

#[test]
fn a_stop_pressed_on_a_board_nobody_asked_is_still_a_stop_and_still_counted() {
    let mut thermostat = Thermostat::new();
    thermostat.press(Press::Stop);
    let order = thermostat.order(&at(50.0), &sound(Hold::Boiler));
    assert_eq!((order.switch, order.presses), (Switch::Off, Presses { start: 0, stop: 1, light: 0 }));
}

#[test]
fn the_switch_and_the_presses_are_the_same_in_every_mode_and_every_water() {
    for hold in [Hold::Power, Hold::Boiler, Hold::Return] {
        let mut thermostat = Thermostat::new();
        thermostat.press(Press::Start);
        for boiler_c in [10.0, 58.0, 80.0] {
            let order = thermostat.order(&at(boiler_c), &sound(hold));
            assert_eq!((order.switch, order.presses.start), (Switch::On, 1), "{hold:?} at {boiler_c}");
        }
    }
}

// ---------------------------------------------------------------- the edge a count makes

#[test]
fn the_edge_between_two_counts_is_the_button_that_moved() {
    use automaton::Told;
    let none = Presses { start: 0, stop: 0, light: 0 };
    assert_eq!(Presses { start: 1, stop: 0, light: 0 }.since(none), Told::Start);
    assert_eq!(Presses { start: 0, stop: 1, light: 0 }.since(none), Told::Stop);
    assert_eq!(none.since(none), Told::Nothing);
    assert_eq!(Presses { start: 3, stop: 2, light: 0 }.since(Presses { start: 3, stop: 2, light: 0 }), Told::Nothing);
    assert_eq!(Presses { start: 4, stop: 2, light: 0 }.since(Presses { start: 3, stop: 2, light: 0 }), Told::Start);
}

#[test]
fn both_buttons_between_two_seconds_are_a_stop_because_stop_beats_every_source() {
    use automaton::Told;
    assert_eq!(Presses { start: 1, stop: 1, light: 0 }.since(Presses { start: 0, stop: 0, light: 0 }), Told::Stop);
}

// ---------------------------------------------------------------- the door

#[test]
fn numbers_that_are_not_a_setpoint_and_a_hysteresis_under_it_are_refused_by_name() {
    let t = tuning(Hold::Boiler);
    for (field, bad, code) in [
        ("want_c", thermostat::Tuning { want_c: 0.0, ..t }, "above_nought"),
        ("want_c", thermostat::Tuning { want_c: f32::NAN, ..t }, "not_a_number"),
        ("hysteresis_c", thermostat::Tuning { hysteresis_c: 0.0, ..t }, "above_nought"),
        ("hysteresis_c", thermostat::Tuning { hysteresis_c: 60.0, ..t }, "below"),
        ("hysteresis_c", thermostat::Tuning { hysteresis_c: f32::NAN, ..t }, "not_a_number"),
    ] {
        let refused = bad.sound(88.0).expect_err(field);
        assert_eq!((refused.field.as_str(), refused.code), (field, code));
    }
}

#[test]
fn a_setpoint_the_overheating_does_not_stand_above_is_refused_by_the_setpoint_it_is_held_against() {
    let refused = tuning(Hold::Boiler).sound(60.0).expect_err("an overheating at the setpoint can never be held");
    assert_eq!((refused.field.as_str(), refused.code, refused.than, refused.limit), ("too_hot_c", "above", Some("want_c"), Some(WANT)));
    assert!(tuning(Hold::Boiler).sound(60.1).is_ok(), "and a hair above the setpoint is above it");
}

#[test]
fn a_level_nobody_can_burn_on_is_refused_whichever_mode_holds_it_at_the_moment() {
    let t = tuning(Hold::Boiler);
    for (field, bad) in [
        ("step.feed.on_s", Tuning { step: Step { feed: Feed { on_s: 0.0, ..t.step.feed }, ..t.step }, ..t }),
        ("power.feed.off_s", Tuning { power: Step { feed: Feed { off_s: 0.0, ..t.power.feed }, ..t.power }, ..t }),
        ("power.pa", Tuning { power: Step { pa: -1.0, ..t.power }, ..t }),
    ] {
        let refused = bad.sound(88.0).expect_err(field);
        assert_eq!(refused.field, field);
    }
    assert!(Tuning { step: Step { pa: 0.0, ..t.step }, ..t }.sound(88.0).is_ok(), "nought is a level too");
}

// ---------------------------------------------------------------- Start switches on, Light lights

#[test]
fn a_start_is_only_the_switch_and_a_light_is_counted_on_its_own() {
    let mut thermostat = Thermostat::new();
    let t = sound(Hold::Boiler);
    thermostat.press(Press::Start);
    let started = thermostat.order(&at(50.0), &t);
    assert_eq!((started.switch, started.presses), (Switch::On, Presses { start: 1, stop: 0, light: 0 }), "a Start asks for heat and lights nothing");
    thermostat.press(Press::Light);
    let lit = thermostat.order(&at(50.0), &t);
    assert_eq!((lit.switch, lit.presses), (Switch::On, Presses { start: 1, stop: 0, light: 1 }), "a Light is a count of its own and leaves the switch where it stood");
}

#[test]
fn the_edge_of_a_light_is_a_light_and_a_start_and_a_stop_keep_theirs() {
    use automaton::Told;
    let none = Presses::default();
    assert_eq!(Presses { start: 0, stop: 0, light: 1 }.since(none), Told::Light);
    assert_eq!(Presses { start: 0, stop: 0, light: 2 }.since(Presses { start: 0, stop: 0, light: 1 }), Told::Light);
    assert_eq!(Presses { start: 0, stop: 0, light: 2 }.since(Presses { start: 0, stop: 0, light: 2 }), Told::Nothing);
    assert_eq!(Presses { start: 1, stop: 0, light: 0 }.since(none), Told::Start, "a Start alone is still a Start");
}

#[test]
fn a_light_beats_a_start_and_a_stop_beats_both_between_two_seconds() {
    use automaton::Told;
    let none = Presses::default();
    assert_eq!(Presses { start: 1, stop: 0, light: 1 }.since(none), Told::Light, "the fire lights on either, and the light is the one that says so");
    assert_eq!(Presses { start: 1, stop: 1, light: 1 }.since(none), Told::Stop, "Stop beats every source");
    assert_eq!(Presses { start: 0, stop: 1, light: 1 }.since(none), Told::Stop);
}
