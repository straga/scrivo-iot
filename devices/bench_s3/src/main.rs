//! The main firmware of STRAGA_BASE_S3 in Rust, in ota_0 next to the recovery image in factory. Joins the
//! saved network and confirms itself, shows its log on the USB console and on TCP port 23, and takes the
//! commands status, restart and down on both.

app_desc::app_desc!();

mod site;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::uart::UartDriver;
use esp_idf_svc::io::vfs::BlockingStdIo;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

#[cfg(feature = "vfs-probe")]
fn vfs_probe() {
    let mut read = Vec::new();
    match vfs::read("/probe/recovery_wrote.txt", |chunk: &[u8]| -> Result<(), ()> {
        read.extend_from_slice(chunk);
        Ok(())
    }) {
        Ok(_) => log::info!("vfs probe: read /probe/recovery_wrote.txt: {:?}", String::from_utf8_lossy(&read)),
        Err(e) => log::warn!("vfs probe: /probe/recovery_wrote.txt not read: {e}"),
    }
    let text = format!("written by bench_s3 {} in Rust\n", env!("APP_VERSION"));
    match firmware::flash_owner::write("/probe/rust_wrote.txt", text.as_bytes()) {
        Ok(()) => log::info!("vfs probe: wrote /probe/rust_wrote.txt: {text:?}"),
        Err(e) => log::warn!("vfs probe: /probe/rust_wrote.txt not written: {e}"),
    }
}

fn main() -> Result<(), esp_idf_svc::sys::EspError> {
    esp_idf_svc::sys::link_patches();
    // the beat of the board's time: before anything that waits, or `run` stops the board (firmware::start_time)
    firmware::start_time();
    firmware::log_tee::start();
    log::info!("bench_s3 {} starting", env!("APP_VERSION"));

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    // The ways down without the network, as early as nvs allows: nvs is up now, and no thread of this firmware exists
    // yet - a start that hangs later is still counted, and nothing here can keep the button from being seen.
    ways_down::start();

    // The console UART of the S3 is UART0 on GPIO43/44, behind the board's USB bridge. With a driver under
    // it stdin blocks instead of returning at once; dropping the driver would undo that, so it lives here.
    let uart = UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio43,
        peripherals.pins.gpio44,
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &Default::default(),
    )?;
    let _stdio = BlockingStdIo::uart(uart)?;

    firmware::run(site::site(), peripherals.modem, sysloop, nvs, || {
        #[cfg(feature = "vfs-probe")]
        vfs_probe();
        #[cfg(feature = "events-probe")]
        site::probe::start();
    })
}
