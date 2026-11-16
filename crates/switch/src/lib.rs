//! A wall switch on a pin with a pull-up: the contact closes the pin to ground, so closed reads low. What a movement
//! of the key does is the switch's mode, with Tasmota's names: a
//! pushbutton toggles on the press only, a toggle switch on every movement.
//!
//! This is what a settled level means; waiting for the contact to settle is the board's (the device waits for an
//! edge, sleeps DEBOUNCE and reads the level once). The state is atomic, so the page
//! reads it from its own thread while the switch's thread moves it.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

/// Long enough for a contact to settle, short against a hand: Hinch's Switch.debounce_ms.
pub const DEBOUNCE: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// SwitchMode 0: every movement toggles - a key that stays where it is put.
    Toggle,
    /// SwitchMode 3: only the press toggles - a key that springs back.
    Pushbutton,
}

pub struct Switch {
    mode: Mode,
    closed: AtomicBool,
    toggles: AtomicU32,
}

impl Switch {
    /// A switch open until its first reading (`start`).
    pub const fn new(mode: Mode) -> Self {
        Self { mode, closed: AtomicBool::new(false), toggles: AtomicU32::new(0) }
    }

    /// Takes the position the key stands in at start without counting it as a movement.
    pub fn start(&self, closed: bool) {
        self.closed.store(closed, Ordering::Relaxed);
    }

    /// The level after the contact settled. Returns whether the position changed and whether that is a toggle; a
    /// level equal to the last one is a bounce that came back, and is neither.
    pub fn settled(&self, closed: bool) -> Movement {
        if self.closed.swap(closed, Ordering::Relaxed) == closed {
            return Movement::None;
        }
        if self.mode == Mode::Toggle || closed {
            self.toggles.fetch_add(1, Ordering::Relaxed);
            Movement::Toggle
        } else {
            Movement::Moved
        }
    }

    pub fn closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    /// How many times the key toggled what it is wired to since the board started.
    pub fn toggles(&self) -> u32 {
        self.toggles.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    /// No movement: the level is the one the switch stood at.
    None,
    /// The key moved, and the mode makes no toggle of it: the release of a pushbutton. Whoever shows it must know.
    Moved,
    /// The key moved, and the mode makes a toggle of it.
    Toggle,
}

/// The rows of this module in a device's dictionary (`page::dictionary`): what a client reads of it and what a person may do. The card it stands on and its name are the device's.
pub mod entry {
    use page::dictionary::{Entry, Kind};

    /// The row of the key NAME on the card CARD: closed is on, never commanded.
    pub const fn row(card: &'static str, name: &'static str) -> Entry {
        Entry::row(card, name, &[Kind::On])
    }

    /// How many times the key NAME of CARD toggled: a count with no digits after the point.
    pub const fn toggles(card: &'static str, name: &'static str) -> Entry {
        Entry { unit: "toggles", decimals: Some(0), ..Entry::part(card, name, "toggles", &[Kind::Number]) }
    }
}
