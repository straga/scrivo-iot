//! Ways down to the recovery image in factory that need no network: the board's button held 10 s and released, and
//! three starts in a row each shorter than 10 s. The C is this crate's own (crates/ways_down/ways_down), built
//! as the ways_down component of this crate; the command `down` of the shell is the third way.

/// Starts both ways: counts this start and starts the task that watches the button. Call once, first thing in
/// main, with nvs up (EspDefaultNvsPartition::take) and before any thread of the firmware: a start that hangs later
/// is still counted, and nothing of the firmware can keep the button from being seen.
pub fn start() {
    unsafe { esp_idf_svc::sys::ways_down::ways_down_start() }
}
