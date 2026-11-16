//! What one reading of the pin means. No hardware here: the ADC is the device's half.

use draught::Reading;

// The divider as it was calculated and soldered on 12.09: two 10 kΩ, so the pin sees half.
const TOP: u32 = 10_000;
const BOTTOM: u32 = 10_000;

#[test]
fn the_pin_sees_the_share_the_two_resistors_leave() {
    assert_eq!(draught::sensor_mv(1_000, TOP, BOTTOM), Some(2_000));
    assert_eq!(draught::sensor_mv(250, TOP, BOTTOM), Some(500));
    assert_eq!(draught::sensor_mv(2_250, TOP, BOTTOM), Some(4_500));
}

#[test]
fn a_divider_that_is_not_a_half_is_read_as_what_it_is() {
    // THE COEFFICIENT IS THE RESISTORS, not a number written down once. If the measurement does
    // not come out at half, measure the resistors and write the real value - so the real value is what goes in.
    // 1000 mV on the pin through 1.78 kΩ over 3.29 kΩ is 1000 × (1780 + 3290) / 3290
    assert_eq!(draught::sensor_mv(1_000, 1_780, 3_290), Some(1_541));
}

#[test]
fn the_offset_zero_of_the_sensor_is_no_pressure() {
    assert_eq!(draught::pascals(500), Reading::Is(0.0));
}

#[test]
fn the_top_of_the_sensor_is_the_top_of_the_scale() {
    assert_eq!(draught::pascals(4_500), Reading::Is(300.0));
}

#[test]
fn the_middle_is_the_middle() {
    assert_eq!(draught::pascals(2_500), Reading::Is(150.0));
}

#[test]
fn a_wire_that_came_off_is_not_a_pressure_of_zero() {
    // THE WHOLE POINT OF AN OFFSET ZERO. A sensor saying "no pressure" says 500 mV; a sensor that is not there
    // says nothing at all. Reading the second as the first is how a regulator would drive a fan at full against
    // a draught it cannot see.
    assert_eq!(draught::pascals(0), Reading::Missing);
    assert_eq!(draught::pascals(50), Reading::Missing);
}

#[test]
fn below_its_zero_but_alive_is_not_missing_and_not_a_number() {
    // between "nothing there" and its own zero: the sensor answers, but not something this scale can carry
    assert_eq!(draught::pascals(400), Reading::Under);
}

#[test]
fn above_its_top_is_named_and_not_clamped() {
    assert_eq!(draught::pascals(4_700), Reading::Over);
}

#[test]
fn the_two_halves_meet_on_the_numbers_that_were_calculated() {
    // 0.250 V on the pin at zero and 2.250 V at 300 Pa - the divider table
    assert_eq!(draught::read(250, TOP, BOTTOM), Reading::Is(0.0));
    assert_eq!(draught::read(2_250, TOP, BOTTOM), Reading::Is(300.0));
}

// ---- a setting that is not a divider is not a broken sensor ----

#[test]
fn a_bottom_resistor_of_nothing_is_a_setting_fault_and_says_so() {
    // THE HOLE THIS PAIR CLOSES. sensor_mv used to answer 0 for a divider with no bottom resistor, and 0 reads
    // as Missing - so a device configured wrongly reported a sensor that had fallen off. A person acts on those
    // two differently: one is a soldering iron, the other is a line in a config.
    assert_eq!(draught::read(250, 10_000, 0), Reading::Unconfigured);
}

#[test]
fn two_resistors_of_nothing_are_a_setting_fault_too() {
    // the same hole by its other door: nothing over nothing is not a share of anything
    assert_eq!(draught::read(250, 0, 0), Reading::Unconfigured);
}

#[test]
fn resistors_that_do_not_fit_a_number_are_a_setting_fault_and_not_a_wrapped_one() {
    // and its third door: top + bottom overflowed a u32 and came out as a small number, which read as a
    // perfectly ordinary pressure. Nonsense in the setting must never leave as a number.
    assert_eq!(draught::read(250, u32::MAX, u32::MAX), Reading::Unconfigured);
}

#[test]
fn a_divider_with_no_top_resistor_is_a_wire_and_is_read_as_one() {
    // not a fault: nothing in the top arm means the pin sees the sensor itself
    assert_eq!(draught::read(500, 0, 10_000), Reading::Is(0.0));
}

#[test]
fn a_good_setting_still_reads_the_sensor() {
    assert_eq!(draught::read(250, TOP, BOTTOM), Reading::Is(0.0));
    assert_eq!(draught::read(2_250, TOP, BOTTOM), Reading::Is(300.0));
    assert_eq!(draught::read(0, TOP, BOTTOM), Reading::Missing);
}

#[test]
fn sensor_mv_says_nothing_rather_than_zero_when_the_divider_is_not_one() {
    // the same rule one level down: the absence of an answer is not the number zero
    assert_eq!(draught::sensor_mv(250, 10_000, 0), None);
    assert_eq!(draught::sensor_mv(250, u32::MAX, u32::MAX), None);
    assert_eq!(draught::sensor_mv(250, TOP, BOTTOM), Some(500));
}

// ---- the window: many samples of a shaking pin, one number ----

// ---- every outcome reaches a person as words ----

#[test]
fn a_pressure_is_the_number_with_its_unit() {
    assert_eq!(draught::words(Reading::Is(0.0)), "0 Pa");
    assert_eq!(draught::words(Reading::Is(137.0)), "137 Pa");
}

#[test]
fn no_outcome_but_a_pressure_is_ever_a_number_or_a_dash() {
    // WHAT THIS PAIR IS FOR. Four of the five outcomes are not a number, and a card that turns them into 0 or -
    // tells a person the burner is fine and reads nothing. Each has to say what it is and what to do about it.
    for said in [Reading::Under, Reading::Over, Reading::Missing, Reading::Unconfigured] {
        let words = draught::words(said);
        assert!(words.parse::<f64>().is_err(), "{said:?} came out as a number: {words}");
        assert_ne!(words, "-", "{said:?} came out as a dash");
        assert!(!words.is_empty(), "{said:?} came out empty");
        // a person is told what to do with it, and that takes more than a label
        assert!(words.contains(" - "), "{said:?} says what it is but not what to do: {words}");
    }
}

#[test]
fn the_five_outcomes_do_not_say_the_same_thing() {
    // a person acts differently on each: the soldering iron, the settings, the tube, the fan
    let said = [Reading::Is(0.0), Reading::Under, Reading::Over, Reading::Missing, Reading::Unconfigured];
    let words: Vec<String> = said.iter().map(|r| draught::words(*r)).collect();
    for (i, one) in words.iter().enumerate() {
        for (j, other) in words.iter().enumerate() {
            assert!(i == j || one != other, "{:?} and {:?} say the same: {one}", said[i], said[j]);
        }
    }
}

#[test]
fn a_sensor_that_fell_off_and_a_divider_nobody_filled_in_send_a_person_to_different_places() {
    // the two faults crates/draught was made to tell apart, said in words a person acts on
    assert!(draught::words(Reading::Missing).contains("12 V"));
    assert!(draught::words(Reading::Unconfigured).contains("resistances"));
}

// ---- what the divider now set means, said back to the person who set it ----

#[test]
fn the_divider_says_where_the_sensors_own_zero_lands_on_the_pin() {
    // THE OTHER WAY ROUND FROM sensor_mv, and it is what a person needs beside the two fields: they write the
    // resistances in, and the card says which millivolts the pin should sit at with no pressure. Comparing that
    // to the millivolts the card shows is the whole of the morning's measurement.
    assert_eq!(draught::pin_mv(draught::ZERO_MV, TOP, BOTTOM), Some(250));
    // 1.78 kΩ over 3.29 kΩ - the divider of the previous board: 500 × 3290 / 5070
    assert_eq!(draught::pin_mv(draught::ZERO_MV, 1_780, 3_290), Some(324));
}

#[test]
fn a_divider_that_is_not_one_says_nothing_about_the_pin_either() {
    // the same rule as everywhere in this crate: no answer is not the number zero
    assert_eq!(draught::pin_mv(500, 10_000, 0), None);
    assert_eq!(draught::pin_mv(500, u32::MAX, u32::MAX), None);
}

#[test]
fn every_outcome_fits_the_row_of_a_phone() {
    // MEASURED ON THE BOARD, NOT GUESSED. The first words written here ran to 130 characters and took FIVE
    // wrapped lines in the card on a 375 px screen (runs/3.3/draught_card_375.png) - a paragraph where a person
    // expects a reading. The value column of that card fits about 22 characters, so 64 is three lines and that
    // is the most a row may take.
    for said in [Reading::Is(300.0), Reading::Under, Reading::Over, Reading::Missing, Reading::Unconfigured] {
        let words = draught::words(said);
        assert!(words.len() <= 64, "{said:?} is {} characters: {words}", words.len());
    }
}

#[test]
fn a_result_too_large_for_the_number_it_is_kept_in_is_a_setting_fault_too() {
    // THE FOURTH DOOR OF THE SAME CLASS, and the one the sum in u64 left open: the SUM of the arms fits a u32,
    // so the check lets the pair through, and it is the RESULT that overflows on the way back down. What came
    // out was an ordinary pressure with nothing to tell it from a real one.
    //
    // The numbers: 4 292 115 452 over 35 at 1506 mV on the pin. The sensor's millivolts work out to
    // 184 683 597 812, of which a u32 keeps 4084 - and 4084 mV is 268 Pa, a draught this boiler could have.
    assert_eq!(draught::read(1_506, 4_292_115_452, 35), Reading::Unconfigured);
    // the same door: 536 870 912 500 keeps 500, which is the sensor's own zero exactly
    assert_eq!(draught::read(250, 2_147_483_649, 1), Reading::Unconfigured);
    // and one level down, where the number is made
    assert_eq!(draught::sensor_mv(1_506, 4_292_115_452, 35), None);
    // THE DOOR IS NOW SHUT ONE FLOOR UP, and that is the point of the cap on the arms: with both of them under
    // a megaohm there is no product in this crate that can wrap, so there is no narrowing for a bogus pressure
    // to come out of. The check on the result stays where it is - it costs nothing and it is the belt to this
    // brace - but the class is closed by the arms.
    assert!(!draught::is_a_divider(4_292_115_452, 35));
    assert!(!draught::is_a_divider(2_147_483_649, 1));
}

#[test]
fn a_pair_right_on_the_cap_is_still_a_divider_and_one_over_it_is_not() {
    // THE PAIR TO THE TEST ABOVE, and without it that one proves nothing: a check that refuses everything looks
    // exactly like a check that refuses what it should. The edge is the cap itself - a megaohm each - and one
    // ohm past it there is no reading at all.
    assert_eq!(draught::sensor_mv(1_506, draught::MOST_OHM, draught::MOST_OHM), Some(3_012));
    assert!(draught::is_a_divider(draught::MOST_OHM, draught::MOST_OHM));
    assert!(!draught::is_a_divider(draught::MOST_OHM + 1, draught::MOST_OHM));
    assert!(!draught::is_a_divider(draught::MOST_OHM, draught::MOST_OHM + 1));
    assert_eq!(draught::sensor_mv(1_506, draught::MOST_OHM + 1, draught::MOST_OHM), None);
    // and the outcome says which fault it is BY ITS CAUSE: arms nobody could have meant are a setting to fix,
    // not a draught to turn down. Answered as "over the scale" the card would say "turn the fan down".
    assert_eq!(draught::read(1_506, draught::MOST_OHM + 1, 1), Reading::Unconfigured);
    assert_eq!(draught::read(1, 4_294_967_294, 1), Reading::Unconfigured);
}


// ---- pascals without the step of a whole millivolt under them ----

/// The pressure of a reading that is one, for the cases below.
fn number(said: Reading) -> f32 {
    match said {
        Reading::Is(pa) => pa,
        other => panic!("expected a pressure, got {other:?}"),
    }
}

#[test]
fn a_curve_and_a_regulator_get_pascals_as_a_fraction() {
    // ONE MILLIVOLT ON THE PIN IS 0.15 Pa through a divider of a half, and a reading rounded to whole pascals
    // steps by one - which at a setpoint of twenty pascals is a step a regulator would chase. The window
    // already counted tenths of a millivolt; this is about not throwing them away again.
    assert_eq!(draught::read_tenths(22_500, TOP, BOTTOM), Reading::Is(300.0));
    assert_eq!(draught::read_tenths(12_500, TOP, BOTTOM), Reading::Is(150.0));
    // and the two entry points meet: the same pin, read in millivolts and in tenths, is the same pressure
    assert_eq!(draught::read_tenths(2_500, TOP, BOTTOM), draught::read(250, TOP, BOTTOM));
}

#[test]
fn the_tenths_keep_the_four_outcomes_apart_instead_of_answering_nothing() {
    // THE WHOLE POINT AGAINST THE OLD `exact`, WHICH ANSWERED None TO ALL THREE. A regulator needs the reason:
    // a sensor under its own zero sends a person to the tube's port, one that is not answering sends them to
    // its 12 V, and a divider nobody filled in sends them to a line of configuration.
    // 1614 tenths = 161.4 mV on the pin = 322.8 mV at the sensor: under its own zero
    assert_eq!(draught::read_tenths(1_614, TOP, BOTTOM), Reading::Under);
    assert_eq!(draught::read_tenths(0, TOP, BOTTOM), Reading::Missing);
    assert_eq!(draught::read_tenths(60_000, TOP, BOTTOM), Reading::Over);
    assert_eq!(draught::read_tenths(22_500, 10_000, 0), Reading::Unconfigured);
    assert_eq!(draught::read_tenths(22_500, u32::MAX, u32::MAX), Reading::Unconfigured);
}

#[test]
fn a_fraction_of_a_pascal_is_kept_where_a_whole_one_would_hide_it() {
    // 3000 tenths is 300.0 mV on the pin, 600 mV at the sensor, 100 tenths of a millivolt over its zero: 7.5 Pa.
    // Rounded to whole pascals this is 7, and the half a pascal that separates two readings is gone.
    let pa = number(draught::read_tenths(3_000, TOP, BOTTOM));
    assert!((pa - 7.5).abs() < 0.001, "{pa}");
    assert_eq!((number(draught::read_tenths(3_010, TOP, BOTTOM)) * 100.0).round(), 765.0);
}

// ---- the row a person reads while the regulator holds ----

#[test]
fn a_good_reading_reads_as_a_pressure_of_a_setpoint_at_an_angle() {
    let said = draught::holding_words(draught::Reading::Is(25.4), 30.0, 33.0);
    assert_eq!(said, "25 Pa of 30 Pa asked, at 33 % of air");
}

#[test]
fn a_fault_stands_whole_and_never_where_the_pressure_goes() {
    // **THE PAIR, AND THE DEFECT IT CLOSES.** The sensor's words are a SENTENCE, and dropped into the slot the
    // pascals live in they read as a pressure: "under 500 mV, its own zero - check the tube's port of 30 Pa
    // asked, at 33 % of air" told a person at a boiler two lies at once - that there was a reading, and what
    // it was. Every reading that is not a number has to come first and come whole.
    for said in [
        draught::Reading::Under,
        draught::Reading::Missing,
        draught::Reading::Over,
        draught::Reading::Unconfigured,
    ] {
        let row = draught::holding_words(said, 30.0, 33.0);
        let fault = draught::words(said);
        assert!(row.starts_with(&fault), "the fault does not open the row: {row}");
        assert!(row.contains("; the fan holds"), "the fault is not fenced off from what the fan is doing: {row}");
        // and the one shape that must never appear: the fault flowing into the number's grammar
        assert!(!row.contains(&format!("{fault} of ")), "the fault was put where the pressure goes: {row}");
    }
}

#[test]
fn a_reading_that_has_not_happened_yet_is_a_fault_too() {
    let row = draught::holding_words_unread(30.0, 33.0);
    assert!(row.starts_with("not read yet;"), "{row}");
    assert!(!row.contains("not read yet of"), "{row}");
}

// ---- the slider's own row, once commanding and saving became two acts ----

#[test]
fn the_row_carries_one_number_while_what_is_held_is_what_comes_back() {
    // NOTHING TO WARN ABOUT IS NOTHING TO SAY. A second clause repeating the first teaches a person to stop
    // reading the line, and then the day it differs they skip that too.
    assert_eq!(draught::asked_words(51.0, 51), "51 Pa asked");
}

#[test]
fn a_setpoint_that_was_not_saved_says_what_comes_back_and_what_to_press() {
    let row = draught::asked_words(60.0, 51);
    assert!(row.contains("60 Pa asked"), "what is held now is not named: {row}");
    assert!(row.contains("51 Pa"), "what the board comes up at is not named: {row}");
    assert!(row.contains("Save on this card"), "the row reports a difference and not what closes it: {row}");
    assert!(row.ends_with("60"), "what Save would keep is not named: {row}");
}

#[test]
fn the_row_never_says_a_number_the_regulator_is_not_holding() {
    // A pressure arrives as a float and is shown whole; rounding it one way in the first clause and another
    // in the second would print two different numbers for one setpoint.
    assert_eq!(draught::asked_words(50.6, 51), "51 Pa asked");
    assert_eq!(draught::asked_words(49.4, 51), "49 Pa asked; comes up at 51 Pa - Save on this card keeps 49");
}

#[test]
fn a_board_with_nothing_kept_still_says_both_numbers() {
    // A setting that never took a value reads as 0 Pa - which is stop, and not what the fan is being held at.
    // The row has to say so rather than hide the nought.
    let row = draught::asked_words(60.0, 0);
    assert!(row.contains("comes up at 0 Pa"), "a kept nought was swallowed: {row}");
}
