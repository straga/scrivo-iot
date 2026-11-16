//! Whether the burner stands at rest, for the board that restarts itself at night: a stop with nothing running and no flame is the only one it may restart in, a fault is not waited out, everything else is fire.

use automaton::{rest, Asked, Doing, Feeding, PumpWhy, Rest, State, Why};

fn doing(state: State, auger: bool, igniter: bool) -> Doing {
    Doing { state, auger, feeding: if auger { Feeding::Dose { left_s: 1.0 } } else { Feeding::Stop }, igniter, fan: Asked::Stop, alarm: false, pump: false, pump_why: PumpWhy::NoNumber }
}

#[test]
fn a_stop_with_nothing_running_and_no_flame_is_at_rest() {
    assert_eq!(rest(&doing(State::Stop, false, false), false), Rest::Standing);
}

#[test]
fn a_stop_with_a_flame_the_igniter_or_the_auger_is_not_at_rest() {
    assert_eq!(rest(&doing(State::Stop, false, false), true), Rest::Fire, "a flame is seen");
    assert_eq!(rest(&doing(State::Stop, false, true), false), Rest::Fire, "the igniter is hot");
    assert_eq!(rest(&doing(State::Stop, true, false), false), Rest::Fire, "the auger turns");
}

#[test]
fn every_state_of_a_fire_is_fire_and_a_fault_is_a_fault() {
    for state in [State::Purge, State::Kindling(1), State::Rising, State::Working, State::Smouldering, State::BurningOff(None), State::BurningOff(Some(Why::NoFuel))] {
        assert_eq!(rest(&doing(state, false, false), false), Rest::Fire, "{state:?}");
    }
    assert_eq!(rest(&doing(State::Fault(Why::NoFuel), false, false), false), Rest::Fault);
    assert_eq!(rest(&doing(State::Fault(Why::NeverLit(3)), false, false), true), Rest::Fault, "a fault is a fault with a flame too");
}
