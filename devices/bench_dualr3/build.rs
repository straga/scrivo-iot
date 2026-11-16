// The pins come from the board's board.yml through tools/build.sh (tools/board_env.py), checked there.
const FROM_BOARD: [&str; 9] = [
    "STRAGA_RELAYS_CLOSE_ON_LOW",
    "STRAGA_STATUS_LED_PIN",
    "STRAGA_STATUS_LED_LIT",
    "STRAGA_PIN_RELAY1",
    "STRAGA_PIN_RELAY2",
    "STRAGA_PIN_SWITCH1",
    "STRAGA_PIN_SWITCH2",
    "STRAGA_PIN_METER_TX",
    "STRAGA_PIN_METER_RX",
];

/// The description's settings as the firmware takes them: DESCRIPTION in OUT_DIR/description.rs, each checked by
/// crates/settings - a name the firmware has not, or a value its setting refuses, stops the build with the reason.
fn description() {
    println!("cargo:rerun-if-env-changed=STRAGA_SETTINGS");
    let lines = std::env::var("STRAGA_SETTINGS")
        .unwrap_or_else(|_| panic!("STRAGA_SETTINGS is not set: build with tools/build.sh bench_dualr3"));
    let pairs: Vec<(&str, &str)> = lines
        .lines()
        .map(|l| l.split_once('=').unwrap_or_else(|| panic!("STRAGA_SETTINGS: '{l}' is not name=value")))
        .collect();
    let list: Vec<&settings::Setting> = settings::ENGINE.iter().copied().chain(bench_dualr3_api::energy::SETTINGS).collect();
    if let Err(why) = settings::check_description(&list, &pairs) {
        panic!("[package.metadata.rust_app.settings] of devices/bench_dualr3/Cargo.toml: {why}");
    }
    let rows: String = pairs.iter().map(|(n, v)| format!("    ({n:?}, {v:?}),\n")).collect();
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("description.rs");
    std::fs::write(out, format!("const DESCRIPTION: &[(&str, &str)] = &[\n{rows}];\n")).unwrap();
}

fn main() {
    description();
    // the dictionary this image serves, OUT_DIR/dictionary.json: made of `bench_dualr3_api`, and refused - the build stops - when its table does not hold together.
    // It is called what the build says (tools/board_env.py): the Dual by its name, the stand that runs its modules on the bench S3 by its own.
    for key in ["STRAGA_DEVICE", "STRAGA_BOARD"] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let device = std::env::var("STRAGA_DEVICE")
        .unwrap_or_else(|_| panic!("STRAGA_DEVICE is not set: build with tools/build.sh, which reads it from the device's description"));
    let board = std::env::var("STRAGA_BOARD")
        .unwrap_or_else(|_| panic!("STRAGA_BOARD is not set: build with tools/build.sh, which reads it from the device's description"));
    dictionary::image::emit(&bench_dualr3_api::ALL, |kit| bench_dualr3_api::json_for(kit, &device, &board));
    embuild::espidf::sysenv::output();
    version::emit_app_version();
    fell_where::emit_link_args();
    for key in FROM_BOARD {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key).unwrap_or_else(|_| {
            panic!("{key} is not set: build with tools/build.sh bench_dualr3, which reads it from the board's board.yml")
        });
        println!("cargo:rustc-env={key}={value}");
    }
}
