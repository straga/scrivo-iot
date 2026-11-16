//! What bench_s3 tells its page beyond what every device does (firmware::site): nothing of its own, and the /events probe when built with it. What the
//! S3 says outward - the dictionary of its entities - is `bench_s3_api`.

use firmware::site::Site;

/// The S3's dictionary, made by build.rs from `bench_s3_api` when this image was built: the JSON a client reads, its version, and the table the values are addressed by.
const DICTIONARY: page::dictionary::Served = page::dictionary::Served {
    json: include_str!(concat!(env!("OUT_DIR"), "/dictionary.json")),
    version: include_str!(concat!(env!("OUT_DIR"), "/dictionary_version.txt")),
    table: &bench_s3_api::TABLE,
};

pub fn site() -> Site {
    Site {
        device: "bench_s3",
        board: "STRAGA_BASE_S3",
        kit: env!("APP_VERSION"),
        // nothing of its own; the probe, when built with it, is the one module
        #[cfg(feature = "events-probe")]
        modules: &[&probe::Probe],
        #[cfg(not(feature = "events-probe"))]
        modules: &[],
        rules: &settings::NO_RULES,
        description: &[],
        dictionary: DICTIONARY,
    }
}

/// The /events probe by the modules' contract (crates/module): a card of one row that says in ms when it last changed, in an image built with it only.
#[cfg(feature = "events-probe")]
pub mod probe {
    use module::card::{Fill, Published, Reads};
    use bench_s3_api::probe as api;

    static PUBLISHED: Published<{ page::dictionary::addresses(&bench_s3_api::TABLE, api::CARD) }, 0> = Published::new();
    static CHANGED_AT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    pub struct Probe;

    impl module::Module for Probe {
        fn card(&self) -> &'static str {
            api::CARD
        }

        fn published(&self) -> Option<&dyn Reads> {
            Some(&PUBLISHED)
        }
    }

    fn publish() {
        let at = CHANGED_AT.load(std::sync::atomic::Ordering::Relaxed);
        firmware::publish(&PUBLISHED, api::CARD, |card| api::put_value(at, &mut |address, value| card.put(address, value)));
    }

    /// Changes the probe every 7 s, publishes it and announces it.
    pub fn start() {
        publish();
        let _ = firmware::task::round("events probe", 4096, firmware::task::Core::Hardware, || loop {
            std::thread::sleep(std::time::Duration::from_millis(7000));
            CHANGED_AT.store(firmware::board::uptime_ms() as u32, std::sync::atomic::Ordering::Relaxed);
            publish();
            web::announce(api::CARD);
        });
    }
}
