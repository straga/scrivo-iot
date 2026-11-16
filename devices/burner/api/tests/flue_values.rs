//! What the flue's module hands the stream for a word off the MAX6675: a number, an open thermocouple, a chip that says nothing - and nothing read yet. Expected
//! from the chip's datasheet (twelve bits of quarter degrees above three low bits, bit 2 set when the thermocouple is open) and from the first burner's board,
//! which says the same of the same words - not from the code that makes them.

use burner_api::flue::{at, value};
use page::dictionary::Value;

/// A word as the chip sends it: QUARTERS quarter degrees over the three low bits.
fn word(quarters: u16) -> u16 {
    quarters << 3
}

fn said(word: u16) -> Value {
    value(Some(max6675::read(word)))
}

#[test]
fn a_word_with_a_temperature_in_it_is_the_number_of_degrees() {
    assert_eq!(said(word(594)), Value::Number(148.5));
    assert_eq!(said(word(80)), Value::Number(20.0));
}

#[test]
fn a_word_with_the_open_bit_set_is_an_open_thermocouple_and_never_a_number() {
    assert_eq!(said(word(594) | 0x0004), Value::Unknown("open"));
}

#[test]
fn a_chip_that_sends_all_zeroes_or_all_ones_says_nothing_and_zero_degrees_is_not_what_a_cold_flue_reads() {
    assert_eq!(said(0x0000), Value::Unknown("silent"));
    assert_eq!(said(0xFFFF), Value::Unknown("silent"));
}

#[test]
fn before_the_first_word_the_flue_is_not_read_and_not_zero() {
    assert_eq!(value(None), Value::Unknown("not_read"));
}

#[test]
fn the_flue_has_an_address_of_its_own_in_the_table() {
    assert_eq!(burner_api::TABLE[at::FLUE_TEMP].name, "flue_temp");
    assert_eq!(burner_api::TABLE[at::FLUE_TEMP].card, "flue");
}
