//! A person's Light carries the fire to the nominal of whoever holds the burner, through the real fire and the real thermostat: the burner lit by hand over a room that says `warm` inside its band goes on burning until the room reaches its
//! target, and only then goes the ordinary way down; a Light over a room at its target lights nothing; a Stop ends the cycle; a protection puts the fire out over it, and the cycle does not light it again.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: 29.10 15:31:28 the room stood at 21.1 under a target of 22, the person pressed Light, the fire was in work at 15:36:21 and a second after that, `warm` (the room coming
//! down from above, inside its band), it was smouldering; at 15:36:45 it was out. "The burner lit by hand goes to the nominal" is the person's word of 29.10 and 30.10. The thermostat's own rows are `crates/thermostat/tests/manual.rs`; these are the same
//! cases with the fire that carries them out, seconds counted.

mod common;

use automaton::{Feed, Lighting, State, Step, Told, Why};
use common::{Burning, Hold, Seen, Silence, Tuning};
use demand::{Asked, By, Player, Tuning as Asks};
use flame::Flame;
use schedule::Schedule;

const TARGET: f32 = 22.0;

fn tuning() -> Tuning {
    Tuning {
        purge_before_s: 30.0,
        purge_after_s: 20.0,
        igniter_s: 240.0,
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
    tuning().sound().expect("sound numbers")
}

fn cold() -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0), draught_pa: Some(0.0), draught_low: false, gate: 0, silence: Silence::HEARD }
}

fn lit(boiler_c: f32) -> Seen {
    Seen { flame: Flame::Steady, boiler_c: Some(boiler_c), return_c: None, flue_c: Some(150.0), draught_pa: Some(120.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// The house as the room's player says it: the room at ROOM_C after it stood above the target - 29.10, the room cooling inside its band.
fn house(room_c: f32) -> Option<Asked> {
    let tuning = Asks { by: By::Room, room_c: TARGET, room_eco_c: 17.0, room_drop_c: 2.0, schedule: Schedule::default() };
    let mut player = Player::new();
    player.ask(&tuning, Some(23.0), None);
    player.ask(&tuning, Some(room_c), None).asked
}

/// A burner that has come up a while ago and stands in Stop, the room at ROOM_C under a target of 22 with the house saying what its player says.
fn board(room_c: f32) -> Burning {
    let mut burning = Burning::new();
    burning.wait_for_first_round();
    burning.first_round_done();
    burning.ask(house(room_c));
    burning.room(Some(room_c), Some(TARGET));
    burning
}

/// One second at ROOM_C: the house says what its player says there, and the fire is given SEEN.
fn second(burning: &mut Burning, room_c: f32, seen: Seen, told: Told) -> State {
    burning.ask(house(room_c));
    burning.room(Some(room_c), Some(TARGET));
    burning.step(told, seen, 1.0, &sound());
    burning.state()
}

/// Light, the thirty seconds of the purge, and a lit pot until the fire works: the way 29.10 15:31-15:36 went, in as many seconds as the tuning needs.
fn lit_by_hand(burning: &mut Burning, room_c: f32) {
    second(burning, room_c, cold(), Told::Light);
    for _ in 0..30 {
        second(burning, room_c, cold(), Told::Nothing);
    }
    for _ in 0..120 {
        if second(burning, room_c, lit(40.0), Told::Nothing) == State::Working {
            return;
        }
    }
    panic!("the scaffolding did not reach work: {:?}", burning.state());
}

#[test]
fn a_burner_lit_by_hand_over_a_room_under_its_target_inside_its_band_works_until_the_room_reaches_the_target_and_then_smoulders() {
    let mut burning = board(21.1);
    lit_by_hand(&mut burning, 21.1);
    // the 29.10 second: the house says warm, and the fire used to go down on it at once; two minutes at 21.1 and a climb to 21.99 are all work
    for room_c in std::iter::repeat(21.1).take(120).chain([21.5, 21.8, 21.99]) {
        assert_eq!(second(&mut burning, room_c, lit(45.0), Told::Nothing), State::Working, "the room at {room_c} is under its target of {TARGET}");
    }
    // the target: the cycle ends, and the house's word (warm) takes the fire down the ordinary way
    let mut state = second(&mut burning, 22.0, lit(45.0), Told::Nothing);
    for _ in 0..5 {
        state = second(&mut burning, 22.0, lit(45.0), Told::Nothing);
    }
    assert_eq!(state, State::Smouldering, "the room reached its target: smouldering is the ordinary way down");
    assert_eq!(burning.ended(), Some(thermostat::Ended::Reached));
}

#[test]
fn a_light_over_a_room_at_its_target_lights_nothing_and_says_by_how_much() {
    let mut burning = board(22.3);
    second(&mut burning, 22.3, cold(), Told::Light);
    for _ in 0..40 {
        assert_eq!(second(&mut burning, 22.3, cold(), Told::Nothing), State::Stop, "a refused Light lit the burner");
    }
    assert_eq!(burning.refused(), Some(thermostat::Refusal { nominal: thermostat::Nominal::Room, now_c: 22.3, to_c: TARGET }));
    assert_eq!(burning.cycle(), None);
}

#[test]
fn a_stop_in_the_middle_of_a_cycle_ends_it_and_the_burner_is_not_lit_again_by_it() {
    let mut burning = board(21.1);
    lit_by_hand(&mut burning, 21.1);
    second(&mut burning, 21.1, lit(45.0), Told::Stop);
    assert_eq!(burning.ended(), Some(thermostat::Ended::Stopped));
    // the burning off runs its course and the burner stands in Stop with the room still under its target: nothing lights it
    for _ in 0..400 {
        second(&mut burning, 21.1, cold(), Told::Nothing);
    }
    assert_eq!(burning.state(), State::Stop, "the burner was lit again by a cycle that a Stop had ended");
}

#[test]
fn water_over_its_setpoint_ends_a_cycle_of_the_room_and_the_protection_of_the_boiler_burns_the_fire_off_as_it_always_did() {
    let mut burning = board(21.1);
    lit_by_hand(&mut burning, 21.1);
    // the water goes over its setpoint of 60 and over the overheating (88): the ceiling ends the cycle at once, the fire's own protection burns it off
    let mut state = second(&mut burning, 21.1, lit(90.0), Told::Nothing);
    assert_eq!(burning.ended(), Some(thermostat::Ended::Water), "the ceiling of the water stands under a cycle of the room");
    for _ in 0..3 {
        state = second(&mut burning, 21.1, lit(90.0), Told::Nothing);
    }
    assert!(matches!(state, State::BurningOff(Some(Why::Overheated(_)))), "{state:?}");
    for _ in 0..400 {
        second(&mut burning, 21.1, cold(), Told::Nothing);
    }
    assert!(matches!(burning.state(), State::Fault(Why::Overheated(_))), "the burner went on after its protection: {:?}", burning.state());
}

#[test]
fn a_protection_puts_the_fire_out_over_a_cycle_held_by_the_return_and_the_cycle_does_not_light_it_again() {
    // held by the return: the ceiling is the return's, so a boiler over its overheating is a thing the cycle meets, and only the fire's protection answers it
    let by_return = |seen: Seen| Seen { return_c: Some(40.0), ..seen };
    let t = Tuning { hold: Hold::Return, ..tuning() }.sound().expect("sound numbers");
    let second = |burning: &mut Burning, seen: Seen, told: Told| {
        burning.ask(house(21.1));
        burning.room(Some(21.1), Some(TARGET));
        burning.step(told, seen, 1.0, &t);
        burning.state()
    };
    let mut burning = board(21.1);
    second(&mut burning, by_return(cold()), Told::Light);
    for _ in 0..30 {
        second(&mut burning, by_return(cold()), Told::Nothing);
    }
    for _ in 0..120 {
        if second(&mut burning, by_return(lit(40.0)), Told::Nothing) == State::Working {
            break;
        }
    }
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach work");
    let mut state = State::Working;
    for _ in 0..4 {
        state = second(&mut burning, by_return(lit(90.0)), Told::Nothing);
    }
    assert!(matches!(state, State::BurningOff(Some(Why::Overheated(_)))), "the cycle held a fire the water had put over its protection: {state:?}");
    // the fire burns off into a fault, which waits for a person: the cycle ended with the fire, and a Stop lets the fault go with the room still under its target
    for _ in 0..400 {
        second(&mut burning, by_return(cold()), Told::Nothing);
    }
    assert!(matches!(burning.state(), State::Fault(Why::Overheated(_))), "{:?}", burning.state());
    assert_eq!(burning.ended(), Some(thermostat::Ended::Out));
    assert_eq!(burning.cycle(), None);
    second(&mut burning, by_return(cold()), Told::Stop);
    for _ in 0..200 {
        second(&mut burning, by_return(cold()), Told::Nothing);
    }
    assert_eq!(burning.state(), State::Stop, "the cycle lit the burner again after a fault");
}
