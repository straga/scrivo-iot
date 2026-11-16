//! The gate that stops itself when the fronts stop, and the rule for it to fire again.
//!
//! **THE STOP IS THE TIMER'S.** The gate's timer is reset by every front of the zero-crossing pin; with none it reaches its period and counts again, and the comparators - which sit inside the period - fire a pulse on every lap with nothing to say where in
//! the wave it is (24.10, 23:47:00: 49-52 pulses a second for as long as the pin gave nothing). The period is [`STOP_AFTER_US`]; the board's callback at the timer's peak (devices/burner, `fan_gate`) moves both comparators out of reach, and it comes
//! again at every lap while the fronts are away. So: after the last front nothing fires but the one pulse that front's own delay asked for, and a stop is never later than the period.
//!
//! The period is four nominal half cycles: a front missing and two in a row are the real pin's ways (`crossings lost by gaps`, 4 to 29 a minute in the journal of `.33`) and a gate that stopped for those would be a fan that stopped for nothing. A stop is not a fault:
//! the fire is put out by the automaton, on the count of the second, not by this.

use crate::LIVE_CROSSINGS;

/// How long without a front the gate is stopped after, in microseconds: the period of its timer. Four half cycles of a 50 Hz mains.
pub const STOP_AFTER_US: u32 = 4 * crate::NOMINAL_HALF_US;

/// What the callback at the timer's peak is given, in microseconds: it must have moved the comparators before the first of them could fire after the wrap. The least delay after a front the gate is ever asked for is 3 500 µs (100 % of air on the shortest half cycle a mains gives).
pub const STOP_ISR_BUDGET_US: u32 = 3_000;

/// Whether a stopped gate may fire again: it IS stopped, no new stop came in the second just counted, and that second held what a mains gives ([`LIVE_CROSSINGS`]). The stray fronts of a silent pin are 1 to 5 a second and never a mains: the gate does not
/// go back to firing at no phase because one of them came.
pub fn may_resume(stopped: bool, new_stops: u32, up: u32) -> bool {
    stopped && new_stops == 0 && LIVE_CROSSINGS.contains(&up)
}
