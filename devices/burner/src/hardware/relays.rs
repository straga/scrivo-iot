//! HERE: the relay outputs: the auger, the igniter, the pump and the power of the modules, on the pins of burner.yaml, closed by the level the yaml names. A relay opens at start, whatever it was before
//! the restart - the open level is written to a pin's latch BEFORE the pin becomes an output, so none closes for a moment on the way (crates/relay, `open_pins`) - and closes only by an order.
//! NEVER HERE: which organ may be switched when, the limits of time on the auger and the igniter, the automaton's claim on them, a word for a person. The pins and the level are handed over by
//! build.rs as numbers, checked against the board's board.yml by tools/board_env.py; the one place they are read is here.
//! CHECKS: crates/relay/tests/relay.rs.

use esp_idf_svc::sys::EspError;
use relay::Relays;

/// Whether a LOW pin closes a relay: the prototype's relays were inverted (`relays_close_on: low`), and a board with the other kind says `high`.
const CLOSED_ON_LOW: bool = matches!(env!("STRAGA_RELAYS_CLOSE_ON_LOW").as_bytes(), b"1");

/// The pins of the four relays, in the order of the organs.
fn pins() -> [u8; 4] {
    [env!("STRAGA_PIN_AUGER"), env!("STRAGA_PIN_IGNITER"), env!("STRAGA_PIN_PUMP"), env!("STRAGA_PIN_AUX_POWER")]
        .map(|pin| u8::try_from(super::pin(pin)).expect("a pin number: tools/board_env.py checks every pin of the device"))
}

/// Opens every relay and starts the owner that carries out the orders: after every change it calls ANNOUNCE, and before every look it takes the limit of each relay from LIMIT_MS, in
/// milliseconds, so a limit changed in place works at once; 0 is no limit. Call once, before the network; a pin that cannot be set up stops it.
pub fn start(relays: &'static Relays<4>, names: &'static [&'static str; 4], limit_ms: fn(usize) -> u32, announce: fn()) -> Result<(), EspError> {
    relay::board::start(relays, names, pins(), CLOSED_ON_LOW, limit_ms, announce)
}
