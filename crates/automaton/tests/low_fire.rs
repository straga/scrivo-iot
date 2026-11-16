//! A low fire is not a dead fire: what the burner takes for "no fuel" and for "the fire has
//! caught", judged on what a person sees of a burner at 3-6 kW - a flue that stands where it stands, and a flame sensor that shakes by
//! 200 mV around its threshold (live burn, 27.09).
//!
//! **THE FLUE ACCUSES THE FUEL BY WHAT IT DOES, NOT BY WHAT IT DOES NOT DO.** A fire at 3 or 6 kW holds its flue at 93-105 °C and does not climb: "not
//! climbing" put out a live fire on the 300th second. What a feed that stopped looks like is a flue that is cold, or one that falls.
//! **A FLAME HAS CAUGHT WHEN IT HOLDS**, not when one reading is over the threshold: the first light from the embers on 27.09 was counted at
//! 2791 mV against 2800, and the second after it was dark again.
//!
//! **THE ONLY PART OF THIS FILE THAT NAMES A FLUE NUMBER OF THE TUNING IS `fire_numbers`**: the rows judge what the burner does, and the
//! numbers they judge it by are the card's - 70 °C for a cold flue, a fall of 10 °C in 10 minutes, a flame held 5 s (the numbers of 27.09).

mod common;

use automaton::{Doing, Feed, Lighting, State, Step, Told, Why};
use common::{Burning, Hold, Seen, Silence, Tuning};
use flame::Flame;

/// The numbers of the fire these rows burn by: the card's three of the flue and the flame, the rest as lighting_again.rs has them.
fn tuning() -> Tuning {
    Tuning {
        purge_before_s: 30.0,
        purge_after_s: 20.0,
        igniter_s: 180.0,
        wait_for_flame_s: 90.0,
        tries: 3,
        dose_after_first: 0.1,
        rise_s: 60.0,
        flame_lost_s: 20.0,
        smoulder_s: 300.0,
        burn_off_s: 600.0,
        burn_out_min_s: 60.0,
        pump_on_c: 35.0,
        flue_holds_c: 100.0,
        fire_numbers: fire_numbers(),
        want_c: 60.0,
        hysteresis_c: 4.0,
        hold: Hold::Boiler,
        power: Step { pa: 42.0, feed: Feed { on_s: 2.0, off_s: 42.0 } },
        pump_off_below_c: 4.0,
        too_hot_c: 88.0,
        alarm_c: 95.0,
        sure_s: 10.0,
        relight_s: [20.0, 10.0, 10.0],
        relight_hot_c: 45.0,
        spin_up_s: 10.0,
        low_pressure_s: 20.0,
        kindling: Lighting { pa: 45.0, dose_s: 32.0 },
        step: Step { pa: 42.0, feed: Feed { on_s: 2.0, off_s: 42.0 } },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

fn sound() -> common::Sound {
    tuning().sound().expect("the numbers of 27.09 are sound")
}

/// A burner by TUNING, and not by the card's: a row about one number changes that number and nothing else.
fn sound_by(t: Tuning) -> common::Sound {
    t.sound().expect("the numbers of the row are sound")
}

/// The fire's seconds as the card holds them by default, in the form the rows' record takes them.
fn fire_numbers() -> common::FireNumbers {
    common::FireNumbers::CARD
}

/// A steady flame, the flue at FLUE, the fan on the step's draught: a fire at work.
fn lit(flue: f32) -> Seen {
    Seen { flame: Flame::Steady, boiler_c: Some(40.0), return_c: None, flue_c: Some(flue), draught_pa: Some(42.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// A flame that the sensor reads weak: over the threshold of a flame, under that of a steady one.
fn weak(flue: f32) -> Seen {
    Seen { flame: Flame::Weak, ..lit(flue) }
}

/// The same fire with the sensor dark.
fn out(flue: f32) -> Seen {
    Seen { flame: Flame::Out, ..lit(flue) }
}

/// A cold pot and the fan blowing the lighting's draught.
fn dark(draught_pa: f32) -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(30.0), return_c: None, flue_c: Some(20.0), draught_pa: Some(draught_pa), draught_low: false, gate: 30, silence: Silence::HEARD }
}

fn second_by(burning: &mut Burning, seen: Seen, t: &common::Sound) -> Doing {
    burning.step(Told::Nothing, seen, 1.0, t)
}

fn second(burning: &mut Burning, seen: Seen) -> Doing {
    second_by(burning, seen, &sound())
}

/// A burner started and purged: the first second of its first attempt at lighting, with the plug.
fn lighting_by(t: &common::Sound) -> Burning {
    let mut burning = Burning::new();
    burning.step(Told::Start, Seen { draught_pa: Some(0.0), draught_low: false, gate: 0, ..dark(0.0) }, 1.0, t);
    for _ in 0..30 {
        second_by(&mut burning, dark(200.0), t);
    }
    assert_eq!(burning.state(), State::Kindling(1), "the scaffolding did not reach the first attempt");
    burning
}

/// A burner lit with the plug and carried into work over a flue at FLUE, a steady flame all the way.
fn at_work_by(flue: f32, t: &common::Sound) -> Burning {
    let mut burning = lighting_by(t);
    for _ in 0..110 {
        second_by(&mut burning, lit(flue), t);
    }
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach work");
    burning
}

fn at_work(flue: f32) -> Burning {
    at_work_by(flue, &sound())
}

/// The burner at work over FLUE, its flame dark for 21 s over a pot hot enough to be lit from the embers: the first attempt from them.
fn from_embers_by(flue: f32, t: &common::Sound) -> Burning {
    let mut burning = at_work_by(flue, t);
    for _ in 0..21 {
        second_by(&mut burning, out(flue), t);
    }
    assert_eq!(burning.state(), State::Kindling(1), "the scaffolding did not reach the first attempt from the embers");
    burning
}

/// The second, counted from 1, in which the burner first stood in BurningOff(NoFuel) while the flue was FLUE(t); None if it never did in SECONDS.
fn put_out_at(burning: &mut Burning, seconds: usize, flue: impl Fn(usize) -> f32, t: &common::Sound) -> Option<usize> {
    for s in 1..=seconds {
        if second_by(burning, lit(flue(s)), t).state == State::BurningOff(Some(Why::NoFuel)) {
            return Some(s);
        }
    }
    None
}

/// The same as `put_out_at`, the flame of each second FLAME(s) and not a steady one: the second, counted from 1, of the verdict `NoFuel`; None if there was none.
fn put_out_by_light(burning: &mut Burning, seconds: usize, flue: impl Fn(usize) -> f32, flame: impl Fn(usize) -> Flame, t: &common::Sound) -> Option<usize> {
    for s in 1..=seconds {
        let seen = Seen { flame: flame(s), ..lit(flue(s)) };
        if second_by(burning, seen, t).state == State::BurningOff(Some(Why::NoFuel)) {
            return Some(s);
        }
    }
    None
}

/// The same, the flame weak every second: the light of a fire that is dimming and not yet out.
fn put_out_dim(burning: &mut Burning, seconds: usize, flue: impl Fn(usize) -> f32, t: &common::Sound) -> Option<usize> {
    put_out_by_light(burning, seconds, flue, |_| Flame::Weak, t)
}

// ---------------------------------------------------------------- 1. the fire is put out for a cold or a falling flue

#[test]
fn a_flue_that_stands_still_on_a_low_fire_does_not_put_the_fire_out() {
    // 6 and 3 kW on 27.09: 93-105 °C, not climbing, for the hour
    for flue in [95.0, 100.0, 75.0] {
        let t = sound();
        let mut burning = at_work_by(flue, &t);
        let at = put_out_at(&mut burning, 40 * 60, |_| flue, &t);
        assert_eq!(at, None, "a fire over a flue standing at {flue} °C was put out for no fuel at second {at:?}");
        assert_eq!(burning.state(), State::Working);
    }
}

#[test]
fn a_flue_under_the_cold_mark_for_the_whole_window_puts_the_fire_out() {
    // the pair of the standing flue: 65 is under 70 °C, and nothing in it ever warms
    let t = sound();
    let mut burning = at_work_by(65.0, &t);
    let at = put_out_dim(&mut burning, 40 * 60, |_| 65.0, &t).expect("a flue that stood at 65 °C for forty minutes was never accused");
    assert!(at >= 60, "a cold flue was a verdict of {at} s");
    assert_eq!(burning.state(), State::BurningOff(Some(Why::NoFuel)));
}

#[test]
fn a_flue_that_warms_from_cold_after_work_began_is_not_accused_on_the_way_up() {
    // 27.09 15:43:49: work began over a flue of 23 °C, and it crossed 70 °C 291 s later. A cold pot warming is a feed that works.
    let t = sound();
    let mut burning = at_work_by(20.0, &t);
    let at = put_out_dim(&mut burning, 40 * 60, |s| (23.0 + s as f32 * 0.17).min(95.0), &t);
    assert_eq!(at, None, "a cold start that warmed 23 -> 70 in five minutes was put out at second {at:?}");
}

#[test]
fn a_flue_that_falls_a_degree_a_minute_puts_the_fire_out_within_twelve_minutes() {
    // 26.09 17:32: 77 °C and falling a degree a minute, the pellets gone
    let t = sound();
    let mut burning = at_work_by(77.0, &t);
    let at = put_out_dim(&mut burning, 12 * 60, |s| 77.0 - s as f32 / 60.0, &t).expect("a flue falling 1 °C a minute was not accused in twelve minutes");
    assert!(at >= 9 * 60, "ten degrees in ten minutes was a verdict at {at} s, before nine minutes");
}

#[test]
fn a_flue_that_fell_eight_degrees_and_stands_is_not_accused() {
    // the pair of the fall, under its number: 80 -> 72 over eight minutes, then standing
    let t = sound();
    let mut burning = at_work_by(80.0, &t);
    let at = put_out_dim(&mut burning, 40 * 60, |s| (80.0 - s as f32 / 60.0).max(72.0), &t);
    assert_eq!(at, None, "a flue that fell 8 °C and stood was put out at second {at:?}");
}

#[test]
fn a_flue_that_falls_ten_degrees_quickly_is_accused_at_once() {
    // the fall is of the window's own highest: ten degrees in a minute is over the number in that minute
    let t = sound();
    let mut burning = at_work_by(100.0, &t);
    let at = put_out_dim(&mut burning, 5 * 60, |s| (100.0 - s as f32 / 3.0).max(80.0), &t).expect("a flue that fell 20 °C in a minute was not accused");
    assert!(at <= 60 * 3, "the fall was a verdict at {at} s");
}

#[test]
fn a_cold_mark_is_the_cards_and_not_the_codes() {
    // the pair of 65 °C: with the mark at 60 the same flue is warm; with it at 80 a flue of 75 is cold
    let mut raised = tuning();
    raised.fire_numbers.flue_cold_c = 80.0;
    let t = sound_by(raised);
    let mut burning = at_work_by(75.0, &t);
    assert!(put_out_dim(&mut burning, 40 * 60, |_| 75.0, &t).is_some(), "a flue of 75 °C was warm with the mark at 80 °C");
    let mut lowered = tuning();
    lowered.fire_numbers.flue_cold_c = 60.0;
    let t = sound_by(lowered);
    let mut burning = at_work_by(65.0, &t);
    assert_eq!(put_out_dim(&mut burning, 40 * 60, |_| 65.0, &t), None, "a flue of 65 °C was cold with the mark at 60 °C");
}

#[test]
fn the_fall_and_its_window_are_the_cards_and_not_the_codes() {
    // half a degree a minute: ten degrees take twenty minutes. A window of ten minutes never sees them; one of twenty-five does.
    let slow = |s: usize| 100.0 - s as f32 / 120.0;
    let t = sound();
    let mut burning = at_work_by(100.0, &t);
    assert_eq!(put_out_dim(&mut burning, 40 * 60, slow, &t), None, "a fall of 0.5 °C a minute was accused over a window of ten minutes");
    let mut longer = tuning();
    longer.fire_numbers.flue_fall_s = 1500.0;
    let t = sound_by(longer);
    let mut burning = at_work_by(100.0, &t);
    let at = put_out_dim(&mut burning, 40 * 60, slow, &t).expect("a fall of 10 °C over twenty minutes was not accused over a window of twenty-five");
    assert!(at >= 17 * 60 && at <= 24 * 60, "ten degrees fell in 1200 s and the verdict came at {at} s");
    let mut bigger = tuning();
    bigger.fire_numbers.flue_fall_c = 20.0;
    let t = sound_by(bigger);
    let mut burning = at_work_by(77.0, &t);
    assert_eq!(put_out_dim(&mut burning, 12 * 60, |s| 77.0 - s as f32 / 60.0, &t), None, "a fall of 12 °C was accused with the number at 20");
}

#[test]
fn a_flue_nobody_read_accuses_nobody() {
    // no number is not a cold flue: the sensor is off, and the burner is not accused on a reading nobody took
    let t = sound();
    let mut burning = at_work_by(65.0, &t);
    for _ in 0..40 * 60 {
        let seen = Seen { flue_c: None, ..weak(65.0) };
        assert_ne!(second_by(&mut burning, seen, &t).state, State::BurningOff(Some(Why::NoFuel)), "a flue that said nothing was accused");
    }
}

#[test]
fn a_flue_nobody_read_for_a_while_forgets_the_highest_it_had() {
    // GUARD ROW, green from the start: the window is dropped whole when the reading goes (`fuel_stopped`), so the 100 °C of before is not what 85 °C is
    // measured against once the thermocouple speaks again. Kept highest, 85 would be 15 under it and accused at once. The flame is weak all the while:
    // a steady second would drop the window by itself and the row would prove nothing about the silence.
    let t = sound();
    let dim_at_100 = || {
        let mut burning = at_work_by(100.0, &t);
        for _ in 0..5 {
            second_by(&mut burning, weak(100.0), &t);
        }
        burning
    };
    let mut burning = dim_at_100();
    for _ in 0..5 {
        let seen = Seen { flue_c: None, ..weak(100.0) };
        assert_eq!(second_by(&mut burning, seen, &t).state, State::Working);
    }
    assert_eq!(put_out_dim(&mut burning, 40 * 60, |_| 85.0, &t), None, "a flue that came back at 85 °C was measured against the 100 °C before the silence");
    // the pair: with no silence the same fall is accused
    let mut burning = dim_at_100();
    assert!(put_out_dim(&mut burning, 40 * 60, |_| 85.0, &t).is_some(), "a fall of 15 °C was not accused without a silence in it");
}

#[test]
fn a_fire_lit_again_from_the_embers_starts_the_window_of_its_flue_anew() {
    // GUARD ROW, green from the start: `enter` drops the window with the state, so work after a light from the embers is measured from its own first second,
    // not against the 100 °C of the work before the flame was lost. Kept, 85 °C would be a fall of 15.
    let t = sound();
    let mut burning = from_embers_by(100.0, &t);
    // the flame is back over a flue that has settled at 85 °C: held five seconds, it is work again
    for _ in 0..5 {
        second_by(&mut burning, weak(85.0), &t);
    }
    assert_eq!(burning.state(), State::Working, "the scaffolding did not get back to work");
    assert_eq!(put_out_dim(&mut burning, 40 * 60, |_| 85.0, &t), None, "the work after the embers was measured against the work before them");
}

// ---------------------------------------------------------------- 2. the flame has caught when it holds

#[test]
fn a_flame_of_one_second_over_the_embers_does_not_light_the_fire() {
    // 27.09 16:09:15, 16:25:38: 2791 and 2777 mV, a second over the threshold, dark again at the next
    let t = sound();
    let mut burning = from_embers_by(99.0, &t);
    assert_eq!(second_by(&mut burning, weak(99.0), &t).state, State::Kindling(1), "one reading of a flame over the embers was taken for a fire");
    assert_eq!(second_by(&mut burning, out(99.0), &t).state, State::Kindling(1), "the attempt was lost to a second of flame");
    for _ in 0..3 {
        assert_eq!(second_by(&mut burning, out(99.0), &t).state, State::Kindling(1));
    }
}

#[test]
fn a_flame_that_holds_five_seconds_over_the_embers_is_work_again() {
    // the pair: the fifth second in a row, and not the fourth
    let t = sound();
    let mut burning = from_embers_by(99.0, &t);
    for s in 1..=4 {
        assert_eq!(second_by(&mut burning, weak(99.0), &t).state, State::Kindling(1), "the fire was taken for caught at second {s} of 5");
    }
    assert_eq!(second_by(&mut burning, weak(99.0), &t).state, State::Working, "five seconds of flame over the embers were not work again");
}

#[test]
fn a_flame_of_one_second_over_the_plug_does_not_light_the_fire_either() {
    // the cold pot: the plug's own attempt, the same hold
    let t = sound();
    let mut burning = lighting_by(&t);
    for _ in 0..40 {
        second_by(&mut burning, dark(45.0), &t);
    }
    assert_eq!(second_by(&mut burning, weak(20.0), &t).state, State::Kindling(1), "one reading of a flame over the plug was taken for a fire");
    assert_eq!(second_by(&mut burning, dark(45.0), &t).state, State::Kindling(1));
    for s in 1..=4 {
        assert_eq!(second_by(&mut burning, weak(20.0), &t).state, State::Kindling(1), "second {s} of 5");
    }
    assert_eq!(second_by(&mut burning, weak(20.0), &t).state, State::Rising, "five seconds of flame over the plug did not begin the rising");
}

#[test]
fn a_flame_that_blinks_inside_the_hold_starts_the_count_again() {
    // four seconds of flame, one of dark, five of flame: the fire has caught at the fifth of the second run
    let t = sound();
    let mut burning = from_embers_by(99.0, &t);
    for _ in 0..4 {
        second_by(&mut burning, weak(99.0), &t);
    }
    assert_eq!(second_by(&mut burning, out(99.0), &t).state, State::Kindling(1), "a blink ended the attempt");
    for s in 1..=4 {
        assert_eq!(second_by(&mut burning, weak(99.0), &t).state, State::Kindling(1), "the count did not start again: caught at second {s} of the second run");
    }
    assert_eq!(second_by(&mut burning, weak(99.0), &t).state, State::Working, "the fifth second of the second run was not work");
}

#[test]
fn the_hold_is_the_cards_and_not_the_codes() {
    // the pair of five: with two seconds the second one is work again, and with ten the fifth is not
    for (hold, at) in [(1.0, 1), (2.0, 2), (10.0, 10)] {
        let mut numbers = tuning();
        numbers.fire_numbers.lit_s = hold;
        let t = sound_by(numbers);
        let mut burning = from_embers_by(99.0, &t);
        for s in 1..at {
            assert_eq!(second_by(&mut burning, weak(99.0), &t).state, State::Kindling(1), "a hold of {hold} s was over at second {s}");
        }
        assert_eq!(second_by(&mut burning, weak(99.0), &t).state, State::Working, "a hold of {hold} s was not over at second {at}");
    }
}

#[test]
fn the_time_of_an_attempt_runs_on_through_the_flashes_of_a_flame() {
    // a flash is not a pause: the attempt from the embers ends at its second whether the sensor flickered or not (20 s of dose, 90 s of wait)
    let t = sound();
    let ended = |flash: bool| {
        let mut burning = from_embers_by(99.0, &t);
        for s in 1..=130usize {
            let seen = if flash && s % 10 == 3 { weak(99.0) } else { out(99.0) };
            if second_by(&mut burning, seen, &t).state == State::Kindling(2) {
                return Some(s);
            }
        }
        None
    };
    let plain = ended(false).expect("the first attempt from the embers never ended");
    assert_eq!(ended(true), Some(plain), "a flash of the flame moved the end of the attempt");
}

// ---------------------------------------------------------------- 3. the light decides that the fire is over, the flue only helps

#[test]
fn a_cold_start_under_a_steady_flame_is_not_put_out_for_no_fuel() {
    // 04.10 10:32-10:42 (26.10.04.1): the flame steady the whole stretch with a weak second now and then, the flue 20 -> 64 °C in ten minutes; the burner was put out
    // at the 605th second of work while the fire burned
    let t = sound();
    let mut burning = at_work_by(20.0, &t);
    let flue = |s: usize| (20.0 + s as f32 * 44.0 / 600.0).min(64.0);
    let flame = |s: usize| if s % 25 == 0 { Flame::Weak } else { Flame::Steady };
    let at = put_out_by_light(&mut burning, 40 * 60, flue, flame, &t);
    assert_eq!(at, None, "a fire with a steady flame over a flue warming 20 -> 64 °C was put out for no fuel at second {at:?}");
    assert_eq!(burning.state(), State::Working);
}

#[test]
fn a_low_fire_on_a_flue_of_93_to_105_under_a_steady_flame_is_not_put_out() {
    // 27.09 15:44-16:24: the flue stands at 95-103 °C and shakes inside them - the record's own largest fall in a window is 8.0 °C - and the
    // flame shakes around its steady mark
    let t = sound();
    let mut burning = at_work_by(99.0, &t);
    let flue = |s: usize| 99.0 + 4.0 * ((s as f32) / 37.0).sin();
    let flame = |s: usize| if s % 7 == 0 { Flame::Weak } else { Flame::Steady };
    let at = put_out_by_light(&mut burning, 40 * 60, flue, flame, &t);
    assert_eq!(at, None, "a fire with a steady flame over a flue of 93-105 °C was put out at second {at:?}");
}

#[test]
fn a_step_down_that_drops_the_flue_fifteen_degrees_under_a_steady_flame_is_not_put_out() {
    // the power steps 6 -> 5 -> 3 kW at the setpoint, and the flue honestly falls over ten degrees in the window; the flame stays steady all the way
    let t = sound();
    let mut burning = at_work_by(100.0, &t);
    let flue = |s: usize| (100.0 - s as f32 * 15.0 / 300.0).max(85.0);
    let at = put_out_by_light(&mut burning, 40 * 60, flue, |_| Flame::Steady, &t);
    assert_eq!(at, None, "a step down that cooled the flue 15 °C under a steady flame put the fire out at second {at:?}");
}

#[test]
fn a_fire_that_dims_while_the_flue_falls_is_put_out_for_no_fuel() {
    // the pair: the pellets gone, the flue falling a degree a minute (the row of 6.6), and the flame weak all the while - the flue and the light agree
    let t = sound();
    let mut burning = at_work_by(77.0, &t);
    let at = put_out_dim(&mut burning, 12 * 60, |s| 77.0 - s as f32 / 60.0, &t).expect("a dim flame over a flue falling a degree a minute was not put out in twelve minutes");
    assert!(at >= 9 * 60, "ten degrees in ten minutes was a verdict at {at} s, before nine minutes");
}

#[test]
fn a_weak_flame_the_whole_window_over_a_cold_flue_is_put_out_for_no_fuel() {
    let t = sound();
    let mut burning = at_work_by(65.0, &t);
    let at = put_out_dim(&mut burning, 40 * 60, |_| 65.0, &t).expect("a weak flame over a flue standing at 65 °C for forty minutes was never put out");
    // the scaffolding has spent under fifty seconds of work before the first second counted here
    assert!(at >= 540, "a cold flue under a weak flame was a verdict of {at} s, inside the window of 600");
    assert_eq!(burning.state(), State::BurningOff(Some(Why::NoFuel)));
}

#[test]
fn a_flame_that_is_out_altogether_is_still_the_lights_verdict() {
    // the flame goes out over a flue that falls: the burner is lit again after the wait (`flame_went`), the flue has not said a word
    let t = sound();
    let mut burning = at_work_by(77.0, &t);
    let mut state = State::Working;
    for s in 1..=40usize {
        state = second_by(&mut burning, Seen { flame: Flame::Out, ..lit(77.0 - s as f32 / 6.0) }, &t).state;
        assert_ne!(state, State::BurningOff(Some(Why::NoFuel)), "a flame gone was put to the flue's account at second {s}");
    }
    assert_eq!(state, State::Kindling(1), "a flame gone for forty seconds was not lit again");
}

#[test]
fn one_steady_second_starts_the_window_of_a_dim_flame_anew() {
    // the light is the judge, so a fire that shows a steady second has shown it is burning: the flue is asked again from that second, a whole window of it
    let t = sound();
    let mut burning = at_work_by(65.0, &t);
    let flame = |s: usize| if s == 500 { Flame::Steady } else { Flame::Weak };
    let at = put_out_by_light(&mut burning, 40 * 60, |_| 65.0, flame, &t).expect("a dim flame over a cold flue was never put out after its steady second");
    assert!(at >= 500 + 600, "the window of the flue was not asked anew after a steady second: the verdict came at {at} s");
}

#[test]
fn a_dim_flame_over_a_cold_flue_that_rises_by_the_fall_is_not_put_out() {
    // a low fire with a dim flame whose flue warms: from its lowest in the window it has risen by `flue_fall_c` (10 °C), so the fire is alive and the cold flue accuses nobody.
    // 50 -> 68 °C at 0.02 °C a second: under the cold mark of 70 all the way, and 12 °C over its lowest in any window of 600 s
    let t = sound();
    let mut burning = at_work_by(50.0, &t);
    let at = put_out_dim(&mut burning, 15 * 60, |s| 50.0 + s as f32 * 0.02, &t);
    assert_eq!(at, None, "a flue rising 12 °C a window under a dim flame was put out for no fuel at second {at:?}");
}

#[test]
fn a_dim_flame_over_a_cold_flue_that_rises_less_than_the_fall_is_put_out() {
    // the pair: the same cold flue rising 6 °C over the window - less than the fall's 10 - is a flue that does not grow
    let t = sound();
    let mut burning = at_work_by(50.0, &t);
    let at = put_out_dim(&mut burning, 40 * 60, |s| (50.0 + s as f32 * 0.01).min(66.0), &t).expect("a cold flue rising 6 °C a window under a dim flame was never put out");
    assert!(at >= 540, "the verdict came at {at} s, inside the window");
}

#[test]
fn the_rise_that_saves_a_cold_flue_is_the_cards_fall_and_not_the_codes() {
    // the same flue rising 6 °C over the window: put out with the fall at 10, and not with it at 5
    let mut lower = tuning();
    lower.fire_numbers.flue_fall_c = 5.0;
    let t = sound_by(lower);
    let mut burning = at_work_by(50.0, &t);
    let at = put_out_dim(&mut burning, 15 * 60, |s| (50.0 + s as f32 * 0.01).min(66.0), &t);
    assert_eq!(at, None, "a flue that rose 6 °C a window was put out with the number at 5");
}

#[test]
fn a_flue_that_rose_and_stands_is_cold_again_once_the_rise_leaves_the_window() {
    // the rise is of the window and not of the work: 50 -> 62 °C at 0.02 °C a second, then standing at 62. The window holds 10 °C of the rise until it starts at the
    // 100th second (52 °C -> 62), so the cold flue is accused from there on: the 700th second, and by the 760th - the lowest of the window is kept by parts, up to a twentieth late
    let t = sound();
    let mut burning = at_work_by(50.0, &t);
    let at = put_out_dim(&mut burning, 40 * 60, |s| (50.0 + s as f32 * 0.02).min(62.0), &t).expect("a flue that rose and stood at 62 °C was never put out");
    assert!((700..=760).contains(&at), "the rise was held for its window, and the verdict was expected between 700 and 760 s, not at {at} s");
}
