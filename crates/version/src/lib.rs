//! The image version of a device, <board>-<yy.mm.dd>.<n>: read from its version.txt by the device's build.rs,
//! checked, and handed to the device crate as APP_VERSION, which app_desc puts into esp_app_desc_t; and the project
//! name of the image beside it, APP_PROJECT_NAME.

/// The variable of the build that names the main image's project name: the name the recovery image accepts for
/// ota_0. It must equal the recovery image's own setting, SCRIVO_SAFE_MAIN_PROJECT (scrivo-safe, main/Kconfig.projbuild).
pub const MAIN_PROJECT_VARIABLE: &str = "SCRIVO_MAIN_PROJECT";
/// The name when the variable is not set.
pub const DEFAULT_MAIN_PROJECT: &str = "micropython";

/// The project name of the image: NAMED (the variable's value) when it is set and not empty, else the default.
/// esp_app_desc_t.project_name is 32 bytes with the terminating zero, and the recovery image compares it as a word.
pub fn main_project(named: Option<&str>) -> Result<&str, &'static str> {
    let name = match named {
        None | Some("") => return Ok(DEFAULT_MAIN_PROJECT),
        Some(name) => name,
    };
    if name.len() > 31 {
        return Err("longer than 31 characters");
    }
    if !name.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("not printable ASCII without spaces");
    }
    Ok(name)
}

/// For a device's build.rs: reads version.txt next to the device's Cargo.toml and sets APP_VERSION; a
/// version of another form stops the build and says why. Sets APP_PROJECT_NAME from SCRIVO_MAIN_PROJECT, and reruns
/// the build when that variable changes.
pub fn emit_app_version() {
    println!("cargo:rerun-if-changed=version.txt");
    let text = std::fs::read_to_string("version.txt").expect("version.txt of the device is missing");
    let version = text.trim();
    if let Err(why) = check_version(version) {
        panic!("version.txt: {why}: '{version}', expected <board>-<yy.mm.dd>.<n>, e.g. stragabases3-26.09.19.1");
    }
    println!("cargo:rustc-env=APP_VERSION={version}");
    println!("cargo:rerun-if-env-changed={MAIN_PROJECT_VARIABLE}");
    let named = std::env::var(MAIN_PROJECT_VARIABLE).ok();
    match main_project(named.as_deref()) {
        Ok(name) => println!("cargo:rustc-env=APP_PROJECT_NAME={name}"),
        Err(why) => panic!("{MAIN_PROJECT_VARIABLE}: {why}: '{}'", named.unwrap_or_default()),
    }
}

pub fn check_version(version: &str) -> Result<(), &'static str> {
    // esp_app_desc_t.version is 32 bytes with the terminating zero.
    if version.len() > 31 {
        return Err("longer than 31 characters");
    }
    let (board, date_build) = version.split_once('-').ok_or("no '-' after the board")?;
    if board.is_empty() || !board.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) {
        return Err("the board is not lowercase letters and digits");
    }
    let digits = |s: &str, len: Option<usize>| {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && len.map_or(true, |len| s.len() == len)
    };
    match date_build.split('.').collect::<Vec<_>>().as_slice() {
        [yy, mm, dd, n] if digits(yy, Some(2)) && digits(mm, Some(2)) && digits(dd, Some(2)) && digits(n, None) => {
            Ok(())
        }
        _ => Err("the date and build are not yy.mm.dd.n"),
    }
}

#[cfg(test)]
mod tests {
    use super::{check_version, main_project, DEFAULT_MAIN_PROJECT};

    #[test]
    fn without_the_variable_the_name_is_micropython() {
        assert_eq!(main_project(None), Ok("micropython"));
        assert_eq!(DEFAULT_MAIN_PROJECT, "micropython");
    }

    #[test]
    fn an_empty_variable_is_no_variable() {
        assert_eq!(main_project(Some("")), Ok("micropython"));
    }

    #[test]
    fn the_variable_names_the_project() {
        assert_eq!(main_project(Some("scrivo-main")), Ok("scrivo-main"));
        assert_eq!(main_project(Some("a".repeat(31).as_str())), Ok("a".repeat(31).as_str()));
    }

    #[test]
    fn a_name_the_description_cannot_hold_or_the_recovery_image_cannot_read_as_a_word_is_refused() {
        for bad in ["a".repeat(32), "two words".to_string(), "tab\there".to_string(), "имя".to_string()] {
            assert!(main_project(Some(bad.as_str())).is_err(), "{bad:?} passed");
        }
    }

    #[test]
    fn version_of_the_domain_rule_passes() {
        assert_eq!(check_version("stragabases3-26.09.19.3"), Ok(()));
        assert_eq!(check_version("g12burner-26.09.13.12"), Ok(()));
    }

    #[test]
    fn broken_versions_are_refused() {
        for bad in [
            "",
            "stragabases3",
            "-26.09.19.1",
            "StragaBases3-26.09.19.1",
            "straga_s3-26.09.19.1",
            "stragabases3-26.9.19.1",
            "stragabases3-26.09.19",
            "stragabases3-26.09.19.x",
            "stragabases3-26.09.19.",
            "stragabases3-26.09.19.1.2",
            "stragabases3-26.09.19.123456789012345",
        ] {
            assert!(check_version(bad).is_err(), "{bad:?} passed");
        }
    }
}
