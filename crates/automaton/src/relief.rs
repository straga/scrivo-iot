//! When the board lets go of a fault by itself, and how often it may in a local day.
//!
//! **ONE FAULT, AND ONLY THE ONE THAT ASKS FOR NOTHING BUT THE FRONTS TO COME BACK.** `Why::NoFronts` is the zero-crossing pin giving nothing; the fire was put out for it and the pot was blown clear, and what is left to wait for is the pin. Once it has given a mains for
//! [`RELIEF_AFTER_S`] seconds in a row (the run is `dimmer::Fronts::alive_s`, the one place that says a loss and a return) the fault is let go of and the burner stands in a stop, to be lit by the room as it is after any stop. Every other fault - no flame, no
//! draught, a silent sensor, the water too hot - is left by a person only: lighting again over those piles pellets in the pot, and the board cannot tell from here that the cause went.
//!
//! **THREE A LOCAL DAY.** A pin that comes back and goes again is a pin worth a person's look: each let-go lights a fire that may go out for the same reason, and three of them in a day is where the board stops trying. The count is the day's, kept like the other counts of the day
//! ([`tally::Tally`]) and as text that outlives a restart ([`Relief::text`]), so a board that restarts does not get three more. A board that does not believe the time of day does not know whose day it is, and lets nothing go: the fault waits for a person, as it always did.
//!
//! **IN DOUBT, TOWARDS THE PERSON.** A file that is not a count, or that the flash would not give, is a day with its three spent ([`Relief::from_flash`]); a let-go whose new count the flash would not take is no let-go. Either way the fault waits for a person until the day turns.
//!
//! No clock here and no flash: the second, the run of the pin, the believed time and what the flash answered are handed in.

use crate::{State, Why};

/// How many seconds in a row the pin must give a mains before the fault for want of fronts is let go.
pub const RELIEF_AFTER_S: u32 = 60;

/// How many faults the board lets go of by itself in one local day.
pub const RELIEF_PER_DAY: u64 = 3;

/// How many seconds a count the flash refused waits before it is offered again: the pause the pellets keep for theirs.
pub const RELIEF_RETRY_S: u32 = (pellets::RETRY_MS / 1000) as u32;

/// What the header of its text says it is.
const HEADER: &str = "relief";

/// What the board found at the file of the count.
pub enum Found<'a> {
    /// There is no file: the board has let go of nothing yet.
    Nothing,
    /// The file, as bytes.
    Bytes(&'a [u8]),
    /// The flash did not give the file; why.
    Unreadable(String),
}

/// The faults let go of today, with the day they belong to.
pub struct Relief {
    tally: tally::Tally<1>,
    /// Whether the fault that stays has been told already ([`Relief::stays`]).
    told: bool,
    /// Seconds left of the pause after a refusal of the flash ([`RELIEF_RETRY_S`]); the count is not offered before it is nought.
    pause_s: u32,
}

impl Relief {
    pub fn new() -> Self {
        Self { tally: tally::Tally::default(), told: false, pause_s: 0 }
    }

    /// The count of a board that cannot tell what it has let go of today: the day's three are spent, on whichever day the time is first believed, and the fault waits for a person until the day turns. A count in doubt is read towards the person.
    fn spent() -> Self {
        let mut relief = Self::new();
        relief.tally.sums.today = [RELIEF_PER_DAY];
        relief
    }

    /// One second: the state the burner stands in, how long the pin has given a mains in a row (None - it does not), and the believed time of day. -> Some(number of this let-go today) when the fault is to be let go of NOW; the caller lets go
    /// ([`crate::Burning::clear_fault`]). WRITE takes the new text of the count to the flash and says whether the flash took it: where it did not, nothing is let go and nothing is counted, and the count is offered again after [`RELIEF_RETRY_S`] seconds (a flash that refuses is not asked every second).
    pub fn second(&mut self, state: State, alive_s: Option<u32>, now: Option<tally::Now>, write: impl FnOnce(&str) -> bool) -> Option<u64> {
        self.tally.at(now);
        self.pause_s = self.pause_s.saturating_sub(1);
        let waits_for_the_fronts = state == State::Fault(Why::NoFronts);
        let back_long_enough = alive_s.is_some_and(|run| run >= RELIEF_AFTER_S);
        if !waits_for_the_fronts || !back_long_enough || !self.tally.sure_of_day() || self.tally.sums.today[0] >= RELIEF_PER_DAY || self.pause_s > 0 {
            return None;
        }
        let before = self.tally;
        self.tally.add([1]);
        // placed in its day at once: the next second must see it
        self.tally.at(now);
        if !write(&self.text()) {
            // a let-go the flash did not take down is not one: a restart would read the old count and give the day's three again
            self.tally = before;
            self.pause_s = RELIEF_RETRY_S;
            return None;
        }
        Some(self.tally.sums.today[0])
    }

    /// How many seconds of the run the fault for want of fronts still owes before it is let go: the whole run while the pin has not come back, what is left of it while it has. None where it will not be let go by the pin - any other state or fault, a day with its let-goes spent,
    /// a time of day the board does not believe - and the person is told so by the page, in other words.
    pub fn left_s(&self, state: State, alive_s: Option<u32>) -> Option<u32> {
        let owes = state == State::Fault(Why::NoFronts) && self.tally.sure_of_day() && self.tally.sums.today[0] < RELIEF_PER_DAY;
        owes.then(|| RELIEF_AFTER_S.saturating_sub(alive_s.unwrap_or(0)))
    }

    /// How many faults were let go of today; None while the board does not believe the time of day, because then it does not know whose day it is.
    pub fn today(&self) -> Option<u64> {
        self.tally.sure_of_day().then_some(self.tally.sums.today[0])
    }

    /// The fault for want of fronts that STAYS: the pin has been back the whole run and the day's let-goes are spent, so the fault waits for a person. -> Some(let-goes today) the first second it is so, and None for the rest of that fault; another fault of the kind,
    /// after a person has been through this one, is told again. What the journal says once, so a person who reads it later finds why the fire did not come back by itself.
    pub fn stays(&mut self, state: State, alive_s: Option<u32>) -> Option<u64> {
        if state != State::Fault(Why::NoFronts) {
            self.told = false;
            return None;
        }
        let spent = self.tally.sure_of_day() && self.tally.sums.today[0] >= RELIEF_PER_DAY;
        let back_long_enough = alive_s.is_some_and(|run| run >= RELIEF_AFTER_S);
        if !spent || !back_long_enough || self.told {
            return None;
        }
        self.told = true;
        Some(self.tally.sums.today[0])
    }

    /// The day and what has been let go of in it, as the text a restart reads back.
    pub fn text(&self) -> String {
        format!("{HEADER}\n{}{}", tally::day_line(self.tally.day), tally::numbers("today", &self.tally.sums.today))
    }

    /// The count the flash gave back (see [`Found`]) and what is to be said about it, if anything.
    pub fn from_flash(found: Found) -> (Self, Option<String>) {
        match found {
            Found::Nothing => (Self::new(), None),
            Found::Bytes(bytes) => match std::str::from_utf8(bytes).map_err(|e| e.to_string()).and_then(Self::parse) {
                Ok(relief) => (relief, None),
                Err(why) => (Self::spent(), Some(why)),
            },
            Found::Unreadable(why) => (Self::spent(), Some(why)),
        }
    }

    /// The count from its text, or why the text is not one.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = tally::Lines::new(text, HEADER)?;
        let day = lines.day()?;
        let today = lines.numbers::<1>("today")?;
        lines.end()?;
        let mut relief = Self::new();
        relief.tally.day = day;
        relief.tally.sums.today = today;
        Ok(relief)
    }
}

impl Default for Relief {
    fn default() -> Self {
        Self::new()
    }
}
