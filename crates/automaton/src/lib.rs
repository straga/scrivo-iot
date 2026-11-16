//! The order the burner burns in. Readings and seconds go in, commands for the auger, the igniter and the fan
//! come out, and nothing here knows a pin, a relay or a bus: the organs are the device's half
//! (devices/burner/src), the air is `crates/regulator`, and the sensors are `crates/flame`, `crates/draught`,
//! `crates/max6675` and `crates/modbus`.
//!
//! **THE STATES ARE NOT INVENTED HERE.** Five running and three of service, and every one of them is a state
//! somebody else's controller already has: Plum's `DeviceState` carries twelve names and its manual describes
//! five running modes (FIRING UP, OPERATION, SUPERVISION, BURNING OFF, STOP), Propel names five of its own, and
//! the factory cycle of THIS burner is a sequence through the same ones.
//! What is ours is which of them we keep and what moves us
//! between them.
//!
//! **NOT ONE NUMBER OF THE FIRE IS WRITTEN IN THIS CRATE.** Seconds of purge, seconds of the igniter, how many
//! attempts, the pascals of a step, the seconds of a dose - all of them arrive in [`Tuning`], for the reason
//! the regulator's setpoint arrives in pascals rather than in percent: a number compiled into the image is a
//! truth nobody has reviewed since the day it was typed, and these are numbers the owner turns on a working
//! boiler. The sources disagree about them anyway - the factory cycle allows 1+7 attempts at lighting, Plum
//! three, our own prototype one - and a crate that picked one of those would be picking it for everybody.
//!
//! **EVERY ROAD TO A FAULT GOES THROUGH BURNING OFF.** The stage's table of transitions says kindling goes to
//! a fault when the attempts run out, and its table of failures says "burn off, then the fault"; the second is
//! the one obeyed here, because the rule above both of them is that burning off may not be skipped - there is
//! fuel in the pot and it may not be left there. Plum does the same with its worst fault: back fire starts
//! BURNING OFF rather than stopping the boiler (manual 16.2).
//!
//! **BURNING OFF BLOWS WITHOUT FEEDBACK, AND THAT IS WHY IT SURVIVES A DEAD SENSOR.** It asks the fan for
//! [`Asked::Purge`] - full air, nothing read - so the one fault that stops the regulator dead, a draught
//! sensor that gives no number, does not also take away the means of putting the fire out. Had burning off
//! held a setpoint, the board would have had to blow the pot clear through an instrument it had just declared
//! untrustworthy.
//!
//! **COMING BACK FROM A POWER CUT, THE RESTART ITSELF LIGHTS NOTHING.** It either carries a live fire on or
//! burns it off; it does not resume an interrupted kindling, and it does not restart itself because the water
//! is cold: a stop is lit by the ordinary doors alone (a press, or the switch with the water and the house cold - a house
//! lights nothing from a stop until the sensors have gone through their first round, whoever plays the demand, and a room that then does not answer leaves a stop to the floor of the boiler's water alone - `crates/demand`, `crates/thermostat`). The board was without power for an unknown time and what it remembers is a hypothesis, not a fact;
//! this project's own rule is that a restart energises nothing by itself (`main.rs`, every organ off
//! whatever it was before). Both hold at once because [`State`] cannot express the middle of anything - see
//! [`Burning::resumed`]. A fire carried on at the first second is the board's own from then on, and a flame it loses
//! later is lit again like any other; what lights nothing is a board that came up to a dark pot.

use door::{a_number, above, above_nought, not_below, not_below_nought};
use flame::Flame;
pub use regulator::Asked;

pub mod door;
mod hands;
mod memory;
mod order;
mod relief;
mod run;
pub mod voice;
pub use hands::{by_hand, hand_is_free, hand_over, plug_by_hand, plug_stays, takes_back, Hands, OnChange, ORGANS};
pub use order::{switch_word, water_word, Held, Order, Presses, Switch, Told, Water, SWITCHES, WATERS};
pub use memory::{broken_at, mode_word, recalled, remembered, semi_of, stage, why_name, MODE_CARD, STAGES, WHYS};
pub use relief::{Found, Relief, RELIEF_AFTER_S, RELIEF_PER_DAY, RELIEF_RETRY_S};
pub use run::{Automaton, Driving};

/// What the burner is doing, and the only thing about it that may be written down and read back after a power
/// cut.
///
/// **IT CARRIES NO PHASE OF ANYTHING, AND THAT IS THE POINT.** How far into a purge, which second of the
/// igniter, where in the dose - all of it lives in the private fields of [`Burning`] and dies with the power.
/// Were "burning off, step three" expressible here, a board coming up would be able to continue it, and
/// continuing is exactly what must not happen: the fire moved on while the board was dark. What may be
/// remembered is "we were burning off", and from that the only thing to do is burn off again from the top.
///
/// The number in `Kindling` is the exception, and it is for a person rather than for the machine: Plum shows
/// the attempt beside its igniter (manual 8.4), and a board that says "attempt 3" says something a board that
/// says "kindling" does not.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    /// Nothing runs. Waiting for a person, or for the water to cool - and only if a person has asked for heat
    /// at all; see [`Order::switch`].
    Stop,
    /// The fan at full, past the regulator. The sensor ends at 300 Pa and the purge runs at the maximum, so
    /// "over the scale" here is an ordinary reading and not a fault (`crates/regulator`, [`Asked::Purge`]).
    Purge,
    /// The igniter, the dose and the wait for a flame, on the attempt numbered from one.
    Kindling(u32),
    /// A flame is there and the fire eats the lighting's dose: the auger stands, and the draught climbs from the
    /// lighting's to the step's.
    Rising,
    /// The step is held by the pair the tuning names: a dose of fuel and a draught.
    Working,
    /// Very little power, and the air may be none at all: the chimney pulls enough for a handful of pellets,
    /// and a fan at its lowest still blows more than they can take.
    Smouldering,
    /// The fuel is off and the fan is clearing what is left, until the flame goes or the time runs out. What
    /// waits at the end of it: a stop, or the fault that sent the burner here.
    BurningOff(Option<Why>),
    /// Something did not add up, and a person is told which thing. Left only by a person.
    Fault(Why),
}

/// How the burner stands, for the board that restarts itself: only at rest is a restart harmless.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rest {
    /// A stop with nothing running and no flame seen.
    Standing,
    /// Anything that is a fire or on its way to one or from one.
    Fire,
    /// A fault, which waits for a person.
    Fault,
}

/// How the burner stands this second, from what it was told to do and whether the flame sensor sees a flame.
pub fn rest(doing: &Doing, flame_seen: bool) -> Rest {
    match doing.state {
        State::Fault(_) => Rest::Fault,
        State::Stop if !flame_seen && !doing.igniter && !doing.auger && doing.feeding == Feeding::Stop => Rest::Standing,
        _ => Rest::Fire,
    }
}

/// What did not add up. Told to a person by [`words`], and nothing here is a guess about the fire: each one is
/// something an instrument said or refused to say.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Why {
    /// No flame after this many attempts.
    NeverLit(u32),
    /// A flame that was burning went out and did not come back within `flame_lost_s`, on a board that may not light
    /// anything: one that came up from a power cut (see [`Burning::resumed`]).
    FlameLost,
    /// A flame that was burning went out, and this many attempts at lighting it again did not bring it back.
    FlameNotBack(u32),
    /// Fuel is not arriving: the flue stood under the cold mark for the whole window, or fell from its own highest by more than the
    /// card allows. **THE SIGN DOES NOT WORK ON A COLD FLUE**, so it is only ever asked in [`State::Working`] - during
    /// kindling and rising the flue is cold for an honest reason.
    NoFuel,
    /// The draught sensor gives no number. The regulator stops on this (`crates/regulator`, [`Blind`]), and a
    /// burner whose air cannot be measured may not be fed.
    ///
    /// [`Blind`]: regulator::Blind
    NoDraught,
    /// The draught sensor answers, and for [`Tuning::low_pressure_s`] it has said "under my zero" while the fan was asked for air: the air does not come. Not a sensor gone: the sensor is the one that says it.
    LowPressure,
    /// The water is hotter than the burner is allowed to leave it, with the temperature seen.
    Overheated(f32),
    /// The flame's two thresholds are not thresholds ([`Flame::Unconfigured`]), so nothing the sensor says
    /// separates fire from no fire. Not a state of the boiler - a line of configuration nobody filled in.
    Unconfigured,
    /// The draught sensor's round has stopped publishing ([`Heard::Silent`]): the same way out as [`Why::NoDraught`],
    /// named for what it is - a sensor gone quiet, not a draught of nothing.
    DraughtSilent,
    /// The flame sensor's round has been silent too long ([`Heard::TooLong`]): a fire nobody can see is burnt off.
    FlameSilent,
    /// The water's reading has been silent too long ([`Heard::TooLong`]): a boiler nobody can read is burnt off.
    WaterSilent,
    /// The return's reading has been silent too long while the thermostat holds the return ([`Water::Silent`]).
    ReturnSilent,
    /// The zero-crossing pin gives no fronts: the fan cannot be fired in step with the mains, so it is not fired, and a burner that cannot be given air may not be fed. It names the PIN and not the mains:
    /// the mains may well be there (24.10: the pin was silent for 86 minutes on a live one), and what the board knows is what the pin gave.
    NoFronts,
}

/// Whether a sensor's round is still publishing its reading (crates/module, `Quiet`): its role grows old past three
/// periods of the round (at least 3 s) and too old past ten (at least 30 s).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Heard {
    #[default]
    Speaks,
    /// Past three periods: what it last said is not given out, and the reading in [`Seen`] is None.
    Silent,
    /// Past ten periods: long enough that a fire run without the reading has to stop.
    TooLong,
}

impl Heard {
    /// Whether the sensor has gone quiet at all.
    pub fn quiet(self) -> bool {
        self != Heard::Speaks
    }
}

/// The three counts of the draught, in seconds, as [`Burning::draught_counts`] gives them.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct DraughtCounts {
    /// How long the fan has been asked to hold a setpoint without a break ([`Tuning::spin_up_s`]).
    pub held_s: f32,
    /// How long the draught has given no number in a row, whatever the fan was asked.
    pub no_number_s: f32,
    /// How long the sensor has said "under my zero" in a row while the fan was asked to hold a setpoint ([`Tuning::low_pressure_s`]).
    pub low_s: f32,
}

/// Which of the sensors the automaton sees by have gone quiet. A reading of a silent sensor is None in
/// [`Seen`] as any other missing number; this says WHY it is None, which is what the automaton needs to name the
/// cause and not to light what it cannot see.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Silence {
    pub draught: Heard,
    pub flame: Heard,
    pub flue: Heard,
    pub water: Heard,
    /// How long the zero-crossing pin has given no fronts, in seconds: `dimmer::Fronts` is the one that says it is lost. None while it gives them, and
    /// when it never did (a board with no mains on its module has lost nothing).
    pub fronts_s: Option<u32>,
}

impl Silence {
    /// Every sensor publishing, and the pin giving its fronts.
    pub const HEARD: Silence = Silence { draught: Heard::Speaks, flame: Heard::Speaks, flue: Heard::Speaks, water: Heard::Speaks, fronts_s: None };
}

/// What the board knows this second. Every one of them may be missing, and a missing one is never a zero: a
/// flue that answers nothing is not a flue at 0 °C, and a burner would act on the difference
/// (`crates/max6675`, `crates/modbus`, `crates/draught` each say the same in their own words).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Seen {
    /// What the photoresistor says. **A BROKEN WIRE READS AS `Out`** and nothing on this circuit tells them
    /// apart (`flame::TELLS_A_BROKEN_WIRE`); the failure is the safe one - a burner that cannot see its flame
    /// stops feeding.
    pub flame: Flame,
    /// The water of the boiler, in degrees. None - the bus said nothing about it this round.
    pub boiler_c: Option<f32>,
    /// The flue gas, in degrees. None - an open thermocouple or a silent chip.
    pub flue_c: Option<f32>,
    /// The draught, in pascals. None - under the scale, nothing on the pin, or a divider nobody configured.
    pub draught_pa: Option<f32>,
    /// The sensor answers and says it is under its own zero (`draught::Reading::Under`): `draught_pa` is None then too, but the sensor is there. **A SENSOR THAT SAYS "LESS THAN NOTHING" IS NOT A SENSOR THAT SAYS NOTHING**:
    /// the first is air that has not come yet or a fan that stalled, and it has a term of its own ([`Tuning::low_pressure_s`]); the second is a loss of the sensor, and has none.
    pub draught_low: bool,
    /// The percent the fan's gate is set to now; 0 - the fan asked for no air. **THE DRAUGHT ALONE IS NOT AIR** (bench
    /// board .30, 27.09: a floating pin read 171 Pa with the gate at 0 %) - see [`air`].
    pub gate: u32,
    /// Which sensors have gone quiet: why a reading above is None when it is.
    pub silence: Silence,
}

/// A dose and the pause after it, in seconds. The auger runs at about 4 g/s on this burner (weighed by the
/// owner), so the power of a step is the dose times the calorific value over the period - which is why a step
/// is a PAIR and not a percentage.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Feed {
    /// How long the auger turns. Above nothing: a dose of no seconds is no dose.
    pub on_s: f32,
    /// How long it waits before the next one. Above nothing, or the auger never stops - and an auger that
    /// never stops is the one fault nobody gets to watch happen twice.
    pub off_s: f32,
}

/// One level of the fire: the air, and the feed that goes with it. **THE TWO ARE ONE THING.** Everybody who
/// runs a pellet burner low sets them together - Plum gives each level of modulation its own pair and its own
/// airflow, and its supervision mode a third set - because a dose that is right at full air smothers at none
/// and blows away at too much.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Step {
    /// The draught held while this level burns, in pascals.
    ///
    /// **NOT BELOW NOUGHT, AND NOUGHT IS A LEVEL.** Smouldering runs on what the chimney pulls, and a fan at
    /// its lowest still blows more than a handful of pellets can take - so the door lets this zero through
    /// deliberately, the same way it lets `smoulder_s` through.
    pub pa: f32,
    pub feed: Feed,
}

/// The air and the dose of an attempt at lighting.
///
/// **ONE DOSE AND NO PAUSE, BECAUSE THE DOSE IS A LEVEL AND NOT A PORTION**: it fills
/// the pot above the plug, or the plug heats air. Once a flame shows there is fuel in plenty, so nothing is fed after it
/// until the fire works - rising eats this dose with the auger standing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Lighting {
    /// The draught an attempt holds once the plug burns with air. Above nothing: a plug in still air burns out.
    pub pa: f32,
    /// How long the auger turns for the first attempt's dose. Above nothing: a dose of no seconds is no dose.
    pub dose_s: f32,
}

/// Every number the burner burns by. **NOT ONE OF THEM HAS A VALUE IN THIS CRATE** - they are the owner's, and
/// he turns them on a boiler that is alight.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tuning {
    /// How long the fan blows before the igniter comes on. The factory cycle says 30 s in one place and 150 s
    /// of pre-ventilation in another, and which of them is ours is not settled.
    ///
    /// Above nothing: a purge of no seconds is a pot lit without having been blown clear, and it happens
    /// silently - the state passes by in the tick it was entered.
    pub purge_before_s: f32,
    /// The least burning off may last, however dark the pot already is.
    ///
    /// **WITHOUT IT, THE WORST CASE GETS THE SHORTEST BURNING OFF.** Its ordinary end is the flame going out,
    /// and after a kindling that never lit the flame never came - so the one pot that certainly holds unburnt
    /// pellets would be the one nobody blew clear, on the very second it was given up on. The factory blows
    /// 20 s after every stop for this reason ("Burner parameters" in its manual), and Plum burns remnants off rather than
    /// declaring the fire over (manual 8.7).
    ///
    /// Above nothing, for the reason written just above: at nought the floor is gone and the worst case gets
    /// the shortest blow again.
    pub purge_after_s: f32,
    /// The most the plug burns WITH AIR in one attempt, after its short heat: the fan is on the lighting draught the
    /// whole of it, and the plug goes off with the flame or here, whichever is first (Plum 850P2,
    /// 14.1). Above nothing: an attempt whose plug never burns with air lights nothing.
    pub igniter_s: f32,
    /// How long the fan blows alone after the plug went off with no flame, before the next attempt: a fire the plug
    /// started late still shows here (Plum 860P, `Fan time extension`).
    ///
    /// Above nothing, and a NUMBER: at nought every attempt is spent in the tick it began, and an infinity
    /// is a burner that waits for a flame for ever.
    pub wait_for_flame_s: f32,
    /// How many attempts before the burner gives up. 1+7 in the factory cycle, three at Plum, one in our
    /// prototype - and the disagreement is why the number is here. At least one: a kindling without a single
    /// attempt is not a kindling.
    pub tries: u32,
    /// What the dose of the second and later attempts is worth against the first. Plum cuts it to a tenth
    /// (manual 8.4): a pot that did not light is a pot with fuel already in it. Only the first lighting's attempts: a
    /// lost flame's attempts pour `relight_s`.
    ///
    /// Above nothing and not above one - the only bound in this record that is a plain ceiling, and it is a
    /// ceiling because the field is a FRACTION of the first dose: more than the whole of it is not a
    /// reduction at all, and nought would run the plug over a pot nobody fed.
    pub dose_after_first: f32,
    /// How long the flame has to hold without a break in rising before the work begins, and how long the draught
    /// takes to climb from the lighting's to the step's. **A FLAME THAT BLINKS STARTS THE COUNT AGAIN FROM ITS RETURN**:
    /// the auger stands the longer, which is the safe side, and a fire that never settles eats the dose
    /// and goes out for good. Above nothing: at nought the work begins on the flame's first second, over a pot full to
    /// above the plug.
    pub rise_s: f32,
    /// How long the flame may be gone in rising or at work before it is taken for lost and lit again (20 s).
    ///
    /// **A BLINK IS NOT A LOST FIRE**: the sensor flickers and pellets settle, so for these seconds the stage goes on as
    /// it was - the same feed, the same air - and a flame that comes back leaves no trace. **THE LIGHT ALONE DECIDES**:
    /// a flue that holds does not stretch the wait. Above nothing: at nought one dark sample lights a live fire again.
    pub flame_lost_s: f32,
    /// How long smouldering may last before the burner puts itself out. **ZERO SKIPS SMOULDERING
    /// ALTOGETHER** and goes straight from working to burning off - Plum's own trick with its supervision
    /// time (manual 8.6), and the plainest way to say "this boiler does not idle".
    ///
    /// **NOT BELOW NOUGHT, AND NOUGHT IS THE ONE THE DOOR MUST LET THROUGH.** A flat rule against zeroes
    /// would break the thing this field is for.
    pub smoulder_s: f32,
    /// The longest the fire may burn out, a setting of ten minutes. Burning out ends
    /// by the fire - `burn_out_min_s` without a flame - and this is the ceiling over it.
    ///
    /// **THE CEILING IS A STOP, NOT A FAULT, WHATEVER THE FLAME SAYS**: Plum 850P2 at
    /// the most of its burning off goes to STOP even with a flame seen (manual 8.7). The pot is blown clear and stopped,
    /// and the stop holds the step's draught while the sensor sees a flame (`air`).
    ///
    /// Above nothing, and not below `purge_after_s` nor `burn_out_min_s`: a ceiling under its own floor ends the
    /// burning off before the floor can bite.
    pub burn_off_s: f32,
    /// How long the flame has to be gone IN A ROW before burning out ends and the pot is blown clear at full.
    ///
    /// **THE FIRE ENDS BURNING OUT, NOT THE CLOCK**: a blow at full while the pot
    /// is still alight blows the fire out rather than burning it, and a flame that blinks back starts the minute again.
    /// 60 s, fixed. Not below nought - at nought the blow begins with the dark - and not above `burn_off_s`.
    pub burn_out_min_s: f32,
    /// The water of the boiler the pump runs from; it stops `pump_off_below_c` below it.
    ///
    /// **THE PUMP FOLLOWS THE WATER IN EVERY STATE, NOT THE FIRE** (Plum `CH pump activation
    /// temperature`): cold water from the system pulled through a boiler that has not warmed up runs down its
    /// walls as condensate, and a hot boiler after a stop still has heat to give away. Two more rules sit over
    /// the threshold: an overheating boiler (`too_hot_c`) has its pump run - the first step of throwing the heat
    /// off - and a boiler that gives no number has its pump run, because not knowing the water and not moving
    /// it is worse than moving it for nothing.
    ///
    /// Above nothing.
    pub pump_on_c: f32,
    /// The flue at or above which a board that came up over a dark pot takes the pot for burning (see [`Burning::resumed`]).
    /// **ASKED AT A RESTART AND NOWHERE ELSE**: what the flue says about the fuel at work is `flue_cold_c`, `flue_fall_c` and
    /// `flue_fall_s`.
    ///
    /// Above nothing: at nought every flue counts as hot, and a boot reads every reading as a sign of fire.
    pub flue_holds_c: f32,
    /// The flue under which a working burner is cold: for the whole of `flue_fall_s` in a row, and not risen `flue_fall_c` from its lowest in it, the fuel is not arriving.
    /// **ASKED IN `Working` AND NOWHERE ELSE** - see [`Why::NoFuel`].
    ///
    /// **A LOW FIRE HOLDS ITS FLUE WHERE IT HOLDS IT AND DOES NOT CLIMB** (27.09: 93-105 °C at 6 and 3 kW): a flue that
    /// "does not climb" put out a live fire. A cold flue and a falling one are what a feed that stopped looks like.
    /// **A COLD START IS NOT COLD FOR THE WHOLE WINDOW**: work began over 23 °C on 27.09 and the flue crossed 70 °C in 291 s, so the
    /// verdict is of a flue that stayed under for the window, not of one that is under at this second.
    ///
    /// Above nothing: at nought no flue is cold, and the sign never arms.
    pub flue_cold_c: f32,
    /// How far the flue may fall from its own highest in the last `flue_fall_s` before the fuel counts as not arriving.
    /// Above nothing: at nought every reading under the highest is a fall, and the sign fires on noise.
    pub flue_fall_c: f32,
    /// The window of both signs of the flue, in seconds: how long the flue stays under `flue_cold_c`, and how far back the highest
    /// flue is looked for. Above nothing: at nought the sign fires on the first tick.
    ///
    /// **THE FLUE IS ASKED ONLY OF A FLAME THAT IS NOT STEADY**: a steady second begins the window again, so a fire the sensor
    /// calls steady is never put out for its flue (04.10: 20 -> 64 °C over a steady flame was put out at the 605th second of work).
    ///
    /// **THE WINDOW IS MINUTES, BECAUSE A FIRE HAS INERTIA**: a minute of work gave the
    /// fuel up on a fire that was burning. The check is asked only at work, so the window is also the margin after
    /// rising. The highest is kept in [`FLUE_PARTS`] parts of the window, so the window it looks back over is the setting and at most
    /// its twentieth more.
    pub flue_fall_s: f32,
    /// How long a flame in `Kindling` must hold, in seconds in a row, before the fire counts as caught - from the embers as work, with the
    /// plug as the rising. A reading of darkness inside them starts the count again and does not end the attempt: the attempt's time
    /// goes on as it did.
    ///
    /// **A FLAME ONE READING OVER THE THRESHOLD IS NOT A FIRE** (27.09: the first light from the embers was counted at 2791 mV
    /// against 2800, and the sensor was dark at the next second). The count is [`Burning::watching`]'s, the mechanism that already
    /// watches a flame the board met after a restart.
    ///
    /// Above nothing: at nought a reading is a fire, which is the defect this field closes.
    pub lit_s: f32,
    /// How far under `pump_on_c` the pump stands off: the pump's own number, and not the setpoint's hysteresis.
    ///
    /// Above nothing: at nought the pump has no band and chatters at its threshold.
    pub pump_off_below_c: f32,
    /// The water at which feeding stops and the burner goes to burning off. The first of the two steps the
    /// stage asks for; the second is `alarm_c`.
    ///
    /// A number, and above the setpoint of the thermostat: a burner that overheats before it reaches the water it
    /// is set to hold can never hold it. **That order is the thermostat's door's to ask** (`thermostat::Tuning::sound`),
    /// since the setpoint is its number; here the number is asked for on its own.
    pub too_hot_c: f32,
    /// The water at which a person is signalled, whatever the burner is already doing. 95 °C at Plum
    /// (manual 16.1).
    ///
    /// A number, and not below `too_hot_c`: a person woken before the burner has even stopped feeding is
    /// woken for something the burner had not yet tried to handle.
    pub alarm_c: f32,
    /// How long a board that came up to a dark pot watches a flame that shows afterwards before it climbs and feeds.
    ///
    /// **A FLAME THAT CAME BACK TO A BOARD WHICH SAW NONE HAS BEEN SEEN ONCE, AND ONCE IS NOT SEEN.** A fire the
    /// board sees at its first second is carried on as it was; this one showed after a wait on a
    /// pot the board does not know, so the burner gives it air, watches it hold for these seconds, and only then
    /// climbs.
    ///
    /// **ABOVE NOTHING, AND THIS IS THE LIMIT THE WHOLE DOOR WAS BUILT FOR.** At nought the watch is over in
    /// the tick it began and the auger turns on a fire the board has seen once - the defect this field
    /// exists to close, walking back in through the setting that closes it.
    pub sure_s: f32,
    /// The doses of the attempts at lighting a lost flame again, in seconds of auger, one per attempt:
    /// 20, 10, 10. **THE POT IS NOT OVERFILLED**: from the second the flame is taken for lost the auger
    /// turns only inside these portions, and the attempt with the plug after the embers pours nothing at all. Not below
    /// nought.
    pub relight_s: [f32; 3],
    /// The flue at or above which a lost flame is lit again WITHOUT the plug, from the embers (a cold start
    /// began at 17-25 °C, the two live losses of 26.09 at 54 and 76 °C). Above nothing.
    pub relight_hot_c: f32,
    /// How long the fan is given to turn up to a setpoint from standing before the draught has to be read.
    ///
    /// **A FAN ASKED FOR AIR THIS SECOND HAS NOT GIVEN ANY YET.** It stands while the plug heats and while the
    /// fire smoulders on the chimney's own pull, and from standing it takes seconds to put a number on the sensor:
    /// 14 Pa one second after a purge began on this burner, 221 Pa the next (23.09). Asked for its draught in the
    /// very second the setpoint was given, every attempt at lighting fell into NoDraught before the fan had moved.
    ///
    /// **ABOVE NOTHING**: at nought the draught is asked of a fan that has not been told to turn yet, which is the
    /// defect this field closes.
    pub spin_up_s: f32,
    /// How long the draught may stay under the sensor's own zero, in a row, while the fan is asked to hold a setpoint, before the fire is burnt off as [`Why::LowPressure`].
    ///
    /// **THE LONGEST DIP THAT CAME BACK IN THE RECORD OF 16-29.10 WAS 9 S** (five such dips, 1 to 9 s): a fan that is turning up from 8 % of travel reads the sensor's zero for seconds and then
    /// a number. A sensor that is GONE (`Missing`, silent) is not counted here: it burns off after [`Tuning::spin_up_s`] as it always did.
    ///
    /// **ABOVE NOTHING**: at nought every dip is a fault by itself, which is what the sensor's loss was and this field is there to tell apart.
    pub low_pressure_s: f32,
    /// The air and the dose of an attempt at lighting.
    pub kindling: Lighting,
    /// The air and the dose of smouldering.
    pub smoulder: Step,
}

/// The codes a refusal of the door carries, in the order of the list the card says (`refused`): what a person is told
/// of a number that was not taken is one of these, and the words are the page's own table.
pub const REFUSALS: [&str; 10] = [
    "not_a_number",
    "infinite",
    "above_nought",
    "not_below_nought",
    "no_tries",
    "dose_over_first",
    "above",
    "not_below",
    "below",
    "kindling_no_air",
];

/// A number of the fire that was not accepted: which field, which of [`REFUSALS`], and the numbers it stands on.
/// **A CODE AND NOT A SENTENCE**: the card showed a sentence such as "cannot be less than purge_after_s (20), and it stands at 10"
/// in Russian to a page that may speak English, and the board built the sentence on the heap to say it; the page
/// has its own words for every code, in both languages.
#[derive(Clone, PartialEq, Debug)]
pub struct Refused {
    /// The field, spelled as [`Tuning`] spells it - `smoulder.pa` for a number inside a step - so that a
    /// person can find it on the card.
    pub field: String,
    /// What is wrong with it: one of [`REFUSALS`].
    pub code: &'static str,
    /// The number that stood in the field; None when it is no number at all (`not_a_number`, `infinite`).
    pub value: Option<f32>,
    /// For an order of two numbers (`above`, `not_below`, `below`): the field the number was held against.
    pub than: Option<&'static str>,
    /// ... and the number that stood there.
    pub limit: Option<f32>,
}

impl Refused {
    pub fn of(field: &str, code: &'static str, value: f32) -> Refused {
        let value = value.is_finite().then_some(value);
        Refused { field: field.into(), code, value, than: None, limit: None }
    }

    pub fn against(field: &str, code: &'static str, value: f32, than: &'static str, limit: f32) -> Refused {
        Refused { than: Some(than), limit: Some(limit), ..Refused::of(field, code, value) }
    }

    /// The refusal as one line for the log and the console, in the board's English.
    pub fn words(&self) -> String {
        let value = self.value.map_or(String::new(), |v| format!(" (is {v})"));
        match (self.than, self.limit) {
            (Some(than), Some(limit)) => format!("{}: {} {than} ({limit}){value}", self.field, self.code),
            _ => format!("{}: {}{value}", self.field, self.code),
        }
    }
}

/// Tuning that has been through the door.
///
/// **THE ONLY THING THE BURNER WILL BURN BY, and [`Tuning::sound`] is the one way to hold one** - the field
/// inside is private, so there is no other. A record of numbers can be filled with anything; this cannot
/// exist unless every number in it was looked at.
///
/// Why a type rather than a check at the top of [`Burning::step`]: a check by the tick is paid every second
/// for something decided once, and a check somebody remembers to call is a check somebody forgets to call.
/// Here forgetting is not among the options - there is no `Burning::step` that takes a raw record.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sound(Tuning);

impl Sound {
    /// The numbers inside, for whoever shows them to a person.
    pub fn numbers(&self) -> &Tuning {
        &self.0
    }
}

/// What the auger is to do from now, as a plan the automaton builds every second out of its own count and the relay's task follows for a while (`relay::HORIZON_MS`), timing it to a few tens of ms: it is the automaton that knows where it
/// stands in a dose and in a rhythm, so the plan says that. **A PLAN IS A POSITION, NOT A BEGINNING**: what is left of a dose, how far into its cycle a rhythm stands; a plan that is said again after a silence is therefore said from where
/// the automaton stands, not from the start.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Feeding {
    /// Open.
    #[default]
    Stop,
    /// Closed for LEFT_S seconds from now, then open: what is left of a dose by this count.
    Dose { left_s: f32 },
    /// Cycles of ON_S closed and OFF_S open, the cycle standing INTO_S seconds into itself now.
    Rhythm { into_s: f32, on_s: f32, off_s: f32 },
}

impl Feeding {
    /// Whether OTHER is the same decision as this one - the same kind, and for a rhythm the same numbers of the cycle - though it stands elsewhere in it. A decision that stands is a new plan from the same decision; one that changes is a command.
    pub fn same_decision(&self, other: &Feeding) -> bool {
        match (self, other) {
            (Feeding::Stop, Feeding::Stop) | (Feeding::Dose { .. }, Feeding::Dose { .. }) => true,
            (Feeding::Rhythm { on_s, off_s, .. }, Feeding::Rhythm { on_s: on, off_s: off, .. }) => on_s == on && off_s == off,
            _ => false,
        }
    }
}

/// What one second decided: the state it left the burner in, and what each organ is to do. **THE AUXILIARY POWER
/// IS NOT HERE**: no rule about it is written in the requirements, and a rule invented for it would be this crate's own
/// invention rather than the burner's. The pump is here, by the water of the boiler.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Doing {
    pub state: State,
    /// Whether the auger turns this second, sampled at the start of it: what the card and the scheme show.
    pub auger: bool,
    /// What the auger is asked to do: the road to the relay, which times the portions itself.
    pub feeding: Feeding,
    /// Whether the igniter is hot this second.
    pub igniter: bool,
    /// What the fan is asked for, in the regulator's own words.
    pub fan: Asked,
    /// The water is past `alarm_c` and a person is to be told NOW, whatever else is going on. Separate from
    /// the state because burning off may not be interrupted to raise it, and a person may not wait for the
    /// burning off to finish to hear it.
    pub alarm: bool,
    /// Whether the boiler's pump runs this second. See [`Tuning::pump_on_c`].
    pub pump: bool,
    /// Why the pump runs or stands this second.
    pub pump_why: PumpWhy,
}

/// Where the auger is in its cycle of dose and pause.
///
/// **THE NUMBERS ARE TAKEN AT THE BOUNDARY AND NEVER INSIDE ONE.** A dose that is under way finishes on the
/// number it began with, and the next one takes whatever the tuning says by then. Otherwise a person turning
/// the dose while the boiler burns makes one portion of a size nobody chose - and makes it invisibly
/// ("the parameters are changed on a working boiler").
#[derive(Clone, Copy, Default)]
struct Feeder {
    /// The feed the present cycle of dose and pause began on, None before the first second: its dose and its pause are
    /// the cycle's until it ends, whatever the tuning says meanwhile.
    cycle: Option<Feed>,
    /// How much of the present cycle has gone, in milliseconds, at the start of the second that was asked last. **Whole milliseconds, always under the cycle's period**: the settings are tenths of a second, so every number of a cycle is a whole count of them, and a count in f32 seconds drifts off the period (the remainder of 1.0 by 0.2 comes out 0.19999999).
    elapsed_ms: u32,
    /// How much of the half that second was in was left at its start, in seconds.
    left_s: f32,
}

/// Whole milliseconds of SECS, the unit the position in a cycle is counted in.
fn ms_of(secs: f32) -> u32 {
    (secs * 1000.0).round().max(0.0) as u32
}

impl Feeder {
    /// A second of feeding by FEED, and whether the auger turns at the start of it. **THE CYCLE BEGINS WITH THE FIRST SECOND**: it stands at its beginning then, and a second later a second into itself - where the relay's task, which follows the plan, stands too.
    /// A cycle that has run out begins anew on FEED as it stands now - which is the whole of the boundary rule. A step can be longer than a cycle (0.1 + 0.1 s is allowed, and a step blocks for up to 9 s): the position is taken down to [0, period) of the cycle it falls in, each cycle that ran out handing over to FEED.
    fn second(&mut self, feed: Feed, secs: f32) -> bool {
        match self.cycle {
            None => {
                self.cycle = Some(feed);
                self.elapsed_ms = 0;
            }
            Some(mut held) => {
                self.elapsed_ms = self.elapsed_ms.saturating_add(ms_of(secs));
                loop {
                    let period = Self::period_ms(held);
                    if self.elapsed_ms < period {
                        break;
                    }
                    if held == feed {
                        self.elapsed_ms %= period;
                    } else {
                        self.elapsed_ms -= period;
                        held = feed;
                    }
                }
                self.cycle = Some(held);
            }
        }
        let held = self.cycle.unwrap_or(feed);
        let on = self.elapsed_ms < ms_of(held.on_s);
        let half_end = if on { ms_of(held.on_s) } else { Self::period_ms(held) };
        self.left_s = (half_end - self.elapsed_ms) as f32 / 1000.0;
        on
    }

    /// The cycle's period in whole milliseconds, never nought (the settings keep both halves above nothing).
    fn period_ms(feed: Feed) -> u32 {
        (ms_of(feed.on_s) + ms_of(feed.off_s)).max(1)
    }

    /// The plan the cycle under way is: its numbers, and how far into itself it stands.
    fn plan(&self, feed: Feed) -> Feeding {
        let held = self.cycle.unwrap_or(feed);
        Feeding::Rhythm { into_s: self.elapsed_ms as f32 / 1000.0, on_s: held.on_s, off_s: held.off_s }
    }
}

/// Every clock the automaton counts, in the order its card shows them (`Burning::clocks` names no other): the card holds
/// a row for each in every state, a dash where the state counts none, so no row comes and goes under a person's eyes.
pub const CLOCKS: [&str; 12] = [
    "lighting in",
    "dose ends in",
    "igniter off in",
    "attempt ends in",
    "fed after watching the flame for",
    "full step in",
    "auger switches in",
    "flame not back, lighting again in",
    "flame not back, burning off in",
    "burning off in",
    "burning out ends in at most",
    "blowing clear ends in",
];

/// One clock of the state the burner is in: what ends, and in how many seconds - None while the thing it counts down to has not begun.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Clock {
    pub what: &'static str,
    pub left_s: Option<f32>,
}

/// Why the pump runs or stands: what of the boiler's water decided it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PumpWhy {
    /// The water at this many degrees, against the pump's threshold and its hysteresis.
    Water(f32),
    /// The water past `too_hot_c`: the pump runs whatever its threshold.
    Overheated(f32),
    /// The boiler gives no number: the pump runs.
    NoNumber,
}

/// The pump's reason for a person, one line: what of the water decided it.
pub fn pump_words(why: PumpWhy) -> String {
    match why {
        PumpWhy::Water(celsius) => format!("water {celsius:.0} °C"),
        PumpWhy::Overheated(celsius) => format!("overheated, {celsius:.0} °C"),
        PumpWhy::NoNumber => "no number from the boiler".into(),
    }
}

/// The pump's reason as one word of a known list, for whatever reads it without the words.
pub fn pump_word(why: PumpWhy) -> &'static str {
    match why {
        PumpWhy::Water(_) => "water",
        PumpWhy::Overheated(_) => "overheated",
        PumpWhy::NoNumber => "no_number",
    }
}

/// Where the parts of one attempt at lighting end, in seconds of the attempt.
#[derive(Clone, Copy)]
struct Attempt {
    /// The dose ends, and the air comes in with the plug: **THE PLUG BURNS ONLY WITH AIR**.
    dose_end: f32,
    plug_end: f32,
    end: f32,
}

/// The draught that proves the fan blows, in pascals: the regulator's proof that it turns (`regulator::TURNS_ABOVE_PA`).
/// Under it the plug burns out in still air, whatever the fan was asked (a plug burnt through).
pub const AIR_PA: f32 = regulator::TURNS_ABOVE_PA;

/// Whether the fan blows under the plug: the gate open - GATE, in percent - AND a draught read at or above [`AIR_PA`].
/// **ONE RULE FOR THE AUTOMATON AND THE HAND**. No number is no air - a stall, a tube off, a sensor
/// gone all read so; and **THE DRAUGHT ALONE IS NOT AIR**: on the bench board .30 (27.09) a floating pin read 171 Pa with
/// the gate at 0 %, and a drift or a chimney pulling on its own read the same.
/// The cause a fire is burnt off for when a sensor it runs by has been silent too long; None while none has. The flue is
/// not one of them: a burner runs without it, taking the pot for cold.
///
/// **THE WATER OF THE BOILER IS ASKED IN EVERY MODE**: the protections read it whatever the burner holds, and a boiler
/// nobody can read is a boiler whose overheating nobody would see. The sensor a mode holds by when it is not the boiler's
/// is the thermostat's to hear (`Water::Silent`): the fire takes its word after its own three.
fn silent_too_long(silence: Silence, water: Water) -> Option<Why> {
    let held = match water {
        Water::Silent(Held::Return) => Some(Why::ReturnSilent),
        Water::Burn | Water::Keep | Water::Enough | Water::Level => None,
    };
    [(silence.draught, Why::DraughtSilent), (silence.flame, Why::FlameSilent), (silence.water, Why::WaterSilent)]
        .into_iter()
        .find(|(heard, _)| *heard == Heard::TooLong)
        .map(|(_, why)| why)
        .or(held)
}

pub fn air(gate: u32, draught_pa: Option<f32>) -> bool {
    gate > 0 && draught_pa.is_some_and(|pa| pa >= AIR_PA)
}

/// Into how many parts the window of the flue's fall is cut: the highest and the lowest flue are kept per part, and the highest of the window is the highest of the
/// parts, the lowest the lowest. The newest part is the one still filling, so the look back is the window and the part it is in: never shorter than the setting (a flue falling a
/// degree a minute is ten degrees under what it was ten minutes ago, and a look back shorter by the filling part would never see it), and longer by its
/// twentieth at most. Twenty parts cost eighty-four bytes for each of the two, where a reading a second over ten minutes costs two kilobytes of a heap the board has little of.
pub const FLUE_PARTS: usize = 20;

/// What the flue has done since work began, as the two signs of the fuel not arriving ask it: how long it has stood under the cold mark, and the highest and
/// the lowest it has reached in the window. Dropped whole whenever the state changes - the clocks of a state are its clocks (see [`Burning::enter`]).
#[derive(Clone, Copy)]
struct FlueWatch {
    /// How long the flue has stood under `flue_cold_c` without a break, in seconds.
    cold_s: f32,
    /// The highest flue of each part of the window, the newest part first; negative infinity - a part nothing was read in.
    peaks: [f32; FLUE_PARTS + 1],
    /// The lowest flue of each part of the window, the newest part first; positive infinity - a part nothing was read in.
    lows: [f32; FLUE_PARTS + 1],
    /// How far into the newest part the window is, in seconds.
    part_s: f32,
}

impl FlueWatch {
    const NEW: Self = Self { cold_s: 0.0, peaks: [f32::NEG_INFINITY; FLUE_PARTS + 1], lows: [f32::INFINITY; FLUE_PARTS + 1], part_s: 0.0 };

    /// A reading of CELSIUS that lasted SECS, in a window of WINDOW_S seconds. Answers the highest and the lowest flue of the window, this reading in it.
    fn read(&mut self, celsius: f32, secs: f32, window_s: f32) -> (f32, f32) {
        let part_s = window_s / FLUE_PARTS as f32;
        self.part_s += secs;
        if self.part_s >= part_s {
            // the parts that ended since the last reading - and no more than the window holds, whatever the reading lasted
            let ended = ((self.part_s / part_s) as usize).min(FLUE_PARTS + 1);
            self.part_s %= part_s;
            self.peaks.rotate_right(ended);
            self.peaks[..ended].fill(f32::NEG_INFINITY);
            self.lows.rotate_right(ended);
            self.lows[..ended].fill(f32::INFINITY);
        }
        self.peaks[0] = self.peaks[0].max(celsius);
        self.lows[0] = self.lows[0].min(celsius);
        (self.peaks.iter().copied().fold(f32::NEG_INFINITY, f32::max), self.lows.iter().copied().fold(f32::INFINITY, f32::min))
    }
}

/// The burner, burning in order.
pub struct Burning {
    state: State,
    /// How long it has been in this state, in seconds.
    for_s: f32,
    /// The button counts the fire last took, to tell a press that is new from one it has seen. **THE FIRE KEEPS NO SWITCH**: whether a
    /// person asked for heat is the thermostat's (`Order::switch`), and what the fire takes from a button is the edge between
    /// two counts. It does not survive a power cut - see [`Burning::resumed`].
    taken: Presses,
    /// The level the fire is carried on at work, as the last order gave it; None before the first. **THE ORDER'S AND NOT THE
    /// FIRE'S**: the table of steps and the choice of a row are the thermostat's, and the fire burns on what it was handed.
    level: Option<Step>,
    /// Where the auger is in its cycle.
    feeder: Feeder,
    /// How long the flame has been gone while the burner was carrying a fire.
    lost_s: f32,
    /// When, in seconds of this rising, the flame last came back and has held since; None - it is out now. See
    /// [`Tuning::rise_s`].
    lit_from: Option<f32>,
    /// What the flue has done at work, for the signs of the fuel not arriving (see [`Tuning::flue_cold_c`]).
    flue: FlueWatch,
    /// How long a flame is still to be watched, in seconds, before it counts as a fire; None - it is not being watched.
    ///
    /// **ONE COUNT FOR TWO FLAMES THAT HAVE BEEN SEEN ONCE.** In `Rising` it is the fire a board met after a restart, which is watched
    /// before it is fed ([`Tuning::sure_s`]); in `Kindling` it is the flame of an attempt, counted from its first reading and
    /// started again by a dark one ([`Tuning::lit_s`]).
    watching: Option<f32>,
    /// How long the fan has been asked to hold a setpoint without a break, in seconds; nought while it stands or
    /// purges. See [`Tuning::spin_up_s`].
    held_s: f32,
    /// How long the sensor has said "under my zero" in a row while the fan was asked to hold a setpoint, in seconds. See [`Tuning::low_pressure_s`].
    low_s: f32,
    /// How long the draught has given no number in a row, in seconds, whatever the fan was asked: what the journal says about a burning off by the draught.
    no_number_s: f32,
    /// When, in seconds of this burning off, the fire was over and the pot began to be blown clear; None - it is
    /// still burning out. See [`Tuning::burn_off_s`].
    blowing_from: Option<f32>,
    /// How long the flame has been gone in a row while the fire burns out, in seconds. See [`Tuning::burn_out_min_s`].
    dark_s: f32,
    /// Whether the sensor sees a flame this second. **THE FAN DOES NOT STAND OVER ONE IN ANY STATE**:
    /// a stop and a fault keep it turning while this holds - see [`Burning::air`].
    alight: bool,
    /// Whether the flame sensor has gone quiet this second: nothing is fed or lit on a flame nobody can
    /// see, however dark or bright it last was.
    flame_quiet: bool,
    /// Whether the pump runs. A state of its own, for its hysteresis: the same water reads on inside it.
    pump: bool,
    /// Why, as of the last second.
    pump_why: PumpWhy,
    /// How many attempts at lighting a lost flame again have been spent; nought - none, and a lighting is a first one.
    /// **ONE LOST FLAME'S COUNT**: kept through the attempts and the rising after
    /// them, and dropped when the fire reaches work or the burner leaves the fire for any other state - see `enter`.
    relit: u32,
    /// Whether this lost flame is lit from its embers, with no plug: the flue stood at `relight_hot_c` or above the
    /// second the flame was taken for lost. **ONE LOST FLAME'S**, kept and dropped with `relit`.
    embers: bool,
    /// Whether the auger may turn. **A PERSON'S BAN AND NOT A STATE**: pellets are in
    /// the pot, the first attempt did not light, and the next doses would pour more on top. Banned, every state runs
    /// as it would, and a dose is time the auger stands. Not remembered across a power cut: a board comes up allowed.
    feed_allowed: bool,
}

impl Burning {
    /// A board that has come up remembering nothing: stopped, and not asking for anything.
    pub fn new() -> Self {
        Self {
            state: State::Stop,
            for_s: 0.0,
            taken: Presses::default(),
            level: None,
            feeder: Feeder::default(),
            lost_s: 0.0,
            lit_from: None,
            flue: FlueWatch::NEW,
            watching: None,
            held_s: 0.0,
            low_s: 0.0,
            no_number_s: 0.0,
            blowing_from: None,
            dark_s: 0.0,
            alight: false,
            flame_quiet: false,
            pump: false,
            pump_why: PumpWhy::NoNumber,
            relit: 0,
            embers: false,
            feed_allowed: true,
        }
    }

    /// Whether the auger may turn.
    pub fn feed_allowed(&self) -> bool {
        self.feed_allowed
    }

    /// A person allows or bans the feed; the next thing told to the organs obeys it.
    pub fn allow_feed(&mut self, allowed: bool) {
        self.feed_allowed = allowed;
    }

    /// A board that has come up and remembers WAS, looking at what the sensors say now.
    ///
    /// **THE SENSORS DECIDE WHICH WAY IT GOES, AND MEMORY ONLY FILLS IN THE DETAIL.** What the board
    /// remembers is an INTENTION, recorded before it went dark for a length of time it does not know; what
    /// the flame says is what is HAPPENING. Where the two disagree the sensor wins, and this is not a
    /// preference - it is the stage's own rule that a reading taken after a failure is a hypothesis while an
    /// instrument is a fact.
    ///
    /// **SO ANY SIGN OF FIRE AT A BOOT GETS AIR, whatever the memory says.** Not a stop, and not the middle
    /// of whatever was interrupted: burning off, with the fan blowing. The board cannot know how much fuel is
    /// in the pot, and of the two ways to be wrong the cheap one is air under a pot that turned out to be
    /// dark - the expensive one is fuel alight under a fan that is not turning, which is the worst thing
    /// this burner can do.
    ///
    /// **AND A HOT FLUE IS A SIGN OF FIRE EVEN WHEN THE FLAME SAYS NOTHING.** On this circuit a broken wire
    /// reads as darkness and nothing tells the two apart (`flame::TELLS_A_BROKEN_WIRE`), so "no flame over a
    /// flue a cold pot cannot make" is the exact shape a fallen-off sensor takes over a pot that is burning.
    /// Twenty seconds of fan on a restart that happened while the flue was still warm from an ordinary stop
    /// is what that costs, and it is the same bargain as the line above.
    ///
    /// **A FIRE THE BOARD REMEMBERS AND SEES IS CARRIED ON AS IT WAS**: memory of
    /// work over a flame is work, fed from the first second - the second live lighting went through a watch and a
    /// rising with the auger standing after every reset, and the fire sat down. Rising and smouldering carry on the
    /// same way.
    ///
    /// **A FIRE THE BOARD REMEMBERS AND DOES NOT SEE IS WAITED FOR**: it comes up in
    /// rising under watch, with the auger standing, and the flame is given `flame_lost_s` like any other flame gone -
    /// back, it is watched and carried into work; not back, the fire is burnt off. The live lighting of 24.09 came up remembering work over a dark pin and a flue of 23.5 °C,
    /// burnt off at once, and the fire was burning: the pin read 0.6-0.9 V half a minute later. Neither the pin in
    /// its first second nor the flue is a verdict about a pot that went on burning while the board was dark.
    ///
    /// It does not ask for heat either, however sure it was before: asking is the thermostat's (`thermostat::Thermostat::restored`),
    /// and the switch a person left in the flash comes back there - a Start pressed before the power went lights the
    /// burner again when the water asks, a Stop stays a stop, and a board nobody pressed anything on asks for nothing. A fault
    /// is the fire's and waits for a person's Stop whatever the switch says.
    pub fn resumed(was: State, seen: &Seen, t: &Sound) -> Self {
        let t = &t.0;
        let mut burning = Self::new();
        let alight = matches!(seen.flame, Flame::Weak | Flame::Steady);
        let flue_warm = seen.flue_c.is_some_and(|celsius| celsius >= t.flue_holds_c);
        // memory carries no verdict of its own - only whether a fire was being carried, and what the burner
        // was already unhappy about
        let carrying = matches!(was, State::Rising | State::Working | State::Smouldering);
        let why = match was {
            State::BurningOff(why) => why,
            State::Fault(why) => Some(why),
            _ => None,
        };
        burning.state = if carrying && alight {
            // seen at its first second: carried on as it was, rising from the bottom of its climb
            burning.lit_from = Some(0.0);
            was
        } else if carrying {
            // waited for over a dark pot, and a flame that shows is watched `sure_s` before the climb
            burning.watching = Some(t.sure_s);
            State::Rising
        } else if alight || flue_warm {
            State::BurningOff(why)
        } else {
            match was {
                // no sign of fire anywhere, and nothing was meant to be burning: the one case where every
                // witness agrees on a burner with nothing in it
                State::Stop | State::Purge => State::Stop,
                _ => State::BurningOff(why),
            }
        };
        burning
    }

    /// What will end in this state and in how many seconds - the clocks a person sees count down on the main screen:
    /// the igniter, the dose, the attempt, the purge. None of them in a state that ends on no clock.
    ///
    /// Counted from the same numbers the state is ended by, so a clock reads nought exactly when its thing ends.
    ///
    /// **A STATE SHOWS THE SAME CLOCKS EVERY SECOND IT LASTS**: a clock that went away when it ran out
    /// moved every row under it up, and the list jumped while a person was reading it. One that has run out stays
    /// and reads nought; one that has not begun stays and has no number (`left_s` None) - the blow behind a burning
    /// out, the watch over a flame that is dark, the climb of a flame that has just blinked. A number that stood whole
    /// every second there was counted down by the page and said whole again by the board; the page
    /// draws a dash, and the clock reads a number from the second its thing begins.
    pub fn clocks(&self, t: &Sound) -> Vec<Clock> {
        let t = &t.0;
        let mut clocks = Vec::new();
        let for_s = self.for_s;
        let mut left = |what: &'static str, left_s: Option<f32>| clocks.push(Clock { what, left_s });
        let until = |end_s: f32| (end_s - for_s).max(0.0);
        // A PART OF AN ATTEMPT RUNS THROUGH ITS END SECOND: the dose turns while for_s <= dose_end, the plug has no
        // air while for_s <= heat_end and burns while for_s <= plug_end (`organs`, `air`), so it changes in the first
        // second past its end - one later than a state, which ends when for_s reaches it. Counted plainly, its clock
        // read nought through its own last second, and the page drew the plug "burning" in still air (stand, 24.09).
        // A part of no seconds never runs - a dose over a pot the work fed, a heat over a hot pot -
        // and reads done from its first second.
        let past = |end_s: f32| if end_s > 0.0 && for_s <= end_s { (end_s - for_s).floor() + 1.0 } else { 0.0 };
        match self.state {
            State::Purge => left("lighting in", Some(until(t.purge_before_s))),
            State::Kindling(try_no) => {
                let a = self.attempt(try_no, t);
                left("dose ends in", Some(past(a.dose_end)));
                left("igniter off in", Some(past(a.plug_end)));
                left("attempt ends in", Some(until(a.end)));
            }
            State::Rising => match self.watching {
                // counts while the flame shows (`step` takes a second off the watch then) and begins again at the first dark second
                Some(watch) => left("fed after watching the flame for", self.alight.then(|| until(self.for_s + watch))),
                // counted from the flame's last return; a dark second takes `lit_from` away and the climb has not begun
                None => left("full step in", self.lit_from.map(|from| until(from + t.rise_s))),
            },
            State::Smouldering if t.smoulder_s > 0.0 => left("burning off in", Some(until(t.smoulder_s))),
            State::BurningOff(_) => match self.blowing_from {
                None => {
                    left("burning out ends in at most", Some(until(t.burn_off_s)));
                    left("blowing clear ends in", None);
                }
                Some(from) => {
                    left("burning out ends in at most", Some(until(self.for_s)));
                    left("blowing clear ends in", Some(until(from + t.purge_after_s)));
                }
            },
            _ => {}
        }
        // THE AUGER'S CYCLE, ONE CLOCK FOR BOTH HALVES: whether the half is a dose or a pause is
        // the card's row `auger`, and one name for both keeps the list of clocks from jumping at every half.
        if matches!(self.state, State::Working | State::Smouldering) {
            clocks.push(Clock { what: "auger switches in", left_s: Some(self.feeder.left_s.max(0.0)) });
        }
        // while the flame is gone in rising or at work: how long it has left to come back, and what follows if it does
        // not - an attempt at lighting it again, or the burning off of a board that has just come up and lights nothing
        if matches!(self.state, State::Rising | State::Working | State::Smouldering) && self.lost_s > 0.0 {
            let what = if self.watching.is_some() { "flame not back, burning off in" } else { "flame not back, lighting again in" };
            clocks.push(Clock { what, left_s: Some((t.flame_lost_s - self.lost_s).max(0.0)) });
        }
        clocks
    }

    /// What it is doing.
    pub fn state(&self) -> State {
        self.state
    }

    /// The seconds the draught has been counted by, for the journal's line about a burning off by the draught: how long the fan has been held to a setpoint, how long the sensor has given no number, and how long it has said "under my zero".
    pub fn draught_counts(&self) -> DraughtCounts {
        DraughtCounts { held_s: self.held_s, no_number_s: self.no_number_s, low_s: self.low_s }
    }

    /// Lets go of a fault of the reason WHY, as a person's Stop does and with no press: the burner stands in a stop. Any other state, and a fault of another reason, is left as it is and the answer is false. This is the one road by which the board leaves a fault
    /// by itself; the rule for when it may is [`crate::Relief`]'s, and it only ever asks for the fault for want of fronts.
    pub fn clear_fault(&mut self, why: Why) -> bool {
        if self.state != State::Fault(why) {
            return false;
        }
        self.enter(State::Stop);
        true
    }

    /// What a person pressed since the fire last took an order: the edge the order's counts make against the ones it took.
    pub fn told(&self, order: &Order) -> Told {
        order.presses.since(self.taken)
    }

    /// The level the fire is carried on: the last order's, and a level that burns nothing before the first one.
    fn level(&self) -> Step {
        self.level.unwrap_or(Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 1.0 } })
    }

    /// One second: what the thermostat orders, what the board sees, how long since the last step.
    ///
    /// **WHAT THE ORDER DOES TO THE FIRE, BY THE STATE IT IS IN**: a stop lights on `Burn` when a person asked for heat
    /// (`Switch::On`), and on a Light at once; a working fire goes down to smouldering on `Enough`; a smouldering one works again
    /// on `Burn` and on `Level`; `Keep` changes nothing; `Silent` burns the fire off with the sensor's fault; a Stop burns a live
    /// fire off and acknowledges a fault. The fire knows no setpoint and no hysteresis: the thermostat said which side of them the
    /// water stood on.
    pub fn step(&mut self, order: &Order, seen: Seen, secs: f32, t: &Sound) -> Doing {
        let t = &t.0;
        // The edge of a button, taken first: the counts move once, and the fire sees a press once.
        let told = self.told(order);
        self.taken = order.presses;
        self.level = Some(order.step);
        let alarm = seen.boiler_c.is_some_and(|celsius| celsius >= t.alarm_c);
        self.pump_why = match seen.boiler_c {
            None => PumpWhy::NoNumber,
            Some(celsius) if celsius >= t.too_hot_c => PumpWhy::Overheated(celsius),
            Some(celsius) => PumpWhy::Water(celsius),
        };
        self.pump = match seen.boiler_c {
            None => true,
            Some(celsius) if celsius >= t.too_hot_c || celsius >= t.pump_on_c => true,
            Some(celsius) => self.pump && celsius >= t.pump_on_c - t.pump_off_below_c,
        };
        let alight = matches!(seen.flame, Flame::Weak | Flame::Steady);
        self.alight = alight;
        self.flame_quiet = seen.silence.flame.quiet();
        self.for_s += secs;
        let holding = matches!(self.air(t), Asked::Hold(pa) if pa > 0.0);
        self.held_s = if holding { self.held_s + secs } else { 0.0 };
        self.low_s = if holding && seen.draught_low { self.low_s + secs } else { 0.0 };
        self.no_number_s = if seen.draught_pa.is_none() { self.no_number_s + secs } else { 0.0 };

        match self.state {
            // **A PROCEDURE THAT MAY NOT BE INTERRUPTED, and nothing below is even asked while it runs.** Not
            // by a person pressing start, not by a fault arriving late: there is fuel in the pot and the only
            // way out is through. Its two ends are the flame going and the clock running out, whichever comes
            // first (Plum, manual 8.7).
            State::BurningOff(why) => {
                // Two parts: the fire burns out until the flame goes or the clock runs out, whichever is first,
                // and then the pot is blown clear for as long as a stop is always blown for (see below the match,
                // where the blow begins).
                if self.blowing_from.is_some_and(|from| self.for_s - from >= t.purge_after_s) {
                    self.enter(match why {
                        Some(why) => State::Fault(why),
                        None => State::Stop,
                    });
                }
            }
            // A fault waits for a person and for nothing else - no clock takes it away, and asking for heat
            // does not answer it. Plum says the same in its own words: after a failed kindling, automatic
            // continuation is not possible (manual 16.6).
            State::Fault(_) => {
                if told == Told::Stop {
                    self.enter(State::Stop);
                }
            }
            State::Stop => {
                // The stage's table gives two ways out of a stop and they are an OR: a person's Light, or the water.
                // The water needs the switch on - a board nobody has asked for heat does not light itself
                // because the weather turned.
                let cold_enough = order.water == Water::Burn;
                if told == Told::Light || (order.switch == Switch::On && cold_enough) {
                    self.enter(State::Purge);
                }
            }
            live => self.live(live, told, order, seen, alight, secs, t),
        }
        // THE BLOW BEGINS WHEN THE FIRE IS OVER: the flame gone `burn_out_min_s` in a row, or
        // the ceiling `burn_off_s`, alight or dark - the ceiling is a stop and not a fault
        if matches!(self.state, State::BurningOff(_)) && self.blowing_from.is_none() {
            self.dark_s = if alight { 0.0 } else { self.dark_s + secs };
            let over = !alight && self.dark_s > t.burn_out_min_s;
            if over || self.for_s >= t.burn_off_s {
                self.blowing_from = Some(self.for_s);
            }
        }
        self.organs(secs, alarm, air(seen.gate, seen.draught_pa), t)
    }

    /// One second of a burner that is carrying, or trying to carry, a fire. Everything that can end such a
    /// second early is here, and it is asked BEFORE the state's own business: a burner that has just been
    /// told it cannot read its air has no business deciding whether the power has finished climbing.
    fn live(&mut self, live: State, told: Told, order: &Order, seen: Seen, alight: bool, secs: f32, t: &Tuning) {
        if told == Told::Stop {
            return self.enter(State::BurningOff(None));
        }
        // Nothing the sensor says separates fire from no fire, so nothing may be fed.
        if seen.flame == Flame::Unconfigured {
            return self.enter(State::BurningOff(Some(Why::Unconfigured)));
        }
        if let Some(celsius) = seen.boiler_c {
            if celsius >= t.too_hot_c {
                return self.enter(State::BurningOff(Some(Why::Overheated(celsius))));
            }
        }
        // **A PIN THAT GIVES NO FRONTS IS NAMED BEFORE THE DRAUGHT.** The gate has no phase to fire in, so the fan stands, and a standing fan gives the draught sensor nothing to read: "no draught" is what the pin's silence
        // looks like from the chimney, and a person told to check the chimney is sent to the wrong place. Nothing is waited for: a burner that cannot be given air may not be fed, and `dimmer::Fronts` has already
        // judged the second (one second under the band of a live mains was a loss every one of the three times it happened in the record of 21-25.10).
        if seen.silence.fronts_s.is_some() {
            return self.enter(State::BurningOff(Some(Why::NoFronts)));
        }
        // **THE DRAUGHT IS ASKED FOR BY A SECOND THAT HOLDS A SETPOINT, AND BY NO OTHER.** A burner whose air
        // cannot be measured while it is being given air is a burner that may not be fed - the regulator stops on
        // exactly this and for exactly this reason (`regulator::Blind`), and a stopped fan over a fed pot is the
        // thing neither of them will have. But a purge runs flat out past the regulator, and a fan that stands on
        // purpose - the plug heating in still air, a fire smouldering on the chimney's own pull - gives the
        // sensor nothing to read: its "no number" is the reading of a standing fan, not a fault. Asked of those,
        // kindling fell into NoDraught on its first second. And a fan that has just been given
        // its setpoint has not turned up to it yet: it is asked for a number once `spin_up_s` has passed.
        // **A SENSOR THAT ANSWERS "UNDER MY ZERO" IS NOT A SENSOR THAT GIVES NO NUMBER**: it is counted apart, for `low_pressure_s`, and a number that comes back before the term undoes the count
        if self.held_s > t.spin_up_s && seen.draught_pa.is_none() && !seen.draught_low {
            // the same way out whichever it is; the cause named is the sensor's when its round has stopped
            let why = if seen.silence.draught.quiet() { Why::DraughtSilent } else { Why::NoDraught };
            return self.enter(State::BurningOff(Some(why)));
        }
        if self.low_s > t.low_pressure_s {
            return self.enter(State::BurningOff(Some(Why::LowPressure)));
        }
        // **TEN PERIODS OF SILENCE, AT LEAST 30 S, AND THE FIRE IS BURNT OFF**: the early ways out above
        // catch a quiet sensor sooner where they ask it; this catches it where nothing asks - a fan standing on
        // purpose, a flame waited for, water nobody else needs this second
        if let Some(why) = silent_too_long(seen.silence, order.water) {
            return self.enter(State::BurningOff(Some(why)));
        }

        match live {
            State::Purge => {
                if self.for_s >= t.purge_before_s {
                    self.enter(State::Kindling(1));
                }
            }
            State::Kindling(try_no) => {
                // **A FLAME BACK FROM THE EMBERS IS WORK AGAIN**: the fire never left the pot, the portion
                // only fed it; a flame the plug lit rises as any lighting does
                // **A FLAME HOLDS BEFORE IT IS A FIRE** (27.09: 2791 mV against 2800, dark at the next second): the count is
                // `watching`'s, a dark reading starts it again, and the attempt's own time goes on as it did
                if alight {
                    let left = self.watching.unwrap_or(t.lit_s) - secs;
                    if left <= 0.0 {
                        return self.enter(if self.embers { State::Working } else { State::Rising });
                    }
                    self.watching = Some(left);
                } else {
                    self.watching = None;
                }
                if self.for_s >= self.attempt(try_no, t).end {
                    // A pot that did not light is a pot with fuel already in it, so the next attempt pours
                    // less (Plum cuts it to a tenth, manual 8.4) - see [`Burning::dose`].
                    if self.relit > 0 {
                        self.relight(t);
                    } else if try_no < t.tries {
                        self.enter(State::Kindling(try_no + 1));
                    } else {
                        self.enter(State::BurningOff(Some(Why::NeverLit(t.tries))));
                    }
                }
            }
            State::Rising => {
                if let Some(left) = self.watching {
                    // **A FIRE UNDER WATCH IS WAITED FOR LIKE ANY OTHER, AND STILL NOT FED**:
                    // the sensor flickers after a power cut as it does before one, so a flame gone is given
                    // `flame_lost_s` here too. What the watch withholds is the auger, not the wait - and a flame that
                    // comes back is watched its whole `sure_s` again, since the watch is of a fire seen holding.
                    if self.flame_went(alight, seen.flue_c, secs, true, t) {
                        return;
                    }
                    if !alight {
                        self.watching = Some(t.sure_s);
                        return;
                    }
                    let left = left - secs;
                    if left > 0.0 {
                        self.watching = Some(left);
                    } else {
                        // watched and held: the climb begins now, from its own first second
                        self.watching = None;
                        self.for_s = 0.0;
                        self.lit_from = Some(0.0);
                    }
                    return;
                }
                // the work waits for a flame that has held rise_s without a break: a blink starts the count again
                self.lit_from = match (alight, self.lit_from) {
                    (false, _) => None,
                    (true, None) => Some(self.for_s),
                    (true, since) => since,
                };
                if self.flame_went(alight, seen.flue_c, secs, true, t) {
                    return;
                }
                if self.lit_from.is_some_and(|since| self.for_s - since >= t.rise_s) {
                    self.enter(State::Working);
                }
            }
            State::Working => {
                if self.flame_went(alight, seen.flue_c, secs, true, t) {
                    return;
                }
                // **WHILE THE FLAME IS WAITED FOR, THE FLUE ACCUSES NOBODY**: the wait is the flame's, and a
                // flue that stopped warming because the flame blinked is not a feed that stopped
                if self.lost_s == 0.0 && self.fuel_stopped(seen, secs, t) {
                    return;
                }
                if order.water == Water::Enough {
                    // A supervision time of nothing skips smouldering altogether and goes straight to burning
                    // off - Plum's own way of saying "this boiler does not idle" (manual 8.6).
                    self.enter(if t.smoulder_s > 0.0 { State::Smouldering } else { State::BurningOff(None) });
                }
            }
            State::Smouldering => {
                // **SMOULDERING CARRIES A FIRE TOO**: a flame dark past its wait is lost here as at work,
                // and a pot fed on the smouldering pair until smoulder_s ran out is the thing the wait exists to stop
                if self.flame_went(alight, seen.flue_c, secs, order.house_cold || matches!(order.water, Water::Burn | Water::Level), t) {
                    return;
                }
                // a burner turned to a level holds no temperature and has no reason to smoulder: it works again at once
                if matches!(order.water, Water::Burn | Water::Level) {
                    return self.enter(State::Working);
                }
                if self.for_s >= t.smoulder_s {
                    self.enter(State::BurningOff(None));
                }
            }
            State::Stop | State::BurningOff(_) | State::Fault(_) => {}
        }
    }

    /// A flame that went while the burner was carrying one: the stage goes on as it was, and the fire is lit again if
    /// the flame is not back within `flame_lost_s` - or burnt off, on a board that has just come up and is only
    /// watching the fire. Answers whether the state has been left.
    ///
    /// **RISING COUNTS AS CARRYING A FIRE.** A flame that dies at thirty seconds of climbing is the same event as one
    /// that dies at three minutes of holding, and it waits the same.
    ///
    /// **AND IT IS NOT A VERDICT ABOUT THE FIRE.** The sensor flickers, pellets settle, and a broken
    /// wire reads as darkness with nothing to tell them apart (`flame::TELLS_A_BROKEN_WIRE`) - so darkness earns a
    /// wait on the same feed and air, and the fault only once the wait is over.
    ///
    /// **THE LIGHT ALONE DECIDES THAT THE FLAME IS LOST**: the flue does not hold the verdict back, it only
    /// says, at that second, whether the pot is hot enough to be lit from its embers.
    ///
    /// HEAT_ASKED says whether the burner is where heat is wanted: rising and work always are, smouldering only while the house says cold or the water says burn (`Keep` is the water standing inside its hysteresis, and says
    /// nothing of the house). A lost flame with none asked goes to burning off, not to a light.
    fn flame_went(&mut self, alight: bool, flue_c: Option<f32>, secs: f32, heat_asked: bool, t: &Tuning) -> bool {
        if alight {
            self.lost_s = 0.0;
            return false;
        }
        self.lost_s += secs;
        // dark up to flame_lost_s is a blink, and the boundary with it: lost is LONGER than the wait
        if self.lost_s <= t.flame_lost_s {
            return false;
        }
        // **A FLAME NOBODY CAN SEE IS NOT LIT AGAIN**: neither from embers nor with the plug; the silence
        // itself ends the fire once it has lasted too long
        if self.flame_quiet {
            return false;
        }
        // **A BOARD THAT HAS COME UP LIGHTS NOTHING** (see the header): the fire it is watching is one it has seen once
        if self.watching.is_some() {
            self.enter(State::BurningOff(Some(Why::FlameLost)));
        } else if !heat_asked {
            // **A FIRE THAT WENT OUT WHEN NO HEAT WAS ASKED IS NOT BLOWN UP**: the house is warm or the water is over its line, the fire was on its way down already, and the ordinary
            // way to a stop (burning off, the blow, the stop) is the one it takes - no fault, as when smouldering runs out
            self.enter(State::BurningOff(None));
        } else {
            // **HOT OR COLD IS DECIDED ONCE, THE SECOND THE FLAME IS TAKEN FOR LOST**: a flue at
            // relight_hot_c or above has embers under it, and they are given the doses with no plug
            self.embers = flue_c.is_some_and(|celsius| celsius >= t.relight_hot_c);
            self.relight(t);
        }
        true
    }

    /// The next attempt at lighting a lost flame again, or - the attempts spent - the burning off that ends in the fault.
    ///
    /// **LIT AGAIN, NOT BURNT OFF**: the fire may have gone out because the pellets did
    /// not arrive, and everybody with an answer lights it again - Plum by its own lighting with a cut dose and three
    /// attempts, Burnit by a new lighting. Between the portions the auger stands.
    ///
    /// **THE DOSES ARE THE CARD'S, ONE PER ATTEMPT** (20, 10, 10 s, `relight_s`), and **A HOT POT IS LIT
    /// FROM ITS EMBERS**: over a flue at `relight_hot_c` or above the three attempts have no plug,
    /// each a dose and the wait for a flame, and a flame back in any of them is work again; embers that did not take are
    /// given the plug once, with nothing poured over three doses. A cold pot has its plug at every attempt, with air only.
    /// Then the fire is burnt off.
    fn relight(&mut self, t: &Tuning) {
        let doses = t.relight_s.len() as u32;
        let plug_after_embers = self.embers && self.relit == doses;
        if self.relit < doses || plug_after_embers {
            self.relit += 1;
            if plug_after_embers {
                self.embers = false;
            }
            self.enter(State::Kindling(self.relit));
        } else {
            self.enter(State::BurningOff(Some(Why::FlameNotBack(self.relit))));
        }
    }

    /// Whether the flue says the fuel has stopped arriving. Answers whether the state has been left.
    ///
    /// **ASKED IN `Working` AND NOWHERE ELSE, and the reason is the flue rather than the code.** A burner
    /// that is lighting or climbing has a cold flue for an honest reason, so the same silence there accuses
    /// a feed that is working perfectly.
    ///
    /// **A COLD FLUE OR A FALLING ONE, NEVER A FLUE THAT DOES NOT CLIMB**: Plum asks the climb right after a feed, when it is bound to
    /// come; asked of a fire at work it accuses every low fire, whose flue stands at 93-105 °C (27.09). What a feed that stopped does is
    /// let the flue cool, and the two ways of seeing it are both here, as Plum ends a burning off by the flame or by the clock, whichever
    /// is first: under `flue_cold_c` for the whole window, or fallen `flue_fall_c` from the highest of the window.
    ///
    /// **THE LIGHT DECIDES THAT THE FIRE IS OVER AND THE FLUE ONLY HELPS**: a flue
    /// is slow and answers to the power, the water and the step as much as to the fuel - a cold start warms it from 20 °C, a step down at the
    /// setpoint cools it - so it puts a fire out only when the flame is not steady either. A steady second starts the window again, as a
    /// reading nobody took does; a fire whose fuel ran out dims first, and the two signs meet.
    ///
    /// **A COLD FLUE THAT IS RISING IS A FIRE THAT IS ALIVE**: a low fire under a dim flame warms its flue slowly, and the flue
    /// under `flue_cold_c` for the whole window accuses the fuel only if it has not risen `flue_fall_c` from its lowest in that window - the same
    /// number as the fall, so the card has no new one. The fall is asked as it was.
    fn fuel_stopped(&mut self, seen: Seen, secs: f32, t: &Tuning) -> bool {
        if seen.flame == Flame::Steady {
            self.flue = FlueWatch::NEW;
            return false;
        }
        let Some(celsius) = seen.flue_c else {
            // No number is not a cold flue, and a burner is not accused on a reading nobody took.
            self.flue = FlueWatch::NEW;
            return false;
        };
        let (highest, lowest) = self.flue.read(celsius, secs, t.flue_fall_s);
        self.flue.cold_s = if celsius < t.flue_cold_c { self.flue.cold_s + secs } else { 0.0 };
        let cold_and_not_rising = self.flue.cold_s >= t.flue_fall_s && celsius - lowest < t.flue_fall_c;
        if cold_and_not_rising || highest - celsius >= t.flue_fall_c {
            self.enter(State::BurningOff(Some(Why::NoFuel)));
            return true;
        }
        false
    }

    /// Moves to STATE and drops everything that belonged to the one being left.
    ///
    /// **THE CLOCKS OF A STATE ARE ITS CLOCKS.** Carrying one across a change is how a burner ends up judging
    /// a new state by seconds spent in the old one - the same trap the regulator closed by making its
    /// `Manoeuvre` one type that is dropped whole.
    fn enter(&mut self, state: State) {
        self.state = state;
        self.for_s = 0.0;
        self.feeder = Feeder::default();
        self.flue = FlueWatch::NEW;
        self.lost_s = 0.0;
        // only rising reads it, and rising is entered on a flame seen this second
        self.lit_from = Some(0.0);
        self.watching = None;
        self.blowing_from = None;
        self.dark_s = 0.0;
        // a lost flame's attempts run through the lighting and the rising after it; anything else ends them
        if !matches!(state, State::Kindling(_) | State::Rising) {
            self.relit = 0;
            self.embers = false;
        }
    }

    /// Where the parts of attempt TRY_NO end, in seconds of the attempt: the dose, the plug's short heat in still
    /// air, the plug burning with air, the wait for a flame.
    fn attempt(&self, try_no: u32, t: &Tuning) -> Attempt {
        let dose_end = self.dose(try_no, t);
        let plug_end = dose_end + if self.embers { 0.0 } else { t.igniter_s };
        Attempt { dose_end, plug_end, end: plug_end + t.wait_for_flame_s }
    }

    /// The dose an attempt at lighting pours, in seconds of auger: whole for the first attempt at a cold pot, cut by
    /// `dose_after_first` for its next ones, and the card's own doses for a lost flame's attempts.
    fn dose(&self, try_no: u32, t: &Tuning) -> f32 {
        if self.relit > 0 {
            // a lost flame's attempts pour the card's doses in turn, and the plug after the embers pours nothing
            t.relight_s.get(self.relit as usize - 1).copied().unwrap_or(0.0)
        } else if try_no <= 1 {
            t.kindling.dose_s
        } else {
            t.kindling.dose_s * t.dose_after_first
        }
    }

    /// What the fan is asked this second, in the state the burner is in now. One answer for the organs and for the
    /// question whether the draught has to be read.
    fn air(&self, t: &Tuning) -> Asked {
        match self.state {
            // a fan that cannot be fired is asked for nothing: the gate would refuse every second, and the log would say it every second
            State::BurningOff(Some(Why::NoFronts)) | State::Fault(Why::NoFronts) => Asked::Stop,
            // a flame the sensor sees keeps the fan turning, stopped or at fault: on the step's draught, or at full past
            // the regulator where the draught cannot be read
            State::Fault(Why::NoDraught) if self.alight => Asked::Purge,
            State::Stop | State::Fault(_) if self.alight => Asked::Hold(self.level().pa),
            State::Stop | State::Fault(_) => Asked::Stop,
            State::Purge => Asked::Purge,
            // a pot being blown clear, and a fire burning out over a draught sensor that gives no number - the
            // regulator stops a fan it cannot measure - are blown at full
            State::BurningOff(why) if self.blowing_from.is_some() || why == Some(Why::NoDraught) => Asked::Purge,
            // **THE FIRE BURNS OUT ON THE STEP'S DRAUGHT**, and the full blow comes only
            // at the end
            State::BurningOff(_) => Asked::Hold(self.level().pa),
            State::Kindling(try_no) => match self.for_s > self.attempt(try_no, t).dose_end {
                true => Asked::Hold(t.kindling.pa),
                false => Asked::Stop,
            },
            State::Rising if self.watching.is_some() => Asked::Hold(t.kindling.pa),
            State::Rising | State::Working => Asked::Hold(self.pa_now(t)),
            State::Smouldering => Asked::Hold(t.smoulder.pa),
        }
    }

    /// The draught the fire is carried on this second, in Rising and Working: rising climbs from the lighting's to the
    /// step's over `rise_s`, and a flame gone does not change it.
    fn pa_now(&self, t: &Tuning) -> f32 {
        match self.state {
            State::Rising => {
                let at = (self.for_s / t.rise_s).clamp(0.0, 1.0);
                t.kindling.pa + (self.level().pa - t.kindling.pa) * at
            }
            _ => self.level().pa,
        }
    }

    /// A second of the work's feeding by FEED: whether the auger turns at the start of it, and the plan of the cycle. **THE CYCLE KEEPS THE NUMBERS IT BEGAN ON** and takes the tuning's at its boundary, so a plan always carries the numbers of the cycle under way.
    fn fed(&mut self, feed: Feed, secs: f32) -> (bool, bool, Feeding) {
        (self.feeder.second(feed, secs), false, self.feeder.plan(feed))
    }

    /// What the organs are told this second. The feeder is advanced by one second here and nowhere else.
    fn organs(&mut self, secs: f32, alarm: bool, blowing: bool, t: &Tuning) -> Doing {
        let fan = self.air(t);
        let (auger, igniter, feeding) = match self.state {
            State::Stop | State::Purge => (false, false, Feeding::Stop),
            // **THE DOSE FIRST, THEN THE PLUG - AND THE PLUG BURNS ONLY WITH AIR**:
            // it waits for the fan to blow, and goes off the second the draught goes - a stall, a tube off, a
            // sensor gone. Three attempts of ten seconds of heat in still air burnt a plug through on 26.09.
            State::Kindling(try_no) => {
                let a = self.attempt(try_no, t);
                // a dose is under way in every second that begins before its end, so a dose of 0.8 s is told for one second
                let pouring = self.for_s > 0.0 && self.for_s - secs < a.dose_end;
                let hot = self.for_s > a.dose_end && self.for_s <= a.plug_end && blowing;
                // the plan is what is left of the dose by this count, from now: the relay's task follows it, and a dose the automaton counts as poured is never begun again
                (pouring, hot, if self.for_s < a.dose_end { Feeding::Dose { left_s: a.dose_end - self.for_s } } else { Feeding::Stop })
            }
            // **RISING FEEDS NOTHING**: the lighting's dose filled the pot above the
            // plug, and the fire eats it before anything is added
            State::Rising => (false, false, Feeding::Stop),
            // **A BLINK CHANGES NOTHING**: the pair runs on, lit or dark, until the flame is taken for lost
            State::Working => self.fed(self.level().feed, secs),
            State::Smouldering => self.fed(t.smoulder.feed, secs),
            // Full air and no feedback, which is what lets it run on a board that has just declared its
            // draught sensor untrustworthy - see the header.
            State::BurningOff(_) => (false, false, Feeding::Stop),
            State::Fault(_) => (false, false, Feeding::Stop),
        };
        // a banned feed stands the auger whatever the state wanted of it; the dose's clock runs on as it would
        let (auger, feeding) = if self.feed_allowed { (auger, feeding) } else { (false, Feeding::Stop) };
        // **A FLAME NOBODY CAN SEE IS NOT FED AND NOT LIT**: stricter than darkness, which a speaking
        // sensor reports and the pair runs on through; the clocks run on as they would
        let (auger, igniter, feeding) = if self.flame_quiet { (false, false, Feeding::Stop) } else { (auger, igniter, feeding) };
        Doing { state: self.state, auger, feeding, igniter, fan, alarm, pump: self.pump, pump_why: self.pump_why }
    }
}

impl Default for Burning {
    fn default() -> Self {
        Self::new()
    }
}

/// What a state says to a person. The name of the thing happening and nothing else - the numbers beside it are
/// the card's business (`crates/page`). In English, as every other word on the burner's page is.
pub fn state_words(state: State) -> String {
    match state {
        State::Stop => "stopped".into(),
        State::Purge => "purging".into(),
        State::Kindling(try_no) => format!("lighting, attempt {try_no}"),
        State::Rising => "rising".into(),
        State::Working => "working".into(),
        State::Smouldering => "smouldering".into(),
        State::BurningOff(None) => "burning off".into(),
        State::BurningOff(Some(why)) => format!("burning off: {}", words(why)),
        State::Fault(why) => format!("fault: {}", words(why)),
    }
}

/// What a fault says to a person. **IT NAMES THE THING THAT DID NOT ADD UP, not a code**: a number on that row
/// sends a person to a table he does not have, which is what Ecoforest's bare `estado` does to us
/// (`02_REF_Ecoforest_Controller.md`).
pub fn words(why: Why) -> String {
    match why {
        Why::NeverLit(tries) => format!("did not light in {tries} attempts"),
        Why::FlameLost => "the flame went out and did not come back".into(),
        Why::FlameNotBack(tries) => format!("the flame did not come back after {tries} attempts at lighting it again - look into the pot"),
        Why::NoFuel => "no fuel arriving: the flue is cold or falling at work".into(),
        Why::NoDraught => "the draught sensor gives no number".into(),
        Why::LowPressure => "the draught stays under the sensor's zero".into(),
        Why::Overheated(celsius) => format!("overheated: water at {celsius:.0} °C"),
        Why::Unconfigured => "the flame's thresholds are not set".into(),
        Why::DraughtSilent => "the draught sensor is silent".into(),
        Why::FlameSilent => "the flame sensor is silent".into(),
        Why::WaterSilent => "the water sensor is silent".into(),
        Why::ReturnSilent => "the return sensor is silent".into(),
        Why::NoFronts => "the zero-crossing pin gives no fronts".into(),
    }
}


impl Step {
    /// The three numbers of a level, under the name the record gives this level.
    pub fn sound(&self, named: &str) -> Result<(), Refused> {
        // nought is a level too - see [`Step::pa`]
        not_below_nought(&format!("{named}.pa"), self.pa)?;
        above_nought(&format!("{named}.feed.on_s"), self.feed.on_s)?;
        above_nought(&format!("{named}.feed.off_s"), self.feed.off_s)?;
        Ok(())
    }
}

impl Tuning {
    /// The door. Every number of the fire goes through it once, and what comes out is the only thing
    /// [`Burning`] will burn by.
    ///
    /// **THE FIRST REFUSAL, IN THE ORDER THE FIELDS ARE WRITTEN.** A person fixes one field at a time on a
    /// card, and a message that named six at once would name none of them legibly.
    ///
    /// **THE LIMIT OF EACH FIELD IS WRITTEN WHERE THE FIELD IS EXPLAINED**, and the line below only asks it.
    /// A table of limits kept apart from the fields is a second copy of the same truth, and the two would
    /// part company the first time one of them moved.
    ///
    /// **NOT ONE FIELD HAS AN UPPER LIMIT OF ITS OWN.** How many seconds a purge may last is the owner's, and
    /// a ceiling invented here would be exactly the thing the header of this crate forbids - a number of the
    /// fire compiled into the image. The four limits that look like ceilings are about the ORDER of two
    /// numbers, not about the size of either: burning off may not end sooner than its own floor, the
    /// hysteresis may not be as wide as the setpoint, and overheating and its alarm stand above the setpoint
    /// in that order.
    pub fn sound(self) -> Result<Sound, Refused> {
        above_nought("purge_before_s", self.purge_before_s)?;
        above_nought("purge_after_s", self.purge_after_s)?;
        above_nought("igniter_s", self.igniter_s)?;
        above_nought("wait_for_flame_s", self.wait_for_flame_s)?;
        if self.tries < 1 {
            return Err(Refused::of("tries", "no_tries", self.tries as f32));
        }
        above_nought("dose_after_first", self.dose_after_first)?;
        if self.dose_after_first > 1.0 {
            return Err(Refused::of("dose_after_first", "dose_over_first", self.dose_after_first));
        }
        above_nought("rise_s", self.rise_s)?;
        above_nought("flame_lost_s", self.flame_lost_s)?;
        not_below_nought("smoulder_s", self.smoulder_s)?;
        above_nought("burn_off_s", self.burn_off_s)?;
        not_below("burn_off_s", self.burn_off_s, "purge_after_s", self.purge_after_s)?;
        not_below_nought("burn_out_min_s", self.burn_out_min_s)?;
        not_below("burn_off_s", self.burn_off_s, "burn_out_min_s", self.burn_out_min_s)?;
        above_nought("flue_holds_c", self.flue_holds_c)?;
        above_nought("flue_cold_c", self.flue_cold_c)?;
        above_nought("flue_fall_c", self.flue_fall_c)?;
        above_nought("flue_fall_s", self.flue_fall_s)?;
        above_nought("lit_s", self.lit_s)?;
        // **A NUMBER BEFORE AN ORDER, AND NEVER AN ORDER ALONE.** Comparing with `NaN` is false whichever
        // way it is written, so a field guarded only by "stands above that one" lets it straight through -
        // which is exactly what happened here until a row walked the whole record.
        above_nought("too_hot_c", self.too_hot_c)?;
        above_nought("alarm_c", self.alarm_c)?;
        // STRICTLY OVER the overheating: an alarm that sounds at the very water the pump is let run at is no second line
        above("alarm_c", self.alarm_c, "too_hot_c", self.too_hot_c)?;
        above_nought("sure_s", self.sure_s)?;
        for (i, dose) in self.relight_s.iter().enumerate() {
            not_below_nought(&format!("relight_s[{i}]"), *dose)?;
        }
        above_nought("relight_hot_c", self.relight_hot_c)?;
        above_nought("pump_on_c", self.pump_on_c)?;
        above_nought("pump_off_below_c", self.pump_off_below_c)?;
        above_nought("spin_up_s", self.spin_up_s)?;
        above_nought("low_pressure_s", self.low_pressure_s)?;
        a_number("kindling.pa", self.kindling.pa)?;
        if self.kindling.pa <= 0.0 {
            return Err(Refused::of("kindling.pa", "kindling_no_air", self.kindling.pa));
        }
        above_nought("kindling.dose_s", self.kindling.dose_s)?;
        self.smoulder.sound("smoulder")?;
        Ok(Sound(self))
    }
}
