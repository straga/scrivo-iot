//! Starting a task that may not start: tried again, with a pause, a number of times - and every failure said, so that a task that did not come up is never a silence.
//! The tries and the pause are the caller's, with the reason for them: a number of this file would be a number of nobody's.

use std::fmt::Display;
use std::time::Duration;

/// Calls START up to TRIES times, with PAUSE between two tries (never after the last), and says each failure in one line to SAID. Answers whether it started: the first `Ok` ends the tries.
///
/// SLEEP is the pause as the caller takes it (the board sleeps its thread, a row counts), and SAID is where the line goes (the board's log, a row's list).
pub fn start_with_retries<E: Display>(
    name: &str,
    tries: u32,
    pause: Duration,
    mut start: impl FnMut() -> Result<(), E>,
    mut sleep: impl FnMut(Duration),
    mut said: impl FnMut(String),
) -> bool {
    for attempt in 1..=tries {
        match start() {
            Ok(()) => return true,
            Err(why) => said(format!("{name}: did not start (try {attempt} of {tries}): {why}")),
        }
        if attempt < tries {
            sleep(pause);
        }
    }
    false
}
