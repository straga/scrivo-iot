//! What the burner does with the reading of its socket: the address the setting names, and the line the journal gets when an organ switched and the mains power moved with it.
//! HERE: the judging of a round - which switches to tell, what was before and what after - and the words of the line.
//! NEVER HERE: the reading itself (crates/tasmota), the relays' notes (crates/relay), a timer or a socket (the module that runs this), a word of the page.
//! CHECKS: tests/mains.rs.
//!
//! **THE POWER "AFTER" IS THE FIRST READING AFTER THE SWITCH**: the burner asks the socket every few seconds, so a round that finds switches noted since the last one tells all of them with the reading it has just made, and with
//! the one before as "before". A switch is never told with a reading that was made before it.

use core::fmt::Write;
use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tasmota::{Energy, Fault};

/// The port a Tasmota socket's page is served on.
const HTTP: u16 = 80;

/// The socket's address from the setting as it is kept (`address:192.0.2.10`); None - `off`, or nothing the board can dial.
pub fn address(kept: &str) -> Option<SocketAddr> {
    let ip: Ipv4Addr = kept.strip_prefix("address:")?.parse().ok()?;
    Some(SocketAddr::V4(SocketAddrV4::new(ip, HTTP)))
}

/// The reading of the last round, which is the "before" of the switches the next one finds.
#[derive(Default)]
pub struct Watch {
    before: Option<Energy>,
}

impl Watch {
    pub const fn new() -> Self {
        Self { before: None }
    }

    /// A round has read the socket (RESULT) and found SWITCHES of the relays since the last one - (relay, closed), in order -, DROPPED more than the notes held. The line for the journal, when there is a switch to tell;
    /// NAME gives the word of a relay.
    pub fn round<'a>(&mut self, result: &Result<Energy, Fault>, switches: &[(usize, bool)], dropped: u32, name: impl Fn(usize) -> &'a str) -> Option<String> {
        let before = core::mem::replace(&mut self.before, result.as_ref().ok().copied());
        if switches.is_empty() {
            return None;
        }
        let mut line = String::from("power: ");
        for (n, (relay, closed)) in switches.iter().enumerate() {
            let word = name(*relay);
            let state = if *closed { "on" } else { "off" };
            if n > 0 {
                line.push_str(", ");
            }
            if word.is_empty() {
                let _ = write!(line, "relay {relay} {state}");
            } else {
                let _ = write!(line, "{word} {state}");
            }
        }
        if dropped > 0 {
            let _ = write!(line, ", {dropped} more");
        }
        match before {
            Some(energy) => {
                let _ = write!(line, " - {} W before", energy.watts.round());
            }
            None => line.push_str(" - unknown before"),
        }
        match result {
            Ok(energy) => {
                let _ = write!(line, ", {} W after", energy.watts.round());
            }
            Err(fault) => {
                let _ = write!(line, ", unknown after ({})", fault.reason());
            }
        }
        Some(line)
    }
}
