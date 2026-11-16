//! The Dual R3's dictionary as its image serves it, for whatever stands in for the board on a computer (the page's stand, a probe): the same JSON build.rs puts into the image, with the kit of
//! devices/bench_dualr3/version.txt.
//!
//!     cargo run -p bench_dualr3_api --example served_json > dictionary.json

fn main() {
    let kit = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../version.txt")).expect("devices/bench_dualr3/version.txt");
    println!("{}", bench_dualr3_api::json(kit.trim()));
}
