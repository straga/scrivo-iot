//! The lines the journal would get from a run, by the policies of crates/flame (`voice`) and crates/automaton (`voice`), against what the burner wrote before them: the same seconds, counted both ways.
//!
//!     python3 tools/journal_lines.py SCENARIO...          # makes the input from the stand's frames and reads this example's answer
//!     cargo run -q -p burner_api --example journal_lines < INPUT > ANSWER
//!
//! INPUT is JSON: {"lit_mv", "steady_mv", "seconds": [{"mv": millivolts or null, "state": the automaton's `doing` word, "organs": what the organs are told, as any text that differs when they do}]}.
//! ANSWER lists the seconds each line would have been written at: `flame_was` - a line at every change of the word, as it stood before -, `flame_info` and `flame_debug`, `automaton_was` - a line at every transition and at every change of the organs -,
//! `automaton_info` and `automaton_debug`. The policy is the crates'; this only feeds it the seconds and counts what it says.

use std::io::Read;

use automaton::voice::{voice, Voice};
use automaton::{State, Why};
use flame::voice::Heard;
use flame::Thresholds;
use serde_json::{json, Value};

/// The automaton's state for a `doing` word. The attempt of a lighting and the reason of a burning off or a fault are not in the word, so a change of attempt is not seen here: the count of transitions is a floor.
fn state_of(word: &str) -> State {
    match word {
        "purge" => State::Purge,
        "kindling" => State::Kindling(1),
        "rising" => State::Rising,
        "working" => State::Working,
        "smouldering" => State::Smouldering,
        "burning_off" | "burning_off_because" => State::BurningOff(None),
        "fault" => State::Fault(Why::NeverLit(1)),
        _ => State::Stop,
    }
}

fn main() {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).expect("the input on stdin");
    let input: Value = serde_json::from_str(&text).expect("the input is JSON");
    let thresholds = Thresholds { lit_mv: input["lit_mv"].as_u64().expect("lit_mv") as u32, steady_mv: input["steady_mv"].as_u64().expect("steady_mv") as u32 };
    let seconds = input["seconds"].as_array().expect("seconds");

    let mut flame_voice = flame::voice::Voice::new();
    let (mut flame_was, mut flame_info, mut flame_debug) = (Vec::new(), Vec::new(), Vec::new());
    let mut word_before = None;
    let (mut automaton_was, mut automaton_info, mut automaton_debug) = (Vec::new(), Vec::new(), Vec::new());
    let mut before: Option<(State, String)> = None;

    for (at, second) in seconds.iter().enumerate() {
        let at = at as u32;
        if second["mv"].as_f64().is_none() {
            flame_voice.unread();
        }
        if let Some(mv) = second["mv"].as_f64() {
            let word = flame::read(mv.round() as u32, thresholds);
            if word_before != Some(word) {
                flame_was.push(at);
                word_before = Some(word);
            }
            match flame_voice.heard(word, at) {
                Some(Heard::Detail) => flame_debug.push(at),
                Some(_) => flame_info.push(at),
                None => {}
            }
        }
        let state = state_of(second["state"].as_str().unwrap_or("stop"));
        let organs = second["organs"].as_str().unwrap_or("").to_string();
        let organs_changed = before.as_ref().is_some_and(|(_, was)| *was != organs);
        // the line as it stood before: a transition or a change of the organs, both at `info`
        if before.is_none() || before.as_ref().is_some_and(|(was, _)| *was != state) || organs_changed {
            automaton_was.push(at);
        }
        match voice(before.as_ref().map(|(was, _)| *was), state, organs_changed) {
            Voice::Event => automaton_info.push(at),
            Voice::Detail => automaton_debug.push(at),
            Voice::Silent => {}
        }
        before = Some((state, organs));
    }
    println!(
        "{}",
        json!({
            "seconds": seconds.len(),
            "flame_was": flame_was, "flame_info": flame_info, "flame_debug": flame_debug,
            "automaton_was": automaton_was, "automaton_info": automaton_info, "automaton_debug": automaton_debug,
        })
    );
}
