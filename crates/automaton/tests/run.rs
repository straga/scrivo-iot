//! Start and Stop, judged by the automaton the moment they are pressed.

mod common;

use automaton::{Feed, Lighting, State, Step, Why};
use common::{Automaton, Burning, Press, Seen, Silence, Tuning};
use flame::Flame;

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
        burn_out_min_s: 60.0,
        pump_on_c: 45.0,
        flue_holds_c: 100.0,
        fire_numbers: common::FireNumbers::CARD,
        want_c: 60.0,
        hysteresis_c: 4.0,
        hold: common::Hold::Boiler,
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
        step: Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

fn cold() -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0), draught_pa: Some(0.0), draught_low: false, gate: 0, silence: Silence::HEARD }
}

fn sound() -> common::Sound {
    tuning().sound().expect("sound numbers")
}

/// Seconds of nobody pressing anything until the automaton is out of its purge.
fn purge_seconds(a: &mut Automaton) -> u32 {
    let mut seconds = 0;
    while matches!(a.state(), State::Stop | State::Purge) && seconds < 100 {
        a.second(cold(), 1.0, &sound());
        seconds += 1;
    }
    seconds
}

fn pressed(a: &mut Automaton, press: Press) {
    a.press(press, cold(), &sound());
}

#[test]
fn start_from_a_stop_purges_for_the_cards_time() {
    let mut a = Automaton::new(Burning::new());
    pressed(&mut a, Press::Start);
    let whole = purge_seconds(&mut a);
    assert!(whole >= 30, "the purge took {whole} s, the card says 30");
}

#[test]
fn a_press_is_applied_when_it_is_judged_and_not_a_second_later() {
    let mut a = Automaton::new(Burning::new());
    pressed(&mut a, Press::Start);
    assert_eq!(a.state(), State::Purge, "start from a stop is a purge the moment it is taken");
    pressed(&mut a, Press::Stop);
    assert_eq!(a.state(), State::BurningOff(None), "stop in a purge is a burning off the moment it is taken");
}

#[test]
fn stop_in_a_fault_is_taken() {
    let mut a = Automaton::new(Burning::new());
    pressed(&mut a, Press::Start);
    // never lit: three attempts with no flame run the burner into a fault
    let mut seconds = 0;
    while !matches!(a.state(), State::Fault(_)) && seconds < 2000 {
        a.second(cold(), 1.0, &sound());
        seconds += 1;
    }
    assert_eq!(a.state(), State::Fault(Why::NeverLit(3)), "after {seconds} s");
    pressed(&mut a, Press::Stop);
    assert_ne!(a.state(), State::Fault(Why::NeverLit(3)), "stop left the fault standing");
}

#[test]
fn a_person_is_refused_the_igniter_the_moment_start_is_taken() {
    let mut a = Automaton::new(Burning::new());
    assert_eq!(a.by_hand("igniter"), Ok(()), "in a stop the hand is free");
    pressed(&mut a, Press::Start);
    assert!(a.by_hand("igniter").is_err(), "start is taken, and the igniter is still a person's");
}
