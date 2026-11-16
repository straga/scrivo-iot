//! API of the demand: its entities - who the house says cold or warm by and why, the profile that stands with what chose it and the window and when it ends, the target of the room, how long the room's sensor has been silent - and the eleven
//! settings of its card: who asks, the two targets of the room and its hysteresis, the profile outside the windows and the six windows that choose between them.
//! NEVER HERE: a card, an order, a label, a level of the page, a phrase for a person: page/menu.json says them. Nor who says cold and who says warm (crates/demand) or a number of the burner (crates/settings).
//! CHECKS: tests/demand_table.rs, tests/demand_values.rs.

use page::dictionary::{joined, Entry, Kind, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json.
pub const CARD: &str = "demand";

/// The settings of this module: who asks the boiler for heat, the targets of the room and its hysteresis, and the profiles with the six windows that choose between them.
pub const SETTINGS: [&settings::Setting; 11] = [
    &burner_settings::AUTO_ASKED_BY,
    &burner_settings::AUTO_ROOM_C,
    &burner_settings::AUTO_ROOM_ECO_C,
    &burner_settings::AUTO_ROOM_GAP_C,
    &burner_settings::AUTO_PROFILE,
    &burner_settings::WINDOW_1,
    &burner_settings::WINDOW_2,
    &burner_settings::WINDOW_3,
    &burner_settings::WINDOW_4,
    &burner_settings::WINDOW_5,
    &burner_settings::WINDOW_6,
];

/// The rows of the demand: what the house says and why, how long the room's sensor has been silent, what stands as the profile now, what chose it with the window and when it ends, and the target of the room.
const ROWS: [Entry; 8] = [
    // none: nobody plays the role of the demand, and the boiler works by its water alone
    Entry { words: &::demand::DEMANDS, reasons: &["none"], ..Entry::row(CARD, "asked", &[Kind::Word, Kind::Unknown]) },
    Entry { words: &::demand::WHYS, reasons: &["none"], ..Entry::row(CARD, "why", &[Kind::Word, Kind::Unknown]) },
    // how long the sensor of the room has been silent, nothing while it speaks or no socket plays its role
    Entry::row(CARD, "silent_room", &[Kind::Seconds]),
    // what stands as the profile now: the profile, what chose it, the window's number and the minute of the day it ends at; none while nobody asks
    Entry { words: &::schedule::PROFILES, reasons: &["none"], ..Entry::row(CARD, "profile", &[Kind::Word, Kind::Unknown]) },
    Entry { words: &::schedule::SOURCES, reasons: &["none"], ..Entry::row(CARD, "source", &[Kind::Word, Kind::Unknown]) },
    // the window's number and the minute of the day it ends at are parts of the source: a page says them in the one line the source is
    Entry { unit: "window", decimals: Some(0), reasons: &["none"], ..Entry::part(CARD, "source", "window", &[Kind::Number, Kind::Unknown]) },
    Entry { unit: "min", decimals: Some(0), reasons: &["none"], ..Entry::part(CARD, "source", "until", &[Kind::Number, Kind::Unknown]) },
    // the target of the room under the profile that stands; none where the source is not the room and under a profile that holds none
    Entry { unit: "°C", decimals: Some(1), reasons: &["none"], ..Entry::row(CARD, "target", &[Kind::Number, Kind::Unknown]) },
];

/// The settings as rows of the settings card: what each keeps, as it is kept.
const fn setting_rows() -> [Entry; SETTINGS.len()] {
    let mut out = [Entry::row("", "", &[]); SETTINGS.len()];
    let mut i = 0;
    while i < SETTINGS.len() {
        out[i] = Entry::row(page::SETTINGS, SETTINGS[i].name, &[Kind::Text]);
        i += 1;
    }
    out
}

const SETTING_ROWS: [Entry; SETTINGS.len()] = setting_rows();

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; SETTING_ROWS.len() + ROWS.len()] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::at;

    const fn row(name: &str) -> usize {
        at(&crate::ALL, super::CARD, name)
    }

    pub const ASKED: usize = row("asked");
    pub const WHY: usize = row("why");
    pub const SILENT_ROOM: usize = row("silent_room");
    pub const PROFILE: usize = row("profile");
    pub const SOURCE: usize = row("source");
    pub const WINDOW: usize = page::dictionary::part_at(&crate::ALL, super::CARD, "source", "window");
    pub const UNTIL: usize = page::dictionary::part_at(&crate::ALL, super::CARD, "source", "until");
    pub const TARGET: usize = row("target");
}

/// What the demand says now.
pub struct Now {
    /// What the house said in the last order and why; None while nobody plays the role.
    pub asked: Option<::demand::Asked>,
    /// How long the room's sensor has been silent, in whole seconds: past three periods of its round, when its reading is no longer given out. None - it speaks, or no socket plays its role.
    pub silent_room: Option<u32>,
    /// The profile that stands and what chose it; None while nobody asks.
    pub profile: Option<::schedule::Profile>,
    pub source: Option<::schedule::Source>,
    /// The minute of the local day the window in force ends at; None outside a window.
    pub until_min: Option<u16>,
    /// The target of the room, in degrees.
    pub target_c: Option<f32>,
}

/// What the stream is told of the demand for NOW: every row of the card, each by its address as `put` takes it. **THE SAME ADDRESSES IN EVERY STATE.**
pub fn put_values(now: &Now, put: &mut dyn FnMut(usize, Value)) {
    put(at::ASKED, now.asked.map_or(Value::Unknown("none"), |asked| Value::Word(::demand::demand_word(asked.demand))));
    put(at::WHY, now.asked.map_or(Value::Unknown("none"), |asked| Value::Word(::demand::why_word(asked.why))));
    put(at::SILENT_ROOM, now.silent_room.map_or(Value::None, Value::Seconds));
    put(at::PROFILE, now.profile.map_or(Value::Unknown("none"), |profile| Value::Word(::schedule::profile_word(profile))));
    put(at::SOURCE, now.source.map_or(Value::Unknown("none"), |source| Value::Word(::schedule::source_word(source))));
    put(at::WINDOW, match now.source {
        Some(::schedule::Source::Window(number)) => Value::Number(f32::from(number)),
        _ => Value::Unknown("none"),
    });
    put(at::UNTIL, now.until_min.map_or(Value::Unknown("none"), |minute| Value::Number(f32::from(minute))));
    put(at::TARGET, now.target_c.map_or(Value::Unknown("none"), Value::Number));
}
