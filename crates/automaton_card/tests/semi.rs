//! Semi: the automaton runs the burner as in Auto, and a person may switch any organ in any stage; what the hand did
//! holds until the automaton decides something new about that organ. A banned feed bans the hand's
//! auger in Semi too, and Stop is above the hand in both modes. The numbers are the rows' own (tests/common), through the
//! mapping the board and the stand share (`automaton_card::tuning`), and the organs are driven by the automaton's own
//! `Driving`, the road the board takes.

mod common;

use automaton::{Asked, Doing, Feeding, Hands, State};
use common::pair::{Automaton, Burning, Driving, Press, Seen, Silence, Sound};
use flame::Flame;


fn sound() -> Sound {
    common::sound(&[])
}

/// The relays and the fan as they stand, whoever switched them last.
#[derive(Default)]
struct Relays {
    auger: bool,
    feeding: Feeding,
    /// Whether the auger's schedule has it turning, as the automaton's last second sampled it.
    sampled: bool,
    igniter: bool,
    pump: bool,
    fan: Option<Asked>,
}

impl Hands for Relays {
    // The relay's owner holds the rhythm and is back at its scheduled state whenever it is told; between the told it
    // switches at the edges of the schedule, which `follow` takes from the automaton's own phase.
    fn auger(&mut self, feeding: Feeding) {
        self.feeding = feeding;
        self.auger = feeding != Feeding::Stop && self.sampled;
    }
    fn auger_still(&mut self, _: Feeding) {}
    fn igniter(&mut self, on: bool) {
        self.igniter = on;
    }
    fn pump(&mut self, on: bool) {
        self.pump = on;
    }
    fn fan(&mut self, asked: Asked) {
        self.fan = Some(asked);
    }
}

type Board = Driving<Relays>;

/// A second of BOARD over SEEN: the relay's owner switches at the edge the second brought.
fn second(board: &mut Board, seen: Seen, secs: f32, t: &Sound) -> Doing {
    let did = board.second(seen, secs, t);
    follow(board, did)
}

fn follow(board: &mut Board, did: Doing) -> Doing {
    let relays = &mut board.hands.inner;
    if relays.feeding != Feeding::Stop && did.auger != relays.sampled {
        relays.auger = did.auger;
    }
    relays.sampled = did.auger;
    did
}

/// A person's command to ORGAN: judged by the automaton, then carried out on the relay itself - past the automaton's
/// hands, as the board carries it (the modules' `person`).
fn hand(board: &mut Board, organ: &str, on: bool) -> Result<(), String> {
    board.automaton.by_hand(organ)?;
    let relays = &mut board.hands.inner;
    match organ {
        "auger" => relays.auger = on,
        "igniter" => relays.igniter = on,
        "pump" => relays.pump = on,
        other => panic!("no relay {other} in these rows"),
    }
    Ok(())
}

/// Lit at 90 s, at work from 150 s; the water at WATER degrees.
fn seen(at: u32, water: f32) -> Seen {
    let flame = if at >= 90 { Flame::Steady } else { Flame::Out };
    Seen { flame, boiler_c: Some(water), return_c: None, flue_c: Some(30.0 + 0.2 * at as f32), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// A board started at 2 s, run to second UNTIL over water at 40 °C; SEMI is the mode from the first second.
fn board_until(until: u32, semi: bool) -> (Board, Vec<Doing>) {
    board_over(until, semi, 40.0)
}

/// The same over water at WATER degrees.
fn board_over(until: u32, semi: bool, water: f32) -> (Board, Vec<Doing>) {
    let t = sound();
    let mut board = Driving::new(Automaton::new(Burning::new()), Relays::default());
    board.set_semi(semi);
    let seconds = (0..until)
        .map(|at| match at {
            2 => board.press(Press::Start, seen(at, water), &t),
            _ => second(&mut board, seen(at, water), 1.0, &t),
        })
        .collect();
    (board, seconds)
}

/// The next second at work whose auger the automaton leaves standing, from 160 s.
fn a_pause_at_work() -> u32 {
    let (_, seconds) = board_until(400, false);
    (160..400).find(|&at| seconds[at as usize].state == State::Working && !seconds[at as usize].auger).expect("no pause at work") as u32
}

#[test]
fn in_semi_the_auger_switched_on_at_work_turns_until_the_automaton_decides_the_auger_anew() {
    let pause = a_pause_at_work();
    let (mut board, _) = board_until(pause, true);
    hand(&mut board, "auger", true).expect("Semi refused the auger at work");
    let t = sound();
    let mut decided = Vec::new();
    let mut relay = Vec::new();
    for at in pause..pause + 60 {
        let d = second(&mut board, seen(at, 40.0), 1.0, &t);
        decided.push(d.auger);
        relay.push(board.hands.inner.auger);
    }
    // the automaton's pause goes on for a while, then its dose, then its next pause
    let dose = decided.iter().position(|on| *on).expect("the automaton never dosed in a minute of work");
    let after = dose + decided[dose..].iter().position(|on| !*on).expect("the dose never ended");
    assert!(relay[..dose].iter().all(|on| *on), "the hand's auger stopped before the automaton decided the auger anew");
    assert!(!relay[after], "the automaton's next pause did not stop the auger the hand had switched on");
}

#[test]
fn in_semi_the_igniter_switched_off_while_lighting_stays_off_to_the_end_of_the_attempt() {
    let (_, seconds) = board_until(90, false);
    let hot = seconds.iter().position(|d| d.igniter).expect("the plug never came on") as u32 + 2;
    let (mut board, _) = board_until(hot, true);
    hand(&mut board, "igniter", false).expect("Semi refused the igniter while lighting");
    let t = sound();
    let dark = |at: u32| Seen { flame: Flame::Out, ..seen(at, 40.0) };
    let mut attempt = Vec::new();
    for at in hot..hot + 400 {
        let d = second(&mut board, dark(at), 1.0, &t);
        if d.state != State::Kindling(1) {
            break;
        }
        attempt.push(board.hands.inner.igniter);
    }
    assert!(attempt.len() > 10, "the attempt ended within {} s of the hand", attempt.len());
    assert!(attempt.iter().all(|on| !*on), "the igniter came back on within the attempt the hand switched it off in");
}

#[test]
fn in_semi_the_pump_switched_on_over_cold_water_runs_until_the_automaton_decides_the_pump_anew() {
    // the pump runs from 35 °C by the card; the water at 20 °C keeps the automaton's pump standing
    let (mut board, _) = board_over(200, true, 20.0);
    assert!(!board.hands.inner.pump, "the rows expect the pump standing over water at 20 °C");
    let t = sound();
    let cold = |at: u32| seen(at, 20.0);
    hand(&mut board, "pump", true).expect("Semi refused the pump at work");
    let relay: Vec<bool> = (201..260).map(|at| { second(&mut board, cold(at), 1.0, &t); board.hands.inner.pump }).collect();
    assert!(relay.iter().all(|on| *on), "the pump the hand switched on stopped while the automaton kept deciding it off");
}

#[test]
fn in_auto_the_hand_at_work_is_refused_in_words_that_say_what_to_switch() {
    let (mut board, seconds) = board_until(200, false);
    assert_eq!(seconds[199].state, State::Working);
    let refused = hand(&mut board, "auger", true).expect_err("Auto let the auger go by hand at work");
    assert!(refused.contains("Semi"), "the refusal does not say that Semi takes the hand: {refused}");
}

#[test]
fn in_semi_a_banned_feed_still_refuses_the_auger_by_hand() {
    let (mut board, _) = board_until(200, true);
    board.feed(false, seen(200, 40.0), &sound());
    assert!(hand(&mut board, "auger", true).is_err(), "the feed is banned and the auger went by hand in Semi");
    assert_eq!(hand(&mut board, "igniter", true), Ok(()), "a ban on the feed took the igniter from the hand in Semi");
}

#[test]
fn in_semi_stop_after_the_hand_puts_out_as_ever() {
    let pause = a_pause_at_work();
    let (mut board, _) = board_until(pause, true);
    hand(&mut board, "auger", true).expect("Semi refused the auger at work");
    hand(&mut board, "igniter", true).expect("Semi refused the igniter at work");
    let did = board.press(Press::Stop, seen(pause, 40.0), &sound());
    assert_eq!(did.state, State::BurningOff(None));
    let relays = &board.hands.inner;
    assert!(!relays.auger && !relays.igniter, "Stop left the hand's auger {} and igniter {}", relays.auger, relays.igniter);
}

#[test]
fn back_to_auto_the_automaton_takes_the_organs_back() {
    let pause = a_pause_at_work();
    let (mut board, _) = board_until(pause, true);
    hand(&mut board, "auger", true).expect("Semi refused the auger at work");
    board.set_semi(false);
    second(&mut board, seen(pause, 40.0), 1.0, &sound());
    assert!(!board.hands.inner.auger, "back in Auto, the hand's auger turned on through the automaton's pause");
}

#[test]
fn the_mode_is_kept_in_a_word_and_comes_back_from_it() {
    assert!(automaton::semi_of(Some(automaton::mode_word(true))), "Semi written down came back as Auto");
    assert!(!automaton::semi_of(Some(automaton::mode_word(false))), "Auto written down came back as Semi");
}

#[test]
fn a_mode_nobody_wrote_is_auto() {
    // the pair: nothing kept, a torn write, a word of something else - the board comes up in Auto
    for word in [None, Some(""), Some("sem"), Some("SEMI"), Some("semi\n")] {
        assert!(!automaton::semi_of(word), "{word:?} came back as Semi");
    }
}

#[test]
fn in_semi_stop_pressed_in_burning_off_still_takes_the_hand_s_auger_off() {
    // a burning off does not hear Stop and does not change, so no change of state takes the organs back: Stop itself does
    let pause = a_pause_at_work();
    let (mut board, _) = board_until(pause, true);
    let t = sound();
    board.press(Press::Stop, seen(pause, 40.0), &t);
    second(&mut board, seen(pause + 1, 40.0), 1.0, &t);
    hand(&mut board, "auger", true).expect("Semi refused the auger in burning off");
    let did = board.press(Press::Stop, seen(pause + 1, 40.0), &t);
    assert!(matches!(did.state, State::BurningOff(_)), "the rows expect the burning off to go on: {:?}", did.state);
    assert!(!board.hands.inner.auger, "Stop in burning off left the hand's auger turning");
}

#[test]
fn in_semi_start_pressed_leaves_the_hand_alone() {
    // the pair: only Stop is above the hand - a Start in the middle of the work changes nothing of what the hand holds
    let pause = a_pause_at_work();
    let (mut board, _) = board_until(pause, true);
    hand(&mut board, "igniter", true).expect("Semi refused the igniter at work");
    board.press(Press::Start, seen(pause, 40.0), &sound());
    assert!(board.hands.inner.igniter, "a Start at work took the igniter from the hand");
}
