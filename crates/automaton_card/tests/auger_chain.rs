//! The whole road of a portion of pellets, on the computer: the automaton's card decides at work, `Driving` hands the rhythm to the relay's owner, the owner switches the relay on its own plan, the count of pellets
//! hears the relay. Expected from what a person sets (0.8 s of dose and 5.0 s of pause on a step) and from what the stage says is done: the relay is closed for 0.8 s ± 50 ms in every portion of the work, and the pellets counted
//! are the seconds the relay was closed times the rate.

mod common;

use automaton::{Asked, Doing, Feeding, Hands, State};
use common::pair::{Automaton, Burning, Driving, Press, Seen, Silence, Sound};
use flame::Flame;
use pellets::{Date, Now, Pellets};
use relay::Relays;

const RATE: u32 = 240;
/// How late a sleeping owner wakes, in ms.
const LATE: u32 = 20;

/// The relay of the auger as the device's `Organs` hands it: the automaton's words in seconds become the owner's in ms, and the owner is told at its clock.
struct Auger<'a> {
    relays: &'a Relays<1>,
    now_ms: u32,
}

/// The automaton's words in seconds as the owner counts them: milliseconds, never none.
fn told_of(feeding: Feeding) -> relay::Feeding {
    let ms = |s: f32| ((s * 1000.0).round() as u32).max(1);
    match feeding {
        Feeding::Stop => relay::Feeding::Stop,
        Feeding::Dose { left_s } => relay::Feeding::Dose { left_ms: ms(left_s) },
        Feeding::Rhythm { into_s, on_s, off_s } => relay::Feeding::Rhythm { into_ms: (into_s * 1000.0).round() as u32, on_ms: ms(on_s), off_ms: ms(off_s) },
    }
}

impl Hands for Auger<'_> {
    fn auger(&mut self, feeding: Feeding) {
        self.relays.feed(0, told_of(feeding), self.now_ms);
    }
    fn auger_still(&mut self, feeding: Feeding) {
        self.relays.still(0, told_of(feeding), self.now_ms);
    }
    fn igniter(&mut self, _: bool) {}
    fn pump(&mut self, _: bool) {}
    fn fan(&mut self, _: Asked) {}
}

fn seen(at: u32) -> Seen {
    let flame = if at >= 90 { Flame::Steady } else { Flame::Out };
    Seen { flame, boiler_c: Some(40.0), return_c: None, flue_c: Some(30.0 + 0.2 * at as f32), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

fn day() -> Option<Now> {
    Some(Now { date: Date { year: 2026, month: 10, day: 19 }, minute: 12 * 60, offset_min: 180 })
}

/// A burner started at 2 s and at work from about 150 s, on SOUND, for SECONDS of the automaton: the edges of the relay by the owner's ms, and what the automaton decided each second.
fn run(sound: &Sound, seconds: u32) -> (Vec<(u32, bool)>, Vec<Doing>, Pellets) {
    run_until_silent(sound, seconds, u32::MAX)
}

/// The same, but the automaton says nothing from second SILENT_FROM on - it stuck, fell, or stopped delivering its decisions - while the owner of the relay goes on to the end.
fn run_until_silent(sound: &Sound, seconds: u32, silent_from: u32) -> (Vec<(u32, bool)>, Vec<Doing>, Pellets) {
    run_with_gap(sound, seconds, silent_from, u32::MAX)
}

/// The same, but the automaton says nothing from second FROM to second BACK: its step stands (in the waits of its loop) or it fell and came up again. Back at BACK it takes the time that went in one step, as the board's loop measures it.
fn run_with_gap(sound: &Sound, seconds: u32, from: u32, back: u32) -> (Vec<(u32, bool)>, Vec<Doing>, Pellets) {
    let silent_from = from;
    let relays = Relays::<1>::new();
    let mut edges: Vec<(u32, bool)> = Vec::new();
    let mut pellets = Pellets::default();
    pellets.at(day());
    let mut board = Driving::new(Automaton::new(Burning::new()), Auger { relays: &relays, now_ms: 0 });
    let mut doings = Vec::new();
    // the owner looks at every edge of its plan and at least once a second; the pellets hear it at each of its looks, as the board's owner announces
    let look = |relays: &Relays<1>, now: u32, edges: &mut Vec<(u32, bool)>, pellets: &mut Pellets| {
        relays.step(now, &mut |_, closed| edges.push((now, closed)));
        pellets.observe(u64::from(now), relays.closed(0), RATE);
    };
    for at in 0..seconds {
        board.hands.inner.now_ms = at * 1000;
        if at < silent_from || at >= back {
            let secs = if at == back { (back - silent_from + 1) as f32 } else { 1.0 };
            let did = match at {
                2 => board.press(Press::Start, seen(at), sound),
                _ => board.second(seen(at), secs, sound),
            };
            doings.push(did);
        }
        let mut now = at * 1000;
        loop {
            look(&relays, now, &mut edges, &mut pellets);
            let to_second = (at + 1) * 1000 - now;
            let wait = relays.wait_ms(now).unwrap_or(1000).min(1000).min(to_second);
            if wait >= to_second {
                break;
            }
            now += wait + LATE.min(to_second - wait - 1);
        }
    }
    (edges, doings, pellets)
}

fn spans(edges: &[(u32, bool)]) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut since = None;
    for &(at, closed) in edges {
        match (closed, since) {
            (true, None) => since = Some(at),
            (false, Some(from)) => {
                out.push((from, at));
                since = None;
            }
            _ => {}
        }
    }
    out
}

#[test]
fn at_work_the_relay_is_closed_for_eight_tenths_in_every_portion_and_the_pellets_follow() {
    let sound = common::sound(&[("auto_feed_on_s", 0.8), ("auto_feed_off_s", 5.0)]);
    let (edges, doings, pellets) = run(&sound, 600);
    let first_work = doings.iter().position(|d| d.state == State::Working).expect("the burner never came to work") as u32 * 1000;
    let work: Vec<(u32, u32)> = spans(&edges).into_iter().filter(|(from, _)| *from >= first_work).collect();
    assert!(work.len() >= 50, "{} portions in 7 minutes of work", work.len());
    for (n, (from, to)) in work.iter().enumerate() {
        assert!((to - from).abs_diff(800) <= 50, "portion {n}: {} ms where 800 ± 50 was asked", to - from);
    }
    for (n, pair) in work.windows(2).enumerate() {
        assert!((pair[1].0 - pair[0].0).abs_diff(5800) <= 50, "period {n}: {} ms where 5800 ± 50 was asked", pair[1].0 - pair[0].0);
    }
    // the pellets are the seconds the relay was closed times the rate, to within 2 %
    let closed_ms: u32 = spans(&edges).iter().map(|(from, to)| to - from).sum();
    let want = closed_ms as f32 / 1000.0 * RATE as f32 / 60.0;
    let got = pellets.kilograms()[2].1 * 1000.0;
    assert!(want > 0.0 && (got - want).abs() <= want * 0.02, "{got} g counted where the relay's {closed_ms} ms make {want} g");
}

#[test]
fn a_whole_step_works_as_it_did() {
    let sound = common::sound(&[("auto_feed_on_s", 2.0), ("auto_feed_off_s", 15.0)]);
    let (edges, doings, _) = run(&sound, 600);
    let first_work = doings.iter().position(|d| d.state == State::Working).expect("the burner never came to work") as u32 * 1000;
    let work: Vec<(u32, u32)> = spans(&edges).into_iter().filter(|(from, _)| *from >= first_work).collect();
    assert!(work.len() >= 20, "{} portions", work.len());
    for (n, (from, to)) in work.iter().enumerate() {
        assert!((to - from).abs_diff(2000) <= 50, "portion {n}: {} ms", to - from);
    }
}

#[test]
fn an_automaton_that_goes_silent_at_work_leaves_the_auger_standing_within_the_horizon() {
    let sound = common::sound(&[("auto_feed_on_s", 20.0), ("auto_feed_off_s", 15.0)]);
    let (_, doings, _) = run(&sound, 400);
    let first_work = doings.iter().position(|d| d.state == State::Working).expect("the burner never came to work") as u32;
    // silent from 5 s into the first dose of 20 s, and from 25 s on, in the pause after it
    for silent_from in [first_work + 5, first_work + 25] {
        let (edges, _, _) = run_until_silent(&sound, 400, silent_from);
        let last_word = (silent_from - 1) * 1000;
        let shut = spans(&edges).last().map(|s| s.1).expect("the relay never closed");
        assert!(shut <= last_word + relay::HORIZON_MS + 50 + LATE, "silent from {silent_from} s: the auger went on to {shut} ms, the last word at {last_word} ms");
        assert!(!edges.last().unwrap().1, "the relay ended closed, silent from {silent_from} s");
        let closed_after = edges.iter().filter(|(at, closed)| *closed && *at > last_word + relay::HORIZON_MS + 50 + LATE).count();
        assert_eq!(closed_after, 0, "silent from {silent_from} s: closed {closed_after} times after the lease");
    }
}

#[test]
fn a_healthy_automaton_whose_step_stood_for_nine_seconds_is_not_cut_off_at_work() {
    // a dose of 20 s on the step: any opening in the middle of it is a cut
    let sound = common::sound(&[("auto_feed_on_s", 20.0), ("auto_feed_off_s", 15.0)]);
    let (_, doings, _) = run(&sound, 400);
    let first_work = doings.iter().position(|d| d.state == State::Working).expect("the burner never came to work") as u32;
    let from = first_work + 4;
    let (edges, _, _) = run_with_gap(&sound, 400, from, from + 8);
    let first = spans(&edges).into_iter().find(|(f, _)| *f >= first_work * 1000).expect("no portion at work");
    assert!(first.1 - first.0 >= 20_000 - 50 - LATE, "the dose of 20 s was cut at {} ms by a step of nine seconds", first.1 - first.0);
}

#[test]
fn an_automaton_back_in_the_lighting_dose_after_a_long_silence_pours_what_its_count_has_left_and_not_the_dose_again() {
    // the lighting dose is 32 s (the card's `auto_dose`); the automaton is silent for 20 s from 5 s into it, so the plan it left runs out at 10 s and it is back with 7 s of the dose by its own count
    let sound = common::sound(&[]);
    let (_, doings, _) = run(&sound, 200);
    let entered = doings.iter().position(|d| matches!(d.state, State::Kindling(_))).expect("the lighting never began") as u32;
    let from = entered + 5;
    let (edges, _, _) = run_with_gap(&sound, 200, from, from + 20);
    let attempt: u32 = spans(&edges).iter().filter(|(f, t)| *f >= entered * 1000 && *t <= (entered + 45) * 1000).map(|(f, t)| t - f).sum();
    assert!(attempt <= 32_000 + 50 + LATE, "the lighting dose of 32 s was poured as {attempt} ms in the attempt: more than the dose");
    assert!(attempt >= 20_000, "the dose was not poured after the return: {attempt} ms");
}
