//! THE WATER LAYER OF THE HEATING: whether the boiler's own water asks for burning, and on which level - the held water against the setpoint and its hysteresis - and a person's Start and Stop, said as the order the burner's fire
//! carries out (`automaton::Order`), with the word of the house laid over the water. Temperatures, the house's word and presses in, an order out, and nothing here knows a pin, a relay or a bus: the sensors are the device's half,
//! and how the fire burns is `crates/automaton`. What the page calls "the thermostat" is the floor above this one: whether the house is cold or warm, and who says it (`crates/demand`, the role `demand`).
//! NEVER HERE: a state of the fire (a purge, a kindling, a smouldering), a protection of the boiler (overheating, the alarm), the pump - they guard the boiler and put the fire out, they do not ask for heat -, a room's
//! target, a window of the clock, who plays the demand (`crates/demand`, `crates/schedule`).
//!
//! **THE HOUSE'S WORD IS LAID OVER THE WATER BEFORE THE LOCKOUT**: warm is `Enough` whatever the water says, the ordinary way down to a stop; cold is the water's own band, and where the water says nothing of burning (the
//! mode of power) it is `Burn`; nobody playing the role leaves the water as it was. The lockout of a light judges the word after this, so a light the house asks for is withheld like one the water asks for. A silent held sensor
//! stays silent under either word: it is a protection, and not a request.
//! A room not read once since the board came up is the third case: it says neither word, and the water is left to the floor of the boiler alone (`demanded`). And before the sensors have gone through their first round NO source of the demand lights a stop,
//! the floor included: the order is made in `order` with the barrier of `waiting_for_first_round` over it - a barrier on a light from a stop, and on nothing that burns.
//!
//! **THE DECISIONS WERE THE FIRE'S UNTIL 6.10, AND THIS CRATE TAKES THEM OUT WHOLE.** Three places in the automaton compared the held
//! water with the setpoint (a stop lighting, a working fire going down, a smouldering one working again) and one asked whether the
//! held sensor was silent; what is left in the automaton is what the fire does when it is told. The numbers - the setpoint, the
//! hysteresis, the mode and the level - are the same settings under the same keys, on the card of this module.
//!
//! **THE WATER IS SAID AS A BAND, NOT AS A DECISION WITH A MEMORY.** Under the setpoint less the hysteresis the order is `Burn`, from
//! the setpoint up it is `Enough`, and between the two it is `Keep`: the fire is what remembers, because a fire lit on warm
//! water has to stay lit through the band, and an order that carried `Enough` down into it would put such a fire to smoulder.
//!
//! **THE LEVEL GOES DOWN WITH THE DISTANCE AND COMES BACK WITH A MARGIN** (Plum's MAX, MED and MIN). In a mode by a sensor
//! the order carries the big level while the water is under the setpoint by more than H2, the middle one from H2 to H1 and the small
//! one under H1; a level goes down the second the water is nearer than its line, and comes back up only once the water is farther
//! than the line by the hysteresis, so water standing on a line does not move the fire between two levels. That memory is the one
//! thing besides the presses this crate keeps, because "farther by a margin" cannot be said of a single reading.
//!
//! **A LIGHT IS A CYCLE TO THE NOMINAL OF WHOEVER HOLDS THE BURNER, AND NOT JUST A SPARK**: the room's target where the room is the source of the demand, the held water's setpoint everywhere else. The cycle is a flag of this crate, laid LAST over
//! the order - over the house's word, the band of the water, the wait after a fire that went out and the barrier of the first round - and it ends at the nominal, when the fire that burned has gone out, and on a person's Stop. A silent held sensor
//! is not overruled: that is a protection. A Light when the nominal is reached already is refused with the numbers and counted for nothing. The mode of power has no nominal: a Light there lights, as it always did.
//!
//! **A PRESS IS A COUNT, NOT A PULSE.** The thermostat counts the presses of Start, Stop and Light and the fire compares the counts with the
//! ones it last took (`Presses::since`): a Start is the switch, and the water lights the fire; a Light is "light now, whatever the water says"; a Stop is a burning off and the
//! acknowledgement of a fault, and neither is lost when the second that should have taken it was busy.

pub mod memory;
pub use memory::{from_word, kept_switch, switch_kept, Settled, FROMS, SWITCH_CARD};

use automaton::door::{a_number, above, above_nought, below, not_below_nought};
use automaton::{Heard, Held, Order, Presses, Refused, Step, Switch, Water};

/// What the burner holds: the temperature of a sensor it keeps at the setpoint, or the power it burns at without one. A
/// person chooses it on the card; the protections of the boiler look at the boiler in every one of them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hold {
    /// No sensor: the burner burns on the chosen level until it is told to stop.
    Power,
    /// The water at the top of the boiler.
    Boiler,
    /// The water coming back.
    Return,
}

/// What the thermostat decides by.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tuning {
    /// What the burner holds; see [`Hold`].
    pub hold: Hold,
    /// The water the burner is held at, and how far it may fall before the fire lights again.
    ///
    /// The setpoint above nothing; the hysteresis above nothing and BELOW the setpoint, because what the order watches is their
    /// difference: as wide as the setpoint it puts the relighting line at nought degrees, and wider still puts it under.
    pub want_c: f32,
    pub hysteresis_c: f32,
    /// The level the fire is carried on at work in a mode by a sensor: the chosen row of the table of steps.
    pub step: Step,
    /// The level in the middle of the way up to the setpoint, and the one nearest it: the rows of the table of steps a person
    /// chose for them, or the chosen step itself when none was chosen - then the three are one level and nothing steps down.
    pub mid: Step,
    pub small: Step,
    /// How far under the setpoint the middle level begins (H2) and the small one (H1), H1 under H2: the fire steps down when the water is
    /// nearer than these, and comes back up when it is farther than they are by `hysteresis_c`.
    pub mid_c: f32,
    pub small_c: f32,
    /// The level the burner burns at in [`Hold::Power`], and no other mode. Sound in every mode: a level nobody can burn on is a card
    /// nobody should be able to save, whichever mode it holds at the moment.
    pub power: Step,
    /// The lockout of a light after the fire has gone out.
    pub lockout: Lockout,
}

/// How long after the fire goes out the thermostat does not light it again by itself, and how cold the held water may get before it does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Lockout {
    /// Seconds from the end of burning; nought - no lockout.
    pub wait_s: f32,
    /// The boiler's water under this lights the fire at once, in every mode and whatever is left of the wait: waiting longer would let the boiler go cold.
    pub floor_c: f32,
    /// Seconds a steady flame has to stand without a break to be a fire that must rest: the automaton's own measure of a flame it believes (`automaton::Tuning::sure_s`), so that the two agree on when there is a fire. Nought - one
    /// look of a steady flame is enough.
    pub sure_s: f32,
}

impl Default for Lockout {
    /// No lockout, and the floor the stage names: the thermostat as it was.
    fn default() -> Self {
        Lockout { wait_s: 0.0, floor_c: 30.0, sure_s: 0.0 }
    }
}

/// What the fire is doing, as the thermostat needs it: whether it is out, and the seconds since the thermostat last looked.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Fire {
    /// The automaton stands in Stop. False - anything else, and for a reading that says nothing of the fire.
    pub out: bool,
    /// The flame sensor sees a steady flame this second. Not the automaton's state: a purge, a kindling that did not take and a fault are not burning.
    pub steady: bool,
    /// The seconds this reading stands for: the real time since the last one. Nought for a second look at the same second.
    pub elapsed_s: f32,
}

/// Tuning that has been through the door, and the only one the thermostat decides by.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sound(Tuning);

impl Sound {
    /// The numbers inside, for whoever shows them to a person.
    pub fn numbers(&self) -> &Tuning {
        &self.0
    }
}

impl Tuning {
    /// The door: the first refusal, in the order the fields are written, as `automaton::Tuning::sound` asks its own.
    ///
    /// **THE OVERHEATING IS NOT THIS RECORD'S, AND ITS ORDER AGAINST THE SETPOINT IS**: a burner that overheats before it reaches the
    /// water it is set to hold can never hold it, so the fire's `too_hot_c` is handed in to be held against the setpoint, and a
    /// refusal names it as the field, as it always did. It is a number before an order, and it was asked of the fire's own door.
    pub fn sound(self, too_hot_c: f32) -> Result<Sound, Refused> {
        above_nought("want_c", self.want_c)?;
        above_nought("hysteresis_c", self.hysteresis_c)?;
        below("hysteresis_c", self.hysteresis_c, "want_c", self.want_c)?;
        above_nought("mid_c", self.mid_c)?;
        above_nought("small_c", self.small_c)?;
        below("small_c", self.small_c, "mid_c", self.mid_c)?;
        not_below_nought("lock_s", self.lockout.wait_s)?;
        above_nought("lock_floor_c", self.lockout.floor_c)?;
        a_number("too_hot_c", too_hot_c)?;
        above("too_hot_c", too_hot_c, "want_c", self.want_c)?;
        self.step.sound("step")?;
        self.mid.sound("mid")?;
        self.small.sound("small")?;
        self.power.sound("power")?;
        Ok(Sound(self))
    }
}

/// What the thermostat is given each second: the two temperatures it may hold by, whether the one it holds only on request is still being heard, and what the house says. **NONE IS NEVER A ZERO**: a sensor that said nothing is
/// not a sensor at 0 °C.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Temperatures {
    /// The water of the boiler, in degrees. None - the bus said nothing about it this round.
    pub boiler_c: Option<f32>,
    /// The water coming back. None - no socket plays the role, or the bus said nothing about it.
    pub return_c: Option<f32>,
    /// Whether the return's round is still publishing (`module::Quiet`): why `return_c` is None, when it is.
    pub return_heard: Heard,
    /// What the house says to the boiler: none - nobody plays the role of the demand, and the boiler works by its water alone.
    pub asked: Option<demand::Asked>,
    /// What the fire is doing, and for how long this reading stands.
    pub fire: Fire,
    /// The room as its role reads it, and the target the house holds it at (`demand::Verdict::target_c`): none where the role cannot say, and none outside the source `room` and under a profile with no target. A manual cycle is carried to the
    /// room's target where the room is the source, and to the water's setpoint where it is not.
    pub room_c: Option<f32>,
    pub room_target_c: Option<f32>,
}

/// A button of the card: Start and Stop, the main switch over everything the thermostat will come to ask for, and Light, the fire lit now past the thermostat.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Press {
    Start,
    Stop,
    Light,
}

/// What a manual cycle (a person's Light) is carried to: the target of the room, where the room is the source of the demand, and the setpoint of the held water otherwise (the water alone, the schedule, a profile that holds no room).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nominal {
    Room,
    Water,
}

/// Every nominal by the name a client reads it by.
pub const NOMINALS: [&str; 2] = ["room", "water"];

/// The name of a nominal, from [`NOMINALS`].
pub fn nominal_word(nominal: Nominal) -> &'static str {
    NOMINALS[nominal as usize]
}

/// A manual cycle that is going: what it is carried to, and the number it ends at, in degrees.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Cycle {
    pub nominal: Nominal,
    pub to_c: f32,
}

/// A Light refused because the nominal it would carry the fire to is reached already: what it is, where the reading stands and where the cycle would end, in degrees.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Refusal {
    pub nominal: Nominal,
    pub now_c: f32,
    pub to_c: f32,
}

impl Refusal {
    /// The refusal as the board says it to a page: a code (by the nominal) and the two numbers it stands on, one digit after the point. **DATA AND NOT A SENTENCE**: the words of it are the page's own, in the language it speaks.
    pub fn wire(&self) -> String {
        format!("light_refused_{} now={:.1} to={:.1}", nominal_word(self.nominal), self.now_c, self.to_c)
    }

    /// The same for the journal, in words.
    pub fn line(&self) -> String {
        match self.nominal {
            Nominal::Room => format!("a Light was refused: the room is at {:.1} °C, its target is {:.1} °C", self.now_c, self.to_c),
            Nominal::Water => format!("a Light was refused: the water is at {:.1} °C, its setpoint is {:.1} °C", self.now_c, self.to_c),
        }
    }
}

impl Cycle {
    /// What the journal says when a cycle begins.
    pub fn began_line(&self) -> String {
        match self.nominal {
            Nominal::Room => format!("a Light begins a cycle to the room's target of {:.1} °C", self.to_c),
            Nominal::Water => format!("a Light begins a cycle to the water's setpoint of {:.1} °C", self.to_c),
        }
    }
}

impl Ended {
    /// What the journal says when a cycle ends, and who ended it.
    pub fn line(&self) -> &'static str {
        match self {
            Ended::Reached => "the cycle ended: the nominal was reached",
            Ended::Out => "the cycle ended: the fire is out",
            Ended::Stopped => "the cycle ended: a person pressed Stop",
            Ended::Water => "the cycle ended: the water reached its setpoint",
            Ended::Unread => "the cycle ended: the number it was carried by cannot be read",
            Ended::ModeChanged => "the cycle ended: the mode that held the burner was changed",
        }
    }
}

/// Why a manual cycle ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ended {
    /// The nominal was reached: the ordinary way down from here.
    Reached,
    /// The fire burned and went out, by whatever cause - a protection and a burning off included.
    Out,
    /// A person's Stop.
    Stopped,
    /// The held water reached its setpoint while the cycle was carried to the room's target: **THE CEILING OF THE WATER STAYS** - the cycle takes away what holds the fire back (the house's warm, the band, the wait, the barrier) and not the water's `Enough` -, and with the water at its setpoint the
    /// ordinary rules ask for no fire, so the cycle ends and does not sit on over a burner that is not asked.
    Water,
    /// What the cycle was begun by is not what holds the burner now: the mode (the room, the boiler's water, the return's, the power) or the source of the demand was changed under it. The cycle was a Light's word about THAT nominal; a new one is a new Light.
    ModeChanged,
    /// The number the cycle is carried by cannot be read - the room, or the held water -: a cycle that cannot see where it is going does not go on.
    Unread,
}

/// Every end of a cycle by the name the journal and a client read it by.
pub const ENDS: [&str; 6] = ["reached", "out", "stopped", "water", "unread", "mode_changed"];

/// The name of an end, from [`ENDS`].
pub fn ended_word(ended: Ended) -> &'static str {
    ENDS[ended as usize]
}

/// The three levels a mode by a sensor carries the fire up to the setpoint on, the biggest first.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rung {
    Big,
    Mid,
    Small,
}

impl Rung {
    /// Where the level stands from the biggest down: the order the way down is judged in.
    fn place(self) -> u8 {
        match self {
            Rung::Big => 0,
            Rung::Mid => 1,
            Rung::Small => 2,
        }
    }
}

/// Every level of the way up by the name a client reads it by.
pub const RUNGS: [&str; 3] = ["big", "mid", "small"];

/// The name of a level, from [`RUNGS`].
pub fn rung_word(rung: Rung) -> &'static str {
    RUNGS[rung.place() as usize]
}

/// The thermostat a board runs: what a person pressed, and nothing else - the order is made of that and of the readings of the second.
#[derive(Default)]
pub struct Thermostat {
    switch: Switch,
    presses: Presses,
    /// The level the last reading put the fire on, kept for the way up: none in the mode of power and before the first reading.
    rung: Option<Rung>,
    /// How far under the setpoint the held water was in the last order.
    left_c: Option<f32>,
    /// The level as it is told to a person: the middle one is the big or the small one when its row is.
    said: Option<Rung>,
    setpoint_c: Option<f32>,
    /// What the house said in the last order.
    asked: Option<demand::Asked>,
    /// THE LOCKOUT OF A LIGHT: whether the fire was out at the last look - none before the first -, the seconds since it went out, counted from the end of burning the thermostat saw (none where it saw none, or a
    /// Start ended the wait), the wait the last order was made by, and what is left of it.
    fire_was_out: Option<bool>,
    /// A FIRE THAT MUST REST IS ONE THE FLAME SENSOR SAW: the seconds of a steady flame in a row while the automaton was not in Stop, and whether they ever reached `sure_s` since the last wait began. Only a fire that burned starts
    /// the wait at its end; a purge, a kindling that did not take, a fault and a state the board came up in from its memory are not one.
    steady_for_s: f32,
    burned: bool,
    out_for_s: Option<f32>,
    wait_s: f32,
    lock_left_s: Option<f32>,
    /// THE SWITCH IN THE BOARD'S MEMORY: the word the flash TOOK last (none - nothing is kept), and whether the switch stands as the memory gave it and no person has pressed since the board came up.
    kept: Option<&'static str>,
    from_memory: bool,
    /// How many writes of a waiting word the flash has refused in a row, and the seconds since the last try of it.
    refused: u32,
    since_try_s: f32,
    /// Whether the sensors have not been asked all round once since the board came up (see [`waiting_for_first_round`](Thermostat::waiting_for_first_round)).
    waiting: bool,
    /// THE MANUAL CYCLE: what it is carried to, and whether the fire has been seen burning since it began - the second of the press the automaton still stands in Stop (it takes the press in the same breath), and that is not the end of a fire.
    manual: Option<Manual>,
    /// What the last order carried the cycle to, for the card; none while there is none.
    cycle_to: Option<Cycle>,
    /// Why the last cycle ended, until the journal has read it.
    ended: Option<Ended>,
    /// The automaton stands in a fault, which waits for a person: the fire is over for a cycle's purpose, and a cycle is not going on over it.
    faulted: bool,
}

/// A manual cycle in progress.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Manual {
    lit: bool,
    /// WHAT THE CYCLE WAS BEGUN BY: the mode the burner held (the boiler's water, the return's, the power) and the kind of nominal it was carried to (the room's target or the water's setpoint, which is the source of the demand). A Light is a word about THAT nominal: any
    /// other mode or source in a later second ends the cycle (`Ended::ModeChanged`) and does not carry the fire to a number nobody pressed Light for.
    hold: Hold,
    nominal: Nominal,
}

impl Thermostat {
    /// A board that has come up asked nothing: no switch, no presses.
    pub fn new() -> Self {
        Self::default()
    }

    /// A board that has come up and found SWITCH in its memory: the switch the person left, and no press - the counts stay at nought, because a press lights at once whatever the water says and a restart is not the person. The word the flash holds is the one this switch is kept as, so a press that agrees with it is not written again.
    ///
    /// **THE FIRE DECIDES WHAT THE SWITCH DOES**: `On` over a stop lights it when the water asks, `Off` and `Unasked` do not, and a fault waits for a person's Stop whatever the switch says (`automaton::Burning::step`).
    pub fn restored(switch: Switch) -> Self {
        Self { switch, kept: memory::switch_kept(switch), from_memory: switch != Switch::Unasked, ..Self::default() }
    }

    /// This thermostat waits for the first full round of the sensors before it orders a light. Said of a board that has just come up, and ended by [`first_round_done`](Self::first_round_done).
    pub fn waiting_for_first_round(self) -> Self {
        Self { waiting: true, ..self }
    }

    /// The sensors have been asked all round once: what they read, and what they did not answer, is now the board's to decide by.
    pub fn first_round_done(&mut self) {
        self.waiting = false;
    }

    /// The word the flash does not hold yet: the switch as it kept, when the last word the flash TOOK is another. None while they agree, and for a switch nobody has set.
    fn pending(&self) -> Option<&'static str> {
        memory::switch_kept(self.switch).filter(|word| Some(*word) != self.kept)
    }

    /// One try at writing PENDING: **THE WORD IS KEPT ONLY WHEN THE FLASH TAKES IT** (6.16, the review) - a write that came back refused leaves the word waiting, and what the board
    /// thinks the flash holds is what the flash answered, not what was asked of it.
    fn attempt(&mut self, put: &mut dyn FnMut(&str) -> Result<(), String>) -> Settled {
        let Some(word) = self.pending() else {
            // nothing waits: a person who pressed back to what the flash holds has ended the series
            self.refused = 0;
            return Settled::Nothing;
        };
        self.since_try_s = 0.0;
        match put(word) {
            Ok(()) => {
                self.kept = Some(word);
                Settled::Written { after_refusals: std::mem::take(&mut self.refused) }
            }
            Err(why) => {
                let first = self.refused == 0;
                self.refused += 1;
                Settled::Refused { first, why }
            }
        }
    }

    /// The word of a press, written at once through PUT: one try for a press that changed what the flash holds, none for a repeat of it. A refusal leaves the word waiting for [`retry`](Self::retry).
    pub fn write_now(&mut self, put: &mut dyn FnMut(&str) -> Result<(), String>) -> Settled {
        self.attempt(put)
    }

    /// The seconds after a refusal: ELAPSED_S is the time since the last look, and a word that waits is tried again once a second has passed since the last try - never oftener, so a flash that keeps refusing is asked once a second
    /// and not in a storm. **NOTHING TO DO WHILE NOTHING WAITS**: every second of a board whose flash took its word answers [`Settled::Nothing`] and touches nothing. The first refusal of a series says `first`, the writes that follow
    /// it do not, and the one that goes through says how many refusals it ended.
    pub fn retry(&mut self, elapsed_s: f32, put: &mut dyn FnMut(&str) -> Result<(), String>) -> Settled {
        if self.pending().is_none() {
            self.refused = 0;
            return Settled::Nothing;
        }
        if self.refused > 0 {
            self.since_try_s += elapsed_s.max(0.0);
            if self.since_try_s < 1.0 {
                return Settled::Nothing;
            }
        }
        self.attempt(put)
    }

    /// Whether the switch stands as the memory gave it and no person has pressed since the board came up: the card says it, so the person who finds the burner lit or stopped after a power cut knows whose word it is.
    pub fn from_memory(&self) -> bool {
        self.from_memory
    }

    /// Whether a person has asked for heat, as the card shows it.
    pub fn switch(&self) -> Switch {
        self.switch
    }

    /// A press of a button, counted now. The order of the next second carries it.
    pub fn press(&mut self, press: Press) -> Option<f32> {
        self.from_memory = false;
        match press {
            Press::Start => {
                self.switch = Switch::On;
                self.presses.start = self.presses.start.wrapping_add(1);
            }
            Press::Stop => {
                self.switch = Switch::Off;
                self.presses.stop = self.presses.stop.wrapping_add(1);
                if self.manual.is_some() {
                    self.end(Ended::Stopped);
                }
            }
            // a light is not the switch: it counts for the fire alone and leaves the switch where it stood
            Press::Light => self.presses.light = self.presses.light.wrapping_add(1),
        }
        // A LIGHT ENDS THE WAIT FOR GOOD (until the fire goes out again), and says what was left of it so the page's journal can: the person's word is the one thing that lights before the time. A Start is the switch and cuts nothing
        if press == Press::Light {
            let cut = self.out_for_s.map(|out_for| self.wait_s - out_for).filter(|left| *left > 0.0);
            self.out_for_s = None;
            self.lock_left_s = None;
            return cut;
        }
        None
    }

    /// A person's Light, judged against the readings of this moment: refused with the numbers when the nominal it would carry the fire to is reached already - then nothing is counted and the fire is not asked -, and otherwise counted as a
    /// press (what was left of the wait it cut, as [`press`](Self::press) says) with a cycle begun to the nominal. The mode of power has none.
    pub fn light(&mut self, readings: &Temperatures, t: &Sound) -> Result<Option<f32>, Refusal> {
        let nominal = nominal(&t.0, readings);
        if let Some((kind, Some(now_c), to_c)) = nominal {
            if now_c >= to_c {
                return Err(Refusal { nominal: kind, now_c, to_c });
            }
        }
        // **THE CEILING OF THE WATER STANDS FOR A ROOM'S CYCLE TOO**: a cold room over a boiler that is at its setpoint asks for nothing in the ordinary mode, and a Light there would light a fire that the same second's order puts out: refused, by the water
        if let (Some((Nominal::Room, _, _)), Some(water_c)) = (nominal, held(&t.0, readings)) {
            if water_c >= t.0.want_c {
                return Err(Refusal { nominal: Nominal::Water, now_c: water_c, to_c: t.0.want_c });
            }
        }
        let cut = self.press(Press::Light);
        // a cycle needs the number it is carried by and the number it is carried to: without them the Light is a plain light, as it always was
        let readable = nominal.is_some_and(|(kind, now_c, _)| now_c.is_some() && (kind == Nominal::Water || held(&t.0, readings).is_some()));
        self.manual = nominal.filter(|_| readable).map(|(nominal, _, _)| Manual { lit: false, hold: t.0.hold, nominal });
        self.cycle_to = nominal.filter(|_| readable).map(|(nominal, _, to_c)| Cycle { nominal, to_c });
        self.ended = None;
        Ok(cut)
    }

    /// The manual cycle that is going, if one is, as the last order made it: what it is carried to and the number it ends at.
    pub fn cycle(&self) -> Option<Cycle> {
        self.cycle_to
    }

    /// Whether the automaton stands in a fault, said every second by the board: a fault waits for a person's Stop and a cycle over it would say on the card that the fire is being carried to a number it is not burning for.
    pub fn faulted(&mut self, faulted: bool) {
        self.faulted = faulted;
    }

    /// Why the last manual cycle ended, once: the journal reads it and it is gone.
    pub fn ended(&mut self) -> Option<Ended> {
        self.ended.take()
    }

    /// The cycle ends for WHY: the order that follows is the ordinary one.
    fn end(&mut self, why: Ended) {
        self.manual = None;
        self.cycle_to = None;
        self.ended = Some(why);
    }

    /// The seconds left before the thermostat lights the fire by itself again, none where there is no wait: the fire burns, or the wait is over, or there is none set, or the board has seen no fire go out.
    pub fn lock_left_s(&self) -> Option<f32> {
        self.lock_left_s
    }

    /// THE LOCKOUT OF A LIGHT: while the time since the fire went out is under the wait, a light the water asks for is kept - `Burn` becomes `Keep`. The time is counted here, from the second the thermostat is told the fire is out
    /// after being told it burned, by the seconds the readings stand for; the schedule moves the setpoint and not this clock, so a window opening or closing in the middle of the wait changes the order and not the time left.
    /// A fire that is not out is never withheld (a smouldering one works again), the mode of power has no water to withhold unless the house asks, a Start ends the wait, and the boiler's water UNDER the floor lights at once, whatever the burner holds: waiting longer would let the boiler go cold.
    fn locked(&mut self, t: &Tuning, readings: &Temperatures, water: Water) -> Water {
        // a mode that holds no temperature has no water to withhold - until the house asks, and then its light is the one withheld
        let withholds = t.hold != Hold::Power || readings.asked.is_some();
        let fire = readings.fire;
        // the flame is counted in a row while the automaton is not in Stop, and a break in it starts the count again; `sure_s` of it is a flame the automaton itself believes
        if !fire.out && fire.steady {
            self.steady_for_s += fire.elapsed_s.max(0.0);
        } else {
            self.steady_for_s = 0.0;
        }
        if self.steady_for_s > 0.0 && self.steady_for_s >= t.lockout.sure_s {
            self.burned = true;
        }
        match (self.fire_was_out, fire.out) {
            // the end of burning: the count starts here if there was a fire (and spends it), and the seconds of this reading were the burning's
            (Some(false), true) => {
                self.out_for_s = self.burned.then_some(0.0);
                self.burned = false;
            }
            (Some(true), true) => {
                if let Some(out_for) = self.out_for_s.as_mut() {
                    *out_for += fire.elapsed_s.max(0.0);
                }
            }
            (_, false) => self.out_for_s = None,
            // a board that came up with the fire out has seen no end of burning, and counts nothing from the power
            (None, true) => {}
        }
        self.fire_was_out = Some(fire.out);
        self.wait_s = t.lockout.wait_s;
        self.lock_left_s = match self.out_for_s {
            Some(out_for) if fire.out && withholds && t.lockout.wait_s > 0.0 && out_for < t.lockout.wait_s => Some(t.lockout.wait_s - out_for),
            _ => None,
        };
        // THE BOILER'S WATER IN EVERY MODE: the floor says the boiler has gone cold, and the air of the boiler room or the return are far under it all the time
        let cold = readings.boiler_c.is_some_and(|celsius| celsius < t.lockout.floor_c);
        match (water, self.lock_left_s) {
            (Water::Burn, Some(_)) if !cold => Water::Keep,
            _ => water,
        }
    }

    /// THE MANUAL CYCLE, LAID LAST: while it goes the water is `Burn` whatever the house, the band, the wait or the barrier said - what HOLDS THE FIRE BACK, and not the ceiling of the water -, bar a silent held sensor, which is a protection and not a request. It ends,
    /// in this order, with the fire that burned and has gone out (or a fault), with a number it cannot read, with the nominal reached, and with the held water at its setpoint when the cycle is the room's; the second of the press, when the automaton still stands in Stop, is none of these.
    fn cycled(&mut self, t: &Tuning, readings: &Temperatures, water: Water) -> Water {
        let Some(mut cycle) = self.manual else {
            self.cycle_to = None;
            return water;
        };
        // **A CYCLE IS BOUND TO THE MODE AND THE SOURCE IT WAS BEGUN BY**: another mode - or the mode of power, which has no nominal -, or another source of the demand in a later second, and the cycle is over
        let Some((nominal, now_c, to_c)) = nominal(t, readings).filter(|(nominal, _, _)| t.hold == cycle.hold && *nominal == cycle.nominal) else {
            self.end(Ended::ModeChanged);
            return water;
        };
        if !readings.fire.out {
            cycle.lit = true;
            self.manual = Some(cycle);
        }
        if (cycle.lit && readings.fire.out) || self.faulted {
            self.end(Ended::Out);
            return water;
        }
        // a cycle that cannot see the number it is carried by does not go on
        let Some(now) = now_c else {
            self.end(Ended::Unread);
            return water;
        };
        if now >= to_c {
            self.end(Ended::Reached);
            return water;
        }
        // **THE CEILING OF THE WATER STAYS**: the cycle takes away what holds the fire back, and not the water's own `Enough`. A cycle of the room ends with the held water at its setpoint - the ordinary rules ask for no fire there
        // whatever the room says, and a cycle that sat on would sawtooth the burner between the setpoint and the band; the water that cannot be read ends it as the room does.
        if nominal == Nominal::Room {
            match held(t, readings) {
                None => {
                    self.end(Ended::Unread);
                    return water;
                }
                Some(water_c) if water_c >= t.want_c => {
                    self.end(Ended::Water);
                    return water;
                }
                Some(_) => {}
            }
        }
        self.cycle_to = Some(Cycle { nominal, to_c });
        self.lock_left_s = None;
        match water {
            Water::Silent(_) => water,
            _ => Water::Burn,
        }
    }

    /// The level the last order was taken from, none in the mode of power and before the first reading. **A middle whose row is the big
    /// one's or the small one's is not told as a level of its own**: a person reads the level the fire burns on.
    pub fn rung(&self) -> Option<Rung> {
        self.said
    }

    /// How far under the setpoint the held water was in the last order, below nought over it, none when it said nothing.
    pub fn left_c(&self) -> Option<f32> {
        self.left_c
    }

    /// The setpoint of the last order, in degrees.
    pub fn setpoint_c(&self) -> Option<f32> {
        self.setpoint_c
    }

    /// What the house said in the last order: none - nobody plays the role, and none before the first order.
    pub fn asked(&self) -> Option<demand::Asked> {
        self.asked
    }

    /// The order of this second: what the held water says, the house's word laid over it, the level to burn at, the switch and the counts.
    pub fn order(&mut self, readings: &Temperatures, t: &Sound) -> Order {
        let t = &t.0;
        // the mode of power holds no temperature, so no setpoint stands and nothing is said of one
        let by_setpoint = t.hold != Hold::Power;
        self.setpoint_c = by_setpoint.then_some(t.want_c);
        self.asked = readings.asked;
        let water = demanded(t, readings, water(t, readings));
        let water = self.locked(t, readings, water);
        // **BEFORE THE FIRST ROUND OF THE SENSORS NO LIGHT IS ORDERED FROM A STOP, WHOEVER PLAYS THE DEMAND AND WHATEVER THE WATER READS**: the schedule says cold on the first second of a board, nobody playing the role leaves the water alone, and the water
        // may be read before the room has been asked - a stop is lit by none of them yet. **THE BARRIER IS ON THE STOP AND ON NOTHING ELSE** (`readings.fire.out`: the automaton stands in Stop): a fire that is lit, rising, working, smouldering or burning off
        // takes the word of the water as it is, so that a smouldering fire still goes back to work on water that burns. `Keep` and not `Enough`: the barrier takes a light and sends nothing down. A person's Light is not here, it is a press and the fire takes it.
        let water = match water {
            Water::Burn if self.waiting && readings.fire.out => Water::Keep,
            other => other,
        };
        let water = self.cycled(t, readings, water);
        self.left_c = match water {
            Water::Level | Water::Silent(_) => None,
            Water::Burn | Water::Keep | Water::Enough => held(t, readings).map(|celsius| t.want_c - celsius),
        };
        match (t.hold, self.left_c) {
            (Hold::Power, _) => self.rung = None,
            (Hold::Boiler | Hold::Return, Some(left)) => self.rung = Some(next(self.rung.unwrap_or(Rung::Big), left, t)),
            // a reading that is not there leaves the fire on the level it had: not a cold water and not a hot one
            (Hold::Boiler | Hold::Return, None) => {}
        }
        let rung = self.rung.unwrap_or(Rung::Big);
        self.said = self.rung.map(|rung| told(rung, t));
        Order {
            water,
            step: match t.hold {
                Hold::Power => t.power,
                Hold::Boiler | Hold::Return => match rung {
                    Rung::Big => t.step,
                    Rung::Mid => t.mid,
                    Rung::Small => t.small,
                },
            },
            switch: self.switch,
            presses: self.presses,
            house_cold: matches!(readings.asked, Some(demand::Asked { demand: demand::Demand::Cold, .. })),
        }
    }
}

/// The word of the water with the house's laid over it: warm is `Enough` and cold is what the water says, or `Burn` where it says nothing of burning (the mode of power); nobody asking leaves the water as it was. A silent held sensor is
/// said whatever the house says: the protection is not a request.
///
/// **A ROOM NOT READ ONCE SINCE THE BOARD CAME UP SAYS NEITHER WARM NOR COLD** (`Why::NotRead`, which the page shows with the word `warm`): it neither overrules the water with `Enough` nor leaves the water's own band to light a
/// stop, since a restart is not a reason to light on water under the setpoint. What stands is the floor alone - the boiler's water under `Lockout::floor_c` is `Burn`, over it or unread `Enough` - and the floor is the boiler's
/// protection from going cold, whoever is or is not asked for heat.
///
/// **THE FIRST ROUND OF THE SENSORS IS A BARRIER OVER EVERY SOURCE, AND NOT OVER THE ROOM'S WORD** (`order`, `waiting_for_first_round`): a boiler read at 25.9 °C and a room not yet asked was a light on every restart (12.10 14:16), and the schedule
/// says cold on the first second of a board and nobody playing the role leaves the water alone. The room's `Why::Starting` is what the page shows in that time; it asks nothing of the water, and the decision not to light from a stop is not made by it - and the barrier is on a stop, so a fire that burns keeps the word of the water.
fn demanded(t: &Tuning, readings: &Temperatures, water: Water) -> Water {
    match (readings.asked.map(|asked| (asked.demand, asked.why)), water) {
        (_, Water::Silent(_)) | (None, _) => water,
        (Some((demand::Demand::Warm, demand::Why::NotRead)), _) => {
            if readings.boiler_c.is_some_and(|celsius| celsius < t.lockout.floor_c) {
                Water::Burn
            } else {
                Water::Enough
            }
        }
        // the room's word is not heard before the first round: it asks nothing of the water, and a light from a stop is held back by the barrier of `order`, not by this word
        (Some((demand::Demand::Warm, demand::Why::Starting)), _) => water,
        (Some((demand::Demand::Warm, _)), _) => Water::Enough,
        (Some((demand::Demand::Cold, _)), Water::Level) => Water::Burn,
        (Some((demand::Demand::Cold, _)), _) => water,
    }
}

/// The level the water of LEFT degrees under the setpoint puts a fire that was on WAS. **DOWN IS AT THE LINE, UP IS PAST THE LINE BY
/// THE HYSTERESIS**: the big level holds while the water is under by more than H2, the middle one from H2 to H1 and the small one
/// under H1, and a level the water has left is taken back only when it is farther than that line by the margin.
fn next(was: Rung, left: f32, t: &Tuning) -> Rung {
    let by = |margin: f32| {
        if left < t.small_c + margin {
            Rung::Small
        } else if left <= t.mid_c + margin {
            Rung::Mid
        } else {
            Rung::Big
        }
    };
    let (down, up) = (by(0.0), by(t.hysteresis_c));
    if down.place() > was.place() {
        down
    } else if up.place() < was.place() {
        up
    } else {
        was
    }
}

/// The level as a person is told it: a middle that is the big row is the big level, and one that is the small row is the small one.
fn told(rung: Rung, t: &Tuning) -> Rung {
    match rung {
        Rung::Mid if t.mid == t.step => Rung::Big,
        Rung::Mid if t.mid == t.small => Rung::Small,
        Rung::Big | Rung::Mid | Rung::Small => rung,
    }
}

/// What a manual cycle is carried to: the kind, where the reading stands now (none where it cannot say) and the number the cycle ends at. The room's target where the house holds a room (`room_target_c`), the held water's setpoint otherwise; none in the mode of
/// power, which holds no temperature.
fn nominal(t: &Tuning, readings: &Temperatures) -> Option<(Nominal, Option<f32>, f32)> {
    if t.hold == Hold::Power {
        return None;
    }
    match readings.room_target_c {
        Some(target) => Some((Nominal::Room, readings.room_c, target)),
        None => Some((Nominal::Water, held(t, readings), t.want_c)),
    }
}

/// The reading of the sensor the burner holds, none in the mode that holds none or when the bus said nothing.
fn held(t: &Tuning, readings: &Temperatures) -> Option<f32> {
    match t.hold {
        Hold::Power => None,
        Hold::Boiler => readings.boiler_c,
        Hold::Return => readings.return_c,
    }
}

/// What the water of the held sensor says against the setpoint, in the five words of the order.
fn water(t: &Tuning, readings: &Temperatures) -> Water {
    // the boiler's silence is the fire's own, asked in every mode; the return's is asked only while the burner holds it
    if t.hold == Hold::Return && readings.return_heard == Heard::TooLong {
        return Water::Silent(Held::Return);
    }
    if t.hold == Hold::Power {
        return Water::Level;
    }
    match held(t, readings) {
        Some(celsius) if celsius >= t.want_c => Water::Enough,
        Some(celsius) if celsius <= t.want_c - t.hysteresis_c => Water::Burn,
        Some(_) | None => Water::Keep,
    }
}
