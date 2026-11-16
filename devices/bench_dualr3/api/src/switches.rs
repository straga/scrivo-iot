//! API of the wall keys: the two keys of the box, S1 and S2 - each closed or open and counting how many times it toggled the relay it is wired to -, and what the module hands the stream for them.
//! A key is read and never commanded: a hand moves it. NEVER HERE: a pin or a debounce (the device), a word for a person (page/menu.json).
//! CHECKS: tests/api.rs, tests/values.rs.

use page::dictionary::{joined, Entry, Value};

/// The card the keys stand on.
pub const CARD: &str = "switches";
/// The keys by name: switch1 is S1 on the box, switch2 is S2.
pub const NAMES: [&str; 2] = ["switch1", "switch2"];

const ONE: [Entry; 2] = [switch::entry::row(CARD, NAMES[0]), switch::entry::toggles(CARD, NAMES[0])];
const TWO: [Entry; 2] = [switch::entry::row(CARD, NAMES[1]), switch::entry::toggles(CARD, NAMES[1])];
/// The rows of the keys, each followed by its count.
pub const ENTRIES: [Entry; 4] = joined(&ONE, &TWO);

/// The addresses of the keys' values in the device's table (`crate::ALL`).
pub mod at {
    use page::dictionary::{at, part_at};
    pub const KEY: [usize; 2] = [at(&crate::ALL, super::CARD, super::NAMES[0]), at(&crate::ALL, super::CARD, super::NAMES[1])];
    pub const TOGGLES: [usize; 2] =
        [part_at(&crate::ALL, super::CARD, super::NAMES[0], "toggles"), part_at(&crate::ALL, super::CARD, super::NAMES[1], "toggles")];
}

/// What a key is now: where it stands and how many times it toggled since the board started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub closed: bool,
    pub toggles: u32,
}

/// What the stream is told of the keys, in the order of NAMES.
pub fn put_values(keys: &[Key; 2], put: &mut dyn FnMut(usize, Value)) {
    for (n, key) in keys.iter().enumerate() {
        put(at::KEY[n], Value::On(key.closed));
        put(at::TOGGLES[n], Value::Number(key.toggles as f32));
    }
}
