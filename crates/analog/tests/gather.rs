//! How a window of samples is gathered while the unit is held: the samples one after another with a pause between them, and no
//! longer than `MOST_HELD` by the steady clock - the lock of the unit is the other sensor's wait. The pause, the read and the clock are the
//! rows' own, so a stall (a pause that runs long) is made by hand.

use std::cell::Cell;

use analog::{gather, Gathered, MOST_HELD_US};

const SAMPLES: usize = 20;

const PAUSE_US: u64 = 2_000;

/// A gathering of SAMPLES whose pauses are PAUSE_US each and EVERY more; the pauses numbered in STALLS (from 1) run long by the given microseconds.
/// What comes back: the gathering, the steady clock when it ended, and how many pauses were made.
fn gathering(stalls: &[(usize, u64)], every: u64) -> (Gathered<()>, u64, usize) {
    let clock = Cell::new(0u64);
    let pauses = Cell::new(0usize);
    let mut samples = [0u16; SAMPLES];
    let got = gather(
        &mut samples,
        || Ok((100, 1000)),
        || {
            pauses.set(pauses.get() + 1);
            let extra = stalls.iter().filter(|(n, _)| *n == pauses.get()).map(|(_, us)| *us).sum::<u64>();
            clock.set(clock.get() + PAUSE_US + extra + every);
        },
        || clock.get(),
    );
    (got, clock.get(), pauses.get())
}

#[test]
fn a_window_that_keeps_its_time_is_whole() {
    let (got, _, _) = gathering(&[], 0);
    assert_eq!((got.taken, got.cut), (SAMPLES, false));
}

#[test]
fn a_window_whose_pauses_run_long_is_cut_at_the_limit_and_keeps_the_samples_it_took() {
    // every pause 20 ms: twenty of them would hold the unit for 400 ms, the other sensor all that time
    let (got, held_us, _) = gathering(&[], 18_000);
    assert!(got.cut, "a window held {held_us} us was not cut");
    assert!(got.taken > 0 && got.taken < SAMPLES, "{} samples kept", got.taken);
    assert!(held_us <= MOST_HELD_US + 20_000, "held {held_us} us, the limit is {MOST_HELD_US} us and one pause");
}

#[test]
fn a_leap_forward_of_the_steady_clock_inside_a_window_does_not_cut_it() {
    // the steady clock counts a leap forward as one PAD of time (between::clock): a window of its own length plus that is still whole
    let pad = between::clock::PAD.as_micros() as u64;
    let (got, _, _) = gathering(&[(10, pad)], 0);
    assert_eq!((got.taken, got.cut), (SAMPLES, false));
}

#[test]
fn a_read_that_is_refused_is_counted_and_the_others_are_taken() {
    let clock = Cell::new(0u64);
    let n = Cell::new(0usize);
    let mut samples = [0u16; SAMPLES];
    let got = gather(
        &mut samples,
        || {
            n.set(n.get() + 1);
            if n.get() % 5 == 0 {
                Err("busy")
            } else {
                Ok((4095, 3100))
            }
        },
        || clock.set(clock.get() + PAUSE_US),
        || clock.get(),
    );
    assert_eq!((got.taken, got.refused), (SAMPLES - 4, Some("busy")));
    assert_eq!(got.at_ceiling, SAMPLES - 4);
}
