//! The burner's image: ONLY THE ASSEMBLY. It raises the board's own services (crates/firmware), starts the modules (src/modules), and gives the
//! network the API it serves (api/). Here is no pin, no number of the fire, no word for a person: the pins are burner.yaml's, what a reading
//! means is crates/, what the burner says outward is api/, how it looks is page/.

app_desc::app_desc!();

// The settings the description seeds, `settings:` of burner.yaml, checked by build.rs: DESCRIPTION.
include!(concat!(env!("OUT_DIR"), "/description.rs"));

mod hardware;
mod modules;
mod roles;

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
    // what the burner did, kept on flash: the board's own lines now, each module's target is added with the module
    firmware::log_tee::keep(&JOURNAL);
    log::info!("burner {} starting", env!("APP_VERSION"));

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    // the ways down without the network, as early as nvs allows: a start that hangs later is still counted
    ways_down::start();

    // The console UART of the S3 is UART0 on GPIO43/44, behind the board's USB bridge. With a driver under it stdin blocks instead of
    // returning at once; dropping the driver would undo that, so it lives here.
    let uart = UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio43,
        peripherals.pins.gpio44,
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &Default::default(),
    )?;
    let _stdio = BlockingStdIo::uart(uart)?;

    // The relays switch mains: every organ off whatever it was before the restart, and moved from now on only by a command. Opened once the console's driver is in: installing it cuts a log line in
    // flight, and the lines of the relays opened are what shows the start off. Until then the pins float, and the relay board holds them open.
    modules::relays::start()?;
    // The pellets count the auger's seconds: after the relays, which tell them, and with vfs open once here, before any thread may mount it at the same moment (firmware::run opens it again - the lock is made by the first).
    let _ = vfs::open_once().inspect_err(|e| log::error!("vfs not mounted: {e} - the pellets count from zero"));
    modules::pellets::start();
    modules::soot::start();
    modules::restart::start();
    // the socket the burner is plugged into is read by the page's loop once it starts: left to it here, before `firmware::run`
    modules::mains::start();
    // The modules, each started where its hardware is. The bus of the water's sensors on UART1: taken here, its driver installed by the round that asks it, on core 1 - and that round.
    hardware::rs485::take(peripherals.uart1);
    modules::temperatures::start_retrying();
    // the flue's thermocouple on SPI: its own card, its own round, nothing shared with the bus
    if let Err(why) = modules::flue::start() {
        log::warn!("flue: the reading of the flue did not start: {why}");
    }
    // ADC1 is opened once for the board, and the draught reads GPIO4 of it, the flame GPIO7.
    if let Err(why) = hardware::adc::open(peripherals.adc1) {
        log::warn!("analog: ADC1 did not open: {why}");
    }
    // the draught on GPIO4 of the same unit, read by the same code: two sensors, one driver
    if let Err(why) = modules::draught::start(peripherals.pins.gpio4) {
        log::warn!("draught: the reading of the pressure did not start: {why}");
    }
    if let Err(why) = modules::flame::start(peripherals.pins.gpio7) {
        log::warn!("flame: the reading of the flame did not start: {why}");
    }
    // the fan, last of the rounds on purpose: the crossings counted and the gate MCPWM fires from them, and the round that holds the draught - it reads what the draught's round writes and moves what the gate fires. A gate MCPWM refuses to
    // build is written in the log and nothing more: a fan must not keep this board off the network, and it then moves only by hand.
    if let Err(why) = modules::fan::start() {
        log::warn!("fan: the draught is not being held: {why} - the fan moves only by hand");
    }

    // the automaton, last of all: it reads every round above by role and hands its commands to the relays and the fan by role. A number of its card the door refuses is said on the card and the burner is run by hand.
    if let Err(why) = modules::automaton::start() {
        log::error!("automaton: did not start: {why} - the burner is run by hand only");
    }

    let site = firmware::site::Site {
        device: env!("STRAGA_DEVICE"),
        board: "STRAGA_BASE_S3",
        kit: env!("APP_VERSION"),
        // THE MODULES OF THIS DEVICE, in the order their cards stand: src/modules/mod.rs.
        modules: modules::ALL,
        rules: &burner_settings::RULES,
        description: DESCRIPTION,
        dictionary: DICTIONARY,
    };
    firmware::run(site, peripherals.modem, sysloop, nvs, || {})
}

/// The burner's dictionary, made by build.rs from `burner_api` when this image was built: the JSON a client reads, its version, and the
/// table the modules' values are addressed by.
const DICTIONARY: page::dictionary::Served = page::dictionary::Served {
    json: include_str!(concat!(env!("OUT_DIR"), "/dictionary.json")),
    version: include_str!(concat!(env!("OUT_DIR"), "/dictionary_version.txt")),
    table: &burner_api::TABLE,
};

/// The log targets the journal on flash keeps, besides every warning and error.
const JOURNAL: [&str; 5] = ["firmware", "burner::modules::fan", "burner::hardware::fan_gate", "burner::modules::automaton", "burner::modules::mains"];
