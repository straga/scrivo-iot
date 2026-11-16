//! What the page's stand starts from for the Dual R3: the board's own card, every setting at its default, and the device's values as its modules write them - a lamp on L1, a meter that reads, some
//! energy counted - by the same functions the board writes them with (`tools/page_stand.py` runs this and reads the JSON).
//!
//!     cargo run -q -p bench_dualr3_api --example stand_values > stand_values.json
//!     cargo run -q -p bench_dualr3_api --example stand_values -- --day-unsure > unsure.json    # the count not sure of its day: today and the month are no number

use bench_dualr3_api::{energy, json, meter, relays, switches, TABLE};
use page::dictionary::Said;

fn main() {
    let kit = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../version.txt")).expect("devices/bench_dualr3/version.txt");
    let served: serde_json::Value = serde_json::from_str(&json(kit.trim())).unwrap();
    let version = served["version"].as_str().unwrap();
    let day_sure = !std::env::args().any(|a| a == "--day-unsure");
    println!(
        "{}",
        dictionary::stand::state_json(&TABLE, &settings::ENGINE.iter().copied().chain(bench_dualr3_api::energy::SETTINGS).collect::<Vec<_>>(), version, &|put| {
            let mut say = |at, value| put(at, Said::Value(value));
            relays::put_values([true, false], &mut say);
            switches::put_values(&[switches::Key { closed: false, toggles: 3 }, switches::Key { closed: false, toggles: 0 }], &mut say);
            let window = |accepted, rejected, missed| meter::Window { accepted, rejected, missed };
            meter::put_values(
                &meter::Now {
                    reading: Some(meter::Reading { volts: 237.4, outputs: [meter::Output { amps: 0.481, watts: 110.2 }, meter::Output { amps: 0.0, watts: 0.0 }] }),
                    now: window(822, 0, 0),
                    last: Some(window(1000, 0, 0)),
                },
                &mut say,
            );
            energy::put_values(Some(&energy::Sums { today: [0.012, 0.110], month: [1.313, 0.503], total: [2.485, 2.084] }), day_sure, &mut say);
            energy::put_setting_values(&bench_dualr3_api::energy::history_room(60, 12, 896 * 1024), &mut say);
        })
    );
}
