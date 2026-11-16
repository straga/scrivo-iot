//! The card every board carries: its console and its station, and what the board is - its memory, how long it has
//! run, its clock. The rows of `page::SYSTEM`; each device's table starts with them.

use page::dictionary::{at, part_at, Entry, Kind};
use page::SYSTEM;

/// The rows of the system card; a device's table starts with them, so their addresses are the same on every board.
pub const ROWS: [Entry; 15] = [
    Entry { unit: "consoles", decimals: Some(0), ..Entry::row(SYSTEM, "console", &[Kind::Number]) },
    Entry { words: &["joined", "not_joined"], ..Entry::row(SYSTEM, "wifi", &[Kind::Word]) },
    Entry::part(SYSTEM, "wifi", "ip", &[Kind::Text]),
    Entry::part(SYSTEM, "wifi", "ssid", &[Kind::Text]),
    // how loud the station is heard; none while not joined
    Entry { unit: "dBm", decimals: Some(0), ..Entry::part(SYSTEM, "wifi", "signal", &[Kind::Number]) },
    // WHAT THE BOARD IS, beside its rows: read by whoever shows the memory and the clock, never a row of the card; in
    // whole kilobytes' steps, so a heap that breathes by a few bytes sends nothing
    Entry { unit: "bytes", decimals: Some(0), ..Entry::row(SYSTEM, "free", &[Kind::Number]) },
    Entry { unit: "bytes", decimals: Some(0), ..Entry::row(SYSTEM, "used", &[Kind::Number]) },
    Entry::row(SYSTEM, "low_memory", &[Kind::On]),
    // sent when a reader opens and not after: the reader counts on from it
    Entry { unit: "ms", decimals: Some(0), once: true, ..Entry::row(SYSTEM, "uptime", &[Kind::Number]) },
    // the local time, `2026-09-24 18:40`; none before a server answered - not in the network yet, or silent
    Entry { reasons: &["not_joined", "silent"], ..Entry::row(SYSTEM, "time", &[Kind::Text, Kind::Unknown]) },
    Entry::part(SYSTEM, "time", "zone", &[Kind::Text]),
    // the server that answered: `gateway` for the router, else its host as the setting names it
    Entry { words: &["gateway"], ..Entry::part(SYSTEM, "time", "from", &[Kind::Word, Kind::Text]) },
    Entry::part(SYSTEM, "time", "gateway", &[Kind::Text]),
    // the servers asked that do not answer, the router apart: the other names, a comma between, and whether it is one
    Entry::part(SYSTEM, "time", "silent", &[Kind::Text]),
    Entry::part(SYSTEM, "time", "router_silent", &[Kind::On]),
];

/// How busy each core of the board is, in per cent of the last five seconds (`core_load`): none before the first window is over. THEY ARE NOT IN `ROWS`: a row added there would move
/// the address of everything a device's table holds after the card, and the clients were given those. A device that wants them puts them LAST in its table; one that has not got
/// them says nothing, and a page that does not find them draws nothing.
pub const CORES: [Entry; 2] = [
    Entry { unit: "%", decimals: Some(0), ..Entry::row(SYSTEM, "core_0", &[Kind::Number]) },
    Entry { unit: "%", decimals: Some(0), ..Entry::row(SYSTEM, "core_1", &[Kind::Number]) },
];

/// Whether TABLE ends with the two rows of [`CORES`], in their order: what a device's table must do, and what each device's own check asks of it by this one function.
pub fn cores_stand_last(table: &[Entry]) -> bool {
    table.len() >= CORES.len() && table[table.len() - CORES.len()..] == CORES[..]
}

pub const CONSOLE: usize = at(&ROWS, SYSTEM, "console");
pub const WIFI: usize = at(&ROWS, SYSTEM, "wifi");
pub const WIFI_IP: usize = part_at(&ROWS, SYSTEM, "wifi", "ip");
pub const WIFI_SSID: usize = part_at(&ROWS, SYSTEM, "wifi", "ssid");
pub const WIFI_SIGNAL: usize = part_at(&ROWS, SYSTEM, "wifi", "signal");
pub const FREE: usize = at(&ROWS, SYSTEM, "free");
pub const USED: usize = at(&ROWS, SYSTEM, "used");
pub const LOW_MEMORY: usize = at(&ROWS, SYSTEM, "low_memory");
pub const UPTIME: usize = at(&ROWS, SYSTEM, "uptime");
pub const TIME: usize = at(&ROWS, SYSTEM, "time");
pub const TIME_ZONE: usize = part_at(&ROWS, SYSTEM, "time", "zone");
pub const TIME_FROM: usize = part_at(&ROWS, SYSTEM, "time", "from");
pub const TIME_GATEWAY: usize = part_at(&ROWS, SYSTEM, "time", "gateway");
pub const TIME_SILENT: usize = part_at(&ROWS, SYSTEM, "time", "silent");
pub const TIME_ROUTER_SILENT: usize = part_at(&ROWS, SYSTEM, "time", "router_silent");
