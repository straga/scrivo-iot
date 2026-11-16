//! HERE: the draught: the window of src/hardware/adc.rs, read ten times a second, through the divider - two settings of crates/settings, taken afresh every round - and crates/draught into
//! the entities of api/src/draught.rs; the curves it keeps; the card it publishes; its place in the module list.
//! NEVER HERE: a pin (burner.yaml, the hardware), what millivolts mean (crates/draught), a word of the page (page/menu.json), a code of the stream (api/src/draught.rs). What this file owns is
//! the ORDER: read, keep, publish.
//! CHECKS: devices/burner/api/tests/draught_values.rs, crates/draught/tests/draught.rs.
//!
//! TEN TIMES A SECOND, AND THE NUMBER IS THE SENSOR'S. The Huba answers in under 10 ms and carries about 10 Hz; read slower and the pulsations the regulator will have to ride are simply not
//! there to see, read faster and the sensor has nothing new to say.

use std::time::Duration;

use burner_api::draught::{self as api, at};
use burner_api::pin::Reading;
use draught::Reading as Pressure;
use firmware::nvs_settings;
use module::card::{Fill, Published, Reads};
use module::sensor::{Kept, Sensor};
use page::dictionary::Value;
use settings::Setting;

use crate::hardware::adc::{DraughtPin, Sample};

/// How often the reading is taken.
const EVERY: Duration = Duration::from_millis(100);

/// THE ROUND'S STACK, a sensor of its own (from the board of 04.10): the least free of the 8192 it had was 4308 bytes over five and a half hours of work, the page and the recording open; 6144 leaves 2260 - not under two kilobytes, the room a round that writes
/// one line of words wants (round::STACK). The number is this round's own: the common one is not touched.
const STACK: usize = 6144;

/// The period of the round, for the role that grows old by it (src/roles.rs).
pub fn every() -> Duration {
    EVERY
}

/// FAST holds 600 readings at one every 100 ms - a minute of the value; SLOW holds 900 means at one a second - a quarter of an hour. RUN is the readings of one point of the slow curve.
/// 3000 bytes for the sensor, and it never grows.
const FAST: usize = 600;
const SLOW: usize = 900;
const RUN: usize = 10;

/// Where its round is asked for its curves and its lines (firmware::sensor): what it keeps is read there only.
static DESK: module::polling::Desk = module::polling::Desk::new();

/// What the draught keeps between rounds: the last window and the two curves of the pin in MILLIVOLTS - the pascals are made by the divider, a setting, and a curve of pascals drawn an hour
/// ago would be arithmetic nobody uses any more; the millivolts are what the board saw, and the pascals are made from them when they are drawn.
static KEPT: Kept<Draught> = Kept::new(Curves { pair: curve::Pair::new(), second: curve::Mean::new(), said_second: None }, EVERY.as_millis() as u32);

/// The two curves of the pin, and the mean of its last second.
struct Curves {
    pair: curve::Pair<FAST, SLOW, RUN>,
    /// The run of rounds a second is made of, in the unit the curves keep, so the mean is taken before anything is thrown away.
    second: curve::Mean<RUN>,
    /// THE MEAN OVER A SECOND, AND IT IS WHAT THE REGULATOR READS - never a single round. The noise here is the breathing of the air itself: about 5 % of the value, and up to half the signal at small travel. The tract answers in seconds,
    /// so a second of readings costs nothing and takes the noise down by about three. The curve and the eye get the raw rounds - the shape of the breathing is what a person reads there.
    said_second: Option<Pressure>,
}

/// The card of the draught as its round last published it.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

/// The two arms of the divider as they are kept now, in ohms. Taken afresh every time they are used, so a number written into the card works from the next reading and not from the next
/// restart.
fn divider() -> (u32, u32) {
    let now = nvs_settings::now();
    let ohms = |s| now.value(s).parse().unwrap_or(0);
    (ohms(&burner_settings::DIVIDER_TOP), ohms(&burner_settings::DIVIDER_BOTTOM))
}

/// The millivolts of the pin in the last window, for the journal's line about a burning off by the draught: what the board saw, before the divider.
pub fn last_mv() -> Option<u32> {
    KEPT.last().map(|s| u32::from(s.window.mv()))
}

fn reading() -> Option<Reading> {
    KEPT.last().map(|s| Reading { window: s.window, at_ceiling: s.at_ceiling })
}

/// Puts the card together and publishes it; the round calls it every round.
fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
    crate::roles::DRAUGHT.publish(KEPT.curves(|c| c.said_second));
}

/// The draught by address: what the API names, from the last window and the divider of now.
fn values(card: &mut dyn Fill) {
    api::put_values(reading(), divider(), &mut |at, value| card.put(at, value));
}

/// Starts the round. Call once, after `hardware::adc::open`.
pub fn start(pin: esp_idf_svc::hal::gpio::Gpio4<'static>) -> Result<(), esp_idf_svc::sys::EspError> {
    let pin = DraughtPin::new(pin)?;
    log::info!("draught: GPIO4, a window every {} ms, curves {FAST} fast and {SLOW} slow", EVERY.as_millis());
    let answers = module::polling::Answers {
        curve,
        lines: |line| {
            lines(line);
            Ok(())
        },
    };
    module::polling::poll("draught", STACK, api::CARD, &KEPT, || EVERY, Draught { pin }, publish, &DESK, answers)
}

/// The draught's pin as a sensor: a window a round.
struct Draught {
    pin: DraughtPin,
}

impl Sensor for Draught {
    type Reading = Sample;
    type Curves = Curves;
    type Shown = Option<Value>;

    fn read(&mut self) -> Option<Sample> {
        self.pin.read()
    }

    fn record(curves: &mut Self::Curves, sample: &Sample) {
        let point = sample.window.point();
        curves.pair.push(point);
        if let Some(mean) = curves.second.add(point) {
            let (top, bottom) = divider();
            curves.said_second = Some(draught::read_tenths(mean as u32, top, bottom));
        }
    }

    /// ANNOUNCED BY WHAT THE ROW SHOWS - the pressure in whole pascals, or its reason -, not by the reading underneath it: a fraction differs from the one before it on every round of a
    /// breathing sensor, which would announce the card ten times a second for a change nobody can see. THE DIVIDER IS TAKEN HERE, EVERY ROUND: a person turns it while the boiler runs.
    fn shown(sample: Option<&Sample>) -> Option<Value> {
        let mut out = None;
        api::put_values(sample.map(|s| Reading { window: s.window, at_ceiling: s.at_ceiling }), divider(), &mut |a, value| {
            if a == at::BLAST_PRESSURE {
                out = Some(match value {
                    Value::Number(pa) => Value::Number(pa.round()),
                    other => other,
                });
            }
        });
        out
    }

    /// A NEW PASCAL IS NOT NEWS, AND THE JOURNAL IS NOT THE CARD: a working fan moves the draught by a whole pascal most seconds. The number lives on the card; the log says it when somebody
    /// turns the module up - with the divider it was read through, so a line can be read without knowing what the settings held.
    fn told(&mut self, sample: Option<&Sample>) {
        if let Some(s) = sample {
            let (top, bottom) = divider();
            log::debug!(
                "draught: {} (pin {} mV, spread {} mV of {}, divider {top}/{bottom} Ω)",
                draught::words(draught::read(u32::from(s.window.mv()), top, bottom)),
                s.window.mv(),
                s.window.spread(),
                s.taken
            );
        }
    }
}

/// The curve called NAME, or None when it is not ours: the pascals the pin means through the divider standing now, and the millivolts it held, two lines. What the lines are called and what they are
/// drawn against are the menu's. The points are tenths of a millivolt and both lines are made from them without a whole-number step between: one millivolt is 0.15 Pa, which drew a quiet pin
/// as square teeth.
fn curve(name: &str) -> Option<page::Curve> {
    let step = KEPT.step_ms();
    let (points, step) = match name {
        n if n == api::CURVES[0] => (KEPT.curves(|c| c.pair.fast.points()), step),
        n if n == api::CURVES[1] => (KEPT.curves(|c| c.pair.slow.points()), step * RUN as u32),
        _ => return None,
    };
    let (top, bottom) = divider();
    let pascals = points
        .iter()
        .map(|tenths| match draught::read_tenths(u32::from(*tenths), top, bottom) {
            draught::Reading::Is(pa) => pa,
            // not a number, and not a zero either: the curve breaks where the reading did
            _ => f32::NAN,
        })
        .collect();
    Some(page::Curve {
        step_ms: step,
        lines: vec![
            page::Line { words: String::new(), unit: "Pa".into(), points: pascals, span: None },
            page::Line { words: String::new(), unit: "mV".into(), points: points.iter().map(|tenths| *tenths as f32 / 10.0).collect(), span: None },
        ],
    })
}

/// The shell's line about the draught, beside the other readings: its state and the pin under it. For a person at a console, not for the page.
fn lines(line: &mut dyn FnMut(&str)) {
    let last = KEPT.last();
    let window = last.map(|s| s.window);
    let (top, bottom) = divider();
    let said = window.map_or_else(|| "not read yet".to_string(), |w| draught::words(draught::read(u32::from(w.mv()), top, bottom)));
    let pin = window.map_or_else(
        || "not read yet".to_string(),
        |w| format!("{} mV · spread {} mV of {}{}", w.mv(), w.spread(), burner_api::pin::SAMPLES, analog::ceiling_words(last.is_some_and(|s| s.at_ceiling), w.mv())),
    );
    line(&format!("blast pressure {said}"));
    line(&format!("pin {pin}"));
}

/// The draught by the modules' contract (crates/module).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn settings(&self) -> Vec<&'static Setting> {
        api::SETTINGS.to_vec()
    }

    /// Where the sensor's own zero lands on the pin through the divider written in, or that it is no divider.
    fn setting_values(&self, each: &mut dyn FnMut(usize, &Value)) {
        each(at::ZERO_AT, &api::zero_at(divider()));
    }

    fn curve(&self, name: &str) -> Option<page::Curve> {
        api::CURVES.contains(&name).then(|| module::polling::curve(&DESK, name)).flatten()
    }

    fn lines(&self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        module::polling::lines(&DESK, line)
    }
}
