//! API of an analog pin read through a window of samples: what every module that reads one says of it - its millivolts, how far the samples of one window spread, where the number
//! stands against the top of the converter and against the window the calibration holds in, and the numbers of the window fixed in the image. Two modules read such a pin - the
//! flame's and the draught's - and say it the same way, so it is said here once.
//! NEVER HERE: what the millivolts mean (crates/flame, crates/draught), a card, a label, a level, a phrase for a person: page/menu.json says them.
//! CHECKS: tests/draught_values.rs, tests/flame_values.rs.

use analog::Window;
use page::dictionary::{Entry, Kind, Value};

/// How many samples one window of a pin is made of, said as a number fixed in the image. The hardware reads that many (src/hardware/adc.rs holds the two equal at build time).
pub const SAMPLES: usize = 20;

/// One window of a pin as the hardware gave it: the millivolts with their spread, and whether every sample stood at the top of the scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub window: Window,
    pub at_ceiling: bool,
}

/// A number fixed in the image: part of a row, never sent.
const fn constant(card: &'static str, row: &'static str, name: &'static str, value: f32, unit: &'static str) -> Entry {
    Entry { unit, fixed: Some(value), ..Entry::part(card, row, name, &[Kind::Number]) }
}

/// The seven entities of a pin of CARD: its millivolts - which draw the CURVES - with the spread of the window, the samples a window is made of, where the number stands against the top
/// of the converter (the stronger fact of the two, said beside the number) and against the calibrated window, and the two ends of that window.
pub const fn rows(card: &'static str, curves: &'static [&'static str]) -> [Entry; 7] {
    [
        Entry { unit: "mV", decimals: Some(0), reasons: &["not_read"], curves, ..Entry::row(card, "pin", &[Kind::Number, Kind::Unknown]) },
        Entry { unit: "mV", decimals: Some(0), ..Entry::part(card, "pin", "spread", &[Kind::Number]) },
        constant(card, "pin", "samples", SAMPLES as f32, "samples"),
        Entry { words: &["top", "under", "over", "inside"], ..Entry::part(card, "pin", "edge", &[Kind::Word]) },
        Entry { words: &["under", "inside", "over"], ..Entry::part(card, "pin", "window", &[Kind::Word]) },
        constant(card, "pin", "window_from", analog::WINDOW_MV.0 as f32, "mV"),
        constant(card, "pin", "window_to", analog::WINDOW_MV.1 as f32, "mV"),
    ]
}

/// The parts of a pin that are sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Pin,
    Spread,
    Edge,
    Window,
}

/// What is said of a pin for READING - None while it has read nothing - each part as `put` takes it. A pin that read nothing is not read, and not a pin at zero.
pub fn put(reading: Option<Reading>, put: &mut dyn FnMut(Part, Value)) {
    match reading {
        None => {
            put(Part::Pin, Value::Unknown("not_read"));
            for part in [Part::Spread, Part::Edge, Part::Window] {
                put(part, Value::None);
            }
        }
        Some(r) => {
            put(Part::Pin, Value::Number(f32::from(r.window.mv())));
            put(Part::Spread, Value::Number(f32::from(r.window.spread())));
            put(Part::Edge, Value::Word(analog::edge(r.at_ceiling, r.window.mv())));
            put(Part::Window, Value::Word(analog::standing(r.window.mv())));
        }
    }
}
