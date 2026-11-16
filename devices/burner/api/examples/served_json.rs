//! The burner's dictionary as its image serves it, for whatever stands in for the board on a computer (the page's stand, a probe): the same JSON
//! build.rs puts into the image, with the kit of devices/burner/version.txt.
//!
//!     cargo run -p burner_api --example served_json > dictionary.json

fn main() {
    let version = concat!(env!("CARGO_MANIFEST_DIR"), "/../version.txt");
    let kit = std::fs::read_to_string(version).expect("devices/burner/version.txt");
    println!("{}", burner_api::json(kit.trim()));
}
