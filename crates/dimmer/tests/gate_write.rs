//! The order the comparators of the gate are written in, with the timer's interrupt allowed to come between any two of the writes. The task that moves the gate (`Gate::move_to`) and the callback that stops it (`gate_stop`) write the same two comparators; the callback
//! comes from the timer, not from the task, and nothing a task holds keeps it out. The rule that holds regardless (crates/dimmer, `write_gate`): the output is held low for the whole write, the flag is read again after the last write, and the output is let go only
//! with the flag clear. These rows put the interrupt before EVERY write of the protocol in turn, and before every write at once, and ask the one question that matters: is there a moment when the gate is free, the fronts are gone, and the comparators are not out of reach?
//!
//! What a row holds is the ORDER, with each register write one step. It does not hold the hardware: that every step is one write and the interrupt cannot split one is the processor's (a store to a peripheral register), and the arbitration at the stopped value is the generator's (see `fan_gate`).

use dimmer::{write_gate, GateHardware, DOWN_AT_US};

const OPEN: u32 = 6_000;
const CLOSE: u32 = 6_050;

/// The gate as the protocol sees it, and where the interrupt of the timer is let in: before the Nth step it takes (None - never), or before every step.
struct Fake {
    forced_low: bool,
    open: u32,
    close: u32,
    stopped: bool,
    step: usize,
    interrupt_before: Option<usize>,
    interrupt_every_step: bool,
    interrupts: usize,
    /// every state the gate was in at a step boundary, for the rows to look at
    states: Vec<(bool, bool, u32, u32)>,
}

impl Fake {
    fn new(interrupt_before: Option<usize>, every: bool) -> Self {
        Fake { forced_low: false, open: DOWN_AT_US, close: DOWN_AT_US, stopped: false, step: 0, interrupt_before, interrupt_every_step: every, interrupts: 0, states: Vec::new() }
    }

    /// What `gate_stop` does, whole: the flag, then both comparators out of reach. It runs to its end before the task goes on.
    fn interrupt(&mut self) {
        self.stopped = true;
        self.open = DOWN_AT_US;
        self.close = DOWN_AT_US;
        self.interrupts += 1;
        self.states.push((self.stopped, self.forced_low, self.open, self.close));
    }

    fn at_step(&mut self) {
        if self.interrupt_every_step || self.interrupt_before == Some(self.step) {
            self.interrupt();
        }
        self.step += 1;
    }

    fn noted(&mut self) {
        self.states.push((self.stopped, self.forced_low, self.open, self.close));
    }

    /// The state in which a pulse can come out at no phase: the gate is let go, the fronts are gone, and a comparator is within reach.
    fn dangerous(state: &(bool, bool, u32, u32)) -> bool {
        let (stopped, forced_low, open, close) = *state;
        stopped && !forced_low && (open != DOWN_AT_US || close != DOWN_AT_US)
    }
}

impl GateHardware for Fake {
    fn hold_low(&mut self) -> Result<(), String> {
        self.at_step();
        self.forced_low = true;
        self.noted();
        Ok(())
    }
    fn set_open(&mut self, at_us: u32) -> Result<(), String> {
        self.at_step();
        self.open = at_us;
        self.noted();
        Ok(())
    }
    fn set_close(&mut self, at_us: u32) -> Result<(), String> {
        self.at_step();
        self.close = at_us;
        self.noted();
        Ok(())
    }
    fn stopped(&self) -> bool {
        self.stopped
    }
    fn release(&mut self) -> Result<(), String> {
        self.at_step();
        self.forced_low = false;
        self.noted();
        Ok(())
    }
}

/// How many steps the protocol takes when nothing interrupts it; the interrupt is let in before each of them and after the last.
fn steps_of_a_quiet_write() -> usize {
    let mut quiet = Fake::new(None, false);
    write_gate(&mut quiet, OPEN, CLOSE).unwrap();
    quiet.step
}

#[test]
fn a_write_with_nothing_in_the_way_holds_the_gate_down_moves_both_and_lets_go_at_what_was_asked() {
    let mut hw = Fake::new(None, false);
    write_gate(&mut hw, OPEN, CLOSE).unwrap();
    assert_eq!((hw.open, hw.close, hw.forced_low), (OPEN, CLOSE, false));
    assert!(hw.states.iter().all(|s| !Fake::dangerous(s)));
    assert!(hw.step >= 4, "hold, two writes, let go");
}

#[test]
fn the_interrupt_before_any_one_step_never_leaves_the_gate_free_with_a_comparator_in_reach() {
    let steps = steps_of_a_quiet_write();
    for before in 0..=steps {
        let mut hw = Fake::new(Some(before), false);
        write_gate(&mut hw, OPEN, CLOSE).unwrap();
        // the interrupt after the last step is the one the task never sees come: let it in now
        if hw.interrupts == 0 {
            hw.interrupt();
        }
        let bad: Vec<_> = hw.states.iter().filter(|s| Fake::dangerous(s)).collect();
        assert!(bad.is_empty(), "interrupt before step {before}: the gate was free with the comparators in reach: {bad:?}");
        assert!(hw.stopped && hw.open == DOWN_AT_US && hw.close == DOWN_AT_US, "interrupt before step {before}: it ended at {:?}", (hw.open, hw.close, hw.stopped));
    }
}

#[test]
fn an_interrupt_before_every_step_ends_out_of_reach_and_never_free_with_a_comparator_in_reach() {
    let mut hw = Fake::new(None, true);
    write_gate(&mut hw, OPEN, CLOSE).unwrap();
    assert!(hw.states.iter().all(|s| !Fake::dangerous(s)), "{:?}", hw.states);
    assert_eq!((hw.open, hw.close, hw.stopped), (DOWN_AT_US, DOWN_AT_US, true));
}

#[test]
fn a_gate_already_stopped_is_not_touched() {
    let mut hw = Fake::new(None, false);
    hw.stopped = true;
    write_gate(&mut hw, OPEN, CLOSE).unwrap();
    assert_eq!(hw.step, 0, "not one write to a stopped gate");
    assert_eq!((hw.open, hw.close), (DOWN_AT_US, DOWN_AT_US));
}

#[test]
fn a_write_that_fails_halfway_still_lets_the_gate_go_when_nothing_stopped_it() {
    struct Failing(Fake);
    impl GateHardware for Failing {
        fn hold_low(&mut self) -> Result<(), String> { self.0.hold_low() }
        fn set_open(&mut self, at_us: u32) -> Result<(), String> { self.0.set_open(at_us) }
        fn set_close(&mut self, _at_us: u32) -> Result<(), String> { Err("the comparator refused".into()) }
        fn stopped(&self) -> bool { self.0.stopped() }
        fn release(&mut self) -> Result<(), String> { self.0.release() }
    }
    let mut hw = Failing(Fake::new(None, false));
    let said = write_gate(&mut hw, OPEN, CLOSE);
    assert!(said.is_err(), "the caller hears about the comparator");
    assert!(!hw.0.forced_low, "a gate left held low is a fan that never turns again");
}
