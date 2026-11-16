//! Futures that are not the page's, run by the page's loop beside its own: what a module of a device does with the network - a read of a socket on the LAN - without a task of its own.
//! HERE: the place a device leaves such a future at, before the loop starts, and the loop's way of running it.
//! NEVER HERE: what a future does (the device's module), and anything of the board: this crate sits below `firmware` and does not know it, a future comes in as a value.
//! CHECKS: tests/outside.rs.
//!
//! **A FUTURE HERE HAS THE LOOP'S ONE TASK, so it waits with `await` and never blocks:** a wait on a std socket or `thread::sleep` would hold the page for as long as it lasted. What it may wait on is the loop's own
//! reactor and timers: edge-nal-std's sockets, `embassy_time`.
//!
//! **ONE THAT ENDS DOES NOT END THE LOOP.** The loop ends when any of its tasks ends (`serve`), and a read of a socket that returned would take the page and the console with it; an errand that ends is said in the log and then waits for good.

use core::future::Future;
use core::pin::Pin;
use std::sync::Mutex;

/// A future of the loop, born once and run for good.
pub type Errand = Pin<Box<dyn Future<Output = ()> + 'static>>;

/// The errands left before the loop started; None - it has started, and a place is no longer left.
static LEFT: Mutex<Option<Vec<fn() -> Errand>>> = Mutex::new(Some(Vec::new()));

/// The loop has started: an errand can no longer be left.
#[derive(Debug, PartialEq, Eq)]
pub struct Late;

/// Leaves START to be run by the loop when it starts. START makes the future where it is called - on the stack of the task of the loop, one at a time -, so it makes it by [`boxed`], which on the board refuses at build one
/// heavier than a task's stack takes.
pub fn add(start: fn() -> Errand) -> Result<(), Late> {
    LEFT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_mut().map(|left| left.push(start)).ok_or(Late)
}

/// FUTURE in a box, as an errand. On the board the build refuses a future heavier than `round::BOXED_ON_START`, as for the loop's own tasks.
pub fn boxed<F: Future<Output = ()> + 'static>(future: F) -> Errand {
    #[cfg(target_os = "espidf")]
    let future = between::fits::<_, { round::BOXED_ON_START }>(future);
    Box::pin(future)
}

/// The errands left, taken once as the loop starts; after this [`add`] answers `Late`.
pub(crate) fn taken() -> Vec<fn() -> Errand> {
    LEFT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take().unwrap_or_default()
}

/// ERRAND, and then for good if it ends.
pub(crate) async fn for_good(errand: Errand) {
    errand.await;
    log::error!("an errand of the loop ended: it waits for good, and what it did is not done any more");
    core::future::pending::<()>().await
}
