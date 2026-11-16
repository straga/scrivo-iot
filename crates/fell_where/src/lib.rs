//! What a recorded fall says to a person. No hardware here: the chip's panic handler writes the numbers into
//! RTC memory through the `panic_note` component of `crates/firmware`, and this file says what they mean - so
//! all of it is tested on the computer.
//!
//! **A BOARD WITH NO WIRE ON IT CANNOT SAY WHERE IT FELL, AND THAT IS THE WHOLE COST.** The reason for the
//! restart is already named (`crates/reset_why`): a panic is our code being wrong somewhere. WHERE is printed
//! by the chip's panic handler into the console it has - the USB wire - and nowhere else, so on a board with
//! the wire off, the one fact that picks the repair out of the whole firmware is the one that is lost. The note
//! this crate reads is that fact kept across the restart.

/// A fall the board wrote down for itself, as the note carries it.
#[derive(Debug, Default, Clone)]
pub struct Fall {
    /// The board's count of starts at the moment it fell; no start is numbered 0.
    pub seq: u32,
    /// `panic_exception_t` of ESP-IDF, as the chip's panic handler filled it.
    pub exception: u32,
    pub core: u32,
    /// The instruction that faulted.
    pub addr: u32,
    /// The address a faulting load or store was made at (the chip's EXCVADDR); 0 when the chip gave none.
    pub data: u32,
    /// The chip's own word for the exception, empty when it had none.
    pub reason: String,
    /// What our own code said before it gave up - a Rust panic's file, line and message. Empty for a fault,
    /// which never goes through our code at all.
    pub said: String,
    /// Return addresses up the stack of the task that fell, the faulting instruction first.
    pub trace: Vec<u32>,
    /// The other core of a two-core chip, and the addresses up ITS stack at the fall; None on a chip with one core.
    ///
    /// **HALF OF A WATCHDOG'S ANSWER IS ON THE OTHER CORE**: the fall of 24.09
    /// named core 0 idle in `WAITI` - a core that was waiting, not one that held anything - and what kept its tick
    /// away is only seen on the core the note did not keep. An empty trace is a core that never reached the panic
    /// handler, and it is said so rather than left out.
    pub other_core: Option<u32>,
    pub other_trace: Vec<u32>,
    /// Whether the image running now is the one that fell. RTC memory outlives a firmware update, and an
    /// address from the build before it points at whatever the new build put there.
    pub same_image: bool,
}

/// Which start the note belongs to.
///
/// The note outlives the fall that wrote it, so a person reading a place and an address has no way of telling
/// a fall a minute old from one a week old. This is what tells them apart.
#[derive(Debug, PartialEq, Eq)]
pub enum When {
    /// The fall that brought this start about.
    ThisStart,
    /// An earlier fall, this many starts back.
    StartsBack(u32),
    /// A note claiming a start the board has not had. RTC memory comes up as whatever the power left in it.
    Impossible,
}

/// Which start a note stamped SEQ belongs to, on a board that has had STARTS starts since its power came on.
pub fn when(seq: u32, starts: u32) -> When {
    match seq {
        0 => When::Impossible,
        seq if seq >= starts => When::Impossible,
        seq if seq == starts - 1 => When::ThisStart,
        seq => When::StartsBack(starts - seq),
    }
}

/// What kind of fall it was, and which repair that points at.
///
/// An unknown kind comes out AS THE NUMBER, the same discipline as `reset_why`: a kind invented to fit a
/// familiar word sends somebody to the wrong repair.
pub fn exception_words(exception: u32) -> String {
    match exception {
        0 => "a debug exception".into(),
        1 => "the interrupt watchdog - a core held too long with interrupts off".into(),
        2 => "the task watchdog - a task that did not let go of its core".into(),
        3 => "abort - our code gave up: a Rust panic, a failed allocation, or a check of ours that did not hold".into(),
        4 => "a fault - our code was wrong about an address or an instruction".into(),
        other => format!("a kind this firmware does not know, {other}"),
    }
}

/// The one line the board says about a fall: which start it belongs to, what kind it was, on which core, at
/// which instruction, and what our own code said if it said anything.
pub fn words(fall: &Fall, starts: u32) -> String {
    let whose = match when(fall.seq, starts) {
        When::Impossible => {
            return format!(
                "fell - but the note is not to be trusted: it claims start {}, and the board has had {starts}",
                fall.seq
            )
        }
        When::ThisStart => "fell just before this start".to_string(),
        When::StartsBack(back) => format!("fell {back} starts ago"),
    };
    let mut line = format!(
        "{whose}: {} on core {} at {:#010x}",
        exception_words(fall.exception),
        fall.core,
        fall.addr
    );
    if !fall.reason.is_empty() {
        line.push_str(&format!(" ({})", fall.reason));
    }
    // what a faulting load or store touched: a corrupted pointer, external RAM, nothing mapped at all
    if fall.exception == 4 && fall.data != 0 {
        line.push_str(&format!(", touched {:#010x}", fall.data));
    }
    if !fall.said.is_empty() {
        line.push_str(&format!(" - our code: {}", fall.said));
    }
    if !fall.same_image {
        // Said before anybody goes looking: the addresses are places in THAT build, and named against this
        // one they come out as real functions in the wrong code, reading exactly as confidently as true ones.
        line.push_str(" - from another image than the one running now, so its addresses are places in that build");
    }
    line
}

/// The addresses up the stack, and the command that turns them into files and lines. None when the walk of the
/// stack gave nothing: an empty `trace` line reads as a stack of nothing, which is not what happened.
pub fn trace_line(fall: &Fall, device: &str) -> Option<String> {
    if fall.trace.is_empty() {
        return None;
    }
    let addresses: Vec<String> = fall.trace.iter().map(|a| format!("{a:#010x}")).collect();
    Some(format!(
        "trace {} - files and lines with: tools/where_fell.py {device} <these addresses>",
        addresses.join(" ")
    ))
}

/// Everything the board says about its last fall, in the order it says it: what it was and where, then the
/// addresses up the stack of each core. `firmware::fell::lines` hands this the note it read.
pub fn lines(fall: &Fall, starts: u32, device: &str) -> Vec<String> {
    let mut said = vec![words(fall, starts)];
    said.extend(trace_line(fall, device));
    said.extend(other_line(fall, device));
    said
}

/// The other core's stack at the fall, or that it left none; None on a chip with one core.
fn other_line(fall: &Fall, device: &str) -> Option<String> {
    let core = fall.other_core?;
    if fall.other_trace.is_empty() {
        return Some(format!("core {core}: no stack - it did not reach the panic handler"));
    }
    let addresses: Vec<String> = fall.other_trace.iter().map(|a| format!("{a:#010x}")).collect();
    Some(format!(
        "core {core} was at {} - files and lines with: tools/where_fell.py {device} <these addresses>",
        addresses.join(" ")
    ))
}

/// For a device's build.rs: makes the chip's panic handler go through the `panic_note` component first, and the tick
/// of FreeRTOS be set up by the `systick` component instead of IDF's (firmware), and every timer of esp_timer
/// be made skipping what a leap of the counter made it miss (`systick/timers.c`, 10.2).
///
/// The flags belong on the FINAL link line, and cargo passes `rustc-link-arg` of a library no further than
/// that library, so every device emits them from its own build.rs - the same way each one emits its version.
pub fn emit_link_args() {
    println!("cargo:rustc-link-arg=-Wl,--wrap=esp_panic_handler");
    println!("cargo:rustc-link-arg=-Wl,--wrap=vPortSetupTimer");
    println!("cargo:rustc-link-arg=-Wl,--wrap=esp_timer_create");
}
