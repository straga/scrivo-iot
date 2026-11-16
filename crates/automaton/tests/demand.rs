//! What the fire does with the word of the house (rows 1 and 2), on the real thermostat that orders it: **WARM IS THE ORDINARY WAY TO A STOP** - a working
//! fire goes down to smouldering and burns out by its time, and a stop is not lit however cold the water; **COLD IS THE WATER'S OWN BAND**, and in the mode of power, where the
//! water never says burn, it is what lights a stop. Nothing is put out past the burning off: a demand that put the fire out at once would be a protection, and it is not.

mod common;

use automaton::{Doing, Feed, Lighting, State, Step, Told};
use common::{Burning, Hold, Seen, Silence, Tuning};
use demand::{Asked, Demand, Why};
use flame::Flame;

fn warm() -> Option<Asked> {
    Some(Asked { demand: Demand::Warm, why: Why::TargetReached })
}

fn cold() -> Option<Asked> {
    Some(Asked { demand: Demand::Cold, why: Why::BelowTarget })
}

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

/// The water under every setpoint of the rows: 40 °C against a setpoint of 45 and a line of 41 to burn.
fn cold_water() -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: Some(30.0), flue_c: Some(20.0), draught_pa: Some(0.0), draught_low: false, gate: 0, silence: Silence::HEARD }
}

fn lit() -> Seen {
    Seen { flame: Flame::Steady, flue_c: Some(150.0), draught_pa: Some(50.0), draught_low: false, gate: 30, ..cold_water() }
}

fn quiet(burning: &mut Burning, seen: Seen, seconds: usize, t: &common::Sound) -> Doing {
    let mut did = burning.step(Told::Nothing, seen, 1.0, t);
    for _ in 1..seconds {
        did = burning.step(Told::Nothing, seen, 1.0, t);
    }
    did
}

/// A burner taken to WORKING the short way, asked for heat by a Start, lit by the cold water under it, and told by nobody yet. (It was lit by a Light; a Light is a cycle to the nominal now, which no word of the house takes the fire down from, and these rows are about the house's word.)
fn working(t: &common::Sound) -> Burning {
    let mut burning = Burning::new();
    burning.step(Told::Start, cold_water(), 0.0, t);
    // the mode of power has no water to light by and no nominal: there a Light is what lights, as it always was
    burning.step(if t.numbers().1.hold == Hold::Power { Told::Light } else { Told::Nothing }, cold_water(), 1.0, t);
    quiet(&mut burning, cold_water(), 30, t);
    quiet(&mut burning, lit(), 92, t);
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach working");
    burning
}

/// A burner taken from working to a stop the way it always went - the water over the setpoint, with nobody asking - the switch still on.
fn stopped(t: &common::Sound) -> Burning {
    let mut burning = working(t);
    let hot = Seen { boiler_c: Some(50.0), ..lit() };
    quiet(&mut burning, hot, 302, t);
    quiet(&mut burning, Seen { boiler_c: Some(50.0), ..cold_water() }, 21, t);
    assert_eq!(burning.state(), State::Stop, "the scaffolding did not reach the stop");
    burning
}

// ---------------------------------------------------------------- warm

#[test]
fn warm_sends_a_working_fire_down_to_smouldering_whatever_the_water_and_not_straight_to_burning_off() {
    let t = sound(by(Hold::Boiler));
    let mut burning = working(&t);
    let still = quiet(&mut burning, lit(), 3, &t);
    assert_eq!(still.state, State::Working, "the water at 40 is under the line and asks for burning: nothing else tells the fire to go down");
    burning.ask(warm());
    let did = quiet(&mut burning, lit(), 1, &t);
    assert_eq!(did.state, State::Smouldering, "a warm house is the ordinary way down, by the fire's own time");
}

#[test]
fn warm_burns_the_fire_out_by_its_own_time_to_a_stop() {
    let t = sound(by(Hold::Boiler));
    let mut burning = working(&t);
    burning.ask(warm());
    quiet(&mut burning, lit(), 299, &t);
    assert_eq!(burning.state(), State::Smouldering, "299 s of the 300 the fire smoulders for");
    quiet(&mut burning, lit(), 3, &t);
    assert!(matches!(burning.state(), State::BurningOff(None)), "the time is out: burning off, and no fault: {:?}", burning.state());
    quiet(&mut burning, cold_water(), 21, &t);
    assert_eq!(burning.state(), State::Stop);
}

#[test]
fn a_stop_is_not_lit_by_cold_water_while_the_house_is_warm() {
    let t = sound(by(Hold::Boiler));
    let mut burning = stopped(&t);
    burning.ask(warm());
    let did = quiet(&mut burning, Seen { boiler_c: Some(30.0), ..cold_water() }, 10, &t);
    assert_eq!(did.state, State::Stop, "the water at 30 is far under the line and the house is warm: the person's Start was an ask to work by the house");
}

#[test]
fn a_stop_is_lit_again_by_the_water_the_second_the_house_turns_cold() {
    let t = sound(by(Hold::Boiler));
    let mut burning = stopped(&t);
    burning.ask(warm());
    let waiting = quiet(&mut burning, Seen { boiler_c: Some(30.0), ..cold_water() }, 5, &t);
    assert_eq!(waiting.state, State::Stop, "the house is warm and the water is not asked");
    burning.ask(cold());
    let did = quiet(&mut burning, Seen { boiler_c: Some(30.0), ..cold_water() }, 1, &t);
    assert_eq!(did.state, State::Purge, "cold house, cold water, switch on");
}

// ---------------------------------------------------------------- cold in the mode of power

#[test]
fn cold_lights_a_stop_in_the_mode_of_power_and_nobody_asking_does_not() {
    let t = sound(by(Hold::Power));
    // a board that came up with the switch on and the fire stopped: the mode of power has no way down to a stop that a person has not pressed
    let mut burning = Burning::resumed_kept(State::Stop, &cold_water(), &t, Some("on"));
    assert!(burning.wanted() && burning.state() == State::Stop, "the scaffolding did not stand a stop with the switch on");
    let nobody = quiet(&mut burning, cold_water(), 10, &t);
    assert_eq!(nobody.state, State::Stop, "with nobody asking, the mode of power holds no water and a stop is not lit by it");
    burning.ask(cold());
    let did = quiet(&mut burning, cold_water(), 1, &t);
    assert_eq!(did.state, State::Purge, "cold is what lights a stop where the water says nothing of burning");
}

#[test]
fn cold_leaves_a_working_fire_working_in_the_mode_of_power() {
    let t = sound(by(Hold::Power));
    let mut burning = working(&t);
    burning.ask(cold());
    let did = quiet(&mut burning, Seen { boiler_c: Some(70.0), ..lit() }, 30, &t);
    assert_eq!(did.state, State::Working, "water past the setpoint does not smoulder a fire the house asks to burn");
}

// ---------------------------------------------------------------- a flame lost in smouldering

/// The dark seconds of these rows: `by` waits for a lost flame 120 s, and a few seconds more are given for the state to change on the way.
const LOST_S: usize = 125;

fn dark() -> Seen {
    Seen { flame: Flame::Out, ..lit() }
}

/// The states of LOST_S dark seconds after the fire was taken down to smouldering by the house.
fn dark_after_smouldering(house: Option<Asked>, water_c: f32) -> Vec<State> {
    let t = sound(by(Hold::Boiler));
    let mut burning = working(&t);
    burning.ask(warm());
    quiet(&mut burning, lit(), 1, &t);
    assert_eq!(burning.state(), State::Smouldering, "the scaffolding did not reach smouldering");
    burning.ask(house);
    (0..LOST_S).map(|_| burning.step(Told::Nothing, Seen { boiler_c: Some(water_c), ..dark() }, 1.0, &t).state).collect()
}

#[test]
fn a_flame_lost_in_smouldering_with_no_heat_asked_goes_to_burning_off_and_is_not_lit_again() {
    // the house is warm, so the water never says burn: 40 °C under the line is not asked for either
    let states = dark_after_smouldering(warm(), 40.0);
    assert!(!states.iter().any(|s| matches!(s, State::Kindling(_))), "a fire nobody asked for was lit again: {states:?}");
    assert!(states.contains(&State::BurningOff(None)), "the lost flame did not take the ordinary way to a stop: {:?}", states.last());
}

#[test]
fn a_flame_lost_in_smouldering_with_the_water_over_its_setpoint_goes_to_burning_off_too() {
    // nobody plays the house: the water alone, 50 °C over the setpoint of 45
    let states = dark_after_smouldering(None, 50.0);
    assert!(!states.iter().any(|s| matches!(s, State::Kindling(_))), "{states:?}");
    assert!(states.contains(&State::BurningOff(None)), "{:?}", states.last());
}

#[test]
fn a_flame_lost_in_smouldering_with_heat_asked_is_lit_again() {
    // the pair: the house turns cold and the water at 40 is under the line - the fire goes back to work and the lost flame is lit again
    let states = dark_after_smouldering(cold(), 40.0);
    assert!(states.contains(&State::Kindling(1)), "heat was asked and the lost flame was not lit again: {:?}", states.last());
}

#[test]
fn a_flame_lost_at_work_is_lit_again_whatever_the_house_said_before() {
    // the pair: at work the water burns (40 °C under the line) and nobody has asked for less
    let t = sound(by(Hold::Boiler));
    let mut burning = working(&t);
    let states: Vec<State> = (0..LOST_S).map(|_| burning.step(Told::Nothing, dark(), 1.0, &t).state).collect();
    assert!(states.contains(&State::Kindling(1)), "{:?}", states.last());
}

#[test]
fn a_flame_lost_in_smouldering_with_the_house_cold_and_the_water_inside_its_hysteresis_is_lit_again() {
    // the water says keep (43 °C between the line of 41 and the setpoint of 45), the house says cold: the fire is wanted, and the word of the water alone does not say it
    let states = dark_after_smouldering(cold(), 43.0);
    assert!(states.contains(&State::Kindling(1)), "the house is cold and the lost flame was not lit again: {:?}", states.last());
    assert!(!states.contains(&State::BurningOff(None)), "{states:?}");
}

#[test]
fn a_flame_lost_in_smouldering_with_the_house_warm_and_the_water_inside_its_hysteresis_goes_to_burning_off() {
    // the pair: the same water and the house warm - nobody wants the fire
    let states = dark_after_smouldering(warm(), 43.0);
    assert!(states.contains(&State::BurningOff(None)), "{:?}", states.last());
    assert!(!states.iter().any(|s| matches!(s, State::Kindling(_))), "{states:?}");
}

// ---------------------------------------------------------------- the first round of the sensors: a barrier on a light from a stop

#[test]
fn a_smouldering_fire_goes_back_to_work_on_burning_water_before_the_first_round_of_the_sensors() {
    // a board that came up with a fire smouldering: the water falls under the line (40 < 41) in the first seconds, and the fire is not kept from work by a round that has not gone through
    let t = sound(by(Hold::Boiler));
    let mut burning = working(&t);
    burning.ask(warm());
    quiet(&mut burning, lit(), 1, &t);
    assert_eq!(burning.state(), State::Smouldering, "the scaffolding did not reach smouldering");
    burning.wait_for_first_round();
    burning.ask(cold());
    let did = quiet(&mut burning, lit(), 2, &t);
    assert_eq!(did.state, State::Working, "the water burns and the fire is alive: the word of the water goes as it is");
}

#[test]
fn a_stop_is_not_lit_by_burning_water_before_the_first_round_of_the_sensors_and_is_after_it() {
    let t = sound(by(Hold::Boiler));
    let mut burning = stopped(&t);
    burning.wait_for_first_round();
    burning.ask(cold());
    let water = Seen { boiler_c: Some(30.0), ..cold_water() };
    assert_eq!(quiet(&mut burning, water, 5, &t).state, State::Stop, "the water is far under the line, the house is cold, and the round has not gone through: a stop is not lit");
    burning.first_round_done();
    assert_eq!(quiet(&mut burning, water, 1, &t).state, State::Purge, "after the round the stop is lit as at any other second");
}
