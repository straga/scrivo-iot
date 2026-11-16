//! What the flame says in the journal: a word is said when it has HELD, and only the words that are an event go to the journal.
//!
//! The flame wrote a long line at every change of its word, and on a low fire it changes several times a second - weak, steady, weak - so the journal of a fire at 3 kW was the flame and nothing else.
//! What a person reads the journal for is that the flame went out and that it came back. So a word is taken to be said only after it has been the same for [`SETTLED_S`] seconds, and the lines are
//! three: out, lit (it appeared: weak or steady), and unconfigured (the thresholds are no thresholds). Weak and steady trading places is a detail, for whoever turns the level up.
//!
//! The seconds are the caller's, a counter that only goes forward.

use crate::Flame;

/// How long a word has to stay the same before it is said: long enough to pass over the flicker of a low fire, short enough that a flame out is told while it still matters - the automaton waits longer than
/// this before it gives a fire up.
pub const SETTLED_S: u32 = 5;

/// What a word that has held is worth.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Heard {
    /// The flame is out: no light on the sensor.
    Out,
    /// A flame has appeared: it was out (or nothing was said yet) and is weak or steady.
    Lit,
    /// The thresholds are not two thresholds: a fault of the setting.
    Unconfigured,
    /// Weak became steady or steady became weak: said at `debug`.
    Detail,
}

/// What the journal has been told of the flame, and what has been the same since when.
pub struct Voice {
    said: Option<Flame>,
    pending: Option<(Flame, u32)>,
}

impl Voice {
    pub const fn new() -> Self {
        Self { said: None, pending: None }
    }

    /// The flame reads WORD at NOW_S. -> what to say of it, or None while nothing new has held.
    pub fn heard(&mut self, word: Flame, now_s: u32) -> Option<Heard> {
        if self.said == Some(word) {
            self.pending = None;
            return None;
        }
        let since = match self.pending {
            Some((was, since)) if was == word => since,
            _ => {
                self.pending = Some((word, now_s));
                now_s
            }
        };
        if now_s.wrapping_sub(since) < SETTLED_S {
            return None;
        }
        let before = self.said.replace(word);
        self.pending = None;
        Some(match word {
            Flame::Out => Heard::Out,
            Flame::Unconfigured => Heard::Unconfigured,
            Flame::Weak | Flame::Steady => match before {
                None | Some(Flame::Out) | Some(Flame::Unconfigured) => Heard::Lit,
                Some(_) => Heard::Detail,
            },
        })
    }
}

impl Voice {
    /// The pin was not read this second: what had been holding has not held through it, so the count of a word begins again. What was said stays said.
    pub fn unread(&mut self) {
        self.pending = None;
    }
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}
