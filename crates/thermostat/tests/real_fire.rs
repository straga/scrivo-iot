//! The wait before a light starts from a fire that was a fire: the flame sensor saw a steady flame, and then the burning ended. A kindling that did not take, a fault with no flame in it, a fault the
//! board came up with from its memory, a flame that was only weak - none of them is a fire that went out, and none starts a wait.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: 20.10 a burner came up with a fault in its memory, was let out of it into a stop, and waited 1:59:33 for a flame it never had. A wait is for
//! a boiler that burned and must rest; the end of burning is still the second the automaton stands in Stop after it did not, and the count still runs from there.
//!
//! Numbers: setpoint 60, hysteresis 4 (the water lights the fire at 56 and under), the wait two hours, the floor 30, the boiler at 50.

use automaton::{Feed, Heard, Step, Water};
use thermostat::{Fire, Hold, Lockout, Press, Temperatures, Thermostat, Tuning};

const WAIT: f32 = 7200.0;
/// Seconds of a steady flame without a break that make it a fire: the automaton's `sure_s`.
const SURE: f32 = 5.0;

fn step() -> Step {
    Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } }
}

fn sound() -> thermostat::Sound {
    Tuning { hold: Hold::Boiler, want_c: 60.0, hysteresis_c: 4.0, step: step(), mid: step(), small: step(), mid_c: 8.0, small_c: 3.0, power: step(), lockout: Lockout { wait_s: WAIT, floor_c: 30.0, sure_s: SURE } }
        .sound(88.0)
        .expect("the numbers are sound")
}

/// One second of the thermostat over water at 50 (it asks for a light): the automaton in Stop or not, the flame sensor steady or not.
fn second(t: &mut Thermostat, out: bool, steady: bool) -> Water {
    let readings = Temperatures { boiler_c: Some(50.0), return_c: Some(30.0), return_heard: Heard::Speaks, asked: None, room_c: None, room_target_c: None, fire: Fire { out, steady, elapsed_s: 1.0 } };
    t.order(&readings, &sound()).water
}

/// A board that came up and looked at the burner in Stop.
fn standing() -> Thermostat {
    let mut t = Thermostat::new();
    second(&mut t, true, false);
    t
}

#[test]
fn a_kindling_that_never_took_and_ended_in_a_stop_starts_no_wait() {
    let mut t = standing();
    for _ in 0..30 {
        second(&mut t, false, false); // purge, kindling: the automaton is not in Stop and the sensor sees no steady flame
    }
    assert_eq!(second(&mut t, true, false), Water::Burn, "the water asks for a light and no wait holds it");
    assert_eq!(t.lock_left_s(), None);
}

#[test]
fn a_fault_with_no_flame_in_it_that_ends_in_a_stop_starts_no_wait() {
    let mut t = standing();
    for _ in 0..600 {
        second(&mut t, false, false);
    }
    second(&mut t, true, false);
    assert_eq!(t.lock_left_s(), None);
}

#[test]
fn a_board_that_came_up_in_a_fault_from_its_memory_and_was_let_out_of_it_starts_no_wait() {
    // the first look finds the automaton in a fault, not in Stop: the board has seen no stop and no flame
    let mut t = Thermostat::new();
    for _ in 0..60 {
        second(&mut t, false, false);
    }
    assert_eq!(second(&mut t, true, false), Water::Burn);
    assert_eq!(t.lock_left_s(), None, "no flame this run: nothing burned that must rest");
}

#[test]
fn a_flame_that_was_only_weak_is_no_fire() {
    let mut t = standing();
    for _ in 0..60 {
        second(&mut t, false, false); // a weak flame is not steady: the sensor says steady false
    }
    second(&mut t, true, false);
    assert_eq!(t.lock_left_s(), None);
}

#[test]
fn a_fire_that_burned_steady_and_went_out_starts_the_wait_as_it_did() {
    let mut t = standing();
    for _ in 0..30 {
        second(&mut t, false, false);
    }
    for _ in 0..600 {
        second(&mut t, false, true);
    }
    assert_eq!(second(&mut t, true, false), Water::Keep, "the fire burned and is out: the light is withheld");
    assert_eq!(t.lock_left_s(), Some(WAIT));
}

#[test]
fn the_wait_runs_from_the_stop_and_not_from_the_moment_the_flame_was_lost() {
    let mut t = standing();
    for _ in 0..300 {
        second(&mut t, false, true);
    }
    for _ in 0..120 {
        second(&mut t, false, false); // the flame is gone and the burner is burning off: not yet out
    }
    second(&mut t, true, false);
    assert_eq!(t.lock_left_s(), Some(WAIT), "the count starts at the stop, with the whole wait before it");
}

#[test]
fn a_flame_that_started_one_wait_starts_no_second_one_for_a_kindling_after_it_that_did_not_take() {
    let mut t = standing();
    for _ in 0..300 {
        second(&mut t, false, true);
    }
    second(&mut t, true, false);
    assert_eq!(t.lock_left_s(), Some(WAIT));
    // the person lights it: the wait is over for good
    assert!(t.press(Press::Light).is_some());
    for _ in 0..30 {
        second(&mut t, false, false); // a kindling, no flame
    }
    second(&mut t, true, false);
    assert_eq!(t.lock_left_s(), None, "the flame of before was spent by its wait; this kindling gave none");
}

// ---------------------------------------------------------------- how long a steady flame has to stand to be a fire: the automaton's sure_s

/// A burner that stood, was lit, held a steady flame for STEADY seconds, lost it, burned off for a minute and stopped: whether the stop starts a wait.
fn after_a_flame_of(steady_s: u32) -> Thermostat {
    let mut t = standing();
    for _ in 0..30 {
        second(&mut t, false, false); // purge and kindling
    }
    for _ in 0..steady_s {
        second(&mut t, false, true);
    }
    for _ in 0..60 {
        second(&mut t, false, false); // the flame is gone, the burner burns off
    }
    second(&mut t, true, false);
    t
}

#[test]
fn a_burst_of_steady_flame_shorter_than_sure_s_in_a_kindling_is_no_fire() {
    let t = after_a_flame_of(SURE as u32 - 1);
    assert_eq!(t.lock_left_s(), None, "four seconds of a flame that the automaton itself does not yet believe");
}

#[test]
fn a_steady_flame_for_sure_s_and_longer_that_goes_out_is_a_fire() {
    for steady_s in [SURE as u32, SURE as u32 + 1, 600] {
        assert_eq!(after_a_flame_of(steady_s).lock_left_s(), Some(WAIT), "{steady_s} s of a steady flame");
    }
}

#[test]
fn a_break_in_the_flame_starts_the_count_again_and_the_seconds_before_it_are_not_added() {
    let mut t = standing();
    for _ in 0..4 {
        second(&mut t, false, true);
    }
    second(&mut t, false, false);
    for _ in 0..4 {
        second(&mut t, false, true);
    }
    second(&mut t, true, false);
    assert_eq!(t.lock_left_s(), None, "four and four seconds with a break between them are not five in a row");
}

#[test]
fn a_flame_that_stands_while_the_automaton_is_in_stop_is_not_counted() {
    let mut t = standing();
    for _ in 0..60 {
        second(&mut t, true, true); // the sensor sees a flame in a stop: it is the automaton's to sort out, not a fire that was lit
    }
    second(&mut t, false, false);
    second(&mut t, true, false);
    assert_eq!(t.lock_left_s(), None);
}
