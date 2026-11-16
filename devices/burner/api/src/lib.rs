//! API: what the burner says outward - the dictionary of its entities (name, kind, unit, limits, key), their values, their commands, the stream.
//! A crate of its own on purpose: it depends on `page` and `dictionary` and on nothing of the hardware, so it cannot reach a pin.
//! HERE THERE IS NO SCREEN: no card, no order, no "show it there", no level, no word for a person, no legend, no limit of a picture. Those are
//! page/menu.json. One file per module names that module's entities; the table below is the board's own rows and the modules' after them.
//! CHECKS: tests/api.rs, tests/board_settings.rs.

use dictionary::board::{board_settings, BOARD, NET};
use page::dictionary::{joined, Entry};

pub mod automaton;
pub mod demand;
pub mod draught;
pub mod fan;
pub mod first_burner;
pub mod flame;
pub mod flue;
pub mod mains;
pub mod pellets;
pub mod pin;
pub mod relays;
pub mod restart;
pub mod soot;
pub mod temperatures;
pub mod thermostat;


/// The board's rows and the rows of the relays, then the temperatures', then the flue's, then the fan's, then the draught's, then the flame's.
const ORGANS: [Entry; BOARD.len() + relays::ENTRIES.len()] = joined(&BOARD, &relays::ENTRIES);
const WATER: [Entry; ORGANS.len() + temperatures::ENTRIES.len()] = joined(&ORGANS, &temperatures::ENTRIES);
const GASES: [Entry; WATER.len() + flue::ENTRIES.len()] = joined(&WATER, &flue::ENTRIES);

const BLOWER: [Entry; GASES.len() + fan::ENTRIES.len()] = joined(&GASES, &fan::ENTRIES);
const AIR: [Entry; BLOWER.len() + draught::ENTRIES.len()] = joined(&BLOWER, &draught::ENTRIES);
const FIRE: [Entry; AIR.len() + flame::ENTRIES.len()] = joined(&AIR, &flame::ENTRIES);
const HEAT: [Entry; FIRE.len() + thermostat::ENTRIES.len()] = joined(&FIRE, &thermostat::ENTRIES);
const LOGIC: [Entry; HEAT.len() + automaton::ENTRIES.len()] = joined(&HEAT, &automaton::ENTRIES);
/// The burner's table: the board's rows, then - as modules are laid through - theirs, the thermostat's and the automaton's, and the pellets' after them: a module laid on later stands last, so the addresses of the ones before it stay.
const PELLETED: [Entry; LOGIC.len() + pellets::ENTRIES.len()] = joined(&LOGIC, &pellets::ENTRIES);
/// The same, and the station's settings after the pellets'.
const NETTED: [Entry; PELLETED.len() + NET.len()] = joined(&PELLETED, &NET);
/// The same, and the demand's.
const DEMANDED: [Entry; NETTED.len() + demand::ENTRIES.len()] = joined(&NETTED, &demand::ENTRIES);
/// The same, and the planned restart's: a module laid on later stands last.
const RESTARTED: [Entry; DEMANDED.len() + restart::ENTRIES.len()] = joined(&DEMANDED, &restart::ENTRIES);
/// The same, and the pellets' yesterday after it: a row added to a card that stands in the middle of the table would move every address after it.
const YESTERDAY: [Entry; RESTARTED.len() + pellets::LATE.len()] = joined(&RESTARTED, &pellets::LATE);
/// The same, and the automaton's silent pin after it, for the same reason.
const SILENCED: [Entry; YESTERDAY.len() + automaton::LATE.len()] = joined(&YESTERDAY, &automaton::LATE);
/// The same, and the thermostat's manual cycle after it, for the same reason.
const MANUAL: [Entry; SILENCED.len() + thermostat::LATE.len()] = joined(&SILENCED, &thermostat::LATE);
/// The same, and the mains' (the socket the burner is plugged into) after it, for the same reason.
const PLUGGED: [Entry; MANUAL.len() + mains::ENTRIES.len()] = joined(&MANUAL, &mains::ENTRIES);
/// The same, and the soot's (how clean the boiler is) after it, for the same reason.
const SOOTED: [Entry; PLUGGED.len() + soot::ENTRIES.len()] = joined(&PLUGGED, &soot::ENTRIES);
/// The same, and how busy each core is (`dictionary::system::CORES`, rows of the board's card kept out of its first rows) after it, for the same reason.
pub const ALL: [Entry; SOOTED.len() + dictionary::system::CORES.len()] = joined(&SOOTED, &dictionary::system::CORES);
/// The same, where the image keeps it once.
pub static TABLE: [Entry; ALL.len()] = ALL;

/// The dictionary the burner's image serves, the image being KIT: made when the image is built, from [`ALL`].
pub fn json(kit: &str) -> String {
    let settings: Vec<_> = [board_settings(), relays::settings(), temperatures::settings(), fan::settings(), draught::settings(), flame::settings(), thermostat::settings(), demand::settings(), automaton::settings(), pellets::settings(), restart::settings(), mains::settings(), soot::settings()].concat();
    page::dictionary::json(
        &ALL,
        &page::dictionary::Meta { device: "burner", board: "STRAGA_BASE_S3", kit, cards: &[thermostat::CARD, demand::CARD, automaton::CARD, relays::CARD, temperatures::CARD, flue::CARD, fan::CARD, draught::CARD, flame::CARD, pellets::CARD, restart::CARD, mains::CARD, soot::CARD], settings: &settings, machine: None },
    )
}
