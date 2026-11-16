//! The leaps of the tick's counter, as the interrupt keeps them, looked at once in a while: what has come since the last look.
//!
//! The leap is counted in the interrupt of the tick (crates/firmware/systick/systick.c), where nothing may call a line of the log, so the line is the board's to write at its next look. The
//! look is a number and not a clock: it tells how many came, the size and the way of the last one, and the moment of the line is the moment of the look - a second at the most after the leap.

/// What the interrupt keeps (systick.c, `systick_leaps_t`), in the units it keeps: ticks, a millisecond each at 1000 Hz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickLeaps {
    pub count: u32,
    /// The size of the last leap, in ticks, and whether it went back.
    pub last_ticks: u32,
    pub last_was_back: bool,
    pub largest_back: u32,
    pub largest_forward: u32,
    pub rearms: u32,
}

/// What has come since the last look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leap {
    /// How many leaps since the last look.
    pub new: u32,
    /// The last of them: its size in ticks and its way.
    pub last_ticks: u32,
    pub last_was_back: bool,
}

/// The count as it stood at the last look.
pub struct Watch {
    seen: u32,
}

impl Watch {
    pub const fn new() -> Self {
        Self { seen: 0 }
    }

    /// Looks at the count NOW. -> the leaps since the last look, None when there are none. A count below the last look is a count that began again: all of it is new.
    pub fn look(&mut self, now: TickLeaps) -> Option<Leap> {
        let new = now.count.checked_sub(self.seen).unwrap_or(now.count);
        self.seen = now.count;
        (new > 0).then_some(Leap { new, last_ticks: now.last_ticks, last_was_back: now.last_was_back })
    }
}

impl Default for Watch {
    fn default() -> Self {
        Self::new()
    }
}
