//! What the MAX6675 says in one reading, and nothing about wires. The chip clocks out sixteen bits and the
//! meaning of every one of them is here; the SPI that fetches them is the device's half (devices/burner/src).
//!
//! Bits: 15 empty, 14..3 the temperature in quarters of a degree, 2 the open
//! thermocouple, 1 the chip's own id, 0 the state of its input.

/// The open-thermocouple bit: set, the chip is saying it has nothing to measure.
const OPEN: u16 = 1 << 2;
/// Where the twelve bits of temperature sit, and what one of them is worth.
const SHIFT: u32 = 3;
const QUARTERS: u16 = 0x0FFF;
const STEP: f32 = 0.25;

/// What one reading of the chip amounts to. Three outcomes and no fourth: a number, a thermocouple that is not
/// there, and no answer at all - and neither of the last two is ever a number, because 0 °C is what a cold flue
/// reads, and a person would act on it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Reading {
    Is(f32),
    Open,
    Silent,
}

/// What a WORD off the chip means.
///
/// All zeroes and all ones are taken as silence rather than as readings. Nothing on the wire clocks in as zeroes,
/// a wire held high clocks in as ones, and the temperatures they would otherwise stand for - 0 °C exactly, or the
/// very top of the range - are not numbers a thermocouple settles on. On 20.09 a PTA9B01 sent forty identical
/// readings and it took an hour to see that the sensor was simply not there: an unchanging number is the shape
/// absence takes, so it is named absence here instead of being shown as a measurement.
pub fn read(word: u16) -> Reading {
    if word == 0x0000 || word == 0xFFFF {
        return Reading::Silent;
    }
    if word & OPEN != 0 {
        return Reading::Open;
    }
    Reading::Is(f32::from(word >> SHIFT & QUARTERS) * STEP)
}

/// The reading as a person reads it. A refusal carries no digits at all: a number on that row reads as a
/// measurement whatever words stand beside it.
pub fn words(reading: Reading) -> String {
    match reading {
        Reading::Is(celsius) => format!("{celsius:.1} °C"),
        Reading::Open => "thermocouple open".into(),
        Reading::Silent => "no answer from the chip".into(),
    }
}
