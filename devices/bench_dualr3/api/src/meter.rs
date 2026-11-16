//! API of the BL0939 meter: the voltage, and for each output of the box - L1 and L2 - its power with its current apart, and the packets the board counted of the meter's answers over a window of askings -
//! the one filling now and the last full one. What the module hands the stream for them. A reading the meter did not give has a reason, not a zero: zero current and power prove nothing, a silent meter gives them too.
//! NEVER HERE: a pin, a baud rate, the checksum of a packet (the device, crates/bl0939), a word for a person (page/menu.json).
//! CHECKS: tests/api.rs, tests/values.rs.

use page::dictionary::{joined, Entry, Kind, Value};

/// The card the meter stands on.
pub const CARD: &str = "meter";
/// How many askings a window of packets counts: the meter's own (crates/bl0939, `WINDOW`), held to it in tests/api.rs.
pub const WINDOW: u32 = 1000;

const VOLTAGE: Entry = Entry { unit: "V", decimals: Some(1), reasons: &["not_read"], ..Entry::row(CARD, "voltage", &[Kind::Number, Kind::Unknown]) };

/// An output: its power as the row, its current as a part.
const fn output(name: &'static str) -> [Entry; 2] {
    [
        Entry { unit: "W", decimals: Some(1), reasons: &["not_read"], ..Entry::row(CARD, name, &[Kind::Number, Kind::Unknown]) },
        Entry { unit: "A", decimals: Some(3), reasons: &["not_read"], ..Entry::part(CARD, name, "amps", &[Kind::Number, Kind::Unknown]) },
    ]
}

/// A count of packets: a number with the unit the page says it in.
const fn packets(row: &'static str, name: &'static str) -> Entry {
    Entry { unit: "packets", decimals: Some(0), ..Entry::part(CARD, row, name, &[Kind::Number]) }
}

const OUTPUTS: [Entry; 4] = joined(&output("l1"), &output("l2"));
const NOW: [Entry; 5] = [
    Entry { words: &["ok", "losing"], ..Entry::row(CARD, "packets", &[Kind::Word]) },
    packets("packets", "accepted"),
    packets("packets", "rejected"),
    packets("packets", "missed"),
    Entry { fixed: Some(WINDOW as f32), unit: "packets", ..Entry::part(CARD, "packets", "window", &[Kind::Number]) },
];
const LAST: [Entry; 4] = [
    Entry { words: &["ok", "losing"], reasons: &["no_window"], ..Entry::row(CARD, "last", &[Kind::Word, Kind::Unknown]) },
    packets("last", "rejected"),
    packets("last", "missed"),
    Entry { fixed: Some(WINDOW as f32), unit: "packets", ..Entry::part(CARD, "last", "window", &[Kind::Number]) },
];
const READINGS: [Entry; 5] = joined(&[VOLTAGE], &OUTPUTS);
const WINDOWS: [Entry; 9] = joined(&NOW, &LAST);

/// The rows of the meter: the voltage, each output, then the two windows of packets.
pub const ENTRIES: [Entry; 14] = joined(&READINGS, &WINDOWS);

/// The addresses of the meter's values in the device's table (`crate::ALL`).
pub mod at {
    use page::dictionary::{at, part_at};
    pub const VOLTAGE: usize = at(&crate::ALL, super::CARD, "voltage");
    /// Power and current of L1, L2.
    pub const POWER: [usize; 2] = [at(&crate::ALL, super::CARD, "l1"), at(&crate::ALL, super::CARD, "l2")];
    pub const AMPS: [usize; 2] = [part_at(&crate::ALL, super::CARD, "l1", "amps"), part_at(&crate::ALL, super::CARD, "l2", "amps")];
    pub const PACKETS: usize = at(&crate::ALL, super::CARD, "packets");
    pub const ACCEPTED: usize = part_at(&crate::ALL, super::CARD, "packets", "accepted");
    pub const REJECTED: usize = part_at(&crate::ALL, super::CARD, "packets", "rejected");
    pub const MISSED: usize = part_at(&crate::ALL, super::CARD, "packets", "missed");
    pub const LAST: usize = at(&crate::ALL, super::CARD, "last");
    pub const LAST_REJECTED: usize = part_at(&crate::ALL, super::CARD, "last", "rejected");
    pub const LAST_MISSED: usize = part_at(&crate::ALL, super::CARD, "last", "missed");
}

/// What one output of the box draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Output {
    pub amps: f32,
    pub watts: f32,
}

/// What the meter said in the last packet it gave, the outputs in the box's order (L1, L2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub volts: f32,
    pub outputs: [Output; 2],
}

/// How the askings of a window came out: a whole packet, one rejected for its checksum, or none in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub accepted: u32,
    pub rejected: u32,
    pub missed: u32,
}

/// The meter now: the last reading - none before the first packet -, the window filling and the last one filled, if one has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Now {
    pub reading: Option<Reading>,
    pub now: Window,
    pub last: Option<Window>,
}

/// How a window of packets went, by code: `ok` when every asking brought a whole packet, `losing` when one did not.
fn link(window: &Window) -> &'static str {
    if window.rejected + window.missed == 0 {
        "ok"
    } else {
        "losing"
    }
}

/// What the stream is told of the meter: the reading, or why there is none; the window filling and the last full one.
pub fn put_values(now: &Now, put: &mut dyn FnMut(usize, Value)) {
    let number = |value: f32| Value::Number(value);
    match &now.reading {
        Some(r) => {
            put(at::VOLTAGE, number(r.volts));
            for (n, output) in r.outputs.iter().enumerate() {
                put(at::POWER[n], number(output.watts));
                put(at::AMPS[n], number(output.amps));
            }
        }
        None => {
            put(at::VOLTAGE, Value::Unknown("not_read"));
            for n in 0..2 {
                put(at::POWER[n], Value::Unknown("not_read"));
                put(at::AMPS[n], Value::Unknown("not_read"));
            }
        }
    }
    put(at::PACKETS, Value::Word(link(&now.now)));
    put(at::ACCEPTED, number(now.now.accepted as f32));
    put(at::REJECTED, number(now.now.rejected as f32));
    put(at::MISSED, number(now.now.missed as f32));
    match &now.last {
        Some(window) => {
            put(at::LAST, Value::Word(link(window)));
            put(at::LAST_REJECTED, number(window.rejected as f32));
            put(at::LAST_MISSED, number(window.missed as f32));
        }
        None => {
            put(at::LAST, Value::Unknown("no_window"));
            put(at::LAST_REJECTED, Value::None);
            put(at::LAST_MISSED, Value::None);
        }
    }
}
