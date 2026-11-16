//! HERE: ADC1, opened once for the whole board and shared by channels: the window of samples of the draught's pin and of the flame's, in millivolts.
//! NEVER HERE: pascals, the divider, the thresholds of the flame: a window of millivolts - its mean, its spread, whether it stood at the top of the
//! scale - is all this file gives. Which pin a module is on is burner.yaml's; the pin is a type here, so the two are held equal at build time.
//! CHECKS: crates/analog/tests/analog.rs, crates/analog/tests/window.rs.

use esp_idf_svc::hal::adc::ADC1;
use esp_idf_svc::hal::gpio::{ADCPin, Gpio4, Gpio7};
use esp_idf_svc::sys::EspError;

pub use analog::board::Sample;

// The window the pin is read in is the one the API says is fixed in the image, and the pin is the one burner.yaml names (build.rs hands it over).
const _: () = assert!(analog::board::SAMPLES == burner_api::flame::SAMPLES, "a window of another size than the API says");
const _: () = assert!(
    matches!(env!("STRAGA_PIN_FLAME_PIN").as_bytes(), b"7"),
    "burner.yaml puts the flame on another pin than GPIO7: change src/hardware/adc.rs with it"
);
const _: () = assert!(
    matches!(env!("STRAGA_PIN_DRAUGHT_PIN").as_bytes(), b"4"),
    "burner.yaml puts the draught on another pin than GPIO4: change src/hardware/adc.rs with it"
);

/// Opens ADC1 for the whole board. Once, before any channel of it is taken.
pub fn open(adc1: ADC1<'static>) -> Result<(), EspError> {
    analog::board::open(adc1)
}

/// The flame's photoresistor, GPIO7, as a channel of ADC1: a window of samples a read.
pub struct FlamePin(analog::board::Pin<<Gpio7<'static> as ADCPin>::AdcChannel>);

impl FlamePin {
    pub fn new(pin: Gpio7<'static>) -> Result<Self, EspError> {
        Ok(Self(analog::board::Pin::new("flame", pin)?))
    }

    /// One window; None when every sample refused - no millivolts at all, which is not the number zero.
    pub fn read(&mut self) -> Option<Sample> {
        self.0.read()
    }
}

/// The draught's pressure sensor behind its divider, GPIO4, as a channel of ADC1: a window of samples a read.
pub struct DraughtPin(analog::board::Pin<<Gpio4<'static> as ADCPin>::AdcChannel>);

impl DraughtPin {
    pub fn new(pin: Gpio4<'static>) -> Result<Self, EspError> {
        Ok(Self(analog::board::Pin::new("draught", pin)?))
    }

    /// One window; None when every sample refused - no millivolts at all, which is not the number zero.
    pub fn read(&mut self) -> Option<Sample> {
        self.0.read()
    }
}
