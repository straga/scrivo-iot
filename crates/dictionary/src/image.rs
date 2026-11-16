//! What a device's build lays down for its image: `dictionary.json`, the JSON a client reads, and `dictionary_version.txt`, the version that JSON carries -
//! made of the device's table and the version of its kit, and refused when the table does not hold together. Called from the device's build.rs.

use std::path::Path;

use page::dictionary::{check, Entry};

/// Lays the dictionary of TABLE into OUT: JSON_OF is the device's `json` for a kit, KIT_FILE the file with the kit's version (`version.txt` of the device).
/// Nothing is laid down when the table does not hold together or the kit has no version.
pub fn write(table: &[Entry], json_of: impl Fn(&str) -> String, kit_file: &Path, out: &Path) -> Result<(), String> {
    check(table).map_err(|why| format!("the device's dictionary does not hold together: {why}"))?;
    let kit = std::fs::read_to_string(kit_file).map_err(|e| format!("{} is not read: {e}", kit_file.display()))?;
    let json = json_of(kit.trim());
    let version: serde_json::Value = serde_json::from_str(&json).map_err(|e| format!("the dictionary the device made is no JSON: {e}"))?;
    let version = version["version"].as_str().ok_or("the dictionary the device made has no version")?.to_string();
    std::fs::write(out.join("dictionary_version.txt"), version).map_err(|e| e.to_string())?;
    std::fs::write(out.join("dictionary.json"), json).map_err(|e| e.to_string())
}

/// For a build.rs: the dictionary of TABLE laid into OUT_DIR from `version.txt` beside the device's Cargo.toml, a build that stops with the reason when it cannot be.
pub fn emit(table: &[Entry], json_of: impl Fn(&str) -> String) {
    println!("cargo:rerun-if-changed=version.txt");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR: this runs in a build.rs"));
    if let Err(why) = write(table, json_of, Path::new("version.txt"), &out) {
        panic!("{why}");
    }
}
