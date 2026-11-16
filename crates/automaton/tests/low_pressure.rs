//! A sensor that answers under its own zero is not a sensor that is gone. Followed through the two crates the board runs for it - the
//! automaton that decides to burn off, and the regulator that decides what the fan does - with the readings of the record of 29.10 and
//! the seconds either side of it. Nothing between the crates is made up but the one fact the device adds: a reading that is a number reaches the automaton as a number.
//!
//! **THE ROWS THAT CAME BEFORE THE FIX AND THE ROWS AFTER IT ARE THE SAME ROWS.** What each says is a person's: a dip of the draught that comes back
//! within its term is not a fault, a sensor that is not there is one at once, and a dip that stays is one with its own name.

mod common;

use automaton::{Asked, Feed, Lighting, State, Step, Told, Why};
use common::{Burning, Hold, Seen, Silence, Tuning};
use draught::Reading;
use flame::Flame;
use regulator::{Blind, Held, Holding};

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

fn sound() -> common::Sound {
    tuning().sound().expect("sound numbers")
}

/// What the board reads with the fire lit and the reading of the draught READING.
fn lit_with(reading: Reading) -> Seen {
    // THE ONE FACT THE DEVICE ADDS (devices/burner/src/modules/automaton.rs, `seen`): a number reaches the automaton as a number, anything else as none
    let draught_pa = match reading {
        Reading::Is(pa) => Some(pa),
        Reading::Over => Some(draught::FULL_PA as f32),
        Reading::Under | Reading::Missing | Reading::Unconfigured => None,
    };
    Seen { flame: Flame::Steady, boiler_c: Some(40.0), return_c: None, flue_c: Some(150.0), draught_pa, draught_low: reading == Reading::Under, gate: 30, silence: Silence::HEARD }
}

/// A burner lit and rising, with the fan held at what it was asked for: the seconds of the lighting are scaffolding, the rows start in the climb.
fn rising() -> Burning {
    let t = sound();
    let cold = Seen { flame: Flame::Out, draught_pa: Some(0.0), gate: 0, flue_c: Some(20.0), ..lit_with(Reading::Is(0.0)) };
    let mut burning = Burning::new();
    burning.step(Told::Start, cold, 1.0, &t);
    for _ in 0..30 {
        burning.step(Told::Nothing, cold, 1.0, &t);
    }
    for _ in 0..60 {
        if burning.state() == State::Rising {
            break;
        }
        burning.step(Told::Nothing, lit_with(Reading::Is(50.0)), 1.0, &t);
    }
    assert_eq!(burning.state(), State::Rising, "the scaffolding did not reach the climb");
    burning
}

/// SECONDS of READING; what the last of them decided.
fn reading_for(burning: &mut Burning, reading: Reading, seconds: usize) -> State {
    let t = sound();
    for _ in 0..seconds {
        burning.step(Told::Nothing, lit_with(reading), 1.0, &t);
    }
    burning.state()
}

/// The record of 29.10 15:44:23-33 as the card said it, one reading a second: the sensor under its zero for six seconds with the fan at 8 %, then numbers.
const RECORD_29_10: [Reading; 11] = [
    Reading::Under,
    Reading::Under,
    Reading::Under,
    Reading::Under,
    Reading::Under,
    Reading::Under,
    Reading::Is(11.0),
    Reading::Is(27.0),
    Reading::Is(43.0),
    Reading::Is(54.0),
    Reading::Is(68.0),
];

#[test]
fn the_record_of_29_10_second_by_second_does_not_burn_off_by_the_draught_rule_alone() {
    // the record's own burner went to `no_draught` at 15:44:33 with a number on the card since 15:44:29; the automaton's rule alone does not say why - this row holds what it does say
    let mut burning = rising();
    for reading in RECORD_29_10 {
        assert_eq!(reading_for(&mut burning, reading, 1), State::Rising, "{reading:?} burnt the fire off");
    }
}

#[test]
fn a_dip_under_the_zero_that_comes_back_is_not_a_fault_once_the_fan_has_had_its_spin_up() {
    // 12 s of numbers: the fan is asked for a setpoint, `spin_up_s` (10 s) is over, and from here a second without a number was a fault by itself
    let mut burning = rising();
    reading_for(&mut burning, Reading::Is(40.0), 12);
    assert_eq!(burning.state(), State::Rising);
    assert_eq!(reading_for(&mut burning, Reading::Under, 6), State::Rising, "six seconds under the zero, inside their term, burnt the fire off");
    assert_eq!(reading_for(&mut burning, Reading::Is(30.0), 1), State::Rising, "and the number's return did not undo it");
}

#[test]
fn a_dip_under_the_zero_burns_off_by_its_own_cause_on_the_second_after_its_term_and_not_before() {
    let mut burning = rising();
    reading_for(&mut burning, Reading::Is(40.0), 12);
    // the term is 20 s (`low_pressure_s` of this tuning): twenty seconds are inside it
    assert_eq!(reading_for(&mut burning, Reading::Under, 20), State::Rising, "the term was cut short");
    assert_eq!(reading_for(&mut burning, Reading::Under, 1), State::BurningOff(Some(Why::LowPressure)), "the second after the term did not name low pressure");
}

#[test]
fn a_number_that_comes_back_inside_the_term_starts_the_count_again() {
    let mut burning = rising();
    reading_for(&mut burning, Reading::Is(40.0), 12);
    for _ in 0..3 {
        for (reading, seconds) in [(Reading::Under, 15), (Reading::Is(10.0), 1)] {
            let state = reading_for(&mut burning, reading, seconds);
            assert!(!matches!(state, State::BurningOff(_) | State::Fault(_)), "{reading:?} burnt the fire off: {state:?}");
        }
    }
}

#[test]
fn a_sensor_that_is_gone_burns_off_at_once_after_the_spin_up_as_it_always_did() {
    let mut burning = rising();
    reading_for(&mut burning, Reading::Is(40.0), 12);
    let state = reading_for(&mut burning, Reading::Missing, 1);
    assert_eq!(state, State::BurningOff(Some(Why::NoDraught)), "a sensor that is not there did not burn the fire off");
}

#[test]
fn a_fan_that_turns_and_reads_under_the_zero_is_stopped_for_a_second_and_kicked_again_as_the_regulator_always_did() {
    // the regulator's own recovery of a stalled rotor (crates/regulator: "the tract empties, `Under` arrives, `stood` runs, and only then a kick"); the record of 29.10 15:44:26-27 shows it: one second blind, then the fan held at 33 %.
    // This row keeps it where it is: the fix of the low pressure is the automaton's term and not a change of this recovery.
    let t = holding_tuning();
    let mut holding = Holding::from(0.0);
    for _ in 0..4 {
        holding.step(Asked::Hold(45.0), Reading::Is(20.0), 1.0, &t);
    }
    assert_eq!(holding.step(Asked::Hold(45.0), Reading::Under, 1.0, &t), Held::Blind(Blind::Under));
}

#[test]
fn a_fan_that_turns_over_a_sensor_that_is_gone_is_stopped_as_it_always_was() {
    let t = holding_tuning();
    let mut holding = Holding::from(0.0);
    for _ in 0..4 {
        holding.step(Asked::Hold(45.0), Reading::Is(20.0), 1.0, &t);
    }
    assert_eq!(holding.step(Asked::Hold(45.0), Reading::Missing, 1.0, &t), Held::Blind(Blind::Missing));
}

#[test]
fn the_automaton_the_board_holds_counts_the_seconds_without_a_number_and_under_the_zero_for_the_journal() {
    // the device's journal line asks the `Automaton` (crates/automaton, run.rs) and not the `Burning` under it: the answer is the same counts
    let t = sound();
    let lit = automaton::Seen { flame: Flame::Steady, boiler_c: Some(40.0), flue_c: Some(150.0), draught_pa: Some(50.0), draught_low: false, gate: 30, silence: automaton::Silence::HEARD };
    let mut board = automaton::Automaton::new(automaton::Burning::resumed(State::Working, &lit, &t.fire));
    let order = thermostat::Thermostat::new().order(&thermostat::Temperatures { boiler_c: Some(40.0), return_c: None, return_heard: automaton::Heard::Speaks, asked: None, room_c: None, room_target_c: None, fire: thermostat::Fire::default() }, &t.heat);
    for _ in 0..3 {
        board.second(&order, automaton::Seen { draught_pa: None, draught_low: true, ..lit }, 1.0, &t.fire);
    }
    let counts = board.draught_counts();
    assert_eq!((counts.no_number_s, counts.low_s), (3.0, 3.0), "{counts:?}");
    board.second(&order, lit, 1.0, &t.fire);
    assert_eq!(board.draught_counts().no_number_s, 0.0, "a number ended the count");
}
