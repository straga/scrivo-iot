//! When the station looks for a stronger access point of its own network, and which one it moves to. No radio here:
//! the signal and the access points found arrive from the Wi-Fi driver, and this says what to do with them.

/// An access point a scan found.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub ssid: String,
    pub bssid: [u8; 6],
    pub channel: u8,
    pub rssi: i8,
}

/// The numbers the station roams by.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    /// A signal under this, in dBm, is weak enough to look for another access point.
    pub weak_below: i8,
    /// How often the signal is judged, in seconds.
    pub every_s: f32,
    /// How much stronger, in dB, an access point has to be to be moved to.
    pub gain: i8,
    /// How many moves may fail in a row before the station stops looking until its next join.
    pub tries: u32,
}

/// One link's roaming: made at a join, dropped when the link is lost.
///
/// **A STATION THAT JOINED A WEAK ACCESS POINT STAYS ON IT UNLESS IT LOOKS AGAIN.** 27.09 the burner took the far access
/// point of its network while the near one was restarting, and sat on -93 dBm the whole morning: the driver picks the
/// strongest at a join and never after. So the signal is judged every `every_s`, and a weak one is scanned for.
///
/// **A FAILED MOVE COUNTS, A SCAN THAT FOUND NOTHING DOES NOT**: an access point that is off for an hour is found by the
/// scan after it came back, and a station that stopped after three empty scans would sit where the burner sat.
#[derive(Default)]
pub struct Roam {
    since_s: f32,
    failed: u32,
}

impl Roam {
    pub fn new() -> Self {
        Self::default()
    }

    /// A stretch of SECS with the signal at RSSI: whether to scan now. The period counts from the last judgement, weak or
    /// not, and a station whose moves failed `tries` times in a row judges nothing until its next join.
    pub fn second(&mut self, rssi: i8, secs: f32, r: &Rules) -> bool {
        if self.failed >= r.tries {
            return false;
        }
        self.since_s += secs;
        if self.since_s < r.every_s {
            return false;
        }
        self.since_s = 0.0;
        rssi < r.weak_below
    }

    /// The access point to move to out of FOUND: the strongest of the same network other than the own one at BSSID, and
    /// only when it is stronger than the own RSSI by `gain` or more.
    pub fn choose(&self, ssid: &str, bssid: [u8; 6], rssi: i8, found: &[Found], r: &Rules) -> Option<Found> {
        found
            .iter()
            .filter(|f| f.ssid == ssid && f.bssid != bssid)
            .max_by_key(|f| f.rssi)
            .filter(|f| i16::from(f.rssi) - i16::from(rssi) >= i16::from(r.gain))
            .cloned()
    }

    /// A move was made: OK - the station holds the chosen access point now, and the failures are cleared.
    pub fn moved(&mut self, ok: bool) {
        self.failed = if ok { 0 } else { self.failed + 1 };
    }
}
