//! API of the water's temperatures: its entities - one reading per socket of the bus module with the role that socket plays and its curve, whether any socket
//! has been given a role, the state of the bus with its counts -, the settings of the bus - the role of each socket, the module's address, the pace of the round -
//! and what the module hands the stream for a round. Names, kinds, units, limits, codes of words and reasons; the address of each.
//! NEVER HERE: a card, an order, a label, a level, a phrase for a person, the limits of a picture: page/menu.json says them. Nor a UART, nor a frame being
//! asked: that is src/hardware; what a round of the bus came to is crates/modbus and crates/burner_map, which this file only gives names to.
//! CHECKS: tests/temperatures_table.rs, tests/temperatures_values.rs.

use burner_map::{Readings, COUNT};
use modbus::{Counts, Reading};
use page::dictionary::{joined, Entry, Kind, Said, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json and in burner.yaml.
pub const CARD: &str = "temperatures";
/// The curves of the sockets, one each, in the order of the map.
pub const CURVES: [&str; COUNT] = ["water_1", "water_2", "water_3", "water_4", "water_5", "water_6", "water_7"];
/// The roles a socket can play, by code (`settings::ROLES` by value); a name of one's own comes as text.
pub const ROLE_CODES: [&str; 7] = ["free", "boiler", "supply", "return", "flue", "hopper", "room"];

/// The settings of this module, in the order a client lists them: the role of each socket, the module's address, the pace of the round.
pub const SETTINGS: [&settings::Setting; 9] = [
    &burner_settings::ROLE_NTC_1,
    &burner_settings::ROLE_NTC_2,
    &burner_settings::ROLE_NTC_3,
    &burner_settings::ROLE_NTC_4,
    &burner_settings::ROLE_NTC_5,
    &burner_settings::ROLE_NTC_6,
    &burner_settings::ROLE_NTC_7,
    &burner_settings::NTC_ADDRESS,
    &burner_settings::POLL_EVERY,
];

/// A socket's reading: a number in degrees, or the reason there is none.
const fn channel(name: &'static str, curve: &'static [&'static str]) -> Entry {
    Entry { unit: "°C", decimals: Some(1), reasons: &["no_sensor", "not_read"], curves: curve, ..Entry::row(CARD, name, &[Kind::Number, Kind::Unknown]) }
}

/// The role a socket plays, a part of its reading.
const fn role(channel: &'static str) -> Entry {
    Entry { words: &ROLE_CODES, ..Entry::part(CARD, channel, "role", &[Kind::Word, Kind::Text]) }
}

/// The rows of the water's temperatures: each socket and its role, then whether any socket is given one, then the bus with its counts.
const ROWS: [Entry; 21] = [
    channel("ntc_1", &["water_1"]),
    role("ntc_1"),
    channel("ntc_2", &["water_2"]),
    role("ntc_2"),
    channel("ntc_3", &["water_3"]),
    role("ntc_3"),
    channel("ntc_4", &["water_4"]),
    role("ntc_4"),
    channel("ntc_5", &["water_5"]),
    role("ntc_5"),
    channel("ntc_6", &["water_6"]),
    role("ntc_6"),
    channel("ntc_7", &["water_7"]),
    role("ntc_7"),
    // on while every socket is free
    Entry::row(CARD, "none_assigned", &[Kind::On]),
    Entry { words: &["ok", "losing"], ..Entry::row(CARD, "bus", &[Kind::Word]) },
    Entry { unit: "askings", decimals: Some(0), ..Entry::part(CARD, "bus", "read", &[Kind::Number]) },
    Entry { unit: "askings", decimals: Some(0), ..Entry::part(CARD, "bus", "silent", &[Kind::Number]) },
    Entry { unit: "askings", decimals: Some(0), ..Entry::part(CARD, "bus", "broken", &[Kind::Number]) },
    Entry { unit: "askings", decimals: Some(0), ..Entry::part(CARD, "bus", "foreign", &[Kind::Number]) },
    // the silent askings of the last full window; none before the first window closes
    Entry { unit: "askings", decimals: Some(0), ..Entry::part(CARD, "bus", "last_silent", &[Kind::Number]) },
];

/// The settings as rows of the settings card: what each keeps, as it is kept.
const SETTING_ROWS: [Entry; SETTINGS.len()] = {
    let mut out = [Entry::row("", "", &[]); SETTINGS.len()];
    let mut i = 0;
    while i < SETTINGS.len() {
        out[i] = Entry::row(page::SETTINGS, SETTINGS[i].name, &[Kind::Text]);
        i += 1;
    }
    out
};

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; SETTINGS.len() + ROWS.len()] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::{at, part_at};
    const fn channel(name: &str) -> usize {
        at(&crate::ALL, super::CARD, name)
    }
    const fn role(name: &str) -> usize {
        part_at(&crate::ALL, super::CARD, name, "role")
    }
    pub const CHANNEL: [usize; 7] = [channel("ntc_1"), channel("ntc_2"), channel("ntc_3"), channel("ntc_4"), channel("ntc_5"), channel("ntc_6"), channel("ntc_7")];
    pub const CHANNEL_ROLE: [usize; 7] = [role("ntc_1"), role("ntc_2"), role("ntc_3"), role("ntc_4"), role("ntc_5"), role("ntc_6"), role("ntc_7")];
    pub const NONE_ASSIGNED: usize = at(&crate::ALL, super::CARD, "none_assigned");
    pub const BUS: usize = at(&crate::ALL, super::CARD, "bus");
    pub const BUS_READ: usize = part_at(&crate::ALL, super::CARD, "bus", "read");
    pub const BUS_SILENT: usize = part_at(&crate::ALL, super::CARD, "bus", "silent");
    pub const BUS_BROKEN: usize = part_at(&crate::ALL, super::CARD, "bus", "broken");
    pub const BUS_FOREIGN: usize = part_at(&crate::ALL, super::CARD, "bus", "foreign");
    pub const BUS_LAST_SILENT: usize = part_at(&crate::ALL, super::CARD, "bus", "last_silent");
}

/// What the stream is told of the water's temperatures for what the last round READ, with each socket's ROLE as it is kept and the bus's COUNTS: every value by
/// its address, as `put` takes it. A number is a number, a socket with nothing in it and a module that did not answer are two reasons of their own, and the role
/// of a socket is its code when it is one of the list and the words that were written when it is not. What the readings mean is crates/burner_map's; here they
/// are only given their codes.
pub fn put_values(readings: &Readings, roles: &[&str; COUNT], counts: &Counts, put: &mut dyn FnMut(usize, Said<'_>)) {
    let mut assigned = 0;
    for (n, (said, stored)) in readings.iter().zip(roles).enumerate() {
        put(
            at::CHANNEL[n],
            Said::Value(match said {
                Reading::Is(celsius) => Value::Number(*celsius),
                Reading::Empty => Value::Unknown("no_sensor"),
                Reading::Unknown => Value::Unknown("not_read"),
            }),
        );
        if *stored != burner_settings::FREE {
            assigned += 1;
        }
        put(
            at::CHANNEL_ROLE[n],
            match ROLE_CODES.iter().find(|code| **code == *stored) {
                Some(code) => Said::Value(Value::Word(code)),
                None => Said::Text(stored.split_once(':').map_or(*stored, |(_, own)| own)),
            },
        );
    }
    put(at::NONE_ASSIGNED, Said::Value(Value::On(assigned == 0)));
    let now = counts.now;
    let lost = now.missed + now.broken + now.foreign;
    put(at::BUS, Said::Value(Value::Word(if lost == 0 { "ok" } else { "losing" })));
    put(at::BUS_READ, Said::Value(Value::Number(now.accepted as f32)));
    put(at::BUS_SILENT, Said::Value(Value::Number(now.missed as f32)));
    put(at::BUS_BROKEN, Said::Value(Value::Number(now.broken as f32)));
    put(at::BUS_FOREIGN, Said::Value(Value::Number(now.foreign as f32)));
    put(at::BUS_LAST_SILENT, Said::Value(counts.last.map_or(Value::None, |t| Value::Number(t.missed as f32))));
}
