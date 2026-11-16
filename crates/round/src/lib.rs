//! A round of the device - or any task of the board -, started so that the board can account for it: with a NAME the kernel knows and a
//! stack that is not a guess.
//!
//! **`std::thread::Builder::name` DOES NOT REACH FREERTOS.** It names the Rust thread, and the task underneath
//! keeps the pthread default - which is why every round of the burner appeared as `pthread` in the board's own
//! listing. On 21.09 the board fell twice with `A stack overflow in task pthread has been detected`, and there
//! was no way to tell from that which round had eaten its stack: five tasks shared the name.
//!
//! **AND THE STACK WAS A GUESS.** The rounds were started at 4096 bytes, copied from one to the next. Measured
//! on the live board seventeen seconds after a start, before any load: three of them had 284, 300 and 368 bytes
//! left of that. Rust's `format!` is what eats it - a round that writes one line of words costs several hundred
//! bytes of stack in `core::fmt` alone - and a round that only just fits today falls the day a line is added to
//! it. What a task has never used is readable at any time: `status`, the line "least stack ever free".

use std::ffi::{CStr, CString};
use std::sync::Mutex;

use esp_idf_svc::sys;

/// What a round of readings or of holding is given, unless it asks for more.
///
/// Twice what the rounds ran on, and the number is from the measurement above rather than from taste: a margin
/// of three hundred bytes is not a margin, and the cost of this one is 4 KB of a heap that has a hundred free.
pub const STACK: usize = 8192;

/// The most a value may weigh and still be built where it is used - on the stack of a task - before it goes to the
/// heap. `Box::new(x)`, `Arc::new(x)` and `Box::pin(x)` build X first where the call stands and copy it after: 5 KB of
/// the net loop's future and 5.4 KB of the regulator's rings overflowed a task of [`STACK`] at
/// start that way. Anything heavier is born in the heap (`Box::new_uninit`, `curve::Ring::boxed`), and the owner
/// whose size a task carries holds itself to this at build time: `const _: () = assert!(size_of::<T>() <= ..)`.
pub const ON_STACK_AT_MOST: usize = 1024;

/// The most a future may weigh on its way into its box when the box is made once, at start, before the task's
/// stack is used for anything else: the tasks of the net loop (web::serve). A future cannot be made in the heap - the
/// call of an async fn returns it where the call stands - so each passes through the stack of `net` (14336 when it was measured; 16896 since 9.28) once, while
/// that stack holds only `run` and `serve`. The heaviest is a page handler, 2288 bytes on the board (
/// `usb_boot_2515.log`); 3072 is it and 784 of room for it to grow, a fifth of the stack, and the stack's measured
/// use under load (12332) is reached later and never with one of these on it.
pub const BOXED_ON_START: usize = 3072;

/// The core a task is born on, and stays on (the owner's word 25.09). There is no "wherever": a task the
/// scheduler moves between the cores lands beside the network when the network is busiest - the falls of the burner on
/// 24.09 by the task watchdog were on core 0, in the thread of lwip. ESPHome keeps its application on core 1 the same
/// way (`components/esp32/core.cpp`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Core {
    /// Core 0: Wi-Fi and lwip (IDF puts them there, sdkconfig.defaults), the network's loop and its reactor, `orders`,
    /// the owner of the flash, the USB console's bridge.
    Net,
    /// Core 1: what keeps time with the hardware - the zero crossings and the gate, the holding of the draught, the
    /// automaton, the relays, the sensors' rounds. An interrupt is served on the core of the task that installs it.
    Hardware,
}

impl Core {
    /// The number IDF knows the core by.
    pub fn number(self) -> i32 {
        match self {
            Core::Net => 0,
            Core::Hardware => 1,
        }
    }
}

/// Starts F as a task called NAME with STACK bytes on CORE, and answers Err if the task could not be made.
///
/// The name is what the board prints when it reports a fall or lists its stacks, so it is the name a person
/// would use for the round: `holding`, `draught`, `flue`.
pub fn round(name: &str, stack: usize, core: Core, f: impl FnOnce() + Send + 'static) -> Result<(), sys::EspError> {
    let made = named(name, stack, core, || std::thread::Builder::new().stack_size(stack).spawn(f))?;
    made.map(|_| ()).map_err(|_| sys::EspError::from_infallible::<{ sys::ESP_ERR_NO_MEM }>())
}

/// Runs START, and a thread it starts is the task NAME with STACK bytes on CORE - for a thread started inside somebody
/// else's code, which calls `std::thread::Builder` itself: the reactor of `async-io-mini` starts its own on the
/// first socket it is given, and would otherwise be one more `pthread` in the board's listing.
///
/// The setting is the calling thread's own (`esp_pthread_set_cfg` keeps it per thread), so no other task's
/// threads are named by it; it is put back before this returns.
pub fn named<R>(name: &str, stack: usize, core: Core, start: impl FnOnce() -> R) -> Result<R, sys::EspError> {
    let named = kept(name)?;
    let mut cfg = unsafe { sys::esp_pthread_get_default_config() };
    cfg.stack_size = stack;
    cfg.thread_name = named.as_ptr();
    cfg.pin_to_core = core.number();
    // inherit_cfg stays false on purpose: a round that starts a helper of its own should say what THAT needs,
    // not silently hand it this round's name and stack.
    cfg.inherit_cfg = false;
    sys::esp!(unsafe { sys::esp_pthread_set_cfg(&cfg) })?;
    let started = start();
    // whatever happens next, the next thread must not inherit this one's name
    let back = unsafe { sys::esp_pthread_get_default_config() };
    unsafe { sys::esp_pthread_set_cfg(&back) };
    Ok(started)
}

/// The names given so far, each once.
static NAMES: Mutex<Vec<&'static CStr>> = Mutex::new(Vec::new());

/// NAME as the kernel keeps it: a C string that outlives the task, since the kernel keeps only the pointer.
///
/// **ONE ALLOCATION PER NAME, NOT PER START.** A task started on every request - the page's "after the answer has
/// left" - would otherwise leave its name behind in the heap each time, and this board runs out of heap before it runs
/// out of anything else (research 6.4).
fn kept(name: &str) -> Result<&'static CStr, sys::EspError> {
    let mut names = NAMES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(known) = names.iter().find(|known| known.to_bytes() == name.as_bytes()) {
        return Ok(known);
    }
    let new: &'static CStr = CString::new(name)
        .map(|c| &*Box::leak(c.into_boxed_c_str()))
        .map_err(|_| sys::EspError::from_infallible::<{ sys::ESP_ERR_INVALID_ARG }>())?;
    names.push(new);
    Ok(new)
}
