//! The order the comparators of the gate are written in, so that the timer's interrupt can come between any two writes and the gate still ends stopped.
//!
//! **TWO WRITERS, ONE OF THEM AN INTERRUPT.** The task that moves the gate (`Gate::move_to`, the board's `fan_gate`) and the callback at the timer's peak (`gate_stop`) write the same two comparators and the same flag. The callback is not held off by anything a task takes: it
//! comes when the fronts have been away for [`STOP_AFTER_US`], and it can come between any two writes of the task. A flag read once before a series of writes says nothing about what is true by the last of them.
//!
//! **THE ORDER, AND WHY IT HOLDS FOR EVERY PLACE THE INTERRUPT CAN COME:**
//! 1. a gate already stopped is not touched (the caller keeps what was asked, and `resume` brings the gate to it);
//! 2. the output is HELD LOW for the whole of the write: whatever the comparators say while the task moves them, nothing comes out;
//! 3. both comparators are written;
//! 4. the flag is read AGAIN, after the last write. Set - the interrupt came in somewhere above, and the comparators may be the task's; they are put out of reach again, and the output STAYS held low (the interrupt's own write may be what the task has just covered);
//! 5. the output is let go only with the flag clear. An interrupt that comes after that read writes the comparators out of reach AFTER the task's last write: its write is the last one, and the output being let go at that moment is harmless, since there is nothing left in reach.
//!
//! So the one state that can put a pulse out at no phase - the gate let go, the fronts gone, a comparator within reach - is not reached by any order of the two writers. The row that holds it is `tests/gate_write.rs`: the interrupt before every step in turn.
//!
//! A SECOND TASK is a different matter and a different tool: two tasks writing the pair at once could leave one comparator of each, and `Gate` takes one lock for the whole write (the board's side; nothing here is shared with an interrupt through it).

use crate::STOP_AFTER_US;

/// Where both comparators stand while the gate is held down: the last tick the timer counts (the hardware period register is the peak minus one, hal/esp32s3/include/hal/mcpwm_ll.h:254-255). The tick IS reached, so the output is kept low there by the generator's priority of the
/// event of comparator B (clear) over comparator A (set) when the two fall on one tick - ESP32-S3 TRM v1.8, table 36.3-3, p. 1341, `open_at` being the first comparator built and so A - and then by the clear at the zero of the next lap.
///
/// A value one higher, `STOP_AFTER_US` itself, is accepted by the driver (it checks against the peak, mcpwm_cmpr.c:195, and the peak is the period, mcpwm_timer.c:78) and lies beyond the period register, where by the TRM (same page: "if the value of A or B is set to be greater than the
/// period, then U/DTEA and U/DTEB will never occur") it would not be reached at all. It is not used: this value is the one the percent-zero order has always written, 342 514 seconds of 21-25.10 with the gate off for four seconds running counted no pulse on the pad with it, and the other
/// value the board has not measured.
pub const DOWN_AT_US: u32 = STOP_AFTER_US - 1;

/// What the write needs of the gate. Each method is ONE write or ONE read of the hardware, because that is the grain the interrupt can come between.
pub trait GateHardware {
    /// Holds the output low until [`GateHardware::release`].
    fn hold_low(&mut self) -> Result<(), String>;
    fn set_open(&mut self, at_us: u32) -> Result<(), String>;
    fn set_close(&mut self, at_us: u32) -> Result<(), String>;
    /// Whether the gate is stopped for want of fronts.
    fn stopped(&self) -> bool;
    fn release(&mut self) -> Result<(), String>;
}

/// Moves the gate to OPEN_AT_US and CLOSE_AT_US in the order above. A gate stopped by the interrupt in the middle is left held low and out of reach, and the answer is Ok: it is the stop that was asked for by the fronts, not a failure of the write.
pub fn write_gate(hw: &mut impl GateHardware, open_at_us: u32, close_at_us: u32) -> Result<(), String> {
    if hw.stopped() {
        return Ok(());
    }
    hw.hold_low()?;
    let moved = hw.set_open(open_at_us).and_then(|()| hw.set_close(close_at_us));
    if hw.stopped() {
        // the interrupt came in while the comparators were moving: out of reach again, and the output stays down
        let out_of_reach = hw.set_open(DOWN_AT_US).and_then(|()| hw.set_close(DOWN_AT_US));
        return moved.and(out_of_reach);
    }
    // Let go whatever happened above: a gate left held low is a fan that never turns again, and the caller hears about the comparator either way.
    let freed = hw.release();
    moved.and(freed)
}
