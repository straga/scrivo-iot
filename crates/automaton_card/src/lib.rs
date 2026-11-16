//! The automaton's card as the page reads it: its values by address, from what the automaton did this second.
//!
//! **ONE CODE FOR THE BOARD AND THE STAND.** The burner builds its card here, and so does the page's stand on a computer
//! (`examples/scenario.rs`), which runs the automaton itself second by second on sensor readings a scenario gives. What
//! the stand shows of the automaton is what the board would send, and not a card written by hand.

mod numbers;

pub use numbers::{heat_tuning, tuning};

use automaton::{Asked, Clock, Doing, State};

/// The card the automaton lives on.
pub const CARD: &str = "automaton";
/// The buttons, by the names the page's press route sends.
pub const START: &str = "start";
pub const STOP: &str = "stop";
/// The switch that allows or bans the feed, by the name the page's on and off routes send.
pub const FEED: &str = "feed";
/// The switch of the mode, Semi on and Auto off, by the name the page's on and off routes send.
pub const SEMI: &str = "semi";

/// What the card shows: the last second, and how it came about.
pub struct Card<'a> {
    /// What the automaton did in its last second; None before its first.
    pub doing: Option<Doing>,
    /// How long it has been in its state, in whole seconds.
    pub lasted_s: u32,
    /// Why the numbers on the card could not be taken, in the door's words; None - they were.
    pub refused: Option<&'a str>,
    /// What ends in this state and in how many seconds (`Burning::clocks`).
    pub clocks: &'a [Clock],
    /// The stage it failed on, while it burns off after the failure and stands in the fault.
    pub broke_at: Option<&'static str>,
    /// The water past which the alarm sounds, for its words.
    pub alarm_c: f32,
    /// Whether the auger may turn (`Automaton::feed_allowed`).
    pub feed_allowed: bool,
    /// Whether the automaton runs in Semi (`Automaton::semi`).
    pub semi: bool,
    /// How long each sensor it sees by has been silent.
    pub silent: Silent,
}

/// How long each sensor the automaton sees by has been silent, in whole seconds: past three periods of its round, when
/// its reading is no longer given out. None - it speaks. Published by the automaton, because the card of a sensor whose
/// round has stopped is published by nobody.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Silent {
    pub draught: Option<u32>,
    pub flame: Option<u32>,
    pub flue: Option<u32>,
    pub water: Option<u32>,
}

/// The card as values by address (`burner_api::first_burner`): every row and part of it, [`page::dictionary::Value::None`]
/// where it has nothing to say in this state. What a person reads is the client's to make of them.
///
/// **THE SAME ADDRESSES IN EVERY STATE**, as the rows were: a reader sees each of them
/// every time the card is sent, and the card's shape never follows the stage.
pub fn values<'c>(card: &Card<'c>, each: &mut dyn FnMut(usize, page::dictionary::Said<'c>)) {
    use burner_api::first_burner::{at, clock_at};
    use page::dictionary::{Said, Value};
    let mut put = |at, value: Value| each(at, Said::Value(value));
    let or_none = |v: Option<Value>| v.unwrap_or(Value::None);
    let state = card.doing.map(|d| d.state);
    put(at::STAGE, state.map_or(Value::Unknown("starting"), |s| Value::Word(automaton::stage(s))));
    let why = match state {
        Some(State::BurningOff(Some(why)) | State::Fault(why)) => Some(why),
        _ => None,
    };
    put(
        at::DOING,
        or_none(state.map(|s| {
            Value::Word(match s {
                State::BurningOff(Some(_)) => "burning_off_because",
                other => automaton::stage(other),
            })
        })),
    );
    put(at::DOING_ATTEMPT, or_none(match state {
        Some(State::Kindling(n)) => Some(Value::Number(n as f32)),
        _ => None,
    }));
    put(at::DOING_WHY, or_none(why.map(|w| Value::Word(automaton::why_name(w)))));
    put(at::DOING_TRIES, or_none(match why {
        Some(automaton::Why::NeverLit(n) | automaton::Why::FlameNotBack(n)) => Some(Value::Number(n as f32)),
        _ => None,
    }));
    put(at::DOING_WATER, or_none(match why {
        Some(automaton::Why::Overheated(c)) => Some(Value::Number(c)),
        _ => None,
    }));
    put(at::BROKE_AT, or_none(card.broke_at.map(Value::Word)));
    let s = card.silent;
    for (address, silent) in [
        (at::SILENT_DRAUGHT, s.draught),
        (at::SILENT_FLAME, s.flame),
        (at::SILENT_FLUE, s.flue),
        (at::SILENT_WATER, s.water),
    ] {
        put(address, or_none(silent.map(Value::Seconds)));
    }
    put(at::FOR, or_none(card.doing.map(|_| Value::Seconds(card.lasted_s))));
    // THE CLOCKS COUNT DOWN: whole seconds up, so a clock never reads nought while its thing still runs
    for what in automaton::CLOCKS {
        let Some(address) = clock_at(what) else { continue };
        let left = card.clocks.iter().find(|c| c.what == what).map(|c| c.left_s.map_or(Value::Unknown(burner_api::automaton::NOT_BEGUN), |left| Value::Seconds(left.ceil() as u32)));
        put(address, or_none(left));
    }
    let doing = card.doing;
    put(at::AUGER, or_none(doing.map(|d| Value::On(d.auger))));
    put(at::FEED, Value::On(card.feed_allowed));
    put(at::SEMI, Value::On(card.semi));
    put(at::IGNITER, or_none(doing.map(|d| Value::On(d.igniter))));
    put(at::PUMP, or_none(doing.map(|d| Value::On(d.pump))));
    put(at::PUMP_BY, or_none(doing.map(|d| Value::Word(automaton::pump_word(d.pump_why)))));
    put(at::PUMP_BY_WATER, or_none(doing.and_then(|d| match d.pump_why {
        automaton::PumpWhy::Water(c) | automaton::PumpWhy::Overheated(c) => Some(Value::Number(c)),
        automaton::PumpWhy::NoNumber => None,
    })));
    put(at::AUTOMATON_FAN, or_none(doing.map(|d| match d.fan {
        Asked::Stop => Value::Word("off"),
        Asked::Purge => Value::Word("full"),
        Asked::Hold(pa) => Value::Number(pa),
        Asked::ByHand => Value::Word("by_hand"),
    })));
    let alarm = doing.is_some_and(|d| d.alarm);
    put(at::ALARM, if alarm { Value::On(true) } else { Value::None });
    put(at::ALARM_PAST, if alarm { Value::Number(card.alarm_c) } else { Value::None });
    put(at::ORGANS, Value::Words(&automaton::ORGANS));
    each(at::REFUSED, card.refused.map_or(Said::Value(Value::None), Said::Text));
}
