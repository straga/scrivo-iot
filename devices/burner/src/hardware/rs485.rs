//! HERE: the UART1 of the RS485 converter: a frame sent, a frame heard within a time, and how the askings came out. Modbus RTU 9600 8N1, direction switched by the
//! converter itself, so the UART is an ordinary one - no RTS, no half-duplex mode.
//! NEVER HERE: the frames of Modbus and what an answer means (crates/modbus), the map of what is read (crates/burner_map), which socket is which. Which pins the bus is
//! on is burner.yaml's, handed over by build.rs as numbers, and the one place they are read is `up` below.
//! CHECKS: crates/modbus/tests/modbus.rs, crates/modbus/tests/channels.rs.
//!
//! The line is the round of the temperatures' alone: it asks, and the mutex below is taken by that round only, so one asking at a time is the round's own order - which is also
//! what the wire allows. The driver is installed by the first asking, in that round's task: a UART's interrupt is served on the core of the task that installs its driver,
//! and that round is on core 1 - from `main` it sat on core 0.

use std::sync::Mutex;
use std::time::Duration;

use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::uart::{config::Config, UartDriver, UART1};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::EspError;
use modbus::{Ask, Counts, Outcome, Receiver, MAX_REPLY};

/// What both modules speak, and what neither changes without a register written to it.
const BAUD: u32 = 9600;
/// How long a module has to answer before the asking counts as silence. A reading of seven registers is 19 bytes, some 20 ms on the wire; the rest is the module thinking.
const ANSWER: Duration = Duration::from_millis(300);

struct Bus {
    uart: UartDriver<'static>,
    rx: Receiver,
    counts: Counts,
}

static BUS: Mutex<Option<Bus>> = Mutex::new(None);

/// Takes UART1 for the bus, once, so nothing else of the firmware takes it; the driver itself is installed by the first asking.
pub fn take(uart1: UART1<'static>) {
    let _ = uart1;
}

/// The bus in HELD, its driver installed now when it is not yet - by the task that asks.
fn up(held: &mut Option<Bus>) -> Option<&mut Bus> {
    if held.is_none() {
        match open(super::pin(env!("STRAGA_PIN_BUS_TX")), super::pin(env!("STRAGA_PIN_BUS_RX"))) {
            Ok(bus) => *held = Some(bus),
            Err(why) => log::error!("rs485: UART1 did not open: {why}"),
        }
    }
    held.as_mut()
}

/// Opens UART1 on TX and RX at BAUD.
fn open(tx: i32, rx: i32) -> Result<Bus, EspError> {
    // the pins are the device's and nothing else of the firmware takes them (burner.yaml, checked against the board's board.yml by the build)
    let uart = UartDriver::new(
        unsafe { UART1::steal() },
        unsafe { AnyIOPin::steal(u8::try_from(tx).expect("a pin number")) },
        unsafe { AnyIOPin::steal(u8::try_from(rx).expect("a pin number")) },
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &Config::new().baudrate(Hertz(BAUD)),
    )?;
    log::info!("rs485: modbus rtu on tx GPIO{tx} rx GPIO{rx}, {BAUD} 8N1, opened on core {}", unsafe {
        esp_idf_svc::sys::xTaskGetCoreID(core::ptr::null_mut())
    });
    Ok(Bus { uart, rx: Receiver::default(), counts: Counts::default() })
}

/// The bytes of a frame as a person reads them in the log.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X} ")).collect::<String>().trim_end().to_string()
}

impl Bus {
    /// Asks and waits up to WAIT for the answer: what it came to, and the registers it carried.
    fn ask(&mut self, ask: Ask, wait: Duration) -> (Outcome, Vec<u16>) {
        let frame = self.rx.ask(ask);
        self.drain();
        if let Err(e) = self.uart.write(&frame) {
            log::error!("rs485: {e}");
        }
        // The converter turns the line around when the last bit has left the shift register, not when write returns: listening before that costs the first byte of
        // the answer (seen on the bench, 20.09 - 01 came back as 07).
        if let Err(e) = self.uart.wait_tx_done(TickType::new_millis(100).ticks()) {
            log::error!("rs485: {e}");
        }
        let until = between::clock::now() + wait;
        while between::clock::now() < until {
            let mut buf = [0u8; MAX_REPLY];
            let left = until.since(between::clock::now());
            match self.uart.read(&mut buf, TickType::new_millis(left.as_millis() as u64).ticks()) {
                Ok(0) => continue,
                Ok(n) => {
                    if self.rx.feed(&buf[..n]) {
                        break;
                    }
                }
                // the driver says ESP_ERR_TIMEOUT when the wait ran out with nothing on the line: that is silence, an outcome of its own, not a fault of the bus
                Err(_) => break,
            }
        }
        let outcome = self.rx.outcome();
        self.counts.count(outcome);
        (outcome, self.rx.values().to_vec())
    }

    /// Drops whatever is still on the line: the tail of an answer that came too late belongs to no asking.
    fn drain(&mut self) {
        let mut junk = [0u8; MAX_REPLY];
        while let Ok(n) = self.uart.read(&mut junk, 0) {
            if n == 0 {
                break;
            }
            log::warn!("rs485: {n} bytes left over: {}", hex(&junk[..n]));
        }
    }
}

/// Asks the bus once and answers what it came to, with the registers it brought. A bus that will not open is a module that never answers.
pub fn ask(ask: Ask) -> (Outcome, Vec<u16>) {
    let Ok(mut held) = BUS.lock() else { return (Outcome::Missed, Vec::new()) };
    match up(&mut held) {
        Some(bus) => bus.ask(ask, ANSWER),
        None => (Outcome::Missed, Vec::new()),
    }
}

/// The counts of the askings, for whoever shows them.
pub fn counts() -> Counts {
    BUS.lock().ok().and_then(|held| held.as_ref().map(|bus| bus.counts)).unwrap_or_default()
}
