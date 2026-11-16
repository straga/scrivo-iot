//! What sixteen bits off a MAX6675 mean. Every case here is a word, not a wire: the chip is not needed to say
//! what 0x0964 stands for, and what the chip actually answers is the board's business.

use max6675::{read, words, Reading};

/// The word a chip sends for a temperature: TEMP in quarters of a degree, shifted into bits 14..3.
fn word_of(quarters: u16) -> u16 {
    quarters << 3
}

#[test]
fn a_word_carries_the_temperature_in_quarters_of_a_degree() {
    // Zero degrees is written with the chip's own low bit set, because the bare 0x0000 is silence and is tested
    // as such below - the two cannot both be true of one word, and silence is the one that matters.
    assert_eq!(read(word_of(0) | 1), Reading::Is(0.0));
    assert_eq!(read(word_of(1)), Reading::Is(0.25));
    assert_eq!(read(word_of(4)), Reading::Is(1.0));
    assert_eq!(read(word_of(100)), Reading::Is(25.0));
    assert_eq!(read(word_of(806)), Reading::Is(201.5));
}

#[test]
fn the_highest_reading_the_chip_can_give_is_read_whole() {
    // twelve bits of quarters: 4095 of them, 1023.75 C - the top of what this chip reports at all
    assert_eq!(read(word_of(4095)), Reading::Is(1023.75));
}

#[test]
fn the_bits_that_are_not_the_temperature_do_not_change_it() {
    // bit 15 is empty, bit 1 is the chip's id, bit 0 is the state of its input: a reading is the same number
    // whatever they happen to be, and a driver that masks them wrongly shifts every degree it reports
    for extra in [0u16, 1 << 15, 1 << 1, 1, (1 << 15) | (1 << 1) | 1] {
        assert_eq!(read(word_of(400) | extra), Reading::Is(100.0), "extra bits {extra:#06x} moved the reading");
    }
}

#[test]
fn an_open_thermocouple_is_said_as_such_and_never_as_a_number() {
    // THE POINT OF THE WHOLE CRATE. A thermocouple taken off reads as bit 2, and the temperature bits beside it
    // are meaningless - drawn as a number they read as a cold flue, which is the one thing a person must not be
    // told when there is no thermocouple at all.
    assert_eq!(read(word_of(0) | 0b100), Reading::Open);
    assert_eq!(read(word_of(400) | 0b100), Reading::Open);
    assert_eq!(read(word_of(4095) | 0b100), Reading::Open);
}

#[test]
fn a_silent_bus_is_not_zero_degrees() {
    // Nothing on the wire reads as all zeroes, and a wire held high reads as all ones. Either is the absence of
    // an answer, and 0 C exactly is what a board would show for it - the same trap the PTA9B01 set on 20.09,
    // where forty identical readings looked like a measurement.
    assert_eq!(read(0x0000), Reading::Silent);
    assert_eq!(read(0xFFFF), Reading::Silent);
}

#[test]
fn a_word_that_is_neither_silence_nor_open_is_a_reading() {
    // the pair to the case above: everything else the chip can send is a temperature, including one bit short of
    // silence at either end
    assert_eq!(read(0x0001), Reading::Is(0.0));
    assert_eq!(read(0xFFF9), Reading::Is(1023.75));
}

#[test]
fn what_a_person_reads_says_the_state_in_words() {
    assert_eq!(words(Reading::Is(201.5)), "201.5 °C");
    assert_eq!(words(Reading::Is(0.0)), "0.0 °C");
    assert_eq!(words(Reading::Open), "thermocouple open");
    assert_eq!(words(Reading::Silent), "no answer from the chip");
}

#[test]
fn the_words_of_a_refusal_carry_no_number() {
    // a person scanning a card must not find a temperature-looking thing on a row that has none
    for said in [words(Reading::Open), words(Reading::Silent)] {
        assert!(!said.contains("°C"), "{said:?} reads as a measurement");
        assert!(!said.chars().any(|c| c.is_ascii_digit()), "{said:?} carries a number");
    }
}
