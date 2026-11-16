//! API of the pellets: its entities - what the burner burned today, yesterday, this month and in all, in kilograms -, the rate the auger pours as a setting, and what the module hands the stream for the three counts.
//! Names, kinds, units, limits; the address of each.
//! NEVER HERE: a card, a label, a level, a phrase for a person, a word that the count is approximate: page/menu.json says them. Nor how the counting goes - seconds of the auger by the rate, the day and the month turning: that is crates/pellets.
//! CHECKS: tests/pellets_table.rs, tests/pellets_values.rs.

use page::dictionary::{joined, Entry, Kind, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json.
pub const CARD: &str = "pellets";

/// The settings of this module: what the auger pours in a minute of turning, as the owner weighed it.
pub const SETTINGS: [&settings::Setting; 1] = [&burner_settings::PELLET_RATE];

/// The settings as rows of the settings card: what each keeps, as it is kept.
const SETTING_ROWS: [Entry; 1] = [Entry::row(page::SETTINGS, burner_settings::PELLET_RATE.name, &[Kind::Text])];

/// A count in kilograms, to a tenth: the count is a count by the clock, and a gram more is not a thing it knows.
const fn kilograms(name: &'static str) -> Entry {
    Entry { unit: "kg", decimals: Some(1), ..Entry::row(CARD, name, &[Kind::Number]) }
}

/// A count of a day or a month, in kilograms: no number while the board is not sure of its day (`day_unsure`) - the kept numbers of a day it has not seen are not what was burned today.
const fn kilograms_of_a_day(name: &'static str) -> Entry {
    Entry { reasons: &[DAY_UNSURE], ..Entry { unit: "kg", decimals: Some(1), ..Entry::row(CARD, name, &[Kind::Number, Kind::Unknown]) } }
}

/// Why there is no number for today or the month: the count is not sure of its day.
pub const DAY_UNSURE: &str = "day_unsure";

/// Why there is no number for yesterday though the board is sure of its day: no day has turned since the count began to keep one.
pub const NONE_YET: &str = "none_yet";

/// Today, this month and in all.
const ROWS: [Entry; 3] = [kilograms_of_a_day("today"), kilograms_of_a_day("month"), kilograms("total")];

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; 4] = joined(&SETTING_ROWS, &ROWS);

/// Yesterday, in kilograms: what the day before today came to. The row stands apart from the three above, last in the burner's table (`crate::ALL`), because a row laid in among them would move the address of every row after it.
pub const LATE: [Entry; 1] = [Entry {
    reasons: &[DAY_UNSURE, NONE_YET],
    ..Entry { unit: "kg", decimals: Some(1), ..Entry::row(CARD, "yesterday", &[Kind::Number, Kind::Unknown]) }
}];

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::at;
    pub const TODAY: usize = at(&crate::ALL, super::CARD, "today");
    pub const MONTH: usize = at(&crate::ALL, super::CARD, "month");
    pub const TOTAL: usize = at(&crate::ALL, super::CARD, "total");
    pub const YESTERDAY: usize = at(&crate::ALL, super::CARD, "yesterday");
}

/// What the stream is told of the pellets: the kilograms of today, this month and in all, in that order, and yesterday's, each by its address as `put` takes it. The same four in every second.
/// DAY_SURE is whether the count is sure of its day (crates/tally): when it is not, today, yesterday and the month are the reason there is no number, and the total is the number it is - nothing is ever taken from it.
/// YESTERDAY is None while no day has turned since the count began to keep one.
pub fn put_values(kg: &[f32; 3], yesterday: Option<f32>, day_sure: bool, put: &mut dyn FnMut(usize, Value)) {
    let of_a_day = |n: f32| if day_sure { Value::Number(n) } else { Value::Unknown(DAY_UNSURE) };
    put(at::TODAY, of_a_day(kg[0]));
    put(at::MONTH, of_a_day(kg[1]));
    put(at::TOTAL, Value::Number(kg[2]));
    put(
        at::YESTERDAY,
        match (day_sure, yesterday) {
            (false, _) => Value::Unknown(DAY_UNSURE),
            (true, Some(n)) => Value::Number(n),
            (true, None) => Value::Unknown(NONE_YET),
        },
    );
}
