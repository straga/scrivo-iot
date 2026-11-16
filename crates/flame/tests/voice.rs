//! What the flame says in the journal: a word is said when it has HELD, and only the words that are an event - the flame is out, the flame is lit - go to the journal; weak and steady trading places is a detail for whoever asks.
//! A flame that wrote a long line at every change of its word would write several a second on a low fire. What is held here is the policy on the seconds the caller counts.

use flame::voice::{Heard, Voice, SETTLED_S};
use flame::Flame::{self, Out, Steady, Unconfigured, Weak};

/// The words of each second, said through a voice; the lines it gives - the second and what it heard.
fn run(words: &[Flame]) -> Vec<(u32, Heard)> {
    let mut voice = Voice::new();
    words.iter().enumerate().filter_map(|(at, word)| voice.heard(*word, at as u32).map(|heard| (at as u32, heard))).collect()
}

fn held(word: Flame, seconds: u32) -> Vec<Flame> {
    vec![word; seconds as usize]
}

fn count(said: &[(u32, Heard)], what: Heard) -> usize {
    said.iter().filter(|(_, heard)| *heard == what).count()
}

#[test]
fn a_flame_that_goes_out_and_stays_is_said_once_when_it_has_held() {
    let said = run(&[held(Steady, 10), held(Out, 10)].concat());
    assert_eq!(count(&said, Heard::Out), 1, "{said:?}");
    assert!(said.contains(&(10 + SETTLED_S, Heard::Out)), "out is said the second it has held {SETTLED_S} s: {said:?}");
}

#[test]
fn a_flame_that_appears_is_said_once_as_lit_whether_it_is_weak_or_steady() {
    for word in [Weak, Steady] {
        let said = run(&[held(Out, 10), held(word, 10)].concat());
        assert_eq!(count(&said, Heard::Lit), 1, "{word:?}: {said:?}");
    }
}

#[test]
fn weak_and_steady_trading_places_is_a_detail_and_never_an_event() {
    let said = run(&[held(Weak, 10), held(Steady, 10), held(Weak, 10), held(Steady, 10)].concat());
    assert_eq!(count(&said, Heard::Lit), 1, "lit once, from the start: {said:?}");
    assert_eq!(count(&said, Heard::Detail), 3, "three settled trades: {said:?}");
    assert_eq!(count(&said, Heard::Out), 0);
}

#[test]
fn a_flame_that_changes_its_word_faster_than_it_holds_one_says_nothing() {
    // the low fire of the owner's snapshot: weak and steady several times a second, here every second
    let mut words = held(Steady, 10);
    for second in 0..60 {
        words.push(if second % 2 == 0 { Weak } else { Steady });
    }
    let said = run(&words);
    assert!(said.iter().all(|(at, _)| *at < SETTLED_S + 1), "a line in the middle of the flicker: {said:?}");
}

#[test]
fn a_flame_that_blinks_out_for_less_than_it_takes_to_hold_is_not_said_out() {
    let said = run(&[held(Steady, 10), held(Out, SETTLED_S - 1), held(Steady, 10)].concat());
    assert_eq!(count(&said, Heard::Out), 0, "{said:?}");
}

#[test]
fn the_lines_of_a_run_are_never_more_than_the_changes_that_held() {
    // a minute of flicker, twenty seconds out at work (work_flame_blinks), and the flame back
    let mut words = Vec::new();
    for _ in 0..4 {
        words.extend(held(Steady, 4));
        words.extend([Weak, Steady, Weak, Steady, Weak, Out, Weak, Steady]);
    }
    words.extend(held(Out, 20));
    words.extend(held(Steady, 16));
    // lit at the start, out, lit again
    let settled_events = 3;
    let said = run(&words);
    let events = count(&said, Heard::Lit) + count(&said, Heard::Out);
    assert!(events <= settled_events, "{events} event lines for {settled_events} settled changes: {said:?}");
}

#[test]
fn an_unconfigured_flame_is_said_once_when_it_holds_and_a_dark_pot_at_the_start_is_one_line() {
    let said = run(&[held(Steady, 10), held(Unconfigured, 10)].concat());
    assert_eq!(count(&said, Heard::Unconfigured), 1, "{said:?}");
    let said = run(&held(Out, 10));
    assert_eq!(said.iter().map(|(_, heard)| *heard).collect::<Vec<_>>(), vec![Heard::Out]);
}

#[test]
fn a_second_with_no_reading_breaks_the_hold_of_a_word_and_is_not_counted_towards_it() {
    // out for three seconds, the pin unread for one, out for the rest: a word that has not been read for a second has not held through it
    let mut voice = Voice::new();
    let mut said = Vec::new();
    for at in 0..SETTLED_S + 4 {
        if at == 3 {
            voice.unread();
            continue;
        }
        if let Some(heard) = voice.heard(Out, at) {
            said.push((at, heard));
        }
    }
    assert!(said.iter().all(|(at, _)| *at >= 3 + SETTLED_S), "out was said before it had held {SETTLED_S} s since the gap: {said:?}");
}

#[test]
fn a_gap_in_the_readings_does_not_take_back_what_was_said() {
    let mut voice = Voice::new();
    for at in 0..SETTLED_S + 1 {
        voice.heard(Steady, at);
    }
    voice.unread();
    assert_eq!(voice.heard(Steady, SETTLED_S + 2), None, "the same word again after a gap is not news");
}
