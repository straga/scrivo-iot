//! What one reading of the flame's pin means. No hardware here: the ADC is the device's half.

use flame::{Flame, Thresholds};

/// Starting thresholds - BOTH CALCULATED, neither measured. They are here as numbers a test
/// feeds in, never as numbers the crate keeps: the real ones are taken off the board with a lamp and a palm.
const START: Thresholds = Thresholds { lit_mv: 2_400, steady_mv: 1_100 };

#[test]
fn darkness_above_the_lit_threshold_is_no_flame() {
    // the measurement of 20.09: 3153 mV on GPIO7 in the dark
    assert_eq!(flame::read(3_153, START), Flame::Out);
    assert_eq!(flame::read(2_401, START), Flame::Out);
}

#[test]
fn under_the_lit_threshold_and_over_the_steady_one_is_a_weak_flame() {
    assert_eq!(flame::read(2_400, START), Flame::Weak);
    assert_eq!(flame::read(2_006, START), Flame::Weak);
    assert_eq!(flame::read(1_101, START), Flame::Weak);
}

#[test]
fn under_the_steady_threshold_the_flame_is_holding() {
    assert_eq!(flame::read(1_100, START), Flame::Steady);
    assert_eq!(flame::read(96, START), Flame::Steady);
}

#[test]
fn the_thresholds_are_taken_as_given_and_nothing_is_kept_in_the_crate() {
    // THE POINT OF THE PARAMETER. The owner has not seen the stage yet and the numbers are calculated; moving
    // them must not touch this code. The same millivolts read as two different states under two sets.
    let lower = Thresholds { lit_mv: 2_200, steady_mv: 900 };
    assert_eq!(flame::read(2_300, START), Flame::Weak);
    assert_eq!(flame::read(2_300, lower), Flame::Out);
    assert_eq!(flame::read(1_000, START), Flame::Steady);
    assert_eq!(flame::read(1_000, lower), Flame::Weak);
}

#[test]
fn thresholds_out_of_order_are_a_setting_fault_and_not_a_state_of_the_fire() {
    // a person fills in two fields, and nothing on a page keeps them in order. "Steady" above "lit" describes
    // no fire at all, and answering it with a state would make the card confident about nonsense.
    let upside_down = Thresholds { lit_mv: 1_000, steady_mv: 2_000 };
    assert_eq!(flame::read(1_500, upside_down), Flame::Unconfigured);
    // equal is the same fault: there would be no weak flame between them at all
    assert_eq!(flame::read(1_500, Thresholds { lit_mv: 1_500, steady_mv: 1_500 }), Flame::Unconfigured);
    // and a threshold of nothing is not a threshold
    assert_eq!(flame::read(1_500, Thresholds { lit_mv: 2_400, steady_mv: 0 }), Flame::Unconfigured);
}

#[test]
fn a_setting_fault_is_answered_before_the_millivolts_are_read_at_all() {
    // whatever the pin holds, the answer is about the setting: a reading through thresholds that are not
    // thresholds is not a reading
    for mv in [0, 96, 1_500, 3_153, 3_300] {
        assert_eq!(flame::read(mv, Thresholds { lit_mv: 100, steady_mv: 200 }), Flame::Unconfigured);
    }
}

// ---- what a person is told ----

#[test]
fn every_outcome_reaches_a_person_as_words_that_fit_a_row() {
    for said in [Flame::Out, Flame::Weak, Flame::Steady, Flame::Unconfigured] {
        let words = flame::words(said);
        assert!(!words.is_empty(), "{said:?} came out empty");
        assert!(words.parse::<f64>().is_err(), "{said:?} came out as a number: {words}");
        assert_ne!(words, "-", "{said:?} came out as a dash");
        // the same 64 characters the draught's rows are held to: three lines in a card at 375 px
        assert!(words.len() <= 64, "{said:?} is {} characters: {words}", words.len());
    }
}

#[test]
fn the_outcomes_do_not_say_the_same_thing() {
    let said = [Flame::Out, Flame::Weak, Flame::Steady, Flame::Unconfigured];
    let words: Vec<String> = said.iter().map(|f| flame::words(*f)).collect();
    for (i, one) in words.iter().enumerate() {
        for (j, other) in words.iter().enumerate() {
            assert!(i == j || one != other, "{:?} and {:?} say the same: {one}", said[i], said[j]);
        }
    }
}

// ---- the question the stage asks: is a broken wire tellable from darkness ----

#[test]
fn a_broken_wire_reads_as_no_flame_and_the_crate_says_so_rather_than_pretending() {
    // ON THIS CIRCUIT THEY ARE NOT TELLABLE APART, and the crate must not invent a fifth outcome for symmetry
    // with the draught. The photoresistor is the LOWER arm: darkness makes it ~2 MΩ and the node sits at
    // 3.14 V, a broken wire leaves the node pulled to 3.3 V through the 100 kΩ - 157 mV apart, and BOTH lie
    // above the 2450 mV where the chip's calibration ends. The measurement of 20.09 read 3153 mV with a spread
    // of ZERO, which is the top of the scale, not a distance from it.
    //
    // So both are Out, and the fault is safe: a burner that cannot see its flame does not go on feeding.
    assert_eq!(flame::read(3_153, START), Flame::Out);
    assert_eq!(flame::read(3_300, START), Flame::Out);
    // and the crate says it in the open, where a reader of the code will meet it
    assert!(flame::TELLS_A_BROKEN_WIRE.is_none());
}
