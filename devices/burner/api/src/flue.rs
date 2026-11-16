//! API of the flue gas: its entity - the temperature, with the three reasons there may be none - the curve it has and what the module hands the stream for a
//! reading. Names, kinds, units, codes of reasons; the address.
//! NEVER HERE: a card, an order, a label, a level, a phrase for a person, the limits of a picture: page/menu.json says them. Nor a pin, nor a word being read off
//! the chip: that is src/hardware; what the sixteen bits mean is crates/max6675, which this file only gives names to.
//! CHECKS: tests/flue_table.rs, tests/flue_values.rs.

use max6675::Reading;
use page::dictionary::{Entry, Kind, Value};

/// The name this module's row goes by, and the name of its entry in page/menu.json and in burner.yaml.
pub const CARD: &str = "flue";
/// The curve of the flue: a quarter of an hour at one a second.
pub const CURVE: &str = "flue";

/// Everything this module adds to the table: the temperature of the gases.
pub const ENTRIES: [Entry; 1] =
    [Entry { unit: "°C", decimals: Some(1), reasons: &["open", "silent", "not_read"], curves: &[CURVE], ..Entry::row(CARD, "flue_temp", &[Kind::Number, Kind::Unknown]) }];

/// The address of this module's value in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::at;
    pub const FLUE_TEMP: usize = at(&crate::ALL, super::CARD, "flue_temp");
}

/// What the stream is told of the flue for what the chip last said - None while it has said nothing -: the degrees, or why there are none. A thermocouple that is
/// open and a chip that answers nothing are never a number, and a flue that has not been read is not a flue at zero.
pub fn value(reading: Option<Reading>) -> Value {
    match reading {
        Some(Reading::Is(celsius)) => Value::Number(celsius),
        Some(Reading::Open) => Value::Unknown("open"),
        Some(Reading::Silent) => Value::Unknown("silent"),
        None => Value::Unknown("not_read"),
    }
}
