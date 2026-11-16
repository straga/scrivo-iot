//! The BL0939 energy meter of the Sonoff Dual R3 on UART2: set up with ESPHome's six writes, asked for its packet once
//! a second, the packet taken apart by crates/bl0939. The last reading and the packet counts are what the page (card
//! "meter") and the shell's `meter` show; every packet taken is published and announced, so an open page moves with it.
//!
//! Every asking comes to exactly one outcome (bl0939::Outcome): accepted, rejected for its checksum, or missed - no
//! whole packet in time. They are counted over a window of 1000 askings (bl0939::Counts), so the counts say how the
//! line is now, and the last full window stays readable while the next fills. Zero current and power prove nothing -
//! a silent meter gives them too; the voltage and the accepted count do. What the meter says outward - its entities, their addresses - is `bench_dualr3_api::meter`.

use std::sync::Mutex;
use std::time::Duration;

use bl0939::{Counts, Outcome, Reading, Receiver, Tally, INIT, PACKET_LEN, READ_ALL};
use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::gpio::{AnyInputPin, AnyIOPin, AnyOutputPin};
use esp_idf_svc::hal::uart::{config::Config, UartDriver, UART2};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::EspError;
use bench_dualr3_api::meter as api;
use module::card::{Fill, Published, Reads};

const TX: &str = env!("STRAGA_PIN_METER_TX");
const RX: &str = env!("STRAGA_PIN_METER_RX");
const BAUD: u32 = 4800;
/// How often the meter is asked. It updates its RMS values every 800 ms (INIT, the mode register).
const EVERY: Duration = Duration::from_secs(1);
/// How long an asking waits for the whole packet: 35 bytes at 4800 baud take 73 ms.
const ANSWER_IN: Duration = Duration::from_millis(300);

/// Which channel of the BL0939 (0 is A, 1 is B) measures which output of the box: L1, switched by relay1, runs
/// through channel B, L2 through channel A - measured with lamps on each output, 19.09. The names on the
/// page follow the box, as the relays' do.
const L1: usize = 1;
const L2: usize = 0;

static LAST: Mutex<Option<Reading>> = Mutex::new(None);
static COUNTS: Mutex<Counts> = Mutex::new(Counts { now: Tally { accepted: 0, rejected: 0, missed: 0 }, last: None });
/// The card of the meter as its task last published it: the reading, or why there is none, and the windows of packets.
static PUBLISHED: Published<{ page::dictionary::addresses(&bench_dualr3_api::TABLE, api::CARD) }, 0> = Published::new();

fn pin(number: &str) -> u8 {
    number.parse().expect("a pin number: tools/board_env.py checks every pin of the device")
}

/// Starts the task that opens UART2 on the meter's pins, sets the meter up and asks it. The meter's pins are the
/// device's and nothing else of the firmware takes them.
///
/// The driver is installed IN the task, on core 1: a UART's interrupt is served on the core that installed it,
/// and the task that reads the meter is the hardware's.
pub fn start(uart: UART2<'static>) -> Result<(), EspError> {
    publish();
    // formatting the count and writing it to vfs (count.rs) run on this task too
    firmware::task::round("meter", 8192, firmware::task::Core::Hardware, move || {
        let opened = UartDriver::new(
            uart,
            unsafe { AnyOutputPin::steal(pin(TX)) },
            unsafe { AnyInputPin::steal(pin(RX)) },
            Option::<AnyIOPin>::None,
            Option::<AnyIOPin>::None,
            &Config::new().baudrate(Hertz(BAUD)),
        );
        match opened {
            Ok(uart) => run(uart),
            Err(why) => log::error!("meter: UART2 did not open: {why} - the meter is not read"),
        }
    })?;
    Ok(())
}

fn run(uart: UartDriver<'static>) {
    for row in INIT {
        if let Err(e) = uart.write(&row) {
            log::error!("meter: setup write {:02X} failed: {e}", row[1]);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut rx = Receiver::default();
    let mut buf = [0u8; PACKET_LEN];
    loop {
        let asked = between::clock::now();
        let _ = uart.clear_rx();
        rx.ask();
        if uart.write(&READ_ALL).is_ok() {
            while !matches!(rx.outcome(), Outcome::Accepted(_)) && asked.elapsed() < ANSWER_IN {
                let left = ANSWER_IN.saturating_sub(asked.elapsed());
                match uart.read(&mut buf, TickType::from(left).0) {
                    Ok(n) if n > 0 => {
                        rx.feed(&buf[..n]);
                    }
                    _ => break,
                }
            }
        }
        let outcome = rx.outcome();
        COUNTS.lock().unwrap().count(&outcome);
        let reading = match outcome {
            Outcome::Accepted(raw) => Some(Reading::from(raw)),
            _ => None,
        };
        crate::count::asked(between::clock::now().as_millis(), reading.map(|r| [r.watts[L1], r.watts[L2]]));
        if let Some(reading) = reading {
            *LAST.lock().unwrap() = Some(reading);
        }
        publish();
        if reading.is_some() {
            web::announce(api::CARD);
        }
        std::thread::sleep(EVERY.saturating_sub(asked.elapsed()));
    }
}

/// The rows of the shell's `meter`, in the order they stand: the voltage, each output of the
/// box with its current and power, the energy counted today, this month and in all, the counts of the window filling
/// now and of the last full one. A dash for a value until the first packet.
fn rows() -> Vec<(&'static str, String)> {
    let last = *LAST.lock().unwrap();
    let output = |n: usize| last.map_or("-".into(), |r| format!("{:.3} A · {:.1} W", r.amps[n], r.watts[n]));
    let (now, window) = COUNTS.lock().unwrap().words(EVERY);
    vec![
        ("voltage", last.map_or("-".into(), |r| format!("{:.1} V", r.volts))),
        ("L1", output(L1)),
        ("L2", output(L2)),
    ]
    .into_iter()
    .chain(crate::count::rows())
    .chain([
        ("packets", now),
        ("last window", window),
    ])
    .collect()
}

/// The meter now as `bench_dualr3_api` takes it: the last reading with its outputs in the box's order, and the windows of packets.
fn now() -> api::Now {
    let window = |t: Tally| api::Window { accepted: t.accepted, rejected: t.rejected, missed: t.missed };
    let output = |n: usize, r: Reading| api::Output { amps: r.amps[n], watts: r.watts[n] };
    let reading = LAST.lock().unwrap().map(|r| api::Reading { volts: r.volts, outputs: [output(L1, r), output(L2, r)] });
    let counts = *COUNTS.lock().unwrap();
    api::Now { reading, now: window(counts.now), last: counts.last.map(window) }
}

/// Puts the card of the meter together and publishes it.
fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| api::put_values(&now(), &mut |at, value| card.put(at, value)));
}

/// The shell's `meter`: the same rows as the page.
fn lines(line: &mut dyn FnMut(&str)) -> Result<(), String> {
    for (name, state) in rows() {
        line(&format!("{name} {state}"));
    }
    Ok(())
}

/// The console word `meter`, which takes no word after it.
fn meter_word(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    words.is_empty().then(|| lines(line))
}

static VERBS: [module::Verb; 1] = [module::Verb {
    name: "meter",
    help: "Voltage, current and power the energy meter reads, the energy counted, and its packet counts",
    run: meter_word,
}];

/// The meter by the modules' contract (crates/module): its card, and the console word `meter`. The energy it counts is the count's card (count.rs).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn verbs(&self) -> &'static [module::Verb] {
        &VERBS
    }
}
