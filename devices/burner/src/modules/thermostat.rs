//! HERE: the thermostat - the water layer, which the page calls the boiler - among the burner's modules: the temperatures by role and the word of the house by the role `DEMAND`, the numbers of its settings through its door, the order published by the role `HEAT` every second and the moment a button is pressed, and the buttons' count.
//! NEVER HERE: whether the water is high or low (crates/thermostat), who says the house is cold or warm (crates/demand, modules/demand.rs), what the fire does with an order (crates/automaton), a number (crates/settings), a word of the page (page/menu.json), a code of the stream (api/src/automaton.rs).
//! CHECKS: crates/thermostat/tests, crates/automaton/tests; on the stand tools/module_compare.py --modules automaton - the same decisions on the same readings, second by second.
//!
//! **IT RUNS IN THE AUTOMATON'S TASK AND NOT IN A TASK OF ITS OWN**: the order is a decision taken once a second on the readings of that second, and a press is judged and applied the moment it comes (crates/automaton, `Automaton`), so the
//! two have to happen in one breath - a task of its own would read the readings a round late, hand a press over a lock to a fire that is in the middle of its step, and cost a stack of the board's small heap. What is the thermostat's is the
//! state it keeps ([`Heat`]) and the role it plays (`roles::HEAT`); the automaton's task only calls it at the places the order has to be made, and holds the role like any consumer.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use ::automaton::{Heard, Order, Refused, Switch, Water};
use ::thermostat::{Press, Rung, Settled, Temperatures, Thermostat};
use burner_api::thermostat::{self as api, Now, LIGHT, START, STOP};
use between::clock::Moment;
use firmware::nvs_settings;
use module::card::{Fill, Reads};
use settings::Setting;

use crate::roles;

/// A role's silence as the thermostat and the automaton read it: whether it speaks, and if not whether for long enough to stop a burner.
pub fn heard(quiet: module::Quiet) -> Heard {
    match quiet {
        module::Quiet::Speaks => Heard::Speaks,
        module::Quiet::Silent(_) => Heard::Silent,
        module::Quiet::TooLong(_) => Heard::TooLong,
    }
}

/// The seconds a role has been silent, as a card says them: past three periods of its round, when its reading is no longer given out; None while it speaks.
pub fn silent_for(quiet: module::Quiet) -> Option<u32> {
    match quiet {
        module::Quiet::Speaks => None,
        module::Quiet::Silent(span) | module::Quiet::TooLong(span) => Some(span.as_secs() as u32),
    }
}

/// How long the sensor the thermostat holds by has been silent now, for its card: the seconds - nothing while it speaks, and nothing when no socket is given that role: it is not silent, it is not
/// there (`Silent::said`, the rule the stand takes from the same place). The card of a sensor whose round has stopped is published by nobody, so the thermostat says it.
fn silent_now() -> api::Silent {
    let at = between::clock::now();
    api::Silent { return_c: api::Silent::said(silent_for(roles::RETURN.quiet_at(at)), crate::modules::temperatures::plays(roles::RETURN.name)) }
}

/// The thermostat's numbers as the card keeps them, and the overheating the fire holds, which the thermostat's door holds the setpoint against.
fn numbers() -> (::thermostat::Tuning, f32) {
    let now = nvs_settings::now();
    let value = |s: &Setting| now.value(s);
    (burner_settings::thermostat_tuning(&value), value(&burner_settings::AUTO_TOO_HOT_C).parse::<f32>().unwrap_or(f32::NAN))
}

/// The thermostat's numbers through its door, or the door's refusal.
pub fn door() -> Result<::thermostat::Sound, Refused> {
    let (tuning, too_hot_c) = numbers();
    tuning.sound(too_hot_c)
}

/// What the readings say at AT, as the thermostat is given them: the two temperatures by their roles, how the one it holds only on request is heard, and the word of the house by its role. **NONE IS NEVER A ZERO**: a role that cannot say gives none.
fn temperatures(at: Moment, fire: ::thermostat::Fire) -> Temperatures {
    Temperatures {
        boiler_c: roles::BOILER.latest_at(at),
        return_c: roles::RETURN.latest_at(at),
        return_heard: heard(roles::RETURN.quiet_at(at)),
        asked: roles::DEMAND.latest_at(at),
        // THE ROOM AND ITS TARGET are what a manual cycle is carried to where the room is the source of the demand: the room by its role, the target as the house holds it this second
        room_c: roles::ROOM.latest_at(at),
        room_target_c: crate::modules::demand::target_c(),
        fire,
    }
}

/// The switch the board's flash holds, as the board comes up: the last press of Start or Stop (a Light is no word of the switch), and `Unasked` where nothing is kept or the word is not one a press writes. Said in the journal either way, so a restart that
/// lit a boiler or left it stopped has its reason written down.
pub fn recalled() -> Switch {
    let word = nvs_settings::recall(::thermostat::SWITCH_CARD);
    let switch = ::thermostat::kept_switch(word.as_deref());
    log::info!("thermostat: came up finding the switch {} ({})", ::automaton::switch_word(switch), word.as_deref().unwrap_or("no key"));
    switch
}

/// What the journal says of a try at writing the switch down: a series of refusals once, at its first, and the recovery once with the number it ended; the tries between are silent.
fn told(settled: Settled) {
    match settled {
        Settled::Refused { first: true, why } => log::error!("thermostat: the switch was not written down: {why} - tried again every second until the flash takes it"),
        Settled::Written { after_refusals } if after_refusals > 0 => log::info!("thermostat: the switch is written down after {after_refusals} refusals"),
        Settled::Nothing | Settled::Written { .. } | Settled::Refused { .. } => {}
    }
}

/// The thermostat a board runs: what a person pressed, the numbers it decides by, and what it has been given.
pub struct Heat {
    thermostat: Thermostat,
    sound: ::thermostat::Sound,
    /// When the thermostat last looked, for the seconds each look stands for (the wait before a light is counted by them).
    looked: Option<Moment>,
}

impl Heat {
    /// A thermostat deciding by SOUND, with the switch the board found in its flash: the person's last word, and no press of its own (`Unasked` where there is none).
    pub fn new(sound: ::thermostat::Sound, switch: Switch) -> Self {
        Self { thermostat: Thermostat::restored(switch).waiting_for_first_round(), sound, looked: None }
    }

    /// The numbers it decides by from now on: the card's, taken at the next second.
    pub fn take(&mut self, sound: ::thermostat::Sound) {
        self.sound = sound;
    }

    /// The level the last order carried the fire up on; None before the first order.
    pub fn rung(&self) -> Option<Rung> {
        self.thermostat.rung()
    }

    /// Whether a person asked for heat, as the card shows it.
    pub fn wanted(&self) -> bool {
        self.thermostat.switch() == Switch::On
    }

    /// The order of the second AT, published by the role `HEAT` for whoever holds it.
    ///
    /// **OUT is whether the automaton stands in Stop**: the thermostat is told it each look and finds the end of burning in it - the look that says out after one that said not -, and counts the wait before a light from there.
    /// **STEADY is whether the flame sensor sees a steady flame this second**: the wait begins only at the end of a fire the sensor saw, `sure_s` of it in a row (`thermostat::Lockout::sure_s`), and not at the end of whatever the automaton did
    /// before it stopped.
    pub fn second(&mut self, at: Moment, out: bool, steady: bool) -> Order {
        let elapsed_s = self.elapsed_at(at);
        // the thermostat orders no light from a stop until the sensors have been asked all round once since the board came up, whoever plays the demand: told here, every second, once it is over
        if crate::modules::temperatures::first_round_done() {
            self.thermostat.first_round_done();
        }
        // A WORD THE FLASH REFUSED IS TRIED AGAIN ON THE SECONDS THAT FOLLOW (6.16, the review), once a second and until it is taken; a board whose flash took its word touches nothing here
        told(self.thermostat.retry(elapsed_s, &mut |word| nvs_settings::remember(::thermostat::SWITCH_CARD, word)));
        let fire = ::thermostat::Fire { out, steady, elapsed_s };
        let order = self.thermostat.order(&temperatures(at, fire), &self.sound);
        roles::HEAT.publish(Some(order));
        record(order.switch, self.thermostat.from_memory(), order.water, self.thermostat.rung(), self.thermostat.left_c());
        // a manual cycle that ended in this second says how, once, in the journal; the card says what is going on
        if let Some(ended) = self.thermostat.ended() {
            log::info!("thermostat: {}", ended.line());
        }
        record_cycle(self.thermostat.cycle());
        SETPOINT.store(self.thermostat.setpoint_c().unwrap_or(f32::NAN).to_bits(), Ordering::Relaxed);
        LOCK.store(self.thermostat.lock_left_s().unwrap_or(f32::NAN).to_bits(), Ordering::Relaxed);
        order
    }

    /// Whether the automaton stands in a fault, said every second: a manual cycle does not go on over a fault, which waits for a person.
    pub fn faulted(&mut self, faulted: bool) {
        self.thermostat.faulted(faulted);
    }

    /// A press of a button, counted now and published in the order of this very moment: the fire takes it at once, in the breath it is pressed in.
    ///
    /// A Light that comes while the wait before a light is running ends it, and the journal says what was left: the person's word is the one thing that lights before the time. **A Light is judged against the readings of this moment**: with the nominal it would carry
    /// the fire to reached already it is refused with the numbers (`Refusal::wire`), nothing is counted and the fire is not asked; otherwise it begins a cycle to the nominal, which the journal says.
    pub fn press(&mut self, press: Press, at: Moment, out: bool, steady: bool) -> Result<Order, ::thermostat::Refusal> {
        if press == Press::Light {
            let fire = ::thermostat::Fire { out, steady, elapsed_s: 0.0 };
            match self.thermostat.light(&temperatures(at, fire), &self.sound) {
                Ok(cut) => {
                    if let Some(left) = cut {
                        log::info!("thermostat: a Light with {} s of the wait before a light left: lighting now", left.ceil() as u32);
                    }
                    if let Some(cycle) = self.thermostat.cycle() {
                        log::info!("thermostat: {}", cycle.began_line());
                    }
                }
                Err(refusal) => {
                    log::info!("thermostat: {}", refusal.line());
                    return Err(refusal);
                }
            }
        } else {
            let _ = self.thermostat.press(press);
        }
        // THE SWITCH IS WRITTEN DOWN ON THE PRESS: one try for a press that changed what the flash holds, so a restart finds the person's last word and the flash is worn once for each; a refusal is the seconds' to try again
        told(self.thermostat.write_now(&mut |word| nvs_settings::remember(::thermostat::SWITCH_CARD, word)));
        Ok(self.second(at, out, steady))
    }

    /// The seconds since the last look, and AT is the new last one: nought for a look at the very moment of the last.
    fn elapsed_at(&mut self, at: Moment) -> f32 {
        let elapsed = self.looked.map_or(0.0, |before| at.since(before).as_secs_f32());
        self.looked = Some(at);
        elapsed
    }
}

/// What the card says, kept by the automaton's task where the card is made from: the switch, the last word of the water and the level and the distance of the last order, as the small numbers [`Switch`], [`Water`] and [`Rung`] go by. Written by that task alone, read by whoever builds the card.
static SWITCH: AtomicU8 = AtomicU8::new(0);
/// Nought - the switch is a person's press of this time or nobody's; one - it stands as the memory gave it after a restart.
static FROM_MEMORY: AtomicU8 = AtomicU8::new(0);
/// Nought - no order yet; one more than the place of the word in `automaton::WATERS` otherwise.
static WATER: AtomicU8 = AtomicU8::new(0);
/// Nought - no level; one more than the place of the word in `thermostat::RUNGS` otherwise.
static RUNG: AtomicU8 = AtomicU8::new(0);
/// The distance of the last order in degrees, as the bits of the number; the bits of NaN - none.
static LEFT: AtomicU32 = AtomicU32::new(0x7fc0_0000);

/// Nought - no manual cycle; one more than the place of the kind in `thermostat::NOMINALS` otherwise.
static MANUAL: AtomicU8 = AtomicU8::new(0);
/// The number of degrees the manual cycle ends at, as the bits of the number; the bits of NaN - none.
static MANUAL_TO: AtomicU32 = AtomicU32::new(0x7fc0_0000);

fn record_cycle(cycle: Option<::thermostat::Cycle>) {
    MANUAL.store(cycle.map_or(0, |cycle| cycle.nominal as u8 + 1), Ordering::Relaxed);
    MANUAL_TO.store(cycle.map_or(f32::NAN, |cycle| cycle.to_c).to_bits(), Ordering::Relaxed);
}

/// The setpoint of the water of the last order in degrees, as the bits of the number; the bits of NaN - none.
static SETPOINT: AtomicU32 = AtomicU32::new(0x7fc0_0000);
/// The seconds left of the wait before a light, as the bits of the number; the bits of NaN - none.
static LOCK: AtomicU32 = AtomicU32::new(0x7fc0_0000);

fn record(switch: Switch, from_memory: bool, water: Water, rung: Option<Rung>, left_c: Option<f32>) {
    FROM_MEMORY.store(u8::from(from_memory), Ordering::Relaxed);
    let at = ::automaton::SWITCHES.iter().position(|w| *w == ::automaton::switch_word(switch)).unwrap_or(0);
    SWITCH.store(at as u8, Ordering::Relaxed);
    let word = ::automaton::WATERS.iter().position(|w| *w == ::automaton::water_word(water)).map_or(0, |at| at + 1);
    WATER.store(word as u8, Ordering::Relaxed);
    let level = rung.map_or(0, |rung| ::thermostat::RUNGS.iter().position(|w| *w == ::thermostat::rung_word(rung)).map_or(0, |at| at + 1));
    RUNG.store(level as u8, Ordering::Relaxed);
    LEFT.store(left_c.unwrap_or(f32::NAN).to_bits(), Ordering::Relaxed);
}

/// The card by address: what a person asked, what the water said, and the level and the distance in the last order.
fn values(put: &mut dyn Fill) {
    let switch = match SWITCH.load(Ordering::Relaxed) {
        1 => Switch::On,
        2 => Switch::Off,
        _ => Switch::Unasked,
    };
    let water = match WATER.load(Ordering::Relaxed) {
        0 => None,
        at => ::automaton::WATERS.get(at as usize - 1).map(|word| match *word {
            "burn" => Water::Burn,
            "keep" => Water::Keep,
            "enough" => Water::Enough,
            "level" => Water::Level,
            _ => Water::Silent(::automaton::Held::Return),
        }),
    };
    let rung = match RUNG.load(Ordering::Relaxed) {
        0 => None,
        at => ::thermostat::RUNGS.get(at as usize - 1).map(|word| match *word {
            "big" => Rung::Big,
            "mid" => Rung::Mid,
            _ => Rung::Small,
        }),
    };
    let left_c = Some(f32::from_bits(LEFT.load(Ordering::Relaxed))).filter(|left| !left.is_nan());
    let setpoint_c = Some(f32::from_bits(SETPOINT.load(Ordering::Relaxed))).filter(|c| !c.is_nan());
    let lock_left_s = Some(f32::from_bits(LOCK.load(Ordering::Relaxed))).filter(|left| !left.is_nan());
    let manual = match MANUAL.load(Ordering::Relaxed) {
        0 => None,
        at => Some(::thermostat::Cycle { nominal: if at == 1 { ::thermostat::Nominal::Room } else { ::thermostat::Nominal::Water }, to_c: f32::from_bits(MANUAL_TO.load(Ordering::Relaxed)) }),
    };
    api::put_values(&Now { switch, from_memory: FROM_MEMORY.load(Ordering::Relaxed) == 1, water, rung, left_c, setpoint_c, lock_left_s, silent: silent_now(), manual }, &mut |at, value| put.said(at, page::dictionary::Said::Value(value)));
}

/// The thermostat's card as it last published it.
static PUBLISHED: module::card::Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = module::card::Published::new();

/// Puts the card together and publishes it, and tells an open page: every second of the automaton's task and every press.
pub fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
    web::announce(api::CARD);
}

/// A press of Start, Stop or Light by name.
fn pressed(name: &str) -> Result<Press, String> {
    match name {
        START => Ok(Press::Start),
        STOP => Ok(Press::Stop),
        LIGHT => Ok(Press::Light),
        other => Err(format!("the thermostat has no button {other}; it has {START}, {STOP} and {LIGHT}")),
    }
}

/// The thermostat by the modules' contract (crates/module): its card, its four settings, its three buttons.
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn takes(&self) -> &'static [&'static str] {
        &["press"]
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn settings(&self) -> Vec<&'static Setting> {
        api::SETTINGS.to_vec()
    }

    /// Its three buttons: the main switch over everything the thermostat will come to ask for, and the light, the fire lit now past the thermostat. The press goes to the automaton's task, where the thermostat runs: it is counted and carried out in one breath.
    fn order(&self, name: &str, order: module::Order) -> Option<Result<(), String>> {
        match (name, order) {
            (START | STOP | LIGHT, module::Order::Press) => Some(pressed(name).and_then(crate::modules::automaton::press)),
            (START | STOP | LIGHT, other) => Some(Err(format!("{name} does not take {other:?}"))),
            _ => None,
        }
    }
}
