//! API of the soot: its entities - how many degrees the flue stands over the water in the burns, how many burns say it, the kilograms burned since the boiler was cleaned, the day of the cleaning, the button of it -,
//! the two edges of the scale as settings, and what the module hands the stream.
//! Names, kinds, units, limits; the address of each.
//! NEVER HERE: a card, a label, a percent, a phrase for a person ("the boiler wants cleaning"): the page makes the percent from the number and the edges, and page/menu.json says the words. Nor how the count goes - which seconds are a burn, the median, the grams since
//! the cleaning: that is crates/soot.
//! CHECKS: tests/soot_table.rs, tests/soot_values.rs.

use page::dictionary::{joined, Entry, Kind, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json.
pub const CARD: &str = "soot";

/// The settings of this module: the edges of the scale, the flue minus the water of a clean boiler and of a dirty one.
pub const SETTINGS: [&settings::Setting; 2] = [&burner_settings::SOOT_CLEAN_C, &burner_settings::SOOT_DIRTY_C];

/// The settings as rows of the settings card: what each keeps, as it is kept.
const SETTING_ROWS: [Entry; 2] = [
    Entry::row(page::SETTINGS, burner_settings::SOOT_CLEAN_C.name, &[Kind::Text]),
    Entry::row(page::SETTINGS, burner_settings::SOOT_DIRTY_C.name, &[Kind::Text]),
];

/// Why there is no number of the boiler: no burn of the working stage has lasted long enough yet.
pub const NO_BURNS: &str = "no_burns";

/// Why there is no day of the cleaning: the button was never pressed on a board that knew the date.
pub const NONE_YET: &str = "none_yet";

/// Why the cleaning was answered with a refusal: the flash did not take the count after it. The cleaning stands in memory and the board tries to write it again by itself; the page says it in its own words (page/menu.json, `ui.refusals`).
pub const NOT_WRITTEN: &str = "clean_not_written";

/// The flue minus the water, the median of the last burns, to a tenth of a degree: the count goes by half degrees.
const SOOT_C: Entry = Entry { unit: "°C", decimals: Some(1), reasons: &[NO_BURNS], ..Entry::row(CARD, "soot_c", &[Kind::Number, Kind::Unknown]) };

/// How many burns the number is made of, up to six.
const SOOT_BURNS: Entry = Entry { unit: "burns", decimals: Some(0), ..Entry::row(CARD, "soot_burns", &[Kind::Number]) };

/// Kilograms of pellets burned since the cleaning, to a tenth.
const KG_SINCE_CLEAN: Entry = Entry { unit: "kg", decimals: Some(1), ..Entry::row(CARD, "kg_since_clean", &[Kind::Number]) };

/// The day of the last cleaning, in days since 1970-01-01: the page makes a date of it.
const CLEANED_DAY: Entry = Entry { unit: "day", decimals: Some(0), reasons: &[NONE_YET], ..Entry::row(CARD, "cleaned_day", &[Kind::Number, Kind::Unknown]) };

/// The cleaning zeroes the kilograms since it: a client asks the person before it sends the command.
const CLEAN: Entry = Entry { can: &["press"], irreversible: true, ..Entry::row(CARD, "clean", &[Kind::Button]) };

const ROWS: [Entry; 5] = [SOOT_C, SOOT_BURNS, KG_SINCE_CLEAN, CLEANED_DAY, CLEAN];

/// Everything this module adds to the table: its settings' rows, then its own. It stands last in the burner's table (`crate::ALL`): a module laid on later moves no address of the ones before it.
pub const ENTRIES: [Entry; 7] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::at;
    pub const SOOT_C: usize = at(&crate::ALL, super::CARD, "soot_c");
    pub const SOOT_BURNS: usize = at(&crate::ALL, super::CARD, "soot_burns");
    pub const KG_SINCE_CLEAN: usize = at(&crate::ALL, super::CARD, "kg_since_clean");
    pub const CLEANED_DAY: usize = at(&crate::ALL, super::CARD, "cleaned_day");
}

/// What the stream is told of the soot, each by its address as `put` takes it: the median of the last burns in degrees (None - no burn counted yet), how many burns it is made of, the grams since the cleaning
/// as kilograms, and the day of the cleaning in days since 1970 (None - not known). The same four in every second.
pub fn put_values(soot_c: Option<f32>, burns: usize, grams_since_clean: u64, cleaned_day: Option<i64>, put: &mut dyn FnMut(usize, Value)) {
    put(at::SOOT_C, soot_c.map_or(Value::Unknown(NO_BURNS), Value::Number));
    put(at::SOOT_BURNS, Value::Number(burns as f32));
    put(at::KG_SINCE_CLEAN, Value::Number((grams_since_clean as f64 / 1000.0) as f32));
    put(at::CLEANED_DAY, cleaned_day.map_or(Value::Unknown(NONE_YET), |day| Value::Number(day as f32)));
}
