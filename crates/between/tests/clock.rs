//! The steady clock over a source that leaps, as the burner's did under fire on 26.09: back 2.6 s, forward ~3 h.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use between::clock::{Jumps, Leap, Moment, Steady};

const MOST: Duration = Duration::from_secs(2);

// every test its own source: they run at once
macro_rules! source {
    ($name:ident) => {
        static $name: AtomicU64 = AtomicU64::new(1_000_000);
    };
}

#[test]
fn a_source_that_goes_back_moves_the_clock_by_nothing_and_is_counted() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    let before = clock.now();
    RAW.store(111_695_000, Ordering::Relaxed);
    let at_111 = clock.now();
    RAW.store(109_053_000, Ordering::Relaxed); // 2.6 s back
    let after = clock.now();
    assert_eq!(after.since(at_111), Duration::ZERO, "a clock gone back moved the steady one");
    assert!(after >= at_111 && at_111 >= before);
    assert_eq!(at_111.since(after), Duration::ZERO, "a span to an earlier moment is not zero");
    let jumps = clock.jumps();
    assert_eq!(jumps.count, 2, "the leap forward of 110 s and the one back: {jumps:?}");
    assert_eq!(jumps.largest_back_ms, 2642);
}

#[test]
fn a_source_that_runs_on_as_it_should_is_the_clock_to_the_microsecond() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    let first = clock.now();
    let mut at = 1_000_000;
    for step in [1, 999_999, 1_500_000, 20_000, 1_999_999] {
        at += step;
        RAW.store(at, Ordering::Relaxed);
        let now = clock.now();
        assert_eq!(now.since(first), Duration::from_micros(at - 1_000_000));
    }
    assert_eq!(clock.jumps(), Jumps::default(), "steps within MOST were taken for jumps");
}

#[test]
fn a_source_that_leaps_forward_moves_the_clock_by_most_once_and_is_counted() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    let before = clock.now();
    RAW.store(1_000_000 + 10_771_098_000, Ordering::Relaxed); // ~3 h
    let after = clock.now();
    assert_eq!(after.since(before), MOST, "a leap of three hours moved the clock by {:?}", after.since(before));
    RAW.store(1_000_000 + 10_771_098_000 + 500_000, Ordering::Relaxed);
    assert_eq!(clock.now().since(after), Duration::from_millis(500), "the clock after the leap does not run on");
    let jumps = clock.jumps();
    assert_eq!((jumps.count, jumps.largest_forward_ms), (1, 10_771_098), "{jumps:?}");
}

#[test]
fn a_moment_and_a_span_add_up() {
    let m = Moment::default() + Duration::from_millis(1500);
    assert_eq!(m.as_millis(), 1500);
    assert_eq!((m + Duration::from_micros(1)).since(m), Duration::from_micros(1));
}

#[test]
fn the_leaps_are_said_in_the_line_of_status() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    clock.now();
    assert_eq!(clock.jumps().to_string(), "clock jumps: 0, largest back 0 ms, forward 0 ms");
    RAW.store(1_000_000 + 10_771_098_000, Ordering::Relaxed);
    clock.now();
    RAW.store(1_000_000 + 10_768_456_000, Ordering::Relaxed);
    clock.now();
    assert_eq!(clock.jumps().to_string(), "clock jumps: 2, largest back 2642 ms, forward 10771098 ms");
}

#[test]
fn tasks_reading_the_board_clock_at_once_take_no_leap_for_one() {
    // the board's own clock over the host's: several readers at once, as the relays, the automaton and the net loop
    let clock = Steady::new(
        || {
            static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
            START.get_or_init(std::time::Instant::now).elapsed().as_micros() as u64
        },
        MOST,
    );
    std::thread::scope(|s| {
        for _ in 0..8 {
            s.spawn(|| {
                for _ in 0..20_000 {
                    clock.now();
                }
            });
        }
    });
    assert_eq!(clock.jumps(), Jumps::default(), "readers at once were taken for leaps of the clock");
}

#[test]
fn the_uptime_does_not_take_a_leap_of_the_source() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    clock.now();
    RAW.store(1_000_000 + 10_771_098_000, Ordering::Relaxed); // about 3 h forward
    let up = clock.uptime_ms();
    assert!(up <= 1000 + 2000, "a leap of 3 h is in the uptime: {up} ms, at most the 1000 it had plus MOST");
}

#[test]
fn the_uptime_of_a_source_that_runs_on_is_the_source() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    clock.now();
    RAW.store(1_500_000, Ordering::Relaxed);
    assert_eq!(clock.uptime_ms(), 1500, "the chip started 1.5 s ago by its own clock");
    RAW.store(2_100_000, Ordering::Relaxed);
    assert_eq!(clock.uptime_ms(), 2100);
}

// the leaps a clock said, one slot per test: `said!` makes the function the clock calls and the place it writes to
macro_rules! said {
    ($log:ident, $hook:ident) => {
        static $log: std::sync::Mutex<Vec<Leap>> = std::sync::Mutex::new(Vec::new());
        fn $hook(leap: Leap) {
            $log.lock().unwrap().push(leap);
        }
    };
}

#[test]
fn a_leap_forward_is_said_once_with_its_size() {
    source!(RAW);
    said!(LOG, hook);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    clock.set_on_leap(hook);
    clock.now();
    RAW.store(1_000_000 + 10_771_098_000, Ordering::Relaxed);
    clock.now();
    clock.now(); // the next reading is no leap
    let said = LOG.lock().unwrap().clone();
    assert_eq!(said, vec![Leap { back: false, ms: 10_771_098 }]);
}

#[test]
fn a_leap_back_is_said_once_with_its_size() {
    source!(RAW);
    said!(LOG, hook);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    clock.set_on_leap(hook);
    RAW.store(111_695_000, Ordering::Relaxed);
    clock.now();
    RAW.store(109_053_000, Ordering::Relaxed); // 2.642 s back
    clock.now();
    clock.now();
    let said = LOG.lock().unwrap().clone();
    assert_eq!(said, vec![Leap { back: true, ms: 2642 }]);
}

#[test]
fn every_leap_is_said_and_a_source_that_runs_on_is_not() {
    source!(RAW);
    said!(LOG, hook);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    clock.set_on_leap(hook);
    clock.now();
    for step in 1..=5u64 {
        RAW.store(1_000_000 + step * 1_500_000, Ordering::Relaxed); // 1.5 s at a time, inside MOST
        clock.now();
    }
    let said = LOG.lock().unwrap().clone();
    assert!(said.is_empty(), "a source that runs on was said to leap: {said:?}");
    RAW.store(100_000_000, Ordering::Relaxed);
    clock.now();
    RAW.store(50_000_000, Ordering::Relaxed);
    clock.now();
    let said = LOG.lock().unwrap().clone();
    assert_eq!(said.len(), 2, "two leaps, two lines: {said:?}");
}

#[test]
fn a_hook_that_reads_the_clock_does_not_deadlock_it() {
    source!(RAW);
    static CLOCK: std::sync::OnceLock<Steady> = std::sync::OnceLock::new();
    static READ: AtomicU64 = AtomicU64::new(0);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    fn hook(_: Leap) {
        // the board's log line takes its stamp from the same clock
        CLOCK.get().unwrap().uptime_ms();
        READ.fetch_add(1, Ordering::Relaxed);
    }
    let clock = CLOCK.get_or_init(|| Steady::new(raw, MOST));
    clock.set_on_leap(hook);
    clock.now();
    RAW.store(1_000_000 + 10_771_098_000, Ordering::Relaxed);
    clock.now();
    assert_eq!(READ.load(Ordering::Relaxed), 1, "the hook was not called, or called more than once");
}

#[test]
fn a_leap_forward_pads_the_span_it_stands_in_by_what_the_clock_added() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    let before = clock.now();
    RAW.store(1_000_000 + 10_771_098_000, Ordering::Relaxed); // ~3 h forward: the clock took MOST of it
    let after = clock.now();
    assert_eq!(after.since(before), MOST);
    assert_eq!(clock.padded_between(before, after), MOST, "the 2 s the clock added are no time gone by");
    assert_eq!(clock.padded_between(after, after), Duration::ZERO, "a reading taken after the leap has it in already");
    RAW.store(1_000_000 + 10_771_098_000 + 500_000, Ordering::Relaxed);
    let later = clock.now();
    assert_eq!(clock.padded_between(after, later), Duration::ZERO, "no leap in the half second after it");
    assert_eq!(clock.padded_between(before, later), MOST, "the leap is in the span from before to later");
}

#[test]
fn a_leap_back_adds_nothing_to_pad_and_two_leaps_forward_pad_twice() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let clock = Steady::new(raw, MOST);
    let start = clock.now();
    RAW.store(500_000, Ordering::Relaxed); // back: the clock adds nothing
    let back = clock.now();
    assert_eq!(clock.padded_between(start, back), Duration::ZERO);
    RAW.store(500_000 + 10_000_000_000, Ordering::Relaxed);
    let first = clock.now();
    RAW.store(500_000 + 20_000_000_000, Ordering::Relaxed);
    let second = clock.now();
    assert_eq!(clock.padded_between(start, second), MOST * 2);
    assert_eq!(clock.padded_between(first, second), MOST, "only the second leap is after the first reading");
}

#[test]
fn a_clock_with_a_pad_takes_a_leap_forward_as_the_pad_and_a_step_inside_most_as_it_is() {
    source!(RAW);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let pad = Duration::from_millis(100);
    let clock = Steady::with_pad(raw, MOST, pad);
    let before = clock.now();
    RAW.store(1_000_000 + 1_500_000, Ordering::Relaxed); // 1.5 s: inside MOST, time gone by
    let inside = clock.now();
    assert_eq!(inside.since(before), Duration::from_millis(1500));
    RAW.store(1_000_000 + 1_500_000 + 10_771_098_000, Ordering::Relaxed); // ~3 h forward
    let after = clock.now();
    assert_eq!(after.since(inside), pad, "a leap of three hours moved the clock by {:?}", after.since(inside));
    assert_eq!(clock.padded_between(inside, after), pad, "what the leap added is no time gone by");
    assert_eq!(clock.jumps().count, 1, "the step inside MOST was taken for a jump");
}
