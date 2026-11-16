// The build of the burner's image: its description, its dictionary, the IDF components and the version.
// The pins and the board come from burner.yaml through tools/build.sh (tools/board_env.py); a module that takes a pin asks the environment for it here.

/// The description's settings as the firmware takes them: DESCRIPTION in OUT_DIR/description.rs, each checked by
/// crates/settings - a name the firmware has not, or a value its setting refuses, stops the build with the reason.
fn description() {
    println!("cargo:rerun-if-env-changed=STRAGA_SETTINGS");
    let lines = std::env::var("STRAGA_SETTINGS")
        .unwrap_or_else(|_| panic!("STRAGA_SETTINGS is not set: build with tools/build.sh burner"));
    let pairs: Vec<(&str, &str)> = lines
        .lines()
        .map(|l| l.split_once('=').unwrap_or_else(|| panic!("STRAGA_SETTINGS: '{l}' is not name=value")))
        .collect();
    if let Err(why) = settings::check_description(&burner_settings::ALL, &pairs) {
        panic!("settings of devices/burner/burner.yaml: {why}");
    }
    let rows: String = pairs.iter().map(|(n, v)| format!("    ({n:?}, {v:?}),\n")).collect();
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("description.rs");
    std::fs::write(out, format!("const DESCRIPTION: &[(&str, &str)] = &[\n{rows}];\n")).unwrap();
}

fn main() {
    description();
    // the dictionary this image serves, OUT_DIR/dictionary.json: made of `burner_api`, and refused - the build stops - when its table does not hold together
    dictionary::image::emit(&burner_api::ALL, burner_api::json);
    embuild::espidf::sysenv::output();
    version::emit_app_version();
    fell_where::emit_link_args();
    // What the image calls itself on the network: the device's name, `device:` of burner.yaml, which firmware::site::Site hands the station - not a word of the code's.
    println!("cargo:rerun-if-env-changed=STRAGA_DEVICE");
    let device = std::env::var("STRAGA_DEVICE").unwrap_or_else(|_| panic!("STRAGA_DEVICE is not set: build with tools/build.sh burner, which reads it from burner.yaml"));
    println!("cargo:rustc-env=STRAGA_DEVICE={device}");
    // The pins burner.yaml names that the hardware takes: handed over, a pin that is a type held equal to it at build time (src/hardware), the rest read as numbers where they are opened.
    for key in ["STRAGA_PIN_AUGER", "STRAGA_PIN_IGNITER", "STRAGA_PIN_PUMP", "STRAGA_PIN_AUX_POWER", "STRAGA_RELAYS_CLOSE_ON_LOW", "STRAGA_PIN_FLAME_PIN", "STRAGA_PIN_DRAUGHT_PIN", "STRAGA_PIN_BUS_TX", "STRAGA_PIN_BUS_RX", "STRAGA_PIN_FLUE_SCK", "STRAGA_PIN_FLUE_CS", "STRAGA_PIN_FLUE_SO", "STRAGA_PIN_ZC", "STRAGA_PIN_DIM"] {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key).unwrap_or_else(|_| panic!("{key} is not set: build with tools/build.sh burner, which reads it from burner.yaml"));
        println!("cargo:rustc-env={key}={value}");
    }
}
