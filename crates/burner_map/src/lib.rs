//! What the burner reads, as one table - and everything that depends on how many readings there are, counted from
//! that table rather than written beside it.
//!
//! This is the whole point of describing readings instead of coding them: **adding a reading is a line in MAP and
//! nothing else.** No array length, no buffer, no loop bound is written by hand anywhere, so the day the flue gas
//! probe arrives, its line goes in and the code that asks the bus, keeps the last values and draws the card is not
//! touched. The sizes below are `MAP.len()` on purpose; a number written twice is a number that will disagree with
//! itself.
//!
//! The hardware half - the UART, the round, nvs, the page - lives in devices/burner. Here there is nothing that
//! needs a board, which is why it is tested on the computer.

use modbus::{Ask, Channel, Reading, Scale};
use settings::Setting;

/// One reading of the burner: what it is called, where it is asked for, and what its number means.
///
/// The address is NOT here - it is a setting, and which setting is what this says. A module remembers its own
/// address; the board only remembers where to ask.
#[derive(Clone, Copy)]
pub struct Spot {
    /// What a person reads on the card, beside the number.
    pub name: &'static str,
    /// The setting that says where this module answers.
    pub address: &'static Setting,
    /// The setting that says what this channel measures - a role out of the list, or a name of one's own. Where
    /// the wires actually go is a fact of the boiler room, and it has no business being written in code.
    pub role: &'static Setting,
    /// The register that holds the reading.
    pub reg: u16,
    /// The unit, shown as it stands.
    pub unit: &'static str,
    pub scale: Scale,
    /// The raw value that means "nothing is plugged in here"; None when the module has no such value.
    pub empty: Option<u16>,
}

/// THE MAP. Adding a reading is a line here - and only here.
///
/// Seven channels of the NT18B07, in the order of its sockets. The names are the module's own until the owner says
/// what is plugged into each; an empty socket answers 0xF555, and that is "no sensor", not -273.1 degrees.
///
/// The flue gas of the PTA9B01 is not here: it has no probe, and without one the module answers an unchanging
/// 377.2 C, which on a page reads as a temperature and is not one. Its line goes in with the probe.
pub const MAP: [Spot; 7] = [
    Spot { name: "ntc 1", address: &burner_settings::NTC_ADDRESS, role: &burner_settings::ROLE_NTC_1, reg: 0x0000, unit: "°C", scale: Scale::Tenths, empty: Some(0xF555) },
    Spot { name: "ntc 2", address: &burner_settings::NTC_ADDRESS, role: &burner_settings::ROLE_NTC_2, reg: 0x0001, unit: "°C", scale: Scale::Tenths, empty: Some(0xF555) },
    Spot { name: "ntc 3", address: &burner_settings::NTC_ADDRESS, role: &burner_settings::ROLE_NTC_3, reg: 0x0002, unit: "°C", scale: Scale::Tenths, empty: Some(0xF555) },
    Spot { name: "ntc 4", address: &burner_settings::NTC_ADDRESS, role: &burner_settings::ROLE_NTC_4, reg: 0x0003, unit: "°C", scale: Scale::Tenths, empty: Some(0xF555) },
    Spot { name: "ntc 5", address: &burner_settings::NTC_ADDRESS, role: &burner_settings::ROLE_NTC_5, reg: 0x0004, unit: "°C", scale: Scale::Tenths, empty: Some(0xF555) },
    Spot { name: "ntc 6", address: &burner_settings::NTC_ADDRESS, role: &burner_settings::ROLE_NTC_6, reg: 0x0005, unit: "°C", scale: Scale::Tenths, empty: Some(0xF555) },
    Spot { name: "ntc 7", address: &burner_settings::NTC_ADDRESS, role: &burner_settings::ROLE_NTC_7, reg: 0x0006, unit: "°C", scale: Scale::Tenths, empty: Some(0xF555) },
];

/// How many readings there are. Counted from the map; never written down.
pub const COUNT: usize = MAP.len();

/// The most askings the map can come to - one per reading, when every one of them sits apart.
pub const MAX_ASKS: usize = COUNT;

/// What the last round read, one per reading, in the order of the map.
pub type Readings = [Reading; COUNT];

/// Nothing read yet.
pub const UNREAD: Readings = [Reading::Unknown; COUNT];

/// A map turned into channels, ADDRESS saying where each module answers now. Taken afresh every round, so an
/// address changed on the page works from the next round.
///
/// Written over a map of ANY length on purpose: the day a line is added, this is called exactly as before and the
/// new length follows the map. A function that knew the length would have to be edited, and that is the thing the
/// description was supposed to spare us.
pub fn channels<const N: usize>(map: &[Spot; N], address: impl Fn(&'static Setting) -> u8) -> [Channel; N] {
    let mut made = [Channel { name: "", addr: 0, reg: 0, unit: "", scale: Scale::Tenths, empty: None }; N];
    for (channel, spot) in made.iter_mut().zip(map) {
        *channel = Channel {
            name: spot.name,
            addr: address(spot.address),
            reg: spot.reg,
            unit: spot.unit,
            scale: spot.scale,
            empty: spot.empty,
        };
    }
    made
}

/// The askings that cover CHANNELS, written into OUT; answers how many. OUT holds one asking per channel, which is
/// the worst a map can come to - every reading on a module of its own.
pub fn asks<const N: usize>(channels: &[Channel; N], out: &mut [Ask; N]) -> usize {
    modbus::asks(channels, out)
}

/// What one reading says to a person: the number with its unit, or why there is no number.
pub fn words(spot: &Spot, reading: Reading) -> String {
    match reading {
        Reading::Is(value) => format!("{value:.1} {}", spot.unit),
        // not a temperature and not a fault: the socket is there and nothing is in it
        Reading::Empty => "no sensor".into(),
        Reading::Unknown => "-".into(),
    }
}

/// What a card says when every socket is free: not an empty card, and not a silence a person has to interpret.
pub const NOTHING_ASSIGNED: (&str, &str) =
    ("no sensors", "say what each socket measures in the settings below");

/// The rows of the card and of the shell: what each channel measures and what it reads, in the order of the map.
///
/// A channel nobody has assigned is not shown at all - a socket with nothing in it is a fact about the module, not
/// a temperature, and seven rows of which three are empty is a page that lies by its shape. When none is assigned
/// the card says so in words: an empty card would read as a broken one.
pub fn rows<const N: usize>(
    map: &[Spot; N],
    read: &[Reading; N],
    role: impl Fn(&'static Setting) -> String,
) -> Vec<(String, String)> {
    let named: Vec<(String, String)> = map
        .iter()
        .zip(read)
        .filter_map(|(spot, reading)| {
            let stored = role(spot.role);
            (stored != burner_settings::FREE).then(|| (burner_settings::role_words(&stored), words(spot, *reading)))
        })
        .collect();
    match named.is_empty() {
        true => vec![(NOTHING_ASSIGNED.0.to_string(), NOTHING_ASSIGNED.1.to_string())],
        false => named,
    }
}

/// What each row of [`rows`] reads as a number with its unit, row for row in the same order; None where the row has
/// no number - a socket that answered nothing, and the one row of a card with nothing assigned. What the page draws
/// by, beside the words of [`rows`].
pub fn values<const N: usize>(
    map: &[Spot; N],
    read: &[Reading; N],
    role: impl Fn(&'static Setting) -> String,
) -> Vec<Option<(f32, &'static str)>> {
    let named: Vec<Option<(f32, &'static str)>> = map
        .iter()
        .zip(read)
        .filter(|(spot, _)| role(spot.role) != burner_settings::FREE)
        .map(|(spot, reading)| match reading {
            Reading::Is(value) => Some((*value, spot.unit)),
            Reading::Empty | Reading::Unknown => None,
        })
        .collect();
    match named.is_empty() {
        // the one row saying nothing is assigned
        true => vec![None],
        false => named,
    }
}

/// The first channel whose socket is set to WANTED, and what it read; None - no socket plays that role.
///
/// **BY ROLE, NOT BY SOCKET**: which socket measures the boiler's water is a setting of the boiler room, and whoever
/// wants the water does not know it. A role matches as it is kept, so `supply` is never taken for `boiler`.
pub fn by_role<'a, const N: usize>(
    map: &'a [Spot; N],
    read: &[Reading; N],
    role: impl Fn(&'static Setting) -> String,
    wanted: &str,
) -> Option<(&'a Spot, Reading)> {
    map.iter().zip(read).find(|(spot, _)| role(spot.role) == wanted).map(|(spot, reading)| (spot, *reading))
}

/// Every role setting the map needs, in the order of its channels: what the card shows under its readings.
pub fn roles<const N: usize>(map: &[Spot; N]) -> Vec<&'static Setting> {
    map.iter().map(|spot| spot.role).collect()
}

/// Every setting a map needs - each named once, in the order it is first asked for. A reading on a module we have
/// not asked before brings its own address setting, and this is what finds it.
pub fn addresses<const N: usize>(map: &[Spot; N]) -> Vec<&'static Setting> {
    let mut named: Vec<&'static Setting> = Vec::new();
    for spot in map {
        if !named.iter().any(|s| s.name == spot.address.name) {
            named.push(spot.address);
        }
    }
    named
}
