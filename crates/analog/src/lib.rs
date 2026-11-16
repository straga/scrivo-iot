//! What many samples of ONE analog pin come to. No hardware here and no sensor either: the ADC that fetches the
//! millivolts is the device's half (crates/firmware, analog.rs), and what the millivolts MEAN belongs to the
//! sensor's own crate - crates/draught for the pressure, crates/flame for the fire.
//!
//! It lives on its own because a second sensor arrived: the draught had this to itself, the flame reads its pin
//! exactly the same way, and a piece that gets copied moves into the common library the same day it is copied.

/// What a round of samples of one pin came to: the number the board goes on, and what the samples ranged over
/// while it was taken.
///
/// THE SPREAD TRAVELS WITH THE MEAN, always. A mean on its own says nothing about whether the pin was steady or
/// swinging, and that is the one thing that says whether the window is long enough.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Window {
    /// The mean of the samples, in TENTHS of a millivolt.
    ///
    /// NOT WHOLE MILLIVOLTS, and this is measured rather than tidy. A step of this converter is about 0.76 mV,
    /// so the mean of twenty samples of a quiet pin lands between whole numbers - and rounded to a whole one it
    /// jumps between two of them. On the board that showed as square teeth over three millivolts where the
    /// sensor was standing still: our own arithmetic drawn as the signal's noise. A tenth is finer than the
    /// converter's own step and coarser than the lie of precision a hundredth would be.
    ///
    /// It costs nothing to keep: a tenth of the biggest reading this chip gives still fits the same two bytes
    /// in a ring as the whole millivolts did.
    pub tenths: u32,
    pub low: u16,
    pub high: u16,
}

impl Window {
    /// How far the samples of this window ranged - the shake of one reading, in millivolts.
    pub fn spread(&self) -> u16 {
        self.high - self.low
    }

    /// The mean in whole millivolts, rounded to the nearest - what a person reads on a card and in the log,
    /// where a tenth of a millivolt is noise on the page rather than information.
    pub fn mv(&self) -> u16 {
        ((self.tenths + 5) / 10) as u16
    }

    /// The mean as a ring keeps it: tenths of a millivolt in two bytes. Anything past 6553.5 mV is held at the
    /// brim - this chip cannot reach it, and a ring is not the place to find out otherwise.
    pub fn point(&self) -> u16 {
        self.tenths.min(u16::MAX as u32) as u16
    }
}

/// SAMPLES of one pin as one reading. None for no samples at all: a round where every read refused has no
/// millivolts, and calling that 0 mV would show a live sensor as one lying on the floor - the same rule
/// `sensor_mv` follows for a divider that is not one.
///
/// The sum is taken in u64, and the length with it: a u16 overflows after three samples, and a u32 after 65 538 -
/// a length nothing here passes today, and exactly the kind of limit nobody remembers when they later average a
/// slow channel over an hour. The mean of a wrapped sum is an ordinary quiet number in the middle of the scale.
/// The mean itself is never wider than the samples, so narrowing it back to u16 loses nothing.
pub fn window(samples: &[u16]) -> Option<Window> {
    let (first, rest) = samples.split_first()?;
    let (mut low, mut high, mut sum) = (*first, *first, *first as u64);
    for sample in rest {
        low = low.min(*sample);
        high = high.max(*sample);
        sum += *sample as u64;
    }
    let n = samples.len() as u64;
    // in tenths, rounded to the nearest and not cut: cutting leans every reading of this board the same way
    Some(Window { tenths: ((sum * 10 + n / 2) / n) as u32, low, high })
}

/// The window the chip's own calibration is valid in, at the widest attenuation: 0.150…2.450 V.
/// A number outside it is a number, but not one the calibration stands behind.
pub const WINDOW_MV: (u16, u16) = (150, 2_450);

/// Where MV stands against the window the calibration holds in: `under`, `inside`, `over`.
pub fn standing(mv: u16) -> &'static str {
    match mv {
        mv if mv < WINDOW_MV.0 => "under",
        mv if mv > WINDOW_MV.1 => "over",
        _ => "inside",
    }
}

/// The stronger of the two facts about MV, by code: `top` when it stood at the converter's top, else [`standing`].
pub fn edge(at_ceiling: bool, mv: u16) -> &'static str {
    match at_ceiling {
        true => "top",
        false => standing(mv),
    }
}

/// What is said beside a reading that fell outside that window - nothing when it is inside.
///
/// It is a word on a row of a card, so it is SHORT: the row already carries the millivolts and the spread, and
/// the first version of this took it to three wrapped lines on a phone. Both sides are named because they are
/// different facts - under the window is a pin nothing drives, over it is where the flame's darkness lives.
pub fn outside(mv: u16) -> &'static str {
    match mv {
        mv if mv < WINDOW_MV.0 => " · under the calibrated 150 mV",
        mv if mv > WINDOW_MV.1 => " · over the calibrated 2450 mV",
        _ => "",
    }
}

/// What is said beside a reading: that it stood at the top of the converter's scale, or - failing that - that
/// it fell outside the window the calibration holds in.
///
/// THE CEILING COMES FIRST AND REPLACES THE OTHER, because it is the stronger fact and the two said together
/// would read as one hedge. Outside the calibrated window the number is still about the pin, only less exactly;
/// at the ceiling it is not about the pin at all - every voltage above that point comes back as this one.
pub fn ceiling_words(at_ceiling: bool, mv: u16) -> String {
    match at_ceiling {
        true => " · at the top of the scale".to_string(),
        false => outside(mv).to_string(),
    }
}

/// The longest a window may hold the unit by the steady clock, in microseconds. A window is 40 ms of its own and a leap forward of the
/// steady clock adds one `PAD` (100 ms) to what it reads, so a limit under 140 ms would cut a window for a leap that cost it no time.
pub const MOST_HELD_US: u64 = 150_000;

/// What the samples of one window came to: how many were taken, how many stood at the top of the converter's scale, why the last read that
/// was refused was, and whether the window was cut for having held the unit past [`MOST_HELD_US`].
pub struct Gathered<E> {
    pub taken: usize,
    pub at_ceiling: usize,
    pub refused: Option<E>,
    pub cut: bool,
}

/// SAMPLES one after another: READ gives the raw count and the millivolts made from it, PAUSE waits between two samples, NOW_US reads the steady clock.
/// The samples taken are the first `taken` of SAMPLES, consecutive on the one pad. A window that has held longer than [`MOST_HELD_US`] by NOW_US is cut after the sample it
/// was in, and keeps what it took: a long pause is the other sensor's wait for the unit. No pause follows the last sample, since nothing is read after it.
pub fn gather<E>(
    samples: &mut [u16],
    mut read: impl FnMut() -> Result<(u16, u16), E>,
    mut pause: impl FnMut(),
    now_us: impl Fn() -> u64,
) -> Gathered<E> {
    let began = now_us();
    let (mut taken, mut refused, mut at_ceiling, mut cut) = (0, None, 0, false);
    for n in 0..samples.len() {
        match read() {
            Ok((raw, mv)) => {
                samples[taken] = mv;
                taken += 1;
                at_ceiling += usize::from(raw >= CEILING_RAW);
            }
            Err(why) => refused = Some(why),
        }
        if n + 1 == samples.len() {
            break;
        }
        pause();
        if now_us().saturating_sub(began) > MOST_HELD_US {
            cut = true;
            break;
        }
    }
    Gathered { taken, at_ceiling, refused, cut }
}

/// The largest raw reading this chip's converter can give at 12 bits - the ceiling of the scale, not of the
/// calibration. A sample standing here says the pin is AT LEAST this high and nothing about how much higher:
/// two voltages above it come back as one number (esp-idf-hal 0.47.0, src/adc.rs, `DirectConverter::MAX_READING`).
///
/// **A saturated sample does not come out at 3100 mV, and that is not a contradiction.** 3100 is what the
/// UNCALIBRATED arithmetic makes of this count - raw × max_mv / 4095, the same file's `DirectConverter`. We read
/// through the chip's curve calibration instead, and it turns the very same count into about 3153 mV on this
/// board. Two ways of naming one saturated reading, not two readings; whichever number is printed, what the
/// count says is "at least this high".
pub const CEILING_RAW: u16 = 4_095;

/// The ADC of the chip and the pins read through it: built for the board only.
#[cfg(target_os = "espidf")]
pub mod board;

pub mod gauge;
