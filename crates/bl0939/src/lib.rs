//! The BL0939 energy meter on its UART: what to send it and what its answer means. No hardware here - the board
//! writes and reads the bytes, this crate says what they are, so all of it is tested on the computer.
//!
//! The protocol is the one ESPHome (components/bl0939/bl0939.cpp) and Tasmota (tasmota_xnrg_energy/
//! xnrg_14_bl09xx.ino) speak: `0x55 0xAA` asks for the full packet, and the meter answers 35 bytes - the header 0x55,
//! eleven 24-bit fields with the LOW byte first, and a checksum. Two meanings: `0x55` plus the 34 bytes before the
//! checksum, inverted.

/// Bytes in the meter's answer to READ_ALL.
pub const PACKET_LEN: usize = 35;
/// The first byte of every answer.
pub const HEADER: u8 = 0x55;
/// What asks the meter for its full packet.
pub const READ_ALL: [u8; 2] = [0x55, 0xAA];

/// The six writes that set the meter up, 1 ms apart - ESPHome's BL0939_INIT, each row with its own checksum: reset,
/// unlock user writes, 50 Hz and RMS updated every 800 ms, temperature every 100 ms, fast RMS of both channels.
pub const INIT: [[u8; 6]; 6] = [
    [0xA5, 0x19, 0x5A, 0x5A, 0x5A, 0x33],
    [0xA5, 0x1A, 0x55, 0x00, 0x00, 0xEB],
    [0xA5, 0x18, 0x00, 0x10, 0x00, 0x32],
    [0xA5, 0x1B, 0xFF, 0x47, 0x00, 0xF9],
    [0xA5, 0x10, 0x1C, 0x18, 0x00, 0x16],
    [0xA5, 0x1E, 0x1C, 0x18, 0x00, 0x08],
];

/// Counts of the meter per unit, from the Sonoff's resistors (Tasmota BL0939_UREF, _IREF, _PREF).
pub const UREF: f32 = 17159.0;
pub const IREF: f32 = 266013.0;
pub const PREF: f32 = 713.0;

/// The fields of one packet, as the meter counts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Raw {
    pub v_rms: u32,
    pub ia_rms: u32,
    pub ib_rms: u32,
    pub a_watt: i32,
    pub b_watt: i32,
}

/// One packet in units: volts, amperes and watts of channels A and B.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub volts: f32,
    pub amps: [f32; 2],
    pub watts: [f32; 2],
}

impl From<Raw> for Reading {
    fn from(raw: Raw) -> Self {
        Reading {
            volts: raw.v_rms as f32 / UREF,
            amps: [raw.ia_rms as f32 / IREF, raw.ib_rms as f32 / IREF],
            watts: [raw.a_watt as f32 / PREF, raw.b_watt as f32 / PREF],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Fewer or more than PACKET_LEN bytes.
    Length(usize),
    /// The first byte is not HEADER.
    Header(u8),
    /// The checksum the packet carries, and the one its bytes give.
    Checksum { carried: u8, computed: u8 },
}

/// The checksum of the 34 bytes before it.
pub fn checksum(head: &[u8]) -> u8 {
    head.iter().fold(READ_ALL[0], |sum, b| sum.wrapping_add(*b)) ^ 0xFF
}

fn unsigned(p: &[u8], at: usize) -> u32 {
    u32::from(p[at]) | u32::from(p[at + 1]) << 8 | u32::from(p[at + 2]) << 16
}

fn signed(p: &[u8], at: usize) -> i32 {
    // the 24-bit value moved to the top of 32 bits and back: the shift right carries the sign down
    ((unsigned(p, at) << 8) as i32) >> 8
}

/// Takes one whole packet apart.
pub fn parse(packet: &[u8]) -> Result<Raw, Refused> {
    if packet.len() != PACKET_LEN {
        return Err(Refused::Length(packet.len()));
    }
    let (carried, computed) = (packet[PACKET_LEN - 1], checksum(&packet[..PACKET_LEN - 1]));
    if carried != computed {
        return Err(Refused::Checksum { carried, computed });
    }
    if packet[0] != HEADER {
        return Err(Refused::Header(packet[0]));
    }
    // header, IA fast RMS, IA RMS, IB RMS, V RMS, IB fast RMS, A watt, B watt, ... - three bytes each
    Ok(Raw {
        ia_rms: unsigned(packet, 4),
        ib_rms: unsigned(packet, 7),
        v_rms: unsigned(packet, 10),
        a_watt: signed(packet, 16),
        b_watt: signed(packet, 19),
    })
}

/// What one asking of the meter came to - exactly one of three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A whole packet came and its checksum held.
    Accepted(Raw),
    /// Bytes came, and every whole packet among them failed its checksum.
    Rejected,
    /// No whole packet came in time: no answer, or a cut one.
    Missed,
}

/// Gathers the bytes the UART hands over during one asking, in whatever pieces, into a packet. A window whose
/// checksum fails moves on to the next HEADER inside it - Tasmota's resync (xnrg_14_bl09xx.ino, issue #1907) - so a
/// byte lost or added on the wire does not cost the packet behind it. `ask` starts an asking afresh: the tail of a
/// cut answer never reaches the next one.
#[derive(Default)]
pub struct Receiver {
    buf: Vec<u8>,
    taken: Option<Raw>,
    refused: bool,
}

impl Receiver {
    /// Starts a new asking: what the last one left behind is dropped.
    pub fn ask(&mut self) {
        self.buf.clear();
        self.taken = None;
        self.refused = false;
    }

    /// Adds BYTES and returns the packet they completed, if any.
    pub fn feed(&mut self, bytes: &[u8]) -> Option<Raw> {
        let mut last = None;
        for &b in bytes {
            if self.buf.is_empty() && b != HEADER {
                continue; // junk before a packet
            }
            self.buf.push(b);
            if self.buf.len() < PACKET_LEN {
                continue;
            }
            match parse(&self.buf) {
                Ok(raw) => {
                    last = Some(raw);
                    self.taken = Some(raw);
                    self.buf.clear();
                }
                Err(_) => {
                    self.refused = true;
                    // the next header inside the window may start the real packet
                    let next = self.buf[1..].iter().position(|&x| x == HEADER).map_or(self.buf.len(), |i| i + 1);
                    self.buf.drain(..next);
                }
            }
        }
        last
    }

    /// What this asking came to so far.
    pub fn outcome(&self) -> Outcome {
        match (self.taken, self.refused) {
            (Some(raw), _) => Outcome::Accepted(raw),
            (None, true) => Outcome::Rejected,
            (None, false) => Outcome::Missed,
        }
    }
}

/// Askings in a window of counts.
pub const WINDOW: u32 = 1000;

/// The outcomes of the askings in one window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub accepted: u32,
    pub rejected: u32,
    pub missed: u32,
}

impl Tally {
    pub fn askings(&self) -> u32 {
        self.accepted + self.rejected + self.missed
    }
}

/// Counts over a window of WINDOW askings, then afresh: a count since start says nothing of the line after a day. The
/// last full window stays readable while the next one fills.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub now: Tally,
    pub last: Option<Tally>,
}

impl Counts {
    /// What a person reads of the counts, now and for the last full window, one asking each EVERY: now
    /// "123/1000 · 0 bad · 0 lost", the last window "1 bad · 1 lost of 1000" - or, while the first one fills, how
    /// long until it is there.
    pub fn words(&self, every: core::time::Duration) -> (String, String) {
        let now = format!("{}/{WINDOW} · {} bad · {} lost", self.now.askings(), self.now.rejected, self.now.missed);
        let last = match self.last {
            Some(t) => format!("{} bad · {} lost of {WINDOW}", t.rejected, t.missed),
            None => {
                let left = every.as_secs() * u64::from(WINDOW - self.now.askings());
                format!("in {} min", left.div_ceil(60))
            }
        };
        (now, last)
    }

    pub fn count(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Accepted(_) => self.now.accepted += 1,
            Outcome::Rejected => self.now.rejected += 1,
            Outcome::Missed => self.now.missed += 1,
        }
        if self.now.askings() >= WINDOW {
            self.last = Some(self.now);
            self.now = Tally::default();
        }
    }
}
