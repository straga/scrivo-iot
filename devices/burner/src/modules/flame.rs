//! HERE: the flame: the window of src/hardware/adc.rs, read ten times a second, through the thresholds - settings of crates/settings, taken afresh every
//! round - and crates/flame into the entities of api/src/flame.rs; the curves it keeps; the card it publishes; its place in the module list.
//! NEVER HERE: a pin (burner.yaml, the hardware), what millivolts mean (crates/flame), a word of the page (page/menu.json), a code of the stream
//! (api/src/flame.rs). What this file owns is the ORDER: read, keep, publish.
//! CHECKS: devices/burner/api/tests/flame_values.rs, crates/flame/tests/flame.rs.

use std::time::Duration;

use burner_api::flame::{self as api, Reading};
use flame::{Flame, Thresholds};
use firmware::nvs_settings;
use module::card::{Fill, Published, Reads};
use module::sensor::{Kept, Sensor};
use page::dictionary::Value;
use settings::Setting;

use crate::hardware::adc::{FlamePin, Sample};

/// How often the reading is taken. Ten times a second, because the thing worth seeing is the flicker of a fire, and on a second's step a flicker
/// is not slow - it is invisible.
const EVERY: Duration = Duration::from_millis(100);

/// THE ROUND'S STACK, a sensor of its own (from the board of 04.10): the least free of the 8192 it had was 5328 bytes over five and a half hours of work, the page and the recording open; 6144 leaves 3280 - not under two kilobytes, the room a round that writes
/// one line of words wants (round::STACK). The number is this round's own: the common one is not touched.
const STACK: usize = 6144;

/// The period of the round, for the role it plays (src/roles.rs), which grows old by it.
pub fn every() -> Duration {
    EVERY
}

/// FAST holds 600 readings at one every 100 ms - a minute of the value; SLOW holds 900 means at one a second - a quarter of an hour. RUN is the
/// readings of one point of the slow curve. 3000 bytes for the sensor, and it never grows.
const FAST: usize = 600;
const SLOW: usize = 900;
const RUN: usize = 10;

/// What the flame keeps between rounds: the last window, the two curves of the pin in raw millivolts - the thresholds that turn them into states
/// are settings, and a curve of states drawn through yesterday's thresholds would answer a question nobody asked - and the step the rounds take.
static DESK: module::polling::Desk = module::polling::Desk::new();
static KEPT: Kept<FlameSensor> = Kept::new(curve::Pair::new(), EVERY.as_millis() as u32);

/// The card of the flame as its round last published it.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

/// The thresholds as they are kept now. Taken afresh by every round: a number typed in works from the next reading, not from the next restart.
fn thresholds() -> Thresholds {
    let now = nvs_settings::now();
    let mv = |s| now.value(s).parse().unwrap_or(0);
    Thresholds { lit_mv: mv(&burner_settings::FLAME_LIT), steady_mv: mv(&burner_settings::FLAME_STEADY) }
}

fn reading() -> Option<Reading> {
    KEPT.last().map(|s| Reading { window: s.window, at_ceiling: s.at_ceiling })
}

/// Puts the card together and publishes it; the round calls it every round.
fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
    crate::roles::FLAME.publish(KEPT.last().map(|s| flame::read(u32::from(s.window.mv()), thresholds())));
}

/// The flame by address: what the API names, from the last reading and the thresholds of now.
fn values(card: &mut dyn Fill) {
    api::put_values(reading(), thresholds(), &mut |name, value| card.put(api::address(name), value));
}

/// Starts the round. Call once, after `hardware::adc::open`.
pub fn start(pin: esp_idf_svc::hal::gpio::Gpio7<'static>) -> Result<(), esp_idf_svc::sys::EspError> {
    let pin = FlamePin::new(pin)?;
    log::info!("flame: GPIO7, a window every {} ms, curves {FAST} fast and {SLOW} slow", EVERY.as_millis());
    let answers = module::polling::Answers {
        curve,
        lines: |line| {
            lines(line);
            Ok(())
        },
    };
    module::polling::poll("flame", STACK, api::CARD, &KEPT, || EVERY, FlameSensor { pin, voice: flame::voice::Voice::new(), started: between::clock::now() }, publish, &DESK, answers)
}

/// The flame's pin as a sensor: a window a round.
struct FlameSensor {
    pin: FlamePin,
    /// What the journal has been told of the flame, and what has held (crates/flame, `voice`).
    voice: flame::voice::Voice,
    /// The start of the seconds the voice counts by.
    started: between::clock::Moment,
}

impl FlameSensor {
    /// WHAT THE FLAME SAYS IN THE JOURNAL: a word is said when it has held, and only out and lit are lines; weak and steady trading places is a detail at `debug`. It wrote a long line at every change of its word,
    /// and on a low fire the word changes several times a second. Called every round with the reading, and the voice decides.
    fn say(&mut self, s: &Sample) {
        let t = thresholds();
        let word = flame::read(u32::from(s.window.mv()), t);
        let at = between::clock::now().since(self.started).as_secs() as u32;
        match self.voice.heard(word, at) {
            Some(flame::voice::Heard::Out) => log::info!("flame: out, {} mV", s.window.mv()),
            Some(flame::voice::Heard::Lit) => log::info!("flame: lit, {} mV", s.window.mv()),
            Some(flame::voice::Heard::Unconfigured) => log::warn!("flame: the thresholds are no thresholds (seen {} mV, steady {} mV)", t.lit_mv, t.steady_mv),
            Some(flame::voice::Heard::Detail) => log::debug!(
                "flame: {} (pin {} mV, spread {} mV of {}, seen {} mV, steady {} mV)",
                flame::words(word),
                s.window.mv(),
                s.window.spread(),
                s.taken,
                t.lit_mv,
                t.steady_mv
            ),
            None => {}
        }
    }
}

impl Sensor for FlameSensor {
    type Reading = Sample;
    type Curves = curve::Pair<FAST, SLOW, RUN>;
    type Shown = Option<Flame>;

    fn read(&mut self) -> Option<Sample> {
        let sample = self.pin.read();
        match &sample {
            Some(s) => self.say(s),
            None => self.voice.unread(),
        }
        sample
    }

    fn record(curves: &mut Self::Curves, sample: &Sample) {
        curves.push(sample.window.point());
    }

    /// What the fire is doing, through the thresholds as they are kept NOW - taken every round, not above it.
    fn shown(sample: Option<&Sample>) -> Option<Flame> {
        sample.map(|s| flame::read(u32::from(s.window.mv()), thresholds()))
    }
}

/// The curve called NAME, or None when it is not ours: the pin's millivolts as it held them, one line, with its unit and its points. What the line
/// is called and what it is drawn against are the menu's.
fn curve(name: &str) -> Option<page::Curve> {
    let step = KEPT.step_ms();
    let (points, step) = match name {
        n if n == api::CURVES[0] => (KEPT.curves(|c| c.fast.points()), step),
        n if n == api::CURVES[1] => (KEPT.curves(|c| c.slow.points()), step * RUN as u32),
        _ => return None,
    };
    Some(page::Curve {
        step_ms: step,
        lines: vec![page::Line { words: String::new(), unit: "mV".into(), points: points.iter().map(|tenths| *tenths as f32 / 10.0).collect(), span: None }],
    })
}

/// The shell's line about the flame, beside the other readings: its state and the pin under it. For a person at a console, not for the page.
fn lines(line: &mut dyn FnMut(&str)) {
    let last = KEPT.last();
    let window = last.map(|s| s.window);
    let said = window.map_or_else(|| "not read yet".to_string(), |w| flame::words(flame::read(u32::from(w.mv()), thresholds())));
    let pin = match window {
        None => "not read yet".to_string(),
        Some(w) => format!("{} mV · spread {} mV of {}{}", w.mv(), w.spread(), api::SAMPLES, analog::ceiling_words(last.is_some_and(|s| s.at_ceiling), w.mv())),
    };
    line(&format!("flame {said}"));
    line(&format!("pin {pin}"));
}

/// The flame by the modules' contract (crates/module).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn settings(&self) -> Vec<&'static Setting> {
        vec![&burner_settings::FLAME_LIT, &burner_settings::FLAME_STEADY]
    }

    /// Whether the two thresholds stand in the order they must: steady under seen.
    fn setting_values(&self, each: &mut dyn FnMut(usize, &Value)) {
        each(api::at::STEADY_ORDER, &api::order_value(thresholds()));
    }

    fn curve(&self, name: &str) -> Option<page::Curve> {
        (api::CURVES.contains(&name)).then(|| module::polling::curve(&DESK, name)).flatten()
    }

    fn lines(&self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        module::polling::lines(&DESK, line)
    }
}
