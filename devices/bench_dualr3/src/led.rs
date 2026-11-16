//! The board's status LED, blinking once a second while the firmware runs. The pin and the level it is lit on are the board's (board.yml, status_led).

use std::time::Duration;

use esp_idf_svc::hal::gpio::{AnyOutputPin, PinDriver};
use esp_idf_svc::sys::EspError;
use esp_idf_svc::timer::{EspTaskTimerService, EspTimer};

const PIN: &str = env!("STRAGA_STATUS_LED_PIN");
const LIT_HIGH: bool = matches!(env!("STRAGA_STATUS_LED_LIT").as_bytes(), b"1");
const PERIOD: Duration = Duration::from_millis(1000);

/// Starts the blinking on a timer of the esp_timer task, not a thread of its own. The LED blinks as long as the
/// returned timer is kept.
pub fn start() -> Result<EspTimer<'static>, EspError> {
    let pin: u8 = PIN.parse().expect("STRAGA_STATUS_LED_PIN is a pin number, board_env.py checks it");
    // The pin is the board's LED and nothing else of the firmware takes it.
    let mut led = PinDriver::output(unsafe { AnyOutputPin::steal(pin) })?;
    led.set_level((!LIT_HIGH).into())?;
    let mut lit = false;
    // timer_nowake: skip_unhandled_events - a leap of the chip's clock forward must not make the blink toggle the pin once
    // for every second it missed
    let timer = EspTaskTimerService::new()?.timer_nowake(move || {
        lit = !lit;
        let _ = led.set_level((lit == LIT_HIGH).into());
    })?;
    timer.every(PERIOD)?;
    Ok(timer)
}
