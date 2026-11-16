//! The planned restart of the board and the note a silent draught leaves. The board hands in what it knows this second; this says what to do, so all of it is tested on the computer.

use schedule::Wall;

/// What the board knows this second.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Moment {
    /// The local time, None while the clock is not sure of it.
    pub wall: Option<Wall>,
    /// How long the board has been up, in seconds.
    pub up_s: u64,
    /// The burner stands with no fire: the state is a stop, the flame is dark, the auger and the plug are off.
    pub standing: bool,
    /// The burner is in a fault: it waits for a person, and the board does not restart under it.
    pub fault: bool,
    /// The count of pellets has something the flash has not taken.
    pub owed: bool,
    /// The relay of the auger is closed now, whoever closed it: the automaton, a hand on the page, the console.
    pub auger_closed: bool,
    /// The relay of the plug is closed now, whoever closed it.
    pub igniter_closed: bool,
}

/// Why a restart that is due does not happen yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Why {
    /// The burner is not standing with no fire.
    Fire,
    /// The burner is in a fault: the fault waits for a person.
    Fault,
    /// The flash has not taken the count yet.
    CountOwed,
    /// A relay of the auger or the plug is closed, whatever the automaton decided: what is on is a fact, and a decision is not.
    Organ,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// Nothing is due.
    Not,
    /// A restart is due and waits.
    Waits(Why),
    /// The board restarts now.
    Restart,
}

/// The restart once a day, from a minute of the local day.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Daily {
    pub from_minute: u16,
}

/// A board that started less than this long before today's hour is not due today: it restarted at the hour (a restart takes seconds, the clock a minute), or was started by hand a moment before it. Ten minutes, so the board that restarted at the hour is not due again until the next night, however long it has been up since.
pub const STARTED_BEFORE_S: u64 = 600;
/// With no clock, the board restarts when it has been up this long.
pub const BLIND_UP_S: u64 = 24 * 3600;

impl Daily {
    /// What to do this second: a restart is due from the set minute of the day (from the uptime alone while the clock is not sure), unless the board started after today's hour began, which is a restart that has happened; it happens when the burner stands with no fire, no relay of the auger or the plug is closed (whoever closed it) and the count is on the flash, and waits otherwise. A fault is not waited out: it waits for a person.
    pub fn judge(&self, at: Moment) -> Verdict {
        let due = match at.wall {
            Some(wall) => {
                let since_hour_s = u64::from(wall.minute.saturating_sub(self.from_minute)) * 60;
                wall.minute >= self.from_minute && at.up_s >= since_hour_s + STARTED_BEFORE_S
            }
            None => at.up_s >= BLIND_UP_S,
        };
        if !due {
            return Verdict::Not;
        }
        if at.fault {
            return Verdict::Waits(Why::Fault);
        }
        if !at.standing {
            return Verdict::Waits(Why::Fire);
        }
        if at.auger_closed || at.igniter_closed {
            return Verdict::Waits(Why::Organ);
        }
        if at.owed {
            return Verdict::Waits(Why::CountOwed);
        }
        Verdict::Restart
    }
}

/// The silence, in seconds, past which the draught's round leaves a note.
pub const SILENT_PAST_S: u32 = 10;

/// The hour a board restarts from when the setting says nothing a hour can be made of.
const DEFAULT_HOUR: u16 = 3;

/// The hour the setting names, or that there is none.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hour {
    Off,
    /// A minute of the local day.
    At(u16),
}

impl Hour {
    /// The setting's text as an hour: `off`, a number of the hours (past the last one is the last), or - a text that is
    /// neither - the default hour, three: a mistyped setting does not switch the restart off.
    pub fn of(text: &str) -> Hour {
        match text.trim() {
            "off" => Hour::Off,
            other => Hour::At(other.parse::<u16>().map_or(DEFAULT_HOUR, |hour| hour.min(23)) * 60),
        }
    }

    /// What to do this second at this hour: with `off`, nothing is ever due.
    pub fn judge(self, at: Moment) -> Verdict {
        match self {
            Hour::Off => Verdict::Not,
            Hour::At(from_minute) => Daily { from_minute }.judge(at),
        }
    }
}

/// Counts a silence of the draught's round into cases: one note for each.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Clues {
    noted: bool,
}

impl Default for Clues {
    fn default() -> Self {
        Clues::default_const()
    }
}

impl Clues {
    /// For a static: the same as `default`, which is not a `const fn`.
    pub const fn default_const() -> Clues {
        Clues { noted: false }
    }
}

impl Clues {
    /// The seconds the round has been silent (None while it speaks): whether a note is to be written now.
    pub fn see(&mut self, silent_s: Option<u32>) -> bool {
        match silent_s {
            Some(s) if s > SILENT_PAST_S => !std::mem::replace(&mut self.noted, true),
            Some(_) => false,
            None => {
                self.noted = false;
                false
            }
        }
    }
}

/// Why a restart that was asked for does not go yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Postponed {
    /// The journal did not go down to the flash: its lines wait in memory, and a restart would lose them.
    Journal,
    /// A relay of the auger or the plug closed after the restart was asked for.
    Organ,
}

/// What the owner of the flash does with a restart that was asked for, this pass.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Settled {
    /// Nothing was asked for.
    Idle,
    /// The board restarts now.
    Go,
    /// It does not, and the next pass asks again. NEW is whether this is a different reason from the pass before, so that the journal says it once and not every second.
    Waits { why: Postponed, new: bool },
}

/// A restart asked for and not yet carried out.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Request {
    asked: bool,
    waited: Option<Postponed>,
}

impl Request {
    pub const fn new() -> Request {
        Request { asked: false, waited: None }
    }

    pub fn ask(&mut self) {
        self.asked = true;
    }

    /// The reason to restart is gone: nothing is asked for any more.
    pub fn cancel(&mut self) {
        *self = Request::new();
    }

    pub fn asked(&self) -> bool {
        self.asked
    }

    /// What to do with the request now that the journal was written (JOURNAL_DOWN) and a relay may be closed (ORGAN_CLOSED).
    pub fn settle(&mut self, journal_down: bool, organ_closed: bool) -> Settled {
        if !self.asked {
            return Settled::Idle;
        }
        let why = match (journal_down, organ_closed) {
            (false, _) => Postponed::Journal,
            (true, true) => Postponed::Organ,
            (true, false) => {
                self.waited = None;
                return Settled::Go;
            }
        };
        // the journal is named before a relay: a journal the flash refused is not turned into a relay by a hand that closed one meanwhile
        let new = self.waited != Some(why);
        self.waited = Some(why);
        Settled::Waits { why, new }
    }
}
