//! HERE: the fault for want of fronts let go of by the board: the count of the day kept on the flash, the believed time of day, and the line of the black box for every one. The rule - which fault, how long the pin must have been back, how many a day - is crates/automaton's (`Relief`).
//! NEVER HERE: a press of Stop (the thermostat's switch stays where it was), a word for a person, any other fault.
//! CHECKS: crates/automaton/tests/rows.rs for the rule; on the board, the line of the journal.

use std::sync::atomic::{AtomicU32, Ordering};

use ::automaton::{Found, Relief, RELIEF_AFTER_S, RELIEF_PER_DAY, RELIEF_RETRY_S, State};
use burner_api::automaton::ReliefNow;
use pellets::Watch;

/// Where the count of the day lies between restarts: in the board's /data, which a kit's upload never touches. A few bytes, written at most three times a day.
const STATE: &str = "/data/relief/state";

/// The count with the judge of the time of day it is kept by.
pub struct Letting {
    relief: Relief,
    watch: Watch,
}

/// The count as the flash holds it. The rule of what the answer means - no file, a count, a file that is not one, a flash that would not give it - is the automaton's ([`Relief::from_flash`]); here the answer is only passed on, and a count in doubt is said.
pub fn open() -> Letting {
    let mut text = Vec::new();
    let read = vfs::read(STATE, |chunk: &[u8]| -> Result<(), ()> {
        text.extend_from_slice(chunk);
        Ok(())
    });
    let found = match read {
        Ok(Ok(_)) => Found::Bytes(&text),
        Ok(Err(())) => Found::Unreadable("the read stopped".into()),
        Err(vfs::Error(-2)) => Found::Nothing,
        Err(other) => Found::Unreadable(format!("not read: {other}")),
    };
    let (relief, doubt) = Relief::from_flash(found);
    if let Some(why) = doubt {
        log::warn!(
            target: firmware::BOX_FRONTS,
            "relief: {STATE} is not a count to trust ({why}); the {RELIEF_PER_DAY} let-goes of the day count as spent, and a fault for want of fronts waits for a person until the day turns"
        );
    }
    Letting { relief, watch: Watch::new() }
}

/// Nothing to say: the sentinel of a value that is not there.
const NONE: u32 = u32::MAX;

/// What the card of the automaton says of the let-go this second, set by the round that judges it and read by the one that publishes (a word each, so one read never sees half of a number).
static LEFT_S: AtomicU32 = AtomicU32::new(NONE);
static TODAY: AtomicU32 = AtomicU32::new(NONE);

/// The seconds the fault still owes before it is let go of, and the faults let go of today, as the last second left them; for the card.
pub fn now() -> ReliefNow {
    let (left_s, today) = (LEFT_S.load(Ordering::Relaxed), TODAY.load(Ordering::Relaxed));
    ReliefNow { left_s: (left_s != NONE).then_some(left_s), today: (today != NONE).then_some(today) }
}

impl Letting {
    /// One second: whether the fault is to be let go of now (the number of this one today). The count is written down BEFORE the caller lets go and a refusal of the flash lets nothing go (the rule is the automaton's), because a let-go that was not counted is one more than the day allows.
    /// The card is told what a person needs while the fault waits, and the journal says once when it waits for a person because the day's let-goes are spent.
    pub fn second(&mut self, state: State, alive_s: Option<u32>) -> Option<u64> {
        let now = crate::modules::pellets::believed(&mut self.watch);
        let number = self.relief.second(state, alive_s, now, |text| match firmware::flash_owner::write(STATE, text.as_bytes()) {
            Ok(()) => true,
            Err(why) => {
                log::warn!(target: firmware::BOX_FRONTS, "fault no_fronts not let go: the count of the day was not written to {STATE} ({why}); the board offers it again in {RELIEF_RETRY_S} s");
                false
            }
        });
        LEFT_S.store(self.relief.left_s(state, alive_s).unwrap_or(NONE), Ordering::Relaxed);
        TODAY.store(self.relief.today().map_or(NONE, |n| n as u32), Ordering::Relaxed);
        if let Some(spent) = self.relief.stays(state, alive_s) {
            log::warn!(
                target: firmware::BOX_FRONTS,
                "fault no_fronts stays: the fronts held {} s in a row, and {spent} of {RELIEF_PER_DAY} let go today are spent; it waits for a person",
                alive_s.unwrap_or(RELIEF_AFTER_S)
            );
        }
        let number = number?;
        log::warn!(
            target: firmware::BOX_FRONTS,
            "fault no_fronts let go by itself: the fronts held {} s in a row; number {number} of {RELIEF_PER_DAY} today",
            alive_s.unwrap_or(RELIEF_AFTER_S)
        );
        Some(number)
    }
}
