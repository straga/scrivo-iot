//! The words a client reads the automaton by: its stages and the reasons of a failure (the dictionary).

use automaton::{State, Why};

/// Every state the automaton comes to, with every reason and a count or a temperature where one belongs.
fn every_state() -> Vec<State> {
    let whys = [Why::NeverLit(3), Why::FlameLost, Why::FlameNotBack(2), Why::NoFuel, Why::NoDraught, Why::LowPressure, Why::Overheated(91.5), Why::Unconfigured, Why::DraughtSilent, Why::FlameSilent, Why::WaterSilent, Why::ReturnSilent, Why::NoFronts];
    let mut all = vec![State::Stop, State::Purge, State::Kindling(1), State::Rising, State::Working, State::Smouldering, State::BurningOff(None)];
    all.extend(whys.iter().map(|w| State::BurningOff(Some(*w))));
    all.extend(whys.iter().map(|w| State::Fault(*w)));
    all
}

#[test]
fn every_stage_and_every_reason_is_a_word_of_the_lists_a_client_reads() {
    for state in every_state() {
        assert!(automaton::STAGES.contains(&automaton::stage(state)), "{state:?}");
        if let State::BurningOff(Some(why)) | State::Fault(why) = state {
            assert!(automaton::WHYS.contains(&automaton::why_name(why)), "{why:?}");
        }
    }
    assert_eq!(automaton::STAGES.len(), 8);
    assert_eq!(automaton::WHYS.len(), 13);
}

#[test]
fn a_reason_is_named_by_the_word_the_board_keeps_it_under() {
    for state in every_state() {
        if let State::BurningOff(Some(why)) | State::Fault(why) = state {
            let kept = automaton::remembered(state);
            let head = kept.split(':').nth(1).unwrap_or("");
            assert_eq!(automaton::why_name(why), head, "{kept}");
        }
    }
}
