//! What the burner's photoresistor says about its flame, from the millivolts on its pin, and nothing about
//! wires. The ADC that fetches those millivolts is the device's half (devices/burner/src).
//!
//! The circuit: a GL5528 from the pin to GROUND, 100 kΩ from the pin to 3.3 V, 1 µF at the pin. **Brighter
//! light, fewer millivolts** - the photoresistor falls to a few kΩ in the light and pulls the node down, and
//! rises to megohms in the dark and lets the 100 kΩ pull it up.
//!
//! The thresholds are NOT here. They are calculated numbers today (2.4 V lit, about 1.1 V steady);
//! the real ones are to be taken off this board with a lamp and a palm. A number kept in a crate is a number somebody has
//! to rebuild to change; these come in as a parameter, the way the draught's divider comes in as two
//! resistances.

pub mod voice;

/// What one reading amounts to. Three states of the fire and one fault of the setting - and NO fourth state for
/// a sensor that is not there. Why there is none is in `TELLS_A_BROKEN_WIRE`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Flame {
    /// No light on the sensor. A broken wire reads the same, and on this circuit nothing tells them apart.
    Out,
    /// Lit, but not as strong as the thresholds call steady.
    Weak,
    /// Burning at or past the strength the thresholds call steady.
    Steady,
    /// The two thresholds are not two thresholds: the steady one has to sit BELOW the lit one, because fewer
    /// millivolts mean more light. Not a state of the fire, and never answered as one.
    Unconfigured,
}

/// Where the states are divided, in millivolts on the pin. Fewer millivolts is more light, so `steady_mv` sits
/// BELOW `lit_mv` - the order looks upside down on paper and is the right way round on this circuit.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Thresholds {
    /// At or under this the sensor sees a flame.
    pub lit_mv: u32,
    /// At or under this the flame is holding.
    pub steady_mv: u32,
}

impl Thresholds {
    /// Whether these two are thresholds at all: steady strictly under lit, and neither of them nothing.
    pub fn sound(&self) -> bool {
        self.steady_mv > 0 && self.steady_mv < self.lit_mv
    }
}

/// WHAT THIS CIRCUIT CANNOT DO, said where a reader of the code meets it rather than in a document nobody opens.
///
/// A photoresistor in the LOWER arm means darkness and a broken wire pull the node the same way: in the dark the
/// GL5528 reaches about 2 MΩ and the node sits at 3.143 V, and with the wire off the 100 kΩ holds it at 3.3 V.
/// Those 157 mV apart would be tellable if the converter could still tell them apart up there. It cannot, and
/// this is measured rather than argued:
///
///   * the chip's ceiling at this attenuation is 3100 mV on a raw count of 4095 - esp-idf-hal 0.47.0,
///     src/adc.rs, `DirectConverter::get_max_mv` for esp32s3 and `MAX_READING`;
///   * on the board, 21.09, every one of twenty samples of GPIO7 in the dark came back AT that raw count
///     (firmware::analog::CEILING_RAW). A sample standing there says "at least this high" and nothing more.
///
/// The card shows 3153 mV for that same count and the ceiling is called 3100: the first comes through the
/// chip's curve calibration, the second is the plain uncalibrated arithmetic on the identical raw count. One
/// saturated reading named two ways - see `firmware::analog::CEILING_RAW`.
///
/// Darkness already saturates, and a broken wire puts MORE voltage on the pin than darkness does, so it
/// saturates too and arrives as the same number. Hence `None`, and no fifth outcome invented for symmetry with
/// the draught - whose sensor has an offset zero exactly so that its absence can be told.
///
/// **What was NOT measured:** a pin actually left open. Nothing was unplugged - the owner's word of 20.09 - and
/// the conclusion does not need it: 3.3 V is the most this circuit can put on the pin, and everything from the
/// dark's 3.14 V upwards is already past the ceiling.
///
/// The failure is at least the safe one: a burner that cannot see its flame reads `Out` and stops feeding,
/// rather than feeding a fire that is not burning.
///
/// What would change it is iron, not code: the photoresistor in the upper arm, or a smaller resistor that
/// brings darkness down inside the scale. That is the owner's call and not this stage's.
pub const TELLS_A_BROKEN_WIRE: Option<&str> = None;

/// What the pin's millivolts mean, given where a person has put the two thresholds.
pub fn read(pin_mv: u32, thresholds: Thresholds) -> Flame {
    // The setting is answered BEFORE the pin: a reading taken through thresholds that are not thresholds is not
    // a reading, whatever the pin holds.
    if !thresholds.sound() {
        return Flame::Unconfigured;
    }
    match pin_mv {
        mv if mv <= thresholds.steady_mv => Flame::Steady,
        mv if mv <= thresholds.lit_mv => Flame::Weak,
        _ => Flame::Out,
    }
}

/// What a reading says to a person, on the card and in the shell. Never a number and never a dash: a row that
/// turns "no flame" into 0 tells somebody the burner is fine and seeing nothing.
///
/// SHORT WORDS, the same 64 characters the draught's rows are held to - these are rows in a card on a phone.
/// The thresholds are not quoted here: they stand in their own fields right under the row, and a number written
/// in two places is a number that will disagree with itself.
pub fn words(said: Flame) -> String {
    match said {
        Flame::Out => "out - dark, and a broken wire reads the same here".to_string(),
        Flame::Weak => "weak - lit, but not steady by the thresholds below".to_string(),
        Flame::Steady => "steady - the flame is holding".to_string(),
        Flame::Unconfigured => "thresholds crossed - steady must sit below lit".to_string(),
    }
}
