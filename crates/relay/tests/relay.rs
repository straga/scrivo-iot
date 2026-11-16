use std::cell::{Cell, RefCell};
use std::sync::atomic::Ordering;
use std::time::Duration;

use relay::*;

thread_local! {
    /// The limits this test has set, by relay: what the owner is handed back at a command, as the board hands back what the relays keep.
    static LIMITS: RefCell<[u32; 8]> = const { RefCell::new([0; 8]) };
}

/// A limit set on RELAYS and written down here, because the relays keep theirs where a test cannot read it.
fn limit<const N: usize>(relays: &Relays<N>, relay: usize, limit_ms: u32) {
    relays.set_limit(relay, limit_ms);
    if relay < 8 {
        LIMITS.with(|l| l.borrow_mut()[relay] = limit_ms);
    }
}

/// Long enough for any owner in these tests.
const WITHIN: Duration = Duration::from_secs(2);

/// ASK made in a thread of its own while this one is the owner at NOW, writing what it switches into WRITTEN and
/// counting its announcements into ANNOUNCED; what the asking came to.
fn asked<const N: usize, T: Send>(
    relays: &Relays<N>,
    now: u32,
    limit_ms: &(impl Fn(usize) -> u32 + Sync),
    written: &mut Vec<(usize, bool)>,
    announced: &Cell<u32>,
    ask: impl FnOnce(&Relays<N>) -> T + Send,
) -> T {
    std::thread::scope(|s| {
        let asker = s.spawn(|| ask(relays));
        while !asker.is_finished() {
            relays.serve_for(Duration::from_millis(10), &|| now, limit_ms, &mut |r, c| written.push((r, c)), &|| {
                announced.set(announced.get() + 1)
            });
        }
        asker.join().unwrap()
    })
}

/// A command made and carried out at NOW with the limits the relays keep.
fn serve<const N: usize>(relays: &Relays<N>, now: u32, written: &mut Vec<(usize, bool)>, ask: impl FnOnce(&Relays<N>) -> Result<(), Refused> + Send) {
    let limits = |r: usize| LIMITS.with(|l| l.borrow()[r]);
    asked(relays, now, &limits, written, &Cell::new(0), ask).unwrap();
}

#[test]
fn a_command_is_answered_after_the_owner_has_switched_the_relay() {
    let relays = Relays::<1>::new();
    let mut written = Vec::new();
    let closed = asked(&relays, 0, &|_| 0, &mut written, &Cell::new(0), |r| {
        r.set(0, true, WITHIN).unwrap();
        // what is read after the answer is the new state
        r.closed(0)
    });
    assert!(closed, "the answer came before the relay was switched");
}

#[test]
fn a_command_nobody_serves_is_refused_as_silent_and_switches_nothing() {
    let relays = Relays::<1>::new();
    assert_eq!(relays.set(0, true, Duration::from_millis(50)), Err(Refused::Silent));
    // the owner comes up afterwards: the command its asker gave up on is not carried out
    let mut written = Vec::new();
    relays.serve_for(Duration::from_millis(20), &|| 0, &|_| 0, &mut |r, c| written.push((r, c)), &|| {});
    assert!(written.is_empty(), "a refused command switched the relay: {written:?}");
    assert!(!relays.closed(0));
}

#[test]
fn at_start_every_relay_is_written_open() {
    let relays = Relays::<2>::new();
    let mut written = Vec::new();
    relays.start(&mut |r, c| written.push((r, c)));
    assert_eq!(written, vec![(0, false), (1, false)]);
    assert!(!relays.closed(0) && !relays.closed(1));
}

#[test]
fn a_command_switches_its_relay_and_announces_once() {
    let relays = Relays::<2>::new();
    let mut written = Vec::new();
    let announced = Cell::new(0);
    asked(&relays, 0, &|_| 0, &mut written, &announced, |r| r.toggle(1, WITHIN)).unwrap();
    assert_eq!(written, vec![(1, true)]);
    assert_eq!(announced.get(), 1);
    assert!(relays.closed(1) && !relays.closed(0));
}

#[test]
fn a_second_command_opens_it_again() {
    let relays = Relays::<1>::new();
    let mut written = Vec::new();
    for _ in 0..2 {
        serve(&relays, 0, &mut written, |r| r.toggle(0, WITHIN));
    }
    assert_eq!(written, vec![(0, true), (0, false)]);
    assert!(!relays.closed(0));
}

#[test]
fn nothing_moves_without_a_command() {
    let relays = Relays::<2>::new();
    let written = RefCell::new(Vec::new());
    relays.start(&mut |_, _| {});
    relays.serve_for(Duration::from_millis(20), &|| 0, &|_| 0, &mut |r, c| written.borrow_mut().push((r, c)), &|| {});
    assert!(written.borrow().is_empty());
}

#[test]
fn a_relay_the_board_does_not_have_is_refused() {
    let relays = Relays::<2>::new();
    assert_eq!(relays.toggle(2, WITHIN), Err(Refused::NoSuchRelay(2)));
    assert!(!relays.closed(2));
}

#[test]
fn a_queue_full_of_commands_nobody_took_refuses_at_once_and_keeps_control() {
    let relays = Relays::<1>::new();
    // the owner stands: every asker gives up, and what it asked stays in the queue
    for _ in 0..QUEUE {
        assert_eq!(relays.toggle(0, Duration::from_millis(10)), Err(Refused::Silent));
    }
    let began = std::time::Instant::now();
    let refused = relays.toggle(0, WITHIN);
    assert_eq!(refused, Err(Refused::Busy));
    assert!(began.elapsed() < Duration::from_millis(100), "busy is said at once, not after the limit");
    assert!(refused.unwrap_err().to_string().contains("refused"));
    // the owner comes back: what its askers gave up on is not carried out, and the next command is taken again
    let mut written = Vec::new();
    relays.serve_for(Duration::from_millis(20), &|| 0, &|_| 0, &mut |r, c| written.push((r, c)), &|| {});
    assert!(written.is_empty(), "a command its asker gave up on was carried out: {written:?}");
    serve(&relays, 0, &mut written, |r| r.toggle(0, WITHIN));
    assert_eq!(written, vec![(0, true)]);
}

#[test]
fn a_limited_relay_opens_by_itself_when_its_limit_runs_out_and_not_before() {
    let relays = Relays::<2>::new();
    limit(&relays, 0, 300_000);
    let mut written = Vec::new();
    serve(&relays, 1_000, &mut written, |r| r.set(0, true, WITHIN));
    assert_eq!(relays.left_ms(0, 1_000), Some(300_000));
    assert_eq!(relays.wait_ms(1_000), Some(300_000));
    // a millisecond before its time the relay is still closed
    assert!(!relays.expire(300_999, &mut |r, c| written.push((r, c))));
    assert!(relays.closed(0));
    assert_eq!(relays.left_ms(0, 300_999), Some(1));
    // at its time it opens, once
    assert!(relays.expire(301_000, &mut |r, c| written.push((r, c))));
    assert!(!relays.closed(0));
    assert_eq!(written, vec![(0, true), (0, false)]);
    assert!(!relays.expire(400_000, &mut |r, c| written.push((r, c))));
    assert_eq!(relays.wait_ms(400_000), None);
}

/// The relays' milliseconds as the board takes them (firmware::relays::now_ms): its steady clock over a chip's
/// clock that the test moves - one second a look, and at the sixth look a leap of three hours (the burner, 26.09).
fn looks_with_a_leap(read: &dyn Fn(u64) -> u32) -> Vec<u32> {
    let mut raw = 1_000_000u64;
    (0..10)
        .map(|look| {
            raw += if look == 6 { 10_771_098_000 } else { 1_000_000 };
            read(raw)
        })
        .collect()
}

#[test]
fn a_leap_of_the_clock_does_not_open_a_limited_relay_before_its_limit() {
    static RAW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1_000_000);
    fn raw() -> u64 {
        RAW.load(Ordering::Relaxed)
    }
    let steady = between::clock::Steady::new(raw, between::clock::MOST);
    let relays = Relays::<1>::new();
    limit(&relays, 0, 30_000);
    let mut written = Vec::new();
    let at = steady.now().as_millis() as u32;
    serve(&relays, at, &mut written, |r| r.set(0, true, WITHIN));
    for now in looks_with_a_leap(&|us| {
        RAW.store(us, Ordering::Relaxed);
        steady.now().as_millis() as u32
    }) {
        relays.expire(now, &mut |r, c| written.push((r, c)));
    }
    // ten looks a second apart and a leap: 9 s and MOST gone by the steady clock, of a limit of 30
    assert!(relays.closed(0), "a leap of the clock opened the igniter before its limit: {written:?}");
    assert_eq!(relays.left_ms(0, steady.now().as_millis() as u32), Some(30_000 - 9_000 - 2_000));
}

#[test]
fn the_same_leap_taken_by_the_chips_clock_would_have_opened_it_at_once() {
    let relays = Relays::<1>::new();
    limit(&relays, 0, 30_000);
    let mut written = Vec::new();
    serve(&relays, 1_000, &mut written, |r| r.set(0, true, WITHIN));
    for now in looks_with_a_leap(&|us| (us / 1000) as u32) {
        relays.expire(now, &mut |r, c| written.push((r, c)));
    }
    assert!(!relays.closed(0), "the chip's clock leapt three hours and the igniter stayed on");
}

#[test]
fn a_relay_without_a_limit_stays_closed() {
    let relays = Relays::<2>::new();
    limit(&relays, 0, 60_000);
    let mut written = Vec::new();
    serve(&relays, 0, &mut written, |r| r.set(1, true, WITHIN));
    assert_eq!(relays.left_ms(1, 0), None);
    assert_eq!(relays.wait_ms(0), None);
    assert!(!relays.expire(u32::MAX / 2, &mut |r, c| written.push((r, c))));
    assert!(relays.closed(1));
    assert_eq!(written, vec![(1, true)]);
}

#[test]
fn closing_it_again_counts_the_limit_from_the_new_command() {
    let relays = Relays::<1>::new();
    limit(&relays, 0, 10_000);
    let mut written = Vec::new();
    serve(&relays, 0, &mut written, |r| r.set(0, true, WITHIN));
    serve(&relays, 8_000, &mut written, |r| r.set(0, true, WITHIN));
    assert!(!relays.expire(12_000, &mut |r, c| written.push((r, c))));
    assert_eq!(relays.left_ms(0, 12_000), Some(6_000));
    assert!(relays.expire(18_000, &mut |r, c| written.push((r, c))));
}

#[test]
fn a_toggle_that_closes_it_starts_the_limit_too() {
    let relays = Relays::<1>::new();
    limit(&relays, 0, 10_000);
    let mut written = Vec::new();
    serve(&relays, 5_000, &mut written, |r| r.toggle(0, WITHIN));
    assert_eq!(relays.left_ms(0, 5_000), Some(10_000));
    assert!(relays.expire(15_000, &mut |r, c| written.push((r, c))));
    assert!(!relays.closed(0));
}

#[test]
fn a_shorter_limit_set_while_it_is_closed_opens_it_on_the_next_look() {
    let relays = Relays::<1>::new();
    limit(&relays, 0, 300_000);
    let mut written = Vec::new();
    serve(&relays, 0, &mut written, |r| r.set(0, true, WITHIN));
    limit(&relays, 0, 10_000);
    assert_eq!(relays.wait_ms(20_000), Some(0));
    assert!(relays.expire(20_000, &mut |r, c| written.push((r, c))));
}

#[test]
fn the_nearest_limit_is_what_the_owner_waits_for() {
    let relays = Relays::<3>::new();
    limit(&relays, 0, 300_000);
    limit(&relays, 1, 60_000);
    let mut written = Vec::new();
    for relay in 0..3 {
        serve(&relays, 0, &mut written, |r| r.set(relay, true, WITHIN));
    }
    assert_eq!(relays.wait_ms(10_000), Some(50_000));
}

#[test]
fn opening_it_by_command_leaves_nothing_to_wait_for() {
    let relays = Relays::<1>::new();
    limit(&relays, 0, 10_000);
    let mut written = Vec::new();
    serve(&relays, 0, &mut written, |r| r.set(0, true, WITHIN));
    serve(&relays, 1_000, &mut written, |r| r.set(0, false, WITHIN));
    assert_eq!(relays.left_ms(0, 1_000), None);
    assert_eq!(relays.wait_ms(1_000), None);
    assert!(!relays.expire(20_000, &mut |r, c| written.push((r, c))));
    assert_eq!(written, vec![(0, true), (0, false)]);
}

#[test]
fn a_limit_of_a_relay_the_board_does_not_have_changes_nothing() {
    let relays = Relays::<1>::new();
    limit(&relays, 5, 10_000);
    assert_eq!(relays.left_ms(5, 0), None);
    assert_eq!(relays.set(5, true, WITHIN), Err(Refused::NoSuchRelay(5)));
}

#[test]
fn the_open_level_is_on_every_pin_before_any_pin_is_an_output() {
    for closed_on_low in [true, false] {
        let steps = RefCell::new(Vec::new());
        open_pins(4, closed_on_low, &mut |p, high| steps.borrow_mut().push(format!("level {p} {high}")), &mut |p| {
            steps.borrow_mut().push(format!("output {p}"))
        });
        let steps = steps.into_inner();
        let open = level(false, closed_on_low);
        let levels: Vec<String> = (0..4).map(|p| format!("level {p} {open}")).collect();
        let outputs: Vec<String> = (0..4).map(|p| format!("output {p}")).collect();
        assert_eq!(steps, [levels, outputs].concat(), "closed on low: {closed_on_low}");
    }
}

#[test]
fn the_level_that_closes_a_relay_follows_its_board() {
    assert!(!level(true, true) && level(false, true));
    assert!(level(true, false) && !level(false, false));
}

#[test]
fn a_relay_says_whether_it_is_on_and_how_long_its_limit_leaves() {
    assert_eq!(words(false, None), "off");
    assert_eq!(words(true, None), "on");
    assert_eq!(words(true, Some(252_000)), "on - off by itself in 4:12");
    // a part of a second left reads as a whole one: the page never shows 0:00 on a closed relay
    assert_eq!(words(true, Some(1)), "on - off by itself in 0:01");
    assert_eq!(words(true, Some(300_000)), "on - off by itself in 5:00");
}

#[test]
fn an_open_relay_says_off_whatever_its_limit() {
    assert_eq!(words(false, Some(10_000)), "off");
}

#[test]
fn a_relay_closes_with_the_limit_in_force_at_the_command() {
    let relays = Relays::<1>::new();
    limit(&relays, 0, 300_000);
    let mut written = Vec::new();
    // the setting says 10 s by now, and the owner has not looked at it since
    asked(&relays, 1_000, &|_| 10_000, &mut written, &Cell::new(0), |r| r.set(0, true, WITHIN)).unwrap();
    assert_eq!(relays.left_ms(0, 1_000), Some(10_000));
}

#[test]
fn opening_a_relay_leaves_its_limit_as_it_was() {
    let relays = Relays::<1>::new();
    limit(&relays, 0, 300_000);
    let mut written = Vec::new();
    asked(&relays, 1_000, &|_| 10_000, &mut written, &Cell::new(0), |r| r.set(0, false, WITHIN)).unwrap();
    serve(&relays, 2_000, &mut written, |r| r.set(0, true, WITHIN));
    assert_eq!(relays.left_ms(0, 2_000), Some(300_000));
}
