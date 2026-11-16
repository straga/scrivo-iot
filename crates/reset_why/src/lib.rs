//! Why the chip came up, in words a person can act on. No hardware here: the code arrives from
//! `esp_reset_reason()` and this says what it means.
//!
//! **A BOARD THAT MERELY "RESTARTED" LEAVES THREE REPAIRS OPEN AT ONCE**, and they are not the same repair:
//! a brownout is wires and a power supply, a watchdog is our code holding a core too long, a panic is our code
//! being wrong somewhere. Until the reason is named, choosing between them is guessing, and every guess costs a
//! round on live iron.
//!
//! The numbers are esp_reset_reason_t of ESP-IDF 5.5 (`components/esp_system/include/esp_system.h`). They are
//! matched as numbers on purpose: the firmware half passes the code straight through, so this whole file is
//! tested on the computer.

/// What a reset code says to a person - and what to do about it, where there is anything to do.
///
/// An unknown code comes out AS THE NUMBER. A reason invented to fit a familiar word sends somebody to the
/// wrong repair, and a code we have never seen is exactly the case where nobody should trust our words.
pub fn words(code: u32) -> String {
    match code {
        0 => "reset for a reason the chip did not record, code 0".into(),
        1 => "started from power being switched on".into(),
        2 => "reset by its reset pin".into(),
        3 => "restarted by software - a command, or an image being written".into(),
        4 => "restarted after a panic: our code faulted - the log before the restart says where".into(),
        5 => "restarted by the interrupt watchdog: something held a core too long with interrupts off".into(),
        6 => "restarted by the task watchdog: a task did not let go of its core".into(),
        7 => "restarted by another watchdog of the chip".into(),
        8 => "woke from deep sleep".into(),
        9 => "restarted because the power dipped (brownout): wires, supply or a load switching".into(),
        10 => "restarted over SDIO".into(),
        11 => "restarted over USB".into(),
        12 => "restarted by JTAG".into(),
        13 => "restarted by an efuse error".into(),
        14 => "restarted because the power glitched: a spike on the supply rail".into(),
        15 => "restarted because a core locked up".into(),
        other => format!("reset for a reason this firmware does not know, code {other}"),
    }
}

/// Whether this reason is a FAULT - something to go and fix - rather than an ordinary way to start.
///
/// An unknown code is not called a fault: we do not know that it is one, and a card painting it red would be
/// asserting something nobody checked.
pub fn is_fault(code: u32) -> bool {
    matches!(code, 4..=7 | 9 | 14 | 15)
}
