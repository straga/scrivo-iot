//! The image's application description, esp_app_desc_t. IDF defines a weak one for its C project,
//! "libespidf" with a git version; `app_desc!()` in a device's main.rs defines the strong one that takes its
//! place, as esp_idf_sys::esp_app_desc! would, with the device's version and project name (APP_VERSION and
//! APP_PROJECT_NAME from its build.rs, crates/version). The ELF hash is filled in by `esptool elf2image
//! --elf-sha256-offset` (tools/build.sh), the way IDF does for its own images.

pub use esp_idf_svc::sys;

const fn c_array<const N: usize>(s: &str) -> [core::ffi::c_char; N] {
    let bytes = s.as_bytes();
    assert!(bytes.len() < N);
    let mut out = [0 as core::ffi::c_char; N];
    let mut i = 0;
    while i < bytes.len() {
        out[i] = bytes[i] as core::ffi::c_char;
        i += 1;
    }
    out
}

const fn log2(mut n: u32) -> u8 {
    let mut bits = 0;
    while n > 1 {
        n >>= 1;
        bits += 1;
    }
    bits
}

/// The description of an image of the given version and project name. The recovery image in factory accepts an upload
/// for ota_0 only under the project name it was built with (scrivo-safe/main/main.c, FIRMWARE_PROJECT), so the name
/// is the build's setting SCRIVO_MAIN_PROJECT (crates/version), the same word as SCRIVO_SAFE_MAIN_PROJECT there; the
/// board the image is for is in the version prefix.
pub const fn describe(version: &str, project_name: &str) -> sys::esp_app_desc_t {
    // SAFETY: esp_app_desc_t is plain data; every field is set below or is a reserved zero.
    let mut desc: sys::esp_app_desc_t = unsafe { core::mem::zeroed() };
    desc.magic_word = sys::ESP_APP_DESC_MAGIC_WORD;
    desc.version = c_array(version);
    desc.project_name = c_array(project_name);
    desc.idf_ver = c_array(sys::const_format::formatcp!(
        "v{}.{}.{}",
        sys::ESP_IDF_VERSION_MAJOR,
        sys::ESP_IDF_VERSION_MINOR,
        sys::ESP_IDF_VERSION_PATCH
    ));
    desc.min_efuse_blk_rev_full = sys::CONFIG_ESP_EFUSE_BLOCK_REV_MIN_FULL as u16;
    desc.max_efuse_blk_rev_full = sys::CONFIG_ESP_EFUSE_BLOCK_REV_MAX_FULL as u16;
    desc.mmu_page_size = log2(sys::CONFIG_MMU_PAGE_SIZE);
    desc
}

/// Written once in a device's main.rs: its esp_app_desc_t, with the APP_VERSION and APP_PROJECT_NAME of that device.
#[macro_export]
macro_rules! app_desc {
    () => {
        #[no_mangle]
        #[used]
        #[link_section = ".rodata_desc"]
        #[allow(non_upper_case_globals)]
        pub static esp_app_desc: $crate::sys::esp_app_desc_t = $crate::describe(env!("APP_VERSION"), env!("APP_PROJECT_NAME"));
    };
}
