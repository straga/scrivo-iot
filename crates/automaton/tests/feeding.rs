//! What the automaton tells the auger's relay once a second. Expected from what a person sets: a step of 0.8 s of dose and 5.0 s of pause is told as that rhythm, a lighting dose of 0.8 or 1.5 s as one portion of that
//! length that is told for as long as it is under way, and a banned feed, a rising, a stop and a burning off tell nothing but a stop. The relay's owner times the portions; here only what is told is looked at.

mod common;

use automaton::{Asked, Feed, Feeding, Lighting, State, Step, Told};
use common::{Burning, Silence, Tuning};
use flame::Flame;

fn tuning(dose_s: f32, step: Feed) -> Tuning {
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
        power: Step { pa: 51.0, feed: step },
        pump_off_below_c: 4.0,
        too_hot_c: 88.0,
        alarm_c: 95.0,
        sure_s: 10.0,
        relight_s: [20.0, 10.0, 10.0],
        relight_hot_c: 45.0,
        spin_up_s: 10.0,
        low_pressure_s: 20.0,
        kindling: Lighting { pa: 45.0, dose_s },
        step: Step { pa: 51.0, feed: step },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

fn cold() -> common::Seen {
    common::Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0), draught_pa: Some(51.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

fn lit() -> common::Seen {
    common::Seen { flame: Flame::Steady, boiler_c: Some(40.0), return_c: None, flue_c: Some(150.0), draught_pa: Some(50.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

const STEP: Feed = Feed { on_s: 0.8, off_s: 5.0 };

/// A burner started and brought to the first second of its first dose, over a cold pot.
fn at_the_dose(t: &common::Sound) -> Burning {
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, t);
    while !matches!(burning.state(), State::Kindling(_)) {
        burning.step(Told::Nothing, cold(), 1.0, t);
    }
    burning
}

/// What the lighting's seconds told, from the first second of the first attempt (the second it is entered in), N of them.
fn dose_told(dose_s: f32, n: usize) -> Vec<Feeding> {
    let t = tuning(dose_s, STEP).sound().expect("sound numbers");
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, &t);
    let mut told = Vec::new();
    let mut began = false;
    while told.len() < n {
        let did = burning.step(Told::Nothing, cold(), 1.0, &t);
        began |= matches!(did.state, State::Kindling(_));
        if began {
            told.push(did.feeding);
        }
    }
    told
}

/// A burner at work on STEP, N seconds of it.
fn work_told(step: Feed, n: usize) -> Vec<Feeding> {
    let t = tuning(32.0, step).sound().expect("sound numbers");
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, &t);
    for _ in 0..30 {
        burning.step(Told::Nothing, cold(), 1.0, &t);
    }
    let mut told = Vec::new();
    for _ in 0..400 {
        let did = burning.step(Told::Nothing, lit(), 1.0, &t);
        if did.state == State::Working {
            told.push(did.feeding);
            if told.len() == n {
                return told;
            }
        }
    }
    panic!("the burner never worked for {n} s: {told:?}");
}

fn into_of(f: &Feeding) -> f32 {
    match f {
        Feeding::Rhythm { into_s, .. } => *into_s,
        other => panic!("not a rhythm: {other:?}"),
    }
}

#[test]
fn the_work_plans_its_cycle_from_where_it_stands_and_the_cycle_begins_with_the_first_second_of_the_work() {
    let told = work_told(STEP, 20);
    assert_eq!(told[0], Feeding::Rhythm { into_s: 0.0, on_s: 0.8, off_s: 5.0 }, "the first plan of the work is the beginning of a cycle: a portion at once");
    for (n, plan) in told.iter().enumerate() {
        let Feeding::Rhythm { on_s, off_s, .. } = plan else { panic!("{plan:?}") };
        assert_eq!((*on_s, *off_s), (0.8, 5.0), "second {n}");
        let want = (n as f32) % 5.8;
        assert!((into_of(plan) - want).abs() < 0.001, "second {n}: the cycle stands {} s into itself where the count says {want} s", into_of(plan));
    }
}

#[test]
fn a_whole_step_is_planned_as_it_was() {
    let told = work_told(Feed { on_s: 2.0, off_s: 15.0 }, 20);
    for (n, plan) in told.iter().enumerate() {
        assert_eq!(*plan, Feeding::Rhythm { into_s: (n as f32) % 17.0, on_s: 2.0, off_s: 15.0 }, "second {n}");
    }
}

#[test]
fn a_dose_of_eight_tenths_is_planned_for_the_second_it_is_entered_in_and_then_stopped() {
    let told = dose_told(0.8, 3);
    assert_eq!(told[..2], [Feeding::Dose { left_s: 0.8 }, Feeding::Stop], "{told:?}");
}

#[test]
fn a_dose_is_planned_by_what_is_left_of_it_by_the_automatons_own_count() {
    let told = dose_told(1.5, 4);
    assert_eq!(told[..3], [Feeding::Dose { left_s: 1.5 }, Feeding::Dose { left_s: 0.5 }, Feeding::Stop], "{told:?}");
    let told = dose_told(3.0, 5);
    assert_eq!(told[..4], [Feeding::Dose { left_s: 3.0 }, Feeding::Dose { left_s: 2.0 }, Feeding::Dose { left_s: 1.0 }, Feeding::Stop], "{told:?}");
}

#[test]
fn a_banned_feed_is_planned_as_a_stop_and_the_dose_goes_on_by_the_clock_when_it_is_allowed_again() {
    let t = tuning(32.0, STEP).sound().expect("sound numbers");
    let mut burning = at_the_dose(&t);
    assert_eq!(burning.step(Told::Nothing, cold(), 1.0, &t).feeding, Feeding::Dose { left_s: 31.0 });
    burning.allow_feed(false);
    let banned = burning.step(Told::Nothing, cold(), 0.0, &t);
    assert_eq!(banned.feeding, Feeding::Stop, "a ban in the middle of a dose");
    assert_eq!(burning.step(Told::Nothing, cold(), 1.0, &t).feeding, Feeding::Stop);
    burning.allow_feed(true);
    // the count went on while the feed was banned: two seconds of the dose are gone, and 29 are left, not 32 again
    assert_eq!(burning.step(Told::Nothing, cold(), 1.0, &t).feeding, Feeding::Dose { left_s: 29.0 }, "allowed again, the dose is planned by what the count has left");
}

#[test]
fn a_stop_plans_nothing_but_a_stop() {
    let t = tuning(0.8, STEP).sound().expect("sound numbers");
    let mut burning = Burning::new();
    for _ in 0..5 {
        assert_eq!(burning.step(Told::Nothing, cold(), 1.0, &t).feeding, Feeding::Stop);
    }
    let _ = Asked::Stop;
}

#[test]
fn a_number_changed_on_a_working_burner_is_taken_at_the_boundary_of_the_cycle_and_not_inside_it() {
    let slow = tuning(32.0, STEP).sound().expect("sound numbers");
    let quick = tuning(32.0, Feed { on_s: 1.5, off_s: 5.0 }).sound().expect("sound numbers");
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, &slow);
    for _ in 0..30 {
        burning.step(Told::Nothing, cold(), 1.0, &slow);
    }
    let mut told = Vec::new();
    for _ in 0..400 {
        let did = burning.step(Told::Nothing, lit(), 1.0, &slow);
        if did.state == State::Working {
            told.push((did.feeding, did.auger));
            break;
        }
    }
    // the number is changed in the first cycle: the plans keep the old numbers to the end of the cycle (5.8 s) and carry the new ones from its next
    for _ in 0..12 {
        let did = burning.step(Told::Nothing, lit(), 1.0, &quick);
        told.push((did.feeding, did.auger));
    }
    // what the numbers give, step n standing n seconds after the first of the work: the old cycle of 5.8 s runs out at the 6th step, 200 ms before it, and what is left of that step goes into the new cycle of 6.5 s (1.5 s closed)
    for (n, (plan, auger)) in told.iter().enumerate() {
        let (into_ms, on_ms, cycle_ms) = if n < 6 { (n as u64 * 1000, 800, 5800) } else { ((200 + (n as u64 - 6) * 1000) % 6500, 1500, 6500) };
        let Feeding::Rhythm { into_s, on_s, off_s } = plan else { panic!("step {n}: {plan:?}") };
        assert_eq!(((on_s * 1000.0).round() as u64, ((on_s + off_s) * 1000.0).round() as u64), (on_ms, cycle_ms), "step {n}: the numbers of the cycle it stands in: {plan:?}");
        assert_eq!((into_s * 1000.0).round() as u64, into_ms, "step {n}: the position (the cycle that took the new number carries the rest of the step, it does not begin at nought): {plan:?}");
        assert_eq!(*auger, into_ms < on_ms, "step {n}: the auger at {into_ms} ms of a portion of {on_ms} ms");
    }
}

// ---- the position in the cycle is reduced to [0, period) at any length of a step and of the work ----

/// A burner brought to work on STEP, then the seconds of STEPS (each the time that went in one step) at work: what each of them planned and the auger it showed.
fn work_steps(step: Feed, steps: &[f32]) -> Vec<(Feeding, bool)> {
    let t = tuning(32.0, step).sound().expect("sound numbers");
    let mut burning = Burning::new();
    burning.step(Told::Start, cold(), 1.0, &t);
    for _ in 0..30 {
        burning.step(Told::Nothing, cold(), 1.0, &t);
    }
    let mut reached = false;
    for _ in 0..400 {
        if burning.step(Told::Nothing, lit(), 1.0, &t).state == State::Working {
            reached = true;
            break;
        }
    }
    assert!(reached, "the burner never came to work");
    steps
        .iter()
        .map(|secs| {
            let did = burning.step(Told::Nothing, lit(), *secs, &t);
            assert_eq!(did.state, State::Working);
            (did.feeding, did.auger)
        })
        .collect()
}

/// Where a cycle of PERIOD_MS that began with the second the burner came to work in stands after each of the steps of STEPS_MS that followed: the sum of them modulo the period.
fn phase_after(steps_ms: &[u64], period_ms: u64) -> Vec<u64> {
    let mut at = 0u64;
    steps_ms.iter().map(|ms| {
        at += ms;
        at % period_ms
    }).collect()
}

fn ms_of(plan: &Feeding) -> (u64, u64, u64) {
    match plan {
        Feeding::Rhythm { into_s, on_s, off_s } => ((into_s * 1000.0).round() as u64, (on_s * 1000.0).round() as u64, ((on_s + off_s) * 1000.0).round() as u64),
        other => panic!("not a rhythm: {other:?}"),
    }
}

#[test]
fn a_cycle_shorter_than_the_step_stands_where_the_count_puts_it_and_the_auger_and_the_plan_say_the_same_phase() {
    // 0.1 s closed and 0.1 s open, a step of a second: five cycles go in a step
    let step = Feed { on_s: 0.1, off_s: 0.1 };
    let told = work_steps(step, &[1.0; 40]);
    for (n, (plan, auger)) in told.iter().enumerate() {
        let (into, on, period) = ms_of(plan);
        assert!(into < period, "step {n}: the cycle stands {into} ms into itself where it holds {period} ms: {plan:?}");
        assert_eq!(*auger, into < on, "step {n}: the auger shows {auger} where the plan stands {into} ms into a portion of {on} ms");
    }
    // and a usual cycle is what it was
    let told = work_steps(Feed { on_s: 0.8, off_s: 5.0 }, &[1.0; 40]);
    let want = phase_after(&[1000; 40], 5800);
    for (n, (plan, auger)) in told.iter().enumerate() {
        let (into, on, _) = ms_of(plan);
        assert_eq!(into, want[n], "step {n} of a usual cycle");
        assert_eq!(*auger, into < on);
    }
}

#[test]
fn a_step_of_nine_seconds_leaves_the_cycle_where_the_count_puts_it() {
    // the sum of the waits a turn of the loop can have: a step of 9 s, in the middle of steady ones
    let mut steps = vec![1.0f32; 10];
    steps.push(9.0);
    steps.extend([1.0; 10]);
    let told = work_steps(Feed { on_s: 0.8, off_s: 5.0 }, &steps);
    let steps_ms: Vec<u64> = steps.iter().map(|s| (s * 1000.0).round() as u64).collect();
    let want = phase_after(&steps_ms, 5800);
    for (n, (plan, auger)) in told.iter().enumerate() {
        let (into, on, period) = ms_of(plan);
        assert!(into < period, "step {n}: {plan:?}");
        assert_eq!(into, want[n], "step {n} (the step of 9 s is the 11th)");
        assert_eq!(*auger, into < on, "step {n}");
    }
}

#[test]
fn a_day_of_work_keeps_the_phase_and_overflows_nothing() {
    // a day by the rows' clock, in steps of a second, on a cycle of 0.2 s (the shortest the settings allow) and on a usual one
    for (step, period_ms) in [(Feed { on_s: 0.1, off_s: 0.1 }, 200u64), (Feed { on_s: 0.8, off_s: 5.0 }, 5800)] {
        let told = work_steps(step, &[1.0; 86_400]);
        let want = phase_after(&[1000; 86_400], period_ms);
        for (n, (plan, _)) in told.iter().enumerate() {
            let (into, _, period) = ms_of(plan);
            assert!(into < period, "step {n} of a day on a cycle of {period_ms} ms: {plan:?}");
            if n % 997 == 0 || n + 1 == told.len() {
                assert_eq!(into, want[n], "step {n} of a day on a cycle of {period_ms} ms");
            }
        }
        // what goes to the relay's task as milliseconds does not saturate
        let (into, _, _) = ms_of(&told.last().unwrap().0);
        assert!(into < u64::from(u32::MAX), "{into}");
    }
}
