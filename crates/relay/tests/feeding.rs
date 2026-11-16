//! What the task of a feeding relay does with the plan the automaton builds every second. Expected from what a person sets - a portion of 0.8 s and a pause of 5.0 s on one step - from what the count of pellets must say of it,
//! and from what the plan promises: the relay is closed for 0.8 s in every portion, to within 50 ms; a plan whose end has come stands for nothing and nothing extends it; an automaton that comes back to a dose it left does not
//! have it poured again; and a healthy automaton, whose step can stand for nine seconds between two plans, is not cut off. The task here sleeps as the board's does: for what `wait_ms` says and never longer than a second, and is late
//! by a few milliseconds. The automaton is the model below: it knows where it stands in a cycle and in a dose, by its own clock, and says that, built a few milliseconds before it is delivered.

use std::cell::{Cell, RefCell};

use pellets::{Date, Now, Pellets};
use relay::{Feeding, Relays, HORIZON_MS};

/// How late a sleeping task wakes, in ms: a tick of the scheduler.
const LATE: u32 = 20;
/// How long a plan is built before it is delivered, in ms: the automaton works out where it stands and the word goes through the modules.
const LAG: u32 = 15;
/// What the stage allows a portion to be off by.
const WITHIN_MS: u32 = 50;
/// Grams a minute the auger pours.
const RATE: u32 = 240;

/// The edges of the relay: (ms, closed).
type Edges = Vec<(u32, bool)>;

/// The plan of an automaton whose rhythm of ON_MS and OFF_MS began at 0, built LAG ms before T.
fn rhythm(on_ms: u32, off_ms: u32) -> impl Fn(u32) -> Option<Feeding> {
    move |t| Some(Feeding::Rhythm { into_ms: t.saturating_sub(LAG) % (on_ms + off_ms), on_ms, off_ms })
}

/// The plan of an automaton whose dose of TOTAL_MS began at 0: what is left of it by its own count, built LAG ms before T; a dose that is over is a stop.
fn dose(total_ms: u32) -> impl Fn(u32) -> Option<Feeding> {
    move |t| Some(if t.saturating_sub(LAG) < total_ms { Feeding::Dose { left_ms: total_ms - t.saturating_sub(LAG) } } else { Feeding::Stop })
}

/// The task of one relay from 0 to UNTIL_MS: at the first look after every whole second the automaton gives SAYS(ms), the first as a command and the rest as the same decision; None - it says nothing (it stuck, fell, or stands
/// in a step). Returns the edges.
fn drive(says: impl Fn(u32) -> Option<Feeding>, until_ms: u32) -> Edges {
    let relays = Relays::<1>::new();
    let now = Cell::new(0u32);
    let edges = RefCell::new(Edges::new());
    let mut write = |_: usize, closed: bool| edges.borrow_mut().push((now.get(), closed));
    relays.start(&mut write);
    let mut said_in = None;
    let mut first = true;
    while now.get() <= until_ms {
        let t = now.get();
        if said_in != Some(t / 1000) {
            said_in = Some(t / 1000);
            if let Some(plan) = says(t) {
                if first {
                    relays.feed(0, plan, t);
                    first = false;
                } else {
                    relays.still(0, plan, t);
                }
            }
        }
        relays.step(t, &mut write);
        let to_second = (t / 1000 + 1) * 1000 - t;
        let wait = relays.wait_ms(t).unwrap_or(1000).min(1000).min(to_second);
        now.set(t + wait.max(1) + LATE);
    }
    edges.into_inner()
}

/// The stretches the relay was closed whole, as (closed at, open at); a stretch the run ended in is left out.
fn closed_spans(edges: &Edges) -> Vec<(u32, u32)> {
    let mut spans = Vec::new();
    let mut since = None;
    for &(at, closed) in edges {
        match (closed, since) {
            (true, None) => since = Some(at),
            (false, Some(from)) => {
                spans.push((from, at));
                since = None;
            }
            _ => {}
        }
    }
    spans
}

fn near(got: u32, want: u32, what: &str) {
    assert!(got.abs_diff(want) <= WITHIN_MS, "{what}: {got} ms where {want} ms ± {WITHIN_MS} was asked");
}

/// The time the relay stood open last, the moment it was last opened; None if it never closed or ended closed.
fn opened_last(edges: &Edges) -> Option<u32> {
    (edges.iter().any(|(_, c)| *c) && !edges.last().is_some_and(|(_, c)| *c)).then(|| edges.last().map(|(at, _)| *at)).flatten()
}

// ---- the portions, as they were asked: the plan is built again every second and the edges still come where they should ----

#[test]
fn a_portion_of_eight_tenths_closes_the_relay_for_eight_tenths_in_every_portion() {
    let spans = closed_spans(&drive(rhythm(800, 5000), 65_000));
    assert!(spans.len() >= 11, "{} portions in 65 s of 5.8 s", spans.len());
    for (n, (from, to)) in spans.iter().enumerate() {
        near(to - from, 800, &format!("portion {n}"));
    }
    for (n, pair) in spans.windows(2).enumerate() {
        near(pair[1].0 - pair[0].1, 5000, &format!("pause {n}"));
    }
}

#[test]
fn a_portion_of_one_and_a_half_is_neither_one_nor_two() {
    let spans = closed_spans(&drive(rhythm(1500, 5000), 40_000));
    assert!(spans.len() >= 5, "{} portions", spans.len());
    for (n, (from, to)) in spans.iter().enumerate() {
        near(to - from, 1500, &format!("portion {n}"));
    }
}

#[test]
fn a_whole_two_seconds_with_fifteen_of_pause_is_what_it_was() {
    let spans = closed_spans(&drive(rhythm(2000, 15_000), 70_000));
    assert!(spans.len() >= 4, "{} portions", spans.len());
    for (n, (from, to)) in spans.iter().enumerate() {
        near(to - from, 2000, &format!("portion {n}"));
    }
    for (n, pair) in spans.windows(2).enumerate() {
        near(pair[1].0 - pair[0].1, 15_000, &format!("pause {n}"));
    }
}

#[test]
fn a_plan_built_again_every_second_does_not_begin_the_portion_again() {
    let spans = closed_spans(&drive(rhythm(800, 5000), 30_000));
    assert!(spans.len() >= 4, "{} portions in 30 s of 5.8 s", spans.len());
    for (n, pair) in spans.windows(2).enumerate() {
        near(pair[1].0 - pair[0].0, 5800, &format!("period {n}"));
    }
}

#[test]
fn a_dose_is_one_portion_of_the_length_the_automaton_counts_and_is_not_begun_again() {
    let spans = closed_spans(&drive(dose(1500), 30_000));
    assert_eq!(spans.len(), 1, "{spans:?}");
    near(spans[0].1 - spans[0].0, 1500, "the dose");
}

#[test]
fn a_stop_opens_the_relay_at_once_and_nothing_closes_it_after() {
    let says = |t: u32| if t < 20_000 { rhythm(3000, 2000)(t) } else { Some(Feeding::Stop) };
    let edges = drive(says, 60_000);
    assert!(edges.iter().any(|(at, closed)| *closed && *at < 20_000), "the relay never turned before the stop: {edges:?}");
    let last = edges.last().copied().expect("no edge at all");
    assert!(!last.1, "the relay ended closed");
    assert!(last.0 <= 20_000 + 1000 + WITHIN_MS, "the stop opened it at {} ms", last.0);
    assert!(edges.iter().filter(|(at, closed)| *closed && *at > 20_000).count() <= 1, "closed again after the stop: {edges:?}");
}

// ---- the pellets that come of it ----

fn day() -> Option<Now> {
    Some(Now { date: Date { year: 2026, month: 10, day: 19 }, minute: 12 * 60, offset_min: 180 })
}

/// What the count of pellets says in grams after the edges, asked at every edge and every second as the relays' owner asks it.
fn grams_after(edges: &Edges, until_ms: u32) -> f32 {
    let mut p = Pellets::default();
    p.at(day());
    let mut asks: Vec<(u32, bool)> = Vec::new();
    let mut closed = false;
    let mut edge = edges.iter().peekable();
    let mut second = 0;
    loop {
        let next_edge = edge.peek().map(|e| e.0);
        let at = match next_edge {
            Some(e) if e <= second => e,
            _ => second,
        };
        if at > until_ms {
            break;
        }
        while let Some(&&(e, c)) = edge.peek() {
            if e > at {
                break;
            }
            closed = c;
            edge.next();
        }
        asks.push((at, closed));
        if at == second {
            second += 1000;
        }
    }
    for (at, closed) in asks {
        p.observe(u64::from(at), closed, RATE);
    }
    p.at(day());
    p.kilograms()[2].1 * 1000.0
}

#[test]
fn ten_portions_of_eight_tenths_are_eight_seconds_of_the_rate() {
    let edges = drive(rhythm(800, 5000), 57_000);
    assert_eq!(closed_spans(&edges).len(), 10);
    let want = 8.0 * RATE as f32 / 60.0;
    let got = grams_after(&edges, 57_000);
    assert!((got - want).abs() <= want * 0.02, "{got} g where {want} g ± 2 % was asked");
}

#[test]
fn the_count_does_not_grow_while_the_auger_stands() {
    let says = |t: u32| if t < 20_000 { rhythm(800, 5000)(t) } else { Some(Feeding::Stop) };
    let edges = drive(says, 200_000);
    let soon = grams_after(&edges, 30_000);
    let later = grams_after(&edges, 200_000);
    assert!(soon > 0.0, "nothing was counted while the auger turned");
    assert_eq!(soon, later, "{soon} g at 30 s and {later} g at 200 s of a standing auger");
}

// ---- a hand in between ----

#[test]
fn a_relay_a_hand_closed_in_the_pause_stays_closed_to_the_next_edge_and_goes_back_when_the_automaton_takes_the_organ() {
    let relays = Relays::<1>::new();
    let mut written: Vec<(usize, bool)> = Vec::new();
    relays.start(&mut |r, c| written.push((r, c)));
    let says = rhythm(800, 5000);
    relays.feed(0, says(0).unwrap(), 0);
    relays.step(0, &mut |r, c| written.push((r, c)));
    relays.step(900, &mut |r, c| written.push((r, c)));
    assert!(!relays.closed(0), "the portion did not end");
    // a hand closes it in the pause
    std::thread::scope(|s| {
        let hand = s.spawn(|| relays.set(0, true, std::time::Duration::from_secs(2)));
        while !hand.is_finished() {
            relays.serve_for(std::time::Duration::from_millis(10), &|| 1000, &|_| 0, &mut |r, c| written.push((r, c)), &|| {});
        }
        hand.join().unwrap().unwrap();
    });
    assert!(relays.closed(0), "the hand's command did not close it");
    // the plans go on every second and take nothing back
    for second in 2..=4 {
        relays.still(0, says(second * 1000).unwrap(), second * 1000);
        relays.step(second * 1000, &mut |r, c| written.push((r, c)));
        assert!(relays.closed(0), "a plan of the same decision took the organ back from a hand at {second} s");
    }
    // a command (the automaton takes the organ back) puts it on the plan
    relays.feed(0, says(4500).unwrap(), 4500);
    relays.step(4500, &mut |r, c| written.push((r, c)));
    assert!(!relays.closed(0), "the command did not take the relay back from a hand");
    // and the plan goes on: the pause ends at 5.8 s and the next portion closes it
    relays.still(0, says(5800).unwrap(), 5800);
    relays.step(5900, &mut |r, c| written.push((r, c)));
    assert!(relays.closed(0), "the next portion did not begin on its plan");
}

// ---- the three defects of the review of round 2, and the plan that answers them ----

#[test]
fn a_plan_that_comes_after_the_end_of_the_one_before_does_not_stretch_it_though_the_task_has_not_looked_yet() {
    let relays = Relays::<1>::new();
    let edges = RefCell::new(Edges::new());
    let now = Cell::new(0u32);
    let mut write = |_: usize, closed: bool| edges.borrow_mut().push((now.get(), closed));
    relays.start(&mut write);
    let says = rhythm(20_000, 1000);
    relays.feed(0, says(0).unwrap(), 0);
    relays.step(0, &mut write);
    assert!(relays.closed(0));
    // the automaton says nothing for longer than the plan holds, and its next plan takes the lock before the task has woken
    let late = HORIZON_MS + 1001;
    now.set(late);
    assert!(relays.still(0, says(late).unwrap(), late), "a plan that came after the end of the one before did not say so");
    relays.step(late, &mut write);
    assert!(
        edges.borrow().iter().any(|(at, closed)| !*closed && *at == late),
        "the plan before had run out and the relay was never opened: the late plan stretched it ({:?})",
        edges.borrow()
    );
    // and the plan after it is followed: the portion goes on from where the automaton says it stands
    assert!(relays.closed(0), "the new plan was not followed: {:?}", edges.borrow());
}

#[test]
fn a_plan_that_comes_before_the_end_of_the_one_before_leaves_the_relay_as_it_is() {
    let relays = Relays::<1>::new();
    let edges = RefCell::new(Edges::new());
    let now = Cell::new(0u32);
    let mut write = |_: usize, closed: bool| edges.borrow_mut().push((now.get(), closed));
    relays.start(&mut write);
    let says = rhythm(20_000, 1000);
    relays.feed(0, says(0).unwrap(), 0);
    relays.step(0, &mut write);
    for t in [9_000u32, 17_000, 25_000] {
        now.set(t);
        assert!(!relays.still(0, says(t).unwrap(), t), "a plan in time was taken for a late one at {t} ms");
        relays.step(t, &mut write);
    }
    assert_eq!(edges.borrow().iter().filter(|(_, c)| !*c).count(), 1, "the relay was opened between plans that came in time: {:?}", edges.borrow());
}

#[test]
fn an_automaton_back_in_the_same_dose_does_not_pour_it_again_whether_the_dose_had_ended_or_was_cut() {
    // (a) a dose of 2 s that ended before the break; the automaton comes back at 30 s and its count says the dose is over
    let edges = drive(|t| if (3_000..30_000).contains(&t) { None } else { dose(2000)(t) }, 40_000);
    let spans = closed_spans(&edges);
    assert_eq!(spans.len(), 1, "a dose that had ended was poured again when the automaton came back: {edges:?}");
    near(spans[0].1 - spans[0].0, 2000, "the dose");
    // (b) a dose of 20 s, the automaton silent from 5 s; the last plan holds 10 s, so the relay is cut at 14 s; the automaton comes back at 25 s and its count says the dose is over
    let edges = drive(|t| if (5_000..25_000).contains(&t) { None } else { dose(20_000)(t) }, 50_000);
    let spans = closed_spans(&edges);
    assert_eq!(spans.len(), 1, "{edges:?}");
    assert!(spans[0].1 <= 4_000 + HORIZON_MS + WITHIN_MS + LATE, "the dose went on past the plan: {spans:?}");
    // (c) the same dose, the automaton back at 16 s with 4 s left by its count: those 4 s are poured and not 20
    let edges = drive(|t| if (5_000..16_000).contains(&t) { None } else { dose(20_000)(t) }, 50_000);
    let spans = closed_spans(&edges);
    let after_return: Vec<_> = spans.iter().filter(|(from, _)| *from >= 14_000).collect();
    let _ = after_return;
    let last = opened_last(&edges).expect("the relay never opened");
    assert!(last <= 20_000 + WITHIN_MS + LATE, "the dose of 20 s was poured in full after the return: open at {last} ms");
    // and without a break the dose goes as it did
    let spans = closed_spans(&drive(dose(20_000), 40_000));
    assert_eq!(spans.len(), 1);
    near(spans[0].1 - spans[0].0, 20_000, "an unbroken dose of 20 s");
}

#[test]
fn an_automaton_whose_step_stands_for_nine_seconds_between_two_plans_is_not_cut_off() {
    // portions of 4 s every 6 s; plans at 0 s and at 9 s: nine seconds are the sum of every wait a turn of the automaton's loop can have after the word to the auger
    let says = |t: u32| if (1_000..9_000).contains(&t) { None } else { rhythm(4000, 2000)(t) };
    let edges = drive(says, 11_500);
    let opened_early = edges.iter().any(|(at, closed)| !*closed && *at > 0 && *at < 4_000 - WITHIN_MS);
    let second_portion = edges.iter().any(|(at, closed)| *closed && (6_000 - WITHIN_MS..=6_000 + WITHIN_MS + LATE).contains(at));
    let third_open = edges.iter().any(|(at, closed)| !*closed && (10_000 - WITHIN_MS..=10_000 + WITHIN_MS + LATE).contains(at));
    assert!(!opened_early, "the rhythm was cut off in the first portion within the step of a healthy automaton: {edges:?}");
    assert!(second_portion, "the second portion did not begin on its plan: {edges:?}");
    assert!(third_open, "the second portion did not end on its plan after the automaton's plan came: {edges:?}");
}

#[test]
fn a_pause_in_the_plans_longer_than_they_hold_opens_the_relay_and_the_next_plan_goes_on_from_where_the_automaton_stands() {
    // cycles of 5 s closed and 2 s open; plans at 0 s and at 13 s: the plan of 0 s runs out at 10 s, in the portion that runs 7-12 s
    let says = |t: u32| if (1_000..13_000).contains(&t) { None } else { rhythm(5000, 2000)(t) };
    let edges = drive(says, 20_000);
    let opened = edges.iter().find(|(at, closed)| !*closed && *at > 7_000).map(|(at, _)| *at).expect("the relay never opened");
    near(opened, HORIZON_MS, "the relay was open when the plan ran out");
    let closed_again = edges.iter().find(|(at, closed)| *closed && *at > 10_500).map(|(at, _)| *at).expect("the rhythm did not go on");
    // the automaton's cycle stands at 13 s 6 s into itself - in the pause that ends at 14 s - and not at its beginning
    near(closed_again, 14_000, "the next portion began from where the automaton stands, not from its own word");
}

#[test]
fn an_automaton_that_goes_silent_leaves_the_auger_open_within_the_horizon_whether_in_a_portion_a_pause_or_a_dose() {
    for silent_from in [2_000u32, 5_000, 7_000] {
        let says = move |t: u32| if t >= silent_from { None } else { rhythm(4000, 2000)(t) };
        let edges = drive(says, 60_000);
        let last_plan = silent_from.saturating_sub(1000);
        let shut = opened_last(&edges).expect("the relay never closed or ended closed");
        assert!(shut <= last_plan + HORIZON_MS + WITHIN_MS + LATE, "silent from {silent_from} ms: open at {shut} ms, the last plan at {last_plan} ms");
        assert!(!edges.iter().any(|(at, closed)| *closed && *at > last_plan + HORIZON_MS + WITHIN_MS + LATE), "closed again after the plan ran out: {edges:?}");
    }
    let says = |t: u32| if t >= 5_000 { None } else { dose(60_000)(t) };
    let edges = drive(says, 60_000);
    let shut = opened_last(&edges).expect("the relay never closed");
    assert!(shut <= 4_000 + HORIZON_MS + WITHIN_MS + LATE, "a dose of 60 s went on to {shut} ms though the automaton said nothing from 4 s");
}

#[test]
fn a_plan_that_is_a_stop_closes_nothing_and_a_plan_in_a_pause_of_a_minute_wakes_the_task_before_it_runs_out() {
    let relays = Relays::<1>::new();
    relays.start(&mut |_, _| {});
    relays.feed(0, Feeding::Stop, 0);
    relays.step(0, &mut |_, _| {});
    assert!(!relays.still(0, Feeding::Stop, 1000));
    relays.step(1000, &mut |_, _| {});
    assert!(!relays.closed(0));
    assert_eq!(relays.wait_ms(1000), None, "a stop has nothing to wake the task for");
    relays.feed(0, Feeding::Rhythm { into_ms: 800, on_ms: 800, off_ms: 60_000 }, 2000);
    relays.step(2000, &mut |_, _| {});
    let wait = relays.wait_ms(2000).expect("a plan in a pause waits for nothing");
    assert!(wait <= HORIZON_MS, "the task would sleep {wait} ms past the end of the plan");
}
