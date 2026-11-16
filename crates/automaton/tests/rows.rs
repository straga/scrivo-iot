//! What the automaton does with rows of readings and seconds, and no board anywhere. Every transition the
//! stage names has a pair here - the second it happens and a second it does not - and every failure of its
//! table has its own row.
//!
//! **THE NUMBERS IN `tuning` ARE THE FACTORY'S**, taken off this burner's own card: 51 Pa and 32 s of dose to
//! light, 120 Pa and 4.6 s at 25 kW, 1 s against 50 of pause to smoulder. They are here so that a row reads
//! like the burner a person knows, and not because the crate believes them - it holds no number of its own.
//!
//! **NOTHING HERE MODELS A FIRE.** A row says what the sensors said; whether a pot would really have lit on
//! that dose is the stand's question. What these rows judge is the order: that the burner does
//! not feed a pot it cannot see, does not skip burning off, and does not light itself after a power cut.

mod common;

use automaton::{Heard, Asked, Doing, Feed, Lighting, Refused, State, Step, Told, Why};
use common::{FireNumbers, Hold, Burning, Seen, Silence, Tuning};
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
        // nought: these rows are about the order of burning off, and the least burning out has rows of its own
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

/// The same numbers, through the door. Every row burns by these; the rows that test the door itself build
/// their own and call `sound()` directly.
fn sound() -> common::Sound {
    tuning().sound().expect("the numbers the factory runs this burner on are sound")
}

/// A cold burner that sees no flame: what the board reads with the boiler standing.
fn cold() -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0), draught_pa: Some(0.0), draught_low: false, gate: 0, silence: Silence::HEARD }
}

/// A cold pot with the fan blowing the lighting's draught: what the board reads once the air comes in an attempt. The
/// plug burns only with air, so a row about the plug gives it this after the dose.
fn blowing() -> Seen {
    Seen { draught_pa: Some(51.0), draught_low: false, gate: 30, ..cold() }
}

/// What the board reads in second N of an attempt with no flame: the fan stands through the dose and blows after it.
fn in_attempt(n: usize) -> Seen {
    if n <= 32 { cold() } else { blowing() }
}

/// A flame gone for good while the fan blows: the sensor dark, and a flue that says nothing.
/// These rows are about the wait and the lighting again; a flue that holds under a dark sensor is a dim fire and not a
/// lost one, which tests/lighting_again.rs judges with a flue that moves.
fn gone() -> Seen {
    Seen { flame: Flame::Out, flue_c: None, ..lit() }
}

/// A burner with a fire in it, warm and blowing.
fn lit() -> Seen {
    Seen { flame: Flame::Steady, boiler_c: Some(40.0), return_c: None, flue_c: Some(150.0), draught_pa: Some(50.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// SECONDS of nobody touching anything, and what the last of them decided.
fn quiet(burning: &mut Burning, seen: Seen, seconds: usize) -> Doing {
    let t = sound();
    let mut did = burning.step(Told::Nothing, seen, 1.0, &t);
    for _ in 1..seconds {
        did = burning.step(Told::Nothing, seen, 1.0, &t);
    }
    did
}

/// One second with a button pressed.
fn told(burning: &mut Burning, what: Told, seen: Seen) -> Doing {
    burning.step(what, seen, 1.0, &sound())
}

/// A burner taken as far as WORKING the short way, and the seconds it took. The route is the one the rows
/// below prove step by step; here it is scaffolding.
fn working() -> Burning {
    working_by(&sound())
}

/// A burner at work over a flue that stands at FLUE from the first second of the lighting: the fall of a flue is of the highest it has had, and the
/// scaffold of `working` has the flue at 150 °C, from which a row about a cool flue would fall at once.
fn working_over(flue: f32) -> Burning {
    let t = sound();
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, &t);
    for _ in 0..30 {
        burning.step(Told::Nothing, cold(), 1.0, &t);
    }
    for _ in 0..92 {
        burning.step(Told::Nothing, Seen { flue_c: Some(flue), ..lit() }, 1.0, &t);
    }
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach work");
    burning
}

/// The same, burning by the numbers handed in.
fn working_by(t: &common::Sound) -> Burning {
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, t);
    for _ in 0..30 {
        burning.step(Told::Nothing, cold(), 1.0, t);
    }
    for _ in 0..92 {
        burning.step(Told::Nothing, lit(), 1.0, t);
    }
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach working");
    burning
}

// ---------------------------------------------------------------- stop -> purge

#[test]
fn a_person_asking_for_heat_sends_a_stopped_burner_to_purge() {
    let mut burning = Burning::new();
    let did = told(&mut burning, Told::Start, cold());
    assert_eq!(did.state, State::Purge);
    assert_eq!(did.fan, Asked::Purge, "a purge is full air past the regulator");
    assert!(!did.auger && !did.igniter, "nothing is fed into a pot that is being blown clear");
}

#[test]
fn a_burner_back_at_a_stop_with_heat_still_wanted_lights_again_by_itself() {
    // asked for, lit, and put out by its own clock - the asking for heat outlives all of it
    let mut burning = working();
    quiet(&mut burning, Seen { boiler_c: Some(61.0), ..lit() }, 302);
    quiet(&mut burning, Seen { boiler_c: Some(70.0), ..cold() }, 21);
    assert_eq!(burning.state(), State::Stop, "burnt off and stopped, and nobody said stop");
    assert!(burning.wanted(), "a person asked for heat and nothing has taken the asking away");
    let hot = quiet(&mut burning, Seen { boiler_c: Some(70.0), ..cold() }, 5);
    assert_eq!(hot.state, State::Stop, "the water is hot: there is nothing to light for");
    let cool = quiet(&mut burning, Seen { boiler_c: Some(55.0), ..cold() }, 1);
    assert_eq!(cool.state, State::Purge, "and when it falls the burner starts on its own");
}

#[test]
fn a_burner_nobody_asked_for_heat_does_not_light_itself_however_cold_the_water() {
    let mut burning = Burning::new();
    let did = quiet(&mut burning, Seen { boiler_c: Some(10.0), ..cold() }, 600);
    assert_eq!(did.state, State::Stop, "the water is freezing and nobody has asked for heat");
    assert!(!burning.wanted());
}

#[test]
fn a_burner_that_was_asked_for_heat_lights_again_when_the_water_falls_far_enough() {
    let mut burning = working();
    // up to the setpoint: it goes to smoulder, and from there it is the water that decides
    quiet(&mut burning, Seen { boiler_c: Some(61.0), ..lit() }, 1);
    assert_eq!(burning.state(), State::Smouldering);
    let warm = quiet(&mut burning, Seen { boiler_c: Some(57.0), ..lit() }, 1);
    assert_eq!(warm.state, State::Smouldering, "57 is inside the hysteresis and asks for nothing");
    let cool = quiet(&mut burning, Seen { boiler_c: Some(55.0), ..lit() }, 1);
    assert_eq!(cool.state, State::Working, "below the setpoint less the hysteresis the fire goes back up");
}

#[test]
fn a_light_over_water_at_or_over_its_setpoint_lights_nothing_and_over_water_inside_its_band_it_lights() {
    // 30.10: a Light is a cycle to the nominal of whoever holds the burner (here the water, its setpoint 60), and with the nominal reached there is nothing to carry the fire to; it was "a person's word and the water as an OR" before
    let mut burning = Burning::new();
    let did = told(&mut burning, Told::Light, Seen { boiler_c: Some(75.0), ..cold() });
    assert_eq!(did.state, State::Stop, "water at 75 against a setpoint of 60 has reached the nominal already");
    assert_eq!(burning.refused(), Some(thermostat::Refusal { nominal: thermostat::Nominal::Water, now_c: 75.0, to_c: 60.0 }));
    // the pair: inside the band, where the water alone would not light, a Light does
    let mut burning = Burning::new();
    let did = told(&mut burning, Told::Light, Seen { boiler_c: Some(58.0), ..cold() });
    assert_eq!(did.state, State::Purge, "the water is under the setpoint: the person's word lights");
    assert_eq!(burning.refused(), None);
}

#[test]
fn a_boiler_whose_water_nobody_can_read_does_not_light_itself() {
    // the pair of the row above: the same burner, back at a stop with heat still wanted, and a bus that says
    // nothing about the boiler this round
    let mut burning = working();
    quiet(&mut burning, Seen { boiler_c: Some(61.0), ..lit() }, 302);
    quiet(&mut burning, Seen { boiler_c: Some(70.0), ..cold() }, 21);
    assert_eq!(burning.state(), State::Stop);
    assert!(burning.wanted());
    let blind = quiet(&mut burning, Seen { boiler_c: None, ..cold() }, 600);
    assert_eq!(blind.state, State::Stop, "a missing temperature is not a cold one, and not a guess either");
}

// ---------------------------------------------------------------- purge -> kindling

#[test]
fn the_purge_lasts_the_seconds_it_was_given_and_then_the_igniter_comes_on() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    let during = quiet(&mut burning, cold(), 28);
    assert_eq!(during.state, State::Purge, "29 seconds of a 30 second purge is still a purge");
    let after = quiet(&mut burning, cold(), 2);
    assert_eq!(after.state, State::Kindling(1), "the first attempt, and a person is told which");
    let first = quiet(&mut burning, cold(), 1);
    assert!(first.auger && !first.igniter, "the dose is what an attempt begins with, and the plug waits for it");
    assert_eq!(first.fan, Asked::Stop, "the fan stands while the dose lands");
}

// ---------------------------------------------------------------- kindling -> rising

#[test]
fn a_flame_ends_the_attempt_and_the_power_starts_climbing() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    // the flame holds five seconds in a row before the fire is its own
    let held = quiet(&mut burning, lit(), 4);
    assert_eq!(held.state, State::Kindling(1), "four seconds of flame are not a fire yet");
    let did = quiet(&mut burning, lit(), 1);
    assert_eq!(did.state, State::Rising);
    assert!(!did.igniter, "the plug goes cold the moment the fire is its own");
}

#[test]
fn no_flame_keeps_the_attempt_running_to_its_end() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    let did = quiet(&mut burning, cold(), 100);
    assert_eq!(did.state, State::Kindling(1), "the attempt is 30 s of plug and 90 s of waiting");
}

// ---------------------------------------------------------------- kindling -> kindling

#[test]
fn an_attempt_that_did_not_light_is_followed_by_another_on_a_smaller_dose() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    quiet(&mut burning, cold(), 152);
    assert_eq!(burning.state(), State::Kindling(2), "the second attempt, counted for a person");
    // the dose of the second attempt: a tenth of the first, so 3.2 s of auger and not 32
    let mut fed = 0;
    for _ in 0..30 {
        if quiet(&mut burning, cold(), 1).auger {
            fed += 1;
        }
    }
    assert!((3..=4).contains(&fed), "a tenth of a 32 second dose, and {fed} seconds of auger is not it");
}

// ---------------------------------------------------------------- kindling -> burning off -> fault

#[test]
fn a_burner_that_never_lit_burns_off_first_and_names_the_attempts_after() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    // three attempts: 152 s the first (dose 32, plug with air 30, wait 90), 124 s each of the other two on a tenth of
    // the dose
    let did = quiet(&mut burning, cold(), 152 + 124 + 124);
    assert_eq!(did.state, State::BurningOff(Some(Why::NeverLit(3))), "the fault waits behind the burning off");
    assert_eq!(did.fan, Asked::Purge);
    assert!(!did.auger, "nothing is fed into a pot that is being cleared");
    // the flame is out already, and the pot is blown clear all the same: this is the one pot that certainly
    // holds unburnt pellets
    let early = quiet(&mut burning, cold(), 18);
    assert_eq!(early.state, State::BurningOff(Some(Why::NeverLit(3))), "the post purge has not run out");
    let then = quiet(&mut burning, cold(), 3);
    assert_eq!(then.state, State::Fault(Why::NeverLit(3)));
}

#[test]
fn an_attempt_that_lights_at_the_last_ends_no_burner() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    quiet(&mut burning, cold(), 162 + 134 + 10);
    assert_eq!(burning.state(), State::Kindling(3), "still the third attempt, and it has not run out");
    let did = quiet(&mut burning, lit(), 5);
    assert_eq!(did.state, State::Rising, "a fire on the last attempt is a fire");
}

// ---------------------------------------------------------------- rising -> working

#[test]
fn the_power_arrives_at_the_step_it_was_climbing_to() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    quiet(&mut burning, lit(), 5);
    let half = quiet(&mut burning, lit(), 30);
    assert_eq!(half.state, State::Rising, "halfway up is not arrived");
    match half.fan {
        Asked::Hold(pa) => assert!(
            (51.0..120.0).contains(&pa),
            "halfway between the kindling step and the one asked for, and {pa} Pa is outside both"
        ),
        other => panic!("the fan was asked for {other:?} while the power was climbing"),
    }
    let there = quiet(&mut burning, lit(), 31);
    assert_eq!(there.state, State::Working);
    assert_eq!(there.fan, Asked::Hold(120.0), "arrived means the step's own draught");
}

// ---------------------------------------------------------------- working -> smouldering and back

#[test]
fn the_setpoint_sends_the_fire_down_to_smouldering_without_anybody_asking() {
    let mut burning = working();
    let under = quiet(&mut burning, Seen { boiler_c: Some(59.0), ..lit() }, 1);
    assert_eq!(under.state, State::Working, "one degree short of the setpoint is still work");
    let at = quiet(&mut burning, Seen { boiler_c: Some(60.0), ..lit() }, 1);
    assert_eq!(at.state, State::Smouldering);
    assert_eq!(at.fan, Asked::Hold(0.0), "smouldering runs on what the chimney pulls");
}

#[test]
fn smouldering_that_nobody_needs_puts_the_burner_out_by_its_own_clock() {
    let mut burning = working();
    quiet(&mut burning, Seen { boiler_c: Some(61.0), ..lit() }, 1);
    let inside = quiet(&mut burning, Seen { boiler_c: Some(61.0), ..lit() }, 298);
    assert_eq!(inside.state, State::Smouldering, "the clock is 300 s and it has not run out");
    let out = quiet(&mut burning, Seen { boiler_c: Some(61.0), ..lit() }, 3);
    assert_eq!(out.state, State::BurningOff(None), "no fault - the boiler simply has nothing to do");
}

#[test]
fn a_smouldering_time_of_nothing_takes_the_burner_straight_to_burning_off() {
    let mut burning = working();
    let t = Tuning { smoulder_s: 0.0, ..tuning() }.sound().expect("a zero smouldering time is the design");
    let did = burning.step(Told::Nothing, Seen { boiler_c: Some(61.0), ..lit() }, 1.0, &t);
    assert_eq!(did.state, State::BurningOff(None), "a zero skips smouldering altogether");
}

// ---------------------------------------------------------------- anything -> burning off

#[test]
fn a_person_saying_stop_gets_a_procedure_and_not_a_switch() {
    let mut burning = working();
    let did = told(&mut burning, Told::Stop, lit());
    assert_eq!(did.state, State::BurningOff(None));
    assert!(!did.auger, "the fuel is off first");
    assert_eq!(did.fan, Asked::Hold(120.0), "and what is left burns out on the step's air");
    assert!(!burning.wanted(), "and the asking for heat goes with it");
}

#[test]
fn nobody_saying_anything_leaves_a_working_burner_working() {
    let mut burning = working();
    let did = quiet(&mut burning, lit(), 120);
    assert_eq!(did.state, State::Working);
}

// ---------------------------------------------------------------- burning off -> stop

// Burning off is two things: the fire burns out on low air until the flame goes or burn_off_s runs
// out, then the pot is blown clear at full for purge_after_s.

#[test]
fn burning_off_burns_out_on_low_air_while_there_is_a_flame() {
    let mut burning = working();
    told(&mut burning, Told::Stop, lit());
    let still = quiet(&mut burning, lit(), 60);
    assert_eq!(still.state, State::BurningOff(None), "there is still fire in the pot");
    assert_eq!(still.fan, Asked::Hold(120.0), "a fire burning out is held on the step's air, not blown at full");
}

#[test]
fn when_the_flame_goes_the_pot_is_blown_clear_at_full_and_only_then_it_stops() {
    let mut burning = working();
    told(&mut burning, Told::Stop, lit());
    quiet(&mut burning, lit(), 20);
    let gone = quiet(&mut burning, cold(), 1);
    assert_eq!(gone.state, State::BurningOff(None), "the flame went, and the pot is not yet blown clear");
    assert_eq!(gone.fan, Asked::Purge, "the pot is blown clear at full");
    let blown = quiet(&mut burning, cold(), 18);
    assert_eq!(blown.state, State::BurningOff(None), "the clear blow ended before its 20 s");
    let done = quiet(&mut burning, cold(), 2);
    assert_eq!(done.state, State::Stop);
    assert_eq!(done.fan, Asked::Stop, "and the fan stops with it");
}

#[test]
fn a_pot_gone_dark_burns_out_its_least_on_the_step_s_air_before_the_blow() {
    // the full blow comes last and short, however dark the pot is when burning off begins
    let t = Tuning { burn_out_min_s: 60.0, ..tuning() }.sound().expect("a least burning out under the most is sound");
    let mut burning = working_by(&t);
    let first = burning.step(Told::Stop, cold(), 1.0, &t);
    assert_eq!(first.state, State::BurningOff(None));
    let fans: Vec<Asked> = (0..80).map(|_| burning.step(Told::Nothing, cold(), 1.0, &t).fan).collect();
    let held = 1 + fans.iter().take_while(|f| **f == Asked::Hold(120.0)).count();
    assert_eq!(held, 60, "a dark pot burnt out {held} s on the step's air, and the least is 60");
    assert_eq!(fans[59], Asked::Purge, "the blow did not follow the least burning out");
}

#[test]
fn burning_off_blows_the_pot_clear_before_it_believes_a_dark_one() {
    let mut burning = working();
    // a fire that is out the second the burning off begins - what a kindling that never lit leaves behind
    told(&mut burning, Told::Stop, cold());
    let early = quiet(&mut burning, cold(), 18);
    assert_eq!(early.state, State::BurningOff(None), "the pot that looks emptiest is the one that is not");
    assert_eq!(early.fan, Asked::Purge);
    let done = quiet(&mut burning, cold(), 3);
    assert_eq!(done.state, State::Stop, "and past the post purge the dark pot ends it");
}

#[test]
fn a_fire_still_alight_at_the_ceiling_of_burning_out_is_blown_clear_and_stopped_and_keeps_its_fan() {
    // the ceiling ends burning out whatever the flame says - a blow and a stop, as at Plum, and no
    // fault; the stop holds the step's draught while the sensor sees the flame
    let mut burning = working();
    told(&mut burning, Told::Stop, lit());
    let t = sound();
    let seconds: Vec<Doing> = (0..170).map(|_| burning.step(Told::Nothing, lit(), 1.0, &t)).collect();
    assert!(seconds.iter().all(|d| !matches!(d.state, State::Fault(_))), "a fire alight at the ceiling was made a fault");
    let blow = seconds.iter().position(|d| d.fan == Asked::Purge).expect("the pot was never blown clear at the ceiling");
    assert_eq!(blow + 1, 120, "the blow came {} s into burning out, and the ceiling is 120", blow + 1);
    let stop = seconds.iter().position(|d| d.state == State::Stop).expect("no stop after the blow");
    assert!(seconds[stop..].iter().all(|d| d.fan == Asked::Hold(120.0)), "the stop's fan stood over a fire the sensor still sees");
}

#[test]
fn burning_off_does_not_hear_a_person_asking_for_heat() {
    let mut burning = working();
    told(&mut burning, Told::Stop, lit());
    let did = told(&mut burning, Told::Start, lit());
    assert_eq!(did.state, State::BurningOff(None), "there is fuel in the pot and it may not be left there");
    let did = quiet(&mut burning, lit(), 5);
    assert_eq!(did.state, State::BurningOff(None));
}

#[test]
fn burning_off_blows_without_feedback_so_a_dead_draught_sensor_does_not_stop_it() {
    let mut burning = working();
    let blind = Seen { draught_pa: None, ..lit() };
    let did = quiet(&mut burning, blind, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoDraught)));
    assert_eq!(did.fan, Asked::Purge, "a purge reads nothing, which is the whole reason it is asked for here");
    let did = quiet(&mut burning, Seen { draught_pa: None, ..cold() }, 21);
    assert_eq!(did.state, State::Fault(Why::NoDraught), "and it still reaches the fault at the end");
}

// ---------------------------------------------------------------- what to do when it does not add up

#[test]
fn a_flame_lost_at_work_leaves_the_work_on_its_own_air_while_it_is_waited_for() {
    let mut burning = working();
    let long = quiet(&mut burning, Seen { flame: Flame::Out, ..lit() }, 119);
    assert_eq!(long.state, State::Working, "119 seconds of darkness is not a lost fire yet");
    assert_eq!(long.fan, Asked::Hold(120.0), "the step's own air, not an air of its own for the dark");
    let back = quiet(&mut burning, lit(), 1);
    assert_eq!(back.state, State::Working, "the fire came back and the burner carries on");
}

#[test]
fn a_flame_lost_while_the_power_is_still_climbing_keeps_climbing_and_is_not_fed() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    quiet(&mut burning, lit(), 1);
    let climbing = quiet(&mut burning, lit(), 39);
    assert_eq!(climbing.state, State::Rising);
    let dark = quiet(&mut burning, Seen { flame: Flame::Out, ..lit() }, 11);
    assert_eq!(dark.state, State::Rising, "the fire is waited for, not given up on");
    match (climbing.fan, dark.fan) {
        (Asked::Hold(was), Asked::Hold(now)) => assert!(now > was && now <= 120.0, "the climb went {was} -> {now} Pa in the dark"),
        other => panic!("the fan was asked for {other:?} while the fire rose"),
    }
    assert!(!dark.auger, "rising feeds nothing: the pot still holds the lighting's dose");
}

#[test]
fn a_flame_that_does_not_come_back_within_its_wait_is_lit_again() {
    // not burnt off - the lighting's own attempt, the first of a lost flame's own count
    let mut burning = working();
    // lost is longer than the wait: the 121st second of dark
    let did = quiet(&mut burning, gone(), 121);
    assert_eq!(did.state, State::Kindling(1), "121 seconds of dark");
}

#[test]
fn a_flue_that_stays_cold_says_the_fuel_is_not_arriving() {
    // cold is under 70 °C for the whole window of 600 s, counted from the first second of work; under a flame that is not steady
    let mut burning = working_over(60.0);
    let cool = Seen { flame: Flame::Weak, flue_c: Some(60.0), ..lit() };
    let inside = quiet(&mut burning, cool, 500);
    assert_eq!(inside.state, State::Working, "the window is 600 s and it has not closed");
    let did = quiet(&mut burning, cool, 120);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoFuel)));
}

#[test]
fn a_flue_that_is_warming_says_nothing_against_the_fuel() {
    let mut burning = working_over(60.0);
    let mut celsius = 60.0;
    for _ in 0..120 {
        quiet(&mut burning, Seen { flame: Flame::Weak, flue_c: Some(celsius), ..lit() }, 1);
        celsius += 0.2;
    }
    assert_eq!(burning.state(), State::Working, "a flue climbing 12 degrees in two minutes is fuel arriving");
}

#[test]
fn a_cold_flue_while_the_fire_is_being_lit_accuses_nobody() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    // lit, climbing, and the flue stone cold the whole way: on this burner it is cold for an honest reason
    quiet(&mut burning, Seen { flue_c: Some(20.0), ..lit() }, 1);
    let did = quiet(&mut burning, Seen { flue_c: Some(20.0), ..lit() }, 59);
    assert_eq!(did.state, State::Rising, "the sign does not work on a cold flue and is not asked there");
}

#[test]
fn a_draught_sensor_that_gives_no_number_stops_the_fire() {
    let mut burning = working();
    let did = quiet(&mut burning, Seen { draught_pa: None, ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoDraught)));
}

#[test]
fn a_purge_asks_the_draught_sensor_for_nothing() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, Seen { draught_pa: None, ..cold() });
    let did = quiet(&mut burning, Seen { draught_pa: None, ..cold() }, 20);
    assert_eq!(did.state, State::Purge, "a purge runs past the regulator and reads nothing");
}

#[test]
fn water_hotter_than_the_burner_may_leave_it_stops_the_feeding() {
    let mut burning = working();
    let under = quiet(&mut burning, Seen { boiler_c: Some(87.0), ..lit() }, 1);
    assert_eq!(under.state, State::Smouldering, "87 is past the setpoint but not past the limit");
    let mut burning = working();
    let over = quiet(&mut burning, Seen { boiler_c: Some(89.0), ..lit() }, 1);
    assert_eq!(over.state, State::BurningOff(Some(Why::Overheated(89.0))));
    assert!(!over.auger);
    assert!(!over.alarm, "89 is not yet the temperature a person is woken for");
}

#[test]
fn water_at_the_alarm_tells_a_person_at_once_and_does_not_wait_for_the_burning_off() {
    let mut burning = working();
    let did = quiet(&mut burning, Seen { boiler_c: Some(96.0), ..lit() }, 1);
    assert!(did.alarm, "95 degrees is a signal, and a signal does not queue behind a procedure");
    assert!(matches!(did.state, State::BurningOff(Some(Why::Overheated(_)))));
}

#[test]
fn thresholds_that_are_not_thresholds_stop_the_burner_instead_of_guessing() {
    let mut burning = working();
    let did = quiet(&mut burning, Seen { flame: Flame::Unconfigured, ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::Unconfigured)));
}

#[test]
fn a_fault_is_left_by_a_person_and_by_nothing_else() {
    let mut burning = working();
    quiet(&mut burning, Seen { draught_pa: None, ..lit() }, 1);
    quiet(&mut burning, Seen { draught_pa: None, ..cold() }, 21);
    assert_eq!(burning.state(), State::Fault(Why::NoDraught));
    let waited = quiet(&mut burning, cold(), 600);
    assert_eq!(waited.state, State::Fault(Why::NoDraught), "a fault does not time out");
    let asked = told(&mut burning, Told::Start, cold());
    assert_eq!(asked.state, State::Fault(Why::NoDraught), "and it does not light on being asked either");
    let cleared = told(&mut burning, Told::Stop, cold());
    assert_eq!(cleared.state, State::Stop, "a person acknowledging it is what clears it");
}

// ---------------------------------------------------------------- coming back from a power cut

/// A board that came up remembering WAS, over a fire: it must give that fire air before anything else.
fn woke_over_a_fire(was: State) {
    let mut burning = Burning::resumed(was, &lit(), &sound());
    assert_eq!(
        burning.state(),
        State::BurningOff(None),
        "remembered {was:?} over a live flame, and the flame is the witness that was actually there"
    );
    let did = burning.step(Told::Nothing, lit(), 1.0, &sound());
    // air, and on the burning out's own draught: the fire burns out before the pot is blown clear
    assert_eq!(did.fan, Asked::Hold(120.0), "a fire at a boot gets air before it gets anything else");
    assert!(!did.auger && !did.igniter, "and nothing is poured onto a pot whose content nobody knows");
}

/// The same, with the flame saying nothing and the flue still hot: on this circuit a broken wire reads as
/// darkness (`flame::TELLS_A_BROKEN_WIRE`), so "no flame over a hot flue" is the exact shape of a sensor that
/// has fallen off a pot that is burning.
fn woke_over_a_hot_flue(was: State) {
    let seen = Seen { flame: Flame::Out, flue_c: Some(150.0), ..lit() };
    let mut burning = Burning::resumed(was, &seen, &sound());
    assert_eq!(
        burning.state(),
        State::BurningOff(None),
        "remembered {was:?} over a flue a cold pot does not make"
    );
    assert_eq!(burning.step(Told::Nothing, seen, 1.0, &sound()).fan, Asked::Purge);
}

#[test]
fn a_board_that_comes_up_remembering_a_stop_over_a_hot_flue_gives_the_pot_air() {
    woke_over_a_hot_flue(State::Stop);
}

#[test]
fn a_board_that_comes_up_remembering_a_purge_over_a_hot_flue_gives_the_pot_air() {
    woke_over_a_hot_flue(State::Purge);
}

#[test]
fn a_board_that_comes_up_remembering_a_stop_over_a_live_fire_still_gives_it_air() {
    woke_over_a_fire(State::Stop);
}

#[test]
fn a_board_that_comes_up_remembering_a_purge_over_a_live_fire_still_gives_it_air() {
    woke_over_a_fire(State::Purge);
}

#[test]
fn a_board_that_comes_up_over_a_live_fire_carries_it_on() {
    let burning = Burning::resumed(State::Working, &lit(), &sound());
    assert_eq!(burning.state(), State::Working, "alive, so carried on as it was - the work");
    assert!(!burning.wanted(), "a restart is not a person asking for heat");
}

#[test]
fn a_board_that_comes_up_remembering_work_over_a_flame_feeds_from_its_first_second() {
    // the second live lighting: after every reset a fire with its flame went through a watch and a
    // rising with the auger standing, and sat down; a fire the board sees at work is fed at once
    let mut burning = Burning::resumed(State::Working, &lit(), &sound());
    let first = quiet(&mut burning, lit(), 1);
    assert_eq!(first.state, State::Working);
    assert!(first.auger, "the auger stood in the first second over a fire the board came up to at work");
    assert_eq!(first.fan, Asked::Hold(120.0), "and the fire gets the step's draught");
}

#[test]
fn a_board_that_comes_up_remembering_smouldering_over_a_flame_smoulders_on() {
    let burning = Burning::resumed(State::Smouldering, &lit(), &sound());
    assert_eq!(burning.state(), State::Smouldering, "a fire carried on as it was, smouldering too");
}

#[test]
fn a_board_that_comes_up_remembering_rising_over_a_flame_climbs_on_and_feeds_nothing_until_work() {
    // rising feeds nothing: carried on, it climbs its whole rise_s over the flame and only then feeds
    let mut burning = Burning::resumed(State::Rising, &lit(), &sound());
    let t = sound();
    let climb: Vec<Doing> = (0..60).map(|_| burning.step(Told::Nothing, lit(), 1.0, &t)).collect();
    let work = climb.iter().position(|d| d.state == State::Working).expect("the climb never came to work");
    assert!(climb[..work].iter().all(|d| d.state == State::Rising && !d.auger), "before the work: {:?}", &climb[..work]);
    assert_eq!(work + 1, 60, "the climb after a reset lasted {} s, and rise_s is 60", work + 1);
}

#[test]
fn a_resumed_fire_whose_flame_goes_is_waited_for_at_work_and_lit_again() {
    // the board saw the fire at work, so the fire is its own: a flame gone is waited for on the same feed and
    // lit again - the lighting nothing rule is for a board that came up to a dark pot
    let dark = gone();
    let mut burning = Burning::resumed(State::Working, &lit(), &sound());
    quiet(&mut burning, lit(), 5);
    let t = sound();
    let waited: Vec<Doing> = (0..120).map(|_| burning.step(Told::Nothing, dark, 1.0, &t)).collect();
    let left = waited.iter().position(|d| d.state != State::Working);
    assert_eq!(left, None, "the work was left at its {:?} dark second", left.map(|s| s + 1));
    let gone = burning.step(Told::Nothing, dark, 1.0, &t);
    assert_eq!(gone.state, State::Kindling(1), "the 121st second without a flame");
}

#[test]
fn a_resumed_fire_whose_flame_blinks_stays_at_work() {
    let mut burning = Burning::resumed(State::Working, &lit(), &sound());
    quiet(&mut burning, lit(), 5);
    let t = sound();
    let blink: Vec<Doing> = (0..20).map(|_| burning.step(Told::Nothing, Seen { flame: Flame::Out, ..lit() }, 1.0, &t)).collect();
    let after: Vec<Doing> = (0..40).map(|_| burning.step(Told::Nothing, lit(), 1.0, &t)).collect();
    assert!(blink.iter().chain(after.iter()).all(|d| d.state == State::Working), "a blink after a reset took the fire off its work");
}

#[test]
fn a_person_s_stop_puts_out_a_resumed_fire_at_once() {
    // the pair of the waiting: the wait is for the fire, and a person is not one
    let alight = Seen { flame: Flame::Steady, flue_c: Some(150.0), ..lit() };
    let mut burning = Burning::resumed(State::Working, &alight, &sound());
    quiet(&mut burning, Seen { flame: Flame::Out, ..alight }, 3);
    let did = told(&mut burning, Told::Stop, Seen { flame: Flame::Out, ..alight });
    assert_eq!(did.state, State::BurningOff(None), "Stop waited for the flame's clock");
    assert!(!did.auger);
}

#[test]
fn a_board_that_comes_up_remembering_work_over_no_flame_and_a_cold_flue_waits_for_the_flame() {
    // the live lighting of 24.09, 11:38:55: "came up remembering working, sees flame Out, flue 23.5
    // - goes to burning off", and the fire was burning; half a minute later the pin read 0.6-0.9 V
    let dark = Seen { flame: Flame::Out, flue_c: Some(23.5), ..lit() };
    let mut burning = Burning::resumed(State::Working, &dark, &sound());
    let t = sound();
    let waited: Vec<Doing> = (0..120).map(|_| burning.step(Told::Nothing, dark, 1.0, &t)).collect();
    let left = waited.iter().position(|d| d.state != State::Rising);
    assert_eq!(left, None, "a board that remembers a fire gave it up at its {:?} dark second", left.map(|s| s + 1));
    assert!(waited.iter().all(|d| !d.auger), "the auger turned while the board waited for the flame");
    let gone = burning.step(Told::Nothing, dark, 1.0, &t);
    assert_eq!(gone.state, State::BurningOff(Some(Why::FlameLost)), "the 121st second without a flame");
}

#[test]
fn a_board_that_comes_up_over_no_flame_and_then_sees_one_carries_the_fire_into_work() {
    let dark = Seen { flame: Flame::Out, flue_c: Some(23.5), ..lit() };
    let back = Seen { flame: Flame::Steady, flue_c: Some(23.5), ..lit() };
    let mut burning = Burning::resumed(State::Working, &dark, &sound());
    quiet(&mut burning, dark, 30);
    let t = sound();
    let after: Vec<Doing> = (0..80).map(|_| burning.step(Told::Nothing, back, 1.0, &t)).collect();
    let work = after.iter().position(|d| d.state == State::Working).expect("the fire the board waited for never came to work");
    assert!(after[..work].iter().all(|d| d.state == State::Rising && !d.auger), "before the work: {:?}", &after[..work]);
    assert_eq!(work + 1, 10 + 60, "the watch and the climb after the flame showed took {} s", work + 1);
}

#[test]
fn a_board_that_comes_up_remembering_a_stop_over_no_sign_of_fire_stays_stopped() {
    // the pair: the wait is for a fire the board remembers, and a stop remembers none
    let dark = Seen { flame: Flame::Out, flue_c: Some(23.5), ..lit() };
    let mut burning = Burning::resumed(State::Stop, &dark, &sound());
    assert_eq!(burning.state(), State::Stop);
    let did = quiet(&mut burning, dark, 5);
    assert_eq!((did.state, did.fan, did.auger), (State::Stop, Asked::Stop, false));
}

#[test]
fn a_board_that_comes_up_in_the_middle_of_burning_off_starts_burning_off_afresh() {
    let burning = Burning::resumed(State::BurningOff(Some(Why::NeverLit(3))), &cold(), &sound());
    assert_eq!(
        burning.state(),
        State::BurningOff(Some(Why::NeverLit(3))),
        "from the top of the procedure, and still carrying why it was in it"
    );
}

#[test]
fn a_board_that_comes_up_in_the_middle_of_lighting_never_carries_on_lighting() {
    for was in [State::Kindling(1), State::Kindling(3)] {
        let burning = Burning::resumed(was, &cold(), &sound());
        assert_eq!(burning.state(), State::BurningOff(None), "a pot that may hold a dose is cleared, not lit");
    }
}

#[test]
fn every_state_the_board_can_remember_is_answered_by_the_sensors_and_not_by_the_memory() {
    // eight remembered states against three shapes of reading. The table is here whole rather than as eight
    // rows, because what it proves is a RULE and a rule is only visible in all of its cases at once - and a
    // mismatch shows every line that moved, not the first.
    let every = [
        State::Stop,
        State::Purge,
        State::Kindling(2),
        State::Rising,
        State::Working,
        State::Smouldering,
        State::BurningOff(Some(Why::NeverLit(3))),
        State::Fault(Why::NoFuel),
    ];
    let readings = [
        ("пламя есть, труба горячая", Seen { flame: Flame::Steady, flue_c: Some(150.0), ..lit() }),
        ("пламя есть, труба холодная", Seen { flame: Flame::Steady, flue_c: Some(20.0), ..lit() }),
        ("пламя есть, трубу не читают", Seen { flame: Flame::Steady, flue_c: None, ..lit() }),
        ("пламени нет, труба горячая", Seen { flame: Flame::Out, flue_c: Some(150.0), ..lit() }),
        ("пламени нет, труба холодная", Seen { flame: Flame::Out, flue_c: Some(20.0), ..lit() }),
    ];
    let mut got = Vec::new();
    for (words, seen) in readings {
        for was in every {
            let mut burning = Burning::resumed(was, &seen, &sound());
            let did = burning.step(Told::Nothing, seen, 1.0, &sound());
            got.push(format!(
                "{words} | {was:?} -> {:?} | {:?} | шнек {}",
                burning.state(),
                did.fan,
                if did.auger { "подаёт" } else { "стоит" }
            ));
        }
    }
    let want = [
        // a flame, whatever the flue: a memory that was carrying a fire carries it on as it was - the work fed from the
        // first second (the smouldering here turns to work because the water is cold); a memory
        // that was not carrying one goes out - with air
        "пламя есть, труба горячая | Stop -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, труба горячая | Purge -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, труба горячая | Kindling(2) -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, труба горячая | Rising -> Rising | Hold(52.15) | шнек стоит",
        "пламя есть, труба горячая | Working -> Working | Hold(120.0) | шнек подаёт",
        "пламя есть, труба горячая | Smouldering -> Working | Hold(120.0) | шнек подаёт",
        "пламя есть, труба горячая | BurningOff(Some(NeverLit(3))) -> BurningOff(Some(NeverLit(3))) | Hold(120.0) | шнек стоит",
        "пламя есть, труба горячая | Fault(NoFuel) -> BurningOff(Some(NoFuel)) | Hold(120.0) | шнек стоит",
        "пламя есть, труба холодная | Stop -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, труба холодная | Purge -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, труба холодная | Kindling(2) -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, труба холодная | Rising -> Rising | Hold(52.15) | шнек стоит",
        "пламя есть, труба холодная | Working -> Working | Hold(120.0) | шнек подаёт",
        "пламя есть, труба холодная | Smouldering -> Working | Hold(120.0) | шнек подаёт",
        "пламя есть, труба холодная | BurningOff(Some(NeverLit(3))) -> BurningOff(Some(NeverLit(3))) | Hold(120.0) | шнек стоит",
        "пламя есть, труба холодная | Fault(NoFuel) -> BurningOff(Some(NoFuel)) | Hold(120.0) | шнек стоит",
        // a flue nobody can read is one witness short like any other
        "пламя есть, трубу не читают | Stop -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, трубу не читают | Purge -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, трубу не читают | Kindling(2) -> BurningOff(None) | Hold(120.0) | шнек стоит",
        "пламя есть, трубу не читают | Rising -> Rising | Hold(52.15) | шнек стоит",
        "пламя есть, трубу не читают | Working -> Working | Hold(120.0) | шнек подаёт",
        "пламя есть, трубу не читают | Smouldering -> Working | Hold(120.0) | шнек подаёт",
        "пламя есть, трубу не читают | BurningOff(Some(NeverLit(3))) -> BurningOff(Some(NeverLit(3))) | Hold(120.0) | шнек стоит",
        "пламя есть, трубу не читают | Fault(NoFuel) -> BurningOff(Some(NoFuel)) | Hold(120.0) | шнек стоит",
        // no flame, but a flue a cold pot cannot make: the shape a fallen-off flame sensor takes over a pot
        // that is burning, and it is answered with air like any other sign of fire
        "пламени нет, труба горячая | Stop -> BurningOff(None) | Purge | шнек стоит",
        "пламени нет, труба горячая | Purge -> BurningOff(None) | Purge | шнек стоит",
        "пламени нет, труба горячая | Kindling(2) -> BurningOff(None) | Purge | шнек стоит",
        "пламени нет, труба горячая | Rising -> Rising | Hold(51.0) | шнек стоит",
        "пламени нет, труба горячая | Working -> Rising | Hold(51.0) | шнек стоит",
        "пламени нет, труба горячая | Smouldering -> Rising | Hold(51.0) | шнек стоит",
        "пламени нет, труба горячая | BurningOff(Some(NeverLit(3))) -> BurningOff(Some(NeverLit(3))) | Purge | шнек стоит",
        "пламени нет, труба горячая | Fault(NoFuel) -> BurningOff(Some(NoFuel)) | Purge | шнек стоит",
        "пламени нет, труба холодная | Stop -> Stop | Stop | шнек стоит",
        "пламени нет, труба холодная | Purge -> Stop | Stop | шнек стоит",
        "пламени нет, труба холодная | Kindling(2) -> BurningOff(None) | Purge | шнек стоит",
        "пламени нет, труба холодная | Rising -> Rising | Hold(51.0) | шнек стоит",
        "пламени нет, труба холодная | Working -> Rising | Hold(51.0) | шнек стоит",
        "пламени нет, труба холодная | Smouldering -> Rising | Hold(51.0) | шнек стоит",
        "пламени нет, труба холодная | BurningOff(Some(NeverLit(3))) -> BurningOff(Some(NeverLit(3))) | Purge | шнек стоит",
        "пламени нет, труба холодная | Fault(NoFuel) -> BurningOff(Some(NoFuel)) | Purge | шнек стоит",
    ];
    assert_eq!(got.join("\n"), want.join("\n"), "the table of what a boot decides has moved");
    // THE ONE ROAD OUT OF A BOOT THAT POURS FUEL is a fire the board remembers feeding and sees at its first second;
    // a dark pot, and any memory that was not feeding, pour nothing
    let feeds = |row: &&str| row.starts_with("пламя есть") && (row.contains("| Working ->") || row.contains("| Smouldering ->"));
    assert!(
        want.iter().all(|row| row.ends_with("шнек подаёт") == feeds(row)),
        "a boot poured fuel on a road other than a seen fire it was feeding, or did not feed one it was"
    );
}

#[test]
fn a_board_that_comes_up_from_a_stop_stays_stopped() {
    for was in [State::Stop, State::Purge] {
        let burning = Burning::resumed(was, &cold(), &sound());
        assert_eq!(burning.state(), State::Stop);
    }
}

#[test]
fn a_board_that_comes_up_from_a_fault_clears_the_pot_before_it_shows_the_fault_again() {
    let burning = Burning::resumed(State::Fault(Why::FlameLost), &cold(), &sound());
    assert_eq!(burning.state(), State::BurningOff(Some(Why::FlameLost)));
}

// ---------------------------------------------------------------- the switch comes back

/// A board that came up remembering WAS and found KEPT of the switch, reading SEEN.
fn came_up(was: State, kept: Option<&str>, seen: Seen) -> Burning {
    Burning::resumed_kept(was, &seen, &sound(), kept)
}

#[test]
fn a_board_that_was_asked_for_heat_lights_itself_over_cold_water_with_nobody_pressing() {
    let mut burning = came_up(State::Stop, Some("on"), cold());
    assert!(burning.wanted(), "the switch is what the person left");
    quiet(&mut burning, cold(), 3);
    assert_eq!(burning.state(), State::Purge, "the water is under the line and heat was asked for: the fire lights, as it did before the power went");
}

#[test]
fn the_same_board_without_the_switch_in_its_memory_does_not_light_over_the_same_water() {
    let mut burning = came_up(State::Stop, None, cold());
    assert!(!burning.wanted());
    quiet(&mut burning, cold(), 120);
    assert_eq!(burning.state(), State::Stop, "a board nobody asked for heat does not light itself because the weather turned");
}

#[test]
fn a_restored_start_lights_only_when_the_water_asks_and_not_on_a_press_of_its_own() {
    for boiler_c in [57.0, 58.0, 59.9, 60.0, 75.0] {
        let mut burning = came_up(State::Stop, Some("on"), Seen { boiler_c: Some(boiler_c), ..cold() });
        quiet(&mut burning, Seen { boiler_c: Some(boiler_c), ..cold() }, 60);
        assert_eq!(burning.state(), State::Stop, "{boiler_c} °C is inside the hysteresis or over the setpoint: nothing asks for heat");
    }
}

#[test]
fn a_board_that_was_stopped_stays_stopped_over_cold_water() {
    let mut burning = came_up(State::Stop, Some("off"), cold());
    assert!(!burning.wanted());
    quiet(&mut burning, cold(), 120);
    assert_eq!(burning.state(), State::Stop, "a burner that was switched off does not switch itself on");
}

#[test]
fn a_fault_stays_a_fault_over_a_restored_start_and_cold_water_and_a_stop_acknowledges_it() {
    let t = sound();
    let mut burning = came_up(State::Fault(Why::FlameLost), Some("on"), cold());
    assert_eq!(burning.state(), State::BurningOff(Some(Why::FlameLost)), "the pot is cleared first, as it always was");
    let mut seconds = 0;
    while burning.state() != State::Fault(Why::FlameLost) {
        burning.step(Told::Nothing, cold(), 1.0, &t);
        seconds += 1;
        assert!(seconds < 300, "the burning off never reached the fault");
    }
    quiet(&mut burning, cold(), 600);
    assert_eq!(burning.state(), State::Fault(Why::FlameLost), "heat asked for, cold water, ten minutes: a restart does not clear a fault");
    told(&mut burning, Told::Stop, cold());
    assert_eq!(burning.state(), State::Stop, "a person's Stop is what clears it");
    assert!(!burning.wanted(), "and the Stop is the switch now");
    quiet(&mut burning, cold(), 120);
    assert_eq!(burning.state(), State::Stop);
}

#[test]
fn a_restored_start_carries_a_fire_it_sees_on_as_it_was() {
    let burning = came_up(State::Working, Some("on"), lit());
    assert_eq!(burning.state(), State::Working);
    assert!(burning.wanted());
}

#[test]
fn the_protections_do_not_wait_for_the_switch_whatever_it_was() {
    for kept in [None, Some("off"), Some("on")] {
        let hot = Seen { boiler_c: Some(96.0), ..cold() };
        let mut burning = came_up(State::Stop, kept, hot);
        assert!(burning.step(Told::Nothing, hot, 1.0, &sound()).alarm, "{kept:?}: the alarm is the water's, from the first second");
        let hot_fire = Seen { boiler_c: Some(90.0), ..lit() };
        let mut burning = came_up(State::Working, kept, hot_fire);
        burning.step(Told::Nothing, hot_fire, 1.0, &sound());
        assert!(matches!(burning.state(), State::BurningOff(Some(Why::Overheated(_)))), "{kept:?}: a fire over the overheating is burnt off, restored or not");
    }
}

// ---------------------------------------------------------------- the dose boundary

#[test]
fn a_dose_under_way_finishes_on_the_number_it_started_with() {
    let mut burning = working();
    let t = sound();
    // wait for a dose to begin
    while !burning.step(Told::Nothing, lit(), 1.0, &t).auger {}
    // and cut it in half while it runs - through the door, the way a person's number reaches the burner
    let cut = Tuning { step: Step { feed: Feed { on_s: 1.0, ..tuning().step.feed }, ..tuning().step }, ..tuning() };
    let t = cut.sound().expect("a second of auger is a dose");
    let mut fed = 1;
    while burning.step(Told::Nothing, lit(), 1.0, &t).auger {
        fed += 1;
        assert!(fed < 10, "the dose never ended");
    }
    assert!(fed >= 4, "the dose was 4.6 s when it began and it ran {fed} s: a person changed it mid-pour");
}

#[test]
fn the_dose_after_the_boundary_is_the_new_one() {
    let mut burning = working();
    let t = Tuning { step: Step { feed: Feed { on_s: 1.0, off_s: 2.0 }, ..tuning().step }, ..tuning() }
        .sound()
        .expect("one second on and two off is a pair");
    // past the first boundary, where the new numbers take
    for _ in 0..20 {
        burning.step(Told::Nothing, lit(), 1.0, &t);
    }
    let mut fed = 0;
    for _ in 0..3 {
        if burning.step(Told::Nothing, lit(), 1.0, &t).auger {
            fed += 1;
        }
    }
    assert_eq!(fed, 1, "one second of auger in every three is what the new pair asks for");
}

// ---------------------------------------------------------------- what may never be fed

#[test]
fn nothing_is_fed_into_a_pot_that_is_being_cleared_or_given_up_on() {
    // one long row through every state there is, asking the one question at every second of it
    let mut burning = Burning::new();
    let mut seen = cold();
    told(&mut burning, Told::Start, seen);
    let t = sound();
    let mut seconds = 0;
    let mut seen_states = 0;
    for second in 0..1200 {
        // a fire that lights at the third minute, holds, and is taken away again at the tenth
        seen.flame = if (180..600).contains(&second) { Flame::Steady } else { Flame::Out };
        seen.flue_c = Some(if (180..600).contains(&second) { 150.0 } else { 20.0 });
        let did = burning.step(Told::Nothing, seen, 1.0, &t);
        let must_not_feed =
            matches!(did.state, State::Stop | State::Purge | State::BurningOff(_) | State::Fault(_));
        assert!(
            !(must_not_feed && did.auger),
            "the auger turned in {:?} at second {second}",
            did.state
        );
        assert!(
            !(must_not_feed && did.igniter),
            "the igniter was hot in {:?} at second {second}",
            did.state
        );
        if matches!(did.state, State::BurningOff(_) | State::Fault(_)) {
            seen_states += 1;
        }
        seconds += 1;
    }
    assert_eq!(seconds, 1200);
    assert!(seen_states > 0, "the row never reached a burning off, so it asked nothing");
}

// ---------------------------------------------------------------- what a person is told

#[test]
fn every_fault_names_the_thing_that_did_not_add_up_and_never_a_code() {
    // every reason there is: a new one that is not listed below stops this match from building
    let _listed = |w: Why| match w {
        Why::NeverLit(_) | Why::FlameLost | Why::FlameNotBack(_) | Why::NoFuel | Why::NoDraught | Why::LowPressure | Why::Overheated(_) | Why::Unconfigured | Why::DraughtSilent | Why::FlameSilent | Why::WaterSilent | Why::ReturnSilent | Why::NoFronts => (),
    };
    for why in [
        Why::NeverLit(3),
        Why::FlameLost,
        Why::FlameNotBack(3),
        Why::NoFuel,
        Why::NoDraught,
        Why::LowPressure,
        Why::Overheated(96.0),
        Why::Unconfigured,
        Why::DraughtSilent,
        Why::FlameSilent,
        Why::WaterSilent,
        Why::ReturnSilent,
        Why::NoFronts,
    ] {
        let said = automaton::words(why);
        assert!(!said.is_empty(), "{why:?} says nothing to a person");
        assert!(said.chars().any(|c| c.is_alphabetic()), "{why:?} answers {said:?}, which is a code and not words");
    }
}

#[test]
fn a_state_says_which_attempt_it_is_on_while_it_is_lighting() {
    assert!(automaton::state_words(State::Kindling(2)).contains('2'), "Plum shows the attempt and so do we");
    // in the page's own language: every other word on it is English
    assert!(automaton::state_words(State::Fault(Why::NoFuel)).contains("fuel"));
    assert!(automaton::state_words(State::Kindling(2)).is_ascii(), "{}", automaton::state_words(State::Kindling(2)));
}

// ---------------------------------------------------------------- the door the numbers come through

/// A step of the factory tuning with one number replaced, so that a row names one thing at a time.
fn with_pa(step: Step, pa: f32) -> Step {
    Step { pa, ..step }
}

fn with_on(step: Step, on_s: f32) -> Step {
    Step { feed: Feed { on_s, ..step.feed }, ..step }
}

fn with_off(step: Step, off_s: f32) -> Step {
    Step { feed: Feed { off_s, ..step.feed }, ..step }
}

/// Every number that has a limit, each broken on its own, and the field the door is expected to name.
fn broken() -> Vec<(&'static str, Tuning)> {
    let t = tuning();
    vec![
        ("purge_before_s", Tuning { purge_before_s: 0.0, ..t }),
        ("purge_after_s", Tuning { purge_after_s: 0.0, ..t }),
        ("igniter_s", Tuning { igniter_s: 0.0, ..t }),
        ("wait_for_flame_s", Tuning { wait_for_flame_s: 0.0, ..t }),
        ("tries", Tuning { tries: 0, ..t }),
        ("dose_after_first", Tuning { dose_after_first: 0.0, ..t }),
        ("dose_after_first", Tuning { dose_after_first: 1.5, ..t }),
        ("rise_s", Tuning { rise_s: 0.0, ..t }),
        ("flame_lost_s", Tuning { flame_lost_s: 0.0, ..t }),
        ("smoulder_s", Tuning { smoulder_s: -1.0, ..t }),
        ("burn_off_s", Tuning { burn_off_s: 0.0, ..t }),
        // the ceiling of burning off under its own floor: the floor would never bite, and the pot that
        // certainly holds unburnt pellets would get the shortest blow of all
        ("burn_off_s", Tuning { burn_off_s: 10.0, purge_after_s: 20.0, ..t }),
        ("burn_out_min_s", Tuning { burn_out_min_s: -1.0, ..t }),
        // a least burning out longer than the most: burning out would end on its ceiling before the floor was reached
        ("burn_off_s", Tuning { burn_out_min_s: 150.0, ..t }),
        ("relight_s[0]", Tuning { relight_s: [-1.0, 10.0, 10.0], ..t }),
        ("relight_s[2]", Tuning { relight_s: [20.0, 10.0, -1.0], ..t }),
        ("relight_hot_c", Tuning { relight_hot_c: 0.0, ..t }),
        // a fire that holds its flue is not obliged to warm it: a slope above nought would call every quiet fire lost
        ("pump_on_c", Tuning { pump_on_c: 0.0, ..t }),
        // the pump stands off some degrees under where it starts: no degrees is a pump that never chatters and never stops
        ("pump_off_below_c", Tuning { pump_off_below_c: 0.0, ..t }),
        ("flue_holds_c", Tuning { flue_holds_c: 0.0, ..t }),
        ("flue_cold_c", Tuning { fire_numbers: FireNumbers { flue_cold_c: 0.0, ..t.fire_numbers }, ..t }),
        ("flue_fall_c", Tuning { fire_numbers: FireNumbers { flue_fall_c: 0.0, ..t.fire_numbers }, ..t }),
        ("flue_fall_s", Tuning { fire_numbers: FireNumbers { flue_fall_s: 0.0, ..t.fire_numbers }, ..t }),
        ("lit_s", Tuning { fire_numbers: FireNumbers { lit_s: 0.0, ..t.fire_numbers }, ..t }),
        ("want_c", Tuning { want_c: 0.0, ..t }),
        ("hysteresis_c", Tuning { hysteresis_c: 0.0, ..t }),
        // a hysteresis as wide as the setpoint puts the relighting line at zero degrees
        ("hysteresis_c", Tuning { hysteresis_c: 60.0, ..t }),
        // overheating before the setpoint is reached: the burner could never hold the water it is set to
        ("too_hot_c", Tuning { too_hot_c: 50.0, ..t }),
        // a person signalled before the burner has even stopped feeding
        ("alarm_c", Tuning { alarm_c: 80.0, ..t }),
        ("sure_s", Tuning { sure_s: 0.0, ..t }),
        ("spin_up_s", Tuning { spin_up_s: 0.0, ..t }),
        ("kindling.pa", Tuning { kindling: Lighting { pa: -1.0, ..t.kindling }, ..t }),
        ("kindling.dose_s", Tuning { kindling: Lighting { dose_s: 0.0, ..t.kindling }, ..t }),
        ("step.pa", Tuning { step: with_pa(t.step, -1.0), ..t }),
        ("step.feed.on_s", Tuning { step: with_on(t.step, 0.0), ..t }),
        ("step.feed.off_s", Tuning { step: with_off(t.step, 0.0), ..t }),
        ("smoulder.pa", Tuning { smoulder: with_pa(t.smoulder, -1.0), ..t }),
        ("smoulder.feed.on_s", Tuning { smoulder: with_on(t.smoulder, 0.0), ..t }),
        ("smoulder.feed.off_s", Tuning { smoulder: with_off(t.smoulder, 0.0), ..t }),
    ]
}

#[test]
fn every_number_with_a_limit_is_refused_by_the_name_of_its_own_field() {
    // one comparison rather than thirty-two, because what is proved is that the door covers the RECORD -
    // and a hole in it is one line of this answer, visible beside all the others
    let got: Vec<String> = broken()
        .into_iter()
        .map(|(field, t)| match t.sound() {
            Ok(_) => format!("{field} -> ПРИНЯТО"),
            Err(refused) => format!("{field} -> {}", refused.field),
        })
        .collect();
    let want: Vec<String> = broken().into_iter().map(|(field, _)| format!("{field} -> {field}")).collect();
    assert_eq!(got.join("\n"), want.join("\n"), "the door let a number through, or named the wrong field");
}

/// The refusals of the door as a person would break them, and what each is expected to say: the field, the code, the number that stood there and, for an order of two numbers, the other one and its value.
#[allow(clippy::type_complexity)]
fn refusals() -> Vec<(&'static str, Tuning, &'static str, Option<f32>, Option<&'static str>, Option<f32>)> {
    let t = tuning();
    vec![
        ("purge_before_s", Tuning { purge_before_s: 0.0, ..t }, "above_nought", Some(0.0), None, None),
        ("smoulder_s", Tuning { smoulder_s: -1.0, ..t }, "not_below_nought", Some(-1.0), None, None),
        ("relight_s[0]", Tuning { relight_s: [-1.0, 10.0, 10.0], ..t }, "not_below_nought", Some(-1.0), None, None),
        ("tries", Tuning { tries: 0, ..t }, "no_tries", Some(0.0), None, None),
        ("dose_after_first", Tuning { dose_after_first: 1.5, ..t }, "dose_over_first", Some(1.5), None, None),
        ("burn_off_s", Tuning { burn_off_s: 10.0, purge_after_s: 20.0, ..t }, "not_below", Some(10.0), Some("purge_after_s"), Some(20.0)),
        ("hysteresis_c", Tuning { hysteresis_c: 60.0, ..t }, "below", Some(60.0), Some("want_c"), Some(t.want_c)),
        ("too_hot_c", Tuning { too_hot_c: 50.0, ..t }, "above", Some(50.0), Some("want_c"), Some(t.want_c)),
        ("alarm_c", Tuning { alarm_c: t.too_hot_c, ..t }, "above", Some(t.too_hot_c), Some("too_hot_c"), Some(t.too_hot_c)),
        ("kindling.pa", Tuning { kindling: Lighting { pa: 0.0, ..t.kindling }, ..t }, "kindling_no_air", Some(0.0), None, None),
        ("wait_for_flame_s", Tuning { wait_for_flame_s: f32::NAN, ..t }, "not_a_number", None, None, None),
        ("wait_for_flame_s", Tuning { wait_for_flame_s: f32::INFINITY, ..t }, "infinite", None, None, None),
    ]
}

#[test]
fn a_refusal_is_a_code_the_field_and_the_numbers_and_never_a_sentence() {
    for (field, t, code, value, than, limit) in refusals() {
        let refused: Refused = t.sound().expect_err("this one is broken on purpose");
        assert_eq!(
            (refused.field.as_str(), refused.code, refused.value, refused.than, refused.limit),
            (field, code, value, than, limit),
            "{field}"
        );
    }
}

#[test]
fn every_code_a_refusal_can_carry_is_in_the_list_the_card_names_and_every_code_in_it_is_reached() {
    for (field, t) in broken() {
        let refused = t.sound().expect_err("this one is broken on purpose");
        assert!(automaton::REFUSALS.contains(&refused.code), "{field}: the code {:?} is not in the list the card says", refused.code);
    }
    let reached: Vec<&str> = refusals().into_iter().map(|(_, _, code, ..)| code).collect();
    for code in automaton::REFUSALS {
        assert!(reached.contains(&code), "the code {code:?} is in the list and no row reaches it");
    }
}

#[test]
fn nan_is_refused_wherever_a_number_of_the_fire_stands() {
    // **EVERY "not less than zero" LETS IT THROUGH**, because a comparison with it is false both ways, so
    // this row walks the whole record rather than the fields somebody remembered
    let t = tuning();
    let every: Vec<(&str, Tuning)> = vec![
        ("purge_before_s", Tuning { purge_before_s: f32::NAN, ..t }),
        ("purge_after_s", Tuning { purge_after_s: f32::NAN, ..t }),
        ("igniter_s", Tuning { igniter_s: f32::NAN, ..t }),
        ("wait_for_flame_s", Tuning { wait_for_flame_s: f32::NAN, ..t }),
        ("dose_after_first", Tuning { dose_after_first: f32::NAN, ..t }),
        ("rise_s", Tuning { rise_s: f32::NAN, ..t }),
        ("flame_lost_s", Tuning { flame_lost_s: f32::NAN, ..t }),
        ("smoulder_s", Tuning { smoulder_s: f32::NAN, ..t }),
        ("burn_off_s", Tuning { burn_off_s: f32::NAN, ..t }),
        ("burn_out_min_s", Tuning { burn_out_min_s: f32::NAN, ..t }),
        ("relight_s[0]", Tuning { relight_s: [f32::NAN, 10.0, 10.0], ..t }),
        ("relight_s[1]", Tuning { relight_s: [20.0, f32::NAN, 10.0], ..t }),
        ("relight_s[2]", Tuning { relight_s: [20.0, 10.0, f32::NAN], ..t }),
        ("relight_hot_c", Tuning { relight_hot_c: f32::NAN, ..t }),
        ("pump_on_c", Tuning { pump_on_c: f32::NAN, ..t }),
        ("flue_holds_c", Tuning { flue_holds_c: f32::NAN, ..t }),
        ("flue_cold_c", Tuning { fire_numbers: FireNumbers { flue_cold_c: f32::NAN, ..t.fire_numbers }, ..t }),
        ("flue_fall_c", Tuning { fire_numbers: FireNumbers { flue_fall_c: f32::NAN, ..t.fire_numbers }, ..t }),
        ("flue_fall_s", Tuning { fire_numbers: FireNumbers { flue_fall_s: f32::NAN, ..t.fire_numbers }, ..t }),
        ("lit_s", Tuning { fire_numbers: FireNumbers { lit_s: f32::NAN, ..t.fire_numbers }, ..t }),
        ("want_c", Tuning { want_c: f32::NAN, ..t }),
        ("hysteresis_c", Tuning { hysteresis_c: f32::NAN, ..t }),
        ("too_hot_c", Tuning { too_hot_c: f32::NAN, ..t }),
        ("alarm_c", Tuning { alarm_c: f32::NAN, ..t }),
        ("sure_s", Tuning { sure_s: f32::NAN, ..t }),
        ("spin_up_s", Tuning { spin_up_s: f32::NAN, ..t }),
        ("kindling.pa", Tuning { kindling: Lighting { pa: f32::NAN, ..t.kindling }, ..t }),
        ("kindling.dose_s", Tuning { kindling: Lighting { dose_s: f32::NAN, ..t.kindling }, ..t }),
        ("step.pa", Tuning { step: with_pa(t.step, f32::NAN), ..t }),
        ("step.feed.on_s", Tuning { step: with_on(t.step, f32::NAN), ..t }),
        ("step.feed.off_s", Tuning { step: with_off(t.step, f32::NAN), ..t }),
        ("smoulder.pa", Tuning { smoulder: with_pa(t.smoulder, f32::NAN), ..t }),
        ("smoulder.feed.on_s", Tuning { smoulder: with_on(t.smoulder, f32::NAN), ..t }),
        ("smoulder.feed.off_s", Tuning { smoulder: with_off(t.smoulder, f32::NAN), ..t }),
        ("pump_off_below_c", Tuning { pump_off_below_c: f32::NAN, ..t }),
        ("power.pa", Tuning { power: with_pa(t.power, f32::NAN), ..t }),
        ("power.feed.on_s", Tuning { power: with_on(t.power, f32::NAN), ..t }),
        ("power.feed.off_s", Tuning { power: with_off(t.power, f32::NAN), ..t }),
    ];
    let got: Vec<String> = every
        .into_iter()
        .map(|(field, t)| match t.sound() {
            Ok(_) => format!("{field} -> ПРИНЯТО"),
            Err(refused) => format!("{field} -> {}", refused.field),
        })
        .collect();
    let want: Vec<String> = got.iter().map(|row| {
        let field = row.split(" -> ").next().unwrap();
        format!("{field} -> {field}")
    }).collect();
    assert_eq!(got.join("\n"), want.join("\n"), "NaN went through somewhere");
    assert_eq!(got.len(), 38, "the record holds thirty-eight numbers that could be NaN, and every one is walked here");
}

#[test]
fn infinity_is_refused_wherever_nan_is() {
    // the other end of the same hole: an infinity passes every "not less than zero" too, and a burner that
    // waits for a flame for ever is a burner that waits for a flame for ever
    for (field, t) in [
        ("wait_for_flame_s", Tuning { wait_for_flame_s: f32::INFINITY, ..tuning() }),
        ("burn_off_s", Tuning { burn_off_s: f32::INFINITY, ..tuning() }),
        ("sure_s", Tuning { sure_s: f32::INFINITY, ..tuning() }),
        // a window that never closes and a flame that is never held long enough are the same endless wait
        ("flue_fall_s", Tuning { fire_numbers: FireNumbers { flue_fall_s: f32::INFINITY, ..tuning().fire_numbers }, ..tuning() }),
        ("lit_s", Tuning { fire_numbers: FireNumbers { lit_s: f32::INFINITY, ..tuning().fire_numbers }, ..tuning() }),
    ] {
        let refused = t.sound().expect_err("an endless number is not a number of this fire");
        assert_eq!(refused.field, field);
    }
}

#[test]
fn the_zeroes_that_mean_something_are_accepted() {
    // **A FLAT RULE AGAINST ZERO WOULD BREAK A WORKING THING.** These two zeroes are the design saying
    // itself: no supervision at all (Plum, manual 8.6), and smouldering on what the chimney pulls.
    Tuning { smoulder_s: 0.0, ..tuning() }.sound().expect("a smouldering time of nothing skips smouldering");
    Tuning { smoulder: with_pa(tuning().smoulder, 0.0), ..tuning() }
        .sound()
        .expect("a smouldering draught of nothing is the chimney doing the work");
}

#[test]
fn the_numbers_this_burner_actually_runs_on_are_accepted() {
    tuning().sound().expect("the factory's own numbers");
}

#[test]
fn a_refused_number_leaves_the_burner_burning_by_the_ones_it_had() {
    let held = sound();
    let before = *held.numbers();
    let _ = Tuning { sure_s: 0.0, ..tuning() }.sound().expect_err("refused");
    assert_eq!(*held.numbers(), before, "a refusal hands back nothing, so nothing of the old could have moved");
    // and the burner goes on burning by them
    let mut burning = working_by(&held);
    assert_eq!(burning.step(Told::Nothing, lit(), 1.0, &held).state, State::Working);
}

// ---------------------------------------------------------------- a fan standing on purpose reads no draught
//
// While the plug heats the fan stands by design, and a standing fan gives
// the draught sensor nothing to read - `Under`, "no number". The automaton asked for a number all the same, and
// kindling fell into NoDraught on its first second. The draught is needed by a second in which the fan is given a
// setpoint to hold, and by no other.

/// A burner taken through its purge onto the first second of the first attempt.
fn kindling() -> Burning {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    let did = quiet(&mut burning, cold(), 30);
    assert_eq!(did.state, State::Kindling(1), "the scaffolding did not reach kindling");
    burning
}

#[test]
fn the_dose_pours_in_still_air_and_a_draught_sensor_with_nothing_to_read_is_no_fault() {
    let mut burning = kindling();
    let still = Seen { draught_pa: None, ..cold() };
    let did = quiet(&mut burning, still, 31);
    assert_eq!(did.state, State::Kindling(1), "the attempt ended while the dose was pouring in still air");
    assert!(did.auger && !did.igniter, "the dose pours and the plug waits for air");
    assert_eq!(did.fan, Asked::Stop, "the fan stands while the dose pours");
}

#[test]
fn once_the_air_comes_back_a_draught_sensor_with_no_number_ends_the_attempt() {
    // the pair of the spin-up below: past the seconds the fan is given, no number is no number
    let mut burning = kindling();
    let still = Seen { draught_pa: None, ..cold() };
    quiet(&mut burning, cold(), 32 + 11);
    let did = quiet(&mut burning, still, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoDraught)), "air is held again and nothing measures it");
}

// ---------------------------------------------------------------- a fan asked for air has not given it yet
//
// On the board, 23.09 (the idle run): the plug heats in still air, and in the very second the
// fan was given 51 Pa the automaton asked the sensor for them - from a fan that had not moved. Every attempt at
// lighting fell into NoDraught at +60 s. A fan turning up from standing is given `spin_up_s` before its draught is
// asked for, wherever it stood: after the plug, and after smouldering on the chimney's own pull.

#[test]
fn a_fan_turning_up_after_the_plug_is_given_its_seconds_before_the_draught_is_asked() {
    let mut burning = kindling();
    let still = Seen { draught_pa: None, ..cold() };
    quiet(&mut burning, still, 32);
    let did = quiet(&mut burning, still, 9);
    assert_eq!(did.state, State::Kindling(1), "the attempt ended while the fan was still turning up");
    assert_eq!(did.fan, Asked::Hold(51.0), "the attempt is not yet at its air, and the row proves nothing");
}

#[test]
fn a_fan_that_turned_up_and_still_gives_no_number_ends_the_attempt() {
    let mut burning = kindling();
    let still = Seen { draught_pa: None, ..cold() };
    quiet(&mut burning, still, 32);
    let did = quiet(&mut burning, still, 11);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoDraught)), "a fan given its seconds and still unmeasured was fed air");
}

#[test]
fn a_fire_leaving_smouldering_gives_its_fan_the_same_seconds() {
    let mut burning = working();
    let warm = Seen { boiler_c: Some(61.0), ..lit() };
    quiet(&mut burning, warm, 1);
    let did = quiet(&mut burning, Seen { draught_pa: None, ..warm }, 5);
    assert_eq!(did.state, State::Smouldering, "the scaffolding did not reach smouldering");
    let cool = Seen { boiler_c: Some(50.0), draught_pa: None, ..lit() };
    let did = quiet(&mut burning, cool, 5);
    assert!(!matches!(did.state, State::BurningOff(_)), "the fire went out while its fan was turning up: {:?}", did.state);
    assert!(matches!(did.fan, Asked::Hold(pa) if pa > 0.0), "the air did not come back, and the row proves nothing: {:?}", did.fan);
    // and the pair: past the seconds, no number is no number here too
    let did = quiet(&mut burning, cool, 7);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoDraught)));
}

#[test]
fn smouldering_on_no_air_reads_no_draught_either() {
    let mut burning = working();
    let warm = Seen { boiler_c: Some(61.0), ..lit() };
    let did = quiet(&mut burning, warm, 1);
    assert_eq!(did.state, State::Smouldering, "the scaffolding did not reach smouldering");
    let did = quiet(&mut burning, Seen { draught_pa: None, ..warm }, 5);
    assert_eq!(did.state, State::Smouldering, "a fire held on the chimney's own pull asked the draught for a number");
}

// ---------------------------------------------------------------- the clocks a person sees count down

fn clock(did: &Burning, what: &str) -> Option<f32> {
    did.clocks(&sound()).into_iter().find(|c| c.what == what).and_then(|c| c.left_s)
}

#[test]
fn a_purge_says_how_long_until_lighting() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 10);
    assert_eq!(clock(&burning, "lighting in"), Some(20.0));
}

#[test]
fn a_clock_reaches_nought_the_second_its_state_ends_and_not_one_second_before() {
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 10);
    let left = clock(&burning, "lighting in").expect("a purge with no clock") as usize;
    let did = quiet(&mut burning, cold(), left - 1);
    assert_eq!(did.state, State::Purge, "the purge ended before its clock said");
    let did = quiet(&mut burning, cold(), 1);
    assert_eq!(did.state, State::Kindling(1), "the clock ran out and the purge went on");
}

// The same promise for what changes INSIDE an attempt (the stand 24.09 drew the plug "burning" in
// still air): a clock that reads N means its organ is as it is for N - 1 more seconds and changes in the Nth. A clock
// that reads nought while the auger still turns, or the plug still heats without air, tells the page a thing that is
// not so.

/// Steps a second at a time and returns the Doing of each.
fn seconds(burning: &mut Burning, n: usize) -> Vec<Doing> {
    (0..n).map(|_| told(burning, Told::Nothing, cold())).collect()
}

#[test]
fn the_dose_turns_as_long_as_its_clock_says_and_stops_when_it_reads_out() {
    let mut burning = kindling();
    quiet(&mut burning, cold(), 5);
    let left = clock(&burning, "dose ends in").expect("an attempt with no dose clock") as usize;
    let did = seconds(&mut burning, left);
    assert!(did[..left - 1].iter().all(|d| d.auger), "the auger stopped before its clock said: {left} s");
    assert!(!did[left - 1].auger, "the clock read {left} s and the auger turned in the second after it");
    // and the air comes in the second the dose ends: the plug burns with it or not at all
    assert_ne!(did[left - 1].fan, Asked::Stop, "the dose ended at {left} s and the fan still stood");
}

#[test]
fn the_plug_burns_as_long_as_its_clock_says_and_goes_off_when_it_reads_out() {
    let mut burning = kindling();
    quiet(&mut burning, cold(), 5);
    let left = clock(&burning, "igniter off in").expect("an attempt with no igniter clock") as usize;
    // the plug is cold through the dose and hot from its end with the air: its clock is read at the end, where it goes off
    let did: Vec<Doing> = (0..left).map(|i| told(&mut burning, Told::Nothing, in_attempt(6 + i))).collect();
    assert!(did[left - 2].igniter, "the plug went off before its clock said: {left} s");
    assert!(!did[left - 1].igniter, "the clock read {left} s and the plug was still hot after it");
}

#[test]
fn lighting_counts_the_igniter_the_dose_and_the_attempt_down() {
    let mut burning = kindling();
    quiet(&mut burning, cold(), 10);
    // the dose and the plug each run through their end second (32, 62) and change in the next; the attempt ends when it
    // reaches its end (152)
    assert_eq!(clock(&burning, "dose ends in"), Some(23.0));
    assert_eq!(clock(&burning, "igniter off in"), Some(53.0));
    assert_eq!(clock(&burning, "attempt ends in"), Some(142.0));
}

// The owner, 23.09: the list of clocks jumped - a clock that ran out went away and the rows under it moved up. A state
// shows the same clocks every second it lasts; one that has run out stays and reads nought.

fn names(did: &Burning) -> Vec<&'static str> {
    did.clocks(&sound()).into_iter().map(|c| c.what).collect()
}

#[test]
fn a_clock_that_has_run_out_stays_and_reads_nought() {
    let mut burning = kindling();
    quiet(&mut burning, cold(), 80);
    assert_eq!(clock(&burning, "igniter off in"), Some(0.0), "the igniter's clock went away when it ran out");
    assert_eq!(clock(&burning, "dose ends in"), Some(0.0), "the dose's clock went away when it ran out");
    assert_eq!(clock(&burning, "attempt ends in"), Some(72.0));
}

#[test]
fn lighting_shows_the_same_clocks_every_second_of_an_attempt() {
    let mut burning = kindling();
    let first = names(&burning);
    assert!(!first.is_empty(), "an attempt with no clocks proves nothing here");
    for second in [1, 9, 20, 30, 60] {
        quiet(&mut burning, cold(), 1);
        let _ = second;
        assert_eq!(names(&burning), first, "the clocks of the attempt changed as it went");
    }
    quiet(&mut burning, cold(), 40);
    assert_eq!(names(&burning), first, "the clocks of the attempt changed as it went");
}

#[test]
fn burning_off_shows_both_of_its_parts_all_the_way() {
    let mut burning = working();
    told(&mut burning, Told::Stop, lit());
    let before = names(&burning);
    assert_eq!(reads(&burning, "blowing clear ends in"), Reads::NotBegun, "the blow has no number behind the burning out, and its row stays");
    quiet(&mut burning, lit(), 20);
    quiet(&mut burning, cold(), 5);
    assert_eq!(names(&burning), before, "the clocks of burning off changed as it went");
    // the blow began in the first dark second, which reads 20 (burning_off_says_how_long_at_most); four more
    assert_eq!(clock(&burning, "blowing clear ends in"), Some(16.0));
}

#[test]
fn a_stop_and_a_fault_count_nothing_down() {
    let burning = Burning::new();
    assert!(burning.clocks(&sound()).is_empty());
}

#[test]
fn burning_off_says_how_long_at_most() {
    let mut burning = working();
    told(&mut burning, Told::Stop, lit());
    quiet(&mut burning, lit(), 20);
    assert_eq!(clock(&burning, "burning out ends in at most"), Some(100.0));
    quiet(&mut burning, cold(), 1);
    assert_eq!(clock(&burning, "burning out ends in at most"), Some(0.0), "the burning out is over once the flame went");
    assert_eq!(clock(&burning, "blowing clear ends in"), Some(20.0), "the blow begins the second the flame went");
}

// ---------------------------------------------------------------- the order of an attempt: the plug lights with air
//
// The owner, 23.09 and 26.09, and Plum 850P2-C 14.1: the dose first, then the fan on the lighting draught with the plug
// on for its seconds - never in still air, which burnt a plug through on 26.09 -, then the plug off and the fan waiting
// for a flame. A flame at any moment of it goes to rising in that second. With the rows' numbers: dose 32 s, plug with
// air 30, wait 90 - an attempt of 152 s.

/// What the attempt that `kindling()` begins does at each of its seconds 1..=SECONDS, with no flame and the fan blowing
/// once it is asked to.
fn attempt(seconds: usize) -> Vec<Doing> {
    let mut burning = kindling();
    (1..=seconds).map(|n| quiet(&mut burning, in_attempt(n), 1)).collect()
}

#[test]
fn an_attempt_pours_its_dose_first_with_the_plug_off_and_the_fan_standing() {
    let did = attempt(32);
    for (i, d) in did.iter().enumerate() {
        assert!(d.auger && !d.igniter && d.fan == Asked::Stop, "second {} of the dose: {d:?}", i + 1);
    }
}

#[test]
fn then_the_plug_burns_with_the_fan_on_the_lighting_draught_for_its_seconds() {
    let did = attempt(62);
    for (i, d) in did.iter().enumerate().skip(32) {
        assert!(!d.auger && d.igniter && d.fan == Asked::Hold(51.0), "second {} of the plug with air: {d:?}", i + 1);
    }
}

#[test]
fn then_the_plug_goes_off_and_the_fan_waits_for_a_flame_and_the_next_attempt_follows() {
    let did = attempt(153);
    for (i, d) in did.iter().enumerate().skip(62).take(89) {
        assert!(!d.auger && !d.igniter && d.fan == Asked::Hold(51.0), "second {} of the wait: {d:?}", i + 1);
    }
    assert_eq!(did[150].state, State::Kindling(1), "the wait ended before its 90 s");
    assert_eq!(did[152].state, State::Kindling(2), "the wait did not end at 152 s of the attempt");
}

#[test]
fn a_plug_never_burns_in_still_air_over_a_whole_cycle() {
    // the board reads the draught the fan was asked for a second ago: 51 Pa while it is asked to blow, 0 while it stands
    let mut burning = Burning::new();
    let mut d = told(&mut burning, Told::Start, cold());
    let (mut run, mut longest, mut plugged) = (0, 0, 0);
    for _ in 0..1000 {
        let seen = if matches!(d.fan, Asked::Hold(pa) if pa > 0.0) { blowing() } else { cold() };
        d = quiet(&mut burning, seen, 1);
        plugged += d.igniter as usize;
        run = if d.igniter && !matches!(d.fan, Asked::Hold(pa) if pa > 0.0) { run + 1 } else { 0 };
        longest = longest.max(run);
        if matches!(d.state, State::Fault(_)) {
            break;
        }
    }
    assert_eq!(burning.state(), State::Fault(Why::NeverLit(3)), "the cycle did not reach its fault");
    // three attempts of 30 s with air, less the second each takes for the fan's draught to be read
    assert!(plugged > 3 * 25, "the plug hardly burned - the row proves nothing: {plugged} s");
    assert_eq!(longest, 0, "the plug burned {longest} s in still air");
}

#[test]
fn a_flame_held_while_the_plug_burns_with_air_goes_to_rising_in_its_fifth_second() {
    let mut burning = kindling();
    quiet(&mut burning, cold(), 50);
    let held = quiet(&mut burning, lit(), 4);
    assert_eq!(held.state, State::Kindling(1), "four seconds of flame were taken for a fire");
    let did = quiet(&mut burning, lit(), 1);
    assert_eq!(did.state, State::Rising, "the plug's seconds were sat out over a flame");
}

#[test]
fn a_flame_held_while_the_fan_waits_goes_to_rising_in_its_fifth_second() {
    let mut burning = kindling();
    quiet(&mut burning, cold(), 100);
    let held = quiet(&mut burning, lit(), 4);
    assert_eq!(held.state, State::Kindling(1), "four seconds of flame were taken for a fire");
    let did = quiet(&mut burning, lit(), 1);
    assert_eq!(did.state, State::Rising, "the wait's seconds were sat out over a flame");
}

#[test]
fn no_flame_in_those_same_seconds_leaves_the_attempt_going() {
    // the pair of the two above
    let mut burning = kindling();
    let did = quiet(&mut burning, cold(), 51);
    assert_eq!(did.state, State::Kindling(1));
    let did = quiet(&mut burning, cold(), 50);
    assert_eq!(did.state, State::Kindling(1));
}

/// An attempt with the plug given four minutes with air and a short wait after it, started
/// and brought to its first second of lighting; and its numbers.
fn kindling_until_the_flame() -> (Burning, common::Sound) {
    let t = Tuning { igniter_s: 240.0, wait_for_flame_s: 20.0, ..tuning() }.sound().expect("sound numbers");
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, &t);
    while burning.state() != State::Kindling(1) {
        burning.step(Told::Nothing, cold(), 1.0, &t);
    }
    (burning, t)
}

#[test]
fn a_plug_burns_with_air_until_the_flame_and_goes_off_with_it() {
    let (mut burning, t) = kindling_until_the_flame();
    // the dose 32 s and three and a half minutes of the plug with air: a slow pot, no flame yet
    for second in 1..=(32 + 210) {
        let did = burning.step(Told::Nothing, in_attempt(second), 1.0, &t);
        assert_eq!(did.state, State::Kindling(1), "second {second}: the attempt ended before its plug's limit");
    }
    let before = burning.step(Told::Nothing, blowing(), 1.0, &t);
    assert!(before.igniter, "the plug went off at 3.5 min of its 4 with no flame");
    // the plug burns through the seconds the flame is held, and goes off with the fire (five seconds)
    for held in 1..=4 {
        let did = burning.step(Told::Nothing, lit(), 1.0, &t);
        assert_eq!(did.state, State::Kindling(1), "second {held} of the flame ended the attempt");
        assert!(did.igniter, "the plug went off in second {held} of a flame that has not held");
    }
    let did = burning.step(Told::Nothing, lit(), 1.0, &t);
    assert_eq!(did.state, State::Rising, "the flame did not end the attempt in its fifth second");
    assert!(!did.igniter, "the plug burns on over a flame");
}

#[test]
fn a_plug_with_no_flame_goes_off_at_its_limit_and_the_fan_alone_waits_briefly() {
    // the pair: nothing lights, and the plug stops where its setting says, not a second later
    let (mut burning, t) = kindling_until_the_flame();
    let did: Vec<Doing> =
        (1..=(32 + 240 + 20 + 1)).map(|n| burning.step(Told::Nothing, in_attempt(n), 1.0, &t)).collect();
    assert!(did[32 + 239].igniter, "the plug went off before its 240 s with air");
    assert!(!did[32 + 240].igniter, "the plug burned past its 240 s with air");
    assert!(matches!(did[32 + 240].fan, Asked::Hold(pa) if pa > 0.0), "the fan stood when the plug went off");
    assert_eq!(did[32 + 240 + 20].state, State::Kindling(2), "the fan alone did not hand over to the next attempt");
}

#[test]
fn a_plug_given_no_air_to_light_with_is_refused_by_its_code() {
    let refused = Tuning { kindling: Lighting { pa: 0.0, ..tuning().kindling }, ..tuning() }.sound().expect_err("a plug in still air");
    assert_eq!((refused.field.as_str(), refused.code, refused.value), ("kindling.pa", "kindling_no_air", Some(0.0)));
}

#[test]
fn a_plug_given_air_for_its_seconds_is_taken() {
    // the pair: the factory's own numbers are sound
    assert!(tuning().sound().is_ok());
    assert!(Tuning { kindling: Lighting { pa: 1.0, ..tuning().kindling }, ..tuning() }.sound().is_ok());
}

// ---------------------------------------------------------------- the pump, by the water of the boiler
//
// the pump runs from auto_pump_on_c on the boiler's water and stops the hysteresis below it; it
// runs on overheating whatever the threshold; it runs when the boiler gives no number. In every state of the burner.

fn water(c: Option<f32>) -> Seen {
    Seen { boiler_c: c, ..cold() }
}

#[test]
fn the_pump_runs_from_its_threshold_and_not_below() {
    let mut burning = Burning::new();
    assert!(!quiet(&mut burning, water(Some(40.0)), 1).pump, "the pump ran on water below its threshold");
    assert!(quiet(&mut burning, water(Some(45.0)), 1).pump, "the pump stood at its threshold");
}

#[test]
fn the_pump_does_not_chatter_at_its_threshold() {
    let mut burning = Burning::new();
    quiet(&mut burning, water(Some(46.0)), 1);
    assert!(quiet(&mut burning, water(Some(42.0)), 1).pump, "the pump stopped inside its hysteresis");
    assert!(!quiet(&mut burning, water(Some(40.0)), 1).pump, "the pump ran below its hysteresis");
    assert!(!quiet(&mut burning, water(Some(44.0)), 1).pump, "the pump came back inside its hysteresis, from below");
}

#[test]
fn an_overheating_boiler_has_its_pump_run_whatever_the_threshold() {
    let high = Tuning { pump_on_c: 90.0, ..tuning() }.sound().unwrap();
    let mut burning = Burning::new();
    let did = burning.step(Told::Nothing, water(Some(88.0)), 1.0, &high);
    assert!(did.pump, "the water is past too_hot_c and the pump stands");
    let did = burning.step(Told::Nothing, water(Some(80.0)), 1.0, &high);
    assert!(!did.pump, "the pair: under too_hot_c and under its own threshold, the pump stands");
}

#[test]
fn a_boiler_that_gives_no_number_has_its_pump_run() {
    let mut burning = Burning::new();
    assert!(quiet(&mut burning, water(None), 1).pump, "the boiler gave no number and the pump stood");
}

#[test]
fn the_pump_follows_the_water_in_every_state_of_the_burner() {
    let mut burning = working();
    let hot = Seen { boiler_c: Some(50.0), ..lit() };
    assert!(quiet(&mut burning, hot, 1).pump, "a working burner over warm water");
    told(&mut burning, Told::Stop, hot);
    let did = quiet(&mut burning, Seen { boiler_c: Some(50.0), ..cold() }, 60);
    assert_eq!(did.state, State::Stop, "the scaffolding did not reach a stop");
    assert!(did.pump, "a stopped burner over warm water left the pump standing");
}

// ---------------------------------------------------------------- the auger's cycle on the clocks
//
// the scheme says whether the auger feeds or pauses and for how long, from the same clocks as the
// card. A working fire's auger runs dose and pause on the step's numbers (4.6 s and 15 s in the rows).

#[test]
fn a_working_fire_counts_down_the_half_of_the_auger_it_is_in() {
    let mut burning = working();
    // find the first second of a dose - the auger going from standing to turning - then read the clock across it
    let mut was = quiet(&mut burning, lit(), 1);
    let mut did = quiet(&mut burning, lit(), 1);
    for _ in 0..120 {
        if did.auger && !was.auger {
            break;
        }
        was = did;
        did = quiet(&mut burning, lit(), 1);
    }
    assert!(did.auger && !was.auger, "the scaffolding did not find the first second of a dose");
    assert_eq!(did.state, State::Working, "the scaffolding is not in a working fire");
    let left = |b: &Burning| clock(b, "auger switches in").map(|s| (s * 10.0).round() / 10.0).expect("no clock of the auger");
    // the dose runs on the clock and not on the seconds: the second that finds it turning may begin up to a second into it, so the clock reads between 3.6 and 4.6 and then counts down by what passes
    let first = left(&burning);
    assert!(first > 3.6 && first <= 4.6, "the dose's clock at the first second it is found turning: {first}");
    quiet(&mut burning, lit(), 3);
    assert!((left(&burning) - (first - 3.0)).abs() < 0.11, "three seconds later: {} after {first}", left(&burning));
    let mut pause = quiet(&mut burning, lit(), 1);
    for _ in 0..3 {
        if !pause.auger {
            break;
        }
        pause = quiet(&mut burning, lit(), 1);
    }
    assert!(!pause.auger, "the dose did not end after its 4.6 s");
    let rest = left(&burning);
    assert!(rest > 14.0 && rest <= 15.0, "the pause's clock at its first second: {rest}");
}

#[test]
fn a_stopped_burner_counts_no_auger_down() {
    // the pair: nothing feeds, nothing is counted
    let burning = Burning::new();
    assert_eq!(clock(&burning, "auger switches in"), None);
}

#[test]
fn the_pump_is_given_its_reason_with_its_state() {
    let mut burning = Burning::new();
    assert_eq!(quiet(&mut burning, water(Some(40.0)), 1).pump_why, automaton::PumpWhy::Water(40.0));
    assert_eq!(quiet(&mut burning, water(Some(95.0)), 1).pump_why, automaton::PumpWhy::Overheated(95.0));
    assert_eq!(quiet(&mut burning, water(None), 1).pump_why, automaton::PumpWhy::NoNumber);
}

// ---------------------------------------------------------------- stop, from every phase

/// A burner walked from a start the ordinary way - a purge, a kindling, a flame, the step, the setpoint reached - and
/// left in the first second whose state IS the one asked for.
fn reached(is: fn(State) -> bool) -> Burning {
    let t = sound();
    let mut burning = Burning::new();
    let mut did = burning.step(Told::Start, cold(), 1.0, &t);
    let path = std::iter::repeat(cold())
        .take(30)
        .chain(std::iter::repeat(lit()).take(92))
        .chain(std::iter::repeat(Seen { boiler_c: Some(60.0), ..lit() }).take(5));
    for seen in path {
        if is(did.state) {
            return burning;
        }
        did = burning.step(Told::Nothing, seen, 1.0, &t);
    }
    assert!(is(did.state), "the walk never reached the state asked for, it ended in {:?}", did.state);
    burning
}

#[test]
fn stop_in_the_middle_of_any_phase_is_a_burning_off_with_air_and_the_fuel_off() {
    let phases: [(&str, fn(State) -> bool); 5] = [
        ("purge", |s| s == State::Purge),
        ("kindling", |s| matches!(s, State::Kindling(_))),
        ("rising", |s| s == State::Rising),
        ("working", |s| s == State::Working),
        ("smouldering", |s| s == State::Smouldering),
    ];
    for (name, is) in phases {
        let mut burning = reached(is);
        let did = told(&mut burning, Told::Stop, lit());
        assert_eq!(did.state, State::BurningOff(None), "stop in {name}");
        assert!(!did.auger, "stop in {name}: the fuel is off");
        assert_ne!(did.fan, Asked::Stop, "stop in {name}: burning off has air, it is not the power cut");
    }
}

#[test]
fn a_start_is_no_stop_in_any_phase() {
    // the pair: the burning off above comes from the word stop, not from any press of a button
    let mut burning = reached(|s| matches!(s, State::Kindling(_)));
    let did = told(&mut burning, Told::Start, cold());
    assert!(matches!(did.state, State::Kindling(_)), "start in kindling went to {:?}", did.state);
}

// ---- a leap of the board's clock: the automaton's seconds are spans of the steady clock ----

/// Seconds as the burner's round takes them: the span of the board's steady clock since the last round, over a
/// chip's clock RAW that the test moves.
struct Rounds {
    clock: between::clock::Steady,
    last: between::clock::Moment,
}

impl Rounds {
    fn over(raw: fn() -> u64) -> Rounds {
        let clock = between::clock::Steady::new(raw, between::clock::MOST);
        let last = clock.now();
        Rounds { clock, last }
    }
    fn secs(&mut self) -> f32 {
        let now = self.clock.now();
        let secs = now.since(self.last).as_secs_f32();
        self.last = now;
        secs
    }
}

const HOUR_US: u64 = 3_600_000_000;

#[test]
fn a_leap_of_an_hour_in_the_dark_at_work_does_not_count_as_the_flame_lost() {
    static RAW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    fn raw() -> u64 {
        RAW.load(std::sync::atomic::Ordering::Relaxed)
    }
    let t = sound();
    let mut burning = working();
    let mut rounds = Rounds::over(raw);
    let dark = gone();
    for _ in 0..10 {
        RAW.fetch_add(1_000_000, std::sync::atomic::Ordering::Relaxed);
        burning.step(Told::Nothing, dark, rounds.secs(), &t);
    }
    RAW.fetch_add(HOUR_US, std::sync::atomic::Ordering::Relaxed);
    let did = burning.step(Told::Nothing, dark, rounds.secs(), &t);
    assert_eq!(did.state, State::Working, "a leap of the clock after 10 s of darkness counted as a lost flame");
}

#[test]
fn the_same_leap_taken_as_it_came_would_have_counted_as_the_flame_lost() {
    let t = sound();
    let mut burning = working();
    let dark = Seen { flame: Flame::Out, ..lit() };
    quiet(&mut burning, dark, 10);
    let did = burning.step(Told::Nothing, dark, (HOUR_US / 1_000_000) as f32, &t);
    assert_ne!(did.state, State::Working, "an hour of darkness in one second was not taken for a lost flame");
}

#[test]
fn a_flame_truly_gone_for_its_time_is_still_taken_for_lost_by_the_steady_clock() {
    static RAW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    fn raw() -> u64 {
        RAW.load(std::sync::atomic::Ordering::Relaxed)
    }
    let t = sound();
    let mut burning = working();
    let mut rounds = Rounds::over(raw);
    let dark = gone();
    let mut state = State::Working;
    for _ in 0..125 {
        RAW.fetch_add(1_000_000, std::sync::atomic::Ordering::Relaxed);
        state = burning.step(Told::Nothing, dark, rounds.secs(), &t).state;
    }
    assert_ne!(state, State::Working, "125 s of darkness by the steady clock did not lose the flame");
}

#[test]
fn a_leap_of_an_hour_in_the_middle_of_kindling_does_not_skip_the_attempt() {
    static RAW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    fn raw() -> u64 {
        RAW.load(std::sync::atomic::Ordering::Relaxed)
    }
    let t = sound();
    let mut burning = Burning::new();
    let mut rounds = Rounds::over(raw);
    burning.step(Told::Start, cold(), 1.0, &t);
    let mut state = State::Purge;
    while !matches!(state, State::Kindling(_)) {
        RAW.fetch_add(1_000_000, std::sync::atomic::Ordering::Relaxed);
        state = burning.step(Told::Nothing, cold(), rounds.secs(), &t).state;
    }
    for _ in 0..5 {
        RAW.fetch_add(1_000_000, std::sync::atomic::Ordering::Relaxed);
        burning.step(Told::Nothing, cold(), rounds.secs(), &t);
    }
    RAW.fetch_add(HOUR_US, std::sync::atomic::Ordering::Relaxed);
    let did = burning.step(Told::Nothing, cold(), rounds.secs(), &t);
    assert_eq!(did.state, State::Kindling(1), "a leap of the clock in the first attempt went to {:?}", did.state);
    // what the same burner does when the round is two seconds late - the most a leap may cost: the same way by whole
    // seconds, then MOST at once
    let mut late = Burning::new();
    late.step(Told::Start, cold(), 1.0, &t);
    while !matches!(late.step(Told::Nothing, cold(), 1.0, &t).state, State::Kindling(_)) {}
    quiet(&mut late, cold(), 5);
    let should = late.step(Told::Nothing, cold(), between::clock::MOST.as_secs_f32(), &t);
    assert_eq!(
        (did.auger, did.igniter),
        (should.auger, should.igniter),
        "a leap of the clock did to the auger and the igniter what a round two seconds late does not"
    );
}

#[test]
fn the_same_leap_in_kindling_taken_as_it_came_would_have_skipped_the_attempt() {
    let t = sound();
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, &t);
    let mut state = State::Purge;
    while !matches!(state, State::Kindling(_)) {
        state = burning.step(Told::Nothing, cold(), 1.0, &t).state;
    }
    quiet(&mut burning, cold(), 5);
    let did = burning.step(Told::Nothing, cold(), (HOUR_US / 1_000_000) as f32, &t);
    assert_ne!(did.state, State::Kindling(1), "an hour in one second stayed in the first attempt");
}

// ---------------------------------------------------------------- a silent sensor
//
// A sensor whose round stopped publishing gives None, as any missing number, and the silence says why. What the rows
// judge: nothing is lit or fed on a sensor that is quiet, the cause is named by the sensor, and past ten periods of
// silence (at least 30 s) a fire nobody can see or read is burnt off. The pair of each is the same burner with the
// sensor speaking.

fn silent(draught: Heard, flame: Heard, flue: Heard, water: Heard) -> Silence {
    Silence { draught, flame, flue, water, return_c: Heard::Speaks, fronts_s: None }
}

/// SECONDS of nobody touching anything by the numbers T.
fn quiet_by(burning: &mut Burning, seen: Seen, seconds: usize, t: &common::Sound) -> Doing {
    let mut did = burning.step(Told::Nothing, seen, 1.0, t);
    for _ in 1..seconds {
        did = burning.step(Told::Nothing, seen, 1.0, t);
    }
    did
}

#[test]
fn a_silent_draught_sensor_burns_the_fire_off_the_same_second_as_no_draught_and_says_it_is_silent() {
    let mut burning = working();
    let silence = silent(Heard::Silent, Heard::Speaks, Heard::Speaks, Heard::Speaks);
    let did = quiet(&mut burning, Seen { draught_pa: None, silence, ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::DraughtSilent)));
    let mut pair = working();
    let did = quiet(&mut pair, Seen { draught_pa: None, ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoDraught)), "a sensor that speaks and gives no number is a draught of nothing");
}

#[test]
fn a_silent_draught_sensor_does_not_burn_the_plug() {
    // second 33 of the first attempt: the dose is in, and the plug burns only with air
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    quiet(&mut burning, cold(), 30);
    quiet(&mut burning, cold(), 33);
    let silence = silent(Heard::Silent, Heard::Speaks, Heard::Speaks, Heard::Speaks);
    let did = quiet(&mut burning, Seen { draught_pa: None, draught_low: false, gate: 30, silence, ..cold() }, 1);
    assert!(!did.igniter, "no air can be seen, so the plug does not burn");
    let mut pair = Burning::new();
    told(&mut pair, Told::Start, cold());
    quiet(&mut pair, cold(), 30);
    quiet(&mut pair, cold(), 33);
    assert!(quiet(&mut pair, blowing(), 1).igniter, "with the sensor speaking and the fan blowing, the plug burns");
}

#[test]
fn a_silent_flame_sensor_stops_the_auger_where_a_dark_one_that_speaks_feeds_on() {
    let silence = silent(Heard::Speaks, Heard::Silent, Heard::Speaks, Heard::Speaks);
    let mut burning = working();
    let fed = (0..20).any(|_| quiet(&mut burning, Seen { flame: Flame::Out, silence, ..lit() }, 1).auger);
    assert!(!fed, "a flame nobody can see is not fed");
    let mut pair = working();
    let fed = (0..20).any(|_| quiet(&mut pair, Seen { flame: Flame::Out, ..lit() }, 1).auger);
    assert!(fed, "a dark flame the sensor speaks of is a blink: the pair runs on until it is taken for lost");
}

#[test]
fn a_silent_flame_is_not_lit_again_from_embers_or_with_the_plug() {
    let mut t = tuning();
    t.flame_lost_s = 10.0;
    let t = t.sound().expect("sound");
    let silence = silent(Heard::Speaks, Heard::Silent, Heard::Speaks, Heard::Speaks);
    let mut burning = working_by(&t);
    let did = quiet_by(&mut burning, Seen { flame: Flame::Out, silence, ..lit() }, 20, &t);
    assert_eq!(did.state, State::Working, "past flame_lost_s, and nothing is lit on a sensor that is quiet");
    assert!(!did.igniter && !did.auger);
    let mut pair = working_by(&t);
    let did = quiet_by(&mut pair, Seen { flame: Flame::Out, ..lit() }, 20, &t);
    assert!(matches!(did.state, State::Kindling(_)), "a flame the sensor saw go is lit again: {:?}", did.state);
}

#[test]
fn a_flame_sensor_silent_too_long_burns_the_fire_off_and_says_so() {
    let mut burning = working();
    let silent_29 = silent(Heard::Speaks, Heard::Silent, Heard::Speaks, Heard::Speaks);
    let did = quiet(&mut burning, Seen { flame: Flame::Out, silence: silent_29, ..lit() }, 1);
    assert_eq!(did.state, State::Working, "silent, not yet too long");
    let too_long = silent(Heard::Speaks, Heard::TooLong, Heard::Speaks, Heard::Speaks);
    let did = quiet(&mut burning, Seen { flame: Flame::Out, silence: too_long, ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::FlameSilent)));
}

#[test]
fn a_water_sensor_silent_too_long_burns_the_fire_off_and_says_so() {
    let mut burning = working();
    let silent_29 = silent(Heard::Speaks, Heard::Speaks, Heard::Speaks, Heard::Silent);
    let did = quiet(&mut burning, Seen { boiler_c: None, silence: silent_29, ..lit() }, 1);
    assert_eq!(did.state, State::Working, "silent, not yet too long");
    assert!(did.pump, "the water nobody can read is pumped");
    let too_long = silent(Heard::Speaks, Heard::Speaks, Heard::Speaks, Heard::TooLong);
    let did = quiet(&mut burning, Seen { boiler_c: None, silence: too_long, ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::WaterSilent)));
    let mut pair = working();
    let did = quiet(&mut pair, Seen { boiler_c: Some(40.0), ..lit() }, 1);
    assert!(!did.pump, "water read under pump_on_c is not pumped");
}

#[test]
fn a_draught_sensor_silent_too_long_burns_off_even_where_the_fan_stands() {
    // smouldering on the chimney's own pull asks the draught nothing, so no early way out catches its silence
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    let too_long = silent(Heard::TooLong, Heard::Speaks, Heard::Speaks, Heard::Speaks);
    let did = quiet(&mut burning, Seen { draught_pa: None, silence: too_long, ..cold() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::DraughtSilent)), "a purge with its sensor gone too long");
    let mut pair = Burning::new();
    told(&mut pair, Told::Start, cold());
    let did = quiet(&mut pair, Seen { draught_pa: None, ..cold() }, 1);
    assert_eq!(did.state, State::Purge, "a purge asks the sensor for nothing");
}

#[test]
fn a_silent_flue_is_a_cold_pot_lit_with_the_plug() {
    // a guard: a flue with no number already sends a lost flame to the plug; a silent one is no number
    let mut t = tuning();
    t.flame_lost_s = 10.0;
    let t = t.sound().expect("sound");
    let silence = silent(Heard::Speaks, Heard::Speaks, Heard::Silent, Heard::Speaks);
    let mut burning = working_by(&t);
    quiet_by(&mut burning, Seen { flame: Flame::Out, flue_c: None, silence, ..lit() }, 11, &t);
    let lit_how = (0..40).any(|_| quiet_by(&mut burning, Seen { flame: Flame::Out, flue_c: None, gate: 30, draught_pa: Some(51.0), silence, ..lit() }, 1, &t).igniter);
    assert!(lit_how, "a pot nobody can read is taken for cold and lit with the plug");
}

// ---------------------------------------------------------------- a clock that has not begun has no number
//
// Three clocks have a state before their thing begins: the blow behind a burning out (until the fire is over), the watch
// over a flame that has come back (while it is dark) and the climb to a full step (until the flame holds). In that state
// they said the number "if it began now" - the same whole every second, and the page counted it down while the board
// said it whole again. They now say that they have not begun, and read a number only from the second they have.

/// What a clock says of itself in this second.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Reads {
    /// The state has no such clock.
    Absent,
    /// The clock is on the card and its thing has not begun.
    NotBegun,
    Left(f32),
}

fn reads(did: &Burning, what: &str) -> Reads {
    match did.clocks(&sound()).into_iter().find(|c| c.what == what) {
        None => Reads::Absent,
        Some(c) => c.left_s.map_or(Reads::NotBegun, Reads::Left),
    }
}

const BLOW: &str = "blowing clear ends in";
const BURN_OUT: &str = "burning out ends in at most";
const WATCH: &str = "fed after watching the flame for";
const FULL_STEP: &str = "full step in";

/// A burner that came up over a dark pot remembering work: rising under watch, the flame not yet seen.
fn watching() -> Burning {
    let burning = Burning::resumed(State::Working, &cold(), &sound());
    assert_eq!(burning.state(), State::Rising, "the scaffolding did not reach the watch");
    burning
}

/// A burner that came up over a flame remembering a rising: the climb counted from its first second.
fn climbing() -> Burning {
    let burning = Burning::resumed(State::Rising, &lit(), &sound());
    assert_eq!(burning.state(), State::Rising, "the scaffolding did not reach the climb");
    burning
}

#[test]
fn the_blow_has_no_number_while_the_fire_burns_out() {
    let mut burning = working();
    told(&mut burning, Told::Stop, lit());
    for second in 0..30 {
        assert_eq!(reads(&burning, BLOW), Reads::NotBegun, "second {second} of a burning out with the flame steady");
        quiet(&mut burning, lit(), 1);
    }
}

#[test]
fn the_blow_counts_from_its_whole_down_to_its_last_second_without_a_step_up_once_the_flame_is_gone() {
    let mut burning = working();
    told(&mut burning, Told::Stop, lit());
    quiet(&mut burning, lit(), 20);
    let mut said = Vec::new();
    for _ in 0..60 {
        quiet(&mut burning, cold(), 1);
        if burning.state() != State::Stop {
            said.push(reads(&burning, BLOW));
        }
    }
    let wanted: Vec<Reads> = (1..=20).rev().map(|s| Reads::Left(s as f32)).collect();
    assert_eq!(said, wanted, "the blow counts the seconds it has left, 20 down to 1, and not a number more or less");
}

#[test]
fn the_blow_begins_when_the_burning_out_has_run_out_whichever_way_it_ends() {
    // by the flame going, and by the ceiling with the flame steady all the way: the number is there exactly from the second the burning out reads nought
    for flame_goes_at in [Some(20), None] {
        let mut burning = working();
        told(&mut burning, Told::Stop, lit());
        let mut begun_at = None;
        for second in 1..200 {
            let seen = if flame_goes_at.is_some_and(|at| second > at) { cold() } else { lit() };
            quiet(&mut burning, seen, 1);
            if burning.state() == State::Stop {
                break;
            }
            let out = reads(&burning, BURN_OUT);
            let blow = reads(&burning, BLOW);
            let burning_out = matches!(out, Reads::Left(left) if left > 0.0);
            assert_eq!(blow == Reads::NotBegun, burning_out, "second {second} (flame goes at {flame_goes_at:?}): the burning out reads {out:?} and the blow {blow:?}");
            if begun_at.is_none() && matches!(blow, Reads::Left(_)) {
                begun_at = Some(second);
            }
        }
        assert!(begun_at.is_some(), "the blow never began (flame goes at {flame_goes_at:?})");
    }
}

#[test]
fn a_watch_over_a_dark_flame_has_no_number_and_a_watch_over_a_flame_counts_down() {
    let mut burning = watching();
    for second in 0..5 {
        assert_eq!(reads(&burning, WATCH), Reads::NotBegun, "second {second} of a dark pot");
        quiet(&mut burning, cold(), 1);
    }
    let mut said = Vec::new();
    for _ in 0..9 {
        quiet(&mut burning, lit(), 1);
        said.push(reads(&burning, WATCH));
    }
    let wanted: Vec<Reads> = (1..=9).rev().map(|s| Reads::Left(s as f32)).collect();
    assert_eq!(said, wanted, "the watch counts the seconds the flame has left to hold, 9 down to 1");
}

#[test]
fn a_flame_that_blinks_under_watch_is_a_dash_and_then_the_watch_counts_again_from_its_whole() {
    let mut burning = watching();
    quiet(&mut burning, lit(), 1);
    quiet(&mut burning, lit(), 1);
    assert_eq!(reads(&burning, WATCH), Reads::Left(8.0));
    quiet(&mut burning, cold(), 1);
    assert_eq!(reads(&burning, WATCH), Reads::NotBegun, "a dark second takes the count away");
    quiet(&mut burning, lit(), 1);
    assert_eq!(reads(&burning, WATCH), Reads::Left(9.0), "the flame is back: the watch begins again");
    quiet(&mut burning, lit(), 1);
    assert_eq!(reads(&burning, WATCH), Reads::Left(8.0));
}

#[test]
fn a_flame_not_back_counts_its_own_wait_while_the_watch_has_no_number() {
    let mut burning = watching();
    quiet(&mut burning, cold(), 7);
    assert_eq!(reads(&burning, WATCH), Reads::NotBegun);
    assert_eq!(reads(&burning, "flame not back, burning off in"), Reads::Left(113.0), "the wait for the flame is a clock that runs, dark or not");
}

#[test]
fn the_climb_has_no_number_while_the_flame_blinks_and_counts_from_its_whole_when_the_flame_holds() {
    let mut burning = climbing();
    assert_eq!(reads(&burning, FULL_STEP), Reads::Left(60.0));
    quiet(&mut burning, lit(), 5);
    assert_eq!(reads(&burning, FULL_STEP), Reads::Left(55.0));
    quiet(&mut burning, cold(), 1);
    assert_eq!(reads(&burning, FULL_STEP), Reads::NotBegun, "a dark second takes the count away");
    quiet(&mut burning, cold(), 2);
    assert_eq!(reads(&burning, FULL_STEP), Reads::NotBegun);
    quiet(&mut burning, lit(), 1);
    assert_eq!(reads(&burning, FULL_STEP), Reads::Left(60.0), "the flame is back: the climb is counted from the start");
    quiet(&mut burning, lit(), 1);
    assert_eq!(reads(&burning, FULL_STEP), Reads::Left(59.0));
}

#[test]
fn a_climb_that_runs_down_to_its_end_reaches_work_without_a_step_up() {
    let mut burning = climbing();
    let mut said = Vec::new();
    for _ in 0..80 {
        quiet(&mut burning, lit(), 1);
        if burning.state() != State::Rising {
            break;
        }
        said.push(reads(&burning, FULL_STEP));
    }
    assert_eq!(burning.state(), State::Working);
    let numbers: Vec<f32> = said.iter().map(|r| match r { Reads::Left(s) => *s, other => panic!("a climb under a steady flame said {other:?}") }).collect();
    assert!(numbers.windows(2).all(|w| w[1] < w[0]), "the climb stepped up or stood: {numbers:?}");
    assert_eq!(numbers.last(), Some(&1.0));
}

#[test]
fn the_clocks_that_run_in_their_state_have_a_number_every_second() {
    // the pair of the three above: a purge, an attempt, a smouldering and a work are never "not begun"
    let mut burning = Burning::new();
    told(&mut burning, Told::Start, cold());
    for _ in 0..29 {
        quiet(&mut burning, cold(), 1);
        assert!(matches!(reads(&burning, "lighting in"), Reads::Left(_)), "a purge's clock has a number");
    }
    let mut burning = kindling();
    for _ in 0..80 {
        quiet(&mut burning, cold(), 1);
        for what in ["dose ends in", "igniter off in", "attempt ends in"] {
            assert!(matches!(reads(&burning, what), Reads::Left(_)), "{what} has a number");
        }
    }
    let mut burning = working();
    quiet(&mut burning, lit(), 3);
    assert!(matches!(reads(&burning, "auger switches in"), Reads::Left(_)));
}

// ---------------------------------------------------------------- the pin that gives no fronts
//
// 24.10 at 23:47 the zero-crossing pin of `.33` gave nothing for 86 minutes while the mains was there. The gate had no phase to fire in, the draught fell, and the burner went out as `no_draught`: the page said "check the chimney".
// What the board knows is that the pin gave no fronts, and that is what it names - ahead of the draught, which is only its consequence.

fn without_fronts(seconds: u32) -> Silence {
    Silence { fronts_s: Some(seconds), ..Silence::HEARD }
}

#[test]
fn a_pin_that_gives_no_fronts_burns_the_fire_off_the_same_second_and_names_the_pin() {
    let mut burning = working();
    let did = quiet(&mut burning, Seen { silence: without_fronts(1), ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoFronts)));
}

#[test]
fn a_pin_that_gives_its_fronts_and_a_draught_that_gives_no_number_is_still_no_draught() {
    // the fan blows and the sensor says nothing: the chimney or the sensor, as before
    let mut burning = working();
    let did = quiet(&mut burning, Seen { draught_pa: None, ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoDraught)));
}

#[test]
fn the_pin_is_named_before_the_draught_that_fell_because_of_it() {
    // without fronts the fan stands, and a standing fan gives the sensor nothing to read: both are missing, and one is the cause of the other
    let mut burning = working();
    let did = quiet(&mut burning, Seen { draught_pa: None, silence: without_fronts(3), ..lit() }, 1);
    assert_eq!(did.state, State::BurningOff(Some(Why::NoFronts)));
}

#[test]
fn a_burning_off_for_want_of_fronts_asks_the_fan_for_nothing_and_does_not_feed() {
    let mut burning = working();
    let did = quiet(&mut burning, Seen { silence: without_fronts(1), ..lit() }, 1);
    assert_eq!(did.fan, Asked::Stop, "a fan that cannot be fired is not asked to blow, or the gate refuses every second");
    assert!(!did.auger && !did.igniter);
}

#[test]
fn a_pin_that_gives_no_fronts_keeps_a_burner_from_lighting() {
    let mut burning = Burning::new();
    let silent = Seen { silence: without_fronts(5), ..cold() };
    told(&mut burning, Told::Start, silent);
    let mut lit_ever = false;
    for _ in 0..60 {
        let did = quiet(&mut burning, silent, 1);
        lit_ever |= did.igniter || did.auger;
    }
    assert!(!lit_ever, "the plug and the auger stayed off");
    assert!(matches!(burning.state(), State::BurningOff(Some(Why::NoFronts)) | State::Fault(Why::NoFronts)), "{:?}", burning.state());
}

#[test]
fn the_fault_after_the_blow_is_the_pin_and_waits_for_a_person() {
    let mut burning = working();
    let silent = Seen { silence: without_fronts(1), ..cold() };
    quiet(&mut burning, Seen { silence: without_fronts(1), ..lit() }, 1);
    let did = quiet(&mut burning, silent, 120);
    assert_eq!(did.state, State::Fault(Why::NoFronts));
    assert_eq!(did.fan, Asked::Stop);
}

#[test]
fn a_pin_that_gives_fronts_again_does_not_clear_a_fault_by_itself() {
    let mut burning = working();
    quiet(&mut burning, Seen { silence: without_fronts(1), ..lit() }, 1);
    quiet(&mut burning, Seen { silence: without_fronts(1), ..cold() }, 120);
    let did = quiet(&mut burning, cold(), 30);
    assert_eq!(did.state, State::Fault(Why::NoFronts), "a fault waits for a person, whichever way its cause went");
}

// ---- the night of 24.10, second by second: `dimmer::Fronts` tells the loss, the automaton names it ----

/// The crossings of every second of the night from 23:46:30 to 01:14:00, the record of `.33` (the fixture of `dimmer`, run-length coded: "start count seconds").
fn crossings_of_the_night() -> Vec<u32> {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../dimmer/tests/fixtures/fronts_night_24_10.txt")).unwrap();
    text.lines()
        .flat_map(|row| {
            let mut parts = row.split(' ').skip(1);
            let count: i64 = parts.next().unwrap().parse().unwrap();
            let run: usize = parts.next().unwrap().parse().unwrap();
            std::iter::repeat_n(count.max(0) as u32, run)
        })
        .collect()
}

#[test]
fn on_the_record_of_the_night_the_burner_is_burnt_off_at_23_47_00_for_the_pin_and_not_for_the_draught() {
    let t = sound();
    let mut burning = working_by(&t);
    let mut fronts = dimmer::Fronts::new();
    let mut named: Vec<(usize, State)> = Vec::new();
    let mut last = burning.state();
    for (second, up) in crossings_of_the_night().into_iter().enumerate().take(60 * 5) {
        fronts.second(up);
        // after the fronts are gone the fan stands and the sensor reads nothing: the chain of that night, as the record has it (the draught fell within two seconds)
        let draught = if fronts.silent_s().is_some_and(|silent| silent >= 2) { None } else { Some(50.0) };
        let silence = Silence { fronts_s: fronts.silent_s(), ..Silence::HEARD };
        let did = burning.step(Told::Nothing, Seen { draught_pa: draught, silence, ..lit() }, 1.0, &t);
        if did.state != last {
            named.push((second, did.state));
            last = did.state;
        }
    }
    // second 30 of the record is 23:47:00
    assert_eq!(named[0], (30, State::BurningOff(Some(Why::NoFronts))), "{named:?}");
    assert!(named.iter().all(|(_, state)| !matches!(state, State::BurningOff(Some(Why::NoDraught)) | State::Fault(Why::NoDraught))), "the cause named is the pin's, never the draught's: {named:?}");
}

#[test]
fn on_the_record_after_01_13_37_the_burner_that_works_goes_on_working_though_the_count_is_91_to_109() {
    let t = sound();
    let mut burning = working_by(&t);
    let mut fronts = dimmer::Fronts::new();
    let counts = crossings_of_the_night();
    // the seconds from 01:13:37 on are the returned pin, 96 to 106 a second; the burner gets the live count and has been alive before
    fronts.second(100);
    for up in counts.iter().skip(counts.len() - 20) {
        fronts.second(*up);
        let silence = Silence { fronts_s: fronts.silent_s(), ..Silence::HEARD };
        let did = burning.step(Told::Nothing, Seen { silence, ..lit() }, 1.0, &t);
        assert_eq!(did.state, State::Working, "a count of {up} a second is a mains, not a loss");
    }
}

// ---------------------------------------------------------------- the fault for want of fronts lets go of itself

use automaton::{Found, Relief, RELIEF_AFTER_S, RELIEF_PER_DAY, RELIEF_RETRY_S};

fn on_day(day: u8) -> Option<tally::Now> {
    Some(tally::Now { date: tally::Date { year: 2026, month: 10, day }, minute: 600, offset_min: 120 })
}

/// A burner that stands in the fault for want of fronts: the fire was working, the pin went silent, the blow was over.
fn faulted_for_want_of_fronts() -> Burning {
    let mut burning = working();
    quiet(&mut burning, Seen { silence: without_fronts(1), ..lit() }, 1);
    let did = quiet(&mut burning, Seen { silence: without_fronts(1), ..cold() }, 120);
    assert_eq!(did.state, State::Fault(Why::NoFronts), "the scaffolding did not reach the fault");
    burning
}

const FAULT: State = State::Fault(Why::NoFronts);

/// A flash that takes whatever it is given.
fn kept(_: &str) -> bool {
    true
}

#[test]
fn a_fault_for_want_of_fronts_is_let_go_when_they_have_been_back_sixty_seconds_in_a_row_and_not_a_second_sooner() {
    let mut relief = Relief::new();
    for run in 1..RELIEF_AFTER_S {
        assert_eq!(relief.second(FAULT, Some(run), on_day(25), kept), None, "after {run} seconds");
    }
    assert_eq!(relief.second(FAULT, Some(RELIEF_AFTER_S), on_day(25), kept), Some(1), "the sixtieth");
}

#[test]
fn fronts_that_are_not_back_or_were_lost_again_before_the_sixty_let_nothing_go() {
    let mut relief = Relief::new();
    // silent: no run at all
    assert_eq!(relief.second(FAULT, None, on_day(25), kept), None);
    // 59 seconds, a loss (the run is gone), 59 again: never sixty
    for _ in 0..2 {
        assert_eq!(relief.second(FAULT, Some(59), on_day(25), kept), None);
        assert_eq!(relief.second(FAULT, None, on_day(25), kept), None);
    }
    assert_eq!(relief.second(FAULT, Some(60), on_day(25), kept), Some(1));
}

#[test]
fn only_three_a_local_day_are_let_go_and_the_fourth_waits_for_a_person_until_the_day_turns() {
    let mut relief = Relief::new();
    for n in 1..=RELIEF_PER_DAY {
        assert_eq!(relief.second(FAULT, Some(60), on_day(25), kept), Some(n), "number {n} of the day");
    }
    assert_eq!(relief.second(FAULT, Some(600), on_day(25), kept), None, "the fourth of the day");
    assert_eq!(relief.second(FAULT, Some(600), on_day(25), kept), None, "and it stays asking");
    assert_eq!(relief.second(FAULT, Some(60), on_day(26), kept), Some(1), "the next day starts the count again");
}

#[test]
fn a_fault_of_any_other_reason_and_any_state_that_is_not_that_fault_is_never_let_go() {
    let mut relief = Relief::new();
    for why in [Why::NeverLit(3), Why::FlameLost, Why::FlameNotBack(3), Why::NoFuel, Why::NoDraught, Why::Overheated(96.0), Why::Unconfigured, Why::DraughtSilent, Why::FlameSilent, Why::WaterSilent, Why::ReturnSilent] {
        assert_eq!(relief.second(State::Fault(why), Some(10_000), on_day(25), kept), None, "{why:?}");
    }
    // not yet a fault: the fire is being burnt off for the want of fronts, and that goes its way
    for state in [State::BurningOff(Some(Why::NoFronts)), State::Stop, State::Purge, State::Kindling(1), State::Rising, State::Working, State::Smouldering] {
        assert_eq!(relief.second(state, Some(10_000), on_day(25), kept), None, "{state:?}");
    }
    assert_eq!(relief.second(FAULT, Some(60), on_day(25), kept), Some(1), "and the one that is let go still is");
}

#[test]
fn with_no_time_of_day_nothing_is_let_go_because_the_day_is_not_known() {
    let mut relief = Relief::new();
    assert_eq!(relief.second(FAULT, Some(600), None, kept), None);
    assert_eq!(relief.second(FAULT, Some(600), on_day(25), kept), Some(1), "once the time is believed it is");
}

#[test]
fn the_count_of_the_day_outlives_a_restart_as_text_and_a_text_that_is_not_one_is_refused() {
    let mut relief = Relief::new();
    for _ in 0..RELIEF_PER_DAY {
        relief.second(FAULT, Some(60), on_day(25), kept);
    }
    let mut after_restart = Relief::parse(&relief.text()).expect("its own text");
    assert_eq!(after_restart.second(FAULT, Some(60), on_day(25), kept), None, "three were let go today, a restart does not give three more");
    let mut next_day = Relief::parse(&relief.text()).unwrap();
    assert_eq!(next_day.second(FAULT, Some(60), on_day(26), kept), Some(1));
    assert!(Relief::parse("").is_err() && Relief::parse("pellets\nday 2026-10-25\n").is_err() && Relief::parse("relief\nday not-a-day\ntoday 1\n").is_err());
}

#[test]
fn the_burner_let_go_of_its_fault_stands_in_a_stop_and_lights_by_the_room_as_it_always_does_with_the_switch_where_it_was() {
    let mut burning = faulted_for_want_of_fronts();
    assert!(burning.clear_fault(Why::NoFronts));
    assert_eq!(burning.state(), State::Stop);
    // heat was asked for before the fault and nobody pressed Stop: the water that wants burning lights it
    let cold_water = Seen { boiler_c: Some(30.0), ..cold() };
    let did = quiet(&mut burning, cold_water, 5);
    assert!(matches!(did.state, State::Purge), "{:?}", did.state);
}

#[test]
fn a_fault_of_another_reason_and_a_burner_that_is_not_in_a_fault_are_not_let_go_by_a_call_for_this_one() {
    let mut burning = working();
    quiet(&mut burning, Seen { draught_pa: None, ..lit() }, 1);
    quiet(&mut burning, Seen { draught_pa: None, ..cold() }, 21);
    assert_eq!(burning.state(), State::Fault(Why::NoDraught));
    assert!(!burning.clear_fault(Why::NoFronts));
    assert_eq!(burning.state(), State::Fault(Why::NoDraught));
    let mut working = working();
    assert!(!working.clear_fault(Why::NoFronts));
    assert_eq!(working.state(), State::Working);
}

// ---------------------------------------------------------------- what a person is told while the fault waits: the seconds left, and the day's count

#[test]
fn the_seconds_a_fault_for_want_of_fronts_still_waits_are_the_run_it_owes_and_none_where_it_will_not_be_let_go() {
    let mut relief = Relief::new();
    relief.second(FAULT, None, on_day(25), kept);
    // the pin has not come back: the whole run is owed
    assert_eq!(relief.left_s(FAULT, None), Some(RELIEF_AFTER_S));
    assert_eq!(relief.left_s(FAULT, Some(1)), Some(RELIEF_AFTER_S - 1));
    assert_eq!(relief.left_s(FAULT, Some(59)), Some(1));
    // nothing is owed in any other state or fault
    assert_eq!(relief.left_s(State::Fault(Why::NoDraught), Some(10)), None);
    assert_eq!(relief.left_s(State::BurningOff(Some(Why::NoFronts)), None), None);
    assert_eq!(relief.left_s(State::Working, None), None);
}

#[test]
fn nothing_is_owed_when_the_day_is_spent_or_the_time_is_not_believed() {
    let mut relief = Relief::new();
    relief.second(FAULT, None, None, kept);
    assert_eq!(relief.left_s(FAULT, None), None, "no time of day: the fault waits for a person, not for the fronts");
    assert_eq!(relief.today(), None, "and the count of the day is not known");
    for _ in 0..RELIEF_PER_DAY {
        relief.second(FAULT, Some(60), on_day(25), kept);
    }
    relief.second(FAULT, None, on_day(25), kept);
    assert_eq!(relief.today(), Some(RELIEF_PER_DAY));
    assert_eq!(relief.left_s(FAULT, None), None, "three today: the fault waits for a person");
}

#[test]
fn the_count_of_the_day_is_what_was_let_go_today_and_starts_again_with_the_day() {
    let mut relief = Relief::new();
    assert_eq!(relief.today(), None, "before the time is believed");
    relief.second(FAULT, None, on_day(25), kept);
    assert_eq!(relief.today(), Some(0));
    relief.second(FAULT, Some(60), on_day(25), kept);
    assert_eq!(relief.today(), Some(1));
    relief.second(State::Stop, None, on_day(26), kept);
    assert_eq!(relief.today(), Some(0));
}

#[test]
fn a_fault_that_stays_for_a_person_is_told_once_when_the_run_is_long_enough_and_the_day_is_spent_and_again_only_in_another_fault() {
    let mut relief = Relief::new();
    for _ in 0..RELIEF_PER_DAY {
        relief.second(FAULT, Some(60), on_day(25), kept);
    }
    // not yet: the fronts are not back for the sixty
    assert_eq!(relief.stays(FAULT, Some(59)), None);
    assert_eq!(relief.stays(FAULT, None), None);
    assert_eq!(relief.stays(FAULT, Some(60)), Some(RELIEF_PER_DAY), "the fourth waits: told, with the day's count");
    assert_eq!(relief.stays(FAULT, Some(61)), None, "once, not every second");
    // the person acknowledged it and the burner went on; another fault of the kind is another telling
    assert_eq!(relief.stays(State::Stop, None), None);
    assert_eq!(relief.stays(FAULT, Some(60)), Some(RELIEF_PER_DAY));
}

#[test]
fn a_fault_that_will_be_let_go_is_never_told_as_staying() {
    let mut relief = Relief::new();
    relief.second(FAULT, None, on_day(25), kept);
    assert_eq!(relief.stays(FAULT, Some(600)), None, "the count of the day is not spent");
    let mut no_time = Relief::new();
    no_time.second(FAULT, Some(600), None, kept);
    assert_eq!(no_time.stays(FAULT, Some(600)), None, "and without the time of day there is no day's count to spend");
}

// ---------------------------------------------------------------- a count the board cannot trust is a spent day, and a let-go is the one the flash took down

/// The count after a restart with the file as FOUND says, on the believed day 25.
fn restarted(found: Found) -> Relief {
    let (mut relief, _) = Relief::from_flash(found);
    relief.second(State::Stop, None, on_day(25), kept);
    relief
}

#[test]
fn a_file_that_is_a_count_is_the_count_and_no_file_is_a_board_that_has_let_go_of_nothing() {
    let mut before = Relief::new();
    before.second(FAULT, Some(60), on_day(25), kept);
    before.second(FAULT, Some(60), on_day(25), kept);
    let text = before.text();
    let mut after = restarted(Found::Bytes(text.as_bytes()));
    assert_eq!(after.today(), Some(2));
    assert_eq!(after.second(FAULT, Some(60), on_day(25), kept), Some(3), "the third of the day is the last one left");
    assert_eq!(after.second(FAULT, Some(60), on_day(25), kept), None);
    let mut fresh = restarted(Found::Nothing);
    assert_eq!(fresh.today(), Some(0));
    assert_eq!(fresh.second(FAULT, Some(60), on_day(25), kept), Some(1));
}

#[test]
fn a_file_that_is_not_a_count_or_cannot_be_read_is_a_day_with_its_let_goes_spent_until_the_day_turns() {
    let cases: [(&str, Found); 5] = [
        ("empty", Found::Bytes(b"")),
        ("another module's file", Found::Bytes(b"pellets\nday 2026-10-25\n")),
        ("a torn count", Found::Bytes(b"relief\nday 2026-10-25\ntoday")),
        ("bytes that are not text", Found::Bytes(&[0xff, 0xfe, 0x00])),
        ("a flash that gave nothing", Found::Unreadable("littlefs error -5".into())),
    ];
    for (what, found) in cases {
        let (mut relief, said) = Relief::from_flash(found);
        assert!(said.is_some(), "{what}: the journal is told");
        assert_eq!(relief.second(FAULT, Some(600), on_day(25), kept), None, "{what}: not let go today");
        assert_eq!(relief.today(), Some(RELIEF_PER_DAY), "{what}: the card says the day is spent");
        assert_eq!(relief.left_s(FAULT, Some(600)), None, "{what}: nothing is owed");
        assert_eq!(relief.stays(FAULT, Some(600)), Some(RELIEF_PER_DAY), "{what}: the fault waits for a person");
        assert_eq!(relief.second(FAULT, Some(60), on_day(26), kept), Some(1), "{what}: the next day starts the count again");
    }
}

#[test]
fn a_spent_day_stays_spent_whenever_the_time_of_day_is_first_believed() {
    let (mut relief, _) = Relief::from_flash(Found::Unreadable("x".into()));
    assert_eq!(relief.second(FAULT, Some(600), None, kept), None);
    assert_eq!(relief.second(FAULT, Some(600), on_day(25), kept), None);
    assert_eq!(relief.second(FAULT, Some(600), on_day(25), kept), None);
}

#[test]
fn a_let_go_is_the_one_whose_new_count_the_flash_took_and_a_refusal_lets_nothing_go_and_counts_nothing() {
    let mut relief = Relief::new();
    let mut offered = Vec::new();
    for _ in 0..3 {
        let got = relief.second(FAULT, Some(60), on_day(25), |text| {
            offered.push(text.to_owned());
            false
        });
        assert_eq!(got, None, "the flash refused");
        for _ in 1..RELIEF_RETRY_S {
            assert_eq!(relief.second(FAULT, Some(60), on_day(25), kept), None, "waiting to offer it again");
        }
    }
    assert_eq!(relief.today(), Some(0), "a let-go that was not written is not counted");
    assert_eq!(offered.len(), 3);
    assert!(offered.iter().all(|t| t.contains("today 1")), "{offered:?}");
    // the flash is back: this is still the first of the day
    let mut taken = String::new();
    let got = relief.second(FAULT, Some(60), on_day(25), |text| {
        taken = text.to_owned();
        true
    });
    assert_eq!(got, Some(1));
    assert_eq!(taken, relief.text(), "what the flash holds is the count the board holds");
    assert_eq!(relief.today(), Some(1));
}

#[test]
fn a_count_the_flash_refused_is_offered_again_after_the_pause_the_pellets_keep_and_not_sooner() {
    assert_eq!(RELIEF_RETRY_S, (pellets::RETRY_MS / 1000) as u32, "the pause is the pellets' own, not a number of this module");
    let mut relief = Relief::new();
    let mut asked = 0;
    assert_eq!(relief.second(FAULT, Some(60), on_day(25), |_| { asked += 1; false }), None);
    for second in 1..RELIEF_RETRY_S {
        assert_eq!(relief.second(FAULT, Some(60), on_day(25), |_| { asked += 1; true }), None, "{second} s after the refusal");
    }
    assert_eq!(asked, 1, "the flash was not asked inside the pause");
    assert_eq!(relief.second(FAULT, Some(60), on_day(25), |_| { asked += 1; true }), Some(1), "after the pause");
    assert_eq!(asked, 2);
}

#[test]
fn the_flash_is_not_asked_where_nothing_is_let_go() {
    let mut relief = Relief::new();
    let mut asked = 0;
    let mut ask = |_: &str| {
        asked += 1;
        true
    };
    relief.second(FAULT, Some(59), on_day(25), &mut ask);
    relief.second(State::Working, Some(600), on_day(25), &mut ask);
    relief.second(FAULT, Some(600), None, &mut ask);
    assert_eq!(asked, 0);
}
