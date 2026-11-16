//! API of the planned restart: its entity - how many minutes ago the board last restarted itself by plan - and the hour it restarts from as a setting.
//! Names, kinds, limits; the address of each.
//! NEVER HERE: a card, a label, a phrase for a person: page/menu.json says them. Nor when a restart is due and what it waits for: that is crates/daily.
//! CHECKS: tests/restart_table.rs.

use page::dictionary::{joined, Entry, Kind, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json.
pub const CARD: &str = "restart";

/// The settings of this module: the hour the board restarts from.
pub const SETTINGS: [&settings::Setting; 1] = [&burner_settings::RESTART_HOUR];

/// The settings as rows of the settings card: what each keeps, as it is kept.
const SETTING_ROWS: [Entry; 1] = [Entry::row(page::SETTINGS, burner_settings::RESTART_HOUR.name, &[Kind::Text])];

/// Why there is no number: the board has not restarted by plan since this record began, or it does not know the time to say how long ago.
pub const NONE_YET: &str = "none_yet";
pub const NO_CLOCK: &str = "no_clock";

/// How long ago the board last restarted itself by plan, in minutes.
const ROWS: [Entry; 1] = [Entry { unit: "min", decimals: Some(0), reasons: &[NONE_YET, NO_CLOCK], ..Entry::row(CARD, "last", &[Kind::Number, Kind::Unknown]) }];

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; 2] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::at;
    pub const LAST: usize = at(&crate::ALL, super::CARD, "last");
}

/// What the stream is told of the last planned restart: minutes since it, or why there is no number.
pub fn put_value(ago_min: Result<u32, &'static str>, put: &mut dyn FnMut(usize, Value)) {
    put(at::LAST, match ago_min {
        Ok(min) => Value::Number(min as f32),
        Err(why) => Value::Unknown(why),
    });
}
