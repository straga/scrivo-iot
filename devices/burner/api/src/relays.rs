//! API of the relays: its entities - the four organs, each a switch that takes on, off and a toggle, the seconds a limit leaves of the auger and of the igniter -, the auger's limit as a setting, and
//! what the module hands the stream for the states of its organs. Names, kinds, units, limits, commands; the address of each.
//! NEVER HERE: a card, an order, a label, a level, a phrase for a person, the limits of a picture: page/menu.json says them. Nor a pin, nor the level that closes a relay: that is src/hardware; when a
//! limit opens an organ is crates/relay, which this file only gives names to. Who holds an organ - the automaton's claim - is the automaton's card, not a row of this one.
//! CHECKS: tests/relays_table.rs, tests/relays_values.rs.

use page::dictionary::{joined, Count, Entry, Kind, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json and in burner.yaml.
pub const CARD: &str = "relays";
/// The organs by name, in the order of the relays: what a person switches, and what the automaton keeps while it runs the burner.
pub const NAMES: [&str; 4] = ["auger", "igniter", "pump", "aux_power"];

/// The settings of this module: how long the auger may turn before it stops by itself. The igniter's limit is no setting - it follows from the automaton's time of the igniter.
pub const SETTINGS: [&settings::Setting; 1] = [&burner_settings::AUGER_LIMIT];

/// An organ: a switch, by hand from the page and the console.
const fn organ(name: &'static str) -> Entry {
    relay::entry::row(CARD, name)
}

/// The rows of the relays: each organ, and after the auger and the igniter the seconds a running limit leaves of them.
const ROWS: [Entry; 6] = [
    organ("auger"),
    Entry { counts: Some(Count::Down), ..Entry::part(CARD, "auger", "off_in", &[Kind::Seconds]) },
    organ("igniter"),
    Entry { counts: Some(Count::Down), ..Entry::part(CARD, "igniter", "off_in", &[Kind::Seconds]) },
    organ("pump"),
    organ("aux_power"),
];

/// The settings as rows of the settings card: what each keeps, as it is kept.
const SETTING_ROWS: [Entry; 1] = [Entry::row(page::SETTINGS, burner_settings::AUGER_LIMIT.name, &[Kind::Text])];

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; 7] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::{at, part_at};
    pub const RELAY: [usize; 4] = [at(&crate::ALL, super::CARD, "auger"), at(&crate::ALL, super::CARD, "igniter"), at(&crate::ALL, super::CARD, "pump"), at(&crate::ALL, super::CARD, "aux_power")];
    /// Where the seconds a limit leaves stand, by organ: only the auger and the igniter have a limit to count.
    pub const RELAY_OFF_IN: [Option<usize>; 4] =
        [Some(part_at(&crate::ALL, super::CARD, "auger", "off_in")), Some(part_at(&crate::ALL, super::CARD, "igniter", "off_in")), None, None];
}

/// What an organ is now: closed or open, and while a limit runs, the whole seconds it has left - rounded up, for a limit with a tenth left is not yet at zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Organ {
    pub closed: bool,
    pub left_s: Option<u32>,
}

/// What the stream is told of the organs, in the order of NAMES: on or off, and the seconds left of the limits that run. Each value by its address, as `put` takes it.
pub fn put_values(organs: &[Organ; 4], put: &mut dyn FnMut(usize, Value)) {
    for (n, organ) in organs.iter().enumerate() {
        put(at::RELAY[n], Value::On(organ.closed));
        if let Some(off_in) = at::RELAY_OFF_IN[n] {
            put(off_in, organ.left_s.map_or(Value::None, Value::Seconds));
        }
    }
}
