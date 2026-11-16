//! The note a fall leaves for the next start: where the board faulted and what the chip called it.
//!
//! The chip's panic handler prints the place and the backtrace into the console it has - the USB wire - and
//! then restarts, which takes the console with it. So a board with the wire off knows THAT it panicked
//! (`crates/reset_why`) and not WHERE, and where is the fact that picks the repair. The `panic_note` component
//! of this crate gets in front of that handler and writes the numbers into RTC memory, which a restart does
//! not reach; here they are read back and turned into words by `crates/fell_where`.

use std::sync::OnceLock;

use esp_idf_svc::sys::panic_note as c;
use fell_where::Fall;

static FALL: OnceLock<Option<Fall>> = OnceLock::new();
static STARTS: OnceLock<u32> = OnceLock::new();
static DEVICE: OnceLock<&'static str> = OnceLock::new();

/// Counts this start, reads the note a fall left, and puts our own hook in front of the next panic. Called
/// once, from the firmware's first lines.
pub fn rise(device: &'static str) {
    let note = unsafe { c::panic_note_rise() };
    let _ = DEVICE.set(device);
    let _ = STARTS.set(unsafe { c::panic_note_starts() });
    let _ = FALL.set(unsafe { note.as_ref() }.map(read));
    hook();
}

/// What the board recorded about its last fall, or None when it has recorded none.
pub fn fall() -> Option<&'static Fall> {
    FALL.get().and_then(|fall| fall.as_ref())
}

/// The lines the board says about its last fall: what it was and where, then the addresses up the stack of each core.
/// Empty when there is no note - a heading with nothing under it reads as a fall nobody can place.
pub fn lines() -> Vec<String> {
    let Some(fall) = fall() else {
        return Vec::new();
    };
    let starts = STARTS.get().copied().unwrap_or(0);
    let device = DEVICE.get().copied().unwrap_or("<device>");
    fell_where::lines(fall, starts, device)
}

fn read(note: &c::panic_note_t) -> Fall {
    Fall {
        seq: note.seq,
        exception: note.exception,
        core: note.core,
        addr: note.addr,
        data: note.excvaddr,
        reason: text(&note.reason),
        said: text(&note.said),
        trace: note.trace[..(note.depth as usize).min(note.trace.len())].to_vec(),
        other_core: (note.other_core != c::PANIC_NOTE_NO_CORE).then_some(note.other_core),
        other_trace: note.other_trace[..(note.other_depth as usize).min(note.other_trace.len())].to_vec(),
        same_image: note.image == unsafe { c::panic_note_image() },
    }
}

fn text(chars: &[core::ffi::c_char]) -> String {
    let bytes: Vec<u8> = chars.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Keeps a Rust panic's file, line and message for the note on its way to abort().
///
/// The chip sees none of that: from where its handler stands a Rust panic is an abort at an address, and the
/// file and line are the one thing that needs no ELF to read. The hook also logs the panic, which the hook it
/// replaces did.
fn hook() {
    std::panic::set_hook(Box::new(|info| {
        let place = info.location().map_or(String::new(), |at| format!("{}:{}:{} ", at.file(), at.line(), at.column()));
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let text = format!("{place}{message}");
        log::error!("panicked: {text}");
        // A C string, cut at a character boundary: the note keeps PANIC_NOTE_SAID bytes and a message can be
        // longer, and a message can carry a zero byte of its own, which would cut the string early.
        let mut bytes: Vec<u8> = text.bytes().take_while(|&b| b != 0).collect();
        while bytes.len() >= c::PANIC_NOTE_SAID as usize {
            bytes.pop();
        }
        bytes.push(0);
        unsafe { c::panic_note_said(bytes.as_ptr().cast()) };
    }));
}

/// Makes the board fall on purpose, so that the note can be read afterwards and the whole way proved without
/// waiting for a fall nobody asked for. HARD takes the way a hardware fault takes - a store to an address the
/// chip refuses - and the other way is our code giving up, which is where a Rust panic goes. Does not return.
pub fn on_purpose(hard: bool) -> ! {
    if hard {
        log::error!("falling on purpose: a store to address 0, the way a hardware fault goes");
        std::thread::sleep(std::time::Duration::from_millis(200));
        unsafe { core::ptr::write_volatile(core::ptr::null_mut::<u32>(), 1) };
    }
    log::error!("falling on purpose: our code giving up, the way a Rust panic goes");
    std::thread::sleep(std::time::Duration::from_millis(200));
    panic!("fell on purpose, asked for from the shell");
}
