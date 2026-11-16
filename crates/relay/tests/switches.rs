//! The switches of the relays kept for whoever tells them in the journal with the mains power beside them: a relay that REALLY changed, by a command, by its limit running out or by its plan, is noted once; a command that
//! leaves it as it was is no switch, and the opening of every relay at start is not one either. Whoever takes the notes empties them.

use std::time::Duration;

use relay::{Feeding, Relays};

const WITHIN: Duration = Duration::from_secs(2);

/// ASK made in a thread of its own while this one is the owner at NOW.
fn served<const N: usize>(relays: &Relays<N>, now: u32, limit_ms: u32, ask: impl FnOnce(&Relays<N>) + Send) {
    std::thread::scope(|s| {
        let asker = s.spawn(|| ask(relays));
        while !asker.is_finished() {
            relays.serve_for(Duration::from_millis(10), &|| now, &|_| limit_ms, &mut |_, _| {}, &|| {});
        }
        asker.join().unwrap()
    })
}

fn taken<const N: usize>(relays: &Relays<N>) -> (Vec<(usize, bool)>, u32) {
    let mut each = Vec::new();
    let dropped = relays.take_switches(&mut |relay, closed| each.push((relay, closed)));
    (each, dropped)
}

#[test]
fn the_opening_at_start_is_no_switch() {
    let relays = Relays::<3>::new();
    relays.start(&mut |_, _| {});
    assert_eq!(taken(&relays), (vec![], 0));
}

#[test]
fn a_command_that_closes_a_relay_and_one_that_opens_it_are_two_switches_in_order() {
    let relays = Relays::<3>::new();
    served(&relays, 0, 0, |r| r.set(1, true, WITHIN).unwrap());
    served(&relays, 0, 0, |r| r.set(1, false, WITHIN).unwrap());
    assert_eq!(taken(&relays), (vec![(1, true), (1, false)], 0));
}

#[test]
fn a_command_that_leaves_the_relay_as_it_was_is_no_switch() {
    let relays = Relays::<3>::new();
    served(&relays, 0, 0, |r| r.set(0, false, WITHIN).unwrap());
    served(&relays, 0, 0, |r| r.set(0, true, WITHIN).unwrap());
    served(&relays, 0, 0, |r| r.set(0, true, WITHIN).unwrap());
    assert_eq!(taken(&relays), (vec![(0, true)], 0));
}

#[test]
fn a_toggle_is_a_switch_each_time() {
    let relays = Relays::<3>::new();
    served(&relays, 0, 0, |r| r.toggle(2, WITHIN).unwrap());
    served(&relays, 0, 0, |r| r.toggle(2, WITHIN).unwrap());
    assert_eq!(taken(&relays), (vec![(2, true), (2, false)], 0));
}

#[test]
fn a_limit_that_runs_out_opens_the_relay_and_that_is_a_switch() {
    let relays = Relays::<2>::new();
    relays.set_limit(0, 1000);
    served(&relays, 100, 1000, |r| r.set(0, true, WITHIN).unwrap());
    assert_eq!(taken(&relays).0, vec![(0, true)]);
    assert!(relays.expire(1200, &mut |_, _| {}));
    assert_eq!(taken(&relays), (vec![(0, false)], 0));
}

#[test]
fn a_plan_that_has_the_relay_close_and_open_gives_a_switch_for_each_edge() {
    let relays = Relays::<2>::new();
    relays.start(&mut |_, _| {});
    // a dose of 800 ms: the relay closes at once, and opens when it has run
    relays.feed(1, Feeding::Dose { left_ms: 800 }, 0);
    relays.step(10, &mut |_, _| {});
    relays.step(2000, &mut |_, _| {});
    assert_eq!(taken(&relays), (vec![(1, true), (1, false)], 0));
}

#[test]
fn what_was_taken_is_not_given_twice() {
    let relays = Relays::<2>::new();
    served(&relays, 0, 0, |r| r.set(0, true, WITHIN).unwrap());
    assert_eq!(taken(&relays).0.len(), 1);
    assert_eq!(taken(&relays), (vec![], 0));
}

#[test]
fn more_switches_than_the_notes_hold_are_counted_and_not_lost_without_a_word() {
    let relays = Relays::<1>::new();
    for _ in 0..6 {
        served(&relays, 0, 0, |r| r.set(0, true, WITHIN).unwrap());
        served(&relays, 0, 0, |r| r.set(0, false, WITHIN).unwrap());
    }
    let (each, dropped) = taken(&relays);
    assert_eq!(each.len() as u32 + dropped, 12, "{each:?} and {dropped} dropped");
    assert!(dropped > 0, "twelve switches fitted in the notes: {each:?}");
    // the first ones are the ones kept, in order
    assert_eq!(each[0], (0, true));
    assert_eq!(each[1], (0, false));
}
