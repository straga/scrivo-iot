//! HERE: the flue gas: the word of src/hardware/thermocouple.rs read once a second through crates/max6675 into the entity of api/src/flue.rs; the curve it keeps; the card it publishes;
//! its place in the module list.
//! NEVER HERE: a pin or a bus (the hardware), what the sixteen bits mean (crates/max6675), a word of the page (page/menu.json), a code of the stream (api/src/flue.rs). What this file
//! owns is the ORDER: read, keep, publish.
//! CHECKS: devices/burner/api/tests/flue_values.rs, crates/max6675/tests/max6675.rs.

use std::time::Duration;

use burner_api::flue::{self as api, at};
use curve::degrees;
use max6675::Reading;
use module::card::{Fill, Published, Reads};
use module::sensor::{Kept, Sensor};

use crate::hardware::thermocouple::Chip;

/// How often the chip is read. Reading it also starts its next measurement, and that measurement takes 170 ms typically and 220 at worst, so a faster round would fetch the same word
/// twice. A second is also the pace at which a person reads a page.
const EVERY: Duration = Duration::from_secs(1);

/// THE ROUND'S STACK, a sensor of its own (from the board of 04.10): the least free of the 8192 it had was 4348 bytes over five and a half hours of work, the page and the recording open; 6144 leaves 2300 - not under two kilobytes, the room a round that writes
/// one line of words wants (round::STACK). The number is this round's own: the common one is not touched.
const STACK: usize = 6144;

/// The period of the round, for the role it plays (src/roles.rs), which grows old by it.
pub fn every() -> Duration {
    EVERY
}

/// How long the curve of the flue is: 900 points at one a second - a quarter of an hour, which is the span the gases answer a portion of fuel in. 1800 bytes, and it never grows.
const SLOW: usize = 900;

/// Where its round is asked for its curves and its lines (firmware::sensor): what it keeps is read there only.
static DESK: module::polling::Desk = module::polling::Desk::new();

/// What the flue keeps between rounds: the last word the chip said, with the raw word beside what it means, and ONE ring of the degrees and not two: the chip converts in 220 ms and
/// the gases move in minutes, so a fast curve here would draw the same number sixty times over. The step of the round is measured there too, and the curve is still drawn on EVERY.
static KEPT: Kept<Flue> = Kept::new(curve::Ring::new(), EVERY.as_millis() as u32);

/// The card of the flue as its round last published it.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

/// Puts the card together and publishes it; the round calls it every round.
fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
    crate::roles::FLUE_GAS.publish(match KEPT.last() {
        Some((max6675::Reading::Is(celsius), _)) => Some(celsius),
        _ => None,
    });
}

/// The flue by address: what the API names, from what the chip last said.
fn values(card: &mut dyn Fill) {
    card.put(at::FLUE_TEMP, api::value(KEPT.last().map(|(said, _)| said)));
}

/// Starts the round. The chip is opened by the round itself, on its first reading.
pub fn start() -> Result<(), esp_idf_svc::sys::EspError> {
    let answers = module::polling::Answers {
        curve,
        lines: |line| {
            lines(line);
            Ok(())
        },
    };
    module::polling::poll("flue", STACK, api::CARD, &KEPT, || EVERY, Flue { chip: None, opened: false, told: None }, publish, &DESK, answers)
}

/// The MAX6675 as a sensor: a word a round, and the raw word kept beside what it means, because the shape of the word - all zeroes or all ones - is what tells one wire fault from another.
struct Flue {
    chip: Option<Chip>,
    /// Whether opening the chip was tried: a bus that would not open is said once, and the round reads nothing after it.
    opened: bool,
    /// The reading as it was last said out loud, so that saying it again is only for one that moved.
    told: Option<Reading>,
}

impl Sensor for Flue {
    type Reading = (Reading, u16);
    type Curves = curve::Ring<SLOW>;
    type Shown = Option<Reading>;

    fn read(&mut self) -> Option<(Reading, u16)> {
        if !self.opened {
            self.opened = true;
            match Chip::open() {
                Ok(chip) => self.chip = Some(chip),
                Err(why) => log::error!("flue: the SPI bus could not be opened: {why}"),
            }
        }
        let chip = self.chip.as_ref()?;
        match chip.word() {
            Ok(word) => Some((max6675::read(word), word)),
            // A bus that will not carry a transaction says nothing about the thermocouple; it says the chip is not being reached, which is what Silent is for.
            Err(why) => {
                log::warn!("flue: the SPI transaction failed: {why}");
                Some((Reading::Silent, 0))
            }
        }
    }

    fn record(curve: &mut Self::Curves, (said, _): &(Reading, u16)) {
        if let Reading::Is(celsius) = said {
            curve.push(degrees::point(*celsius));
        }
    }

    fn shown(reading: Option<&(Reading, u16)>) -> Option<Reading> {
        reading.map(|(said, _)| *said)
    }

    /// "THE READING CHANGED" IS NOT AN EVENT ON A THERMOCOUPLE, AND A THRESHOLD DOES NOT SAVE IT. Measured 21.09: the reading wanders a whole degree at a still 19 °C - 18.2, 20.2, 19.2 - so a
    /// threshold that silenced the wander would also swallow real movement of the same size. Where movement belongs is the curve, which the page draws from every reading. What is an event is
    /// the reading ceasing to be a number at all, or becoming one again: that is a wire question, and somebody has to go and look at it. Every reading waits for `log burner::flue debug`,
    /// and the raw word goes with it.
    fn told(&mut self, reading: Option<&(Reading, u16)>) {
        let Some(&(said, raw)) = reading else { return };
        let telling = !matches!((self.told, said), (Some(Reading::Is(_)), Reading::Is(_))) && self.told != Some(said);
        if telling {
            log::info!("flue: {} (word {raw:#06x})", max6675::words(said));
            self.told = Some(said);
        } else {
            log::debug!("flue: {} (word {raw:#06x})", max6675::words(said));
        }
    }
}

/// The curve called NAME, or None when it is not ours: the degrees as the chip gave them, ONE line and not two - the chip gives degrees, not a voltage we scale, so there is no raw reading of
/// our own to draw beside them. What the line is called and what it is drawn against are the menu's.
fn curve(name: &str) -> Option<page::Curve> {
    if name != api::CURVE {
        return None;
    }
    let points = KEPT.curves(|c| c.points());
    Some(page::Curve {
        step_ms: EVERY.as_millis() as u32,
        lines: vec![page::Line { words: String::new(), unit: "°C".into(), points: points.into_iter().map(degrees::celsius).collect(), span: None }],
    })
}

/// The shell's line about the flue, beside the readings of the bus: what it measures, not what measures it.
fn lines(line: &mut dyn FnMut(&str)) {
    let said = KEPT.last().map_or_else(|| "not read yet".to_string(), |(said, _)| max6675::words(said));
    line(&format!("flue gas {said}"));
}

/// The flue's thermocouple by the modules' contract (crates/module).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn curve(&self, name: &str) -> Option<page::Curve> {
        (name == api::CURVE).then(|| module::polling::curve(&DESK, name)).flatten()
    }

    fn lines(&self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        module::polling::lines(&DESK, line)
    }
}
