//! The fronts of the zero-crossing pin, lost and back: the second that tells it, once each.
//!
//! A detector is ALIVE while a second of it holds what a mains of 40 to 71 Hz gives ([`crate::LIVE_CROSSINGS`]). The fall from alive to below the band is the loss, and the first
//! second back in the band is the return. A detector that has never been alive says nothing: a board with no mains on its module (the stand) has lost nothing. The band and not
//! a hundred, because a real grid and a slow edge give 91 to 109 a second without being lost; over the 452 544 seconds of the record of 21-25.10 a count fell from alive to below
//! the band three times, and each was a loss.

use crate::LIVE_CROSSINGS;

/// What a second of the detector changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// The fronts stopped: the count of the last alive second.
    Lost { last_alive: u32 },
    /// A mains again; the seconds it was silent, the one that told it excluded.
    Returned { silent_s: u32 },
}

#[derive(Clone, Copy, Debug)]
enum State {
    /// No second has been in the band yet.
    NeverAlive,
    /// The last second in the band, with its count, and how many seconds in a row the pin has been in it.
    Alive { last: u32, run: u32 },
    /// The seconds below the band so far.
    Silent { seconds: u32 },
}

/// Whether the pin gives a mains, from the count of each second.
#[derive(Clone, Copy, Debug)]
pub struct Fronts {
    state: State,
}

impl Fronts {
    pub const fn new() -> Self {
        Self { state: State::NeverAlive }
    }

    /// How long the pin has been silent, in seconds, while it is: what the loss is told by once and the automaton and the page are told by every second. None while it gives a mains, and when it never did.
    pub fn silent_s(&self) -> Option<u32> {
        match self.state {
            State::Silent { seconds } => Some(seconds),
            _ => None,
        }
    }

    /// How many seconds in a row the pin has given a mains, this one included, while it does: what a fault that waits for the fronts to come back is let go by. None while it is silent, and when it never gave one.
    pub fn alive_s(&self) -> Option<u32> {
        match self.state {
            State::Alive { run, .. } => Some(run),
            _ => None,
        }
    }

    /// The crossings counted in one more second. -> the change, when this second made one.
    pub fn second(&mut self, up: u32) -> Option<Change> {
        let alive = LIVE_CROSSINGS.contains(&up);
        match (self.state, alive) {
            (State::NeverAlive, false) => None,
            (State::NeverAlive, true) => {
                self.state = State::Alive { last: up, run: 1 };
                None
            }
            (State::Alive { run, .. }, true) => {
                self.state = State::Alive { last: up, run: run.saturating_add(1) };
                None
            }
            (State::Alive { last, .. }, false) => {
                self.state = State::Silent { seconds: 1 };
                Some(Change::Lost { last_alive: last })
            }
            (State::Silent { seconds }, false) => {
                self.state = State::Silent { seconds: seconds.saturating_add(1) };
                None
            }
            (State::Silent { seconds }, true) => {
                self.state = State::Alive { last: up, run: 1 };
                Some(Change::Returned { silent_s: seconds })
            }
        }
    }
}

impl Default for Fronts {
    fn default() -> Self {
        Self::new()
    }
}
