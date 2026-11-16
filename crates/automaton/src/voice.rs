//! What the automaton says in the journal: a change of state is an event, and what the organs do inside one state is a detail.
//!
//! The auger on and off at every dose of a fire, 44 lines in ten minutes, buried the transitions - the lines that are the reason a journal exists. The card shows the organs every second; the journal says where
//! the fire went and who pressed what. The decision is here and the words are the device's.

use crate::{DraughtCounts, Heard, Seen, State, Why};

/// How a second of the automaton is worth saying.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Voice {
    /// A line at `info`: the state changed, or this is the first second.
    Event,
    /// A line at `debug`: the state is the same and what the organs are told is not.
    Detail,
    /// Nothing changed.
    Silent,
}

/// WAS the state of the second before (None at the first second), NOW the state now, ORGANS_CHANGED whether what the organs are told reads differently from the second before.
pub fn voice(was: Option<State>, now: State, organs_changed: bool) -> Voice {
    match was {
        Some(before) if before == now => {
            if organs_changed {
                Voice::Detail
            } else {
                Voice::Silent
            }
        }
        _ => Voice::Event,
    }
}

/// The journal's line about a burning off by the draught (`Why::NoDraught`, `Why::LowPressure`, `Why::DraughtSilent`), or none for any other reason: **WHAT LAY IN `Seen` AT THE SECOND THE DECISION WAS TAKEN**, so that a burning off whose cause the record cannot tell (29.10 15:44:33,
/// a number on the card for four seconds before) names it: what the sensor said, the pin's millivolts of the last window, how long the fan was held to a setpoint, how long there was no number, how long the sensor said "under my zero".
pub fn draught_line(why: Why, seen: &Seen, counts: DraughtCounts, pin_mv: Option<u32>) -> Option<String> {
    if !matches!(why, Why::NoDraught | Why::LowPressure | Why::DraughtSilent) {
        return None;
    }
    let said = match (seen.draught_pa, seen.draught_low, seen.silence.draught) {
        (Some(pa), _, _) => format!("a number, {pa:.0} Pa"),
        (None, true, _) => "under its zero".to_string(),
        (None, false, Heard::Silent) => "silent, its round stopped".to_string(),
        (None, false, Heard::TooLong) => "silent for long".to_string(),
        (None, false, Heard::Speaks) => "no number".to_string(),
    };
    let pin = pin_mv.map_or(String::new(), |mv| format!(", pin {mv} mV"));
    Some(format!(
        "burning off by the draught ({}): the sensor said {said} this second{pin}; the fan held {:.0} s, no number {:.0} s, under zero {:.0} s",
        crate::why_name(why),
        counts.held_s,
        counts.no_number_s,
        counts.low_s
    ))
}
