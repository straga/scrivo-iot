//! API: what the bench S3 says outward - the dictionary of what every board has (the system card, the clock's and the station's settings) and the /events probe's row.
//! A crate of its own on purpose: it depends on `page`, `dictionary` and `settings` and on nothing of the hardware, so it cannot reach a pin.
//!
//! **THE PROBE'S ROW IS IN THE TABLE OF EVERY IMAGE, built with the probe or not**, because the table is made twice - by the device's build.rs for the dictionary a client
//! reads, and by the compiler for the addresses the firmware writes by - and a feature of the device reaches the second and not the first. A row in both is the same table; an image
//! without the probe never writes it, and the board says nothing at its address.
//! CHECKS: tests/api.rs.

use dictionary::board::{board_settings, BOARD, NET};
use page::dictionary::joined;

/// The /events probe: a row that says in ms when it last changed, written by an image built with the probe only.
pub mod probe {
    use page::dictionary::{Entry, Kind, Value};

    /// The card the probe stands on.
    pub const CARD: &str = "probe";
    pub const ENTRIES: [Entry; 1] = [Entry { unit: "ms", decimals: Some(0), ..Entry::row(CARD, "changed_at", &[Kind::Number]) }];
    /// Where the probe's value stands in the table.
    pub const AT: usize = page::dictionary::at(&crate::ALL, CARD, "changed_at");

    /// What the stream is told of the probe: the time of the last change, in ms since the board started.
    pub fn put_value(changed_at_ms: u32, put: &mut dyn FnMut(usize, Value)) {
        put(AT, Value::Number(changed_at_ms as f32));
    }
}

/// The cards of the device, in the order its page draws them.
pub const CARDS: [&str; 1] = [probe::CARD];

/// What every board has, then the probe's row, then the station's settings, then how busy each core is (`dictionary::system::CORES`), last of all.
const OWNED: [page::dictionary::Entry; BOARD.len() + probe::ENTRIES.len()] = joined(&BOARD, &probe::ENTRIES);
const NETTED: [page::dictionary::Entry; OWNED.len() + NET.len()] = joined(&OWNED, &NET);
pub const ALL: [page::dictionary::Entry; NETTED.len() + dictionary::system::CORES.len()] = joined(&NETTED, &dictionary::system::CORES);
/// The same, where the image keeps it once.
pub static TABLE: [page::dictionary::Entry; ALL.len()] = ALL;

/// The device's name in the dictionary and on the page.
pub const DEVICE: &str = "bench_s3";
/// The board it runs on, by the name of its `boards/<board>`.
pub const BOARD_NAME: &str = "STRAGA_BASE_S3";

/// The dictionary the S3's image serves, the image being KIT: made when the image is built, from [`ALL`].
pub fn json(kit: &str) -> String {
    page::dictionary::json(&ALL, &page::dictionary::Meta { device: DEVICE, board: BOARD_NAME, kit, cards: &CARDS, settings: &board_settings(), machine: None })
}
