//! Reading an analog pin of ADC1 (the half of this crate that needs the chip; on a computer only the logic above is built): the driver every such pin shares, one channel per pin, and a window of
//! samples taken off it.
//!
//! WHY THIS IS HERE AND NOT IN A MODULE OF THE DEVICE. Two sensors of the burner read ADC1 - the draught on
//! GPIO4 and the flame on GPIO7 - and the driver of the unit is ONE object: a module that takes `ADC1` for
//! itself leaves the second sensor nothing to read through. So the unit is opened once and lent out by
//! reference, which is what `AdcChannelDriver` takes (it holds any `Borrow<AdcDriver>`).
//!
//! What the samples come to is crates/analog, tested on the computer. What the millivolts MEAN is the sensor's
//! own crate. Here are the pins, the attenuation and the waiting.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use esp_idf_svc::hal::adc::oneshot::config::{AdcChannelConfig, Calibration};
use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::adc::{attenuation, AdcChannel, Resolution, ADCU1, ADC1};
use esp_idf_svc::hal::gpio::ADCPin;
use esp_idf_svc::sys::EspError;

use crate::gauge::{self, Lane};

/// How the chip is asked to turn raw counts into millivolts. NOT THE SAME EVERYWHERE, and this is a compile
/// error rather than a wrong number: the S3 and the C-series fit a curve, the plain ESP32 and the S2 fit a
/// line, and each knows only its own. The burner is an S3, bench_dualr3 is an ESP32, and this crate is common
/// to both - which is exactly how the difference was found.
#[cfg(any(esp32c3, esp32c6, esp32s3))]
const CALIBRATION: Calibration = Calibration::Curve;
#[cfg(any(esp32, esp32c2, esp32s2))]
const CALIBRATION: Calibration = Calibration::Line;

/// How every pin of this board is read: the widest attenuation, 12 bits, the calibration the chip has.
///
/// `DB_12` is the same attenuation our records call 11 dB and the same one the draught was measured through -
/// IDF renamed the constant because the real figure is closer to 12 (esp-idf-hal, adc.rs: `DB_11` is now a
/// deprecated alias of `DB_12`). The window of 0.150…2.450 V above is that attenuation's.
const HOW: AdcChannelConfig =
    AdcChannelConfig { attenuation: attenuation::DB_12, resolution: Resolution::Resolution12Bit, calibration: CALIBRATION };

/// One channel of ADC1, as this firmware holds them.
///
/// THE DRIVER INSIDE IS PRIVATE ON PURPOSE. Its own `read()` is public, and a module holding it could take a
/// sample without the queue below - one ESP_ERR_TIMEOUT and two pads mixed into one window, found again the
/// hard way. Wrapped like this, the only way to a sample is `window`, which holds the unit. A rule nobody can
/// break is worth more than a rule everybody has to remember.
pub struct Channel<C: AdcChannel<AdcUnit = ADCU1>>(AdcChannelDriver<'static, C, &'static AdcDriver<'static, ADCU1>>);

pub use crate::CEILING_RAW;

static UNIT: OnceLock<&'static AdcDriver<'static, ADCU1>> = OnceLock::new();

/// WHOSE TURN IT IS AT THE UNIT. One ADC1, two sensors, and a thread each: read at the same time and IDF hands
/// back ESP_ERR_TIMEOUT, while the samples that do come through are taken with the other channel's pad being
/// switched in between them. Both were measured the moment the flame's round started - the draught's window
/// went from a spread of 5…28 mV to 37…40, and three reads in twenty of the flame refused.
///
/// So a window holds the unit for its whole length, and no longer than `MOST_HELD_US`. A window is some 40 ms on the board (measured, 11.27) and a round
/// 100 ms: the two sensors hold the unit four tenths of the time each, and buy back both numbers.
static TURN: Mutex<()> = Mutex::new(());

/// Opens ADC1 and keeps it for as long as the board runs. Call once, from main; every channel afterwards is
/// built through `channel`.
pub fn open(adc1: ADC1<'static>) -> Result<(), EspError> {
    let driver: &'static AdcDriver<'static, ADCU1> = &*Box::leak(Box::new(AdcDriver::new(adc1)?));
    // A second call would leave the first driver leaked and in charge; say so rather than swap it silently.
    if UNIT.set(driver).is_err() {
        log::warn!("analog: ADC1 was already open; the second opening is ignored");
    }
    Ok(())
}

/// A channel for PIN, on the unit opened by `open`. None when nothing has opened it - a module that reads
/// before main has opened the unit is a bug, and it says so instead of reading zeroes.
///
/// BUILT ONCE AND READ MANY TIMES: building one resets the pad, and a reading that resets the pin it is about
/// would be measuring its own hand.
pub fn channel<C, P>(pin: P) -> Result<Channel<C>, EspError>
where
    C: AdcChannel<AdcUnit = ADCU1>,
    P: ADCPin<AdcChannel = C> + 'static,
{
    let unit = *UNIT.get().ok_or_else(EspError::from_infallible::<{ esp_idf_svc::sys::ESP_ERR_INVALID_STATE }>)?;
    Ok(Channel(AdcChannelDriver::new(unit, pin, &HOW)?))
}

/// One reading of CHANNEL: SAMPLES.len() samples GAP apart, as one number with the spread they ranged over.
///
/// A single sample shakes - measured at 5…28 mV on the draught's pin - so nothing here reads a pin once. The
/// refusals are counted rather than thrown away: one refusal in twenty leaves nineteen samples of the same pin,
/// and a round where every read refused has no millivolts at all, which is not the number zero.
///
/// The unit is held for the length of the window (see TURN): the samples of one reading must be consecutive on
/// one pad, or the number is an average of two pins taking turns.
pub fn window<C: AdcChannel<AdcUnit = ADCU1>>(
    channel: &mut Channel<C>,
    samples: &mut [u16],
    gap: Duration,
    lane: &Lane,
) -> Round {
    // held for the whole window, not per sample: samples of one window have to be consecutive on one pad. The lane counts the wait for the lock
    // and the hold of it, outside the lock (gauge::through).
    gauge::through(lane, clock_us, || TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner()), || {
        // ONE SAMPLE IS THROWN AWAY BEFORE THE WINDOW, and it is measured rather than borrowed from a datasheet:
        // with a second sensor on the unit, the first sample of every window came back high - 166…168 mV where the
        // rest of the window sat at 161…164, in eleven windows of twelve. It is the pad being switched back from
        // the other channel, and it is not about this pin.
        let _ = channel.0.read_raw();
        // THE RAW COUNT IS READ, NOT THE MILLIVOLTS, and the millivolts are made from it here. One reading then
        // answers both questions - what the pin holds, and whether the converter had any room left to say it.
        // The pause is a nap by the tick (between::nap), not std's sleep: that one ends by the chip's clock, and a leap of
        // it back lengthens the sleep by the leap while this thread holds the unit (696 ms, the burner 20.10).
        let got = crate::gather(
            samples,
            || channel.0.read_raw().and_then(|raw| channel.0.raw_to_mv(raw).map(|mv| (raw, mv))),
            || between::nap(gap),
            clock_us,
        );
        let taken = got.taken;
        Round { window: crate::window(&samples[..taken]), taken, at_ceiling: got.at_ceiling, refused: got.refused, cut: got.cut }
    })
}

/// The board's steady clock in microseconds, read without telling a leap: this runs under the lock of the unit, where a line of the log has no place.
fn clock_us() -> u64 {
    between::clock::now_quiet().as_micros()
}

/// The lanes of the pins opened so far, by the name of the round that reads through each: what `status` and the note a silence leaves say.
static LANES: Mutex<Vec<(&'static str, &'static Lane)>> = Mutex::new(Vec::new());

/// One line for each pin opened: what its round held the unit for, waited for it and took as a step (gauge::line).
pub fn lane_lines() -> Vec<String> {
    let now_ms = clock_us() / 1000;
    let lanes = LANES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    lanes.iter().map(|(name, lane)| gauge::line(name, &lane.report(now_ms))).collect()
}

/// How many samples one reading of a pin is made of, and how far apart they are taken.
///
/// MEASURED, NOT CHOSEN. One sample of an ADC pin shakes by 3 mV (the scan of ADC1). Twenty samples a
/// millisecond apart cover 20 ms, which is two whole periods of the 100 Hz a phase dimmer puts on the mains and one
/// of the 50 Hz behind it: whatever of that gets past the capacitor at the pin averages to nothing over a whole
/// number of periods, and a window that is NOT a whole number of them would let a beat through.
pub const SAMPLES: usize = 20;
pub const GAP: Duration = Duration::from_millis(1);

/// One pin read a window at a time - the draught's and the flame's way, written once.
pub struct Pin<C: AdcChannel<AdcUnit = ADCU1>> {
    /// What the log calls it when the converter refuses samples.
    name: &'static str,
    channel: Channel<C>,
    /// What this pin's round costs the unit, written by its read and read by `status`.
    lane: &'static Lane,
    /// When the last read began, for the step.
    last_began: Option<u64>,
}

/// One window of a pin: its millivolts with their spread, how many samples it was made of, and whether every one
/// of them stood at the top of the scale.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sample {
    pub window: crate::Window,
    pub taken: usize,
    pub at_ceiling: bool,
}

impl<C: AdcChannel<AdcUnit = ADCU1>> Pin<C> {
    /// PIN as a channel of the unit `open` opened, called NAME in the log.
    pub fn new<P>(name: &'static str, pin: P) -> Result<Self, EspError>
    where
        P: ADCPin<AdcChannel = C> + 'static,
    {
        let lane: &'static Lane = Box::leak(Box::new(Lane::new()));
        LANES.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push((name, lane));
        Ok(Self { name, channel: channel(pin)?, lane, last_began: None })
    }

    /// One window; None when every sample refused - no millivolts at all, which is not the number zero.
    pub fn read(&mut self) -> Option<Sample> {
        let began = clock_us();
        if let Some(before) = self.last_began.replace(began) {
            self.lane.step(began.saturating_sub(before).min(u64::from(u32::MAX)) as u32, began / 1000);
        }
        let mut samples = [0u16; SAMPLES];
        let round = window(&mut self.channel, &mut samples, GAP, self.lane);
        if round.cut {
            log::warn!("{}: a window held the unit past {} ms and was cut after {} of {SAMPLES} samples", self.name, crate::MOST_HELD_US / 1000, round.taken);
        }
        if let Some(why) = &round.refused {
            log::warn!("{}: {} of {SAMPLES} reads refused: {why}", self.name, SAMPLES - round.taken);
        }
        let at_ceiling = round.all_at_ceiling();
        round.window.map(|window| Sample { window, taken: round.taken, at_ceiling })
    }
}

/// What one round of reading came to.
pub struct Round {
    pub window: Option<crate::Window>,
    /// How many samples were actually taken - the rest refused.
    pub taken: usize,
    /// How many of them stood at `CEILING_RAW`. **This is the one fact that tells "the pin is high" from "the
    /// pin is higher than this chip can say"**, and neither the millivolts nor the spread carry it: a pin pinned
    /// to the ceiling reads as a steady number, which looks exactly like a steady source.
    pub at_ceiling: usize,
    pub refused: Option<EspError>,
    /// Whether the window was cut for having held the unit past `MOST_HELD_US`: it has fewer samples than asked, all consecutive.
    pub cut: bool,
}

impl Round {
    /// Whether every sample of this round stood at the top of the scale - the pin is at least that high, and
    /// how much higher is not a question this reading can answer.
    pub fn all_at_ceiling(&self) -> bool {
        self.taken > 0 && self.at_ceiling == self.taken
    }
}
