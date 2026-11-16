//! What every device with a network keeps in its table after the system card: the clock's settings and the station's, as rows of the settings card,
//! and what a setting of crates/settings says of itself to a client. A device's table may lay them in an order of its own
//! (its `api` crate); a device that has none lays them as they stand here.

use page::dictionary::{joined, Entry, Kind};

/// The settings every board keeps - the clock's - as rows of the settings card, and what the time setting says now, by code.
pub const CLOCK: [Entry; 6] = [
    Entry::row(page::SETTINGS, settings::TIME.name, &[Kind::Text]),
    Entry::row(page::SETTINGS, settings::ZONE.name, &[Kind::Text]),
    Entry { words: &["came_from", "came_from_at", "silent_time_kept", "no_answer"], ..Entry::part(page::SETTINGS, settings::TIME.name, "note", &[Kind::Word]) },
    Entry { words: &["gateway"], ..Entry::part(page::SETTINGS, settings::TIME.name, "from", &[Kind::Word, Kind::Text]) },
    Entry::part(page::SETTINGS, settings::TIME.name, "gateway", &[Kind::Text]),
    Entry::part(page::SETTINGS, settings::TIME.name, "at", &[Kind::Text]),
];

/// The settings of the station's way out of a network it cannot join: rows of the settings card a device lays after its own, so the addresses of
/// the rows before them stay.
pub const NET: [Entry; 3] = [
    Entry::row(page::SETTINGS, settings::NET_POINT_TRIES.name, &[Kind::Text]),
    Entry::row(page::SETTINGS, settings::NET_RESET_LAPS.name, &[Kind::Text]),
    Entry::row(page::SETTINGS, settings::NET_REBOOT_LAPS.name, &[Kind::Text]),
];

/// The rows every board has: the system card, then the clock's settings.
pub const BOARD: [Entry; crate::system::ROWS.len() + CLOCK.len()] = joined(&crate::system::ROWS, &CLOCK);

/// The settings every board keeps, as a client lists them: the clock's, the station's - they stand on the tab Settings.
pub fn board_settings() -> Vec<page::dictionary::Setting> {
    settings::ENGINE.into_iter().map(|s| setting(s, &settings::NO_RULES)).collect()
}

/// A setting of crates/settings as a client needs it: name, default, what it takes - the whole numbers, the seconds, the choices and what each choice bears on.
/// No card: where a setting stands is the page's menu, and the board says only what the setting IS. What a choice bears on - the settings it shows, the role a socket plays for it - is the device's to say: its RULES.
pub fn setting(s: &'static settings::Setting, rules: &settings::Rules) -> page::dictionary::Setting {
    use page::dictionary::{Needs, Setting, Shape, Variant};
    Setting {
        name: s.name,
        card: None,
        default: s.default,
        tuning: s.level == settings::Level::Tuning,
        effect: None,
        shape: match &s.kind {
            settings::Kind::Choice(choices) => {
                let variants: Vec<Variant> = choices
                    .iter()
                    .map(|c| {
                        let bears = (rules.bearing)(s.name, c.value);
                        Variant {
                            value: c.value,
                            takes: c.takes.is_some(),
                            shows: bears.map_or(&[], |b| b.shows),
                            needs: bears.and_then(|b| b.role).map(|holds| Needs { among: rules.among, holds }),
                        }
                    })
                    .collect();
                // made when the image is built, once, and kept for the rest of the build
                Shape::Choice(Box::leak(variants.into_boxed_slice()))
            }
            settings::Kind::Months { from, to } => Shape::Months { from: *from, to: *to },
            settings::Kind::Seconds { from, to } => Shape::Seconds { from: *from, to: *to },
            settings::Kind::Tenths { from, to, unit } => Shape::Tenths { from: *from, to: *to, unit },
            settings::Kind::Whole { from, to, unit } => Shape::Whole { from: *from, to: *to, unit },
        },
    }
}
