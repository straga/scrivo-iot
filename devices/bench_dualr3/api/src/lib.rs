//! API: what the Sonoff Dual R3 says outward - the dictionary of its entities (name, kind, unit, key), their values, their commands.
//! A crate of its own on purpose: it depends on `page`, `dictionary` and `settings` and on nothing of the hardware, so it cannot reach a pin.
//! HERE THERE IS NO SCREEN: no card title, no order, no label, no level, no phrase for a person. Those are `devices/bench_dualr3/page/menu.json`.
//! One file per module names that module's entities; the table below is the board's own rows and the modules' after them.
//! CHECKS: tests/api.rs, tests/values.rs.

use dictionary::board::{board_settings, BOARD, NET};
use page::dictionary::{joined, Entry};

pub mod energy;
pub mod meter;
pub mod relays;
pub mod switches;

/// The board's rows, then the relays', the keys', the meter's, and the energy's after them: a module laid on later stands last, so the addresses of the ones before it stay.
const WALL: [Entry; BOARD.len() + relays::ENTRIES.len()] = joined(&BOARD, &relays::ENTRIES);
const KEYED: [Entry; WALL.len() + switches::ENTRIES.len()] = joined(&WALL, &switches::ENTRIES);
const METERED: [Entry; KEYED.len() + meter::ENTRIES.len()] = joined(&KEYED, &meter::ENTRIES);
const COUNTED: [Entry; METERED.len() + energy::ENTRIES.len()] = joined(&METERED, &energy::ENTRIES);
/// Everything above, and the station's settings after it.
const NETTED: [Entry; COUNTED.len() + NET.len()] = joined(&COUNTED, &NET);
/// The Dual R3's table: everything above, and how busy each core is last of all (`dictionary::system::CORES`): the card's first rows stay where every board has them.
pub const ALL: [Entry; NETTED.len() + dictionary::system::CORES.len()] = joined(&NETTED, &dictionary::system::CORES);
/// The same, where the image keeps it once.
pub static TABLE: [Entry; ALL.len()] = ALL;

/// The device's name in the dictionary and on the page.
pub const DEVICE: &str = "bench_dualr3";
/// The board it runs on, by the name of its `boards/<board>`.
pub const BOARD_NAME: &str = "SONOFF_DUAL_R3";

/// The cards of the device, in the order its page draws them.
pub const CARDS: [&str; 4] = [relays::CARD, switches::CARD, meter::CARD, energy::CARD];

/// The dictionary the Dual's image serves, the image being KIT: made when the image is built, from [`ALL`].
pub fn json(kit: &str) -> String {
    json_for(kit, DEVICE, BOARD_NAME)
}

/// The same dictionary for an image that carries the Dual's modules under another name - the stand, the Dual's firmware on a bench S3 -, called what the build says it is
/// (STRAGA_DEVICE, STRAGA_BOARD of tools/board_env.py): a person who opens the stand's page reads the stand's name there, not the Dual's.
pub fn json_for(kit: &str, device: &str, board: &str) -> String {
    let settings: Vec<_> = [board_settings(), energy::settings()].concat();
    page::dictionary::json(&ALL, &page::dictionary::Meta { device, board, kit, cards: &CARDS, settings: &settings, machine: None })
}
