//! HERE: the demand among the burner's modules: who asks the boiler for heat - nobody, the room against its target, or the schedule against the clock - picked by the setting `auto_asked_by`; the word of the house published by the
//! role `DEMAND` every second; the time of the board the windows follow; and the card that tells a person what is asked and why.
//! NEVER HERE: whether the room is cold or the window open (crates/demand, crates/schedule), what the boiler does with the word (crates/thermostat), a number (crates/settings), a word of the page (page/menu.json), a code of the stream (api/src/demand.rs).
//! CHECKS: crates/demand/tests, crates/thermostat/tests/demand.rs; on the stand tools/module_compare.py and the scenarios of tools/stand_scenarios - the same decisions on the same readings, second by second.
//!
//! **IT RUNS IN THE AUTOMATON'S TASK AND NOT IN A TASK OF ITS OWN**, as the thermostat does: the word of the house is read by the order of the same second, and a task of its own would hand the thermostat the word of a second ago. What
//! is this module's is the state it keeps ([`Demanding`]) and the role it plays (`roles::DEMAND`); the automaton's task calls it before the thermostat orders.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use ::demand::{Asked, Verdict};
use ::schedule::{Profile, Source, Wall};
use burner_api::demand::{self as api, Now};
use between::clock::Moment;
use firmware::nvs_settings;
use module::card::Reads;
use settings::Setting;

use crate::roles;

/// The numbers the players say by, as the card keeps them.
fn tuning() -> ::demand::Tuning {
    let now = nvs_settings::now();
    burner_settings::demand_tuning(&|s: &Setting| now.value(s))
}

/// The local time of the board as the windows follow it: none until a server of time has answered, and then the schedule says nothing and the main profile stands.
fn wall() -> Option<Wall> {
    firmware::clock::now().map(|t| Wall::of_date(t.year, t.month, t.day, t.minute))
}

/// The players of the role and the numbers they say by.
pub struct Demanding {
    player: ::demand::Player,
    tuning: ::demand::Tuning,
}

impl Demanding {
    pub fn new() -> Self {
        Self { player: ::demand::Player::after_boot(), tuning: tuning() }
    }

    /// The numbers it says by from now on: the card's, taken at the next second.
    pub fn take(&mut self) {
        self.tuning = tuning();
    }

    /// What the house says at AT, published by the role `DEMAND` for whoever holds it: **THE ROOM IS READ BY ITS ROLE, AND A ROOM THAT CANNOT SAY IS NONE** - the player says it is silent, and the role never grows old for it.
    pub fn second(&mut self, at: Moment) {
        // the player waits for the first full round of the sensors after the board came up, and is told here, every second, once it is over
        if crate::modules::temperatures::first_round_done() {
            self.player.first_round_done();
        }
        let verdict = self.player.ask(&self.tuning, roles::ROOM.latest_at(at), wall());
        roles::DEMAND.publish(verdict.asked);
        record(&verdict);
    }
}

impl Default for Demanding {
    fn default() -> Self {
        Self::new()
    }
}

/// What the card says, kept by the automaton's task where the card is made from, written by that task alone and read by whoever builds the card.
/// Nought - nobody asks; one more than the place the words make in `Asked::code` otherwise.
static ASKED: AtomicU8 = AtomicU8::new(0);
/// Nought - no profile; one more than the place of the word in `schedule::PROFILES` otherwise.
static PROFILE: AtomicU8 = AtomicU8::new(0);
/// Nought - nothing stands; one more than the place of the kind in `schedule::SOURCES` otherwise.
static SOURCE: AtomicU8 = AtomicU8::new(0);
/// The number of the window in force, from one; nought - none.
static WINDOW: AtomicU8 = AtomicU8::new(0);
/// The minute of the day the window in force ends at; `u32::MAX` - none.
static UNTIL: AtomicU32 = AtomicU32::new(u32::MAX);
/// The target of the room in degrees, as the bits of the number; the bits of NaN - none.
static TARGET: AtomicU32 = AtomicU32::new(0x7fc0_0000);

/// The target of the room as the house holds it now: none outside the source `room` and under a profile that holds no target. The thermostat's manual cycle is carried to it (`crates/thermostat`, `Temperatures::room_target_c`).
pub fn target_c() -> Option<f32> {
    Some(f32::from_bits(TARGET.load(Ordering::Relaxed))).filter(|c| !c.is_nan())
}

fn record(verdict: &Verdict) {
    ASKED.store(verdict.asked.map_or(0, Asked::code), Ordering::Relaxed);
    // a player that is nobody leaves the profile unsaid: the card of a boiler nobody asks for heat does not talk of the clock
    let said = verdict.asked.is_some();
    PROFILE.store(if said { ::schedule::PROFILES.iter().position(|w| *w == ::schedule::profile_word(verdict.profile)).map_or(0, |at| at as u8 + 1) } else { 0 }, Ordering::Relaxed);
    SOURCE.store(if said { ::schedule::SOURCES.iter().position(|w| *w == ::schedule::source_word(verdict.source)).map_or(0, |at| at as u8 + 1) } else { 0 }, Ordering::Relaxed);
    WINDOW.store(if let (true, Source::Window(number)) = (said, verdict.source) { number } else { 0 }, Ordering::Relaxed);
    UNTIL.store(if said { verdict.until.map_or(u32::MAX, u32::from) } else { u32::MAX }, Ordering::Relaxed);
    TARGET.store(verdict.target_c.unwrap_or(f32::NAN).to_bits(), Ordering::Relaxed);
}

/// How long the sensor of the room has been silent now, for the card: the seconds - nothing while it speaks, and nothing when no socket is given that role: it is not silent, it is not there (`Silent::said`).
fn silent_room() -> Option<u32> {
    let at = between::clock::now();
    burner_api::thermostat::Silent::said(crate::modules::thermostat::silent_for(roles::ROOM.quiet_at(at)), crate::modules::temperatures::plays(roles::ROOM.name))
}

/// The card by address: what the house says and why, the profile that stands with what chose it, the target of the room, and how long the room's sensor has been silent.
fn values(put: &mut dyn module::card::Fill) {
    let asked = Asked::of_code(ASKED.load(Ordering::Relaxed));
    let profile = match PROFILE.load(Ordering::Relaxed) {
        0 => None,
        at => ::schedule::PROFILES.get(at as usize - 1).map(|word| match *word {
            "comfort" => Profile::Comfort,
            "eco" => Profile::Eco,
            _ => Profile::Off,
        }),
    };
    let source = match (SOURCE.load(Ordering::Relaxed), WINDOW.load(Ordering::Relaxed)) {
        (0, _) => None,
        (at, window) => ::schedule::SOURCES.get(at as usize - 1).map(|word| match *word {
            "main" => Source::Main,
            "window" => Source::Window(window),
            _ => Source::NoClock,
        }),
    };
    let until_min = Some(UNTIL.load(Ordering::Relaxed)).filter(|minute| *minute != u32::MAX).map(|minute| minute as u16);
    let target_c = Some(f32::from_bits(TARGET.load(Ordering::Relaxed))).filter(|c| !c.is_nan());
    api::put_values(&Now { asked, silent_room: silent_room(), profile, source, until_min, target_c }, &mut |at, value| put.said(at, page::dictionary::Said::Value(value)));
}

/// The demand's card as it last published it.
static PUBLISHED: module::card::Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = module::card::Published::new();

/// Puts the card together and publishes it, and tells an open page: every second of the automaton's task.
pub fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
    web::announce(api::CARD);
}

/// The demand by the modules' contract (crates/module): its card and its eleven settings, and no buttons - a person asks the boiler by Start and Stop, which are the thermostat's.
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
}
