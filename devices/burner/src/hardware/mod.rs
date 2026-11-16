//! HARDWARE: what the board does with its pins - the relays, the RS485 bus, the analog inputs, the thermocouple's SPI, the fan's zero-crossing
//! and gate. One file per piece of hardware.
//! HERE THERE IS NO MEANING: a file reads a pin and gives a number, or sets a pin from a number. Which millivolts are a flame, which pascals
//! a draught, who may switch a relay when - that is crates/ and src/modules. No entity name, no word for a person.

pub mod adc;
pub mod fan_gate;
pub mod relays;
pub mod rs485;
pub mod thermocouple;
pub mod zc_dump;

/// A pin as burner.yaml's number reaches the build: `17`, or `GPIO17` where the description writes it so. The one place a pin handed over as text becomes a number.
pub(crate) fn pin(said: &str) -> i32 {
    said.trim_start_matches("GPIO").parse().expect("a pin of the device's description is GPIO<n>: tools/board_env.py checks every pin")
}
