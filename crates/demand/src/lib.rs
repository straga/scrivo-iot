//! WHETHER THE HOUSE ASKS FOR HEAT: the word of the demand - cold or warm - and why, and the players that say it. The boiler reads the word by its role and does not know who plays it; a room against its target
//! and a schedule against the clock are the two players here, and a contact on a pin or a command over the network will be a third without a line of the boiler changed.
//! NEVER HERE: the water and what the fire does with a word (crates/thermostat, crates/automaton), a sensor being read or the time of the board being asked (the device's modules), a word of the page (page/menu.json).

use schedule::{Profile, Schedule, Source, Wall};

/// What the house says to the boiler.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Demand {
    /// The house is cold: the boiler works by its own water.
    Cold,
    /// The house is warm: the boiler goes the ordinary way to a stop.
    Warm,
}

/// Both words by the name a client reads them by.
pub const DEMANDS: [&str; 2] = ["cold", "warm"];

/// The name of a word of the demand, from [`DEMANDS`].
pub fn demand_word(demand: Demand) -> &'static str {
    match demand {
        Demand::Cold => DEMANDS[0],
        Demand::Warm => DEMANDS[1],
    }
}

/// Why the house says what it says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Why {
    /// The room is under its target by the hysteresis, or came down to it and has not risen to the target since.
    BelowTarget,
    /// The room reached its target and has not fallen under it by the hysteresis since.
    TargetReached,
    /// The profile that stands has no target for the room: nothing to hold.
    ProfileOff,
    /// The schedule stands on a profile that burns.
    ScheduledOn,
    /// The schedule stands on the profile that does not.
    ScheduledOff,
    /// The source did not answer for longer than its role holds a number: cold, because a house without heat is worse than heat too much.
    SourceSilent,
    /// The room has not been read once since the board came up: nothing is known of the house yet. It says the word `warm` so that the boiler is not lit on a guess, and the thermostat knows the reason: the water is left to the floor of the boiler alone (`crates/thermostat`).
    NotRead,
    /// The sensors have not been asked all round once since the board came up: the room may be unread only because its turn has not come. It says the word `warm`, which the page shows; that nothing is lit before the round is the thermostat's barrier for every source of the demand (`crates/thermostat`), and once the round has gone through the room's word stands, or `NotRead` for a room
    /// that did not answer.
    Starting,
}

/// Every reason by the name a client reads it by.
pub const WHYS: [&str; 8] = ["below_target", "target_reached", "profile_off", "scheduled_on", "scheduled_off", "source_silent", "not_read", "starting"];

/// The name of a reason, from [`WHYS`].
pub fn why_word(why: Why) -> &'static str {
    match why {
        Why::BelowTarget => WHYS[0],
        Why::TargetReached => WHYS[1],
        Why::ProfileOff => WHYS[2],
        Why::ScheduledOn => WHYS[3],
        Why::ScheduledOff => WHYS[4],
        Why::SourceSilent => WHYS[5],
        Why::NotRead => WHYS[6],
        Why::Starting => WHYS[7],
    }
}

/// What a player says: the word and its reason.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Asked {
    pub demand: Demand,
    pub why: Why,
}

impl Asked {
    /// The two words as one small number for a card that keeps them in a byte: nought is nothing asked, so a word is never nought.
    pub fn code(self) -> u8 {
        let demand = DEMANDS.iter().position(|w| *w == demand_word(self.demand)).unwrap_or(0);
        let why = WHYS.iter().position(|w| *w == why_word(self.why)).unwrap_or(0);
        (demand * WHYS.len() + why) as u8 + 1
    }

    /// The words a [`code`](Self::code) stands for; none for nought and for a number no word makes.
    pub fn of_code(code: u8) -> Option<Asked> {
        let at = usize::from(code.checked_sub(1)?);
        let demand = match at / WHYS.len() {
            0 => Demand::Cold,
            1 => Demand::Warm,
            _ => return None,
        };
        let why = [Why::BelowTarget, Why::TargetReached, Why::ProfileOff, Why::ScheduledOn, Why::ScheduledOff, Why::SourceSilent, Why::NotRead, Why::Starting][at % WHYS.len()];
        Some(Asked { demand, why })
    }
}

/// Who plays the role of the demand.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum By {
    /// Nobody: the boiler works from button to button by its water alone.
    Nobody,
    /// A room sensor against the target the profile that stands names.
    Room,
    /// The schedule itself: a profile that burns says cold, the one that does not says warm.
    Schedule,
}

/// The numbers the players say by.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tuning {
    pub by: By,
    /// The target of the room under the comfort profile and under the eco one; the off profile has none.
    pub room_c: f32,
    pub room_eco_c: f32,
    /// How far under its target the room may fall before it says cold: the room's own hysteresis, not the water's.
    pub room_drop_c: f32,
    pub schedule: Schedule,
}

/// What the players said this second, with what the page shows beside it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Verdict {
    /// None - nobody plays the role.
    pub asked: Option<Asked>,
    /// The profile that stands and what chose it, and the minute of the day the window in force ends at.
    pub profile: Profile,
    pub source: Source,
    pub until: Option<u16>,
    /// The target of the room now: none outside the source `room` and under a profile with none.
    pub target_c: Option<f32>,
}

/// The players of the role, with the memory the room's band needs.
///
/// **THE BAND KEEPS WHAT WAS SAID, BECAUSE A SINGLE READING CANNOT SAY IT**: between the target less the hysteresis and the target a room that came up from the cold is still cold and one that came down from the warm is still warm, as
/// the water's band is said by the fire that remembers it. **THE MEMORY STARTS EMPTY**: a board that comes up knows nothing of the house, so a room read inside the band for the first time is warm - a restart does not light the
/// burner - and a room not read once since the board came up asks for nothing, neither warm nor cold ([`Why::NotRead`], said with the word `warm`). **THE BOUNDARY IS AN EVENT AND NOT A TIME**: until the sensors have been asked all round once
/// ([`first_round_done`](Self::first_round_done)) nothing that lights is said whatever the room reads ([`Why::Starting`]), and from then on a room that did not answer is a dead sensor ([`Why::NotRead`]). Once the room has spoken,
/// a silent room sets the memory cold, since that is what the boiler was told.
pub struct Player {
    /// None - the room has not been read since the board came up.
    cold: Option<bool>,
    /// Whether the sensors have been asked all round once since the board came up.
    round: bool,
}

impl Player {
    pub fn new() -> Self {
        Player { cold: None, round: true }
    }

    /// A player of a board that has just come up: it waits for the first full round of the sensors ([`first_round_done`](Self::first_round_done)) before it says anything that lights.
    pub fn after_boot() -> Self {
        Player { cold: None, round: false }
    }

    /// The sensors have been asked all round once: the room's silence from here on is the room's.
    pub fn first_round_done(&mut self) {
        self.round = true;
    }

    /// What the house says now: ROOM_C is what the room's role holds (none - it cannot say), WALL the local time of the board (none - it does not know it, and the main profile stands).
    pub fn ask(&mut self, t: &Tuning, room_c: Option<f32>, wall: Option<Wall>) -> Verdict {
        let (profile, source, until) = t.schedule.pick(wall);
        let target_c = match (t.by, profile) {
            (By::Room, Profile::Comfort) => Some(t.room_c),
            (By::Room, Profile::Eco) => Some(t.room_eco_c),
            (By::Room, Profile::Off) | (By::Nobody | By::Schedule, _) => None,
        };
        let asked = match t.by {
            By::Nobody => None,
            By::Schedule => Some(scheduled(profile)),
            By::Room => Some(self.room(profile, target_c, room_c, t.room_drop_c)),
        };
        Verdict { asked, profile, source, until, target_c }
    }

    /// The room against TARGET_C, or nothing to hold under a profile that names no target. **A SILENT ROOM UNDER A PROFILE THAT HOLDS NOTHING ASKS FOR NOTHING**: the person said off.
    fn room(&mut self, profile: Profile, target_c: Option<f32>, room_c: Option<f32>, drop_c: f32) -> Asked {
        let Some(target_c) = target_c else {
            debug_assert_eq!(profile, Profile::Off);
            return Asked { demand: Demand::Warm, why: Why::ProfileOff };
        };
        // **THE ROOM IS NOT HEARD BEFORE THE FIRST ROUND OF THE SENSORS HAS GONE THROUGH**: its turn may not have come, and a reading among the first may be the first of many - the memory is not touched, and nothing that lights is said
        if !self.round {
            return Asked { demand: Demand::Warm, why: Why::Starting };
        }
        let Some(room_c) = room_c else {
            if self.cold.is_none() {
                return Asked { demand: Demand::Warm, why: Why::NotRead };
            }
            self.cold = Some(true);
            return Asked { demand: Demand::Cold, why: Why::SourceSilent };
        };
        let mut cold = self.cold.unwrap_or(false);
        if room_c >= target_c {
            cold = false;
        } else if room_c <= target_c - drop_c {
            cold = true;
        }
        self.cold = Some(cold);
        if cold {
            Asked { demand: Demand::Cold, why: Why::BelowTarget }
        } else {
            Asked { demand: Demand::Warm, why: Why::TargetReached }
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

/// What the schedule says at the profile that stands: only the profile that is off stands, and every other one burns.
fn scheduled(profile: Profile) -> Asked {
    match profile {
        Profile::Off => Asked { demand: Demand::Warm, why: Why::ScheduledOff },
        Profile::Comfort | Profile::Eco => Asked { demand: Demand::Cold, why: Why::ScheduledOn },
    }
}
