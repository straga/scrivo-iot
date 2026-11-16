//! How long a task runs without falling asleep: what the watchdog of its core sees. The idle task of a core feeds the
//! task watchdog, and it gets the core only while every task of that core is asleep; a loop that runs on for seconds
//! has the board reset (6.6, a device in service on 26.09).
//!
//! A step is from the moment the task wakes to the moment it sleeps again. Polls with a wake already waiting between
//! them are one step: the task does not sleep there. What a step costs is the CPU the task took in it, not the time
//! on the clock: a task that waits inside a poll for a lock another task holds sleeps there, and its core is free -
//! counted by the clock, such a wait read as a step of 221 ms on `.30` (26.09) while the core was the flash's.

use core::future::Future;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::{Context, Poll};

/// The longest step of one task since it started, and when it ended.
pub struct Steps {
    longest_us: AtomicU32,
    at_ms: AtomicU32,
}

impl Steps {
    pub const fn new() -> Self {
        Steps { longest_us: AtomicU32::new(0), at_ms: AtomicU32::new(0) }
    }

    /// The longest step in microseconds of CPU, and the time since start in milliseconds when it ended.
    pub fn longest(&self) -> (u32, u32) {
        (self.longest_us.load(Ordering::Relaxed), self.at_ms.load(Ordering::Relaxed))
    }

    /// A step that took US of CPU has ended.
    fn ended(&self, us: u32) {
        if us > self.longest_us.load(Ordering::Relaxed) {
            self.longest_us.store(us, Ordering::Relaxed);
            let now = crate::clock::now();
            self.at_ms.store(now.as_millis().min(u32::MAX as u64) as u32, Ordering::Relaxed);
        }
    }
}

impl Default for Steps {
    fn default() -> Self {
        Self::new()
    }
}

/// The CPU this task has taken since it started, in microseconds, as FreeRTOS counts it at every switch of tasks
/// (CONFIG_FREERTOS_GENERATE_RUN_TIME_STATS, by esp_timer): the time since the task was last switched in is not in it
/// yet, so a step is read after the sleep that ends it. FreeRTOS 10.5 of IDF 5.5 hands the counter out only in
/// vTaskGetInfo; the state is given, so it is not looked up, and the stack is not walked.
#[cfg(target_os = "espidf")]
fn cpu_us() -> u32 {
    use esp_idf_svc::sys;
    let mut status: sys::TaskStatus_t = unsafe { core::mem::zeroed() };
    unsafe { sys::vTaskGetInfo(sys::xTaskGetCurrentTaskHandle(), &mut status, 0, sys::eTaskState_eRunning) };
    status.ulRunTimeCounter as u32
}

/// The CPU this thread has taken, in microseconds.
#[cfg(not(target_os = "espidf"))]
fn cpu_us() -> u32 {
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t) };
    (t.tv_sec as u64 * 1_000_000 + t.tv_nsec as u64 / 1_000) as u32
}

/// [`crate::block_on`] that writes the CPU of every step of this task into STEPS, and puts the task to sleep for a
/// tick once a step has run BUDGET by the clock though a wake waits: a task always woken again before it sleeps would
/// otherwise hold its core for as long as the wakes keep coming.
#[cfg(target_os = "espidf")]
pub fn block_on_stepped<F: Future>(fut: F, steps: &Steps, budget: core::time::Duration) -> F::Output {
    use esp_idf_svc::hal::task::notification::Notification;

    let notification = Notification::new();
    let waker = notification.notifier().into();
    let mut cx = Context::from_waker(&waker);
    let mut fut = core::pin::pin!(crate::fits::<_, { round::ON_STACK_AT_MOST }>(fut));
    let (mut from, mut cpu) = (crate::clock::now(), cpu_us());
    loop {
        if let Poll::Ready(out) = fut.as_mut().poll(&mut cx) {
            return out;
        }
        // woken while it ran: polled again at once, the same step - till the step reaches the budget
        if notification.wait(0).is_some() {
            if from.elapsed() < budget {
                continue;
            }
            // the wake taken above is kept by polling once more after the tick
            esp_idf_svc::hal::delay::FreeRtos::delay_ms(1);
        } else {
            notification.wait_any();
        }
        steps.ended(cpu_us().wrapping_sub(cpu));
        (from, cpu) = (crate::clock::now(), cpu_us());
    }
}

/// [`crate::block_on`] that writes the CPU of every step of this thread into STEPS, and puts the thread to sleep for a
/// millisecond once a step has run BUDGET by the clock though a wake waits - as the board's does.
#[cfg(not(target_os = "espidf"))]
pub fn block_on_stepped<F: Future>(fut: F, steps: &Steps, budget: core::time::Duration) -> F::Output {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::task::Wake;

    struct Unpark {
        thread: std::thread::Thread,
        woken: AtomicBool,
    }
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.woken.store(true, Ordering::Release);
            self.thread.unpark();
        }
    }
    let unpark = Arc::new(Unpark { thread: std::thread::current(), woken: AtomicBool::new(false) });
    let waker = unpark.clone().into();
    let mut cx = Context::from_waker(&waker);
    let mut fut = core::pin::pin!(fut);
    let (mut from, mut cpu) = (crate::clock::now(), cpu_us());
    loop {
        if let Poll::Ready(out) = fut.as_mut().poll(&mut cx) {
            steps.ended(cpu_us().wrapping_sub(cpu));
            return out;
        }
        if unpark.woken.swap(false, Ordering::Acquire) {
            if from.elapsed() < budget {
                continue;
            }
            steps.ended(cpu_us().wrapping_sub(cpu));
            std::thread::sleep(core::time::Duration::from_millis(1));
        } else {
            steps.ended(cpu_us().wrapping_sub(cpu));
            while !unpark.woken.swap(false, Ordering::Acquire) {
                std::thread::park();
            }
        }
        (from, cpu) = (crate::clock::now(), cpu_us());
    }
}
