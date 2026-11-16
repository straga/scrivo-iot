//! The relays of a device on its pins, and the one owner that switches them: what every device with relays runs, whatever hangs
//! on them. The half of this crate that needs the chip; on a computer only the logic above is built. The pins are opened without a moment of a closed relay - the open level first,
//! the output after (relay::open_pins) -, every relay starts open, and the owner carries out the commands and opens a
//! relay whose limit has run out. While a limit runs the owner announces once a second, so an open page counts down.

use std::time::Duration;

use esp_idf_svc::hal::gpio::{AnyOutputPin, Output, PinDriver};
use esp_idf_svc::sys::{self, EspError};
use crate::{Relays, HORIZON_MS};

/// How often an open page hears of a relay whose limit runs.
const TICK_MS: u32 = 1000;

/// How long a person, the automaton or a switch waits for the relays' owner to answer a command. The owner answers
/// in a write of a pin; a second is an owner that stands.
pub const WITHIN: Duration = Duration::from_secs(1);

/// The board's time for the relays: ms of its steady clock, in the 32 bits crates/relay counts in. The chip's own
/// clock leapt three hours forward under fire (26.09), and a limit counted by it lets the igniter and the auger go at
/// once; by the steady clock a leap moves a limit by a slice of the time driver's beat at most (between::clock::PAD).
pub fn now_ms() -> u32 {
    between::clock::now().as_millis() as u32
}

/// The owner's stack. Measured on `.30` under the page's and the console's commands: 708 bytes of 4096
/// left, 3388 used; that and `board::ROOMY` (1536), up to a multiple of 512.
const STACK: usize = 5120;

/// Opens the relays NAMES on PINS - closed on low when CLOSED_ON_LOW - and starts their owner, which calls ANNOUNCE
/// after every change and takes each relay's limit from LIMIT_MS (relay) before every look, so a limit changed in
/// place works at once; 0 is no limit. Call once; a pin that cannot be set up stops it.
pub fn start<const N: usize>(
    relays: &'static Relays<N>,
    names: &'static [&'static str; N],
    pins: [u8; N],
    closed_on_low: bool,
    limit_ms: fn(usize) -> u32,
    announce: fn(),
) -> Result<(), EspError> {
    let failed = std::cell::RefCell::new(None);
    let mut opened: Vec<Option<PinDriver<'static, Output>>> = (0..N).map(|_| None).collect();
    crate::open_pins(
        N,
        closed_on_low,
        // the level goes to the pin's output latch, which holds it for the moment the pin becomes an output
        &mut |i, high| {
            if let Err(e) = sys::esp!(unsafe { sys::gpio_set_level(i32::from(pins[i]), u32::from(high)) }) {
                failed.borrow_mut().get_or_insert(e);
            }
        },
        // The pins are the device's and nothing else of the firmware takes them. PinDriver::output leaves the latch
        // as it is: the open level written first is what the pin drives from its first moment as an output.
        &mut |i| match PinDriver::output(unsafe { AnyOutputPin::steal(pins[i]) }) {
            Ok(driver) => opened[i] = Some(driver),
            Err(e) => {
                failed.borrow_mut().get_or_insert(e);
            }
        },
    );
    if let Some(e) = failed.into_inner() {
        return Err(e);
    }
    let mut drivers: Vec<PinDriver<'static, Output>> = opened.into_iter().flatten().collect();
    let mut write = move |relay: usize, closed: bool| {
        if let Err(e) = drivers[relay].set_level(crate::level(closed, closed_on_low).into()) {
            log::error!("{}: pin not set: {e}", names[relay]);
        }
        log::info!("{} {}", names[relay], if closed { "on" } else { "off" });
    };
    relays.start(&mut write);
    // named, so `status` says whose stack it is (research 6.4: five tasks called `pthread`)
    let mut cuts = 0;
    round::round("relays", STACK, round::Core::Hardware, move || loop {
        for relay in 0..N {
            relays.set_limit(relay, limit_ms(relay));
        }
        // commands are carried out as they come; a limit running is looked at once a tick
        let wait = relays.wait_ms(now_ms()).map_or(TICK_MS, |left| left.min(TICK_MS));
        relays.serve_for(Duration::from_millis(u64::from(wait)), &now_ms, &limit_ms, &mut write, &announce);
        // the portions of a feeding relay end here, on their plan: the owner has slept to the nearest edge and no longer than a tick
        let stepped = relays.step(now_ms(), &mut write);
        if relays.cuts() != cuts {
            cuts = relays.cuts();
            log::warn!("a feeding relay got no new plan from the automaton for {HORIZON_MS} ms: its plan ran out and the relay is open");
        }
        if relays.expire(now_ms(), &mut write) || stepped || relays.wait_ms(now_ms()).is_some() {
            announce();
        }
    })?;
    Ok(())
}

/// Tells relay RELAY what it is to do (`Relays::feed`) and has the owner look at it now, so a stop or a new portion does not wait for the owner's next tick.
pub fn feed<const N: usize>(relays: &Relays<N>, relay: usize, feeding: crate::Feeding) -> Result<(), crate::Refused> {
    relays.feed(relay, feeding, now_ms());
    relays.look(WITHIN)
}

/// Hands relay RELAY a word of the automaton: a new one is carried out now (as [`feed`]), one that confirms the last only renews its lease and waits for nobody - unless it is a new one in the guise of a confirmation.
pub fn tell<const N: usize>(relays: &Relays<N>, relay: usize, told: crate::Told) -> Result<(), crate::Refused> {
    match told {
        crate::Told::Say(feeding) => feed(relays, relay, feeding),
        // a plan for the decision that stands waits for nobody; one that came after the end of the plan before (the automaton was silent longer than a plan holds) has the task look now, so that the end is honoured and the new plan followed from the moment it came
        crate::Told::Still(feeding) => {
            if relays.still(relay, feeding, now_ms()) {
                relays.look(WITHIN)?;
            }
            Ok(())
        }
    }
}

/// Whole seconds up until relay RELAY opens by itself; None when it is open or has no limit running.
pub fn left_s<const N: usize>(relays: &Relays<N>, relay: usize) -> Option<u32> {
    relays.closed(relay).then(|| relays.left_ms(relay, now_ms())).flatten().map(|ms| ms.div_ceil(1000))
}

/// What relay RELAY of RELAYS says of itself on the page and in the console: on or off, and while a limit runs, how
/// long it has left - "on - off by itself in 4:12".
pub fn words<const N: usize>(relays: &Relays<N>, relay: usize) -> String {
    crate::words(relays.closed(relay), relays.left_ms(relay, now_ms()))
}
