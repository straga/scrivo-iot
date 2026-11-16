//! API of the relays: the two relays of the box, L1 and L2 on its terminals - each a thing that is on or off and that a person turns on, off or over - and what the module hands the stream for them.
//! Names, kinds, commands; the address of each. NEVER HERE: a pin or the level that closes a relay (the device's Cargo.toml), a word for a person (page/menu.json).
//! CHECKS: tests/api.rs, tests/values.rs.

use page::dictionary::{Entry, Value};

/// The card the relays stand on.
pub const CARD: &str = "relays";
/// The relays by name, in the order of the relays: relay1 is L1 on the box, relay2 is L2.
pub const NAMES: [&str; 2] = ["relay1", "relay2"];

/// The rows of the relays.
pub const ENTRIES: [Entry; 2] = [relay::entry::row(CARD, NAMES[0]), relay::entry::row(CARD, NAMES[1])];

/// The addresses of the relays' values in the device's table (`crate::ALL`).
pub mod at {
    use page::dictionary::at;
    pub const RELAY: [usize; 2] = [at(&crate::ALL, super::CARD, super::NAMES[0]), at(&crate::ALL, super::CARD, super::NAMES[1])];
}

/// What the stream is told of the relays, in the order of NAMES: each closed or open. Each value by its address, as `put` takes it.
pub fn put_values(closed: [bool; 2], put: &mut dyn FnMut(usize, Value)) {
    for (n, closed) in closed.into_iter().enumerate() {
        put(at::RELAY[n], Value::On(closed));
    }
}
