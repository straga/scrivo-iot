//! A person's hand on the organs while the automaton runs the burner: refused in words while it
//! runs, free in a stop and in a fault; and the automaton taking the organs back from a person when it starts.

use automaton::{by_hand, hand_over, Asked, Doing, Feeding, Hands, OnChange, PumpWhy, State, Why, ORGANS};

/// Every state the automaton runs the burner in.
const RUNNING: [State; 7] = [
    State::Purge,
    State::Kindling(1),
    State::Kindling(3),
    State::Rising,
    State::Working,
    State::Smouldering,
    State::BurningOff(None),
];

#[test]
fn a_person_switching_the_igniter_in_the_middle_of_kindling_is_refused_in_words_that_say_what_to_do() {
    let refused = by_hand(State::Kindling(1), true, false, "igniter").expect_err("the automaton is lighting the burner");
    assert!(refused.contains("kindling"), "the words name what the automaton is doing: {refused}");
    assert!(refused.contains("stop"), "and say what to press: {refused}");
    assert!(refused.contains("igniter"), "and which thing was refused: {refused}");
}

#[test]
fn every_organ_is_refused_to_a_person_in_every_state_the_automaton_runs() {
    for state in RUNNING {
        for organ in ORGANS {
            assert!(by_hand(state, true, false, organ).is_err(), "{organ} by hand in {state:?} went through");
        }
    }
}

#[test]
fn in_a_stop_the_hand_is_free() {
    for organ in ORGANS {
        assert_eq!(by_hand(State::Stop, true, false, organ), Ok(()), "{organ} by hand in a stop");
    }
}

#[test]
fn in_a_fault_the_hand_is_free_for_a_person_is_what_the_fault_waits_for() {
    for why in [Why::NeverLit(3), Why::FlameLost, Why::NoDraught, Why::Overheated(90.0)] {
        assert_eq!(by_hand(State::Fault(why), true, false, "igniter"), Ok(()), "igniter by hand in {why:?}");
    }
}

/// Organs that write down every call.
#[derive(Default)]
struct Calls(Vec<String>);

impl Hands for Calls {
    fn auger(&mut self, feeding: Feeding) {
        self.0.push(format!("auger {feeding:?}"));
    }
    fn auger_still(&mut self, feeding: Feeding) {
        self.0.push(format!("auger still {feeding:?}"));
    }
    fn igniter(&mut self, on: bool) {
        self.0.push(format!("igniter {on}"));
    }
    fn pump(&mut self, on: bool) {
        self.0.push(format!("pump {on}"));
    }
    fn fan(&mut self, asked: Asked) {
        self.0.push(format!("fan {asked:?}"));
    }
}

#[test]
fn a_banned_feed_refuses_the_auger_by_hand_in_a_stop_and_in_a_fault_and_says_what_to_press() {
    let stands = [State::Stop, State::Fault(Why::NeverLit(3)), State::Fault(Why::FlameLost), State::Fault(Why::NoDraught)];
    for state in stands {
        let refused = by_hand(state, false, false, "auger").expect_err("the feed is banned, and the auger went by hand");
        assert!(refused.contains("banned"), "the words say the feed is banned: {refused}");
        assert!(refused.contains("Allow feed"), "and name the button that allows it: {refused}");
    }
}

#[test]
fn a_banned_feed_leaves_the_other_organs_to_the_hand() {
    for organ in ORGANS.into_iter().filter(|o| *o != "auger") {
        assert_eq!(by_hand(State::Stop, false, false, organ), Ok(()), "{organ} by hand in a stop under a ban");
    }
}

#[test]
fn an_allowed_feed_leaves_the_auger_to_the_hand_in_a_stop_and_in_a_fault() {
    assert_eq!(by_hand(State::Stop, true, false, "auger"), Ok(()));
    assert_eq!(by_hand(State::Fault(Why::NeverLit(3)), true, false, "auger"), Ok(()));
}

#[test]
fn a_ban_in_a_stop_turns_off_an_auger_a_person_turned_on() {
    // the automaton last said "off"; a person turned the auger on by hand; the ban must say "off" again, to it alone
    let mut hands = OnChange::new(Calls::default());
    hand_over(&stop_second(), &mut hands);
    hands.inner.0.clear();
    hands.forget_auger();
    hand_over(&stop_second(), &mut hands);
    assert_eq!(hands.inner.0, ["auger Stop"], "the ban did not reach the auger, or reached organs a person holds");
}

fn stop_second() -> Doing {
    Doing {
        state: State::Stop,
        auger: false,
        feeding: Feeding::Stop,
        igniter: false,
        pump: false,
        pump_why: PumpWhy::Water(40.0),
        fan: Asked::Stop,
        alarm: false,
    }
}

#[test]
fn the_same_decision_the_next_second_reaches_no_organ() {
    // a relay the fuse opened stays open until the automaton decides something new
    let mut hands = OnChange::new(Calls::default());
    hand_over(&stop_second(), &mut hands);
    hands.inner.0.clear();
    hand_over(&stop_second(), &mut hands);
    // the auger alone hears the decision again, as a word that commands nothing (the next row); no organ is commanded twice
    assert_eq!(hands.inner.0, ["auger still Stop"], "sent again: {:?}", hands.inner.0);
}

#[test]
fn the_automaton_taking_the_organs_back_sends_every_one_of_them_again() {
    // a person switched the igniter on by hand in a stop; the automaton, which last said "off", must say it again
    let mut hands = OnChange::new(Calls::default());
    hand_over(&stop_second(), &mut hands);
    hands.inner.0.clear();
    hands.forget();
    hand_over(&stop_second(), &mut hands);
    assert_eq!(hands.inner.0, ["auger Stop", "igniter false", "pump false", "fan Stop"]);
}

fn working_second(feeding: Feeding) -> Doing {
    Doing { feeding, auger: true, ..stop_second() }
}

#[test]
fn a_decision_of_the_auger_that_stands_gets_a_new_plan_every_second_and_commands_nothing() {
    let at = |s: f32| Feeding::Rhythm { into_s: s, on_s: 0.8, off_s: 5.0 };
    let mut hands = OnChange::new(Calls::default());
    hand_over(&working_second(at(0.0)), &mut hands);
    assert_eq!(hands.inner.0[0], format!("auger {:?}", at(0.0)), "the first plan is a command: {:?}", hands.inner.0);
    for second in 1..6 {
        hands.inner.0.clear();
        hand_over(&working_second(at(second as f32)), &mut hands);
        assert_eq!(hands.inner.0, [format!("auger still {:?}", at(second as f32))], "second {second}: the same decision was commanded again, or its plan not given");
    }
}

#[test]
fn a_decision_that_changes_reaches_the_auger_as_a_command_and_a_dose_that_goes_on_as_a_plan() {
    let mut hands = OnChange::new(Calls::default());
    hand_over(&working_second(Feeding::Dose { left_s: 32.0 }), &mut hands);
    hands.inner.0.clear();
    hand_over(&working_second(Feeding::Dose { left_s: 31.0 }), &mut hands);
    assert_eq!(hands.inner.0, ["auger still Dose { left_s: 31.0 }"], "a dose that went on was taken for a new decision");
    hands.inner.0.clear();
    hand_over(&working_second(Feeding::Rhythm { into_s: 0.0, on_s: 2.0, off_s: 15.0 }), &mut hands);
    assert_eq!(hands.inner.0, ["auger Rhythm { into_s: 0.0, on_s: 2.0, off_s: 15.0 }"], "a decision that changed was taken for the same one");
    hands.inner.0.clear();
    // the same kind of decision with the numbers of its cycle changed is another decision
    hand_over(&working_second(Feeding::Rhythm { into_s: 1.0, on_s: 1.5, off_s: 15.0 }), &mut hands);
    assert_eq!(hands.inner.0, ["auger Rhythm { into_s: 1.0, on_s: 1.5, off_s: 15.0 }"]);
    hands.inner.0.clear();
    hands.forget_auger();
    hand_over(&working_second(Feeding::Rhythm { into_s: 2.0, on_s: 1.5, off_s: 15.0 }), &mut hands);
    assert_eq!(hands.inner.0, ["auger Rhythm { into_s: 2.0, on_s: 1.5, off_s: 15.0 }"], "an auger taken back from a hand must be commanded, not only given a plan");
}
