//! A board that comes up over a fire blows in its first second: the automaton asks for air,
//! the regulator turns that into a percent, and the gate fires it before the mains has been measured - ten seconds of
//! counting are ten seconds of fire without air otherwise. Followed through the three crates the board runs, with
//! nothing between them made up.

mod common;

use automaton::{Feed, Lighting, State, Step, Told};
use common::{Hold, Burning, Seen, Silence, Tuning};
use flame::Flame;
use regulator::{Held, Holding};

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
        burn_out_min_s: 60.0,
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
        kindling: Lighting { pa: 45.0, dose_s: 32.0 },
        step: Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

/// A regulator's gains and limits for these rows, written here: the first second after a boot is a kick whatever they are.
fn holding_tuning() -> regulator::Tuning {
    regulator::Tuning {
        up: regulator::Gains { kp_milli: 300, ki_milli: 400 },
        down: regulator::Gains { kp_milli: 300, ki_milli: 400 },
        min: 8.0,
        max: 80.0,
        step: 5.0,
        band_pa: 4.0,
        band_share: 0.25,
    }
}

/// The first second after a boot remembering WAS over SEEN: what the gate does with it, before the mains is measured,
/// on a live detector (a hundred crossings in the last second).
fn first_second(was: State, seen: Seen) -> dimmer::Order {
    let sound = tuning().sound().expect("sound numbers");
    let mut burning = Burning::resumed(was, &seen, &sound);
    let asked = burning.step(Told::Nothing, seen, 1.0, &sound).fan;
    // the fan stands at the boot: the sensor reads under its zero
    let held = Holding::from(0.0).step(asked, draught::Reading::Under, 1.0, &holding_tuning());
    let percent = match held {
        Held::At(angle) => angle.round() as u32,
        other => panic!("the regulator did not command the gate: {other:?} for {asked:?}"),
    };
    let half = dimmer::half_to_fire(None, 100);
    match percent {
        0 => dimmer::Order::Off,
        p => dimmer::order(p, half.expect("the gate refused to fire before the mains was measured")).unwrap(),
    }
}

#[test]
fn a_boot_over_a_hot_flue_fires_the_gate_in_its_first_second() {
    let hot = Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: None, flue_c: Some(150.0), draught_pa: None, draught_low: false, gate: 0, silence: Silence::HEARD };
    assert!(matches!(first_second(State::Stop, hot), dimmer::Order::Fire { .. }), "a hot pot got no air");
}

#[test]
fn a_boot_over_a_live_flame_fires_the_gate_in_its_first_second() {
    let lit = Seen { flame: Flame::Steady, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0), draught_pa: None, draught_low: false, gate: 0, silence: Silence::HEARD };
    assert!(matches!(first_second(State::Fault(automaton::Why::NeverLit(3)), lit), dimmer::Order::Fire { .. }));
}

#[test]
fn a_boot_with_no_sign_of_fire_leaves_the_fan_alone() {
    // the pair: a cold, dark pot remembered as stopped - the one cell where the fan stands
    let cold = Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0), draught_pa: None, draught_low: false, gate: 0, silence: Silence::HEARD };
    assert_eq!(first_second(State::Stop, cold), dimmer::Order::Off);
}
