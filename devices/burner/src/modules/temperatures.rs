//! HERE: the water's temperatures: the bus of src/hardware/rs485.rs asked round and round - the askings built from crates/burner_map, what each answer says taken by crates/modbus -
//! the readings it keeps, the curves of the sockets, the card it publishes, its place in the module list. Each socket's role and the pace of the round are settings of crates/settings,
//! taken afresh every round.
//! NEVER HERE: a pin or a frame (the hardware, crates/modbus), what a reading means (crates/burner_map), a word of the page (page/menu.json), a code of the stream
//! (api/src/temperatures.rs). What this file owns is the ORDER: ask, keep, publish.
//! CHECKS: devices/burner/api/tests/temperatures_values.rs, crates/burner_map/tests/map.rs.
//!
//! WHAT is read and HOW MANY there are lives in crates/burner_map, tested on the computer: adding a reading is a line in its MAP, and every size follows from it. Nothing here is told how
//! many readings there are. An outcome other than "accepted" leaves the readings unknown - not zero and not the last value. A frame from the other module on the bus is an outcome of its
//! own: the PTA9B01 can be set to send by itself, and then its frame lands in the middle of an asking addressed elsewhere.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use burner_api::temperatures::{self as api, CURVES};
use burner_map::{Readings, COUNT, MAP, MAX_ASKS, UNREAD};
use curve::degrees;
use firmware::nvs_settings;
use modbus::{Ask, Outcome, Reading};
use module::card::{Fill, Published, Reads};
use module::sensor::{Kept, Sensor};
use settings::Setting;

use crate::hardware::rs485;

/// How long to wait between rounds: the setting `poll_every`, asked afresh each round, so a number typed in works from the next one.
pub fn every() -> Duration {
    Duration::from_secs(nvs_settings::value(&burner_settings::POLL_EVERY).parse().unwrap_or(2))
}

/// How long each channel's curve is: 450 points at one every two seconds - a quarter of an hour, the same span as the flue's. Seven channels of 900 bytes each: 6300 bytes, and it
/// never grows.
const SLOW: usize = 450;

/// Where its round is asked for its curves and its lines (firmware::sensor): what it keeps is read there only.
static DESK: module::polling::Desk = module::polling::Desk::new();

/// What the temperatures keep between rounds: the last round's readings and a curve per channel of the map, in the map's own order - tenths of a degree lifted by 100 °C
/// (curve::degrees). The curves are drawn on the period of the setting, which is what the rounds are asked for.
static KEPT: Kept<Temperatures> = Kept::new([const { curve::Ring::new() }; COUNT], 2_000);

/// The card of the temperatures as their round last published it; a text slot per channel, for a role of one's own.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, COUNT> = Published::new();

/// Where a module answers now, by the setting the map named for it.
fn address(setting: &'static Setting) -> u8 {
    nvs_settings::value(setting).parse().unwrap_or(2)
}

/// Whether a socket is given ROLE now - by the roles as they are kept. A role nobody plays reads nothing from the day the board started, and the card of the automaton says no silence of it.
pub fn plays(role: &str) -> bool {
    let now = nvs_settings::now();
    burner_settings::plays(role, &|s| now.value(s))
}

/// Whether the sensors have been asked all round once since the board came up: set by the first round that has gone through (every socket asked, answered or not) and never taken back.
static FIRST_ROUND: AtomicBool = AtomicBool::new(false);

/// Whether the first full round of the sensors is over. **A BOUNDARY OF AN EVENT AND NOT OF A TIME**: until it is, a role that reads nothing may only be a role whose turn has not come, and the demand lights nothing (crates/demand, `Why::Starting`).
pub fn first_round_done() -> bool {
    FIRST_ROUND.load(Ordering::Relaxed)
}

/// Puts the card together and publishes it; the round calls it every round, and the first of them closes the wait of the demand for the first round.
fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
    let read = KEPT.last().unwrap_or(UNREAD);
    let now = nvs_settings::now();
    // every temperature the automaton can hold is the socket set to its role, and a socket that reads nothing is None
    for role in [&crate::roles::BOILER, &crate::roles::RETURN, &crate::roles::ROOM] {
        role.publish(match burner_map::by_role(&MAP, &read, |s| now.value(s), role.name) {
            Some((_, Reading::Is(celsius))) => Some(celsius),
            _ => None,
        });
    }
    FIRST_ROUND.store(true, Ordering::Relaxed);
}

/// The temperatures by address: what the API names, from the last round's readings, the roles as they are kept now and the bus's counts.
fn values(card: &mut dyn Fill) {
    let read = KEPT.last().unwrap_or(UNREAD);
    let now = nvs_settings::now();
    let stored: [String; COUNT] = std::array::from_fn(|n| now.value(MAP[n].role));
    let roles: [&str; COUNT] = std::array::from_fn(|n| stored[n].as_str());
    api::put_values(&read, &roles, &rs485::counts(), &mut |at, said| card.said(at, said));
}

/// How many times the start of the round is tried, and the pause between two tries. **A ROUND THAT DOES NOT START IS A BOARD THAT READS NOTHING**, and a start fails for want of memory for a stack that the next few seconds of
/// the boot may give back: the pause is the period of the round itself (`poll_every`, two seconds by default), one round's worth of waiting, and five tries are ten seconds - longer than the rest of the boot takes to settle,
/// and shorter than any wait a person would take for a hang.
const START_TRIES: u32 = 5;
const START_PAUSE: Duration = Duration::from_secs(2);

/// Starts the round, tried again if it does not start, every failure in the journal. **IF IT NEVER STARTS THERE IS NO ROUND TO WAIT FOR**: the first round is then taken as over, so that the barrier of the thermostat (`Why::Starting`) does
/// not stand for ever in front of a board that reads nothing - which decides by nothing it has read, as a bus that answers nobody does.
pub fn start_retrying() {
    let started = module::retry::start_with_retries("temperatures", START_TRIES, START_PAUSE, start, std::thread::sleep, |line| log::error!("{line}"));
    if !started {
        log::error!("temperatures: no round of the bus will come - the first round is taken as over, and the board reads nothing");
        FIRST_ROUND.store(true, Ordering::Relaxed);
    }
}

/// Starts the round. Call once, after the bus is taken (`hardware::rs485::take`).
pub fn start() -> Result<(), esp_idf_svc::sys::EspError> {
    log::info!("temperatures: {} readings, every {} s", MAP.len(), every().as_secs());
    let answers = module::polling::Answers { curve, lines };
    module::polling::poll("temperatures", firmware::task::STACK, api::CARD, &KEPT, every, Temperatures, publish, &DESK, answers)
}

/// The readings of the map as one sensor: every asking the map builds, one round.
struct Temperatures;

impl Sensor for Temperatures {
    type Reading = Readings;
    type Curves = [curve::Ring<SLOW>; COUNT];
    type Shown = Readings;

    fn read(&mut self) -> Option<Readings> {
        let channels = burner_map::channels(&MAP, address);
        let mut built = [Ask::read(0, 0, 0); MAX_ASKS];
        let made = burner_map::asks(&channels, &mut built);
        let mut read = UNREAD;
        for ask in &built[..made] {
            let (outcome, values) = rs485::ask(*ask);
            if outcome != Outcome::Accepted {
                continue; // whatever it was, it says nothing about these readings: they stay unknown
            }
            for (slot, channel) in read.iter_mut().zip(&channels) {
                match modbus::reading(channel, ask, &values) {
                    Reading::Unknown => {} // another asking covers it
                    said => *slot = said,
                }
            }
        }
        Some(read)
    }

    fn record(curves: &mut Self::Curves, read: &Readings) {
        for (ring, said) in curves.iter_mut().zip(read) {
            if let Reading::Is(celsius) = said {
                ring.push(degrees::point(*celsius));
            }
        }
    }

    /// Every reading of the round as it is: nothing read yet shows the same as a round that read nothing.
    fn shown(read: Option<&Readings>) -> Readings {
        read.copied().unwrap_or(UNREAD)
    }
}

/// The curve called NAME, or None when it is not ours: the degrees of one socket as it held them, one line, with its unit and its points. What the line is called and what it is drawn
/// against are the menu's.
fn curve(name: &str) -> Option<page::Curve> {
    let at = CURVES.iter().position(|curve| *curve == name)?;
    let points = KEPT.curves(|c| c[at].points());
    Some(page::Curve {
        step_ms: every().as_millis() as u32,
        lines: vec![page::Line { words: String::new(), unit: "°C".into(), points: points.into_iter().map(degrees::celsius).collect(), span: None }],
    })
}

/// The shell's `meter` lines: the readings as the map orders them, and the counts of the bus after them - what was read last says nothing of a line that has started to lose answers.
/// For a person at a console, not for the page.
fn lines(line: &mut dyn FnMut(&str)) -> Result<(), String> {
    let read = KEPT.last().unwrap_or(UNREAD);
    let now = nvs_settings::now();
    let counts = rs485::counts();
    let (now_counts, window) = (counts.now, counts.last);
    for (name, state) in burner_map::rows(&MAP, &read, |setting| now.value(setting)) {
        line(&format!("{name} {state}"));
    }
    line(&format!(
        "bus {} read · {} silent · {} broken · {} foreign{}",
        now_counts.accepted,
        now_counts.missed,
        now_counts.broken,
        now_counts.foreign,
        window.map_or(String::new(), |t| format!(" · last window {} silent", t.missed))
    ));
    Ok(())
}

/// The console word `meter`: the lines of every module of the burner that has any (the sensors' readings), in the order of the cards. It takes no word after it.
fn meter_word(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    words.is_empty().then(|| module::lines(super::ALL, line))
}

static VERBS: [module::Verb; 1] = [module::Verb { name: "meter", help: "What the sensors read, one line each", run: meter_word }];

/// The readings of the bus by the modules' contract (crates/module), and the console word `meter` that asks the sensors' modules for their lines.
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

    fn curve(&self, name: &str) -> Option<page::Curve> {
        CURVES.contains(&name).then(|| module::polling::curve(&DESK, name)).flatten()
    }

    fn verbs(&self) -> &'static [module::Verb] {
        &VERBS
    }

    fn lines(&self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        module::polling::lines(&DESK, line)
    }
}
