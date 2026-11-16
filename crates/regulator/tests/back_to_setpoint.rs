//! How fast the regulator comes back to its setpoint on the numbers of 3.10 (300/400, a band of 4 Pa), in the four cases the burner
//! showed on 26.09: the igniter switched off, the fire gone out, a
//! small setpoint on an empty pot, a setpoint moved. Each has to be back within 15 s. The pair: near the setpoint the
//! gate moves no more than half as much again as it did on the numbers of 26.09.
//!
//! **THE TRACT IS THE MEASURED ONE AND THE TROUBLE IS THE DAY'S.** `pa = m·0.186·angle^1.92` with the tract's 4 s
//! (crates/regulator/tests/holding.rs, `ColdFan`), one more second before an angle reaches the air, and the gate given
//! a whole percent, as `regulator::carry` gives it. What changed on 26.09 is `m`: how much the pot in front of the fan
//! holds the air back. Switching the igniter off made it 1.74 times steeper over a few seconds with no fire in the pot
//! (17:12:35 and 17:22:03: 45 Pa at 22 % became 50 Pa at 17 %); the fire going out made it 0.55 times as steep
//! (17:29:59: 17.6 % -> 23.5 % for the same 42 Pa); the empty pot after a purge held 7 Pa at 12.5 % (17:49).
//!
//! **THE BREATHING OF THE AIR IS THE BURNER'S TOO:** a reading moves 1.7-1.9 Pa from one second to the next under fire
//! and cold (CSV 26.09 17:23-17:53), which uniform noise of +-2.2 Pa reproduces. The time is counted on the pressure
//! in the tract, not on the reading: a reading is the pressure plus the breathing, and "back" is about the pressure.

use std::collections::VecDeque;

use draught::Reading;
use regulator::{Asked, Gains, Held, Holding, Tuning};

/// The steepness and the power of the fan's curve, measured 21.09 (holding.rs, `STEEPNESS`, `CURVE`).
const STEEPNESS: f32 = 0.186;
const CURVE: f32 = 1.92;
/// The tract's first-order time constant, measured 21.09 (`regulator::TRACT_ANSWERS_S`).
const TRACT_S: f32 = 4.0;
/// How many seconds the trouble of the day took to arrive: the pressure after the igniter went off climbed for 5-10 s.
const TROUBLE_S: f32 = 5.0;
/// Half the width of the breathing, uniform, in Pa - see the header.
const BREATHING_PA: f32 = 2.2;

/// A boiler whose pot holds the air back by M, which moves towards M_GOAL over TROUBLE_S.
struct Burner {
    pa: f32,
    m: f32,
    m_goal: f32,
    seed: u32,
    /// the angles on their way to the air: one second more than the step itself
    late: VecDeque<f32>,
}

impl Burner {
    fn at(angle: f32, m: f32, seed: u32) -> Self {
        Self { pa: m * STEEPNESS * angle.powf(CURVE), m, m_goal: m, seed, late: VecDeque::from([angle]) }
    }

    /// A second of blowing at the angle the regulator asked for; answers what the sensor says.
    fn second(&mut self, angle: f32) -> Reading {
        self.m += (self.m_goal - self.m) / TROUBLE_S;
        // the gate is given a whole percent (regulator::carry takes angle.round())
        self.late.push_back(angle.round());
        let angle = self.late.pop_front().unwrap();
        let wants = self.m * STEEPNESS * angle.powf(CURVE);
        self.pa += (wants - self.pa) / TRACT_S;
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let breath = ((self.seed >> 16) % 601) as f32 / 300.0 - 1.0;
        Reading::Is((self.pa + breath * BREATHING_PA).max(0.6))
    }
}

/// The numbers these rows judge the regulator by: 300/400 both ways and a band of 4 Pa at a quarter. The
/// rows are about what the regulator DOES with such numbers; which numbers a board holds is a person's to set.
fn board() -> Tuning {
    Tuning {
        up: Gains { kp_milli: 300, ki_milli: 400 },
        down: Gains { kp_milli: 300, ki_milli: 400 },
        min: 8.0,
        max: 80.0,
        step: 5.0,
        band_pa: 4.0,
        band_share: 0.25,
    }
}

/// The numbers the burner ran on 26.09: the yardstick the pair is measured against.
fn of_26_09() -> Tuning {
    Tuning {
        up: Gains { kp_milli: 150, ki_milli: 45 },
        down: Gains { kp_milli: 150, ki_milli: 30 },
        // there was no band on 26.09: the gains acted in full at any miss
        band_pa: 0.0,
        band_share: 1.0,
        ..board()
    }
}

/// Holds WANT at ANGLE with the pot at M for four minutes, then the pot goes to M_THEN and the setpoint to WANT_THEN.
/// Answers the second the pressure came within WITHIN Pa of WANT_THEN and stayed there ten seconds, if it did in
/// five minutes.
fn back_after(t: &Tuning, angle: f32, m: f32, want: f32, m_then: f32, want_then: f32, within: f32) -> Option<u32> {
    let mut burner = Burner::at(angle, m, 7);
    let mut holding = Holding::from(angle);
    let mut angle = angle;
    let step = |burner: &mut Burner, holding: &mut Holding, angle: &mut f32, want: f32| {
        let said = burner.second(*angle);
        match holding.step(Asked::Hold(want), said, 1.0, t) {
            Held::At(at) => *angle = at,
            other => panic!("the regulator answered {other:?} on a turning fan with a reading"),
        }
    };
    for _ in 0..240 {
        step(&mut burner, &mut holding, &mut angle, want);
    }
    burner.m_goal = m_then;
    let mut inside = 0;
    for second in 1..=300 {
        step(&mut burner, &mut holding, &mut angle, want_then);
        inside = if (burner.pa - want_then).abs() <= within { inside + 1 } else { 0 };
        if inside == 10 {
            return Some(second - 9);
        }
    }
    None
}

/// How far the gate travels a second, on average, while the regulator holds 42 Pa under fire: 100 s after 300 s of
/// holding, over forty runs of breathing.
fn gate_travel(t: &Tuning) -> f32 {
    let mut travel = 0.0;
    for seed in 1..=40 {
        let mut burner = Burner::at(17.0, 1.13, seed);
        let mut holding = Holding::from(17.0);
        let mut angle = 17.0f32;
        for second in 0..400 {
            let was = angle.round();
            if let Held::At(at) = holding.step(Asked::Hold(42.0), burner.second(angle), 1.0, t) {
                angle = at;
            }
            if second >= 300 {
                travel += (angle.round() - was).abs();
            }
        }
    }
    travel / (40.0 * 100.0)
}

const BACK_S: u32 = 15;

#[test]
fn after_the_igniter_goes_off_the_draught_is_back_within_fifteen_seconds() {
    // 26.09 17:22:03: 45 Pa held at 22 % with the igniter on, and the pot 1.74 times steeper once it went off
    let back = back_after(&board(), 22.0, 0.65, 45.0, 1.13, 45.0, 5.0);
    println!("igniter off: back in {back:?} s");
    assert!(back.is_some_and(|s| s <= BACK_S), "the draught came back in {back:?} s, and {BACK_S} is the most");
}

#[test]
fn after_the_fire_goes_out_the_draught_is_back_within_fifteen_seconds() {
    // 26.09 17:29:59: 42 Pa held at 17 % under fire, and the pot 0.55 times as steep once the fire was out
    let back = back_after(&board(), 17.0, 1.13, 42.0, 0.62, 42.0, 5.0);
    println!("fire out: back in {back:?} s");
    assert!(back.is_some_and(|s| s <= BACK_S), "the draught came back in {back:?} s, and {BACK_S} is the most");
}

#[test]
fn six_pascals_on_an_empty_pot_are_reached_within_fifteen_seconds() {
    // 26.09 17:45:03: the setpoint 20 -> 6 Pa after a purge, where 12.5 % held 7 Pa; within 2 Pa, since 5 would be
    // most of the setpoint
    let back = back_after(&board(), 20.0, 0.3, 20.0, 0.3, 6.0, 2.0);
    println!("20 -> 6 Pa: there in {back:?} s");
    assert!(back.is_some_and(|s| s <= BACK_S), "6 Pa was reached in {back:?} s, and {BACK_S} is the most");
}

#[test]
fn a_setpoint_moved_under_fire_is_reached_within_fifteen_seconds() {
    // 26.09 17:25:51: the setpoint of the step 51 -> 42 Pa under fire
    let back = back_after(&board(), 18.0, 1.13, 51.0, 1.13, 42.0, 5.0);
    println!("51 -> 42 Pa: there in {back:?} s");
    assert!(back.is_some_and(|s| s <= BACK_S), "42 Pa was reached in {back:?} s, and {BACK_S} is the most");
}

#[test]
fn near_the_setpoint_the_gate_travels_no_more_than_half_as_much_again_as_on_26_09() {
    let now = gate_travel(&board());
    let then = gate_travel(&of_26_09());
    println!("gate travel near the setpoint: {now:.3} %/s, on the numbers of 26.09 {then:.3} %/s, {:.2} times", now / then);
    assert!(now <= 1.5 * then, "the gate travels {now:.3} %/s near the setpoint, {:.2} times the {then:.3} of 26.09",
            now / then);
}

#[test]
fn quick_gains_without_the_band_chase_the_breathing_and_the_measure_sees_it() {
    // The pair of the one above: the yardstick has to be able to say no. The gains of the owner's first word on 26.09,
    // 1000/400, in full at every miss, answer the breathing with the gate, and the measure has to catch that.
    let chasing = Tuning { up: Gains { kp_milli: 1000, ki_milli: 400 }, down: Gains { kp_milli: 1000, ki_milli: 400 },
                           band_pa: 0.0, band_share: 1.0, ..board() };
    let then = gate_travel(&of_26_09());
    let now = gate_travel(&chasing);
    println!("1000/400 with no band: {now:.3} %/s, {:.2} times 26.09", now / then);
    assert!(now > 1.5 * then, "1000/400 with no band travels {now:.3} %/s, and the measure let it pass");
}

// ---- what the board tells a person its knobs do: the same regulator, on the fan it knows ----

#[test]
fn the_answer_to_a_jump_on_the_board_s_numbers_is_back_within_fifteen_seconds() {
    // what the fan card will say under its gains: 42 Pa held at 17 %, and a jump of 20 Pa in the tract
    let mut seconds = 0;
    let said = regulator::answer_to_a_jump(&board(), 42.0, 17.0, 20.0, &mut |_, _| seconds += 1);
    println!("the board's numbers: back in {:?} s, dips {:.1} Pa, {seconds} seconds drawn", said.back_s, said.dip_pa);
    assert!(said.back_s.is_some_and(|s| s <= BACK_S), "back in {:?} s on the board's own numbers", said.back_s);
    assert_eq!(seconds, regulator::ANSWER_S, "the curve of the answer is not {} seconds long", regulator::ANSWER_S);
}

#[test]
fn the_answer_to_a_jump_on_the_numbers_of_26_09_says_they_were_slow() {
    // the pair: the same question on the numbers the burner ran on 26.09 has to come out slow, or the words under the
    // knobs would say nothing a person can act on
    let said = regulator::answer_to_a_jump(&of_26_09(), 42.0, 17.0, 20.0, &mut |_, _| {});
    println!("the numbers of 26.09: back in {:?} s", said.back_s);
    assert!(said.back_s.is_none_or(|s| s > 30), "the numbers of 26.09 came back in {:?} s", said.back_s);
}

#[test]
fn an_answer_is_not_made_up_for_a_fan_that_holds_nothing() {
    // no pressure or no angle is no point on a curve: nothing is simulated and nothing is said
    for (pa, angle) in [(0.0, 17.0), (42.0, 0.0), (f32::NAN, 17.0), (-3.0, 17.0)] {
        let mut drawn = 0;
        let said = regulator::answer_to_a_jump(&board(), pa, angle, 20.0, &mut |_, _| drawn += 1);
        assert_eq!((said.back_s, drawn), (None, 0), "an answer was made up for {pa} Pa at {angle} %");
    }
}
