//! Rising does not feed: the fire eats the lighting's dose. The dose is not a portion of fuel but
//! the pot filled above the plug, so once a flame shows there is fuel in plenty and nothing to add until the fire has
//! settled. Through rising the auger stands, the draught climbs from the lighting's to the step's, and the work begins
//! on its own pair once the flame has held for `auto_rise_s` without a break; a flame that blinks starts that count
//! again from its return. The numbers are the rows' own (tests/common), through the mapping the board and
//! the stand share (`automaton_card::tuning`).

mod common;

use automaton::{Asked, Doing, State};
use common::pair::{Automaton, Burning, Press, Seen, Silence, Sound};
use flame::Flame;
use common::sound;


/// The automaton from a Start at 2 s, with the flame FLAME gives each second; the water cool, the flue warming.
fn run(seconds: u32, sound: &Sound, flame: impl Fn(u32) -> Flame) -> Vec<Doing> {
    run_over(seconds, sound, flame, |at| Some(20.0 + at as f32 * 0.5))
}

/// The same with the flue FLUE gives each second - no number is a flue that says nothing.
fn run_over(seconds: u32, sound: &Sound, flame: impl Fn(u32) -> Flame, flue: impl Fn(u32) -> Option<f32>) -> Vec<Doing> {
    let mut a = Automaton::new(Burning::new());
    (0..seconds)
        .map(|at| {
            let seen = Seen { flame: flame(at), boiler_c: Some(40.0), return_c: None, flue_c: flue(at), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD };
            match at {
                2 => a.press(Press::Start, seen, sound),
                _ => a.second(seen, 1.0, sound),
            }
        })
        .collect()
}

/// The first second the automaton is in WANTED, or None.
fn first(seconds: &[Doing], wanted: State) -> Option<usize> {
    seconds.iter().position(|d| d.state == wanted)
}

/// Lit from the second the first dose is down on: the lighting's own clock ends well before 90 s.
fn lit_from_90(at: u32) -> Flame {
    if at >= 90 { Flame::Steady } else { Flame::Out }
}

#[test]
fn the_auger_stands_through_the_whole_of_rising() {
    let seconds = run(400, &sound(&[]), lit_from_90);
    let rising: Vec<&Doing> = seconds.iter().filter(|d| d.state == State::Rising).collect();
    assert!(rising.len() >= 60, "the fire did not rise long enough to be measured: {} s", rising.len());
    let fed = rising.iter().filter(|d| d.auger).count();
    assert_eq!(fed, 0, "the auger turned {fed} s of {} s of rising, over a pot the lighting filled", rising.len());
}

#[test]
fn the_draught_climbs_from_the_lighting_s_to_the_step_s_through_rising() {
    let seconds = run(400, &sound(&[("auto_kindle_pa", 42.0), ("auto_pa", 60.0)]), lit_from_90);
    let climb: Vec<f32> = seconds
        .iter()
        .filter(|d| d.state == State::Rising)
        .map(|d| match d.fan {
            Asked::Hold(pa) => pa,
            other => panic!("the fan was asked for {other:?} while the fire was rising"),
        })
        .collect();
    assert!(climb.windows(2).all(|w| w[1] >= w[0]), "the draught fell while the fire rose: {climb:?}");
    let (from, to) = (climb[0], *climb.last().unwrap());
    assert!((42.0..46.0).contains(&from), "rising began on {from} Pa, not near the lighting's 42");
    // its last second is one short of the whole climb, so a hair under the step's draught
    assert!(to > 59.0 && to <= 60.0, "rising ended on {to} Pa, not at the step's 60");
    let work = first(&seconds, State::Working).expect("the fire never came to work");
    assert_eq!(seconds[work].fan, Asked::Hold(60.0), "the work holds the step's draught");
}

#[test]
fn the_work_begins_on_its_own_pair_two_seconds_of_twenty_two() {
    let seconds = run(400, &sound(&[]), lit_from_90);
    let work = first(&seconds, State::Working).expect("the fire never came to work");
    let augers: Vec<bool> = seconds[work..work + 44].iter().map(|d| d.auger).collect();
    let expected: Vec<bool> = (0..44).map(|s| s % 22 < 2).collect();
    assert_eq!(augers, expected, "the first two cycles of work, from its first second");
}

#[test]
fn the_work_begins_when_the_flame_has_held_for_the_rising_time() {
    let seconds = run(400, &sound(&[]), lit_from_90);
    let rise = first(&seconds, State::Rising).expect("the fire never rose");
    let work = first(&seconds, State::Working).expect("the fire never came to work");
    assert_eq!(work - rise, 60, "rising lasted {} s, and the card says 60", work - rise);
}

#[test]
fn a_rising_time_set_on_the_card_is_how_long_rising_lasts() {
    let seconds = run(400, &sound(&[("auto_rise_s", 90.0)]), lit_from_90);
    let rise = first(&seconds, State::Rising).expect("the fire never rose");
    let work = first(&seconds, State::Working).expect("the fire never came to work");
    assert_eq!(work - rise, 90, "rising lasted {} s, and the card was set to 90", work - rise);
}

#[test]
fn a_flame_that_blinks_in_rising_starts_the_count_again_from_its_return() {
    // lit at 90, dark 130..=132, back at 133: the flame has held without a break from 133, so the work waits 60 s from there
    let blinks = |at: u32| if (130..133).contains(&at) { Flame::Out } else { lit_from_90(at) };
    let seconds = run(400, &sound(&[]), blinks);
    let work = first(&seconds, State::Working).expect("the fire never came to work");
    assert_eq!(work, 133 + 60, "the work began at {work} s, not 60 s after the flame came back at 133 s");
    assert!(seconds[..work].iter().all(|d| !(d.state == State::Rising && d.auger)), "the auger turned in rising");
}

#[test]
fn a_flame_gone_for_good_in_rising_never_reaches_work_and_feeds_nothing() {
    // the pair of the rows above: a fire that does not settle is not carried into work. What it does instead is the
    // wait and the attempt at lighting again, whose own tenth of a dose is not rising's to pour
    let goes = |at: u32| if (90..110).contains(&at) { Flame::Steady } else { Flame::Out };
    // and a flue that says nothing once it has gone: a cold pot, lit again with the plug
    let seconds = run_over(400, &sound(&[]), goes, |at| if at >= 110 { None } else { Some(20.0 + at as f32 * 0.5) });
    assert!(first(&seconds, State::Rising).is_some(), "the fire never rose");
    assert_eq!(first(&seconds, State::Working), None, "a flame gone for good was carried into work");
    let rise = first(&seconds, State::Rising).unwrap();
    let fed = seconds[rise..].iter().filter(|d| d.state == State::Rising && d.auger).count();
    assert_eq!(fed, 0, "the auger turned {fed} s after the flame showed, over a pot with the dose in it");
}
