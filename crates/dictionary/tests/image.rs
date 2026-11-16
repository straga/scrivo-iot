//! What a device's build lays down for its image: the dictionary it serves, as the JSON a client reads, and the version that dictionary carries - from the device's table and the version of its
//! kit (`version.txt`), and never from a table that does not hold together. One helper for every device's build.rs; each used to write the same dozen lines.

use std::path::PathBuf;

use dictionary::image;
use page::dictionary::{Entry, Kind, Meta};

static TABLE: [Entry; 2] = [Entry { unit: "V", ..Entry::row("meter", "voltage", &[Kind::Number]) }, Entry::row("relays", "relay1", &[Kind::On])];

fn json_of(kit: &str) -> String {
    page::dictionary::json(&TABLE, &Meta { device: "probe", board: "PROBE", kit, cards: &["meter", "relays"], settings: &[], machine: None })
}

/// A folder of this test's own, empty.
fn folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("dictionary_image_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn the_dictionary_and_its_version_are_laid_down_for_the_image_of_the_kit_the_device_names() {
    let out = folder("laid");
    let kit = out.join("version.txt");
    std::fs::write(&kit, "probe-26.10.08.1\n").unwrap();
    image::write(&TABLE, json_of, &kit, &out).expect("a table that holds together is laid down");
    let said: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out.join("dictionary.json")).unwrap()).unwrap();
    assert_eq!(said["device"], "probe");
    let version = std::fs::read_to_string(out.join("dictionary_version.txt")).unwrap();
    assert_eq!(version, said["version"].as_str().unwrap());
    assert!(version.starts_with("probe-26.10.08.1"), "the version carries the kit's, without the newline of its file: {version}");
}

#[test]
fn a_table_that_does_not_hold_together_is_refused_and_nothing_is_laid_down() {
    static BAD: [Entry; 2] = [Entry::row("meter", "voltage", &[Kind::Number]), Entry::row("meter", "voltage", &[Kind::Number])];
    let out = folder("bad");
    let kit = out.join("version.txt");
    std::fs::write(&kit, "probe-1").unwrap();
    let why = image::write(&BAD, json_of, &kit, &out).unwrap_err();
    assert!(why.contains("does not hold together") && why.contains("meter/voltage"), "{why}");
    assert!(!out.join("dictionary.json").exists() && !out.join("dictionary_version.txt").exists());
}

#[test]
fn a_device_without_a_version_file_is_refused_in_words_that_name_the_file() {
    let out = folder("nokit");
    let why = image::write(&TABLE, json_of, &out.join("version.txt"), &out).unwrap_err();
    assert!(why.contains("version.txt"), "{why}");
    assert!(!out.join("dictionary.json").exists());
}
