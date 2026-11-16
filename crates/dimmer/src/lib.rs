//! A percent of the fan into a moment to fire the triac, and nothing about wires. The pins, the zero crossing and
//! the MCPWM that makes the pulse are the device's half (devices/burner/src/fan.rs).
//!
//! The angles are the garage burner's, measured on the owner's own fan by its MicroPython module:
//! a 10 kHz timer, so one tick
//! is 100 µs and a 50 Hz half cycle is a hundred of them; `power_max = 50` was full speed and `power_min = 74`
//! was the lowest it would still turn at. THAT SCALE WAS UPSIDE DOWN - a bigger number was a longer delay and
//! less air - and the turning is done here, once, so that a percent means what a person means by it.
//!
//! What is NOT taken from there: the gate pulse. Its `puls_width = 400` is commented "us" but written to RMT at
//! 100 ns a tick, so the fan actually latched on 40 µs.

mod fronts;
mod gate_stop;
mod gate_write;
pub use fronts::{Change, Fronts};
pub use gate_write::{write_gate, GateHardware, DOWN_AT_US};
pub use gate_stop::{may_resume, STOP_AFTER_US, STOP_ISR_BUDGET_US};

/// Where the fan runs at full speed, on a 50 Hz half cycle: 50 ticks of that 100 µs timer.
pub const FULL_US: u32 = 5_000;
/// Where it barely turns: 74 of the same ticks. Below this angle it stalls - measured on that fan, and the live
/// one confirms or moves it.
pub const LOW_US: u32 = 7_400;
/// The half cycle those two were measured on. Everything is kept as a SHARE of it, so a mains that drifts moves
/// the firing with it instead of turning the same percent into a different angle.
pub const NOMINAL_HALF_US: u32 = 10_000;

/// The percent the boiler's fan needs to BREAK AWAY FROM REST, and the lowest it will still turn on once it is
/// already going. Two numbers and not one, because they are not the same thing: measured on the owner's own fan
/// at the boiler, 20.09.2026, by hand through the page - it will not start at 7 %, it starts at 25, and once
/// turning it holds down to 8, labouring. At 30 it runs evenly (his word).
///
/// NOTHING HERE ENFORCES THEM. They are what the fan does, written where the next reader will find them; what to
/// do about a fan asked for less than it can start on belongs to whoever holds the draught, and to
/// the row the person reads (devices/burner/src/fan.rs).
///
/// They are about THIS fan on THIS boiler. Another fan gets its own pair, measured the same way.
pub const STARTS_AT: u32 = 25;
pub const HOLDS_AT: u32 = 8;

/// How long the gate is held. 50 µs is ESPHome's (ac_dimmer.cpp:28) and Espressif's own; a BT136S needs 2 µs, and
/// the garage fan latched on 40. The rest is margin for the optocoupler.
pub const GATE_US: u32 = 50;

/// How much of the half cycle is left untouched before the next crossing. THE GATE MUST BE DOWN BEFORE THE MAINS
/// CROSSES: held through it, the triac opens at the start of the next half cycle and the fan runs at full power
/// whatever was asked - the defect our own triac.c carried.
pub const GUARD_US: u32 = 200;

/// The half cycles a mains can give: 40 Hz is 12 500 µs and 71 Hz is 7 000. Anything outside is not a mains but a
/// miscounted crossing, and firing on it puts the phase anywhere.
pub const SHORTEST_HALF_US: u32 = 7_000;
pub const LONGEST_HALF_US: u32 = 12_500;

/// What the board should do in this half cycle.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Order {
    Fire { delay_us: u32, gate_us: u32 },
    Off,
}

/// When to fire for PERCENT of air, given the half cycle the board has just measured.
///
/// Zero is OFF and not a very late firing: fired late the triac still conducts a sliver of every half cycle, and
/// the motor still hums at it.
pub fn order(percent: u32, half_cycle_us: u32) -> Result<Order, String> {
    if percent > 100 {
        return Err(format!("the fan takes 0 to 100 percent of air; {percent} is not one of them"));
    }
    if !(SHORTEST_HALF_US..=LONGEST_HALF_US).contains(&half_cycle_us) {
        return Err(format!(
            "a half cycle of {half_cycle_us} µs is no mains: it lies outside {SHORTEST_HALF_US}..{LONGEST_HALF_US}, \
             so the zero crossings are being miscounted and the angle would be anywhere"
        ));
    }
    if percent == 0 {
        return Ok(Order::Off);
    }
    let share = |of: u32| (of as u64 * half_cycle_us as u64 / NOMINAL_HALF_US as u64) as u32;
    let (full, low) = (share(FULL_US), share(LOW_US));
    // 1 % sits at the lowest angle and 100 % at the fullest; the ninety-nine steps between them are even
    let delay_us = low - (percent - 1) * (low - full) / 99;
    Ok(Order::Fire { delay_us, gate_us: GATE_US })
}

/// The half cycle of the mains, from how many crossings the board counted in a window it measured itself.
///
/// THE RATE IS THE MEASUREMENT, and it is taken this way because of where it has to be taken from: the only
/// clock an interrupt handler may read is one that costs no call, and on this board there is none - reaching for
/// a timestamp inside the handler is what a whole stage was lost to. Counting costs one instruction, so the
/// board counts, and the arithmetic happens out here. Over ten seconds a miscount of one crossing moves the
/// answer by 0.1 %, which is 10 µs of a half cycle - a fifth of the gate pulse itself.
pub fn half_cycle_of(crossings: u32, over_ms: u32) -> Result<u32, String> {
    if crossings == 0 || over_ms == 0 {
        return Err(format!(
            "no crossings to read a mains from: {crossings} of them in {over_ms} ms. Either the detector is \
             silent or nothing has been counted yet"
        ));
    }
    let half_cycle_us = (over_ms as u64 * 1_000 / crossings as u64) as u32;
    if !(SHORTEST_HALF_US..=LONGEST_HALF_US).contains(&half_cycle_us) {
        return Err(format!(
            "{crossings} crossings in {over_ms} ms make a half cycle of {half_cycle_us} µs, and that is no \
             mains: it lies outside {SHORTEST_HALF_US}..{LONGEST_HALF_US}. Too few means crossings are being \
             missed, too many means the edges are bouncing"
        ));
    }
    Ok(half_cycle_us)
}

/// How many crossings a window of WINDOW_MS owes on a nominal 50 Hz mains: two a cycle, a hundred a second.
///
/// THE ONLY REFERENCE THE DETECTOR DOES NOT FEED. Counting crossings and counting gate pulses both grow out of
/// the same pin, so they agree with each other even when a crossing is missed - and a measure that cannot show a
/// disagreement proves nothing. The board's own clock can.
///
/// It is a NOMINAL fifty, not a measured one. A real grid wanders inside about 49.9-50.1 Hz, which is ±12 over a
/// minute, so a difference of that size is the grid and not the detector; a difference beyond it, and any
/// difference between the counts of the two EDGES of the same pulse, is ours.
pub fn owed_by_the_clock(window_ms: u32) -> u32 {
    window_ms / 10
}

/// How many crossings went missing in a GAP of `gap_us` between two crossings the board counted.
///
/// THE ONLY MEASURE THAT CAN SHOW A SINGLE MISS. A sum over a minute averages one away: 5999 against 6000 sits
/// inside the band a real grid wanders in, so a lost half cycle and a slow mains read the same. A gap does not
/// average anything - where 10 ms belongs, a miss puts 20, and the grid has no way to make that happen. It moves
/// by fractions of a percent; a miss doubles.
///
/// Anything shorter than a half cycle is not a miss and is not counted here: that is bounce, and the doubled-edge
/// counter is what sees it.
pub fn missed_in(gap_us: u32, half_cycle_us: u32) -> u32 {
    if half_cycle_us == 0 || gap_us * 2 < half_cycle_us * 3 {
        return 0;
    }
    (gap_us + half_cycle_us / 2) / half_cycle_us - 1
}

/// WHAT A MINUTE OF THE GATE'S ACCOUNTING IS WORTH IN THE JOURNAL. A line only when a half cycle was ACTUALLY lost: a crossing missed by the gaps, or the ring of edges written over before the round read
/// it. A second that came up one gate pulse short (`worst_missed`) with no crossing lost is the pulse the two counts differ by, not trouble - it stood at `warn` every ~75 s, eight lines in ten minutes of fire, and drowned
/// the line that would mean something. It rides on the line when there is one. The two causes, which were two lines, are one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Minute {
    /// Crossings lost by gaps in the counting this minute ([`missed_in`]).
    pub missed: u32,
    /// The second of the minute that came up shortest of gate pulses, by how many.
    pub worst_missed: u32,
    /// How many times the ring of edges was written over before the round read it.
    pub edges_lost: u32,
}

impl Minute {
    /// Whether the minute is to be said at all.
    pub fn worth_a_line(&self) -> bool {
        self.missed > 0 || self.edges_lost > 0
    }

    /// The line, with every number the minute has.
    pub fn words(&self) -> String {
        format!(
            "fan: {} crossings lost by gaps in the counting, the ring of edges written over {} times before the round read it, \
             a second short by {} gate pulses at worst - the detector or the gate is dropping half cycles",
            self.missed, self.edges_lost, self.worst_missed
        )
    }
}

/// One edge of the zero-crossing detector as the interrupt writes it down, raw: which way it went, the free-running timer's count at that moment, and the count of the timer the crossing resets. NOTHING IS JUDGED where it is
/// written - whether it is a transition at all, how far it is from the one before, how wide the pulse is - that is [`EdgeReader`]'s, in the round that counts the seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub rising: bool,
    /// The free-running timer, in microseconds, wrapping at 65 536: the difference between two of them is the true gap.
    pub free_us: u16,
    /// The timer the crossing resets, in microseconds: at a falling edge it holds the pulse's width. Fifteen bits: it counts to 20 000 at most.
    pub pulse_us: u16,
}

impl Edge {
    /// The edge as the one word the interrupt writes: the way in the top bit, the pulse's count under it, the free count in the low half. A function that is always put in place and never called, because the interrupt calls nothing.
    #[inline(always)]
    pub const fn pack(self) -> u32 {
        (self.rising as u32) << 31 | (self.pulse_us as u32 & 0x7FFF) << 16 | self.free_us as u32
    }

    pub const fn unpack(word: u32) -> Edge {
        Edge { rising: word >> 31 == 1, free_us: word as u16, pulse_us: (word >> 16 & 0x7FFF) as u16 }
    }
}

/// What the round that counts the seconds makes of the edges the interrupt wrote down: the edges that are no transition - the same way twice running -, the crossings the gaps between rises swallowed, the widest of those gaps, and the
/// narrowest and widest pulse. The state of the chain of edges is its own: one that was broken by a ring written over ([`EdgeReader::lost`]) starts a new one.
#[derive(Clone, Debug)]
pub struct EdgeReader {
    /// The way of the edge before this one; None at the start and after a break.
    before: Option<bool>,
    /// The free count at the last rise that was a transition.
    last_rise: Option<u16>,
    doubled: (u32, u32),
    missed: u32,
    widest_gap: u32,
    narrowest: u32,
    widest: u32,
}

impl Default for EdgeReader {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeReader {
    pub const fn new() -> Self {
        EdgeReader { before: None, last_rise: None, doubled: (0, 0), missed: 0, widest_gap: 0, narrowest: u32::MAX, widest: 0 }
    }

    /// The next edge the interrupt wrote down, in the order it wrote them.
    pub fn feed(&mut self, edge: Edge) {
        // WAS IT A TRANSITION AT ALL. A pulse has one rise and one fall; a rise straight after a rise is the same transition seen twice, and counting it as a crossing is how a count climbs above what the clock owes. The way of the
        // edge before is kept whatever came of it.
        let doubled = self.before == Some(edge.rising);
        self.before = Some(edge.rising);
        if doubled {
            match edge.rising {
                true => self.doubled.0 = self.doubled.0.wrapping_add(1),
                false => self.doubled.1 = self.doubled.1.wrapping_add(1),
            }
            return;
        }
        if edge.rising {
            // THE GAP TO THE CROSSING BEFORE THIS ONE, the only measure that can show a SINGLE miss. A sum over a minute averages one away; a gap cannot be averaged: where 10 ms belongs, a lost half cycle puts 20.
            if let Some(last) = self.last_rise {
                let gap = u32::from(edge.free_us.wrapping_sub(last));
                self.missed += missed_in(gap, NOMINAL_HALF_US);
                self.widest_gap = self.widest_gap.max(gap);
            }
            self.last_rise = Some(edge.free_us);
        } else {
            // the timer the crossing resets counts from the rising edge, so at the falling edge its count IS the pulse's width
            let wide = u32::from(edge.pulse_us);
            self.narrowest = self.narrowest.min(wide);
            self.widest = self.widest.max(wide);
        }
    }

    /// The ring the interrupt writes was written over before it was read: what came between is gone, and no gap is made across it - the chain of edges starts again.
    pub fn lost(&mut self) {
        self.before = None;
        self.last_rise = None;
    }

    /// The extra edges seen since the start - a rise straight after a rise, a fall after a fall.
    pub fn doubled(&self) -> (u32, u32) {
        self.doubled
    }

    /// How many crossings the gaps swallowed, and the widest gap, since this was last asked; asking starts them again.
    pub fn gaps(&mut self) -> (u32, u32) {
        let seen = (self.missed, self.widest_gap);
        (self.missed, self.widest_gap) = (0, 0);
        seen
    }

    /// The narrowest and widest pulse since this was last asked, None when no pulse was seen; asking starts them again. Kept as an all-time pair they only ever grew, and a line that says "0 to 693 µs" about a minute in which nothing was
    /// near either end describes no minute at all.
    pub fn pulses(&mut self) -> Option<(u32, u32)> {
        let seen = (self.narrowest != u32::MAX).then_some((self.narrowest, self.widest));
        (self.narrowest, self.widest) = (u32::MAX, 0);
        seen
    }
}

/// What a task that reads a ring of LEN places, which an interrupt writes at one more place every time, finds there: HEAD is how many places the interrupt has written since the start, TAIL how many the task has read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unread {
    /// The count of the first place to read; the place itself is `first % LEN`.
    pub first: u32,
    /// How many places there are to read.
    pub count: usize,
    /// How many the interrupt wrote over before they were read - places the task will never see.
    pub lost: u32,
}

pub fn unread(head: u32, tail: u32, len: usize) -> Unread {
    let written = head.wrapping_sub(tail);
    let count = (written as usize).min(len);
    Unread { first: head.wrapping_sub(count as u32), count, lost: written - count as u32 }
}

/// How many crossings a live detector gives in a second on a mains this crate fires into: 80 at 40 Hz to 142 at 71 Hz,
/// the band SHORTEST_HALF_US..LONGEST_HALF_US holds. Outside it the detector is silent, or counting both edges.
pub const LIVE_CROSSINGS: core::ops::RangeInclusive<u32> = (1_000_000 / LONGEST_HALF_US)..=(1_000_000 / SHORTEST_HALF_US);

/// The half cycle a percent is fired against now: MEASURED - the board's window, None while its first is still
/// filling - and how many crossings the detector gave in the last second.
///
/// **BEFORE THE FIRST WINDOW, THE NOMINAL HALF CYCLE, AS LONG AS THE DETECTOR IS ALIVE**. The
/// window takes ten seconds, and a board that came up over a fire must blow in the first of them: ten seconds of
/// fire without air is the worst thing this burner does. The angle is kept as a share of the half cycle, so on the
/// 50 Hz this board was measured on (9 980 µs, 24.09) the nominal moves the firing by a fifth of a percent. The gate
/// is synced from the crossing every half cycle whatever number it was built against; what the nominal cannot be
/// fired over is a detector that gives no crossings or twice too many, where the phase itself is unknown.
///
/// A window that counted and found no mains is not replaced by the nominal: that is a detector that has been
/// measured and failed.
pub fn half_to_fire(measured: Option<Result<u32, String>>, crossings_last_second: u32) -> Result<u32, String> {
    match measured {
        Some(read) => read,
        None if LIVE_CROSSINGS.contains(&crossings_last_second) => Ok(NOMINAL_HALF_US),
        None => Err(format!(
            "the mains has not been measured yet, and the detector gave {crossings_last_second} crossings in the \
             last second - a mains gives {} to {}: nothing is fired over a phase nobody knows",
            LIVE_CROSSINGS.start(),
            LIVE_CROSSINGS.end()
        )),
    }
}

/// What the registers of the zero-crossing pin hold, as fields: the words are read where the chip is, and this is only what their bits say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZcRegs {
    /// Function of the pad: 1 is GPIO on the S3, the others hand it to a peripheral (`MCU_SEL`).
    pub func: u8,
    pub input_enable: bool,
    pub pull_up: bool,
    pub pull_down: bool,
    /// The pad's own glitch filter (`FILTER_EN`).
    pub filter: bool,
    /// What edge the pin raises its interrupt on: 0 none, 1 rising, 2 falling, 3 any, 4 low level, 5 high level (`INT_TYPE`).
    pub int_type: u8,
    /// Which of the five interrupt lines are enabled for the pin (`INT_ENA`), as a mask.
    pub int_ena: u8,
    /// The level of the pin now.
    pub level: bool,
}

/// The fields of the words IO_MUX_GPIOn_REG (IO_MUX), GPIO_PINn_REG (PIN) and the input word that holds the pin (LEVEL_WORD, the pin at LEVEL_BIT), read on the S3: the places of the bits are the chip's (soc/esp32s3/register/soc/io_mux_reg.h, gpio_reg.h).
pub fn zc_regs(io_mux: u32, pin: u32, level_word: u32, level_bit: u32) -> ZcRegs {
    ZcRegs {
        func: (io_mux >> 12 & 0x7) as u8,
        input_enable: io_mux >> 9 & 1 == 1,
        pull_up: io_mux >> 8 & 1 == 1,
        pull_down: io_mux >> 7 & 1 == 1,
        filter: io_mux >> 15 & 1 == 1,
        int_type: (pin >> 7 & 0x7) as u8,
        int_ena: (pin >> 13 & 0x1F) as u8,
        level: level_word >> level_bit & 1 == 1,
    }
}

impl ZcRegs {
    /// One line of `key=value` pairs, codes and numbers only, in the order a person compares them with the setup in the code: the console word `zc` prints it after what the IDF dump says.
    pub fn line(&self, rising: u32, falling: u32) -> String {
        format!(
            "zc: func={} ie={} pu={} pd={} filter={} int_type={} int_ena={} level={} rising={rising} falling={falling}",
            self.func,
            u8::from(self.input_enable),
            u8::from(self.pull_up),
            u8::from(self.pull_down),
            u8::from(self.filter),
            self.int_type,
            self.int_ena,
            u8::from(self.level)
        )
    }
}
