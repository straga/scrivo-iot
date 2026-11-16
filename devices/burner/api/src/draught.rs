//! API of the draught: its entity - the pressure of the blast in pascals, the top of the scale as at least it, or the code of why there is no number: not read, under the sensor's own
//! zero, nothing on the pin, a divider that is no divider -, the pin under it with the parts a person judges the number by, the curves it has, the two arms of the divider as settings
//! with the part that says where the sensor's own zero lands on the pin, and what the module hands the stream for a window. Names, kinds, units, limits, codes of words and reasons;
//! the address of each.
//! NEVER HERE: a card, an order, a label, a level, a phrase for a person, the limits of a picture: page/menu.json says them - and it says whether a reason is a fault or none. Nor a pin, nor a
//! window being read: that is src/hardware; what the millivolts mean is crates/draught, which this file only gives names to.
//! CHECKS: tests/draught_table.rs, tests/draught_values.rs.

use ::draught::Reading as Pressure;
use page::dictionary::{joined, Entry, Kind, Value};

use crate::pin::{self, Part, Reading};

/// The name this module's rows go by, and the name of its entry in page/menu.json and in burner.yaml.
pub const CARD: &str = "draught";
/// The curves of the sensor: a minute at ten a second, a quarter of an hour at one.
pub const CURVES: [&str; 2] = ["draught_fast", "draught_slow"];

/// The arms of the divider the pin sits behind, as the settings a client lists: what a voltmeter says, written in.
pub const SETTINGS: [&settings::Setting; 2] = [&burner_settings::DIVIDER_TOP, &burner_settings::DIVIDER_BOTTOM];

/// A number of the sensor fixed in the image: part of the pressure row, never sent.
const fn constant(name: &'static str, value: f32, unit: &'static str) -> Entry {
    Entry { unit, fixed: Some(value), ..Entry::part(CARD, "blast_pressure", name, &[Kind::Number]) }
}

const PIN: [Entry; 7] = pin::rows(CARD, &CURVES);

/// The rows of the draught: the pressure with the three numbers of the sensor that it is judged by, then the pin under it.
const ROWS: [Entry; 11] = [
    Entry {
        unit: "Pa", decimals: Some(0),
        reasons: &["not_read", "own_zero", "missing", "unconfigured"],
        ..Entry::row(CARD, "blast_pressure", &[Kind::Number, Kind::AtLeast, Kind::Unknown])
    },
    constant("zero_mv", ::draught::ZERO_MV as f32, "mV"),
    constant("missing_below_mv", ::draught::MISSING_BELOW_MV as f32, "mV"),
    constant("full_pa", ::draught::FULL_PA as f32, "Pa"),
    PIN[0],
    PIN[1],
    PIN[2],
    PIN[3],
    PIN[4],
    PIN[5],
    PIN[6],
];

/// The divider's arms as rows of the settings card, and the part of the bottom one that says where the sensor's own zero lands on the pin.
const SETTING_ROWS: [Entry; 3] = [
    Entry::row(page::SETTINGS, burner_settings::DIVIDER_TOP.name, &[Kind::Text]),
    Entry::row(page::SETTINGS, burner_settings::DIVIDER_BOTTOM.name, &[Kind::Text]),
    Entry { unit: "mV", decimals: Some(0), reasons: &["not_a_divider"], ..Entry::part(page::SETTINGS, burner_settings::DIVIDER_BOTTOM.name, "zero_at", &[Kind::Number, Kind::Unknown]) },
];

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; 14] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::{at, part_at};
    pub const BLAST_PRESSURE: usize = at(&crate::ALL, super::CARD, "blast_pressure");
    pub const PIN: usize = at(&crate::ALL, super::CARD, "pin");
    pub const SPREAD: usize = part_at(&crate::ALL, super::CARD, "pin", "spread");
    pub const EDGE: usize = part_at(&crate::ALL, super::CARD, "pin", "edge");
    pub const WINDOW: usize = part_at(&crate::ALL, super::CARD, "pin", "window");
    pub const ZERO_AT: usize = part_at(&crate::ALL, page::SETTINGS, burner_settings::DIVIDER_BOTTOM.name, "zero_at");
}

/// What the stream is told of the draught for a window of the pin - None while it has read nothing - read through DIVIDER, the two arms in ohms: the pressure by its address, and the pin
/// under it. What the millivolts mean is crates/draught's; here they are only given their codes. A pin under the sensor's own zero is `own_zero` and no more: whether that is a fault is
/// the menu's word, for a still fan reads it every day.
pub fn put_values(reading: Option<Reading>, divider: (u32, u32), put: &mut dyn FnMut(usize, Value)) {
    put(
        at::BLAST_PRESSURE,
        match reading.map(|r| ::draught::read(u32::from(r.window.mv()), divider.0, divider.1)) {
            None => Value::Unknown("not_read"),
            Some(Pressure::Is(pa)) => Value::Number(pa),
            Some(Pressure::Over) => Value::AtLeast(::draught::FULL_PA as f32),
            Some(Pressure::Under) => Value::Unknown("own_zero"),
            Some(Pressure::Missing) => Value::Unknown("missing"),
            Some(Pressure::Unconfigured) => Value::Unknown("unconfigured"),
        },
    );
    pin::put(reading, &mut |part, value| {
        put(
            match part {
                Part::Pin => at::PIN,
                Part::Spread => at::SPREAD,
                Part::Edge => at::EDGE,
                Part::Window => at::WINDOW,
            },
            value,
        )
    });
}

/// Where the sensor's own zero lands on the pin through DIVIDER - the millivolts a voltmeter on the pin says while no pressure is on the sensor -, or that the arms are no divider.
pub fn zero_at(divider: (u32, u32)) -> Value {
    ::draught::pin_mv(::draught::ZERO_MV, divider.0, divider.1).map_or(Value::Unknown("not_a_divider"), |mv| Value::Number(mv as f32))
}
