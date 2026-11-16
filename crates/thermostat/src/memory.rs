//! What the board writes down about the thermostat: the last press of Start or Stop, and nothing else.
//!
//! **THE SWITCH IS A PERSON'S WORD, AND THE BOARD KEEPS IT AS ONE.** It is not a setting: a setting is a number a person types
//! on a card and an image never touches, and this is the state of a button that a restart must not drop - a boiler that was
//! asked for heat and lost its power at night lights itself when the power is back, and a burner that was stopped does not.
//! Only the two words of a press are written. A board nobody has pressed anything on writes nothing, and reads back `Unasked`.
//!
//! **A WORD NOBODY WROTE IS NOT A PRESS.** A key left by an older image, a write torn by the power, a word typed into the
//! console by hand: [`kept_switch`] answers `Unasked` for all of them, the side where nothing lights itself.

use automaton::Switch;

/// The card whose memory keeps the switch (`settings::memory_key`: `m.heat_switch`, thirteen bytes of the fifteen nvs takes).
pub const SWITCH_CARD: &str = "heat_switch";

/// The word a switch is kept under: `on` or `off`; none for `Unasked`, which is not written because nobody has said it.
pub fn switch_kept(switch: Switch) -> Option<&'static str> {
    match switch {
        Switch::On => Some("on"),
        Switch::Off => Some("off"),
        Switch::Unasked => None,
    }
}

/// The switch a kept WORD stands for; `Unasked` for nothing kept and for any word [`switch_kept`] never writes.
pub fn kept_switch(word: Option<&str>) -> Switch {
    match word {
        Some("on") => Switch::On,
        Some("off") => Switch::Off,
        _ => Switch::Unasked,
    }
}

/// Every answer to "whose word is the switch" by the name a client reads it by: a person's press this time, or the memory of the last one.
pub const FROMS: [&str; 2] = ["person", "memory"];

/// The name of whose word the switch is, from [`FROMS`].
pub fn from_word(from_memory: bool) -> &'static str {
    FROMS[usize::from(from_memory)]
}

/// What one try at writing the switch down came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Settled {
    /// Nothing to write, or a refused write that waits for its second.
    Nothing,
    /// The flash took the word; AFTER_REFUSALS is how many times it had refused before.
    Written { after_refusals: u32 },
    /// The flash refused. FIRST is true for the first of a series only: the journal says a series once.
    Refused { first: bool, why: String },
}
