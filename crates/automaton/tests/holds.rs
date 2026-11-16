//! What the burner holds: the water at the top of the boiler, the water
//! coming back, or no temperature at all and one level of power. Every row is a pair: the
//! second the chosen sensor decides and a second where ONLY ANOTHER sensor stands where it would have decided, so a
//! burner that still looks at the boiler cannot pass either.
//!
//! **THE PROTECTIONS DO NOT MOVE WITH THE MODE**: overheating, the alarm and the pump look at the boiler in every mode,
//! and the pump has a number of its own for standing off - the setpoint's hysteresis is the setpoint's.

mod common;

use automaton::{Asked, Doing, Feed, Heard, Lighting, State, Step, Told, Why};
use common::{Burning, Hold, Seen, Silence, Tuning};
use flame::Flame;

/// The numbers of the board's card and the factory's: the return held at 45 °C, lit again 4 under it.
fn by(hold: Hold) -> Tuning {
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
        want_c: 45.0,
        hysteresis_c: 4.0,
        hold,
        // a level a person chose by hand, far from the step the boiler works on
        power: Step { pa: 22.0, feed: Feed { on_s: 2.0, off_s: 21.0 } },
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

fn sound(t: Tuning) -> common::Sound {
    t.sound().expect("the numbers of the row are sound")
}

fn cold() -> Seen {
    Seen {
        flame: Flame::Out,
        boiler_c: Some(40.0),
        return_c: Some(30.0),
        flue_c: Some(20.0),
        draught_pa: Some(0.0), draught_low: false,
        gate: 0,
        silence: Silence::HEARD,
    }
}

fn lit() -> Seen {
    Seen { flame: Flame::Steady, flue_c: Some(150.0), draught_pa: Some(50.0), draught_low: false, gate: 30, ..cold() }
}

fn quiet(burning: &mut Burning, seen: Seen, seconds: usize, t: &common::Sound) -> Doing {
    let mut did = burning.step(Told::Nothing, seen, 1.0, t);
    for _ in 1..seconds {
        did = burning.step(Told::Nothing, seen, 1.0, t);
    }
    did
}

/// A burner taken to WORKING the short way: asked for heat by a Start and lit by a Light; the readings the scaffolding gives are under every setpoint of the rows.
fn working(t: &common::Sound) -> Burning {
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 0.0, t);
    burning.step(Told::Light, cold(), 1.0, t);
    quiet(&mut burning, cold(), 30, t);
    quiet(&mut burning, lit(), 92, t);
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach working");
    burning
}

fn silence_of(return_c: Heard) -> Silence {
    Silence { return_c, ..Silence::HEARD }
}

// ---------------------------------------------------------------- the return

#[test]
fn held_by_the_return_the_burner_smoulders_when_the_return_reaches_the_setpoint_and_the_boiler_has_not() {
    let t = sound(by(Hold::Return));
    let mut burning = working(&t);
    let did = quiet(&mut burning, Seen { return_c: Some(46.0), boiler_c: Some(40.0), ..lit() }, 1, &t);
    assert_eq!(did.state, State::Smouldering, "the return is past 45 and the burner kept working");
}

#[test]
fn held_by_the_return_the_burner_keeps_working_while_the_return_is_under_the_setpoint_whatever_the_boiler_says() {
    let t = sound(by(Hold::Return));
    let mut burning = working(&t);
    let did = quiet(&mut burning, Seen { return_c: Some(44.0), boiler_c: Some(70.0), ..lit() }, 1, &t);
    assert_eq!(did.state, State::Working, "the boiler at 70 smouldered a burner that holds the return");
}

#[test]
fn held_by_the_return_a_smouldering_burner_works_again_when_the_return_falls_by_the_hysteresis() {
    let t = sound(by(Hold::Return));
    let mut burning = working(&t);
    quiet(&mut burning, Seen { return_c: Some(46.0), ..lit() }, 1, &t);
    assert_eq!(burning.state(), State::Smouldering);
    let warm = quiet(&mut burning, Seen { return_c: Some(42.0), boiler_c: Some(40.0), ..lit() }, 1, &t);
    assert_eq!(warm.state, State::Smouldering, "the return at 42 is inside the hysteresis of 45 - 4");
    let cool = quiet(&mut burning, Seen { return_c: Some(41.0), boiler_c: Some(80.0), ..lit() }, 1, &t);
    assert_eq!(cool.state, State::Working, "the return at 41 is the line, and the boiler at 80 must not hold it back");
}

#[test]
fn held_by_the_return_a_stopped_burner_that_was_asked_for_heat_lights_again_by_the_return_and_not_by_the_boiler() {
    let t = sound(by(Hold::Return));
    let mut burning = working(&t);
    quiet(&mut burning, Seen { return_c: Some(46.0), ..lit() }, 302, &t);
    quiet(&mut burning, Seen { return_c: Some(46.0), ..cold() }, 21, &t);
    assert_eq!(burning.state(), State::Stop, "the scaffolding did not reach the stop");
    let held = quiet(&mut burning, Seen { return_c: Some(44.0), boiler_c: Some(30.0), ..cold() }, 5, &t);
    assert_eq!(held.state, State::Stop, "the boiler at 30 lit a burner that holds the return");
    let lit_again = quiet(&mut burning, Seen { return_c: Some(41.0), boiler_c: Some(70.0), ..cold() }, 1, &t);
    assert_eq!(lit_again.state, State::Purge, "the return fell by the hysteresis and the burner did not light");
}

// ---------------------------------------------------------------- the silence of the held sensor

#[test]
fn held_by_the_return_ten_periods_of_its_silence_burn_the_fire_off_and_name_the_return() {
    let t = sound(by(Hold::Return));
    let mut burning = working(&t);
    let too_long = Seen { return_c: None, silence: silence_of(Heard::TooLong), ..lit() };
    let did = quiet(&mut burning, too_long, 1, &t);
    assert_eq!(did.state, State::BurningOff(Some(Why::ReturnSilent)));
}

#[test]
fn held_by_the_return_a_return_that_was_silent_short_of_ten_periods_and_answered_burns_on_by_the_return() {
    let t = sound(by(Hold::Return));
    let mut burning = working(&t);
    let silent = Seen { return_c: None, silence: silence_of(Heard::Silent), ..lit() };
    let did = quiet(&mut burning, silent, 9, &t);
    assert_eq!(did.state, State::Working, "nine periods are not ten, and no number is no reason to smoulder");
    let answered = quiet(&mut burning, Seen { return_c: Some(44.0), boiler_c: Some(70.0), ..lit() }, 1, &t);
    assert_eq!(answered.state, State::Working, "the burner went by the boiler when the return came back");
    let reached = quiet(&mut burning, Seen { return_c: Some(46.0), boiler_c: Some(40.0), ..lit() }, 1, &t);
    assert_eq!(reached.state, State::Smouldering, "and still holds the return");
}

#[test]
fn a_silent_return_does_not_burn_off_a_burner_that_holds_the_boiler() {
    let t = sound(by(Hold::Boiler));
    let mut burning = working(&t);
    let too_long = Seen { return_c: None, silence: silence_of(Heard::TooLong), ..lit() };
    let did = quiet(&mut burning, too_long, 1, &t);
    assert_eq!(did.state, State::Working, "a sensor the mode does not use is not the mode's business");
}

// ---------------------------------------------------------------- the boiler, and the power

#[test]
fn held_by_the_boiler_the_burner_smoulders_at_the_setpoint_on_the_boiler_whatever_the_return_says() {
    let t = sound(by(Hold::Boiler));
    let mut burning = working(&t);
    let under = quiet(&mut burning, Seen { boiler_c: Some(44.0), return_c: Some(60.0), ..lit() }, 1, &t);
    assert_eq!(under.state, State::Working);
    let did = quiet(&mut burning, Seen { boiler_c: Some(45.0), return_c: Some(20.0), ..lit() }, 1, &t);
    assert_eq!(did.state, State::Smouldering, "the boiler is at the setpoint");
}

#[test]
fn held_on_a_level_the_burner_burns_on_the_chosen_level_and_smoulders_at_no_temperature() {
    let t = sound(by(Hold::Power));
    let mut burning = working(&t);
    let mut fed = 0;
    let mut did = quiet(&mut burning, Seen { boiler_c: Some(70.0), return_c: Some(60.0), ..lit() }, 1, &t);
    for _ in 0..46 {
        fed += did.auger as u32;
        did = quiet(&mut burning, Seen { boiler_c: Some(70.0), return_c: Some(60.0), ..lit() }, 1, &t);
    }
    assert_eq!(did.state, State::Working, "water past the setpoint smouldered a burner that holds no temperature");
    assert_eq!(did.fan, Asked::Hold(22.0), "the draught is the chosen level's, not the boiler's step");
    assert!(fed <= 6, "the auger turned {fed} s of 47 on a level that gives 2 s of 23");
}

#[test]
fn held_by_the_boiler_the_burner_works_on_the_boilers_step_and_not_on_the_chosen_level() {
    let t = sound(by(Hold::Boiler));
    let mut burning = working(&t);
    let did = quiet(&mut burning, lit(), 5, &t);
    assert_eq!(did.fan, Asked::Hold(120.0));
}

// ---------------------------------------------------------------- the protections and the pump

#[test]
fn the_boiler_overheating_burns_the_fire_off_in_every_mode() {
    for hold in [Hold::Power, Hold::Boiler, Hold::Return] {
        let t = sound(by(hold));
        let mut burning = working(&t);
        let did = quiet(&mut burning, Seen { boiler_c: Some(89.0), ..lit() }, 1, &t);
        assert_eq!(did.state, State::BurningOff(Some(Why::Overheated(89.0))), "{hold:?}");
    }
}

#[test]
fn the_pump_stands_off_by_its_own_number_and_not_by_the_hysteresis_of_the_setpoint() {
    // pump from 35, standing off 4 under it; the setpoint's hysteresis is 10 and must not be the pump's
    let t = sound(Tuning { pump_on_c: 35.0, hysteresis_c: 10.0, pump_off_below_c: 4.0, ..by(Hold::Boiler) });
    let mut burning = Burning::new();
    let water = |c: f32| Seen { boiler_c: Some(c), ..cold() };
    assert!(quiet(&mut burning, water(40.0), 1, &t).pump);
    assert!(quiet(&mut burning, water(32.0), 1, &t).pump, "the pump stood off inside its own 4 degrees");
    assert!(!quiet(&mut burning, water(30.0), 1, &t).pump, "31 is where this pump stands off, and the water is at 30");
}

#[test]
fn a_smouldering_burner_whose_mode_is_turned_to_a_level_works_again_at_once() {
    // a mode that holds no temperature has no reason to smoulder: what a person turns on the card takes the next second
    let by_boiler = sound(by(Hold::Boiler));
    let mut burning = working(&by_boiler);
    quiet(&mut burning, Seen { boiler_c: Some(46.0), ..lit() }, 1, &by_boiler);
    assert_eq!(burning.state(), State::Smouldering, "the scaffolding did not reach smouldering");
    let by_level = sound(by(Hold::Power));
    let did = quiet(&mut burning, Seen { boiler_c: Some(46.0), ..lit() }, 1, &by_level);
    assert_eq!(did.state, State::Working);
}
