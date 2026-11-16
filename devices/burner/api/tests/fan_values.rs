//! What the fan's module hands the stream for a second: the mode asked, the state the regulator is in by code, the angle and the draught that came out, why the gate stopped by code, the two sliders and the purge, the
//! crossings of the mains and the pulses of the gate - and the numbers under the regulator's knobs. Expected from what the fan does and what a person must be told - a fan asked for a draught is holding it, a gate that
//! refused is said before anything the regulator thinks, a gate asked for less than the fan breaks away on is "wont_start" only while no draught proves it turning, a hundred crossings a second are a mains being
//! followed - and from the first burner's board, which says the same; not from the code that makes the values.

use burner_api::fan::{at, gate_of, mains_refusal, put_setting_values, put_values, Gate, Now, Refusal, Second};
use draught::Reading as Pressure;
use page::dictionary::Value;
use regulator::{Asked, Blind, Carried, Gains, Held, Tuning};
use std::collections::HashMap;

/// A fan holding 50 Pa at 31 % over a mains followed to the pulse: the case every other is a change of.
fn holding() -> Now {
    Now {
        asked_to: Asked::Hold(50.0),
        said: Some((Held::At(31.0), Gate::Out)),
        draught: Some(Pressure::Is(48.0)),
        second: Some(Second { crossings: 100, pulses: 100 }),
        counted_this: true,
        percent: Some(31),
        seeded_pa: 50.0,
    }
}

fn values(now: &Now) -> HashMap<usize, Value> {
    let mut out = HashMap::new();
    put_values(now, &mut |at, value| {
        out.insert(at, value);
    });
    out
}

#[test]
fn a_fan_asked_for_a_draught_is_holding_it_at_the_angle_with_the_pressure_that_came_out() {
    let v = values(&holding());
    assert_eq!(v[&at::HOLDING], Value::Word("hold"));
    assert_eq!(v[&at::HOLDING_STATE], Value::Word("holding"));
    assert_eq!(v[&at::HOLDING_ANGLE], Value::Number(31.0));
    assert_eq!(v[&at::HOLDING_ASKED], Value::Number(50.0));
    assert_eq!(v[&at::HOLDING_CAME_OUT], Value::Number(48.0));
    assert_eq!((v[&at::HOLDING_GATE_WHY], v[&at::HOLDING_STOP_WHY], v[&at::HOLDING_REFUSED_ANGLE]), (Value::None, Value::None, Value::None));
    assert_eq!((v[&at::GATE], v[&at::GATE_SLIDER], v[&at::GATE_STATE], v[&at::GATE_JUDGED]), (Value::Number(31.0), Value::Number(31.0), Value::Word("firing"), Value::Word("ok")));
    assert_eq!((v[&at::ZERO_CROSSINGS], v[&at::ZERO_CROSSINGS_MAINS], v[&at::GATE_PULSES], v[&at::GATE_CROSSINGS]), (Value::Number(100.0), Value::Word("ok"), Value::Number(100.0), Value::Number(100.0)));
}

#[test]
fn what_the_sensor_cannot_say_is_never_a_pressure_that_came_out() {
    for (reading, said) in [
        (Some(Pressure::Over), Value::AtLeast(300.0)),
        (Some(Pressure::Under), Value::Unknown("under")),
        (Some(Pressure::Missing), Value::Unknown("missing")),
        (Some(Pressure::Unconfigured), Value::Unknown("unconfigured")),
        (None, Value::Unknown("not_read")),
    ] {
        let v = values(&Now { draught: reading, ..holding() });
        assert_eq!(v[&at::HOLDING_CAME_OUT], said, "{reading:?}");
    }
}

#[test]
fn a_fan_that_is_not_asked_to_hold_has_no_pressure_to_say_and_no_asked_one() {
    for (asked, mode, state) in [
        (Asked::Stop, "off", "off"),
        (Asked::Purge, "full", "purging"),
    ] {
        let v = values(&Now { asked_to: asked, ..holding() });
        assert_eq!((v[&at::HOLDING], v[&at::HOLDING_STATE]), (Value::Word(mode), Value::Word(state)), "{asked:?}");
        assert_eq!((v[&at::HOLDING_ASKED], v[&at::HOLDING_CAME_OUT]), (Value::None, Value::None), "{asked:?}");
    }
    let by_hand = values(&Now { asked_to: Asked::ByHand, said: Some((Held::Alone, Gate::Out)), ..holding() });
    assert_eq!((by_hand[&at::HOLDING], by_hand[&at::HOLDING_STATE]), (Value::Word("by_hand"), Value::Word("by_hand_free")));
    // the regulator alone, with nobody having asked it by hand: it touches nothing
    let alone = values(&Now { said: Some((Held::Alone, Gate::Out)), ..holding() });
    assert_eq!(alone[&at::HOLDING_STATE], Value::Word("alone"));
    let purging = values(&Now { asked_to: Asked::Purge, ..holding() });
    assert_eq!(purging[&at::PURGE], Value::On(true));
    assert_eq!(values(&holding())[&at::PURGE], Value::On(false));
}

#[test]
fn a_regulator_that_cannot_read_the_draught_says_which_way_it_is_blind_and_holds_no_angle() {
    for (why, state) in [
        (Blind::Under, "blind_under"),
        (Blind::Missing, "blind_missing"),
        (Blind::Unconfigured, "blind_unconfigured"),
        (Blind::NeverStarted, "blind_never_started"),
    ] {
        let v = values(&Now { said: Some((Held::Blind(why), Gate::Out)), ..holding() });
        assert_eq!(v[&at::HOLDING_STATE], Value::Word(state), "{why:?}");
        assert_eq!((v[&at::HOLDING_ANGLE], v[&at::HOLDING_ASKED]), (Value::None, Value::None), "{why:?}");
    }
    assert_eq!(values(&Now { said: None, ..holding() })[&at::HOLDING_STATE], Value::Word("not_holding_yet"));
}

#[test]
fn a_gate_that_refused_is_said_before_anything_the_regulator_thinks_and_by_its_code() {
    let stopped = Now { said: Some((Held::At(31.0), Gate::Stopped { angle: 31, why: Refusal::MainsUnknown })), ..holding() };
    let v = values(&stopped);
    assert_eq!(v[&at::HOLDING_STATE], Value::Word("gate_stopped"));
    assert_eq!((v[&at::HOLDING_REFUSED_ANGLE], v[&at::HOLDING_GATE_WHY], v[&at::HOLDING_STOP_WHY]), (Value::Number(31.0), Value::Word("mains_unknown"), Value::None));
    // and it is not painted over by a regulator that is blind besides
    let blind_and_refused = Now { said: Some((Held::Blind(Blind::Under), Gate::Stopped { angle: 0, why: Refusal::NoGate })), ..holding() };
    assert_eq!(values(&blind_and_refused)[&at::HOLDING_STATE], Value::Word("gate_stopped"));
    let lost = Now { said: Some((Held::At(31.0), Gate::Lost { angle: 31, why: Refusal::GateRefused, stop: Refusal::NoGate })), ..holding() };
    let v = values(&lost);
    assert_eq!(v[&at::HOLDING_STATE], Value::Word("gate_lost"));
    assert_eq!((v[&at::HOLDING_GATE_WHY], v[&at::HOLDING_STOP_WHY]), (Value::Word("gate_refused"), Value::Word("no_gate")));
}

#[test]
fn every_refusal_has_a_code_of_the_list_the_dictionary_names_and_no_two_share_one() {
    let all = [Refusal::MainsUnknown, Refusal::MainsWrong, Refusal::OverScale, Refusal::NoGate, Refusal::GateRefused];
    let codes: Vec<&str> = all.iter().map(|r| r.code()).collect();
    assert_eq!(codes, burner_api::fan::REFUSALS);
}

#[test]
fn the_gate_part_of_a_round_takes_the_refusals_in_the_order_the_gate_gave_them() {
    let stopped = Carried::Stopped { angle: 31, why: "x".into() };
    assert_eq!(gate_of(&stopped, &[Refusal::OverScale]), Gate::Stopped { angle: 31, why: Refusal::OverScale });
    let lost = Carried::Lost { angle: 31, why: "x".into(), stop_why: "y".into() };
    assert_eq!(gate_of(&lost, &[Refusal::MainsWrong, Refusal::GateRefused]), Gate::Lost { angle: 31, why: Refusal::MainsWrong, stop: Refusal::GateRefused });
    assert_eq!(gate_of(&Carried::Out, &[Refusal::NoGate]), Gate::Out);
    // a refusal nobody wrote down is the gate's own
    assert_eq!(gate_of(&stopped, &[]), Gate::Stopped { angle: 31, why: Refusal::GateRefused });
}

#[test]
fn a_mains_that_was_measured_and_is_none_is_not_a_mains_nobody_knows_yet() {
    assert_eq!(mains_refusal(Some(&Err("not a mains".to_string()))), Refusal::MainsWrong);
    assert_eq!(mains_refusal(Some(&Err(String::new()))), Refusal::MainsUnknown);
    assert_eq!(mains_refusal(Some(&Ok(10_000))), Refusal::MainsUnknown);
    assert_eq!(mains_refusal(None), Refusal::MainsUnknown);
}

#[test]
fn the_draught_slider_stands_where_the_fan_is_held_or_where_the_board_comes_up() {
    let v = values(&holding());
    assert_eq!((v[&at::DRAUGHT], v[&at::DRAUGHT_SLIDER], v[&at::DRAUGHT_COMES_UP]), (Value::Number(50.0), Value::Number(50.0), Value::None));
    // held at another than the one kept: the slider shows what is asked and the row says where the board comes up
    let moved = values(&Now { asked_to: Asked::Hold(42.4), seeded_pa: 50.0, ..holding() });
    assert_eq!((moved[&at::DRAUGHT_SLIDER], moved[&at::DRAUGHT_COMES_UP]), (Value::Number(42.0), Value::Number(50.0)));
    // off, the slider waits at the kept setpoint, not at zero
    let off = values(&Now { asked_to: Asked::Stop, ..holding() });
    assert_eq!((off[&at::DRAUGHT], off[&at::DRAUGHT_SLIDER]), (Value::Word("off"), Value::Number(50.0)));
    assert_eq!(values(&Now { asked_to: Asked::Purge, ..holding() })[&at::DRAUGHT], Value::Word("full"));
    assert_eq!(values(&Now { asked_to: Asked::ByHand, ..holding() })[&at::DRAUGHT], Value::Word("by_hand"));
}

#[test]
fn a_gate_that_is_asked_for_nothing_is_off_and_is_not_judged_by_its_pulses() {
    let v = values(&Now { percent: Some(0), second: Some(Second { crossings: 100, pulses: 0 }), ..holding() });
    assert_eq!((v[&at::GATE_STATE], v[&at::GATE_JUDGED]), (Value::Word("off"), Value::Word("ok")));
    let never = values(&Now { percent: None, ..holding() });
    assert_eq!(never[&at::GATE_STATE], Value::Word("off"));
    assert_eq!(never[&at::GATE], Value::Number(0.0));
}

#[test]
fn a_gate_that_does_not_follow_the_mains_is_not_firing_and_is_bad() {
    let v = values(&Now { second: Some(Second { crossings: 100, pulses: 60 }), ..holding() });
    assert_eq!((v[&at::GATE_STATE], v[&at::GATE_JUDGED]), (Value::Word("not_firing"), Value::Word("bad")));
    // one pulse either way is where the second was cut, not a half cycle lost
    let by_one = values(&Now { second: Some(Second { crossings: 101, pulses: 100 }), ..holding() });
    assert_eq!((by_one[&at::GATE_STATE], by_one[&at::GATE_JUDGED]), (Value::Word("firing"), Value::Word("ok")));
    // a second counted before the angle moved says nothing about the angle held now
    let old = values(&Now { second: Some(Second { crossings: 100, pulses: 60 }), counted_this: false, ..holding() });
    assert_eq!((old[&at::GATE_STATE], old[&at::GATE_JUDGED]), (Value::Word("firing"), Value::Word("ok")));
}

#[test]
fn a_fan_asked_for_less_than_it_breaks_away_on_wont_start_only_while_no_draught_proves_it_turning() {
    let standing = Now { percent: Some(20), draught: Some(Pressure::Under), ..holding() };
    assert_eq!(values(&standing)[&at::GATE_STATE], Value::Word("wont_start"));
    // a draught is the one proof of turning this board has: over it, the fan holds under what it started at
    for turning in [Pressure::Is(30.0), Pressure::Over] {
        assert_eq!(values(&Now { draught: Some(turning), ..standing })[&at::GATE_STATE], Value::Word("firing"), "{turning:?}");
    }
    // and at the percent it starts at, there is nothing to say
    assert_eq!(values(&Now { percent: Some(25), ..standing })[&at::GATE_STATE], Value::Word("firing"));
}

#[test]
fn a_mains_that_is_not_a_hundred_crossings_a_second_is_bad_before_anything_else() {
    for crossings in [0, 98, 102, 200] {
        let v = values(&Now { second: Some(Second { crossings, pulses: crossings }), ..holding() });
        assert_eq!((v[&at::ZERO_CROSSINGS_MAINS], v[&at::GATE_JUDGED]), (Value::Word("bad"), Value::Word("bad")), "{crossings}");
    }
    // the first second after a start is its own word, and only for a gate that is asked for something
    let first = values(&Now { second: None, ..holding() });
    assert_eq!(first[&at::GATE_JUDGED], Value::Word("first_second"));
    // nothing asked and nothing counted yet is a mains nobody has followed: bad, not "first second"
    let idle = values(&Now { second: None, percent: None, ..holding() });
    assert_eq!(idle[&at::GATE_JUDGED], Value::Word("bad"));
}

#[test]
fn the_knobs_numbers_are_the_regulators_own_on_the_settings_standing_now() {
    let tuning = Tuning {
        up: Gains { kp_milli: 300, ki_milli: 400 },
        down: Gains { kp_milli: 300, ki_milli: 400 },
        min: 8.0,
        max: 80.0,
        step: 5.0,
        band_pa: 4.0,
        band_share: 0.25,
    };
    let mut v: HashMap<usize, Value> = HashMap::new();
    put_setting_values(&tuning, (42.0, 17.0), &mut |at, value| {
        v.insert(at, value);
    });
    let number = |at: usize| match v[&at] {
        Value::Number(n) => n,
        other => panic!("{other:?}"),
    };
    assert!((number(at::GAINS_MOVE[0]) - 0.59).abs() < 0.005 && (number(at::GAINS_MOVE[1]) - 0.78).abs() < 0.005);
    assert_eq!((number(at::STEP_FROM), number(at::STEP_TO)), (8.0, 80.0));
    assert!((number(at::STEP_TAKES) - 14.4).abs() < 0.01);
    assert_eq!((number(at::BAND_BAND), number(at::SHARE_BAND)), (4.0, 4.0));
    assert!((number(at::SHARE_UP) - 0.06).abs() < 0.005 && (number(at::SHARE_DOWN) - 0.06).abs() < 0.005);
    // a step of nothing has no travel time; a regulator too weak to bring a jump back says it did not
    let mut stuck: HashMap<usize, Value> = HashMap::new();
    let weak = Tuning { step: 0.0, up: Gains { kp_milli: 0, ki_milli: 1 }, down: Gains { kp_milli: 0, ki_milli: 1 }, ..tuning };
    put_setting_values(&weak, (42.0, 17.0), &mut |at, value| {
        stuck.insert(at, value);
    });
    assert_eq!((stuck[&at::STEP_TAKES], stuck[&at::BAND_BACK_IN]), (Value::Unknown("no_step"), Value::Unknown("not_back")));
}

#[test]
fn a_fan_with_no_point_to_be_answered_for_says_nothing_under_its_knobs() {
    let tuning = Tuning { up: Gains { kp_milli: 300, ki_milli: 400 }, down: Gains { kp_milli: 300, ki_milli: 400 }, min: 8.0, max: 80.0, step: 5.0, band_pa: 4.0, band_share: 0.25 };
    let mut said = 0;
    put_setting_values(&tuning, (0.0, 0.0), &mut |_, _| said += 1);
    assert_eq!(said, 0);
}
