//! API of the mains: what the socket the burner is plugged into measures - power, voltage and current as three numbers, or why there are none - and the two settings that say where the socket is and how often it is asked.
//! Names, kinds, units, codes of reasons; the address of each.
//! NEVER HERE: a card, a label, a phrase for a person: page/menu.json says them. Nor how the socket is asked and its answer taken apart: that is crates/tasmota.
//! CHECKS: tests/mains_table.rs.

use page::dictionary::{joined, Entry, Kind, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json.
pub const CARD: &str = "mains";

/// The settings of this module: where the socket is, and how often it is asked.
pub const SETTINGS: [&settings::Setting; 2] = [&burner_settings::MAINS_AT, &burner_settings::MAINS_EVERY];

/// The settings as rows of the settings card: what each keeps, as it is kept.
const SETTING_ROWS: [Entry; 2] =
    [Entry::row(page::SETTINGS, burner_settings::MAINS_AT.name, &[Kind::Text]), Entry::row(page::SETTINGS, burner_settings::MAINS_EVERY.name, &[Kind::Text])];

/// Why there is no number: no socket is set, nothing has been read yet, the socket did not answer, or it answered and is not a meter this board can read.
pub const OFF: &str = "off";
pub const NOT_READ: &str = "not_read";
pub const SILENT: &str = "silent";
pub const UNREADABLE: &str = "unreadable";

const REASONS: &[&str] = &[OFF, NOT_READ, SILENT, UNREADABLE];

/// What the socket measures. A reading is three numbers or three reasons: a socket that does not answer is not a socket at zero.
const ROWS: [Entry; 3] = [
    Entry { unit: "W", decimals: Some(0), reasons: REASONS, ..Entry::row(CARD, "power", &[Kind::Number, Kind::Unknown]) },
    Entry { unit: "V", decimals: Some(0), reasons: REASONS, ..Entry::row(CARD, "voltage", &[Kind::Number, Kind::Unknown]) },
    Entry { unit: "A", decimals: Some(2), reasons: REASONS, ..Entry::row(CARD, "current", &[Kind::Number, Kind::Unknown]) },
];

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; 5] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::at;
    pub const POWER: usize = at(&crate::ALL, super::CARD, "power");
    pub const VOLTAGE: usize = at(&crate::ALL, super::CARD, "voltage");
    pub const CURRENT: usize = at(&crate::ALL, super::CARD, "current");
}

/// What the stream is told of the mains: the watts, volts and amps, or the same reason for all three.
pub fn put_values(reading: Result<(f32, f32, f32), &'static str>, put: &mut dyn FnMut(usize, Value)) {
    match reading {
        Ok((watts, volts, amps)) => {
            put(at::POWER, Value::Number(watts));
            put(at::VOLTAGE, Value::Number(volts));
            put(at::CURRENT, Value::Number(amps));
        }
        Err(why) => {
            put(at::POWER, Value::Unknown(why));
            put(at::VOLTAGE, Value::Unknown(why));
            put(at::CURRENT, Value::Unknown(why));
        }
    }
}
