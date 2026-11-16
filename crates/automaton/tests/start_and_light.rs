//! "Start" switches the burner on and the water lights it; "light now" lights it past the water. The real thermostat orders the real fire, as a board runs them,
//! and a row is a person's press and the readings of the seconds that follow.
//!
//! **THE EXPECTATIONS ARE THE BURNER'S, NOT THE CODE'S**: a person who presses Start on a warm room finds the switch on and the fire out; the room falling cold lights it by itself; the button
//! that lights at once is a button of its own, and it is the one the pause after a fire went out and the first round of the sensors after a restart do not hold back.

mod common;

use automaton::{Doing, Feed, Lighting, State, Step, Told};
use common::{Burning, Hold, Seen, Silence, Tuning};
use demand::{Asked, Demand, Why};
use flame::Flame;

/// The pause after a fire went out, in seconds: two hours, with the floor of the boiler's water at 30 °C.
const PAUSE_S: f32 = 7200.0;

fn tuning() -> Tuning {
    Tuning {
        purge_before_s: 30.0,
        purge_after_s: 20.0,
        igniter_s: 30.0,
        wait_for_flame_s: 90.0,
        tries: 3,
        dose_after_first: 0.1,
        rise_s: 60.0,
        flame_lost_s: 120.0,
        smoulder_s: 300.0,
        burn_off_s: 120.0,
        burn_out_min_s: 0.0,
        pump_on_c: 45.0,
        flue_holds_c: 100.0,
        fire_numbers: common::FireNumbers::CARD,
        want_c: 60.0,
        hysteresis_c: 4.0,
        hold: Hold::Boiler,
        power: Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } },
        pump_off_below_c: 4.0,
        too_hot_c: 88.0,
        alarm_c: 95.0,
        sure_s: 10.0,
        relight_s: [20.0, 10.0, 10.0],
        relight_hot_c: 45.0,
        spin_up_s: 10.0,
        low_pressure_s: 20.0,
        kindling: Lighting { pa: 51.0, dose_s: 32.0 },
        step: Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

fn sound() -> common::Sound {
    tuning().sound().expect("the numbers are sound")
}

/// The same numbers with a pause of two hours after a fire went out.
fn sound_with_pause() -> common::Sound {
    tuning().sound_with(thermostat::Lockout { wait_s: PAUSE_S, floor_c: 30.0, sure_s: 0.0 }).expect("the numbers are sound")
}

/// No fire, the boiler at C: 40 is under the line of 56 (the water says burn), 70 is over the setpoint of 60 (enough), 58 is inside the hysteresis (keep).
fn water(boiler_c: f32) -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(boiler_c), return_c: None, flue_c: Some(20.0), draught_pa: Some(0.0), draught_low: false, gate: 0, silence: Silence::HEARD }
}

/// A fire that burns, the boiler at C.
fn burning_at(boiler_c: f32) -> Seen {
    Seen { flame: Flame::Steady, boiler_c: Some(boiler_c), return_c: None, flue_c: Some(150.0), draught_pa: Some(50.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

fn warm() -> Option<Asked> {
    Some(Asked { demand: Demand::Warm, why: Why::TargetReached })
}

fn cold_room() -> Option<Asked> {
    Some(Asked { demand: Demand::Cold, why: Why::BelowTarget })
}

fn second(burning: &mut Burning, told: Told, seen: Seen, t: &common::Sound) -> Doing {
    burning.step(told, seen, 1.0, t)
}

/// SECONDS of nobody pressing anything, every one of them asserted to be in STATE.
fn stays(burning: &mut Burning, state: State, seen: Seen, seconds: usize, t: &common::Sound) {
    for n in 1..=seconds {
        assert_eq!(second(burning, Told::Nothing, seen, t).state, state, "second {n} of {seconds}");
    }
}

// ---------------------------------------------------------------- a start switches on

#[test]
fn a_start_with_the_room_warm_switches_on_and_lights_nothing() {
    let t = sound();
    let mut burning = Burning::new();
    burning.ask(warm());
    assert_eq!(second(&mut burning, Told::Start, water(40.0), &t).state, State::Stop, "the press is the switch, and a warm room asks nothing of the water");
    assert!(burning.wanted(), "and the switch is on");
    stays(&mut burning, State::Stop, water(40.0), 600, &t);
}

#[test]
fn a_start_with_the_water_over_the_setpoint_switches_on_and_lights_nothing() {
    let t = sound();
    let mut burning = Burning::new();
    assert_eq!(second(&mut burning, Told::Start, water(70.0), &t).state, State::Stop);
    assert!(burning.wanted());
    stays(&mut burning, State::Stop, water(70.0), 600, &t);
}

#[test]
fn a_start_with_water_that_burns_lights_the_fire() {
    let t = sound();
    let mut burning = Burning::new();
    burning.ask(cold_room());
    assert_eq!(second(&mut burning, Told::Start, water(40.0), &t).state, State::Purge, "the switch is on and the water says burn: the thermostat lights it");
}

#[test]
fn a_switched_on_stop_lights_by_itself_when_the_room_falls_cold() {
    let t = sound();
    let mut burning = Burning::new();
    burning.ask(warm());
    second(&mut burning, Told::Start, water(40.0), &t);
    stays(&mut burning, State::Stop, water(40.0), 60, &t);
    burning.ask(cold_room());
    assert_eq!(second(&mut burning, Told::Nothing, water(40.0), &t).state, State::Purge, "the room fell out of its band: the fire is lit with nobody pressing anything");
}

#[test]
fn a_switched_on_stop_lights_by_itself_when_the_water_falls_to_the_line() {
    let t = sound();
    let mut burning = Burning::new();
    second(&mut burning, Told::Start, water(70.0), &t);
    stays(&mut burning, State::Stop, water(58.0), 60, &t);
    assert_eq!(second(&mut burning, Told::Nothing, water(56.0), &t).state, State::Purge, "56 is the line itself: the fire lights on it");
}

// ---------------------------------------------------------------- a light lights

#[test]
fn a_light_lights_a_switched_on_stop_whose_room_is_warm_and_whose_water_is_enough() {
    let t = sound();
    // the water under its setpoint of 60, in the band or under it: the light lights past the thermostat (over the setpoint it is refused, 30.10: crates/automaton/tests/rows.rs)
    for boiler_c in [40.0, 58.0] {
        let mut burning = Burning::new();
        burning.ask(warm());
        second(&mut burning, Told::Start, water(boiler_c), &t);
        stays(&mut burning, State::Stop, water(boiler_c), 30, &t);
        assert_eq!(second(&mut burning, Told::Light, water(boiler_c), &t).state, State::Purge, "water at {boiler_c}: the light lights past the thermostat");
    }
}

// ---------------------------------------------------------------- after a restart: the first round

#[test]
fn a_start_after_a_restart_lights_nothing_before_the_first_round_and_the_water_lights_after_it() {
    let t = sound();
    let mut burning = Burning::new();
    burning.wait_for_first_round();
    assert_eq!(second(&mut burning, Told::Start, water(40.0), &t).state, State::Stop, "the sensors have not been asked all round once: a Start is not a light");
    stays(&mut burning, State::Stop, water(40.0), 30, &t);
    burning.first_round_done();
    assert_eq!(second(&mut burning, Told::Nothing, water(40.0), &t).state, State::Purge, "the round is over, the switch is on and the water says burn");
}

#[test]
fn a_light_after_a_restart_lights_before_the_first_round() {
    let t = sound();
    let mut burning = Burning::new();
    burning.wait_for_first_round();
    assert_eq!(second(&mut burning, Told::Light, water(40.0), &t).state, State::Purge, "a light is a person's word, and the barrier does not hold it");
}

// ---------------------------------------------------------------- the pause after a fire went out

/// A fire lit by a Start on water that burns, taken to work, sent down on hot water and left to burn out by its own time: it ends in a stop with the switch still on, and the pause begins
/// with it. The boiler then cools to 40, which would light it at once but for the pause.
fn into_the_pause(burning: &mut Burning, t: &common::Sound) {
    burning.wait_for_first_round();
    burning.first_round_done();
    second(burning, Told::Start, water(40.0), t);
    for _ in 0..30 {
        second(burning, Told::Nothing, water(40.0), t);
    }
    for _ in 0..92 {
        second(burning, Told::Nothing, burning_at(40.0), t);
    }
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach work");
    second(burning, Told::Nothing, burning_at(70.0), t);
    assert_eq!(burning.state(), State::Smouldering, "the scaffolding did not reach smouldering");
    for _ in 0..301 {
        second(burning, Told::Nothing, burning_at(70.0), t);
    }
    assert_eq!(burning.state(), State::BurningOff(None), "the scaffolding did not reach the burning off");
    let mut seconds = 0;
    while burning.state() != State::Stop && seconds < 200 {
        second(burning, Told::Nothing, water(70.0), t);
        seconds += 1;
    }
    assert_eq!(burning.state(), State::Stop, "the scaffolding did not reach a stop");
    assert!(burning.wanted(), "the scaffolding: the switch is still on");
    // the pause holds: the water has cooled under the line, the switch is on, and nothing lights it
    stays(burning, State::Stop, water(40.0), 600, t);
}

#[test]
fn a_start_in_the_pause_does_not_cut_it() {
    let t = sound_with_pause();
    let mut burning = Burning::new();
    into_the_pause(&mut burning, &t);
    assert_eq!(second(&mut burning, Told::Start, water(40.0), &t).state, State::Stop, "a Start is the switch: the pause goes on");
    stays(&mut burning, State::Stop, water(40.0), 600, &t);
}

#[test]
fn a_light_in_the_pause_cuts_it_and_lights() {
    let t = sound_with_pause();
    let mut burning = Burning::new();
    into_the_pause(&mut burning, &t);
    assert_eq!(second(&mut burning, Told::Light, water(40.0), &t).state, State::Purge, "the light is the person's word, and the one thing that lights before the time");
}

// ---------------------------------------------------------------- a burning off and a fault

/// A fire lit by a Start on water that burns and taken to work.
fn at_work(t: &common::Sound) -> Burning {
    let mut burning = Burning::new();
    second(&mut burning, Told::Start, water(40.0), t);
    for _ in 0..30 {
        second(&mut burning, Told::Nothing, water(40.0), t);
    }
    for _ in 0..92 {
        second(&mut burning, Told::Nothing, burning_at(40.0), t);
    }
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach work");
    burning
}

#[test]
fn a_start_in_a_burning_off_switches_on_and_the_burning_off_runs_its_course() {
    let t = sound();
    let mut burning = at_work(&t);
    assert_eq!(second(&mut burning, Told::Stop, burning_at(40.0), &t).state, State::BurningOff(None));
    assert!(!burning.wanted(), "the scaffolding: a Stop withdrew the heat");
    assert!(matches!(second(&mut burning, Told::Start, burning_at(40.0), &t).state, State::BurningOff(_)), "a Start does not interrupt a burning off");
    assert!(burning.wanted(), "and it asks for heat");
    for n in 1..=5 {
        assert!(matches!(second(&mut burning, Told::Nothing, burning_at(40.0), &t).state, State::BurningOff(_)), "second {n}");
    }
}

#[test]
fn a_light_in_a_burning_off_interrupts_nothing() {
    let t = sound();
    let mut burning = at_work(&t);
    second(&mut burning, Told::Stop, burning_at(40.0), &t);
    assert!(matches!(second(&mut burning, Told::Light, burning_at(40.0), &t).state, State::BurningOff(_)), "there is fuel in the pot and the only way out is through");
}

/// A fire lit by a Start that never lights: three attempts with no flame end in a fault.
fn into_a_fault(t: &common::Sound) -> Burning {
    let mut burning = Burning::new();
    second(&mut burning, Told::Start, water(40.0), t);
    let mut seconds = 0;
    while !matches!(burning.state(), State::Fault(_)) && seconds < 3000 {
        second(&mut burning, Told::Nothing, water(40.0), t);
        seconds += 1;
    }
    assert!(matches!(burning.state(), State::Fault(_)), "the scaffolding did not reach a fault after {seconds} s");
    burning
}

#[test]
fn a_start_in_a_fault_switches_on_and_only_a_stop_answers_the_fault() {
    let t = sound();
    let mut burning = into_a_fault(&t);
    assert!(matches!(second(&mut burning, Told::Start, water(40.0), &t).state, State::Fault(_)), "a Start does not answer a fault");
    assert!(burning.wanted(), "and it asks for heat");
    stays_in_a_fault(&mut burning, 30, &t);
    assert_eq!(second(&mut burning, Told::Stop, water(40.0), &t).state, State::Stop, "a press of Stop does");
}

#[test]
fn a_light_in_a_fault_lights_nothing() {
    let t = sound();
    let mut burning = into_a_fault(&t);
    assert!(matches!(second(&mut burning, Told::Light, water(40.0), &t).state, State::Fault(_)), "a fault waits for a person's Stop and for nothing else");
    stays_in_a_fault(&mut burning, 30, &t);
}

fn stays_in_a_fault(burning: &mut Burning, seconds: usize, t: &common::Sound) {
    for n in 1..=seconds {
        assert!(matches!(second(burning, Told::Nothing, water(40.0), t).state, State::Fault(_)), "second {n} of {seconds}");
    }
}
