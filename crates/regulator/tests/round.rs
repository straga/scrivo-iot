//! One round of the regulator, the gate handed in: what it decides is carried to the gate as a whole percent, a refused angle is followed by a stop and the regulator is told, a regulator that cannot read
//! the draught stops the fan, and a person holding the angle by hand is not written over. The gate here is a row of answers: the board's own is a triac and cannot be asked to refuse.

use draught::Reading;
use regulator::{round, Asked, Blind, Carried, Gains, Held, Holding, Tuning};

fn tuning() -> Tuning {
    Tuning { up: Gains { kp_milli: 300, ki_milli: 400 }, down: Gains { kp_milli: 300, ki_milli: 400 }, min: 8.0, max: 80.0, step: 5.0, band_pa: 4.0, band_share: 0.25 }
}

/// A gate that takes every angle it is given and writes it down.
fn taking(seen: &mut Vec<u32>) -> impl FnMut(u32) -> Result<String, String> + '_ {
    move |percent| {
        seen.push(percent);
        Ok(format!("{percent}%"))
    }
}

/// A regulator that has been holding 42 Pa for a while at an angle it was started on: it turns.
fn turning_at(angle: f32) -> Holding {
    let mut holding = Holding::from(angle);
    for _ in 0..30 {
        let mut seen = Vec::new();
        round(&mut holding, Asked::Hold(42.0), Reading::Is(42.0), 1.0, &tuning(), &mut taking(&mut seen));
    }
    assert!(holding.turning(), "the setup: a regulator that has held a draught for thirty seconds turns");
    holding
}

#[test]
fn a_decision_is_carried_to_the_gate_as_a_whole_percent() {
    let mut holding = turning_at(30.0);
    let mut seen = Vec::new();
    let out = round(&mut holding, Asked::Hold(50.0), Reading::Is(40.0), 1.0, &tuning(), &mut taking(&mut seen));
    let Held::At(angle) = out.held else { panic!("a regulator that reads 40 Pa of 50 holds an angle: {:?}", out.held) };
    assert_eq!(seen, vec![angle.round() as u32], "the gate is asked once, for the angle decided on rounded to a whole percent");
    assert_eq!((out.commanded, out.carried), (Some(angle.round() as u32), Carried::Out));
}

#[test]
fn a_refused_angle_is_followed_by_a_stop_and_the_regulator_is_told_it_did_not_get_through() {
    let mut holding = turning_at(30.0);
    let mut asked_for = Vec::new();
    let mut refusing = |percent: u32| {
        asked_for.push(percent);
        if percent == 0 { Ok("0%".to_string()) } else { Err("the gate refused".to_string()) }
    };
    let out = round(&mut holding, Asked::Hold(50.0), Reading::Is(40.0), 1.0, &tuning(), &mut refusing);
    assert!(matches!(out.carried, Carried::Stopped { .. }), "{:?}", out.carried);
    assert_eq!(asked_for.len(), 2);
    assert_eq!(asked_for[1], 0, "the stop follows the refusal, always");
    assert!(!holding.turning(), "what could not be carried out cannot be continued from: the regulator is told");
}

#[test]
fn a_gate_that_refuses_even_the_stop_is_lost_and_the_regulator_is_told_too() {
    let mut holding = turning_at(30.0);
    let mut never = |_: u32| Err::<String, String>("no".to_string());
    let out = round(&mut holding, Asked::Hold(50.0), Reading::Is(40.0), 1.0, &tuning(), &mut never);
    assert!(matches!(out.carried, Carried::Lost { .. }), "{:?}", out.carried);
    assert!(!holding.turning());
}

#[test]
fn a_regulator_that_cannot_read_the_draught_stops_the_fan_and_says_why() {
    let mut holding = turning_at(30.0);
    let mut seen = Vec::new();
    let out = round(&mut holding, Asked::Hold(50.0), Reading::Under, 1.0, &tuning(), &mut taking(&mut seen));
    assert_eq!(out.held, Held::Blind(Blind::Under));
    assert_eq!((seen, out.commanded, out.carried), (vec![0], Some(0), Carried::Out), "blind, it commands zero and not the last angle");
}

#[test]
fn a_person_holding_the_angle_by_hand_is_not_written_over() {
    let mut holding = turning_at(30.0);
    let mut seen = Vec::new();
    let out = round(&mut holding, Asked::ByHand, Reading::Is(40.0), 1.0, &tuning(), &mut taking(&mut seen));
    assert_eq!(out.held, Held::Alone);
    assert!(seen.is_empty(), "not even a zero: {seen:?}");
    assert_eq!((out.commanded, out.carried), (None, Carried::Out));
    assert!(holding.turning(), "nothing was refused, so nothing is told");
}
