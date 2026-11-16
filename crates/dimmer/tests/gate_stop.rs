//! The gate that stops itself when the fronts stop. On 24.10 at 23:47:00 the pin of `.33` gave no fronts and the gate went on firing 49-52 pulses a second: the timer that the fronts reset reached its period and counted again, with nothing to say
//! where in the wave it was. The mechanism is the board's own (MCPWM: a period of `STOP_AFTER_US` and a callback at the timer's peak that moves both comparators out of reach); what these rows hold is the ARITHMETIC of it, on a model of the timer
//! that is checked against the record before it is trusted: with the period the board had then (20 000 µs) and no stop, the model fires what the record says (60 pulses in the second the fronts stopped in, 49-52 in the seconds after).
//!
//! THE MODEL IS NOT THE BOARD: it is the sync-reset timer, two comparators and a stop, written down. The board's side - the callback running in IRAM, the comparators moved - is proved by the image and by the minute on the burner.

use dimmer::{may_resume, order, Order, LIVE_CROSSINGS, STOP_AFTER_US, STOP_ISR_BUDGET_US};

/// What a timer reset by the fronts does: a pulse AT_US after each front (when the next front does not come first), and - when no front comes within PERIOD_US - another AT_US after each wrap, on its own. With STOP set, the first wrap stops the gate for good.
fn pulses(fronts_us: &[u64], period_us: u64, open_at_us: u64, stop: bool, until_us: u64) -> Vec<u64> {
    let mut out = Vec::new();
    let mut stopped = false;
    for (i, &front) in fronts_us.iter().enumerate() {
        let next = fronts_us.get(i + 1).copied().unwrap_or(until_us);
        let mut wrap = 0;
        loop {
            let from = front + wrap * period_us;
            if from >= next || from >= until_us {
                break;
            }
            if wrap >= 1 && stop {
                stopped = true;
            }
            let at = from + open_at_us;
            if !stopped && at < next && at < until_us {
                out.push(at);
            }
            wrap += 1;
        }
    }
    out
}

fn second(pulses: &[u64], s: u64) -> usize {
    pulses.iter().filter(|p| **p >= s * 1_000_000 && **p < (s + 1) * 1_000_000).count()
}

/// The fronts of the seconds 0..N of the night, as the record counts them: SECONDS are the counts of consecutive seconds from 23:46:57, a front every 10 ms from the start of each second (the record has counts and not the moments inside a second).
fn fronts_of(counts: &[u64]) -> Vec<u64> {
    counts.iter().enumerate().flat_map(|(s, n)| (0..*n).map(move |k| s as u64 * 1_000_000 + k * 10_000)).collect()
}

// 23:46:57 .. 23:47:04 on the record: 101, 99, 102, then 21 (the fronts stop), 0, 1, 0, 0
const NIGHT: [u64; 8] = [101, 99, 102, 21, 0, 1, 0, 0];
// the gate pulses of the record in the same seconds: 102, 99, 101, 60, 49, 52, 49, 50
const NIGHT_PULSES: [usize; 8] = [102, 99, 101, 60, 49, 52, 49, 50];

fn open_at_of_the_night() -> u64 {
    // 21 % of air on the half cycle of the record's mains
    match order(21, 10_000).unwrap() {
        Order::Fire { delay_us, .. } => u64::from(delay_us),
        Order::Off => unreachable!(),
    }
}

#[test]
fn the_model_with_the_period_the_board_had_and_no_stop_fires_what_the_record_says_in_the_seconds_after_the_fronts() {
    let fronts = fronts_of(&NIGHT);
    let fired = pulses(&fronts, 20_000, open_at_of_the_night(), false, NIGHT.len() as u64 * 1_000_000);
    // the second the fronts stopped in, and the seconds after: the record said 60, 49, 52, 49, 50
    for s in 3..8 {
        let (model, record) = (second(&fired, s as u64), NIGHT_PULSES[s]);
        assert!(model.abs_diff(record) <= 3, "second {s}: the model fires {model}, the record says {record}");
    }
}

#[test]
fn with_the_stop_nothing_fires_later_than_the_period_after_the_last_front() {
    let fronts = fronts_of(&NIGHT);
    let last_front = *fronts.iter().filter(|f| **f < 4_000_000).last().unwrap();
    let fired = pulses(&fronts, u64::from(STOP_AFTER_US), open_at_of_the_night(), true, NIGHT.len() as u64 * 1_000_000);
    let latest_before_the_silence = fired.iter().copied().filter(|p| *p < 3_300_000).max().unwrap();
    let late: Vec<u64> = fired.iter().copied().filter(|p| *p > last_front + u64::from(STOP_AFTER_US)).collect();
    assert!(late.is_empty(), "pulses after the last front and the period: {late:?}");
    assert!(latest_before_the_silence <= last_front + open_at_of_the_night(), "no pulse of the gate's own after the last front's one");
}

#[test]
fn one_front_missing_or_two_in_a_row_do_not_stop_the_gate() {
    let open_at = 6_000;
    for missing in [1u64, 2] {
        // a front every 10 ms, and MISSING of them gone at 50 ms
        let fronts: Vec<u64> = (0..30).map(|k| k * 10_000).filter(|t| !(*t >= 50_000 && *t < 50_000 + missing * 10_000)).collect();
        let fired = pulses(&fronts, u64::from(STOP_AFTER_US), open_at, true, 300_000);
        assert!(fired.iter().any(|p| *p > 200_000), "{missing} missing: the gate fires after the hole");
    }
}

#[test]
fn a_hole_of_the_whole_period_stops_it_and_it_stays_stopped_though_stray_fronts_come() {
    let open_at = 6_000;
    let mut fronts: Vec<u64> = (0..5).map(|k| k * 10_000).collect();
    // the pin gives nothing from 40 ms on, then a few stray fronts at no phase
    fronts.extend([153_000, 777_000]);
    let fired = pulses(&fronts, u64::from(STOP_AFTER_US), open_at, true, 1_000_000);
    assert!(fired.iter().all(|p| *p <= 40_000 + open_at), "{fired:?}");
}

#[test]
fn the_callback_has_time_to_run_before_the_comparator_would_fire_after_the_wrap() {
    // the least delay the gate is ever asked to fire after a front: 100 % of air on the shortest half cycle a mains can give
    let least = match order(100, dimmer::SHORTEST_HALF_US).unwrap() {
        Order::Fire { delay_us, .. } => delay_us,
        Order::Off => unreachable!(),
    };
    assert!(least >= STOP_ISR_BUDGET_US, "{least} µs between the wrap and the first comparator is less than the callback is given");
    assert_eq!(STOP_AFTER_US, 4 * dimmer::NOMINAL_HALF_US, "four half cycles of the nominal mains");
}

#[test]
fn a_stopped_gate_goes_back_to_firing_only_after_a_second_with_no_new_stop_and_a_mains_worth_of_fronts() {
    let live = *LIVE_CROSSINGS.start();
    assert!(may_resume(true, 0, 100));
    assert!(may_resume(true, 0, live), "the lowest count a mains gives");
    assert!(!may_resume(true, 0, live - 1), "a second the band does not hold");
    assert!(!may_resume(true, 1, 100), "a stop came in this very second");
    assert!(!may_resume(true, 0, 2), "the stray fronts of a silent pin");
    assert!(!may_resume(false, 0, 100), "a gate that is not stopped has nothing to go back to");
}
