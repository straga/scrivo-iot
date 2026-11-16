//! API of the thermostat - the water layer, which the page calls the boiler: its entities - whether a person asked for heat, what the water says of burning with the house's word laid over it, the level the fire burns on and how far the water is from the setpoint, the setpoint, what is left of the wait before a light, the three buttons - and the ten settings of its card: the mode, the setpoint, the hysteresis, the level, the middle and the small level with the distances they begin at, the wait before a light and the water cold enough to end it. Who asks and the profiles by the clock are the demand's (demand.rs).
//! NEVER HERE: a card, an order, a label, a level of the page, a phrase for a person: page/menu.json says them. Nor whether the water is high or low (crates/thermostat) or a number of the burner (crates/settings).
//! CHECKS: tests/thermostat_table.rs, tests/thermostat_values.rs.

use ::automaton::{Switch, Water};
use page::dictionary::{joined, Count, Entry, Kind, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json.
pub const CARD: &str = "thermostat";
/// The buttons, by the names the web's press route sends: the main switch, over everything the thermostat will come to ask for, and the light, the fire lit now past the thermostat.
pub const START: &str = "start";
pub const STOP: &str = "stop";
pub const LIGHT: &str = "light";

/// The settings of this module: what the burner holds, the setpoint, how far under it the burner works again, the level it burns on, the levels of the way up to the setpoint - the middle and the small one, and the distances they begin at -, and the wait before a light.
pub const SETTINGS: [&settings::Setting; 10] = [
    &burner_settings::AUTO_HOLD,
    &burner_settings::AUTO_HOLD_C,
    &burner_settings::AUTO_DROP_C,
    &burner_settings::AUTO_STEP,
    &burner_settings::AUTO_STEP_MID,
    &burner_settings::AUTO_STEP_SMALL,
    &burner_settings::AUTO_MID_C,
    &burner_settings::AUTO_SMALL_C,
    &burner_settings::AUTO_LOCK_MIN,
    &burner_settings::AUTO_LOCK_C,
];

const fn button(name: &'static str) -> Entry {
    Entry { can: &["press"], ..Entry::row(CARD, name, &[Kind::Button]) }
}

/// The rows of the thermostat: whether a person asked for heat, what the water says of burning, the level the order carries and how far under the setpoint the water is, the three buttons.
const ROWS: [Entry; 11] = [
    Entry { words: &::automaton::SWITCHES, ..Entry::row(CARD, "switch", &[Kind::Word]) },
    // whose word the switch is: a person's press since the board came up, or the memory of the last one; none while nobody has asked
    Entry { words: &::thermostat::FROMS, reasons: &["none"], ..Entry::row(CARD, "switch_from", &[Kind::Word, Kind::Unknown]) },
    Entry { words: &::automaton::WATERS, reasons: &["starting"], ..Entry::row(CARD, "heat", &[Kind::Word, Kind::Unknown]) },
    // how long the sensor the thermostat holds by has been silent, beside the reason it gives for not asking for heat; nothing while it speaks or no socket plays its role
    Entry::row(CARD, "silent_return", &[Kind::Seconds]),
    // none: the mode holds no temperature, or no reading has come yet
    Entry { words: &::thermostat::RUNGS, reasons: &["none"], ..Entry::row(CARD, "rung", &[Kind::Word, Kind::Unknown]) },
    Entry { unit: "°C", decimals: Some(1), reasons: &["none"], ..Entry::row(CARD, "left", &[Kind::Number, Kind::Unknown]) },
    // the setpoint of the water the last order was made by; none in the mode that holds no temperature
    Entry { unit: "°C", decimals: Some(1), reasons: &["none"], ..Entry::row(CARD, "setpoint", &[Kind::Number, Kind::Unknown]) },
    // what is left of the wait before the thermostat lights the fire again by itself: a clock that counts down, in seconds; none where there is no wait
    Entry { clock: true, counts: Some(Count::Down), ..Entry::row(CARD, "lock", &[Kind::Seconds]) },
    button(START),
    button(STOP),
    button(LIGHT),
];

/// The manual cycle a person's Light began, as two rows laid LAST in the burner's table (`crate::ALL`), so that no address before them moves: what the cycle is carried to - the room's target or the water's setpoint, none while there is no cycle -, and the number of degrees
/// it ends at. The card says it beside the buttons, so a person who pressed Light sees what the burner is burning for.
pub const LATE: [Entry; 2] = [
    Entry { words: &::thermostat::NOMINALS, reasons: &["none"], ..Entry::row(CARD, "manual", &[Kind::Word, Kind::Unknown]) },
    Entry { unit: "°C", decimals: Some(1), reasons: &["none"], ..Entry::row(CARD, "manual_c", &[Kind::Number, Kind::Unknown]) },
];

/// The settings as rows of the settings card: what each keeps, as it is kept.
const fn setting_rows() -> [Entry; SETTINGS.len()] {
    let mut out = [Entry::row("", "", &[]); SETTINGS.len()];
    let mut i = 0;
    while i < SETTINGS.len() {
        out[i] = Entry::row(page::SETTINGS, SETTINGS[i].name, &[Kind::Text]);
        i += 1;
    }
    out
}

const SETTING_ROWS: [Entry; SETTINGS.len()] = setting_rows();

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; SETTING_ROWS.len() + ROWS.len()] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::at;

    const fn row(name: &str) -> usize {
        at(&crate::ALL, super::CARD, name)
    }

    pub const SWITCH: usize = row("switch");
    pub const SWITCH_FROM: usize = row("switch_from");
    pub const HEAT: usize = row("heat");
    pub const SILENT_RETURN: usize = row("silent_return");
    pub const RUNG: usize = row("rung");
    pub const LEFT: usize = row("left");
    pub const SETPOINT: usize = row("setpoint");
    pub const LOCK: usize = row("lock");
    pub const MANUAL: usize = row("manual");
    pub const MANUAL_C: usize = row("manual_c");
    pub const START: usize = row(super::START);
    pub const STOP: usize = row(super::STOP);
    pub const LIGHT: usize = row(super::LIGHT);
}

/// How long the sensor the thermostat holds by has been silent, in whole seconds: past three periods of its round, when its reading is no longer given out. None - it speaks, or no socket plays its role.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Silent {
    /// The return's sensor, which the burner holds by in the hold by the return.
    pub return_c: Option<u32>,
}

impl Silent {
    /// The seconds a role's sensor has been silent, as the card says them: SILENT_FOR - the seconds it has gone without a number, None while it speaks - when a role is ASSIGNED to a sensor, and nothing when none is.
    /// **A ROLE NOBODY PLAYS IS NOT SILENT, IT IS NOT THERE**: its role reads nothing from the day the board started, and said by the seconds it would be a sensor that had failed since the start. The board and the stand
    /// take the same rule from here, and `burner_settings::plays` says whether a role is played.
    pub fn said(silent_for: Option<u32>, assigned: bool) -> Option<u32> {
        if assigned {
            silent_for
        } else {
            None
        }
    }
}

/// What the thermostat orders now, and whether a person asked.
pub struct Now {
    /// Whether a person asked for heat.
    pub switch: Switch,
    /// Whether the switch stands as the board's memory gave it after a restart, and no person has pressed since.
    pub from_memory: bool,
    /// What the water said of burning in the last order; None before the first.
    pub water: Option<Water>,
    /// The level the last order was taken from; None in the mode that holds no temperature and before the first reading.
    pub rung: Option<::thermostat::Rung>,
    /// How far under the setpoint the held water was in the last order, below nought over it; None when it said nothing.
    pub left_c: Option<f32>,
    /// The setpoint of the water the last order was made by, in degrees; None in the mode that holds no temperature.
    pub setpoint_c: Option<f32>,
    /// The seconds left before the thermostat lights the fire again by itself; None where there is no wait.
    pub lock_left_s: Option<f32>,
    /// How long the sensor it holds by has been silent.
    pub silent: Silent,
    /// The manual cycle a person's Light began and has not ended: what it is carried to and the number it ends at; None while there is none.
    pub manual: Option<::thermostat::Cycle>,
}

/// What the stream is told of the thermostat for NOW: every row of the card, each by its address as `put` takes it. **THE SAME ADDRESSES IN EVERY STATE.**
pub fn put_values(now: &Now, put: &mut dyn FnMut(usize, Value)) {
    put(at::SWITCH, Value::Word(::automaton::switch_word(now.switch)));
    put(at::SWITCH_FROM, if now.switch == Switch::Unasked { Value::Unknown("none") } else { Value::Word(::thermostat::from_word(now.from_memory)) });
    put(at::HEAT, now.water.map_or(Value::Unknown("starting"), |water| Value::Word(::automaton::water_word(water))));
    put(at::SILENT_RETURN, now.silent.return_c.map_or(Value::None, Value::Seconds));
    put(at::RUNG, now.rung.map_or(Value::Unknown("none"), |rung| Value::Word(::thermostat::rung_word(rung))));
    put(at::LEFT, now.left_c.map_or(Value::Unknown("none"), Value::Number));
    put(at::SETPOINT, now.setpoint_c.map_or(Value::Unknown("none"), Value::Number));
    // whole seconds, rounded up: a page never says nought while a second is left, as the automaton's clocks
    put(at::LOCK, now.lock_left_s.map_or(Value::None, |left| Value::Seconds(left.ceil() as u32)));
    // the manual cycle: what it is carried to and where it ends; none in both while there is none
    put(at::MANUAL, now.manual.map_or(Value::Unknown("none"), |cycle| Value::Word(::thermostat::nominal_word(cycle.nominal))));
    put(at::MANUAL_C, now.manual.map_or(Value::Unknown("none"), |cycle| Value::Number(cycle.to_c)));
}
