//! The ring a curve is drawn from. No hardware here and no sensor: readings in, the last N of them out.

use curve::{Mean, Ring};

#[test]
fn a_new_ring_has_nothing_to_draw() {
    let ring: Ring<4> = Ring::new();
    assert_eq!(ring.len(), 0);
    assert_eq!(ring.points(), Vec::<u16>::new());
}

#[test]
fn points_come_out_oldest_first_so_a_curve_reads_left_to_right() {
    let mut ring: Ring<4> = Ring::new();
    for v in [10, 20, 30] {
        ring.push(v);
    }
    assert_eq!(ring.len(), 3);
    assert_eq!(ring.points(), vec![10, 20, 30]);
}

#[test]
fn the_oldest_point_is_the_one_that_goes_when_the_ring_is_full() {
    // THE POINT OF A RING: memory does not grow, and what is lost is the past rather than the present.
    let mut ring: Ring<4> = Ring::new();
    for v in [10, 20, 30, 40, 50, 60] {
        ring.push(v);
    }
    assert_eq!(ring.len(), 4);
    assert_eq!(ring.points(), vec![30, 40, 50, 60]);
}

#[test]
fn a_ring_of_one_keeps_the_last_reading_and_nothing_else() {
    let mut ring: Ring<1> = Ring::new();
    ring.push(7);
    ring.push(9);
    assert_eq!(ring.points(), vec![9]);
}

#[test]
fn what_a_ring_costs_is_its_length_and_never_more() {
    // the number that has to be defensible in the work: points × 2 bytes, plus the bookkeeping
    assert_eq!(std::mem::size_of::<Ring<600>>(), 600 * 2 + std::mem::size_of::<usize>() * 2);
}

// ---- the mean of a run, for the slow ring under a fast sensor ----

#[test]
fn a_mean_answers_only_when_its_run_is_full() {
    // the slow ring takes one point a second off a sensor read ten times a second: nine of the ten readings
    // add to the run and say nothing, and the tenth is the point
    let mut mean: Mean<3> = Mean::new();
    assert_eq!(mean.add(10), None);
    assert_eq!(mean.add(20), None);
    assert_eq!(mean.add(33), Some(21));
    assert_eq!(mean.add(10), None);
}

#[test]
fn a_mean_is_rounded_to_the_nearest_like_the_window_of_samples() {
    let mut mean: Mean<2> = Mean::new();
    assert_eq!(mean.add(10), None);
    assert_eq!(mean.add(11), Some(11));
}

#[test]
fn a_run_of_the_biggest_readings_does_not_wrap() {
    // the same door the window of samples was caught in: a sum of u16 does not fit a u16
    let mut mean: Mean<8> = Mean::new();
    let mut said = None;
    for _ in 0..8 {
        said = mean.add(u16::MAX);
    }
    assert_eq!(said, Some(u16::MAX));
}

// ---- a fast sensor keeps two curves, and they answer different questions ----

#[test]
fn the_fast_ring_takes_every_reading_and_the_slow_one_takes_the_mean_of_a_run() {
    // FAST 4, SLOW 3, a run of 2: every reading lands on the fast curve, and every second one puts a mean on
    // the slow curve. The questions are different - pulsations against the drift of minutes - and so are the
    // curves.
    let mut pair: curve::Pair<4, 3, 2> = curve::Pair::new();
    for v in [10, 20, 30, 40] {
        pair.push(v);
    }
    assert_eq!(pair.fast.points(), vec![10, 20, 30, 40]);
    assert_eq!(pair.slow.points(), vec![15, 35]);
}

#[test]
fn a_run_that_has_not_finished_puts_nothing_on_the_slow_curve() {
    let mut pair: curve::Pair<4, 3, 10> = curve::Pair::new();
    for v in [10, 20, 30] {
        pair.push(v);
    }
    assert_eq!(pair.fast.len(), 3);
    assert_eq!(pair.slow.len(), 0);
}

#[test]
fn what_a_pair_costs_is_both_rings_and_the_run_between_them() {
    // the number the work has to defend: 600 fast points and 900 slow ones, two bytes each
    let of_pair = std::mem::size_of::<curve::Pair<600, 900, 10>>();
    assert!(of_pair >= (600 + 900) * 2, "{of_pair}");
    assert!(of_pair <= (600 + 900) * 2 + 64, "{of_pair} - the bookkeeping is more than a few words");
}

#[test]
fn a_ring_of_numbers_keeps_fractions_signs_and_seconds_with_nothing_to_say() {
    // the regulator's halves of a move: under nought, in fractions, and NaN in a second it did not regulate
    let mut ring: Ring<3, f32> = Ring::filled_with(f32::NAN);
    assert!(ring.points().is_empty(), "the unfilled places were read out as readings");
    for v in [-0.25, f32::NAN, 1.5, 0.125] {
        ring.push(v);
    }
    let got = ring.points();
    assert_eq!(got.len(), 3);
    assert!(got[0].is_nan(), "a second with nothing to say came out as {}", got[0]);
    assert_eq!((got[1], got[2]), (1.5, 0.125));
    assert_eq!(std::mem::size_of::<Ring<300, f32>>(), 300 * 4 + std::mem::size_of::<usize>() * 2);
}
