//! Modbus RTU as the master of the burner's bus speaks it: the frame it sends, what came back, and what one asking
//! came to. No hardware here - the board writes and reads the bytes, this crate says what they are, so all of it is
//! tested on the computer.
//!
//! Two functions are enough for the modules on the bus: 0x03 reads
//! holding registers - two of the PTA9B01, seven of the NT18B07 - and 0x06 writes one, which is how their addresses
//! are set apart. A frame carries no header of its own: where it ends is known from what was asked, and the CRC
//! says whether it held together.
//!
//! The crate takes no dependency, the way crates/bl0939 takes none: a request is eight bytes and the CRC is a loop,
//! while everything the acceptance stands on - a foreign address told apart from a broken frame, the awaited
//! length, the refusal - is ours either way.

#![no_std]

/// Read holding registers.
pub const READ: u8 = 0x03;
/// Write one holding register.
pub const WRITE: u8 = 0x06;
/// A slave that refuses answers with the function it was asked and this bit on.
pub const REFUSAL: u8 = 0x80;

/// Every request of ours is this long: address, function, register, count or value, CRC.
pub const REQUEST_LEN: usize = 8;
/// A refusal is this long: address, function with REFUSAL, the code, CRC.
pub const REFUSAL_LEN: usize = 5;
/// Nothing shorter than address, function and CRC can be a frame at all.
pub const MIN_FRAME: usize = 4;
/// The most registers one asking takes: the seven channels of the NT18B07.
pub const MAX_VALUES: usize = 8;
/// The longest answer awaited: MAX_VALUES registers read.
pub const MAX_REPLY: usize = 5 + 2 * MAX_VALUES;

/// The CRC of a Modbus RTU frame, which travels after it with the low byte first.
pub fn crc16(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0xFFFF, |crc, b| {
        (0..8).fold(crc ^ u16::from(*b), |c, _| if c & 1 == 1 { (c >> 1) ^ 0xA001 } else { c >> 1 })
    })
}

/// One asking of one slave: what goes on the wire, and how long its answer is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ask {
    pub addr: u8,
    pub func: u8,
    pub reg: u16,
    /// READ: how many registers; WRITE: the value to put in the one register.
    pub count: u16,
}

impl Ask {
    /// COUNT registers of the slave ADDR from REG.
    pub const fn read(addr: u8, reg: u16, count: u16) -> Self {
        Ask { addr, func: READ, reg, count }
    }

    /// VALUE into the one register REG of the slave ADDR.
    pub const fn write(addr: u8, reg: u16, value: u16) -> Self {
        Ask { addr, func: WRITE, reg, count: value }
    }

    /// The bytes to put on the wire.
    pub fn frame(&self) -> [u8; REQUEST_LEN] {
        let mut frame = [0; REQUEST_LEN];
        frame[0] = self.addr;
        frame[1] = self.func;
        frame[2..4].copy_from_slice(&self.reg.to_be_bytes());
        frame[4..6].copy_from_slice(&self.count.to_be_bytes());
        let crc = crc16(&frame[..6]).to_le_bytes();
        frame[6..8].copy_from_slice(&crc);
        frame
    }

    /// How long the answer is when the slave does what it was asked: a reading carries its registers, a writing is
    /// the request said back.
    pub const fn reply_len(&self) -> usize {
        match self.func {
            READ => 5 + 2 * self.count as usize,
            _ => REQUEST_LEN,
        }
    }

    /// How many registers the answer carries.
    pub const fn values(&self) -> usize {
        match self.func {
            READ => self.count as usize,
            _ => 1,
        }
    }
}

/// Why a frame is not the answer to this asking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Fewer bytes than the shortest frame there is.
    Short(usize),
    /// The CRC the frame carries, and the one its bytes give.
    Crc { carried: u16, computed: u16 },
    /// Another slave on the line answered.
    Foreign { asked: u8, answered: u8 },
    /// The slave refuses, with the Modbus code it names.
    Exception(u8),
    /// An answer to another function.
    Function { asked: u8, answered: u8 },
    /// READ: the byte count the answer says, against the bytes it carries.
    Count { said: u8, carried: usize },
    /// Not as long as the answer to this asking is.
    Length { awaited: usize, came: usize },
    /// WRITE: the slave sent back another register or another value.
    Echo { reg: u16, value: u16 },
}

/// How long a frame is BY ITS OWN TELLING: the function says its shape, and a reading carries the count of its
/// data bytes. None when the function is none of ours - then there is nothing inside to measure it by.
///
/// This is what lets a frame be judged before it is known whose it is. Measuring by the asking instead would mean
/// that an answer of another length never reaches the address at all - and on a bus with two modules, a frame from
/// the neighbour is almost never as long as ours.
fn own_len(frame: &[u8]) -> Option<usize> {
    match frame[1] {
        func if func & REFUSAL != 0 => Some(REFUSAL_LEN),
        READ if frame.len() > 2 => Some(REFUSAL_LEN + usize::from(frame[2])),
        WRITE => Some(REQUEST_LEN),
        _ => None,
    }
}

/// Takes the answer to ASK apart into OUT, and says how many registers it held.
///
/// The order the refusals come in is the order that keeps them apart, and it goes from what the frame says of
/// itself to what we wanted of it. First it is taken by its own length, so that a whole frame is whole whoever
/// sent it; then the CRC, because only an unbroken frame says whose it is; then the address, which sorts the
/// neighbour's business from ours; and only after that the asking has its say - the right length for what was
/// asked, the byte count, the echo.
pub fn parse(ask: &Ask, frame: &[u8], out: &mut [u16]) -> Result<usize, Refused> {
    if frame.len() < MIN_FRAME {
        return Err(Refused::Short(frame.len()));
    }
    let awaited = ask.reply_len();
    match own_len(frame) {
        // whole by its own measure: read on, whosever it turns out to be
        Some(own) if own == frame.len() => {}
        // not whole by its own measure, but exactly as long as this asking awaits: what is wrong is inside it,
        // and the checks below name what
        _ if frame.len() == awaited || frame.len() == REFUSAL_LEN => {}
        _ => return Err(Refused::Length { awaited, came: frame.len() }),
    }
    let (body, crc) = frame.split_at(frame.len() - 2);
    let (carried, computed) = (u16::from_le_bytes([crc[0], crc[1]]), crc16(body));
    if carried != computed {
        return Err(Refused::Crc { carried, computed });
    }
    if frame[0] != ask.addr {
        return Err(Refused::Foreign { asked: ask.addr, answered: frame[0] });
    }
    if frame[1] == ask.func | REFUSAL {
        return Err(Refused::Exception(frame[2]));
    }
    if frame[1] != ask.func {
        return Err(Refused::Function { asked: ask.func, answered: frame[1] });
    }
    // ours, and answering what was asked - so now its length is measured against the asking
    if frame.len() != awaited {
        return Err(Refused::Length { awaited, came: frame.len() });
    }
    if ask.func == WRITE {
        let (reg, value) = (word(frame, 2), word(frame, 4));
        if (reg, value) != (ask.reg, ask.count) {
            return Err(Refused::Echo { reg, value });
        }
        out[0] = value;
        return Ok(1);
    }
    let carried = frame.len() - REFUSAL_LEN;
    if usize::from(frame[2]) != carried {
        return Err(Refused::Count { said: frame[2], carried });
    }
    // ask.values() comes from outside too - the console takes it off a typed line - so it says how much was asked
    // for, not how much there is room for. What comes back is what was really written: a count bigger than OUT
    // would have the caller slice past the end of it.
    let room = ask.values().min(out.len());
    for (n, value) in out.iter_mut().enumerate().take(room) {
        *value = word(frame, 3 + 2 * n);
    }
    Ok(room)
}

/// The register at AT, high byte first, as Modbus puts it on the wire.
fn word(frame: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([frame[at], frame[at + 1]])
}

/// What one asking came to - exactly one of five.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Outcome {
    /// The slave answered, whole and its own.
    Accepted,
    /// The slave refuses, with the Modbus code it names.
    Exception(u8),
    /// Bytes came and what they made was not a frame: the CRC, the count, another function.
    Broken,
    /// Another slave on the line answered.
    Foreign,
    /// Nothing whole came.
    #[default]
    Missed,
}

/// Gathers the bytes of one answer, in whatever pieces the UART hands them over. A frame carries no header, so
/// where it ends is known from the asking: an answer is `reply_len` long, a refusal REFUSAL_LEN, and which of the
/// two it is, is read off the function byte as soon as that byte arrives. `ask` starts an asking afresh, so the
/// tail of a late answer never counts towards the next one.
pub struct Receiver {
    ask: Ask,
    buf: [u8; MAX_REPLY],
    got: usize,
    whole: bool,
    values: [u16; MAX_VALUES],
    taken: usize,
    outcome: Outcome,
}

impl Default for Receiver {
    fn default() -> Self {
        Receiver {
            ask: Ask::read(0, 0, 0),
            buf: [0; MAX_REPLY],
            got: 0,
            whole: false,
            values: [0; MAX_VALUES],
            taken: 0,
            outcome: Outcome::Missed,
        }
    }
}

impl Receiver {
    /// Starts an asking: what the last one left behind is dropped, and the frame to send comes back.
    pub fn ask(&mut self, ask: Ask) -> [u8; REQUEST_LEN] {
        self.ask = ask;
        self.got = 0;
        self.whole = false;
        self.taken = 0;
        self.outcome = Outcome::Missed;
        ask.frame()
    }

    /// Adds BYTES; true once the answer is whole and taken apart.
    pub fn feed(&mut self, bytes: &[u8]) -> bool {
        for &b in bytes {
            if self.whole {
                break; // whatever else is on the line belongs to no asking of ours
            }
            if self.got < MAX_REPLY {
                self.buf[self.got] = b;
                self.got += 1;
            }
            match self.awaited() {
                // The length a frame states is a CLAIM, and it is made by whoever sent it. A count byte of 0xFF
                // claims 260 bytes where MAX_REPLY can hold 21: waiting for them means waiting out the whole
                // timeout and reporting silence - and silence is what we read as a sensor gone. Refused as soon
                // as the claim is read, which is the byte that carries it.
                Some(claimed) if claimed > MAX_REPLY => {
                    self.whole = true;
                    self.outcome = Outcome::Broken;
                }
                Some(whole) if self.got == whole => self.finish(),
                _ => {}
            }
        }
        self.whole
    }

    /// How long the frame coming in is, read off the frame itself as soon as its bytes allow: the function byte
    /// says the shape, and a reading's third byte counts its data. Falls back on the asking only when the function
    /// is none we know, because then the frame says nothing of its own size.
    fn awaited(&self) -> Option<usize> {
        if self.got < 2 {
            return None;
        }
        match own_len(&self.buf[..self.got]) {
            Some(own) => Some(own),
            None if self.buf[1] == READ => None, // a reading whose count byte has not arrived yet
            None => Some(self.ask.reply_len()),
        }
    }

    /// Takes the frame apart and says what the asking came to.
    fn finish(&mut self) {
        self.whole = true;
        let mut values = [0; MAX_VALUES];
        self.outcome = match parse(&self.ask, &self.buf[..self.got], &mut values) {
            Ok(taken) => {
                self.values = values;
                self.taken = taken;
                Outcome::Accepted
            }
            Err(Refused::Exception(code)) => Outcome::Exception(code),
            Err(Refused::Foreign { .. }) => Outcome::Foreign,
            Err(_) => Outcome::Broken,
        };
    }

    /// What this asking came to so far.
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// The registers of the answer, empty until one is accepted.
    pub fn values(&self) -> &[u16] {
        &self.values[..self.taken]
    }
}

/// Askings in a window of counts.
pub const WINDOW: u32 = 1000;

/// The outcomes of the askings in one window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub accepted: u32,
    pub exception: u32,
    pub broken: u32,
    pub foreign: u32,
    pub missed: u32,
}

impl Tally {
    pub fn askings(&self) -> u32 {
        self.accepted + self.exception + self.broken + self.foreign + self.missed
    }
}

/// Counts over a window of WINDOW askings, then afresh: a count since start says nothing of the line after a day.
/// The last full window stays readable while the next one fills.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub now: Tally,
    pub last: Option<Tally>,
}

impl Counts {
    pub fn count(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Accepted => self.now.accepted += 1,
            Outcome::Exception(_) => self.now.exception += 1,
            Outcome::Broken => self.now.broken += 1,
            Outcome::Foreign => self.now.foreign += 1,
            Outcome::Missed => self.now.missed += 1,
        }
        if self.now.askings() >= WINDOW {
            self.last = Some(self.now);
            self.now = Tally::default();
        }
    }
}

/// How the number a module sends becomes the number a person reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    /// Signed tenths of the unit: 0x00CF is 20.7, 0xFF15 is -23.5. Both modules of the burner count this way.
    Tenths,
    /// The number as the module sent it.
    Whole,
}

/// One reading a person sees, described rather than coded: where it is taken from and what its number means.
///
/// This is the whole of what a reading is. Adding one is a line of description, and the askings that cover the bus
/// are built from these lines - no code is written per register, and none is touched when a channel appears.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    /// What a person reads on the card, beside the number.
    pub name: &'static str,
    /// The module on the bus.
    pub addr: u8,
    /// The register that holds it.
    pub reg: u16,
    /// The unit the number is in, shown as it stands.
    pub unit: &'static str,
    pub scale: Scale,
    /// The raw value that means "nothing is plugged in here" - 0xF555 on the NT18B07. None when the module has no
    /// such value.
    pub empty: Option<u16>,
}

/// What one channel came to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reading {
    /// The number in its unit.
    Is(f32),
    /// The channel is there and nothing is plugged into it. Not a temperature, and not an error either.
    Empty,
    /// This asking says nothing about this channel - and saying nothing is not saying zero.
    Unknown,
}

/// The askings that cover CHANNELS, written into OUT; answers how many. One asking per run of registers on one
/// module, and a run is at most MAX_VALUES registers - the modules answer a whole run in one go, which is what
/// makes a bus of many readings cheap.
pub fn asks(channels: &[Channel], out: &mut [Ask]) -> usize {
    let mut made = 0;
    for channel in channels {
        // an asking already on the list that this channel fits into, once it is stretched over it
        let joined = out[..made].iter_mut().any(|ask| {
            if ask.addr != channel.addr {
                return false;
            }
            let first = ask.reg.min(channel.reg);
            let last = (ask.reg + ask.count - 1).max(channel.reg);
            let span = usize::from(last - first) + 1;
            if span > MAX_VALUES {
                return false; // too far apart to be read in one go
            }
            ask.reg = first;
            #[allow(clippy::cast_possible_truncation)]
            {
                ask.count = span as u16;
            }
            true
        });
        if !joined {
            if made == out.len() {
                break; // no room left: the channels past this one are not covered, and the caller sees it by count
            }
            out[made] = Ask::read(channel.addr, channel.reg, 1);
            made += 1;
        }
    }
    made
}

/// What CHANNEL reads in the VALUES that ASK brought back.
pub fn reading(channel: &Channel, ask: &Ask, values: &[u16]) -> Reading {
    if ask.addr != channel.addr || ask.func != READ {
        return Reading::Unknown; // another module, or not a reading at all
    }
    let Some(at) = channel.reg.checked_sub(ask.reg) else {
        return Reading::Unknown; // before the run this asking covers
    };
    if at >= ask.count {
        return Reading::Unknown; // past its end
    }
    let Some(&raw) = values.get(usize::from(at)) else {
        return Reading::Unknown; // the answer was shorter than the asking: missing is not zero
    };
    if Some(raw) == channel.empty {
        return Reading::Empty;
    }
    Reading::Is(match channel.scale {
        #[allow(clippy::cast_possible_wrap)]
        Scale::Tenths => f32::from(raw as i16) / 10.0,
        Scale::Whole => f32::from(raw),
    })
}
