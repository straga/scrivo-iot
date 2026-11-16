//! The feed banned from the main screen: pellets are already in the pot, the first attempt
//! did not light, and the next doses would pour more on top for a person to scoop out. Banned, the cycle runs as it
//! would, without a dose; the ban acts in the second it is given.

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
        kindling: Lighting { pa: 45.0, dose_s: 32.0 },
        step: Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

fn dark() -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// An automaton started and brought to the fifth second of its first dose.
fn in_the_first_dose() -> (Automaton, common::Sound) {
    let sound = tuning().sound().expect("sound numbers");
    let mut a = Automaton::new(Burning::new());
    a.press(Press::Start, dark(), &sound);
    while a.state() != State::Kindling(1) {
        a.second(dark(), 1.0, &sound);
    }
    for _ in 0..5 {
        assert!(a.second(dark(), 1.0, &sound).auger, "the dose is not pouring before the ban");
    }
    (a, sound)
}

#[test]
fn a_ban_in_the_middle_of_a_dose_stops_the_auger_in_the_same_second() {
    let (mut a, sound) = in_the_first_dose();
    let second = a.feed(false, dark(), &sound);
    assert!(!second.auger, "the auger turned on after the feed was banned");
    assert_eq!(second.state, State::Kindling(1), "the ban moved the automaton out of its attempt");
    assert!(!a.feed_allowed());
    assert!(!a.second(dark(), 1.0, &sound).auger, "the auger turned on the second after the ban");
}

#[test]
fn a_cycle_under_a_ban_never_turns_the_auger_and_runs_the_rest_as_it_would() {
    let (mut a, sound) = in_the_first_dose();
    a.feed(false, dark(), &sound);
    let (mut auger, mut igniter, mut seconds) = (0, 0, 0);
    while !matches!(a.state(), State::Fault(_)) && seconds < 2000 {
        let doing = a.second(dark(), 1.0, &sound);
        auger += doing.auger as u32;
        igniter += doing.igniter as u32;
        seconds += 1;
    }
    assert_eq!(auger, 0, "the auger turned for {auger} s under a ban");
    assert!(igniter > 0, "the igniter never came on: the ban stopped more than the feed");
    assert_eq!(a.state(), State::Fault(Why::NeverLit(3)), "the cycle did not run its three attempts to their end");
}

#[test]
fn an_allowed_feed_pours_its_dose_as_before() {
    let (mut a, sound) = in_the_first_dose();
    let second = a.feed(true, dark(), &sound);
    assert!(second.auger, "allowing an allowed feed stopped the dose");
    assert!(a.feed_allowed());
    let mut poured = 5;
    while a.second(dark(), 1.0, &sound).auger {
        poured += 1;
    }
    assert_eq!(poured, 32, "the dose poured {poured} s of 32");
}

#[test]
fn stop_is_obeyed_under_a_ban() {
    let (mut a, sound) = in_the_first_dose();
    a.feed(false, dark(), &sound);
    let second = a.press(Press::Stop, dark(), &sound);
    assert!(matches!(second.state, State::BurningOff(None)), "stop under a ban went to {:?}", second.state);
}

// ---- the ban holds a person's hand too ----

#[test]
fn a_ban_in_a_stop_refuses_the_auger_by_hand() {
    let sound = tuning().sound().expect("sound numbers");
    let mut a = Automaton::new(Burning::new());
    assert_eq!(a.by_hand("auger"), Ok(()), "in a stop with the feed allowed the auger is a person's");
    a.feed(false, dark(), &sound);
    let refused = a.by_hand("auger").expect_err("the feed is banned in a stop, and the auger went by hand");
    assert!(refused.contains("Allow feed"), "the words name the button that allows it: {refused}");
    // the pair: allowed again, the hand is free again
    a.feed(true, dark(), &sound);
    assert_eq!(a.by_hand("auger"), Ok(()), "the feed is allowed again, and the auger is still refused");
}

#[test]
fn a_ban_in_a_fault_refuses_the_auger_by_hand() {
    let sound = tuning().sound().expect("sound numbers");
    let mut a = Automaton::new(Burning::new());
    a.press(Press::Start, dark(), &sound);
    for _ in 0..3_600 {
        if matches!(a.state(), State::Fault(_)) {
            break;
        }
        a.second(dark(), 1.0, &sound);
    }
    assert_eq!(a.state(), State::Fault(Why::NeverLit(3)), "a dark pot did not end in a fault");
    assert_eq!(a.by_hand("auger"), Ok(()), "in a fault with the feed allowed the auger is a person's");
    a.feed(false, dark(), &sound);
    assert!(a.by_hand("auger").is_err(), "the feed is banned in a fault, and the auger went by hand");
    assert_eq!(a.by_hand("igniter"), Ok(()), "a ban on the feed took the igniter from a person");
}
