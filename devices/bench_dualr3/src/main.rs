//! The main firmware of the Sonoff Dual R3 on the bench in Rust, in ota_0 next to the recovery image in factory. What
//! every device runs (crates/firmware): the network and the confirmation, the page, the log and the shell on TCP port
//! 23 - the board's status LED blinking, the two relays with their wall switches (wall.rs), and the BL0939 energy
//! meter read once a second (meter.rs), the energy counted from it and kept in vfs (count.rs). What it says outward - the
//! dictionary of its entities, their values and commands - is `bench_dualr3_api`; the words of its page are `page/menu.json`.

app_desc::app_desc!();

// The settings the description of this device seeds, [package.metadata.rust_app.settings] of its Cargo.toml, checked
// by build.rs: DESCRIPTION.
include!(concat!(env!("OUT_DIR"), "/description.rs"));

mod count;
mod led;
mod meter;
mod wall;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::uart::UartDriver;
use esp_idf_svc::io::vfs::BlockingStdIo;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

fn main() -> Result<(), esp_idf_svc::sys::EspError> {
    esp_idf_svc::sys::link_patches();
    // the beat of the board's time: before anything that waits, or `run` stops the board (firmware::start_time)
    firmware::start_time();
    firmware::log_tee::start();
    log::info!("bench_dualr3 {} starting", env!("APP_VERSION"));

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    // The ways down without the network, as early as nvs allows: nvs is up now, and no thread of this firmware exists
    // yet - a start that hangs later is still counted, and nothing here can keep the button from being seen.
    ways_down::start();

    // The console UART of the ESP32 is UART0 on GPIO1/3, the pads at the back of the board (board.yml). With a driver
    // under it stdin blocks instead of returning at once; dropping the driver would undo that, so it lives here.
    let uart = UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio1,
        peripherals.pins.gpio3,
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &Default::default(),
    )?;
    let _stdio = BlockingStdIo::uart(uart)?;

    let _led = led::start().inspect_err(|e| log::error!("status LED not started: {e}"));

    // The relays switch mains: opened here, before the network, and moved from now on only by a command.
    wall::start()?;
    // the count before the meter's first asking: vfs is opened here once, before any thread may mount it at the same
    // moment (firmware::run opens it again - the lock is made by the first)
    let _ = vfs::open_once().inspect_err(|e| log::error!("vfs not mounted: {e} - the energy counts from zero"));
    count::start();
    let _ = meter::start(peripherals.uart2).inspect_err(|e| log::error!("meter not started: {e}"));

    let site = firmware::site::Site {
        device: "bench_dualr3",
        board: "SONOFF_DUAL_R3",
        kit: env!("APP_VERSION"),
        // the modules, in the order the page draws their cards (crates/module): the order of `bench_dualr3_api::CARDS`
        modules: &[&wall::RelaysModule, &wall::KeysModule, &meter::Module, &count::Module],
        rules: &settings::NO_RULES,
        description: DESCRIPTION,
        dictionary: DICTIONARY,
    };
    firmware::run(site, peripherals.modem, sysloop, nvs, || {})
}

/// The Dual's dictionary, made by build.rs from `bench_dualr3_api` when this image was built: the JSON a client reads, its version, and the table the modules' values are addressed by.
const DICTIONARY: page::dictionary::Served = page::dictionary::Served {
    json: include_str!(concat!(env!("OUT_DIR"), "/dictionary.json")),
    version: include_str!(concat!(env!("OUT_DIR"), "/dictionary_version.txt")),
    table: &bench_dualr3_api::TABLE,
};
