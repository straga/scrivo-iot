//! What each knob of the regulator does, as numbers, held to what the page worked out of the same settings until the numbers moved here (tools/test_page_knobs.js): a growing miss moves the
//! gate at once, a miss that stays moves it by the second, the step bounds the travel's time, and inside the band each way acts at its share of its own gain.

use regulator::knobs::{consequences, known, Consequences, JUMP_PA, MISS_PA, NEAR_PA};
use regulator::{answer_to_a_jump, Gains, Holding, Tuning};

/// The fan card on the board's defaults.
fn defaults() -> Tuning {
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

/// The fan held 42 Pa at 17 %: a steepness of 5.1 Pa a percent.
const AT: (f32, f32) = (42.0, 17.0);

fn of(t: &Tuning) -> Consequences {
    consequences(t, AT.0, AT.1).expect("a fan held at a point has consequences")
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.005
}

#[test]
fn a_proportional_gain_moves_the_gate_at_once_by_its_share_of_the_steepness() {
    // 0.3 * 10 Pa / 5.1 Pa a percent = 0.59 %
    let c = of(&defaults());
    assert!(near(c.up_kp, 0.59) && near(c.down_kp, 0.59), "{c:?}");
    assert_eq!(MISS_PA, 10.0);
}

#[test]
fn an_integral_gain_moves_the_gate_by_the_second_and_each_way_by_its_own_gain() {
    // 0.4 * 10 / 5.1 = 0.78 % a second
    let c = of(&defaults());
    assert!(near(c.up_ki, 0.78) && near(c.down_ki, 0.78), "{c:?}");
    let apart = Tuning { down: Gains { kp_milli: 300, ki_milli: 100 }, ..defaults() };
    let c = of(&apart);
    assert!(near(c.up_ki, 0.78) && near(c.down_ki, 0.20), "{c:?}");
}

#[test]
fn the_step_says_how_long_the_whole_travel_takes() {
    // (80 - 8) / 5 = 14.4 s
    let c = of(&defaults());
    assert!(c.ramp_s.is_some_and(|s| (s - 14.4).abs() < 0.01), "{c:?}");
    // a step of nothing is no travel time, not an endless one
    assert_eq!(of(&Tuning { step: 0.0, ..defaults() }).ramp_s, None);
}

#[test]
fn inside_the_band_each_way_acts_at_its_share_of_its_own_gain() {
    // 0.25 * 0.4 * 3 / 5.1 = 0.06 % a second, each way
    let c = of(&defaults());
    assert!(near(c.near_up, 0.06) && near(c.near_down, 0.06), "{c:?}");
    // down_ki 100: 0.25 * 0.1 * 3 / 5.1 = 0.015
    let apart = Tuning { down: Gains { kp_milli: 300, ki_milli: 100 }, ..defaults() };
    let c = of(&apart);
    assert!(near(c.near_up, 0.06) && (c.near_down - 0.0147).abs() < 0.001, "{c:?}");
    assert_eq!(NEAR_PA, 3.0);
}

#[test]
fn the_answer_to_a_jump_is_the_regulators_own() {
    let t = defaults();
    let jump = answer_to_a_jump(&t, AT.0, AT.1, JUMP_PA, &mut |_, _| {});
    let c = of(&t);
    assert_eq!((c.back_s, c.dip_pa), (jump.back_s, jump.dip_pa));
    // gains too weak to bring the draught back in the time the answer follows it say so: no second to name
    let slow = Tuning { up: Gains { kp_milli: 0, ki_milli: 1 }, down: Gains { kp_milli: 0, ki_milli: 1 }, ..t };
    assert_eq!(of(&slow).back_s, None);
}

#[test]
fn a_point_that_is_no_point_has_no_consequences() {
    let t = defaults();
    for (pa, angle) in [(0.0, 17.0), (42.0, 0.0), (f32::NAN, 17.0)] {
        assert_eq!(consequences(&t, pa, angle), None, "{pa} Pa at {angle} %");
    }
}

#[test]
fn a_fan_that_has_held_nothing_is_answered_for_the_fan_like_it() {
    assert_eq!(known(None), regulator::A_FAN_LIKE_THIS);
    let mut held = Holding::from(0.0);
    held.was_holding(42.0, 17.0);
    assert_eq!(known(Some(&held)), (42.0, 17.0));
}
