//! What the board has written of the count and when to write it again - without the flash. The module takes the snapshot, writes it, and tells this what the flash answered.
//!
//! **WRITTEN IS WHAT THE FLASH TOOK**: a snapshot taken is not a snapshot written, and the count stays owed until `finish` hears that the write went through. A refusal leaves it owed and
//! tried again after [`RETRY_MS`].
//! What cannot be had again - the median of a burn that ended, the zero of a cleaning - is written at once; the grams, which a restart loses by a few minutes at worst, wait for [`SAVE_EVERY_MS`].

use tally::{Keeper, Snapshot};

use crate::Soot;

/// How often a count whose grams changed goes to flash.
pub const SAVE_EVERY_MS: u64 = 300_000;
/// How soon a write the flash refused is tried again.
pub const RETRY_MS: u64 = 10_000;

/// A snapshot taken and what the count was when it was taken.
#[derive(Clone, Debug)]
pub struct Pending {
    /// The text of the count and its place in the order snapshots were taken (`tally::Keeper`): a snapshot taken before another never lands after it.
    pub snapshot: Snapshot,
    changes: u64,
    urgent: u64,
}

/// What the flash holds of the count, as the flash answered, and when it was last asked.
#[derive(Clone, Debug)]
pub struct Saving {
    saved_changes: u64,
    saved_urgent: u64,
    tried_at_ms: u64,
    refused: bool,
}

impl Saving {
    /// For a count taken back from the flash at AT_MS: what it holds is what the flash held.
    pub fn new(soot: &Soot, at_ms: u64) -> Saving {
        Saving { saved_changes: soot.changes, saved_urgent: soot.urgent, tried_at_ms: at_ms, refused: false }
    }

    /// Whether the count has something the flash has not taken: a restart now would lose it.
    pub fn owed(&self, soot: &Soot) -> bool {
        soot.changes != self.saved_changes
    }

    /// The snapshot to write now, or None when nothing is due: the count moved, and the interval is gone since the last try - none for what cannot be had again - or, after a refusal, the retry term.
    pub fn due(&mut self, keeper: &Keeper, soot: &Soot, at_ms: u64) -> Option<Pending> {
        if !self.owed(soot) {
            return None;
        }
        let wait = if self.refused {
            RETRY_MS
        } else if soot.urgent != self.saved_urgent {
            0
        } else {
            SAVE_EVERY_MS
        };
        if at_ms.saturating_sub(self.tried_at_ms) < wait {
            return None;
        }
        Some(self.take(keeper, soot, at_ms))
    }

    /// The snapshot of a count just changed by a person - a cleaning -, taken at once whatever is due, for a caller that writes it through the desk of the flash and answers the person by what the flash said.
    pub fn now(&mut self, keeper: &Keeper, soot: &Soot, at_ms: u64) -> Pending {
        self.take(keeper, soot, at_ms)
    }

    fn take(&mut self, keeper: &Keeper, soot: &Soot, at_ms: u64) -> Pending {
        self.tried_at_ms = at_ms;
        Pending { snapshot: keeper.take(soot.snapshot()), changes: soot.changes, urgent: soot.urgent }
    }

    /// What the flash answered to PENDING: only a write it took makes the count written, and what changed after the snapshot was taken stays owed.
    pub fn finish(&mut self, pending: &Pending, written: bool) {
        if !written {
            self.refused = true;
            return;
        }
        self.refused = false;
        if pending.changes >= self.saved_changes {
            self.saved_changes = pending.changes;
            self.saved_urgent = pending.urgent;
        }
    }
}
