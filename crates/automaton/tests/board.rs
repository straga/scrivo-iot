//! What the board does with the automaton besides running it: whether its commands reach the organs, and what
//! it writes down to come back to after a power cut.
//!
//! **THE IDLE RUN IS THE WHOLE CYCLE, NOT ONE SECOND.** What has to hold is that not ONE command of the automaton
//! reaches the auger or the igniter while the burner idles, and that the fan gets every one of its own - so the rows
//! drive a real `Burning` through a purge, every attempt at lighting, burning off and the fault, and count every call
//! the organs received. One second with the auger on would prove only that one second.

mod common;

use automaton::{hand_over, recalled, remembered, stage, Asked, Feed, Feeding, Lighting, Hands, State, Step, Told, Why};
use common::{Hold, Burning, Seen, Silence, Tuning};
use flame::Flame;

fn tuning() -> Tuning {
    Tuning {
        purge_before_s: 30.0,
        purge_after_s: 20.0,
        igniter_s: 30.0,
        wait_for_flame_s: 90.0,
        tries: 3,
        dose_after_first: 0.1,
        rise_s: 60.0,
        flame_lost_s: 120.0,
        smoulder_s: 300.0,
        burn_off_s: 120.0,
        burn_out_min_s: 60.0,
        pump_on_c: 45.0,
        flue_holds_c: 100.0,
        fire_numbers: common::FireNumbers::CARD,
        want_c: 60.0,
        hysteresis_c: 4.0,
        hold: Hold::Boiler,
        power: Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } },
        pump_off_below_c: 4.0,
        too_hot_c: 88.0,
        alarm_c: 95.0,
        sure_s: 10.0,
        relight_s: [20.0, 10.0, 10.0],
        relight_hot_c: 45.0,
        spin_up_s: 10.0,
        low_pressure_s: 20.0,
        kindling: Lighting { pa: 51.0, dose_s: 32.0 },
        step: Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

/// A cold burner under a fan that answers: what the board reads when nothing lights.
fn cold() -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0), draught_pa: Some(51.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// The organs as a row sees them: every call, in order.
#[derive(Default)]
struct Organs {
    calls: Vec<String>,
}

impl Hands for Organs {
    fn auger(&mut self, feeding: Feeding) {
        self.calls.push(format!("auger {feeding:?}"));
    }
    fn auger_still(&mut self, feeding: Feeding) {
        self.calls.push(format!("auger still {feeding:?}"));
    }
    fn igniter(&mut self, on: bool) {
        self.calls.push(format!("igniter {on}"));
    }
    fn fan(&mut self, asked: Asked) {
        self.calls.push(format!("fan {asked:?}"));
    }
    fn pump(&mut self, on: bool) {
        self.calls.push(format!("pump {on}"));
    }
}

/// A burner started and left to run to its fault with no flame ever, through MODE: what each second decided, and
/// what the organs received.
fn a_cycle_that_never_lights() -> (Vec<automaton::Doing>, Organs) {
    let t = tuning().sound().unwrap();
    let mut burning = Burning::new();
    let mut organs = Organs::default();
    let mut decided = Vec::new();
    for second in 0..1000 {
        let told = if second == 0 { Told::Start } else { Told::Nothing };
        let doing = burning.step(told, cold(), 1.0, &t);
        hand_over(&doing, &mut organs);
        decided.push(doing);
        if matches!(doing.state, State::Fault(_)) {
            break;
        }
    }
    (decided, organs)
}

// ---------------------------------------------------------------- every organ, for real, every second
//
// The owner, 23.09: the automaton switches the organs for real, always; the mode where the relays did not was not asked
// for and is gone.

#[test]
fn the_burner_hands_every_organ_every_second_of_what_it_decided() {
    let (decided, organs) = a_cycle_that_never_lights();
    // the cycle really is the whole of it, or the list below proves nothing
    assert_eq!(decided.last().map(|d| d.state), Some(State::Fault(Why::NeverLit(3))), "the run did not reach its fault");
    assert!(decided.iter().any(|d| d.auger) && decided.iter().any(|d| d.igniter), "the automaton never fed or heated");
    let expected: Vec<String> = decided
        .iter()
        .flat_map(|d| {
            [format!("auger {:?}", d.feeding), format!("igniter {}", d.igniter), format!("pump {}", d.pump), format!("fan {:?}", d.fan)]
        })
        .collect();
    assert_eq!(organs.calls, expected, "the organs did not get what it decided, second by second");
}

// ---------------------------------------------------------------- what is written down

fn every_state() -> Vec<State> {
    let why = [
        Why::NeverLit(8),
        Why::FlameLost,
        Why::FlameNotBack(3),
        Why::NoFuel,
        Why::NoDraught,
        Why::Overheated(92.5),
        Why::Unconfigured,
        Why::DraughtSilent,
        Why::FlameSilent,
        Why::WaterSilent,
        Why::ReturnSilent,
        Why::NoFronts,
    ];
    // every reason there is: a new one that is not listed above stops this match from building
    let _listed = |w: Why| match w {
        Why::NeverLit(_) | Why::FlameLost | Why::FlameNotBack(_) | Why::NoFuel | Why::NoDraught | Why::LowPressure | Why::Overheated(_) | Why::Unconfigured | Why::DraughtSilent | Why::FlameSilent | Why::WaterSilent | Why::ReturnSilent | Why::NoFronts => (),
    };
    let mut all = vec![State::Stop, State::Purge, State::Kindling(1), State::Kindling(8), State::Rising, State::Working, State::Smouldering, State::BurningOff(None)];
    all.extend(why.iter().map(|w| State::BurningOff(Some(*w))));
    all.extend(why.iter().map(|w| State::Fault(*w)));
    all
}

#[test]
fn every_state_comes_back_as_itself_from_the_word_it_was_written_as() {
    for state in every_state() {
        let word = remembered(state);
        assert_eq!(recalled(&word), Some(state), "{state:?} was written as {word:?}");
    }
}

#[test]
fn a_word_nobody_wrote_is_not_taken_for_a_state() {
    for word in ["", "stopped", "kindling", "kindling:0", "kindling:x", "kindling:-1", "fault", "fault:", "fault:nothing",
        "burning_off:never_lit", "burning_off:overheated:hot", "stop:1", "Stop", " stop"]
    {
        assert_eq!(recalled(word), None, "{word:?} was taken for a state");
    }
}

#[test]
fn the_word_is_short_enough_for_a_key_of_nvs_and_carries_no_line_break() {
    for state in every_state() {
        let word = remembered(state);
        assert!(!word.is_empty() && word.len() <= 64 && !word.contains('\n'), "{state:?} -> {word:?}");
    }
}

// ---------------------------------------------------------------- the stage a page draws

#[test]
fn each_state_has_its_stage_on_the_strip() {
    let pairs = [
        (State::Stop, "stop"),
        (State::Purge, "purge"),
        (State::Kindling(3), "kindling"),
        (State::Rising, "rising"),
        (State::Working, "working"),
        (State::Smouldering, "smouldering"),
        (State::BurningOff(Some(Why::NoFuel)), "burning_off"),
        (State::Fault(Why::FlameLost), "fault"),
    ];
    for (state, want) in pairs {
        assert_eq!(stage(state), want, "{state:?}");
    }
}

// ---------------------------------------------------------------- the stage it broke on, for the strip

#[test]
fn the_stage_it_broke_on_is_taken_when_the_failure_begins_and_kept_to_the_fault() {
    let kindling = State::Kindling(3);
    let off = State::BurningOff(Some(Why::NeverLit(3)));
    let at = automaton::broken_at(None, Some(kindling), off);
    assert_eq!(at, Some("kindling"));
    let at = automaton::broken_at(at, Some(off), off);
    assert_eq!(at, Some("kindling"), "the burning off forgot where it began");
    let at = automaton::broken_at(at, Some(off), State::Fault(Why::NeverLit(3)));
    assert_eq!(at, Some("kindling"), "the fault forgot where it began");
    // a fault straight from a stage, with no burning off between
    assert_eq!(automaton::broken_at(None, Some(State::Purge), State::Fault(Why::NoDraught)), Some("purge"));
}

#[test]
fn nothing_is_broken_outside_a_failure_and_a_burning_off_a_person_asked_for_is_not_one() {
    assert_eq!(automaton::broken_at(Some("kindling"), Some(State::Fault(Why::NeverLit(3))), State::Stop), None, "Stop did not clear it");
    assert_eq!(automaton::broken_at(None, Some(State::Working), State::BurningOff(None)), None);
    assert_eq!(automaton::broken_at(Some("working"), Some(State::BurningOff(None)), State::Stop), None);
    // a board that came up in a fault does not know where it broke, and does not guess
    assert_eq!(automaton::broken_at(None, None, State::Fault(Why::FlameLost)), None);
    for state in every_state() {
        let failing = matches!(state, State::BurningOff(Some(_)) | State::Fault(_));
        if !failing {
            assert_eq!(automaton::broken_at(Some("rising"), Some(State::Working), state), None, "{state:?} kept a stage");
        }
    }
}

// ---------------------------------------------------------------- the organs, one list for the card and the scheme
//
// the scheme's completeness is held by a test against the list of organs the automaton commands.
// That list has to be what the automaton really reaches, or the test holds the scheme to the wrong list.

#[test]
fn the_list_of_organs_is_exactly_what_a_live_burner_reaches() {
    let doing = automaton::Doing {
        state: State::Working, auger: true, feeding: Feeding::Stop, igniter: true, fan: Asked::Hold(51.0), alarm: false, pump: true,
        pump_why: automaton::PumpWhy::NoNumber,
    };
    let mut organs = Organs::default();
    hand_over(&doing, &mut organs);
    let mut reached: Vec<String> = organs.calls.iter().map(|c| c.split(' ').next().unwrap().to_string()).collect();
    reached.sort();
    let mut listed: Vec<String> = automaton::ORGANS.iter().map(|o| o.to_string()).collect();
    listed.sort();
    assert_eq!(listed, reached, "ORGANS is not the list of what the automaton commands");
}

#[test]
fn the_pump_says_what_of_the_water_decided_it() {
    use automaton::{pump_words, PumpWhy};
    assert_eq!(pump_words(PumpWhy::Water(47.0)), "water 47 °C");
    assert_eq!(pump_words(PumpWhy::Overheated(90.0)), "overheated, 90 °C");
    assert_eq!(pump_words(PumpWhy::NoNumber), "no number from the boiler");
}

#[test]
fn the_pumps_reason_is_one_word_of_a_known_list_a_word_for_each_reason() {
    use automaton::{pump_word, PumpWhy};
    assert_eq!(pump_word(PumpWhy::Water(40.0)), "water");
    assert_eq!(pump_word(PumpWhy::Water(80.0)), "water", "the number is not part of the word");
    assert_eq!(pump_word(PumpWhy::Overheated(90.0)), "overheated");
    assert_eq!(pump_word(PumpWhy::NoNumber), "no_number", "a word of a list is one word, not a phrase");
}
