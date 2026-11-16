//! API of the flame: its entities - the state by code (out, weak, steady, why there is none), the pin's millivolts with their spread and where the
//! number stands, the curves it has, the two thresholds as settings with their limits and the order they stand in - and what the module hands the
//! stream for a reading. Names, kinds, units, limits, codes of words and reasons; the address of each.
//! NEVER HERE: a card, an order, a label, a level, a phrase for a person, the limits of a picture: page/menu.json says them. Nor a pin, nor a window
//! being read: that is src/hardware; what the millivolts mean is crates/flame, which this file only gives names to.
//! CHECKS: tests/flame_table.rs, tests/flame_values.rs.

use flame::{Flame, Thresholds};
use page::dictionary::{joined, Entry, Kind, Value};

use crate::pin::{self, Part};

pub use crate::pin::{Reading, SAMPLES};

/// The name this module's rows go by, and the name of its entry in page/menu.json and in burner.yaml.
pub const CARD: &str = "flame";
/// The curves of the pin: a minute at ten a second, a quarter of an hour at one.
pub const CURVES: [&str; 2] = ["flame_fast", "flame_slow"];

const PIN: [Entry; 7] = pin::rows(CARD, &CURVES);

/// The rows of the flame: its state, and the pin under it with the parts a person judges the number by.
const ROWS: [Entry; 8] = [
    Entry { words: &["out", "weak", "steady"], reasons: &["not_read", "crossed"], ..Entry::row(CARD, "flame", &[Kind::Word, Kind::Unknown]) },
    PIN[0],
    PIN[1],
    PIN[2],
    PIN[3],
    PIN[4],
    PIN[5],
    PIN[6],
];

/// The two thresholds as rows of the settings card, and the part of the second that says whether they stand in the order they must.
const SETTING_ROWS: [Entry; 3] = [
    Entry::row(page::SETTINGS, burner_settings::FLAME_LIT.name, &[Kind::Text]),
    Entry::row(page::SETTINGS, burner_settings::FLAME_STEADY.name, &[Kind::Text]),
    Entry { words: &["in_order", "crossed"], ..Entry::part(page::SETTINGS, burner_settings::FLAME_STEADY.name, "order", &[Kind::Word]) },
];

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; 11] = joined(&SETTING_ROWS, &ROWS);

/// The two thresholds as a client needs them: name, default, the whole numbers they take. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    [&burner_settings::FLAME_LIT, &burner_settings::FLAME_STEADY].into_iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::{at, part_at};
    pub const FLAME: usize = at(&crate::ALL, super::CARD, "flame");
    pub const PIN: usize = at(&crate::ALL, super::CARD, "pin");
    pub const SPREAD: usize = part_at(&crate::ALL, super::CARD, "pin", "spread");
    pub const EDGE: usize = part_at(&crate::ALL, super::CARD, "pin", "edge");
    pub const WINDOW: usize = part_at(&crate::ALL, super::CARD, "pin", "window");
    pub const STEADY_ORDER: usize = part_at(&crate::ALL, page::SETTINGS, burner_settings::FLAME_STEADY.name, "order");
}

/// The address of the value called NAME - `flame`, `pin`, `spread`, `edge`, `window` - for whoever puts values by address.
pub fn address(name: &str) -> usize {
    match name {
        "flame" => at::FLAME,
        "pin" => at::PIN,
        "spread" => at::SPREAD,
        "edge" => at::EDGE,
        "window" => at::WINDOW,
        _ => panic!("the flame has no value called {name}"),
    }
}

/// What the stream is told of the flame for READING - None while the pin has read nothing - through THRESHOLDS: each value by its name, as `put`
/// takes it. What the millivolts mean is crates/flame's; here they are only given their codes.
pub fn put_values(reading: Option<Reading>, thresholds: Thresholds, put: &mut dyn FnMut(&'static str, Value)) {
    put(
        "flame",
        match reading.map(|r| flame::read(u32::from(r.window.mv()), thresholds)) {
            None => Value::Unknown("not_read"),
            Some(Flame::Out) => Value::Word("out"),
            Some(Flame::Weak) => Value::Word("weak"),
            Some(Flame::Steady) => Value::Word("steady"),
            // thresholds that are not thresholds say no state of the fire
            Some(Flame::Unconfigured) => Value::Unknown("crossed"),
        },
    );
    pin::put(reading, &mut |part, value| {
        put(
            match part {
                Part::Pin => "pin",
                Part::Spread => "spread",
                Part::Edge => "edge",
                Part::Window => "window",
            },
            value,
        )
    });
}

/// Whether the two thresholds stand in the order they must - steady under lit, because more light is fewer millivolts - as the part `order` of the
/// second says it.
pub fn order_value(thresholds: Thresholds) -> Value {
    Value::Word(if thresholds.sound() { "in_order" } else { "crossed" })
}
