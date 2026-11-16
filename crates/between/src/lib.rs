//! What passes between the board's tasks: the raw lock under every channel -
//! `Watch`, `Channel`, `Signal` -, the wait that sleeps until woken, and an owner's desk ([`Desk`]), where the other
//! tasks ask the one task that owns some state and are answered.
//!
//! THE LOCK ON THE BOARD IS A FREERTOS MUTEX, NOT A CRITICAL SECTION. `CriticalSectionRawMutex` turns interrupts off for
//! as long as it is held, and a channel holds it while it copies a whole card; a zero-crossing interrupt
//! then comes late. `EspRawMutex` is a recursive FreeRTOS mutex (esp-idf-hal `task.rs`, `CriticalSection`):
//! interrupts stay on, a task that waits for it sleeps. The price is that an interrupt handler must never touch a
//! channel built on it - ours do not, they count into their own words (a device's zero-crossing counter: `edge`, `fired`).
//!
//! The mutex is created on the first lock, from the heap, once per channel.
//!
//! On the host, where the tests run, there is no FreeRTOS, and the critical section of std stands in.

pub mod clock;
mod desk;
pub mod nap;
pub mod steady_time;
mod steps;
pub mod tick;

pub use desk::{Desk, Refused};
pub use nap::nap;
pub use steps::{block_on_stepped, Steps};

/// Refuses at build time a value heavier than LIMIT bytes, and gives it back as it came - for a value made where the
/// call stands, on the stack of a task: a future handed to [`block_on`] lives there for as long as it runs, and one
/// that is boxed passes through there on its way. A future weighs all its awaits at once from the moment it is made;
/// the net loop's was 86 KB on a stack of 12. Nothing else sees it: the size is the compiler's to know.
///
/// ```
/// between::fits::<_, 1024>([0u8; 16]);
/// ```
/// ```compile_fail,E0080
/// between::fits::<_, 1024>([0u8; 2048]);
/// ```
pub fn fits<F, const LIMIT: usize>(value: F) -> F {
    const { assert!(core::mem::size_of::<F>() <= LIMIT, "heavier than a task's stack takes: born in the heap instead") };
    value
}

/// Runs a future to its end on this thread, asleep between polls: on the board the task waits on its FreeRTOS
/// notification (esp-idf-hal task::block_on); on the host, where the tests run, the thread parks the same way.
///
/// On the board FUT lives on this task's stack while it runs, and the build refuses one heavier than
/// `round::ON_STACK_AT_MOST` ([`fits`]); on the host the sizes are not the board's, and nothing is refused.
#[cfg(target_os = "espidf")]
pub fn block_on<F: core::future::Future>(fut: F) -> F::Output {
    esp_idf_svc::hal::task::block_on(fits::<_, { round::ON_STACK_AT_MOST }>(fut))
}

#[cfg(not(target_os = "espidf"))]
pub fn block_on<F: core::future::Future>(fut: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake};

    struct Unpark(std::thread::Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Arc::new(Unpark(std::thread::current())).into();
    let mut cx = Context::from_waker(&waker);
    let mut fut = core::pin::pin!(fut);
    loop {
        if let Poll::Ready(out) = fut.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::park();
    }
}

#[cfg(target_os = "espidf")]
pub type Lock = esp_idf_svc::hal::task::embassy_sync::EspRawMutex;

#[cfg(not(target_os = "espidf"))]
pub type Lock = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
