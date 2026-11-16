//! HERE: the MAX6675 on SPI, three wires and no data going out: the sixteen bits of a reading, as the chip sends them.
//! NEVER HERE: degrees, the open-thermocouple bit and what all zeroes mean (crates/max6675), when the flue is read, what to say of a word. Which pins the chip is on is burner.yaml's,
//! handed over by build.rs as numbers.
//! CHECKS: crates/max6675/tests/max6675.rs.
//!
//! Two drivers were read before this one was written, and neither is a dependency:
//!
//!   * ours, off the live burner of the garage - its MicroPython driver: 220 ms a measurement, CS low,
//!     twelve bits, the open bit, two bits more. It clocks the wires by hand because MicroPython had nothing else; we have the peripheral.
//!   * ESPHome's - components/max6675/max6675.cpp, forty lines: the measurement starts on the RISING edge of CS, takes 170 ms typically and 220 at worst,
//!     two bytes are read.
//!
//! THE BUS IS OPENED THROUGH IDF RATHER THAN THROUGH esp-idf-hal, for one reason: the chip has no data input at all, and every public constructor of the hal's SpiDriver asks for a
//! pin to drive data out (spi.rs:460, `sdo: impl OutputPin`). IDF takes `mosi_io_num: -1` for exactly this case, and a pin spent on a signal nothing listens to is a pin the next
//! module will not have.

use std::time::Duration;

use esp_idf_svc::sys::{self, spi_bus_config_t, spi_device_handle_t, spi_device_interface_config_t, spi_transaction_t, EspError};

/// What the chip takes to convert, with the margin ESPHome allows (170 ms typically, 220 at worst, ESPHome waits 250 to be sure). Only the first reading waits on it: after that the
/// round that asks is longer than this.
const CONVERSION: Duration = Duration::from_millis(250);

/// Well under the 4.3 MHz the chip takes: two bytes at this speed are 16 µs, and a slow clock is kinder to a thermocouple's wiring than a fast one.
const CLOCK_HZ: i32 = 1_000_000;

/// The chip on its own device of the SPI bus. IDF drives CS itself, so every reading ends with the rising edge that starts the next measurement.
pub struct Chip {
    dev: spi_device_handle_t,
}

// The handle is IDF's and is used from the one thread that made it.
unsafe impl Send for Chip {}

impl Chip {
    /// Opens the bus on the pins of burner.yaml and takes the first word - thrown away, then waited for: the chip has been converting since it was powered, and when that
    /// measurement began is not known here. Reading it starts a fresh one, and that one is waited for.
    pub fn open() -> Result<Self, EspError> {
        let (sck, cs, so) = (super::pin(env!("STRAGA_PIN_FLUE_SCK")), super::pin(env!("STRAGA_PIN_FLUE_CS")), super::pin(env!("STRAGA_PIN_FLUE_SO")));
        log::info!("thermocouple: MAX6675 on SCK {sck}, CS {cs}, SO {so}");
        let mut bus = spi_bus_config_t { sclk_io_num: sck, max_transfer_sz: 4, ..Default::default() };
        // no pin goes out: this chip only ever talks
        bus.__bindgen_anon_1.mosi_io_num = -1;
        bus.__bindgen_anon_2.miso_io_num = so;
        bus.__bindgen_anon_3.quadwp_io_num = -1;
        bus.__bindgen_anon_4.quadhd_io_num = -1;
        sys::esp!(unsafe { sys::spi_bus_initialize(sys::spi_host_device_t_SPI2_HOST, &bus, sys::spi_common_dma_t_SPI_DMA_DISABLED) })?;

        // A line nobody drives reads as zeroes, and so does a chip that is not powered - the two are the same word. Pulled up, an undriven line reads as ones instead, and the two
        // stop looking alike.
        sys::esp!(unsafe { sys::gpio_set_pull_mode(so, sys::gpio_pull_mode_t_GPIO_PULLUP_ONLY) })?;

        // Mode 0: the chip puts a bit out on the falling edge of the clock and it is sampled on the rising one. Half duplex, because there is no other half: with no pin going out, a
        // full-duplex transaction would still be described as writing sixteen bits nothing can carry.
        let device = spi_device_interface_config_t {
            mode: 0,
            clock_speed_hz: CLOCK_HZ,
            spics_io_num: cs,
            queue_size: 1,
            flags: sys::SPI_DEVICE_HALFDUPLEX,
            ..Default::default()
        };
        let mut dev: spi_device_handle_t = core::ptr::null_mut();
        sys::esp!(unsafe { sys::spi_bus_add_device(sys::spi_host_device_t_SPI2_HOST, &device, &mut dev) })?;
        let chip = Self { dev };
        let _ = chip.word();
        std::thread::sleep(CONVERSION);
        Ok(chip)
    }

    /// One reading: sixteen bits, most significant byte first. The rising edge of CS at the end of it starts the chip's next measurement, so a round of reads needs no separate
    /// command to begin one.
    pub fn word(&self) -> Result<u16, EspError> {
        let mut got = [0u8; 2];
        // Half duplex: nothing is written, sixteen bits are read.
        let mut asked = spi_transaction_t { length: 0, rxlength: 16, ..Default::default() };
        asked.__bindgen_anon_2.rx_buffer = got.as_mut_ptr().cast();
        sys::esp!(unsafe { sys::spi_device_polling_transmit(self.dev, &mut asked) })?;
        Ok(u16::from_be_bytes(got))
    }
}
