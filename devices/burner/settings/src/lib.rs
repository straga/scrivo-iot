//! The burner's settings: every number of its fire, its fan, its draught, its pellets and its sockets, as the page's tab Settings
//! and the console take them, and the rules of this device's set - what an older image kept and what it means now, which mode
//! needs a sensor on a socket. The mechanism that keeps, checks and carries a setting is crates/settings; this crate is what the
//! burner says to it. **A BOARD WITHOUT A BURNER NEVER HAS THIS CRATE IN ITS IMAGE.**
//!
//! A setting stands here because the burner's layer reads it (devices/burner, its API) and the crates that know only the burner
//! (automaton_card, burner_map): the pure crates of the fire - automaton, flame, draught, regulator - read none, they are handed a
//! `Tuning` that [`fire`] makes of these.

pub mod fire;

use settings::{Bearing, Carry, Choice, Kind, Level, Refusal, Rules, Setting, Store, LEFT};

pub use fire::{automaton_tuning, demand_tuning, thermostat_tuning};

/// What the igniter's relay is given on top of the kindling before it opens by itself.
pub const IGNITER_MARGIN_S: u32 = 50;

/// How long the igniter's relay may stay closed before it opens by itself, in seconds, out of the settings as VALUE
/// reads them: the plug's burning with air, one closing of the relay, and [`IGNITER_MARGIN_S`] on top.
pub fn igniter_limit_s(value: &dyn Fn(&Setting) -> String) -> u32 {
    value(&AUTO_IGNITER).parse::<u32>().unwrap_or(0) + IGNITER_MARGIN_S
}



/// How long the burner's auger may run before it stops by itself.
pub const AUGER_LIMIT: Setting = Setting {
    name: "auger_limit",
    label: "Auger stops after",
    default: "60",
    kind: Kind::Seconds { from: 10, to: 600 },
    level: Level::Work,
    card: Some("relays"),
};

/// The hour of the day from which the board restarts itself once a day, when the burner stands with no fire, or `off`: no planned restart. A restart clears whatever a long run leaves behind in the board (the draught's round went silent after 75 hours on 19.10); `off` leaves a board to run until the trouble shows itself and leaves its note.
pub const RESTART_HOUR: Setting = Setting {
    name: "restart_hour",
    label: "Restarts from",
    default: "3",
    kind: Kind::Choice(&RESTART_CHOICES),
    level: Level::Work,
    card: Some("restart"),
};

/// `off`, then every hour of the day: the value is the hour as a number, the words are its time.
const RESTART_CHOICES: [Choice; 25] = [
    Choice { value: "off", words: "off", takes: None, checks: None, means: None },
    Choice { value: "0", words: "00:00", takes: None, checks: None, means: None },
    Choice { value: "1", words: "01:00", takes: None, checks: None, means: None },
    Choice { value: "2", words: "02:00", takes: None, checks: None, means: None },
    Choice { value: "3", words: "03:00", takes: None, checks: None, means: None },
    Choice { value: "4", words: "04:00", takes: None, checks: None, means: None },
    Choice { value: "5", words: "05:00", takes: None, checks: None, means: None },
    Choice { value: "6", words: "06:00", takes: None, checks: None, means: None },
    Choice { value: "7", words: "07:00", takes: None, checks: None, means: None },
    Choice { value: "8", words: "08:00", takes: None, checks: None, means: None },
    Choice { value: "9", words: "09:00", takes: None, checks: None, means: None },
    Choice { value: "10", words: "10:00", takes: None, checks: None, means: None },
    Choice { value: "11", words: "11:00", takes: None, checks: None, means: None },
    Choice { value: "12", words: "12:00", takes: None, checks: None, means: None },
    Choice { value: "13", words: "13:00", takes: None, checks: None, means: None },
    Choice { value: "14", words: "14:00", takes: None, checks: None, means: None },
    Choice { value: "15", words: "15:00", takes: None, checks: None, means: None },
    Choice { value: "16", words: "16:00", takes: None, checks: None, means: None },
    Choice { value: "17", words: "17:00", takes: None, checks: None, means: None },
    Choice { value: "18", words: "18:00", takes: None, checks: None, means: None },
    Choice { value: "19", words: "19:00", takes: None, checks: None, means: None },
    Choice { value: "20", words: "20:00", takes: None, checks: None, means: None },
    Choice { value: "21", words: "21:00", takes: None, checks: None, means: None },
    Choice { value: "22", words: "22:00", takes: None, checks: None, means: None },
    Choice { value: "23", words: "23:00", takes: None, checks: None, means: None },
];

/// What the auger pours in a minute of turning without a stop, in grams, as the owner weighed it: the burner counts the pellets it burns by the seconds of the auger times this. 240 g/min is 4 g/s.
/// Grams a minute and not a second, because a setting is a whole number and a gram in a second is a quarter of the whole: a rate weighed to a tenth of a gram a second is read to a part in two hundred and forty. A dose of a few seconds weighs less
/// than this says: the count is approximate, and the page says so.
pub const PELLET_RATE: Setting = Setting {
    name: "pellet_rate",
    label: "Auger pours, per minute of turning",
    default: "240",
    kind: Kind::Whole { from: 60, to: 600, unit: "g/min" },
    level: Level::Work,
    card: Some("pellets"),
};

/// The edge of the soot scale where the boiler counts as clean: the flue minus the water over the window of a burn (the fifth to the fifteenth minute, on the big rung), in degrees, as it stood after the cleaning of 08.11 (the median of five burns, 36 °C). The page makes a percent of a clean boiler from it.
/// The spans of this edge and of `soot_dirty_c` do not meet, so the clean edge is always below the dirty one and the page never divides by nothing.
pub const SOOT_CLEAN_C: Setting = Setting {
    name: "soot_clean_c",
    label: "Clean boiler: flue minus water",
    default: "36",
    kind: Kind::Whole { from: 10, to: 50, unit: "°C" },
    level: Level::Work,
    card: Some("soot"),
};

/// The edge of the soot scale where the boiler counts as dirty: the third quarter of the burns before the cleaning of 08.11 over the same window (51 °C, which is also the lowest this setting takes), a number from one cycle, to be corrected by the curve of the boiler over the cleanings.
pub const SOOT_DIRTY_C: Setting = Setting {
    name: "soot_dirty_c",
    label: "Dirty boiler: flue minus water",
    default: "51",
    kind: Kind::Whole { from: 51, to: 120, unit: "°C" },
    level: Level::Work,
    card: Some("soot"),
};

/// Where the NT18B07 of the burner answers on its Modbus line. Two modules left on the address they came with
/// stay silent both, so the address is a thing a person changes - and it is written into the module itself, by
/// hand, not seeded from here.
pub const NTC_ADDRESS: Setting = Setting {
    name: "ntc_address",
    // THE RANGE IS NOT IN THE LABEL. It used to be, and then it hung over the field all day saying the same
    // thing to somebody who was not doing anything wrong. Where it belongs is the refusal, which already says
    // it in words: "'300' is not a whole number from 1 to 247".
    label: "Module answers at",
    default: "2",
    kind: Kind::Whole { from: 1, to: 247, unit: "" },
    level: Level::Work,
    card: Some("temperatures"),
};

/// How often the burner walks its Modbus line for temperatures. Water and case warm slowly; asking oftener buys
/// nothing and keeps the line busy.
pub const POLL_EVERY: Setting = Setting {
    name: "poll_every",
    label: "Read every",
    default: "2",
    kind: Kind::Seconds { from: 1, to: 60 },
    level: Level::Work,
    card: Some("temperatures"),
};

/// What a channel of the burner measures: a role out of the list, or "own" with a name written in.
///
/// Both are needed and neither replaces the other. The NAME is for a person - the card says "water", not "ntc 1".
/// The ROLE is for the automaton that comes later: it will look for the water by its role, not by a word a person
/// could have written seven ways. A channel nothing is plugged into stays `free` and leaves the page.
///
/// **THE BOILER AND THE SUPPLY ARE TWO ROLES.** There is no buffer tank on this boiler: the sensor at
/// the top of the boiler reads the hottest water, and that is what the boiler is heated by; the pipe out of it is the
/// supply, a little further along. One role for both would let the automaton hold the wrong point.
const ROLES: [Choice; 8] = [
    Choice { value: "free", words: "nothing plugged in - not shown on the page", takes: None, checks: None, means: None },
    Choice { value: BOILER, words: "water in the boiler - at its top, what the burner holds", takes: None, checks: None, means: None },
    Choice { value: SUPPLY, words: "water leaving the boiler", takes: None, checks: None, means: None },
    Choice { value: RETURN, words: "water coming back", takes: None, checks: None, means: None },
    Choice { value: "flue", words: "flue gas", takes: None, checks: None, means: None },
    Choice { value: "hopper", words: "the pellet hopper", takes: None, checks: None, means: None },
    Choice { value: ROOM, words: "the boiler room", takes: None, checks: None, means: None },
    Choice { value: "own", words: "something else", takes: Some("what it measures, in your own words"), checks: None, means: None },
];

/// The role of one channel. The label says which socket of the module it is, so a person can match it to the wire.
///
/// No `card`, so it stands on the tab Settings and not in the card of temperatures. A card holds what is turned
/// while the boiler runs - where the module answers, how often it is asked; a role is set once, when the sensor is
/// screwed on, and never touched again. Seven of them inside the card would make a person scroll past what they
/// came to read.
macro_rules! role {
    ($konst:ident, $name:literal, $label:literal) => {
        pub const $konst: Setting =
            Setting { name: $name, label: $label, default: "free", kind: Kind::Choice(&ROLES), level: Level::Work, card: None };
    };
}

/// The names of the seven sockets' roles: the settings a mode by a sensor looks at to see that its sensor is plugged in.
pub const SOCKET_ROLES: [&str; 7] = [ROLE_NTC_1.name, ROLE_NTC_2.name, ROLE_NTC_3.name, ROLE_NTC_4.name, ROLE_NTC_5.name, ROLE_NTC_6.name, ROLE_NTC_7.name];

/// The seven sockets' roles, for whoever asks which of them plays a role.
/// Whether a socket is set to ROLE: VALUE reads a setting as the one who asks holds it - the board's kept value, or what a scenario of the stand says. **ONE RULE FOR THE BOARD AND THE STAND**: a role is played by the
/// sockets set to it and by nothing else - not by a line of readings the stand happens to have, not by a sensor that is plugged in - and a role nobody plays reads nothing from the day the board started.
pub fn plays(role: &str, value: &dyn Fn(&Setting) -> String) -> bool {
    ROLES_OF_SOCKETS.iter().any(|socket| value(socket) == role)
}

const ROLES_OF_SOCKETS: [&Setting; 7] = [&ROLE_NTC_1, &ROLE_NTC_2, &ROLE_NTC_3, &ROLE_NTC_4, &ROLE_NTC_5, &ROLE_NTC_6, &ROLE_NTC_7];

role!(ROLE_NTC_1, "role_ntc_1", "Socket 1 measures");
role!(ROLE_NTC_2, "role_ntc_2", "Socket 2 measures");
role!(ROLE_NTC_3, "role_ntc_3", "Socket 3 measures");
role!(ROLE_NTC_4, "role_ntc_4", "Socket 4 measures");
role!(ROLE_NTC_5, "role_ntc_5", "Socket 5 measures");
role!(ROLE_NTC_6, "role_ntc_6", "Socket 6 measures");
role!(ROLE_NTC_7, "role_ntc_7", "Socket 7 measures");

/// The card of the mains the burner draws: the socket it is plugged into, read over the network - where it is and how often it is asked. **A SETTING'S KEY IN NVS IS ITS CARD'S.**
const MAINS: Option<&str> = Some("mains");

/// The card the burner's automaton and its numbers stand on.
const AUTOMATON: Option<&str> = Some("automaton");

/// The card the burner's thermostat and its numbers stand on: the mode, the setpoint, the hysteresis and the level of power. **A SETTING'S KEY IN NVS IS ITS CARD'S**, so these four moved from `s.automaton` to `s.thermostat`.
const THERMOSTAT: Option<&str> = Some("thermostat");

/// The card the burner's demand and its numbers stand on: who asks the boiler for heat, the targets of the room, and the profiles with the windows that move them. **A SETTING'S KEY IN NVS IS ITS CARD'S**, so the profile and the windows moved from `s.thermostat` to `s.demand`, and a value kept under the old key is not read.
const DEMAND: Option<&str> = Some("demand");

// The numbers of the one step the automaton lights and burns on, the factory's kindling step of this burner:
// purge, then the igniter and the dose timed onto it, then the wait for a flame, so many attempts; 51 Pa, 32 s of
// dose, 15 s of pause. Every other
// number of the automaton is fixed in the image for now and the page says which (devices/burner/src/automaton.rs).
//
// **THE SPANS ARE WHAT A FIELD ON A CARD TAKES, NOT A JUDGEMENT ABOUT THE FIRE.** The automaton's own door
// (`automaton::Tuning::sound`) is what refuses a number that makes no burner; these only keep a typed value
// within reach of one.

/// How long the fan blows the pot clear before the igniter. 30 s is the factory cycle's; its table of parameters
/// says 150 s before, and which is ours is the owner's question.
pub const AUTO_PURGE: Setting = Setting {
    name: "auto_purge",
    label: "Purge before lighting",
    default: "30",
    kind: Kind::Seconds { from: 5, to: 600 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The most the igniter burns WITH AIR in one attempt - and it burns only with air: it burns until
/// the flame and goes off with it, or goes off here - as Plum's does, "after this time the heater
/// works still until detecting the flame" (850P2, 14.1), and as the prototype's did, up to 300 s. Four minutes to start
/// with. The relay's own limit follows from this ([`igniter_limit_s`]): one number a person sets, and no second one to
/// set apart from it.
pub const AUTO_IGNITER: Setting = Setting {
    name: "auto_igniter",
    label: "Igniter with air until a flame, at most",
    default: "240",
    kind: Kind::Seconds { from: 5, to: 300 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long the fan blows alone after the igniter has gone off with no flame, before the next attempt: a fire the plug
/// started late still shows here (Plum 860P, `Fan time extension`, 14).
pub const AUTO_WAIT: Setting = Setting {
    name: "auto_wait",
    label: "Then the fan alone, waiting for a flame",
    default: "90",
    kind: Kind::Seconds { from: 10, to: 600 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How many attempts at lighting before the burner gives up. **THREE, NOT THE FACTORY'S 1+7**: every
/// attempt heats the plug again, and eight in a row wear it out; between two attempts it rests for the wait for a
/// flame (AUTO_WAIT).
pub const AUTO_TRIES: Setting = Setting {
    name: "auto_tries",
    label: "Attempts at lighting",
    default: "3",
    kind: Kind::Whole { from: 1, to: 8, unit: "" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The draught an attempt at lighting holds once the plug is hot and the dose is on it.
///
/// **ITS OWN NUMBER AND NOT THE STEP'S**: a fire being lit wants a little less air than a fire being
/// carried, and the two are set apart so the lighting can be turned down without touching the burning. The factory
/// sets both to 51 Pa (the factory's parameter list: "Fan Lighting 51", "Power1 51"), the prototype lit
/// on 42 and burned on 52, and Plum keeps a kindling airflow of its own (`kindling_airflow_power`).
pub const AUTO_KINDLE_PA: Setting = Setting {
    name: "auto_kindle_pa",
    label: "Draught of lighting",
    default: "51",
    kind: Kind::Whole { from: 0, to: draught::FULL_PA, unit: "Pa" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The water at the top of the boiler the pump runs from; it stops that much below it that the automaton's
/// hysteresis says.
///
/// **THE BOILER IS KEPT FROM CONDENSING** (Plum, `CH pump activation temperature`, ecoMAX860P 14.3): cold water
/// from the system pulled through a boiler that has not warmed up yet runs down its walls as condensate. 45 °C is
/// no concern of a cast-iron boiler, which is what the burner heats: 35 °C, the water moving early.
pub const AUTO_PUMP_ON_C: Setting = Setting {
    name: "auto_pump_on_c",
    label: "Pump runs from",
    default: "35",
    kind: Kind::Whole { from: 20, to: 85, unit: "°C" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How far under `auto_pump_on_c` the pump stands off. Its own number and not the setpoint's hysteresis: the pump follows
/// the water of the boiler in every state, and the burner's setpoint moves with what it holds.
/// 4 °C, the number the pump stood off at before it had one.
pub const AUTO_PUMP_OFF_C: Setting = Setting {
    name: "auto_pump_off_c",
    label: "Pump stops lower by",
    default: "4",
    kind: Kind::Whole { from: 1, to: 20, unit: "°C" },
    level: Level::Tuning,
    card: AUTOMATON,
};

// ---- what the burner holds ----
//
// The mode and its numbers are what a person turns from day to day, so they are the card's `Work` settings; everything
// else on the card is `Tuning`. The defaults are what the burner did before the mode existed: the boiler, held at 60 °C,
// lit again 4 °C under it.

/// What the burner holds: the temperature of a sensor, or one level of power without one. A mode by a sensor is taken
/// only while a socket plays that sensor's role ([`change_all`]).
pub const AUTO_HOLD: Setting = Setting {
    name: "auto_hold",
    label: "The burner holds",
    default: "boiler",
    kind: Kind::Choice(&HOLDS),
    level: Level::Work,
    card: THERMOSTAT,
};

const HOLDS: [Choice; 3] = [
    Choice { value: "power", words: "power", takes: None, checks: None, means: Some("burns on the level chosen below and holds no temperature") },
    Choice { value: BOILER, words: "the boiler", takes: None, checks: None, means: Some("holds the water at the top of the boiler") },
    Choice { value: RETURN, words: "the return", takes: None, checks: None, means: Some("holds the water coming back") },
];

/// The numbers a mode by a sensor holds by: its temperature and how far under it the burner works again. The level the fire is carried on is the burner's in
/// every mode (`auto_step`), so no mode names it.
const BY_A_SENSOR: [&str; 2] = [AUTO_HOLD_C.name, AUTO_DROP_C.name];

/// The same, and the levels of the way up to the setpoint with the distances they begin at and the wait before a light: the thermostat of the new burner steps down by them, the first burner has none.
const BY_A_SENSOR_WITH_LEVELS: [&str; 8] = [
    AUTO_HOLD_C.name,
    AUTO_DROP_C.name,
    AUTO_MID_C.name,
    AUTO_SMALL_C.name,
    AUTO_STEP_MID.name,
    AUTO_STEP_SMALL.name,
    AUTO_LOCK_MIN.name,
    AUTO_LOCK_C.name,
];

/// What the demand brings with it when the room plays it: the targets of the room and its hysteresis, and the profiles with the windows that name the target.
const BY_THE_ROOM: [&str; 10] = [
    AUTO_ROOM_C.name,
    AUTO_ROOM_ECO_C.name,
    AUTO_ROOM_GAP_C.name,
    AUTO_PROFILE.name,
    WINDOW_1.name,
    WINDOW_2.name,
    WINDOW_3.name,
    WINDOW_4.name,
    WINDOW_5.name,
    WINDOW_6.name,
];

/// What the demand brings with it when the schedule plays it: the profiles and the windows, and no target.
const BY_THE_SCHEDULE: [&str; 7] = [AUTO_PROFILE.name, WINDOW_1.name, WINDOW_2.name, WINDOW_3.name, WINDOW_4.name, WINDOW_5.name, WINDOW_6.name];

const fn bearings(shows: &'static [&'static str]) -> [Bearing; 2] {
    [Bearing { setting: AUTO_HOLD.name, choice: BOILER, shows, role: Some(BOILER) }, Bearing { setting: AUTO_HOLD.name, choice: RETURN, shows, role: Some(RETURN) }]
}

/// What each mode of the burner brings with it: the modes by a sensor the temperature, and each of them the role its sensor plays on a socket. The mode
/// that holds none brings nothing - the level stands in every mode. **The first burner's**: it works a hold by a temperature on the two numbers and nothing steps down.
pub static BEARINGS: [Bearing; 2] = bearings(&BY_A_SENSOR);

/// What each mode brings with it on the new burner: the same modes and roles, and the levels of the way up besides the two numbers - and what each player of the demand brings: the room the role of the room and its targets, the schedule its profiles. **A SOURCE IS TAKEN
/// ONLY WHILE ITS SENSOR IS PLAYED**, as a mode by a sensor is.
pub static BEARINGS_WITH_LEVELS: [Bearing; 4] = {
    let by_sensor = bearings(&BY_A_SENSOR_WITH_LEVELS);
    [
        by_sensor[0],
        by_sensor[1],
        Bearing { setting: AUTO_ASKED_BY.name, choice: ROOM, shows: &BY_THE_ROOM, role: Some(ROOM) },
        Bearing { setting: AUTO_ASKED_BY.name, choice: SCHEDULE, shows: &BY_THE_SCHEDULE, role: None },
    ]
};

/// What CHOICE of SETTING bears on for the first burner, or None when it bears on nothing.
pub fn bearing(setting: &str, choice: &str) -> Option<&'static Bearing> {
    BEARINGS.iter().find(|b| b.setting == setting && b.choice == choice)
}

/// What CHOICE of SETTING bears on for the new burner, or None when it bears on nothing.
pub fn bearing_with_levels(setting: &str, choice: &str) -> Option<&'static Bearing> {
    BEARINGS_WITH_LEVELS.iter().find(|b| b.setting == setting && b.choice == choice)
}

/// The temperature the burner holds: at it the fire goes down to smouldering. One span for every sensor.
pub const AUTO_HOLD_C: Setting = Setting {
    name: "auto_hold_c",
    label: "Hold until",
    default: "60",
    kind: Kind::Whole { from: 5, to: 85, unit: "°C" },
    level: Level::Work,
    card: THERMOSTAT,
};

/// How far under the held temperature the burner works again.
pub const AUTO_DROP_C: Setting = Setting {
    name: "auto_drop_c",
    label: "Work again when lower by",
    default: "4",
    kind: Kind::Whole { from: 1, to: 20, unit: "°C" },
    level: Level::Work,
    card: THERMOSTAT,
};

/// The level the burner burns on, in kilowatts of fuel, in every mode: what stands behind it - the dose, the pause and the draught of each level - is
/// the table of steps below.
pub const AUTO_STEP: Setting = Setting {
    name: "auto_step",
    label: "Level of power",
    default: "6",
    kind: Kind::Choice(&STEPS),
    level: Level::Work,
    card: THERMOSTAT,
};

const STEPS: [Choice; 7] = [
    Choice { value: "14", words: "14 kW", takes: None, checks: None, means: None },
    Choice { value: "10", words: "10 kW", takes: None, checks: None, means: None },
    Choice { value: "8", words: "8 kW", takes: None, checks: None, means: None },
    Choice { value: "6", words: "6 kW", takes: None, checks: None, means: None },
    Choice { value: "5", words: "5 kW", takes: None, checks: None, means: None },
    Choice { value: "4", words: "4 kW", takes: None, checks: None, means: None },
    Choice { value: "3", words: "3 kW", takes: None, checks: None, means: None },
];

/// The middle level of the way up to the setpoint and the small one nearest it (Plum's MED and MIN): a row of the table of steps, or `chosen`
/// - the level of power above, and then there is no level of its own and nothing steps down.
pub const CHOSEN: &str = "chosen";

const STEPS_OR_CHOSEN: [Choice; 8] = [
    Choice { value: CHOSEN, words: "the level of power", takes: None, checks: None, means: Some("burns on the level of power chosen above: no step of its own") },
    Choice { value: "14", words: "14 kW", takes: None, checks: None, means: None },
    Choice { value: "10", words: "10 kW", takes: None, checks: None, means: None },
    Choice { value: "8", words: "8 kW", takes: None, checks: None, means: None },
    Choice { value: "6", words: "6 kW", takes: None, checks: None, means: None },
    Choice { value: "5", words: "5 kW", takes: None, checks: None, means: None },
    Choice { value: "4", words: "4 kW", takes: None, checks: None, means: None },
    Choice { value: "3", words: "3 kW", takes: None, checks: None, means: None },
];

/// The level the burner burns on while the water is under the setpoint by H2 or less, and by more than H1.
pub const AUTO_STEP_MID: Setting = Setting {
    name: "auto_step_mid",
    label: "Level of power, middle",
    default: CHOSEN,
    kind: Kind::Choice(&STEPS_OR_CHOSEN),
    level: Level::Work,
    card: THERMOSTAT,
};

/// The level the burner burns on while the water is under the setpoint by less than H1.
pub const AUTO_STEP_SMALL: Setting = Setting {
    name: "auto_step_small",
    label: "Level of power, small",
    default: CHOSEN,
    kind: Kind::Choice(&STEPS_OR_CHOSEN),
    level: Level::Work,
    card: THERMOSTAT,
};

/// How far under the setpoint the middle level begins (H2): farther than this the burner burns on the level of power, nearer on the middle one.
pub const AUTO_MID_C: Setting = Setting {
    name: "auto_mid_c",
    label: "Middle level when nearer than",
    default: "8",
    kind: Kind::Whole { from: 2, to: 40, unit: "°C" },
    level: Level::Work,
    card: THERMOSTAT,
};

/// How far under the setpoint the small level begins (H1), under H2: nearer than this the burner burns on the small level.
pub const AUTO_SMALL_C: Setting = Setting {
    name: "auto_small_c",
    label: "Small level when nearer than",
    default: "3",
    kind: Kind::Whole { from: 1, to: 39, unit: "°C" },
    level: Level::Work,
    card: THERMOSTAT,
};

// ---- the lockout of a light after the fire went out ----

/// How long after the fire goes out the thermostat does not light it again by itself, in minutes; nought - no lockout. A person's Start and water under [`AUTO_LOCK_C`] light it at once.
pub const AUTO_LOCK_MIN: Setting = Setting {
    name: "auto_lock_min",
    label: "Do not light again sooner than",
    default: "0",
    kind: Kind::Whole { from: 0, to: 720, unit: "min" },
    level: Level::Work,
    card: THERMOSTAT,
};

/// How cold the water of the boiler may get while the lockout runs, in every mode: under this the fire is lit at once, because waiting longer would let the boiler go cold.
pub const AUTO_LOCK_C: Setting = Setting {
    name: "auto_lock_c",
    label: "But at once when colder than",
    default: "30",
    kind: Kind::Whole { from: 5, to: 60, unit: "°C" },
    level: Level::Work,
    card: THERMOSTAT,
};

// ---- who asks the boiler for heat, and the target by the clock ----
//
// The thermostat of the page is the floor above the boiler's water: the house says cold or warm, and the boiler works by its own water when it is cold and goes the ordinary way to a stop when it is warm. Who says it is one choice - nobody, the room
// against its target, or the schedule alone. The profile that stands names the target of the room (comfort, eco; off holds none), the main profile says which stands outside the windows, and a window is one setting holding one line of
// text: `mon-fri 22:00-06:00 eco`. One line a window and not a field for each part of it, because the card's keys are read through one buffer (`MAX_READ`) and a window of seven fields is a seventh of that buffer by itself; the grammar is the
// schedule's (`schedule::Window::parse`), which is the one place that knows what the text means. The numbers of the room are whole degrees, as every number here is.

/// The word `schedule` of the choice of who asks.
pub const SCHEDULE: &str = "schedule";

/// Who asks the boiler for heat: nobody - it works from button to button by its water alone -, the room against its target, or the schedule alone. The room is taken only while a socket plays its role ([`change_all`]).
pub const AUTO_ASKED_BY: Setting = Setting {
    name: "auto_asked_by",
    label: "The thermostat asks by",
    default: "none",
    kind: Kind::Choice(&ASKED_BY),
    level: Level::Work,
    card: DEMAND,
};

const ASKED_BY: [Choice; 3] = [
    Choice { value: "none", words: "nothing", takes: None, checks: None, means: Some("the boiler works from Start to Stop by its water alone") },
    Choice { value: ROOM, words: "the room", takes: None, checks: None, means: Some("the boiler works while the room is under its target, and goes to a stop when the room reaches it") },
    Choice { value: SCHEDULE, words: "the schedule", takes: None, checks: None, means: Some("the boiler works in the windows that burn and goes to a stop outside them") },
];

/// The target of the room under the comfort profile.
pub const AUTO_ROOM_C: Setting = Setting {
    name: "auto_room_c",
    label: "Room target, comfort",
    default: "21",
    kind: Kind::Tenths { from: 60, to: 350, unit: "°C" },
    level: Level::Work,
    card: DEMAND,
};

/// The target of the room under the eco profile: the hours the house can do with less.
pub const AUTO_ROOM_ECO_C: Setting = Setting {
    name: "auto_room_eco_c",
    label: "Room target, eco",
    default: "17",
    kind: Kind::Tenths { from: 60, to: 350, unit: "°C" },
    level: Level::Work,
    card: DEMAND,
};

/// How far under its target the room may fall before it says cold: its own number and not the water's. The span keeps it under the lowest target, so the line to say cold is never at nought or under.
pub const AUTO_ROOM_GAP_C: Setting = Setting {
    name: "auto_room_gap_c",
    label: "Room cold when lower by",
    default: "1",
    kind: Kind::Whole { from: 1, to: 5, unit: "°C" },
    level: Level::Work,
    card: DEMAND,
};

/// The profile that stands outside every window.
pub const AUTO_PROFILE: Setting = Setting {
    name: "auto_profile",
    label: "Outside the windows",
    default: "comfort",
    kind: Kind::Choice(&PROFILE_CHOICES),
    level: Level::Work,
    card: DEMAND,
};

const PROFILE_CHOICES: [Choice; 3] = [
    Choice { value: "comfort", words: "comfort", takes: None, checks: None, means: Some("the room holds the comfort target; as the schedule it asks for heat") },
    Choice { value: "eco", words: "eco", takes: None, checks: None, means: Some("the room holds the eco target; as the schedule it asks for heat") },
    Choice { value: "off", words: "off", takes: None, checks: None, means: Some("the house is not asked for heat") },
];

/// The longest a window may be written: a line is a few dozen characters, and a card's keys are read through one buffer.
const WINDOW_TEXT_MAX: usize = 64;

/// Whether TEXT is a window as the schedule reads it, and short enough to be kept.
fn window_text(text: &str) -> bool {
    text.len() <= WINDOW_TEXT_MAX && schedule::Window::parse(text).is_ok()
}

const WINDOW_CHOICES: [Choice; 2] = [
    Choice { value: "off", words: "not used", takes: None, checks: None, means: Some("no window in this place") },
    Choice {
        value: "set",
        words: "a window",
        takes: Some("days, times and a profile: mon-fri 22:00-06:00 eco"),
        checks: Some(window_text),
        means: Some("the window written below"),
    },
];

macro_rules! window_setting {
    ($konst:ident, $name:literal, $label:literal) => {
        /// One window of the schedule, or `off` where there is none.
        pub const $konst: Setting =
            Setting { name: $name, label: $label, default: "off", kind: Kind::Choice(&WINDOW_CHOICES), level: Level::Work, card: DEMAND };
    };
}

window_setting!(WINDOW_1, "window_1", "Window 1");
window_setting!(WINDOW_2, "window_2", "Window 2");
window_setting!(WINDOW_3, "window_3", "Window 3");
window_setting!(WINDOW_4, "window_4", "Window 4");
window_setting!(WINDOW_5, "window_5", "Window 5");
window_setting!(WINDOW_6, "window_6", "Window 6");

/// The windows of the schedule in the order the card lists them: the first that is open stands.
pub const WINDOW_SETTINGS: [&Setting; schedule::WINDOWS] = [&WINDOW_1, &WINDOW_2, &WINDOW_3, &WINDOW_4, &WINDOW_5, &WINDOW_6];

// ---- the power steps: a table a person edits ----
//
// On each level of power a dose of the auger, a pause and a draught, and the burner burns on the level that `auto_step` names in every hold. The numbers are the owner's table:
// a dose of 2 s - 8 g at 4 g/s -, the pause from the period 136 / kW, the draught in proportion to the power from the factory's point of 14 kW at 51 Pa; the 3 kW row is what the burner held on 27.09
// (1 s of dose, 21 s of pause, 11 Pa - about 2 kW of fuel in fact). A number is the spans of a field a person types in: the automaton's own door (`automaton::Tuning::sound`) is what
// refuses a burner that cannot burn.

macro_rules! step_number {
    ($konst:ident, $name:literal, $kw:literal, $what:literal, $default:literal, $kind:expr) => {
        #[doc = concat!("The ", $what, " of the ", $kw, " kW step.")]
        pub const $konst: Setting =
            Setting { name: $name, label: concat!($kw, " kW: ", $what), default: $default, kind: $kind, level: Level::Tuning, card: AUTOMATON };
    };
}

/// How long the auger turns in each feed on a step.
const DOSE: Kind = Kind::Tenths { from: 1, to: 1200, unit: "s" };
/// How long the auger rests between feeds on a step.
const PAUSE: Kind = Kind::Tenths { from: 1, to: 6000, unit: "s" };
/// The draught a step holds, in what the sensor reads.
const DRAUGHT: Kind = Kind::Whole { from: 0, to: draught::FULL_PA, unit: "Pa" };

step_number!(AUTO_14_ON_S, "auto_14_on_s", "14", "the auger turns", "2", DOSE);
step_number!(AUTO_14_OFF_S, "auto_14_off_s", "14", "the auger rests", "8", PAUSE);
step_number!(AUTO_14_PA, "auto_14_pa", "14", "draught", "51", DRAUGHT);
step_number!(AUTO_10_ON_S, "auto_10_on_s", "10", "the auger turns", "2", DOSE);
step_number!(AUTO_10_OFF_S, "auto_10_off_s", "10", "the auger rests", "12", PAUSE);
step_number!(AUTO_10_PA, "auto_10_pa", "10", "draught", "36", DRAUGHT);
step_number!(AUTO_8_ON_S, "auto_8_on_s", "8", "the auger turns", "2", DOSE);
step_number!(AUTO_8_OFF_S, "auto_8_off_s", "8", "the auger rests", "15", PAUSE);
step_number!(AUTO_8_PA, "auto_8_pa", "8", "draught", "29", DRAUGHT);
step_number!(AUTO_6_ON_S, "auto_6_on_s", "6", "the auger turns", "2", DOSE);
step_number!(AUTO_6_OFF_S, "auto_6_off_s", "6", "the auger rests", "21", PAUSE);
step_number!(AUTO_6_PA, "auto_6_pa", "6", "draught", "22", DRAUGHT);
step_number!(AUTO_5_ON_S, "auto_5_on_s", "5", "the auger turns", "2", DOSE);
step_number!(AUTO_5_OFF_S, "auto_5_off_s", "5", "the auger rests", "25", PAUSE);
step_number!(AUTO_5_PA, "auto_5_pa", "5", "draught", "18", DRAUGHT);
step_number!(AUTO_4_ON_S, "auto_4_on_s", "4", "the auger turns", "2", DOSE);
step_number!(AUTO_4_OFF_S, "auto_4_off_s", "4", "the auger rests", "32", PAUSE);
step_number!(AUTO_4_PA, "auto_4_pa", "4", "draught", "15", DRAUGHT);
step_number!(AUTO_3_ON_S, "auto_3_on_s", "3", "the auger turns", "1", DOSE);
step_number!(AUTO_3_OFF_S, "auto_3_off_s", "3", "the auger rests", "21", PAUSE);
step_number!(AUTO_3_PA, "auto_3_pa", "3", "draught", "11", DRAUGHT);

/// The three numbers of one level of power.
pub struct StepNumbers {
    /// The level in kilowatts, as `auto_step` keeps it.
    pub kw: &'static str,
    pub on_s: &'static Setting,
    pub off_s: &'static Setting,
    pub pa: &'static Setting,
}

/// The table: a row a level, from the strongest down - the order a person reads it and `STEPS` offers the levels in.
pub const STEP_TABLE: [StepNumbers; 7] = [
    StepNumbers { kw: "14", on_s: &AUTO_14_ON_S, off_s: &AUTO_14_OFF_S, pa: &AUTO_14_PA },
    StepNumbers { kw: "10", on_s: &AUTO_10_ON_S, off_s: &AUTO_10_OFF_S, pa: &AUTO_10_PA },
    StepNumbers { kw: "8", on_s: &AUTO_8_ON_S, off_s: &AUTO_8_OFF_S, pa: &AUTO_8_PA },
    StepNumbers { kw: "6", on_s: &AUTO_6_ON_S, off_s: &AUTO_6_OFF_S, pa: &AUTO_6_PA },
    StepNumbers { kw: "5", on_s: &AUTO_5_ON_S, off_s: &AUTO_5_OFF_S, pa: &AUTO_5_PA },
    StepNumbers { kw: "4", on_s: &AUTO_4_ON_S, off_s: &AUTO_4_OFF_S, pa: &AUTO_4_PA },
    StepNumbers { kw: "3", on_s: &AUTO_3_ON_S, off_s: &AUTO_3_OFF_S, pa: &AUTO_3_PA },
];


/// How long the auger turns in the lighting's one dose. No longer than the relay's own limit may let it (AUGER_LIMIT).
///
/// **A LEVEL AND NOT A PORTION**: the dose fills the pot above the plug, or the plug
/// heats air, and the fire eats it through rising with the auger standing.
pub const AUTO_DOSE: Setting = Setting {
    name: "auto_dose",
    label: "Lighting dose: pellets up above the igniter",
    default: "32",
    kind: Kind::Tenths { from: 1, to: 1200, unit: "s" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The dose of the second and later attempts at lighting a cold pot, in per cent of the lighting's dose: a pot that did not
/// light already holds fuel, and the plug wants it covered and no more. Plum cuts it to a tenth (manual 8.4). A lost
/// flame is lit again by doses of its own ([`AUTO_RELIGHT_1`]).
pub const AUTO_DOSE_AGAIN: Setting = Setting {
    name: "auto_dose_again",
    label: "Next attempts at lighting a cold pot: dose, of the lighting dose",
    default: "10",
    kind: Kind::Whole { from: 1, to: 100, unit: "%" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The doses of the three attempts at lighting a lost flame again, in seconds of auger: the first pours
/// the most, the next two top up. A hot pot is lit from its embers with no plug; a cold one with the plug and air.
pub const AUTO_RELIGHT_1: Setting = Setting {
    name: "auto_relight_1",
    label: "Lighting a lost flame again: 1st dose",
    default: "20",
    kind: Kind::Tenths { from: 0, to: 600, unit: "s" },
    level: Level::Tuning,
    card: AUTOMATON,
};

pub const AUTO_RELIGHT_2: Setting = Setting {
    name: "auto_relight_2",
    label: "Lighting a lost flame again: 2nd dose",
    default: "10",
    kind: Kind::Tenths { from: 0, to: 600, unit: "s" },
    level: Level::Tuning,
    card: AUTOMATON,
};

pub const AUTO_RELIGHT_3: Setting = Setting {
    name: "auto_relight_3",
    label: "Lighting a lost flame again: 3rd dose",
    default: "10",
    kind: Kind::Tenths { from: 0, to: 600, unit: "s" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long the flame has to hold without a break after lighting before the work begins, the auger standing and the
/// draught climbing from the lighting's to the step's. A flame that blinks starts it again. 60 s to
/// start with; the owner sets the real one on a live lighting.
pub const AUTO_RISE_S: Setting = Setting {
    name: "auto_rise_s",
    label: "After lighting, the flame holds with the auger standing",
    default: "60",
    kind: Kind::Seconds { from: 10, to: 600 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long the flame may be gone in rising or at work before it is taken for lost and lit again: the feed and the air go
/// on as they were, and a flame back within it leaves no trace. 20 s. **NOT ABOVE A MINUTE**: dark longer
/// is no blink, and every second of it is a feed poured into a pot that may be out - a board that kept 120 s from an
/// older image has it put aside at the start (`seed`) and reads the default.
pub const AUTO_NO_FLAME_S: Setting = Setting {
    name: "auto_no_flame_s",
    label: "No flame for this long, and the fire is lit again",
    default: "20",
    kind: Kind::Seconds { from: 5, to: 60 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long the flue is watched at work, in seconds, for the fuel not arriving: a flue that stays under the cold mark for all of it, or
/// that falls from the highest of it by more than the fall allows, means the fuel has stopped. Minutes, because a fire has inertia - a window of 60 s
/// gave up on a burning fire a minute into the work (journal, 24.09 08:27). 10 min to start with (27.09: a cold start crosses 70 °C in 291 s).
pub const AUTO_FLUE_FALL_S: Setting = Setting {
    name: "auto_flue_win_s",
    label: "Flue window: cold or falling means no fuel",
    default: "600",
    kind: Kind::Seconds { from: 60, to: 1800 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The longest the fire may burn out after a stop or a fault: burning out ends when the flame has
/// been gone a minute, and here the pot is blown clear and stopped whatever the flame says - the stop keeps the
/// fan on the step's draught while a flame shows. Ten minutes - the live lighting of 24.09 burnt out to 120 s and stopped
/// the fan over a pot with most of its dose alight.
pub const AUTO_BURN_OFF_S: Setting = Setting {
    name: "auto_burn_off_s",
    label: "Burning out lasts at most, then a stop",
    default: "600",
    kind: Kind::Seconds { from: 60, to: 3600 },
    level: Level::Tuning,
    card: AUTOMATON,
};

// THE NUMBERS THE BURNER BURNS BY THAT A PERSON CAN TURN: every number of the
// fire's behaviour is a setting of the card, seeded with what the burner burned by, for the owner to fit to his boiler.
// The spans keep a typed value within reach of a burner; the automaton's door still refuses a pair that makes none (a
// burning out shorter than its blowing, an alarm under the overheating). **THE PROTECTIONS HAVE HARD SPANS IN THE CODE**
// - overheating 70 to 90 °C, the alarm 71 to 99 °C - because a page must not be able to turn them off: a thermostat of
// 85 °C on the boiler's own iron is the hardware's, and these two stand before it.

/// How long the fan blows at least after a stop, in seconds: the factory blows 20 s after every stop.
pub const AUTO_PURGE_END: Setting = Setting {
    name: "auto_purge_end",
    label: "Fan blows after a stop at least",
    default: "20",
    kind: Kind::Seconds { from: 5, to: 300 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The least a burning out lasts without a flame: it ends when the flame has been gone this long.
pub const AUTO_OUT_MIN_S: Setting = Setting {
    name: "auto_out_min_s",
    label: "Burning out ends when there is no flame for",
    default: "60",
    kind: Kind::Seconds { from: 10, to: 600 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long smouldering lasts before the burner puts itself out. An iron boiler is slow to cool, and the owner holds it for fifteen minutes; the first burner held it for five, and its comparison asks for 300 explicitly.
pub const AUTO_SMOULDER_S: Setting = Setting {
    name: "auto_smoulder_s",
    label: "Smouldering lasts",
    default: "900",
    kind: Kind::Seconds { from: 0, to: 7200 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The draught smouldering holds: none, the fan stands.
pub const AUTO_SMOULD_PA: Setting = Setting {
    name: "auto_smould_pa",
    label: "Smouldering: draught",
    default: "0",
    kind: Kind::Whole { from: 0, to: 60, unit: "Pa" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long the auger turns in smouldering.
pub const AUTO_SMOULD_ON: Setting = Setting {
    name: "auto_smould_on",
    label: "Smouldering: the auger turns",
    default: "1",
    kind: Kind::Seconds { from: 1, to: 60 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long the auger rests in smouldering.
pub const AUTO_SMOULD_OFF: Setting = Setting {
    name: "auto_smould_off",
    label: "Smouldering: the auger rests",
    default: "50",
    kind: Kind::Seconds { from: 1, to: 600 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The flue at or above which a board that came up over a dark pot takes the pot for burning: a restart is asked this and nothing else.
pub const AUTO_FLUE_C: Setting = Setting {
    name: "auto_flue_c",
    label: "Restart over a dark pot: a flue from this is a fire",
    default: "100",
    kind: Kind::Whole { from: 50, to: 200, unit: "°C" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The flue under which a working burner is cold: under it for all of `auto_flue_win_s`, the fuel counts as not arriving. A low fire holds its
/// flue at 93-105 °C and stands there (27.09), and a cold start crosses this mark in the first minutes of work, so the mark is a flue that stayed cold.
pub const AUTO_FLUE_COLD_C: Setting = Setting {
    name: "auto_flue_cold",
    label: "Cold flue at work: under",
    default: "70",
    kind: Kind::Whole { from: 30, to: 150, unit: "°C" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How far the flue may fall from the highest of the window before the fuel counts as not arriving: 10 °C in 10 min is what 26.09 17:32 looked like
/// (77 °C, a degree a minute).
pub const AUTO_FLUE_FALL_C: Setting = Setting {
    name: "auto_flue_fall",
    label: "Flue falls from its highest by",
    default: "10",
    kind: Kind::Whole { from: 2, to: 50, unit: "°C" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The water past which the pump runs whatever its threshold. A PROTECTION: its span is the code's.
pub const AUTO_TOO_HOT_C: Setting = Setting {
    name: "auto_too_hot_c",
    label: "Overheating: the pump runs from",
    default: "88",
    kind: Kind::Whole { from: 70, to: 90, unit: "°C" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The water past which the alarm sounds and the burner puts itself out. A PROTECTION: its span is the code's, and the automaton's door keeps it over the overheating.
pub const AUTO_ALARM_C: Setting = Setting {
    name: "auto_alarm_c",
    label: "Alarm from",
    default: "95",
    kind: Kind::Whole { from: 71, to: 99, unit: "°C" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long a reading must hold before it is believed.
pub const AUTO_SURE_S: Setting = Setting {
    name: "auto_sure_s",
    label: "A reading is believed after it holds for",
    default: "10",
    kind: Kind::Seconds { from: 1, to: 120 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long a flame must hold in the lighting before the fire counts as caught: a reading over the threshold for one second is not a fire - the
/// first light from the embers on 27.09 was counted at 2791 mV against 2800 and the sensor was dark at the next second. 5 s to start with.
pub const AUTO_LIT_S: Setting = Setting {
    name: "auto_lit_s",
    label: "A flame is a fire after it holds for",
    default: "5",
    kind: Kind::Seconds { from: 1, to: 60 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// A lost flame over a flue this hot is lit again from the embers.
pub const AUTO_EMBER_C: Setting = Setting {
    name: "auto_ember_c",
    label: "A lost flame over the flue this hot is lit from the embers",
    default: "45",
    kind: Kind::Whole { from: 20, to: 90, unit: "°C" },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long the fan is given to spin up.
pub const AUTO_SPIN_UP_S: Setting = Setting {
    name: "auto_spin_up_s",
    label: "The fan is given to spin up",
    default: "10",
    kind: Kind::Seconds { from: 1, to: 60 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// How long the draught may stay under the sensor's own zero while the fan is asked for air.
pub const AUTO_LOW_PRESSURE_S: Setting = Setting {
    name: "auto_low_pa_s",
    label: "The draught may stay under zero for",
    default: "20",
    kind: Kind::Seconds { from: 1, to: 120 },
    level: Level::Tuning,
    card: AUTOMATON,
};

/// The value a channel takes when nothing is plugged into it.
pub const FREE: &str = "free";

/// The water at the top of the boiler: the hottest there is, and the one the burner and the thermostat hold.
pub const BOILER: &str = "boiler";
/// The water leaving the boiler, a little further along the pipe.
pub const SUPPLY: &str = "supply";
/// The water coming back to the boiler.
pub const RETURN: &str = "return";
/// The air of the boiler room.
pub const ROOM: &str = "room";

/// What a role says to a person: the words of the choice, or the name written in for "own".
pub fn role_words(stored: &str) -> String {
    match stored.split_once(':') {
        Some((_, own)) if !own.is_empty() => own.to_string(),
        _ => ROLES
            .iter()
            .find(|c| c.value == stored)
            .map_or_else(|| stored.to_string(), |c| c.words.split(" - ").next().unwrap_or(c.words).to_string()),
    }
}

/// The two resistors of the burner's pressure divider, as a voltmeter reads them.
///
/// TWO RESISTANCES AND NOT ONE COEFFICIENT, because a voltmeter measures resistances and that is what a person
/// can write down without thinking. A single coefficient would have them divide in their head, and a slip in
/// that division looks exactly like a board working properly on a wrong pressure - the dearest kind of fault,
/// the kind nobody notices. The coefficient is worked out of these two and kept nowhere: two sources of one
/// truth drift apart.
///
/// The words say which arm is which, so nobody has to open a schematic: the upper one comes FROM THE SENSOR,
/// the lower one goes TO GROUND. The range is not in the label - the refusal says it when it matters. Under a
/// hundred ohms the pair loads the sensor's output; over a megaohm the ADC's own input starts to be part of the
/// divider and the arithmetic here stops describing the board.
///
/// Seeded with what was calculated and soldered on 12.09 - two of 10 kΩ. Seeded, not written in stone: the
/// measurement read 161 mV on the pin where this pair would give 250, and what the resistors really
/// are is a thing a voltmeter says, not a thing a document does.
pub const DIVIDER_TOP: Setting = Setting {
    name: "div_top",
    label: "From the sensor",
    default: "10000",
    kind: Kind::Whole { from: 100, to: draught::MOST_OHM, unit: "Ω" },
    level: Level::Work,
    card: Some("draught"),
};

/// The lower arm of the same divider: from the pin to ground. Nothing at all here is not a divider, and the
/// reading says so in its own words rather than as a pressure (crates/draught, `Unconfigured`).
pub const DIVIDER_BOTTOM: Setting = Setting {
    name: "div_bottom",
    label: "To ground",
    default: "10000",
    kind: Kind::Whole { from: 100, to: draught::MOST_OHM, unit: "Ω" },
    level: Level::Work,
    card: Some("draught"),
};

/// Where the flame's states are divided, in millivolts on its pin.
///
/// FEWER MILLIVOLTS MEAN MORE LIGHT - the photoresistor is the lower arm of the pair - so the steady threshold
/// sits BELOW the lit one. That is upside down against every intuition about "more", and nobody is expected to
/// work it out from the numbers: the labels say "at or under", and the note beside them says whether the two
/// stand in the order they have to.
///
/// Seeded with the numbers to start from - 2.4 V and about 1.1 V - and BOTH ARE CALCULATED. They come
/// off the delta of the divider, not off this board: a reading of the dark and of a working flame on GPIO7 with a
/// lamp and a palm, and those two numbers replace these. Until then a person looking at the card is looking at
/// arithmetic, and the range in the label is what the pin can hold at all.
pub const FLAME_LIT: Setting = Setting {
    name: "flame_lit",
    label: "Seen at or under",
    default: "2400",
    kind: Kind::Whole { from: 100, to: 3_300, unit: "mV" },
    level: Level::Work,
    card: Some("flame"),
};

/// The second of the two: at or under this the flame is holding rather than merely lit. Below `FLAME_LIT`, and
/// a pair that crosses is answered as a fault of the setting rather than as a state of the fire (crates/flame).
pub const FLAME_STEADY: Setting = Setting {
    name: "flame_steady",
    label: "Steady at or under",
    default: "1100",
    kind: Kind::Whole { from: 100, to: 3_300, unit: "mV" },
    level: Level::Work,
    card: Some("flame"),
};

/// What the fan is asked to hold, in PASCALS.
///
/// **NOT A PERCENT, and the reason is a measurement.** A percent is the position of a dimmer's knob: the owner
/// read 30 % as 89 Pa and, within the hour, as 47 Pa on the same cold burner. The chimney's draught
/// moves with the weather and the motor behaves differently cold and warm, so there is no table of percent to
/// pascals to be had. Only pascals repeat, and the burner's own factory table is in pascals for the same reason.
///
/// Seeded with 51 Pa - the setpoint the first lighting is to run on, by the owner's word. The span ends at the
/// sensor's own 300 Pa; asking for more than the fan can make leaves the angle sitting on its ceiling, which the
/// card shows as a number rather than hiding.
pub const DRAUGHT_PA: Setting = Setting {
    name: "draught_pa",
    label: "Hold the draught at",
    default: "51",
    kind: Kind::Whole { from: 0, to: draught::FULL_PA, unit: "Pa" },
    level: Level::Work,
    card: Some("fan"),
};

/// The lowest angle the regulator may hold a TURNING fan at.
///
/// Two numbers, not one, and they are the owner's, measured on his own fan on 20.09 (`dimmer::STARTS_AT`,
/// `dimmer::HOLDS_AT`): it will not start under 25 %, and once turning it holds down to 8, labouring. This is
/// the second of them. Breaking away from rest is the kick's business, not this floor's.
pub const FAN_MIN: Setting = Setting {
    name: "fan_min",
    label: "Fan, lowest it is held at",
    // `dimmer::HOLDS_AT`, and a test holds the two together: a default written as a word cannot be built from a
    // number at build time, so the agreement is checked rather than constructed
    default: "8",
    kind: Kind::Whole { from: 5, to: 60, unit: "%" },
    level: Level::Work,
    card: Some("fan"),
};

/// The highest.
///
/// About 80 % of travel is where the MEASUREMENT ends, not where the fan does: the pin reads 2317 mV of a window
/// that closes at 2450, and the sensor is already past its 300 Pa. Above it the regulator would be
/// chasing a number that cannot move. The purge is not subject to it - it runs at the maximum on purpose.
pub const FAN_MAX: Setting = Setting {
    name: "fan_max",
    label: "Fan, highest it is held at",
    default: "80",
    kind: Kind::Whole { from: 20, to: 100, unit: "%" },
    level: Level::Work,
    card: Some("fan"),
};

/// The most the regulator may move the angle in one second. Without a limit a big error throws the angle across
/// its whole travel in one step and the fan bangs.
pub const FAN_STEP: Setting = Setting {
    name: "fan_step",
    label: "Fan, most it moves a second",
    default: "5",
    kind: Kind::Whole { from: 1, to: 50, unit: "%" },
    level: Level::Work,
    card: Some("fan"),
};

/// The four gains: two sets, because the response is not symmetric.
///
/// **THE MISS THEY ACT ON IS IN PERCENTS OF ANGLE, NOT IN PASCALS.** The regulator divides the error by how
/// steeply the draught answers the angle where the fan stands now (`regulator::curve_power`), so what
/// reaches these gains is "the fan is this many percent of angle away from where it should be". That is what
/// lets ONE set of numbers be right at both ends of the travel: the fan itself is not a straight line - 15 %
/// gives 26 Pa, 35 % gives 141 and 50 % gives 245 - and gains in pascals are either sluggish low down or
/// twitchy high up.
///
/// **THOUSANDTHS, and the card says what that comes to in words.** A person types whole numbers, and after the
/// division the number is a plain ratio: 150 means a miss of one percent of angle moves the angle by 0.15 %.
///
/// **300 AND 400, THE SAME BOTH WAYS, AND THE NUMBER IS THE STAND'S.** On the tract measured on the burner (4 s,
/// `regulator::TRACT_ANSWERS_S`) and the four jumps it showed on 26.09 - the igniter switched off, the fire gone out,
/// a small setpoint on an empty pot, a setpoint moved - these bring the draught back within fifteen seconds
/// (crates/regulator/tests/back_to_setpoint.rs), where the numbers of 26.09, 150/45 up and 150/30 down, took half a
/// minute to a minute and three quarters: their slow root was 20-40 s. The fire needs no set of its own - the jump
/// the igniter makes comes with no fire in the pot at all. Near
/// the setpoint they act at a quarter of themselves ([`FAN_BAND`]).
///
/// **A BOARD THAT ALREADY HAS NUMBERS IN ITS nvs KEEPS THEM, AND THEY NOW MEAN SOMETHING ELSE** - five times
/// gentler than they did. Whoever tuned a board before this change re-types these four.
pub const UP_KP: Setting = Setting {
    name: "up_kp",
    label: "Pushing up, by the miss in angle",
    default: "300",
    kind: Kind::Whole { from: 0, to: 2_000, unit: "m%/%" },
    level: Level::Work,
    card: Some("fan"),
};

pub const UP_KI: Setting = Setting {
    name: "up_ki",
    label: "Pushing up, by how long the miss lasts",
    default: "400",
    kind: Kind::Whole { from: 0, to: 2_000, unit: "m%/%·s" },
    level: Level::Work,
    card: Some("fan"),
};

/// Letting go is its own set, and what it is about is the TRACT rather than the travel: a step of the angle
/// reaches 63 % of its change in 4 s going up and in 3 s coming down (`tools/tract_lag_probe.py`, 21.09). The
/// two ends of the travel are no longer its business - the division by the steepness covers those - so whether
/// these two sets can be collapsed into one is now a question for a measurement rather than a matter of
/// principle.
pub const DOWN_KP: Setting = Setting {
    name: "down_kp",
    label: "Letting go, by the miss in angle",
    default: "300",
    kind: Kind::Whole { from: 0, to: 2_000, unit: "m%/%" },
    level: Level::Work,
    card: Some("fan"),
};

pub const DOWN_KI: Setting = Setting {
    name: "down_ki",
    label: "Letting go, by how long the miss lasts",
    default: "400",
    kind: Kind::Whole { from: 0, to: 2_000, unit: "m%/%·s" },
    level: Level::Work,
    card: Some("fan"),
};

/// What the regulator holds the draught by, out of the fan card's settings as VALUE reads them: the board's round reads
/// them afresh every second through this, and the stand's rows through the same, with the defaults.
pub fn fan_tuning(value: &dyn Fn(&Setting) -> String) -> regulator::Tuning {
    let whole = |s: &Setting| value(s).parse::<u32>().unwrap_or(0);
    regulator::Tuning {
        up: regulator::Gains { kp_milli: whole(&UP_KP), ki_milli: whole(&UP_KI) },
        down: regulator::Gains { kp_milli: whole(&DOWN_KP), ki_milli: whole(&DOWN_KI) },
        min: whole(&FAN_MIN) as f32,
        max: whole(&FAN_MAX) as f32,
        step: whole(&FAN_STEP) as f32,
        band_pa: whole(&FAN_BAND) as f32,
        band_share: whole(&FAN_BAND_SHARE) as f32 / 100.0,
    }
}

/// How near the setpoint the gains act at [`FAN_BAND_SHARE`] of themselves, in pascals.
///
/// Four pascals: the air breathes 1.7-1.9 Pa from one second to the next (26.09, under fire and cold), and a band a
/// little wider than twice that keeps the quick gains from answering the breathing with the gate. On the stand the
/// gate then travels about as much near the setpoint as it did on the slow numbers of 26.09, and the jumps still
/// come back within fifteen seconds (crates/regulator/tests/back_to_setpoint.rs). 0 switches the band off.
pub const FAN_BAND: Setting = Setting {
    name: "fan_band",
    label: "Near the setpoint, within",
    default: "4",
    kind: Kind::Whole { from: 0, to: 30, unit: "Pa" },
    level: Level::Work,
    card: Some("fan"),
};

/// What share of their strength the gains act at inside [`FAN_BAND`], in percent.
pub const FAN_BAND_SHARE: Setting = Setting {
    name: "fan_band_share",
    label: "Near the setpoint the gains act at",
    default: "25",
    kind: Kind::Whole { from: 0, to: 100, unit: "%" },
    level: Level::Work,
    card: Some("fan"),
};

// ---- the socket the burner is plugged into ----

/// Whether TEXT is an IPv4 address the board can dial as it is: four numbers of up to three digits, no name to look up, no port, nothing around them.
fn ipv4(text: &str) -> bool {
    text.parse::<core::net::Ipv4Addr>().is_ok()
}

const MAINS_AT_CHOICES: [Choice; 2] = [
    Choice { value: "off", words: "none", takes: None, checks: None, means: Some("no socket is read") },
    Choice {
        value: "address",
        words: "a socket",
        takes: Some("the socket's IPv4 address: 192.0.2.10"),
        checks: Some(ipv4),
        means: Some("the Tasmota socket at the address written below"),
    },
];

/// Where the Tasmota socket the burner is plugged into answers, or none. The burner only READS it (its power, voltage and current): a command to it is never sent, because it powers the whole burner. An address and not a name: the board
/// makes no lookups. A new board reads no socket until a person names one.
pub const MAINS_AT: Setting =
    Setting { name: "mains_at", label: "Socket at", default: "off", kind: Kind::Choice(&MAINS_AT_CHOICES), level: Level::Work, card: MAINS };

/// How often the burner asks the socket. The journal's power after a switch of an organ is the first reading after it, so a period is also how long that takes.
pub const MAINS_EVERY: Setting = Setting { name: "mains_every", label: "Read every", default: "5", kind: Kind::Seconds { from: 1, to: 60 }, level: Level::Work, card: MAINS };

/// Every setting the firmware knows, in the order the tab shows them; a device's description names only these.
pub const ALL: [&Setting; 109] = [
    &AUTO_HOLD,
    &AUTO_HOLD_C,
    &AUTO_DROP_C,
    &AUTO_STEP,
    &AUTO_STEP_MID,
    &AUTO_STEP_SMALL,
    &AUTO_MID_C,
    &AUTO_SMALL_C,
    &AUTO_LOCK_MIN,
    &AUTO_LOCK_C,
    &AUTO_ASKED_BY,
    &AUTO_ROOM_C,
    &AUTO_ROOM_ECO_C,
    &AUTO_ROOM_GAP_C,
    &AUTO_PROFILE,
    &WINDOW_1,
    &WINDOW_2,
    &WINDOW_3,
    &WINDOW_4,
    &WINDOW_5,
    &WINDOW_6,
    &AUTO_PURGE,
    &AUTO_IGNITER,
    &AUTO_WAIT,
    &AUTO_TRIES,
    &AUTO_KINDLE_PA,
    &AUTO_DOSE,
    &AUTO_DOSE_AGAIN,
    &AUTO_RELIGHT_1,
    &AUTO_RELIGHT_2,
    &AUTO_RELIGHT_3,
    &AUTO_RISE_S,
    &AUTO_NO_FLAME_S,
    &AUTO_FLUE_FALL_S,
    &AUTO_BURN_OFF_S,
    &AUTO_PURGE_END,
    &AUTO_OUT_MIN_S,
    &AUTO_SMOULDER_S,
    &AUTO_SMOULD_PA,
    &AUTO_SMOULD_ON,
    &AUTO_SMOULD_OFF,
    &AUTO_FLUE_C,
    &AUTO_FLUE_COLD_C,
    &AUTO_FLUE_FALL_C,
    &AUTO_TOO_HOT_C,
    &AUTO_ALARM_C,
    &AUTO_SURE_S,
    &AUTO_LIT_S,
    &AUTO_EMBER_C,
    &AUTO_SPIN_UP_S,
    &AUTO_LOW_PRESSURE_S,
    &AUTO_PUMP_ON_C,
    &AUTO_PUMP_OFF_C,
    &AUTO_14_ON_S,
    &AUTO_14_OFF_S,
    &AUTO_14_PA,
    &AUTO_10_ON_S,
    &AUTO_10_OFF_S,
    &AUTO_10_PA,
    &AUTO_8_ON_S,
    &AUTO_8_OFF_S,
    &AUTO_8_PA,
    &AUTO_6_ON_S,
    &AUTO_6_OFF_S,
    &AUTO_6_PA,
    &AUTO_5_ON_S,
    &AUTO_5_OFF_S,
    &AUTO_5_PA,
    &AUTO_4_ON_S,
    &AUTO_4_OFF_S,
    &AUTO_4_PA,
    &AUTO_3_ON_S,
    &AUTO_3_OFF_S,
    &AUTO_3_PA,
    &settings::TIME,
    &settings::ZONE,
    &settings::NET_POINT_TRIES,
    &settings::NET_RESET_LAPS,
    &settings::NET_REBOOT_LAPS,
    &AUGER_LIMIT,
    &RESTART_HOUR,
    &PELLET_RATE,
    &SOOT_CLEAN_C,
    &SOOT_DIRTY_C,
    &NTC_ADDRESS,
    &POLL_EVERY,
    &DIVIDER_TOP,
    &DIVIDER_BOTTOM,
    &FLAME_LIT,
    &FLAME_STEADY,
    &DRAUGHT_PA,
    &FAN_MIN,
    &FAN_MAX,
    &FAN_STEP,
    &UP_KP,
    &UP_KI,
    &DOWN_KP,
    &DOWN_KI,
    &FAN_BAND,
    &FAN_BAND_SHARE,
    &ROLE_NTC_1,
    &ROLE_NTC_2,
    &ROLE_NTC_3,
    &ROLE_NTC_4,
    &ROLE_NTC_5,
    &ROLE_NTC_6,
    &ROLE_NTC_7,
    &MAINS_AT,
    &MAINS_EVERY,
];

/// Five times what it was, held inside the setting's own span.
///
/// The four gains act on an error that is now divided by the steepness of the fan's curve, and the steepness
/// at the bottom of the working travel is about 5 Pa a percent - so a gain that moved the angle by a certain
/// amount per pascal has to be five times bigger to move it by the same amount per percent of angle.
///
/// **CLAMPED RATHER THAN REFUSED.** Five times the old top of the span is past the new one, and a value
/// outside its span reads as the default ([`value`]) - so an uncarried maximum would quietly come back as a
/// middling number, undoing what somebody deliberately set to the top.
fn five_times(s: &Setting, old: &str) -> Option<String> {
    let top = match s.kind {
        Kind::Whole { to, .. } => to,
        _ => return None,
    };
    old.parse::<u32>().ok().map(|n| n.saturating_mul(5).min(top).to_string())
}

/// A kept `water` is the supply's: it stood on the socket of the pipe out of the boiler. Any other role stays.
fn water_is_supply(_s: &Setting, old: &str) -> Option<String> {
    Some(if old == "water" { SUPPLY.to_string() } else { old.to_string() })
}

/// A value equal to the old default (40 clean, 60 dirty) is carried as the default, 36 and 51: the store keeps no mark of a hand, and the number meant another scale; any other value stays.
fn scale_of_the_window(s: &Setting, old: &str) -> Option<String> {
    let before = if s.name == "soot_clean_c" { "40" } else { "60" };
    (old != before).then(|| old.to_string())
}

/// The values that have to be carried, and up to which meaning.
const CARRIES: [Carry; 13] = [
    Carry { name: "up_kp", to: 1, carry: five_times },
    Carry { name: "up_ki", to: 1, carry: five_times },
    Carry { name: "down_kp", to: 1, carry: five_times },
    Carry { name: "down_ki", to: 1, carry: five_times },
    Carry { name: "role_ntc_1", to: 2, carry: water_is_supply },
    Carry { name: "role_ntc_2", to: 2, carry: water_is_supply },
    Carry { name: "role_ntc_3", to: 2, carry: water_is_supply },
    Carry { name: "role_ntc_4", to: 2, carry: water_is_supply },
    Carry { name: "role_ntc_5", to: 2, carry: water_is_supply },
    Carry { name: "role_ntc_6", to: 2, carry: water_is_supply },
    Carry { name: "role_ntc_7", to: 2, carry: water_is_supply },
    Carry { name: "soot_clean_c", to: 5, carry: scale_of_the_window },
    Carry { name: "soot_dirty_c", to: 5, carry: scale_of_the_window },
];

/// Whether a person has set S in SET: a kept value that is not its default. **A KEY IS NO EDIT**: the seeding of an image before wrote the defaults of every step as keys, and a board that never had a hand on its table holds them all.
fn edited(set: &[(String, String)], s: &Setting) -> bool {
    set.iter().find(|(n, _)| n == s.name).is_some_and(|(_, v)| v != s.default)
}

/// VALUE under NAME in SET: in the place of the name when it is there, at the end when it is not.
fn put_in(set: &mut Vec<(String, String)>, name: &str, value: &str) {
    match set.iter_mut().find(|(n, _)| n == name) {
        Some(slot) => slot.1 = value.to_string(),
        None => set.push((name.to_string(), value.to_string())),
    }
}

/// The numbers a board burned by when it held a temperature, carried into the step they belong to: what the burner burned by before this image is what it burns by after it.
///
/// **THE PAIR GOES WHERE ITS NUMBERS ALREADY ARE.** A pair that equals the defaults of a step IS that step - the old pair was the 3 kW row before the table existed - so that step becomes the chosen one and its row is not written
/// at all. Carried into the step that happened to be chosen, it made the chosen "6 kW" a small fire and the person set the rows right by hand. A pair that is no step's defaults is the person's own tuning and goes
/// into the step chosen, as it burned.
///
/// **ONLY A BOARD THAT HELD A TEMPERATURE CARRIES ITS PAIR**: in the hold of a level of power the pair was never what burned - the level's own numbers were - and its values are the defaults nobody read. A number this image
/// does not take, or one the step already holds, is left out, and the step keeps its own; **a row a person already set is never written over, and a pair that equals the defaults of such a row moves nothing.** The old names stay in the set: `seed` puts them aside with their values, where nothing is lost.
fn pair_into_step(set: &mut Vec<(String, String)>) -> Vec<String> {
    let kept = |set: &[(String, String)], name: &str| set.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
    let hold = kept(set, AUTO_HOLD.name).unwrap_or_else(|| AUTO_HOLD.default.to_string());
    if hold == "power" {
        return Vec::new();
    }
    let pair = [kept(set, "auto_pa"), kept(set, "auto_feed_on_s"), kept(set, "auto_feed_off_s")];
    let equals = |row: &StepNumbers| {
        matches!(&pair, [Some(pa), Some(on), Some(off)] if *pa == row.pa.default && *on == row.on_s.default && *off == row.off_s.default)
    };
    if let Some(row) = STEP_TABLE.iter().find(|r| equals(r)) {
        let chosen = kept(set, AUTO_STEP.name).unwrap_or_else(|| AUTO_STEP.default.to_string());
        let held = [row.pa, row.on_s, row.off_s].iter().any(|s| edited(set, s));
        if chosen == row.kw || held {
            return Vec::new();
        }
        put_in(set, AUTO_STEP.name, row.kw);
        return vec![format!("setting {} carried from the pair: {}", AUTO_STEP.name, row.kw)];
    }
    let chosen = kept(set, AUTO_STEP.name).unwrap_or_else(|| AUTO_STEP.default.to_string());
    let Some(row) = STEP_TABLE.iter().find(|r| r.kw == chosen) else { return Vec::new() };
    let mut lines = Vec::new();
    for (old, new) in [("auto_pa", row.pa), ("auto_feed_on_s", row.on_s), ("auto_feed_off_s", row.off_s)] {
        let Some(was) = kept(set, old) else { continue };
        if edited(set, new) || !settings::sound(new, &was) {
            continue;
        }
        put_in(set, new.name, &was);
        lines.push(format!("setting {} carried from {old}: {was}", new.name));
    }
    lines
}

/// A board that ran the image with the table and the first carry (meaning 3) holds the pair the burner used to work by in the chosen step, and the pair itself under LEFT, where the first start put it aside. When the
/// chosen step holds EXACTLY the pair, the pair equals the defaults of ANOTHER step, and that step stands at its defaults, the burner burned by those numbers in a row they were never meant for: the choice moves to the
/// step they equal and the chosen one gets its own numbers back. Anything else is a person's, or nobody's, and is left as it is - the owner's "6 kW" set by hand to 22 Pa / 1 s / 21 s against the pair 11 / 1 / 21 is
/// not the pair and is not touched. A hold of power never burned by the pair, so it carries nothing. The pair stays under LEFT: nothing is lost.
fn pair_from_left(set: &mut Vec<(String, String)>, store: &dyn Store) -> Vec<String> {
    let left = settings::kept(store, LEFT);
    let aside = |name: &str| left.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
    let (Some(pa), Some(on), Some(off)) = (aside("auto_pa"), aside("auto_feed_on_s"), aside("auto_feed_off_s")) else { return Vec::new() };
    let held = |set: &[(String, String)], name: &str, default: &str| set.iter().find(|(n, _)| n == name).map_or_else(|| default.to_string(), |(_, v)| v.clone());
    if held(set, AUTO_HOLD.name, AUTO_HOLD.default) == "power" {
        return Vec::new();
    }
    let chosen_kw = held(set, AUTO_STEP.name, AUTO_STEP.default);
    let Some(chosen) = STEP_TABLE.iter().find(|r| r.kw == chosen_kw) else { return Vec::new() };
    let numbers = |row: &StepNumbers| [(row.pa, &pa), (row.on_s, &on), (row.off_s, &off)];
    if numbers(chosen).iter().any(|(s, pair)| held(set, s.name, s.default) != **pair) {
        return Vec::new();
    }
    let Some(target) = STEP_TABLE.iter().find(|r| r.kw != chosen.kw && numbers(r).iter().all(|(s, pair)| s.default == pair.as_str())) else { return Vec::new() };
    if [target.pa, target.on_s, target.off_s].iter().any(|s| edited(set, s)) {
        return Vec::new();
    }
    for s in [chosen.pa, chosen.on_s, chosen.off_s] {
        put_in(set, s.name, s.default);
    }
    put_in(set, AUTO_STEP.name, target.kw);
    vec![format!("setting {} carried from the pair held in {} kW: {}", AUTO_STEP.name, chosen.kw, target.kw)]
}



/// What the mechanism cannot know of this device's set (see [`Rules`]): the values carried across the meanings 1 to 4, the choices that
/// need a socket, and the meaning now.
pub static RULES: Rules = Rules {
    meaning: MEANING,
    carries: &CARRIES,
    carried_cards: &[AUTOMATON],
    carry_sets,
    modes: &[AUTO_HOLD.name, AUTO_ASKED_BY.name],
    refuse,
    bearing: bearing_with_levels,
    among: &SOCKET_ROLES,
};

/// What the SET of settings MEANS now, as a number that goes up by one whenever a value stops meaning what it
/// meant.
///
/// **A NEW IMAGE DOES NOT TOUCH WHAT A PERSON SET IN PLACE - which is right, and is exactly why this exists.**
/// A setting can change its unit between images: the four gains did on 22.09, when the regulator began
/// dividing the error by the steepness of the fan's curve and their unit went from thousandths of a percent
/// of angle per PASCAL to a plain ratio. The number kept in nvs stayed 30 and came back meaning a fifth of
/// what it used to - a board tuned by hand came up five times gentler and said nothing about it. The label on
/// the card changed, so what a person read was right; what the fan did was not.
///
/// 2 - `water` stopped being a role on 23.09, when the boiler and the supply were told apart: a kept `water` stood on
/// the supply's socket and is carried to `supply`.
///
/// 3 - the pair the burner worked by (`auto_pa`, `auto_feed_on_s`, `auto_feed_off_s`) is gone, and the burner burns on the
/// chosen step of the table in every hold: a pair kept by a board that held a temperature is carried into the chosen step
/// ([`pair_into_step`]).
///
/// 4 - the pair goes where its numbers already are: the first carry put it into the step that happened to be chosen, and a board of that image (meaning 3) holds it there while the step it equals stands at
/// its defaults; [`pair_from_left`] moves the choice to that step and gives the chosen one its own numbers back, and touches nothing a person set.
///
/// 5 - the edges of the soot scale (`soot_clean_c`, `soot_dirty_c`) mean the flue minus the water over the window of a burn and not over the whole burn: a board that holds the old defaults, 40 and 60, reads the new ones, 36 and 51
/// ([`scale_of_the_window`]); any other value stays.
pub const MEANING: u32 = 5;

/// The pair of the old burner moves into the table of steps for a set older than meaning 3, and out of what an image put aside for meaning 3.
fn carry_sets(stored: u32, set: &mut Vec<(String, String)>, store: &dyn Store) -> Vec<String> {
    if stored < 3 {
        pair_into_step(set)
    } else if stored < 4 {
        pair_from_left(set, store)
    } else {
        Vec::new()
    }
}

/// A mode by a sensor, or a room that asks, is taken while a socket plays its role, and the roles are read from the set as the save leaves it.
fn refuse(name: &'static str, mode: &str, set: &[(String, String)]) -> Option<Refusal> {
    let role = bearing_with_levels(name, mode).and_then(|b| b.role)?;
    let played = ROLES_OF_SOCKETS.iter().any(|socket| set.iter().any(|(n, v)| n == socket.name && v == role));
    (!played).then(|| Refusal::NoSocket { name, role, said: role_words(role) })
}
