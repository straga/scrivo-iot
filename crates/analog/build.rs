// The cfgs of the chip - esp32, esp32s3 and the rest - come from esp-idf-sys, and a crate gets them only by asking for them here. `board.rs` needs
// them: the ADC's calibration scheme is not the same on every chip, and naming the wrong one does not build at all (Curve on an ESP32, Line on an S3).
// On a computer there is no chip and no `board.rs`, so nothing is asked.
fn main() {
    // Cargo checks cfg names against a list, and the chips' names are not on it - esp-idf-sys sets them, and only the ones the current target
    // needs. Without this every arm of the calibration is a warning about a name Cargo has never heard of.
    for chip in ["esp32", "esp32s2", "esp32s3", "esp32c2", "esp32c3", "esp32c6"] {
        println!("cargo::rustc-check-cfg=cfg({chip})");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("espidf") {
        embuild::espidf::sysenv::output();
    }
}
