//! What the board has written of the count and when to write it again - without the flash. The module takes a snapshot under the count's lock, writes it outside, and tells this what the flash answered.
//!
//! **WRITTEN IS WHAT THE FLASH TOOK** (6.16, the review of 9.25): a snapshot taken is not a snapshot written, and the count stays owed until `finish` hears that the write went through. A refusal leaves it owed and
//! tried again after [`RETRY_MS`] - a reset the flash refused is written on the next try, with the burner standing still or not.

use tally::{Date, Keeper, Snapshot, Yesterday};

use crate::{Pellets, YESTERDAY};

/// How often a count that changed goes to flash.
pub const SAVE_EVERY_MS: u64 = 300_000;
/// How soon a write the flash refused is tried again.
pub const RETRY_MS: u64 = 10_000;

/// A snapshot taken and what the count was when it was taken.
#[derive(Clone, Debug)]
pub struct Pending {
    pub snapshot: Snapshot,
    changes: u64,
    day: Option<Date>,
}

/// What the flash holds of the count, as the flash answered, and when it was last asked.
#[derive(Clone, Debug)]
pub struct Saving {
    saved_changes: u64,
    saved_day: Option<Date>,
    tried_at_ms: u64,
    refused: bool,
    /// The day the file of yesterday holds, as the flash answered, and when it was last tried; the file is written when the day turns, not by the interval.
    saved_yesterday: Option<Yesterday<1>>,
    yesterday_tried_ms: u64,
    yesterday_refused: bool,
}

impl Saving {
    /// For a count taken back from the flash at AT_MS: what it holds is what the flash held.
    pub fn new(p: &Pellets, at_ms: u64) -> Saving {
        Saving {
            saved_changes: p.changes,
            saved_day: p.tally.day,
            tried_at_ms: at_ms,
            refused: false,
            saved_yesterday: p.tally.yesterday,
            yesterday_tried_ms: at_ms,
            yesterday_refused: false,
        }
    }

    /// The snapshot the round writes now, or None when nothing is due: the count moved, and a day turned or the interval is gone since the last try - the retry, after a refusal.
    pub fn due(&mut self, keeper: &Keeper, p: &Pellets, at_ms: u64) -> Option<Pending> {
        let owed = p.changes != self.saved_changes;
        let turned = p.tally.day != self.saved_day;
        let wait = if self.refused {
            RETRY_MS
        } else if turned {
            0
        } else {
            SAVE_EVERY_MS
        };
        if !owed || at_ms.saturating_sub(self.tried_at_ms) < wait {
            return None;
        }
        Some(self.take(keeper, p, at_ms))
    }

    /// Whether the count has something the flash has not taken: a restart now would lose it.
    pub fn owed(&self, p: &Pellets) -> bool {
        p.changes != self.saved_changes || p.tally.day != self.saved_day || self.yesterday_owed(p)
    }

    /// The snapshot of a count just reset, written at once.
    pub fn now(&mut self, keeper: &Keeper, p: &Pellets, at_ms: u64) -> Pending {
        self.take(keeper, p, at_ms)
    }

    fn take(&mut self, keeper: &Keeper, p: &Pellets, at_ms: u64) -> Pending {
        self.tried_at_ms = at_ms;
        Pending { snapshot: keeper.take(p.snapshot()), changes: p.changes, day: p.tally.day }
    }

    /// What the flash answered to PENDING: only a write it took makes the count written. A snapshot that was passed over for a newer one already written counts as taken - the newer holds more.
    pub fn finish(&mut self, pending: &Pending, written: bool) {
        if !written {
            self.refused = true;
            return;
        }
        self.refused = false;
        if pending.changes >= self.saved_changes || pending.day > self.saved_day {
            self.saved_changes = pending.changes;
            self.saved_day = pending.day;
        }
    }
}

/// The kept day, as taken for the file of yesterday.
#[derive(Clone, Debug)]
pub struct YesterdayPending {
    /// The text of the file to write whole; None - the file is to be taken off.
    pub text: Option<String>,
    day: Option<Yesterday<1>>,
}

impl Saving {
    /// What the round does to the file of yesterday now, or None when nothing is due: the kept day is not what the flash holds - a day turned, or a reset forgot it - and, after a refusal, [`RETRY_MS`] have passed.
    /// The text is the file written whole; no text, the file taken off. Nothing else brings it: what is counted in a day does not change what yesterday says, so the flash is written once a day.
    pub fn yesterday_due(&mut self, p: &Pellets, at_ms: u64) -> Option<YesterdayPending> {
        let day = p.tally.yesterday;
        if day == self.saved_yesterday || (self.yesterday_refused && at_ms.saturating_sub(self.yesterday_tried_ms) < RETRY_MS) {
            return None;
        }
        self.yesterday_tried_ms = at_ms;
        Some(YesterdayPending { text: day.map(|day| day.text(YESTERDAY)), day })
    }

    /// Whether the kept day is something the flash has not taken: a restart now would lose it, or bring back a day of a count that was reset.
    pub fn yesterday_owed(&self, p: &Pellets) -> bool {
        p.tally.yesterday != self.saved_yesterday
    }

    /// What the flash answered: only a write or a removal it took makes the day written.
    pub fn yesterday_finish(&mut self, pending: &YesterdayPending, written: bool) {
        self.yesterday_refused = !written;
        if written {
            self.saved_yesterday = pending.day;
        }
    }
}
