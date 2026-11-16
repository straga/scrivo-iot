//! What a board calls itself on the network: the device's name and the tail of its station MAC - `burner-f950` -, which DHCP shows in the router's list
//! and mDNS answers to as `burner-f950.local`. The recovery image names itself `scrivo-safe-<the same tail>` (scrivo-safe/main/host_name.c), so one board is one
//! tail in both images. No radio here: the firmware asks the driver for the MAC and hands it over.

/// The longest name a board gives itself, in bytes: what the DHCP client's hostname holds (esp-idf-svc, `heapless::String<30>`, which crates/firmware names by this number so that the two
/// cannot part). A DNS label may be 63, but one name serves DHCP and mDNS both, and a name that DHCP cannot take would leave the board with neither.
pub const NAME_MAX: usize = 30;

/// The hyphen and the four digits of the tail: what a name keeps whatever the device is called.
const TAIL: usize = 5;

/// The last two bytes of the station MAC as four lower-case digits, the form the recovery image writes (`%02x%02x` of bytes 4 and 5) and the router lists.
pub fn tail(mac: [u8; 6]) -> String {
    format!("{:02x}{:02x}", mac[4], mac[5])
}

/// The device's name as a host name's label: small letters, digits and hyphens, an underscore a hyphen (a host name has none; ESPHome does the same). None for a name that
/// cannot be one - empty, any other character, a hyphen at either end: a name is not made up for a device that has none.
fn label(device: &str) -> Option<String> {
    let label: String = device.chars().map(|c| if c == '_' { '-' } else { c.to_ascii_lowercase() }).collect();
    let characters = label.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    (!label.is_empty() && characters && !label.starts_with('-') && !label.ends_with('-')).then_some(label)
}

/// `<device>-<tail>`: what the board calls itself in DHCP and mDNS, at most [`NAME_MAX`] bytes. A device whose name leaves less room than that is CUT, so that the tail of the MAC - the one
/// thing that says which board this is - stays; a hyphen the cut leaves at the end goes with it. None when the device's name is no host name.
pub fn of(device: &str, mac: [u8; 6]) -> Option<String> {
    let mut label = label(device)?;
    // the label is ASCII, a byte is a character
    label.truncate(NAME_MAX - TAIL);
    while label.ends_with('-') {
        label.pop();
    }
    Some(format!("{label}-{}", tail(mac)))
}
