//! What the page's stand starts from for the bench S3: the board's own card, every setting at its default, and the probe changed a minute ago (`tools/page_stand.py` runs this and reads the JSON).
//!
//!     cargo run -q -p bench_s3_api --example stand_values > stand_values.json

use bench_s3_api::{json, probe, TABLE};
use page::dictionary::Said;

fn main() {
    let kit = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../version.txt")).expect("devices/bench_s3/version.txt");
    let served: serde_json::Value = serde_json::from_str(&json(kit.trim())).unwrap();
    let version = served["version"].as_str().unwrap();
    println!(
        "{}",
        dictionary::stand::state_json(&TABLE, &settings::ENGINE, version, &|put| probe::put_value(60_000, &mut |at, value| put(at, Said::Value(value))))
    );
}
