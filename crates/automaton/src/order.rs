//! WHAT THE BURNER IS ASKED, second by second: the order the thermostat gives and the fire carries out. Water, a level and a
//! person's switch go in the order; the setpoint, the hysteresis and the mode that made them stay with whoever made it.
//! NEVER HERE: a setpoint, a hysteresis, a mode, a sensor's reading - the fire does not know what the heat is held by.
//!
//! **THE ORDER HAS FIVE WORDS FOR THE WATER, NOT THREE**, because the fire answers differently to each of them and one of them is
//! "do nothing": between the setpoint and the setpoint less the hysteresis a working fire stays working and a smouldering one
//! stays smouldering. An order that remembered its last word through the band would carry `Enough` down into water the fire was
//! lit on and put it to smoulder.

use crate::Step;

/// What the water of the held sensor says about burning. The fire acts on it by the state it is in; the table of what it does is
/// written at [`Burning::step`](crate::Burning::step).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Water {
    /// At or under the setpoint less the hysteresis: a stop lights if heat is asked, a smouldering fire works again.
    Burn,
    /// Inside the hysteresis, or the held sensor says nothing yet: the fire changes nothing.
    Keep,
    /// At or over the setpoint: a working fire goes down to smouldering.
    Enough,
    /// The burner holds no temperature: the level is given and the water is none. A stop is not lit by it, a smouldering fire works
    /// again at once, and a working fire is never sent down.
    Level,
    /// The sensor the burner holds by has been silent too long: the fire is burnt off and the fault names the sensor.
    Silent(Held),
}

/// The sensor a mode holds by, when it is not the boiler's: the boiler's silence is the fire's own, asked in every mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Held {
    Return,
}

/// Whether a person has asked for heat. **A STATE OF THREE AND NOT A SWITCH OF TWO**: a board that has just come up has not been
/// asked anything, and that is not the same as being told to stop - what the page says of it, and what a switch that outlives a
/// restart will have to remember.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Switch {
    /// Nobody has pressed anything since the board came up.
    #[default]
    Unasked,
    /// A person pressed Start and has not pressed Stop since.
    On,
    /// A person pressed Stop and has not pressed Start since.
    Off,
}

/// How many times each button has been pressed since the board came up. **AN EDGE IS A COUNT THAT MOVED**: the order is read
/// by the fire when it takes its second, and a press that is a pulse would be lost by a second that was busy - a count is
/// still there to be compared.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Presses {
    pub start: u32,
    pub stop: u32,
    pub light: u32,
}

/// What a person pressed since the fire last looked: the edge the counts make.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Told {
    Nothing,
    /// Heat is wanted: the switch is on ([`Switch::On`]), and the fire is lit by the water saying `Burn`, now or when it falls. The fire itself
    /// takes nothing from this press.
    Start,
    /// Heat is not wanted any more. **A PROCEDURE AND NOT A SWITCH**: the burner goes to burning off and reaches a stop through it,
    /// the way Froling's "Boiler off" does (P4 instructions); and a fault is acknowledged by it, and by nothing else.
    Stop,
    /// A person lights the fire now. From a stop it lights AT ONCE, whatever the water says - a button that answered "the water is warm enough"
    /// would leave a person with no way to light his own boiler and look at it.
    Light,
}

impl Presses {
    /// The edge between the counts the fire last took and these. **SEVERAL MOVED, STOP WINS, THEN LIGHT**: buttons between two seconds
    /// are not told apart by their counts, and "Stop beats every source" is the safe side; a Light is the one that says the fire lights.
    pub fn since(self, last: Presses) -> Told {
        if self.stop != last.stop {
            Told::Stop
        } else if self.light != last.light {
            Told::Light
        } else if self.start != last.start {
            Told::Start
        } else {
            Told::Nothing
        }
    }
}

/// What the burner is asked this second: the word of the water, the level to burn at, whether a person asked for heat, and the
/// button counts.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Order {
    pub water: Water,
    /// The level the fire is carried on at work: the dose and the pause and the draught, as the thermostat chose them - a row of
    /// the table of steps, or the level of the mode that holds no temperature.
    pub step: Step,
    pub switch: Switch,
    pub presses: Presses,
    /// Whether the house says cold this second. **THE WATER'S WORD AND THE HOUSE'S ARE TWO THINGS**: `Keep` is the water standing inside its hysteresis, and a house that is cold under it still wants the fire
    /// kept alive, which the word of the water alone does not say.
    pub house_cold: bool,
}

/// Every word of the water an order says, by the name a client reads it by: what the thermostat's card shows of the order, and the words the page has a text for. **A CODE AND NOT A SENTENCE**: the words are the page's own.
pub const WATERS: [&str; 5] = ["burn", "keep", "enough", "level", "silent_return"];

/// The name of a word of the water, from [`WATERS`].
pub fn water_word(water: Water) -> &'static str {
    match water {
        Water::Burn => "burn",
        Water::Keep => "keep",
        Water::Enough => "enough",
        Water::Level => "level",
        Water::Silent(Held::Return) => "silent_return",
    }
}

/// Every state of the switch, by the name a client reads it by.
pub const SWITCHES: [&str; 3] = ["unasked", "on", "off"];

/// The name of a state of the switch, from [`SWITCHES`].
pub fn switch_word(switch: Switch) -> &'static str {
    match switch {
        Switch::Unasked => "unasked",
        Switch::On => "on",
        Switch::Off => "off",
    }
}
