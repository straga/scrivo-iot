//! HERE: the relays: the organs of src/hardware/relays.rs - the auger, the igniter, the pump and the power of the modules - turned on and off by hand from the page, the web and the console, their
//! limits of time, the card they publish, their place in the module list. The auger and the igniter carry a limit - a forgotten button neither keeps the igniter hot nor pours pellets: the auger's
//! is a setting of crates/settings (`auger_limit`), the igniter's follows from the automaton's time of the igniter and is no setting (`burner_settings::igniter_limit_s`); the pump and the power stay as they
//! are told.
//! NEVER HERE: a pin or the level that closes a relay (the hardware, burner.yaml), when a limit opens an organ (crates/relay), a word of the page (page/menu.json), a code of the stream
//! (api/src/relays.rs). What this file owns is the ORDER: carry out, keep, publish.
//! CHECKS: devices/burner/api/tests/relays_values.rs, crates/relay/tests/relay.rs.

use burner_api::relays::{self as api, Organ, NAMES};
use firmware::nvs_settings;
use module::card::{Fill, Published, Reads};
use relay::Relays;
use settings::Setting;

/// The relays, owned by the round `hardware::relays::start` makes.
static RELAYS: Relays<4> = Relays::new();

/// The card of the relays as their owner last published it: every organ and the seconds left of a limit running.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

/// Puts the card together and publishes it; the relays' owner calls it on every change and every second of a limit.
fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
}

/// How long relay RELAY may stay closed before it opens by itself, in ms; 0 - no limit. The igniter's follows from the kindling as the automaton's settings give it, so the hand's "On" and the
/// automaton's are cut by one number and nobody can set the relay shorter than the kindling it serves.
fn limit_ms(relay: usize) -> u32 {
    let now = nvs_settings::now();
    let seconds = match NAMES[relay] {
        "auger" => now.value(&burner_settings::AUGER_LIMIT).parse::<u32>().unwrap_or(0),
        "igniter" => burner_settings::igniter_limit_s(&|s: &Setting| now.value(s)),
        _ => 0,
    };
    seconds.saturating_mul(1000)
}

/// The relays that really changed since the last taking, as (relay, closed) in order, and how many more changed than the notes hold: what the mains' journal line tells.
pub fn take_switches(each: &mut impl FnMut(usize, bool)) -> u32 {
    RELAYS.take_switches(each)
}

/// The word of relay RELAY, or "" - there is no such relay.
pub fn name(relay: usize) -> &'static str {
    NAMES.get(relay).copied().unwrap_or("")
}

/// Opens the relays - every organ off - and starts their owner. Call once, before the network.
pub fn start() -> Result<(), esp_idf_svc::sys::EspError> {
    publish();
    crate::hardware::relays::start(&RELAYS, &NAMES, limit_ms, || {
        publish();
        // the seconds the auger turns are the pellets' to count: told at every change and every second of a limit
        crate::modules::pellets::observe(auger_closed());
        web::announce(api::CARD);
    })
}

/// The organs by address: what the API names, from the relays and the seconds their limits leave.
fn values(card: &mut dyn Fill) {
    let organs: [Organ; 4] = std::array::from_fn(|n| Organ { closed: RELAYS.closed(n), left_s: relay::board::left_s(&RELAYS, n) });
    api::put_values(&organs, &mut |at, value| card.put(at, value));
}

fn find(name: &str) -> Result<usize, String> {
    NAMES.iter().position(|n| *n == name).ok_or_else(|| format!("the burner has no {name}; it has: {}", NAMES.join(", ")))
}

/// Carries out ORDER on the organ NAME - from the page, the web or the console - and waits for the relays' owner to answer, so what is read next is the new state.
fn order(name: &str, order: module::Order) -> Result<(), String> {
    let relay = find(name)?;
    let done = match order {
        module::Order::Switch(on) => RELAYS.set(relay, on, relay::board::WITHIN),
        module::Order::Toggle => RELAYS.toggle(relay, relay::board::WITHIN),
        other => return Err(format!("{name} is switched on and off, not {other:?}")),
    };
    done.map_err(|why| why.to_string())
}

/// The automaton's hand on the relay playing ROLE: on or off, and the relays' owner told at once.
fn command(role: &str, what: &dyn std::any::Any) -> Option<Result<(), String>> {
    // THE AUGER IS TOLD A RHYTHM, and its owner times the portions; the igniter and the pump are told on and off
    if role == crate::roles::AUGER.name {
        let at = find("auger").ok()?;
        return Some(module::take::<relay::Told>(what, role).and_then(|told| relay::board::tell(&RELAYS, at, told).map_err(|why| why.to_string())));
    }
    let played = [(&crate::roles::IGNITER, "igniter"), (&crate::roles::PUMP, "pump")];
    let (_, name) = played.iter().find(|(r, _)| r.name == role)?;
    Some(module::take::<bool>(what, role).and_then(|on| order(name, module::Order::Switch(on))))
}

/// Whether the auger's relay is closed now, whoever closed it.
pub fn auger_closed() -> bool {
    NAMES.iter().position(|n| *n == "auger").is_some_and(|at| RELAYS.closed(at))
}

/// Whether the igniter's relay holds the plug on now, whoever closed it.
pub fn igniter_closed() -> bool {
    NAMES.iter().position(|n| *n == "igniter").is_some_and(|at| RELAYS.closed(at))
}

/// The relays by the modules' contract (crates/module).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn takes(&self) -> &'static [&'static str] {
        &["toggle", "on", "off"]
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn is_on(&self, name: &str) -> Option<bool> {
        find(name).ok().map(|relay| RELAYS.closed(relay))
    }

    fn settings(&self) -> Vec<&'static Setting> {
        api::SETTINGS.to_vec()
    }

    fn order(&self, name: &str, order: module::Order) -> Option<Result<(), String>> {
        NAMES.contains(&name).then(|| self::order(name, order))
    }

    fn command(&self, role: &str, what: &dyn std::any::Any) -> Option<Result<(), String>> {
        command(role, what)
    }
}
