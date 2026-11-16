//! The work feeds by a pair of its own, not by the dose of the lighting: the lighting pours
//! one dose of `auto_dose`, and the fire at work turns the auger `auto_feed_on_s` of every `auto_feed_on_s +
//! auto_feed_off_s` - 2 of 22 s on the rows' own numbers (tests/common), through the mapping the board and the stand share
//! (`automaton_card::tuning`), and the automaton runs on them second by second.

mod common;

use automaton::{State};
use common::pair::{Automaton, Burning, Press, Seen, Silence, Sound};
use common::sound;
use flame::Flame;


/// A cold boiler lit at 2 s: the flame shows once the first dose is down, the flue warms steadily, the water stays cool.
fn seen_at(at: u32) -> Seen {
    let flame = if at >= 90 { Flame::Steady } else { Flame::Out };
    Seen { flame, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0 + at as f32 * 0.5), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// The automaton's seconds from Start on SOUND, and a second NEW that takes over at the second AT, if any: each second
/// the state and whether the auger turns.
fn run(seconds: u32, first: &Sound, then: Option<(u32, &Sound)>) -> Vec<(State, bool)> {
    let mut a = Automaton::new(Burning::new());
    let mut out = Vec::new();
    for at in 0..seconds {
        let sound = match then {
            Some((from, new)) if at >= from => new,
            _ => first,
        };
        let second = match at {
            2 => a.press(Press::Start, seen_at(at), sound),
            _ => a.second(seen_at(at), 1.0, sound),
        };
        out.push((second.state, second.auger));
    }
    out
}

/// The whole doses the auger poured while the automaton was in a state WANTED, from the second FROM on: (on, off) of each
/// cycle, from the start of one dose to the start of the next.
fn cycles(seconds: &[(State, bool)], wanted: State, from: usize) -> Vec<(u32, u32)> {
    let mut cycles = Vec::new();
    let (mut on, mut off, mut begun) = (0, 0, false);
    for pair in seconds[from..].windows(2) {
        let [(_, was), (state, now)] = pair else { unreachable!() };
        if *state != wanted {
            continue;
        }
        if *now && !*was {
            if begun {
                cycles.push((on, off));
            }
            (on, off, begun) = (0, 0, true);
        }
        if begun {
            if *now {
                on += 1;
            } else {
                off += 1;
            }
        }
    }
    cycles
}

#[test]
fn at_work_the_auger_turns_two_seconds_of_twenty_two() {
    let seconds = run(700, &sound(&[]), None);
    let worked = cycles(&seconds, State::Working, 0);
    assert!(worked.len() >= 3, "the fire did not work long enough to be measured: {worked:?}");
    for (on, off) in &worked {
        assert_eq!((*on, on + off), (2, 22), "a cycle at work: {on} s of the auger in {} s", on + off);
    }
}

#[test]
fn the_lighting_pours_one_dose_of_thirty_two_seconds() {
    // the pair: the work's own numbers do not reach the lighting
    let seconds = run(120, &sound(&[]), None);
    let kindling: Vec<bool> = seconds.iter().filter(|(s, _)| *s == State::Kindling(1)).map(|(_, on)| *on).collect();
    let poured = kindling.iter().filter(|on| **on).count();
    let doses = kindling.windows(2).filter(|w| w[1] && !w[0]).count() + usize::from(kindling.first() == Some(&true));
    assert_eq!((poured, doses), (32, 1), "the lighting poured {poured} s in {doses} doses");
}

#[test]
fn a_work_pair_saved_while_the_fire_works_acts_from_the_next_dose() {
    let first = sound(&[]);
    let changed = sound(&[("auto_feed_on_s", 5.0), ("auto_feed_off_s", 25.0)]);
    let seconds = run(900, &first, Some((500, &changed)));
    let after = cycles(&seconds, State::Working, 530);
    assert!(after.len() >= 3, "the fire did not work long enough after the change: {after:?}");
    for (on, off) in &after {
        assert_eq!((*on, on + off), (5, 30), "a cycle after the work's pair was changed to 5 / 25 s");
    }
}
