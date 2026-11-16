//! Which access point a station that cannot join tries next, and when it resets its radio and restarts the board. No
//! radio here: the scan and the failures arrive from the Wi-Fi task, and this says what to do about them.

use roam::Found;

/// The numbers the station climbs by.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    /// How many joins in a row may fail at one access point before it is put off.
    pub tries: u32,
    /// How many laps over the network's access points may end without a join before the radio is reset.
    pub reset_laps: u32,
    /// How many laps more after the reset before the board restarts.
    pub reboot_laps: u32,
}

/// What the station does now.
#[derive(Clone, Debug, PartialEq)]
pub enum Next {
    /// Join at this access point.
    Try(Found),
    /// The scan shows no access point of the network: join and let the driver look for itself.
    Any,
    ResetRadio,
    Restart,
}

/// The station's way out of a network it cannot join: made at the first failure, forgotten at a join.
///
/// **A STATION THAT CANNOT JOIN TRIES THE SAME JOIN FOR HOURS UNLESS SOMETHING ENDS IT.** 05.10 the burner's radio broke
/// the handshake at the near access point from 04:25 on, and the driver, which takes the strongest access point at every
/// join, took that one every ~74 s for four hours. So a join that fails `tries` times in a row at an access point puts it
/// off, and the next try goes to the next strongest of the network; when every access point the scan shows is put off,
/// that was a lap, and the next one begins with every point back. Laps without a join are the ladder's rungs: the radio
/// is reset after `reset_laps`, the board restarts after `reboot_laps` more. Counted in tries and not in minutes: a try
/// takes as long as the router makes it take.
#[derive(Default)]
pub struct Rescue {
    /// Failures in a row at each access point the station tried.
    failed: Vec<([u8; 6], u32)>,
    put_off: Vec<[u8; 6]>,
    laps: u32,
    on: Rung,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Rung {
    #[default]
    Radio,
    Board,
    Done,
}

impl Rescue {
    pub fn new() -> Self {
        Self::default()
    }

    /// The next step, out of the scan FOUND for the network SSID: a rung of the ladder when the laps are done, else an
    /// access point of the network - the strongest not put off, the one listed first when signals are equal.
    pub fn next(&mut self, ssid: &str, found: &[Found], r: &Rules) -> Next {
        if let Some(step) = self.step(r) {
            return step;
        }
        let seen: Vec<&Found> = found.iter().filter(|f| f.ssid == ssid).collect();
        if seen.is_empty() {
            return Next::Any;
        }
        let mut best = self.strongest(&seen);
        if best.is_none() {
            // every access point of the network is put off: a lap is over
            self.lap();
            if let Some(step) = self.step(r) {
                return step;
            }
            best = self.strongest(&seen);
        }
        best.map_or(Next::Any, |f| Next::Try(f.clone()))
    }

    /// The join failed: at BSSID when `next` named one, at none when it said `Any` - a network the scan does not show has
    /// no access points to go round, so each failure there is a lap. -> whether this failure put the access point off.
    pub fn failed(&mut self, bssid: Option<[u8; 6]>, r: &Rules) -> bool {
        let Some(bssid) = bssid else {
            self.lap();
            return false;
        };
        let count = match self.failed.iter_mut().find(|(b, _)| *b == bssid) {
            Some(entry) => {
                entry.1 += 1;
                entry.1
            }
            None => {
                self.failed.push((bssid, 1));
                1
            }
        };
        let off = count >= r.tries && !self.put_off.contains(&bssid);
        if off {
            self.put_off.push(bssid);
        }
        off
    }

    /// The station joined: every count starts over.
    pub fn joined(&mut self) {
        *self = Self::default();
    }

    fn strongest<'a>(&self, seen: &[&'a Found]) -> Option<&'a Found> {
        let mut best: Option<&Found> = None;
        for f in seen.iter().filter(|f| !self.put_off.contains(&f.bssid)) {
            if best.map_or(true, |b| f.rssi > b.rssi) {
                best = Some(f);
            }
        }
        best
    }

    fn lap(&mut self) {
        self.laps += 1;
        self.put_off.clear();
        self.failed.clear();
    }

    /// The rung the laps have reached, taken once: the count of laps starts over after it.
    fn step(&mut self, r: &Rules) -> Option<Next> {
        match self.on {
            Rung::Radio if self.laps >= r.reset_laps => {
                self.laps = 0;
                self.on = Rung::Board;
                Some(Next::ResetRadio)
            }
            Rung::Board if self.laps >= r.reboot_laps => {
                self.on = Rung::Done;
                Some(Next::Restart)
            }
            _ => None,
        }
    }
}
