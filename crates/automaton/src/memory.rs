//! What the board writes down about the automaton, and the name of the stage a page draws.
//!
//! **ONLY THE STATE IS WRITTEN, because it is all [`crate::Burning::resumed`] takes.** How far into a purge, which
//! second of the igniter - those die with the power on purpose (see [`State`]), so there is nothing else to keep.
//!
//! **A WORD NOBODY WROTE IS NOT A STATE.** A key left by an older image, a write torn by the power, a word typed
//! into the console by hand: [`recalled`] answers None for all of them, and the board then comes up as one that
//! remembers nothing - which `resumed` still answers by the sensors, so a fire under a board with a spoiled
//! memory is burnt off all the same.

use crate::{State, Why};

/// The word STATE is kept under: one line of ASCII, the stage and what belongs to it after colons.
pub fn remembered(state: State) -> String {
    match state {
        State::Stop => "stop".into(),
        State::Purge => "purge".into(),
        State::Kindling(try_no) => format!("kindling:{try_no}"),
        State::Rising => "rising".into(),
        State::Working => "working".into(),
        State::Smouldering => "smouldering".into(),
        State::BurningOff(None) => "burning_off".into(),
        State::BurningOff(Some(why)) => format!("burning_off:{}", why_word(why)),
        State::Fault(why) => format!("fault:{}", why_word(why)),
    }
}

/// The state a kept WORD stands for; None for a word [`remembered`] never writes.
pub fn recalled(word: &str) -> Option<State> {
    let (head, rest) = match word.split_once(':') {
        Some((head, rest)) => (head, Some(rest)),
        None => (word, None),
    };
    match (head, rest) {
        ("stop", None) => Some(State::Stop),
        ("purge", None) => Some(State::Purge),
        ("kindling", Some(n)) => whole(n).map(State::Kindling),
        ("rising", None) => Some(State::Rising),
        ("working", None) => Some(State::Working),
        ("smouldering", None) => Some(State::Smouldering),
        ("burning_off", None) => Some(State::BurningOff(None)),
        ("burning_off", Some(why)) => why_of(why).map(|why| State::BurningOff(Some(why))),
        ("fault", Some(why)) => why_of(why).map(State::Fault),
        _ => None,
    }
}

/// The stage of the page's strip STATE belongs to: the name of its kind, without the attempt or the reason.
pub fn stage(state: State) -> &'static str {
    match state {
        State::Stop => "stop",
        State::Purge => "purge",
        State::Kindling(_) => "kindling",
        State::Rising => "rising",
        State::Working => "working",
        State::Smouldering => "smouldering",
        State::BurningOff(_) => "burning_off",
        State::Fault(_) => "fault",
    }
}

/// The stage it broke on, at a change of state from WAS to NOW, for the strip to light: taken when
/// a failure begins - a burning off with a reason, or a fault straight away - kept through that burning off and the
/// fault, and dropped at any other state. KEPT is what it was before this change; WAS is None on the first second.
pub fn broken_at(kept: Option<&'static str>, was: Option<State>, now: State) -> Option<&'static str> {
    let failing = |s: State| matches!(s, State::BurningOff(Some(_)) | State::Fault(_));
    match (was, failing(now)) {
        (_, false) => None,
        (Some(was), true) if !failing(was) => Some(stage(was)),
        _ => kept,
    }
}

/// The stages of the page's strip, every word [`stage`] answers.
pub const STAGES: &[&str] = &["stop", "purge", "kindling", "rising", "working", "smouldering", "burning_off", "fault"];

/// The reasons of a failure, every word [`why_name`] answers.
pub const WHYS: &[&str] = &["never_lit", "flame_lost", "flame_not_back", "no_fuel", "no_draught", "overheated", "unconfigured", "draught_silent", "flame_silent", "water_silent", "return_silent", "no_fronts", "low_pressure"];

/// The reason WHY by its word, without the count or the temperature it carries.
pub fn why_name(why: Why) -> &'static str {
    match why {
        Why::NeverLit(_) => "never_lit",
        Why::FlameLost => "flame_lost",
        Why::FlameNotBack(_) => "flame_not_back",
        Why::NoFuel => "no_fuel",
        Why::NoDraught => "no_draught",
        Why::LowPressure => "low_pressure",
        Why::Overheated(_) => "overheated",
        Why::Unconfigured => "unconfigured",
        Why::DraughtSilent => "draught_silent",
        Why::FlameSilent => "flame_silent",
        Why::WaterSilent => "water_silent",
        Why::ReturnSilent => "return_silent",
        Why::NoFronts => "no_fronts",
    }
}

fn why_word(why: Why) -> String {
    match why {
        Why::NeverLit(tries) | Why::FlameNotBack(tries) => format!("{}:{tries}", why_name(why)),
        // Display of an f32 is the shortest text that reads back as the same number
        Why::Overheated(celsius) => format!("{}:{celsius}", why_name(why)),
        _ => why_name(why).into(),
    }
}

fn why_of(word: &str) -> Option<Why> {
    match word.split_once(':') {
        Some(("never_lit", n)) => whole(n).map(Why::NeverLit),
        Some(("flame_not_back", n)) => whole(n).map(Why::FlameNotBack),
        Some(("overheated", c)) => c.parse::<f32>().ok().filter(|c| c.is_finite()).map(Why::Overheated),
        Some(_) => None,
        None => match word {
            "flame_lost" => Some(Why::FlameLost),
            "no_fuel" => Some(Why::NoFuel),
            "no_draught" => Some(Why::NoDraught),
            "low_pressure" => Some(Why::LowPressure),
            "unconfigured" => Some(Why::Unconfigured),
            "draught_silent" => Some(Why::DraughtSilent),
            "flame_silent" => Some(Why::FlameSilent),
            "water_silent" => Some(Why::WaterSilent),
            "return_silent" => Some(Why::ReturnSilent),
            "no_fronts" => Some(Why::NoFronts),
            _ => None,
        },
    }
}

/// A count from one: an attempt, or how many there were. Nought is no attempt at all.
fn whole(text: &str) -> Option<u32> {
    text.parse::<u32>().ok().filter(|n| *n >= 1).filter(|n| n.to_string() == text)
}

/// The card whose memory keeps Auto or Semi (`settings::memory_key`: `m.auto_mode`), apart from the state's.
pub const MODE_CARD: &str = "auto_mode";

/// The word the mode is kept under: `semi` or `auto`.
pub fn mode_word(semi: bool) -> &'static str {
    if semi {
        "semi"
    } else {
        "auto"
    }
}

/// Whether a kept WORD is Semi. Anything else - nothing kept, a torn write - is Auto.
pub fn semi_of(word: Option<&str>) -> bool {
    word == Some("semi")
}
