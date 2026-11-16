//! What every device of this project runs, whatever its board: the log on the wire and on TCP port 23, the shell on
//! both, the station on the saved network with the image confirmed once it is up, and the page with its two things -
//! the console and the station. A device's main adds its own: its console UART, its own things, its own tasks.

pub mod board;
pub mod clock;
pub mod console;
pub mod cores;
pub mod fell;
pub mod flash_owner;
pub mod log_tee;
pub mod net;
pub mod nvs_settings;
pub mod orders;
pub mod site;
pub mod task;

/// The kinds of line of the black box (journal::folded): a device names one as the target of a line that must be written whole.
pub use journal::{BOX_CLOCK, BOX_FRONTS, BOX_TICK};

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;

/// Starts the beat of the board's time driver (`between::steady_time`): the FIRST CALL of a device's main, after the patches
/// are linked and before anything that waits - the owners of the relays, the rounds of the sensors, the network. The slices of
/// every wait of the board are counted by it, and the board that cannot start it falls here with the reason in its note
/// (`fell`) instead of running on a clock that every leap of the chip's counter holds. [`run`] checks that it stood first.
///
/// NO BEAT, NO BOARD, as with the watcher of the tick in [`run`]: a task that could not be made (no memory) cannot be said by a
/// timer that set it, and the image is rolled back to the last one that could start.
pub fn start_time() {
    if let Err(e) = between::steady_time::start_beat() {
        panic!("time: its beat not started ({e}) - no board without it");
    }
}

/// Runs the firmware for good: vfs, the consoles, the station, the page. EXTRA runs once vfs is open and the station
/// is set up, before the page starts - where a device starts its own tasks. Returns only if something could not
/// be set up.
pub fn run(
    site: site::Site,
    modem: Modem<'static>,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    extra: impl FnOnce(),
) -> Result<(), EspError> {
    // A leap of the chip's clock is a line of the log, with its size: the board's clock does not take it (between::clock),
    // so this is the only place it shows. Set before the first line, whose stamp is the first reading of that clock.
    between::clock::on_leap(|leap| {
        let way = if leap.back { "back" } else { "forward" };
        log::warn!(target: journal::BOX_CLOCK, "clock leap {way} {} ms", leap.ms);
        clock::leapt();
    });
    // FIRST LINE OF THE LOG, before anything else can fill it. A board that restarts takes its log with it, so
    // the reason has to be the thing waiting at the top when somebody joins.
    let (code, why) = board::why_up();
    match reset_why::is_fault(code) {
        true => log::warn!("came up: {why} [code {code}]"),
        false => log::info!("came up: {why} [code {code}]"),
    }
    // WHERE it fell, which the line above cannot say: the chip prints that into the wire and restarts. Nothing
    // is said when there is no note - an empty line about a fall reads as a fall nobody can place.
    fell::rise(site.device);
    for line in fell::lines() {
        log::warn!("{line}");
    }

    site::keep(&site);
    // the settings every board has - the clock's and the station's - and the modules' after them
    let mut list = settings::ENGINE.to_vec();
    list.extend(site::settings());
    let owner = nvs_settings::start(nvs.clone(), list, site.rules, site.description, clock::changed);
    clock::set_zone();
    // the watcher of the tick: a leap of the systimer's counter back leaves the tick's alarm silent, and no tick comes
    // to set it again.
    //
    // NO WATCHER, NO BOARD. It is the one guard against that silence, and without it the next leap resets the board
    // by the interrupt watchdog with no note - a device under load. So the board falls here, with the reason in its
    // note (`fell`), before the image is confirmed (net::start): an image that cannot start its watcher is rolled back
    // to the last one that could.
    let watching = unsafe { esp_idf_svc::sys::systick::systick_watch() };
    if watching != 0 {
        panic!("tick: its watcher not started (esp_err {watching}) - no board without it");
    }
    // the beat of the time driver stands, and stood first: `start_time` is the first call of the device's main, so that no timer
    // of embassy-time was set before it. A timer that came first is held by every leap of the chip's counter back until the beat
    // comes, and a board without the beat is held by each of them.
    //
    // NO BEAT FIRST, NO BOARD, as with the watcher: it falls here, with the reason in its note, before the image is confirmed.
    let timers = between::steady_time::waiters();
    if !timers.beating || timers.early != 0 {
        panic!(
            "time: the beat was not first (beating {}, {} timers before it) - firmware::start_time() is the first call of the device's main",
            timers.beating, timers.early
        );
    }
    // The first mount creates app_fs.c's lock: here, before any thread can mount at the same moment.
    if let Err(e) = vfs::open_once() {
        log::warn!("vfs not mounted: {e} - the page will say its files are missing");
    }
    // the one writer of the flash: the settings, the modules' memory, the files and the journal on flash (log_tee::keep)
    // - the journal's lines since main began wait for it
    if let Err(why) = flash_owner::start(owner) {
        log::error!("flash: its owner not started ({why}) - no setting is saved and no journal written");
    }
    console::spawn("usb console", console::STACK, console::usb);
    let station = net::start(site.device, modem, sysloop, nvs)?;
    extra();
    // the site lives as long as the program: the loop and the task `orders` read it for good
    let site: &'static site::Site = Box::leak(Box::new(site));
    orders::start(site);
    match log_tee::to_net() {
        Some(log) => web::esp::start(site, &orders::ORDERS, log)?,
        None => log::error!("net loop: no log to carry - log_tee::start was not called"),
    }
    net::run(station);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

/// Puts the card of the module whose card is CARD together with FILL and publishes it into PUBLISHED - what every
/// owner of a card calls when its values may have moved. A card that does not fit is said in the log by the module's
/// name, and the last one stays.
pub fn publish<const N: usize, const T: usize>(
    published: &module::card::Published<N, T>,
    card: &str,
    fill: impl FnOnce(&mut module::card::Card<N, T>),
) {
    let mut put = module::card::Card::new();
    fill(&mut put);
    if let Err(why) = published.publish(put) {
        log::error!("{card}: its card not published: {why}");
    }
}
