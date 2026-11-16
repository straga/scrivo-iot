//! HERE: the mains the burner is plugged into, read from the Tasmota socket over the network: the address and the period from the settings, one read of the socket's status every period, the card of power, voltage and current it
//! publishes (or the reason there is none), the role `mains_power`, and the line of the journal that tells an organ's switch with the power before and after it.
//! NEVER HERE: how the socket is asked and its answer taken apart (crates/tasmota), which switches to tell and the words of the line (crates/mains), a word of the page (page/menu.json), a code of the stream (api/src/mains.rs).
//! What this file owns is the ORDER: read, publish, tell.
//! **THE SOCKET IS ONLY READ**: it powers the whole burner, and the one request this module can make is crates/tasmota's `REQUEST`, a read of its sensor status.
//! **NO TASK OF ITS OWN**: the read is a future of the page's loop (`web::outside`), which waits on the loop's reactor and timers and never blocks - a socket that does not answer holds the read for its limit and the page for nothing.
//! CHECKS: crates/tasmota/tests, crates/mains/tests, devices/burner/api/tests/mains_table.rs, crates/web/tests/outside.rs.

use std::time::Duration;

use burner_api::mains as api;
use embassy_time::Timer;
use firmware::nvs_settings;
use mains::Watch;
use module::card::{Fill, Published, Reads};
use settings::Setting;
use tasmota::{Energy, Fault};

/// The longest a read waits for the socket, from the connection to the close of the answer. The socket answers in 35 to 60 ms from the computer and in about 500 ms to this board [probed: 03.11, `.77`].
const WITHIN: Duration = Duration::from_millis(1500);

/// The room for an answer: 387 bytes on the live socket, headers and body.
const BUFFER: usize = 512;

/// The card of the mains as the read last published it.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

/// How often the socket is asked, from the setting; the role of the power grows old by it.
pub fn every() -> Duration {
    Duration::from_secs(nvs_settings::value(&burner_settings::MAINS_EVERY).parse::<u64>().unwrap_or(5).max(1))
}

/// What the card says: the reading, or the reason there is none.
type Said = Result<(f32, f32, f32), &'static str>;

fn said_of(result: &Result<Energy, Fault>) -> Said {
    match result {
        Ok(e) => Ok((e.watts, e.volts, e.amps)),
        Err(fault) => Err(fault.reason()),
    }
}

/// Puts the card together and publishes it, and the role with it. The page is told when what the card shows has changed.
fn publish(said: Said, was: &mut Option<Said>) {
    firmware::publish(&PUBLISHED, api::CARD, |card| api::put_values(said, &mut |at, value| card.put(at, value)));
    crate::roles::MAINS_POWER.publish(said.ok().map(|(watts, _, _)| watts));
    if *was != Some(said) {
        *was = Some(said);
        web::announce(api::CARD);
    }
}

/// Leaves the read to the page's loop, which runs it once it starts. Call before `firmware::run`.
pub fn start() {
    publish(Err(api::NOT_READ), &mut None);
    if web::outside::add(errand).is_err() {
        log::error!("mains: the page's loop is running already - the socket is not read");
    }
}

fn errand() -> web::outside::Errand {
    web::outside::boxed(run())
}

/// The read, for good: every period it asks the socket the setting names, publishes what it said, and tells the journal about the switches of the relays since the last round.
async fn run() {
    let net = edge_nal_std::Stack::new();
    let mut buf = vec![0u8; BUFFER];
    let mut watch = Watch::new();
    let mut was = None;
    let mut fault: Option<Fault> = None;
    loop {
        let at = mains::address(&nvs_settings::value(&burner_settings::MAINS_AT));
        match at {
            // no socket is set: nothing is asked, and the notes of the relays are let go so that they do not wait for a socket that is not there
            None => {
                publish(Err(api::OFF), &mut was);
                fault = None;
                crate::modules::relays::take_switches(&mut |_, _| {});
            }
            Some(at) => {
                let result = tasmota::read(&net, at, &mut buf, WITHIN).await;
                match (&result, fault) {
                    (Err(f), was_fault) if was_fault != Some(*f) => log::warn!("mains: the socket at {at} gives no reading: {f}"),
                    (Ok(_), Some(_)) => log::info!("mains: the socket at {at} answers again"),
                    _ => {}
                }
                fault = result.as_ref().err().copied();
                publish(said_of(&result), &mut was);
                let mut switches = Vec::new();
                let dropped = crate::modules::relays::take_switches(&mut |relay, closed| switches.push((relay, closed)));
                if let Some(line) = watch.round(&result, &switches, dropped, crate::modules::relays::name) {
                    log::info!("{line}");
                }
            }
        }
        Timer::after(embassy_time::Duration::from_micros(every().as_micros() as u64)).await;
    }
}

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
