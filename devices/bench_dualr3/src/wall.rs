//! The relays and the wall switches of the Sonoff Dual R3: relay1 is L1 on the box, switch1 is S1, and each switch toggles its own relay. The pins are the device's
//! (Cargo.toml, [package.metadata.rust_app.pins]), checked against the board's board.yml by the build.
//!
//! One owner switches the relays (crates/relay): a switch, the page and the console ask it through its queue, and after a switch the owner publishes the relays' card and announces
//! it, so an open page draws it at once. A switch waits for an edge of its pin, lets the contact settle and reads the level once (crates/switch); an edge missed while it reads is
//! caught by the next look, one second later at the most. What the relays and the keys say outward - their entities, their addresses - is `bench_dualr3_api`.

use std::time::Duration;

use between::block_on;
use bench_dualr3_api::{relays as api_relays, switches as api_switches};
use esp_idf_svc::hal::gpio::{AnyInputPin, PinDriver, Pull};
use esp_idf_svc::sys::EspError;
use module::card::{Fill, Published, Reads};
use relay::Relays;
use switch::{Movement, Switch};

const RELAYS_N: usize = 2;
static RELAY_NAMES: [&str; RELAYS_N] = api_relays::NAMES;
const CLOSED_ON_LOW: bool = matches!(env!("STRAGA_RELAYS_CLOSE_ON_LOW").as_bytes(), b"1");
const RELAY_PINS: [&str; RELAYS_N] = [env!("STRAGA_PIN_RELAY1"), env!("STRAGA_PIN_RELAY2")];
const SWITCH_NAMES: [&str; 2] = api_switches::NAMES;
const SWITCH_PINS: [&str; 2] = [env!("STRAGA_PIN_SWITCH1"), env!("STRAGA_PIN_SWITCH2")];
/// Which relay each switch toggles, by index: switch1 - relay1, switch2 - relay2.
const WALL: [usize; 2] = [0, 1];
/// How long a look at a switch waits for an edge before it reads the level anyway.
const LOOK: Duration = Duration::from_secs(1);

static RELAYS: Relays<RELAYS_N> = Relays::new();
static SWITCHES: [Switch; 2] = [const { Switch::new(switch::Mode::Pushbutton) }; 2];

/// The card of the relays as their owner last published it.
static RELAYS_CARD: Published<{ page::dictionary::addresses(&bench_dualr3_api::TABLE, api_relays::CARD) }, 0> = Published::new();
/// The card of the keys as their last movement published it.
static KEYS_CARD: Published<{ page::dictionary::addresses(&bench_dualr3_api::TABLE, api_switches::CARD) }, 0> = Published::new();

fn pin(number: &str) -> u8 {
    number.parse().expect("a pin number: tools/board_env.py checks every pin of the device")
}

/// Puts the card of the relays together and publishes it.
fn publish_relays() {
    firmware::publish(&RELAYS_CARD, api_relays::CARD, |card| {
        api_relays::put_values(std::array::from_fn(|n| RELAYS.closed(n)), &mut |at, value| card.put(at, value));
    });
}

/// Puts the card of the keys together and publishes it.
fn publish_keys() {
    firmware::publish(&KEYS_CARD, api_switches::CARD, |card| {
        let keys = std::array::from_fn(|n| api_switches::Key { closed: SWITCHES[n].closed(), toggles: SWITCHES[n].toggles() });
        api_switches::put_values(&keys, &mut |at, value| card.put(at, value));
    });
}

/// What the relays' owner calls after every change: the card published, an open page told.
fn relays_changed() {
    publish_relays();
    web::announce(api_relays::CARD);
}

/// Opens the relays, starts their owner and the switches. Call once; a pin that cannot be set up stops it.
pub fn start() -> Result<(), EspError> {
    publish_relays();
    publish_keys();
    relay::board::start(&RELAYS, &RELAY_NAMES, RELAY_PINS.map(pin), CLOSED_ON_LOW, |_| 0, relays_changed)?;

    for (i, number) in SWITCH_PINS.iter().enumerate() {
        let mut input = PinDriver::input(unsafe { AnyInputPin::steal(pin(number)) }, Pull::Up)?;
        SWITCHES[i].start(input.is_low());
        publish_keys();
        firmware::task::round(SWITCH_NAMES[i], 4096, firmware::task::Core::Hardware, move || loop {
            let _ = block_on(embassy_time::with_timeout(
                embassy_time::Duration::from_millis(LOOK.as_millis() as u64),
                input.wait_for_any_edge(),
            ));
            std::thread::sleep(switch::DEBOUNCE);
            let closed = input.is_low();
            let movement = SWITCHES[i].settled(closed);
            if movement != Movement::None {
                log::info!("{} {}", SWITCH_NAMES[i], if closed { "closed" } else { "open" });
            }
            match movement {
                Movement::None => continue,
                Movement::Moved => {}
                Movement::Toggle => {
                    if let Err(why) = RELAYS.toggle(WALL[i], relay::board::WITHIN) {
                        log::warn!("{}: {why}", SWITCH_NAMES[i]);
                    }
                }
            }
            publish_keys();
            web::announce(api_switches::CARD);
        })?;
    }
    Ok(())
}

/// Asks the owner to carry out ORDER on the relay NAME and waits for its answer, so what is read next is the new state.
fn order(name: &str, order: module::Order) -> Result<(), String> {
    let relay = RELAY_NAMES
        .iter()
        .position(|n| *n == name)
        .ok_or_else(|| format!("this board switches no {name}; it switches: {}", RELAY_NAMES.join(", ")))?;
    let done = match order {
        module::Order::Switch(on) => RELAYS.set(relay, on, relay::board::WITHIN),
        module::Order::Toggle => RELAYS.toggle(relay, relay::board::WITHIN),
        other => return Err(format!("{name} is switched on and off, not {other:?}")),
    };
    done.map_err(|why| why.to_string())
}

/// The relays by the modules' contract (crates/module): their card, and the commands a person gives them.
pub struct RelaysModule;

impl module::Module for RelaysModule {
    fn card(&self) -> &'static str {
        api_relays::CARD
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&RELAYS_CARD)
    }

    fn takes(&self) -> &'static [&'static str] {
        &["toggle", "on", "off"]
    }

    fn is_on(&self, name: &str) -> Option<bool> {
        RELAY_NAMES.iter().position(|n| *n == name).map(|relay| RELAYS.closed(relay))
    }

    fn order(&self, name: &str, order: module::Order) -> Option<Result<(), String>> {
        RELAY_NAMES.contains(&name).then(|| self::order(name, order))
    }
}

/// The wall switches by the modules' contract: their card, read and never commanded.
pub struct KeysModule;

impl module::Module for KeysModule {
    fn card(&self) -> &'static str {
        api_switches::CARD
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&KEYS_CARD)
    }
}
