//! API of the automaton: its entities - the stage and what it does by code, the attempt, the reason of a failure with what it carries, where it broke, how long the state has lasted, the clocks counting down, the
//! organs it commands, the feed and the mode as switches, the pump and why, what the fan is asked for, the alarm, the silence of the sensors it sees by, the door's refusal of a number -, the settings it goes by with their limits, and what the module hands the stream for a second. Names, kinds, units, limits, commands, codes of words and reasons; the address of each.
//! NEVER HERE: a card, an order, a label, a level, a phrase for a person, the limits of a picture: page/menu.json says them. Nor a state being decided or a number of the fire: the first is crates/automaton, the numbers
//! are crates/settings (`fire`), which this file only gives names to. A refusal of the door is the one text the card carries, in the door's words (contract 3, "the value on the wire").
//! CHECKS: tests/automaton_table.rs, tests/automaton_values.rs.

use ::automaton::{Asked, Clock, Doing, PumpWhy, State, Why};
use page::dictionary::{joined, Count, Entry, Kind, Said, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json and in burner.yaml.
pub const CARD: &str = "automaton";
/// The switch that allows or bans the feed, and the switch of the mode - Semi on, Auto off -, by the names the web's on and off routes send.
pub const FEED: &str = "feed";
pub const SEMI: &str = "semi";

/// The settings of this module: how the burner is lit, held and put out, the table of its power steps - a dose, a pause and a draught on each - and when the pump runs.
pub const SETTINGS: [&settings::Setting; 52] = [
    &burner_settings::AUTO_PURGE,
    &burner_settings::AUTO_IGNITER,
    &burner_settings::AUTO_WAIT,
    &burner_settings::AUTO_TRIES,
    &burner_settings::AUTO_KINDLE_PA,
    &burner_settings::AUTO_DOSE,
    &burner_settings::AUTO_DOSE_AGAIN,
    &burner_settings::AUTO_RELIGHT_1,
    &burner_settings::AUTO_RELIGHT_2,
    &burner_settings::AUTO_RELIGHT_3,
    &burner_settings::AUTO_RISE_S,
    &burner_settings::AUTO_NO_FLAME_S,
    &burner_settings::AUTO_FLUE_FALL_S,
    &burner_settings::AUTO_BURN_OFF_S,
    &burner_settings::AUTO_PURGE_END,
    &burner_settings::AUTO_OUT_MIN_S,
    &burner_settings::AUTO_SMOULDER_S,
    &burner_settings::AUTO_SMOULD_PA,
    &burner_settings::AUTO_SMOULD_ON,
    &burner_settings::AUTO_SMOULD_OFF,
    &burner_settings::AUTO_FLUE_C,
    &burner_settings::AUTO_FLUE_COLD_C,
    &burner_settings::AUTO_FLUE_FALL_C,
    &burner_settings::AUTO_TOO_HOT_C,
    &burner_settings::AUTO_ALARM_C,
    &burner_settings::AUTO_SURE_S,
    &burner_settings::AUTO_LIT_S,
    &burner_settings::AUTO_EMBER_C,
    &burner_settings::AUTO_SPIN_UP_S,
    &burner_settings::AUTO_PUMP_ON_C,
    &burner_settings::AUTO_PUMP_OFF_C,
    &burner_settings::AUTO_14_ON_S,
    &burner_settings::AUTO_14_OFF_S,
    &burner_settings::AUTO_14_PA,
    &burner_settings::AUTO_10_ON_S,
    &burner_settings::AUTO_10_OFF_S,
    &burner_settings::AUTO_10_PA,
    &burner_settings::AUTO_8_ON_S,
    &burner_settings::AUTO_8_OFF_S,
    &burner_settings::AUTO_8_PA,
    &burner_settings::AUTO_6_ON_S,
    &burner_settings::AUTO_6_OFF_S,
    &burner_settings::AUTO_6_PA,
    &burner_settings::AUTO_5_ON_S,
    &burner_settings::AUTO_5_OFF_S,
    &burner_settings::AUTO_5_PA,
    &burner_settings::AUTO_4_ON_S,
    &burner_settings::AUTO_4_OFF_S,
    &burner_settings::AUTO_4_PA,
    &burner_settings::AUTO_3_ON_S,
    &burner_settings::AUTO_3_OFF_S,
    &burner_settings::AUTO_3_PA,
];

/// The settings added after the table stood, whose rows are in [`LATE`] and not among the others: a row laid in among the settings of the card would move every address after it.
pub const LATE_SETTINGS: [&settings::Setting; 1] = [&burner_settings::AUTO_LOW_PRESSURE_S];

/// The clocks of the automaton (`automaton::CLOCKS`, what its journal says) by their codes, in the same order.
pub const CLOCKS: [(&str, &str); 12] = [
    ("lighting in", "lighting_in"),
    ("dose ends in", "dose_ends_in"),
    ("igniter off in", "igniter_off_in"),
    ("attempt ends in", "attempt_ends_in"),
    ("fed after watching the flame for", "fed_after_flame_for"),
    ("full step in", "full_step_in"),
    ("auger switches in", "auger_switches_in"),
    ("flame not back, lighting again in", "relight_in"),
    ("flame not back, burning off in", "no_relight_burning_off_in"),
    ("burning off in", "burning_off_in"),
    ("burning out ends in at most", "burning_out_ends_in"),
    ("blowing clear ends in", "blowing_clear_ends_in"),
];

const fn clock(name: &'static str) -> Entry {
    Entry { clock: true, counts: Some(Count::Down), ..Entry::row(CARD, name, &[Kind::Seconds]) }
}

/// Why a clock has no number: the thing it counts down to has not begun - the blow behind a burning out, the watch over a dark flame, the climb to a full step.
pub const NOT_BEGUN: &str = "not_begun";

/// A clock that has a state before its thing begins: a number from the second it does, and `not_begun` until then.
const fn clock_that_may_not_have_begun(name: &'static str) -> Entry {
    Entry { reasons: &[NOT_BEGUN], ..Entry { clock: true, counts: Some(Count::Down), ..Entry::row(CARD, name, &[Kind::Seconds, Kind::Unknown]) } }
}

const fn organ(name: &'static str) -> Entry {
    Entry::row(CARD, name, &[Kind::On])
}

/// The rows of the automaton: the stage and what it does, how long, what ends and when, each organ, the switches, the pump, the fan it asks for, the alarm, the refusal. The buttons and whether heat is wanted are the thermostat's.
const ROWS: [Entry; 40] = [
    Entry { words: ::automaton::STAGES, reasons: &["starting"], ..Entry::row(CARD, "stage", &[Kind::Word, Kind::Unknown]) },
    Entry {
        words: &["stop", "purge", "kindling", "rising", "working", "smouldering", "burning_off", "burning_off_because", "fault"],
        ..Entry::row(CARD, "doing", &[Kind::Word])
    },
    Entry { unit: "attempt", decimals: Some(0), ..Entry::part(CARD, "doing", "attempt", &[Kind::Number]) },
    Entry { words: ::automaton::WHYS, ..Entry::part(CARD, "doing", "why", &[Kind::Word]) },
    Entry { unit: "attempts", decimals: Some(0), ..Entry::part(CARD, "doing", "tries", &[Kind::Number]) },
    Entry { unit: "°C", decimals: Some(1), ..Entry::part(CARD, "doing", "water", &[Kind::Number]) },
    Entry { words: ::automaton::STAGES, ..Entry::row(CARD, "broke_at", &[Kind::Word]) },
    // how long each sensor the automaton sees by has been silent; nothing while it speaks
    Entry::row(CARD, "silent_draught", &[Kind::Seconds]),
    Entry::row(CARD, "silent_flame", &[Kind::Seconds]),
    Entry::row(CARD, "silent_flue", &[Kind::Seconds]),
    Entry::row(CARD, "silent_water", &[Kind::Seconds]),
    // how long the stage has lasted: it counts up by itself, and the reader counts it
    Entry { counts: Some(Count::Up), ..Entry::row(CARD, "for", &[Kind::Seconds]) },
    clock("lighting_in"),
    clock("dose_ends_in"),
    clock("igniter_off_in"),
    clock("attempt_ends_in"),
    clock_that_may_not_have_begun("fed_after_flame_for"),
    clock_that_may_not_have_begun("full_step_in"),
    clock("auger_switches_in"),
    clock("relight_in"),
    clock("no_relight_burning_off_in"),
    clock("burning_off_in"),
    clock("burning_out_ends_in"),
    clock_that_may_not_have_begun("blowing_clear_ends_in"),
    organ("auger"),
    Entry { can: &["switch"], ..Entry::row(CARD, FEED, &[Kind::On]) },
    Entry { can: &["switch"], ..Entry::row(CARD, SEMI, &[Kind::On]) },
    organ("igniter"),
    organ("pump"),
    Entry { words: &["water", "overheated", "no_number"], ..Entry::row(CARD, "pump_by", &[Kind::Word]) },
    Entry { unit: "°C", decimals: Some(1), ..Entry::part(CARD, "pump_by", "water", &[Kind::Number]) },
    Entry { unit: "Pa", decimals: Some(0), words: &["off", "full", "by_hand"], ..Entry::row(CARD, "fan", &[Kind::Number, Kind::Word]) },
    organ("alarm"),
    Entry { unit: "°C", decimals: Some(1), ..Entry::part(CARD, "alarm", "past", &[Kind::Number]) },
    Entry { words: &::automaton::ORGANS, ..Entry::row(CARD, "organs", &[Kind::Words]) },
    // the door's refusal of the settings' numbers: the code, the field, the number that stood there and, for an order of two numbers, the field it was held against and its number - the words are the page's
    Entry { words: &::automaton::REFUSALS, ..Entry::row(CARD, "refused", &[Kind::Word]) },
    Entry::part(CARD, "refused", "field", &[Kind::Text]),
    Entry { unit: "units", decimals: Some(1), ..Entry::part(CARD, "refused", "value", &[Kind::Number]) },
    Entry::part(CARD, "refused", "than", &[Kind::Text]),
    Entry { unit: "units", decimals: Some(1), ..Entry::part(CARD, "refused", "limit", &[Kind::Number]) },
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

/// How long the zero-crossing pin has given no fronts, in seconds; nothing while it gives them. The row stands apart from the card's others, last in the burner's table (`crate::ALL`), because a row laid in among them would move the address of every row after it.
pub const LATE: [Entry; 5] = [
    Entry::row(CARD, "silent_fronts", &[Kind::Seconds]),
    // what a person is told while the fault for want of fronts waits: the seconds of the run it still owes, the faults let go of today, and how many a day are let go of
    Entry::row(CARD, "relief_in", &[Kind::Seconds]),
    Entry { unit: "times", decimals: Some(0), ..Entry::row(CARD, "relief_today", &[Kind::Number]) },
    Entry { unit: "times", decimals: Some(0), ..Entry::row(CARD, "relief_limit", &[Kind::Number]) },
    // the term of the low pressure: a setting of the card, laid last for the same reason
    Entry::row(page::SETTINGS, "auto_low_pa_s", &[Kind::Text]),
];

/// Every setting this module keeps on the board, the ones laid in the table's middle and the ones laid last: THE LIST THE DEVICE'S MODULE REGISTERS (devices/burner, `Module::settings`). A setting the module does not register is read as its default and
/// refused on save - the page offers it, the automaton runs on the default, and nobody can turn it (6.23, 29.10: `auto_low_pa_s` stood in the dictionary and not in the module's list).
pub fn kept() -> Vec<&'static settings::Setting> {
    SETTINGS.iter().chain(LATE_SETTINGS.iter()).copied().collect()
}

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    kept().into_iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::{at, part_at};

    const fn row(name: &str) -> usize {
        at(&crate::ALL, super::CARD, name)
    }
    const fn part(row: &str, name: &str) -> usize {
        part_at(&crate::ALL, super::CARD, row, name)
    }

    pub const STAGE: usize = row("stage");
    pub const DOING: usize = row("doing");
    pub const DOING_ATTEMPT: usize = part("doing", "attempt");
    pub const DOING_WHY: usize = part("doing", "why");
    pub const DOING_TRIES: usize = part("doing", "tries");
    pub const DOING_WATER: usize = part("doing", "water");
    pub const BROKE_AT: usize = row("broke_at");
    pub const SILENT_DRAUGHT: usize = row("silent_draught");
    pub const SILENT_FLAME: usize = row("silent_flame");
    pub const SILENT_FLUE: usize = row("silent_flue");
    pub const SILENT_WATER: usize = row("silent_water");
    pub const SILENT_FRONTS: usize = row("silent_fronts");
    pub const RELIEF_IN: usize = row("relief_in");
    pub const RELIEF_TODAY: usize = row("relief_today");
    pub const RELIEF_LIMIT: usize = row("relief_limit");
    pub const FOR: usize = row("for");
    pub const AUGER: usize = row("auger");
    pub const FEED: usize = row(super::FEED);
    pub const SEMI: usize = row(super::SEMI);
    pub const IGNITER: usize = row("igniter");
    pub const PUMP: usize = row("pump");
    pub const PUMP_BY: usize = row("pump_by");
    pub const PUMP_BY_WATER: usize = part("pump_by", "water");
    pub const FAN: usize = row("fan");
    pub const ALARM: usize = row("alarm");
    pub const ALARM_PAST: usize = part("alarm", "past");
    pub const ORGANS: usize = row("organs");
    pub const REFUSED: usize = row("refused");
    pub const REFUSED_FIELD: usize = part("refused", "field");
    pub const REFUSED_VALUE: usize = part("refused", "value");
    pub const REFUSED_THAN: usize = part("refused", "than");
    pub const REFUSED_LIMIT: usize = part("refused", "limit");
}

/// The address of the clock the automaton calls WHAT (`automaton::CLOCKS`); None for one the table has not.
pub fn clock_at(what: &str) -> Option<usize> {
    let (_, code) = CLOCKS.iter().find(|(said, _)| *said == what)?;
    page::dictionary::address(&crate::TABLE, CARD, code)
}

/// How long each sensor the automaton sees by has been silent, in whole seconds: past three periods of its round, when its reading is no longer given out. None - it speaks. Said by the automaton, because the card of
/// a sensor whose round has stopped is said by nobody. The return and the air are the thermostat's to say (`thermostat::Silent`).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Silent {
    pub draught: Option<u32>,
    pub flame: Option<u32>,
    pub flue: Option<u32>,
    pub water: Option<u32>,
    /// The zero-crossing pin: how long it has given no fronts.
    pub fronts: Option<u32>,
}

/// What the automaton lets go of by itself (`automaton::Relief`), as it stands this second: None - nothing is owed, or the board does not know.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct ReliefNow {
    /// The seconds of the run the fault for want of fronts still owes before it is let go of.
    pub left_s: Option<u32>,
    /// How many faults were let go of today.
    pub today: Option<u32>,
}

/// What the automaton did in its last second, and how it came about.
pub struct Now<'a> {
    /// What it did in its last second; None before its first.
    pub doing: Option<Doing>,
    /// How long it has been in its state, in whole seconds.
    pub lasted_s: u32,
    /// Why the numbers of its settings could not be taken, as the door says it; None - they were.
    pub refused: Option<&'a ::automaton::Refused>,
    /// What ends in this state and in how many seconds.
    pub clocks: &'a [Clock],
    /// The stage it failed on, while it burns off after the failure and stands in the fault.
    pub broke_at: Option<&'static str>,
    /// Whether the auger may turn.
    pub feed_allowed: bool,
    /// Whether the automaton runs in Semi.
    pub semi: bool,
    /// How long each sensor it sees by has been silent.
    pub silent: Silent,
    /// What it lets go of by itself, and how far along that is.
    pub relief: ReliefNow,
    /// The water the alarm sounds past, as the setting stands now.
    pub alarm_c: f32,
}

/// What the stream is told of the automaton for NOW: every row and part of the card, [`Value::None`] where it has nothing to say in this state, each by its address as `put` takes it.
///
/// **THE SAME ADDRESSES IN EVERY STATE**: a reader sees each of them every time the card is sent, and the card's shape never follows the stage.
pub fn put_values<'a>(now: &Now<'a>, put: &mut dyn FnMut(usize, Said<'a>)) {
    let mut put_value = |address: usize, value: Value| put(address, Said::Value(value));
    let or_none = |v: Option<Value>| v.unwrap_or(Value::None);
    let state = now.doing.map(|d| d.state);
    put_value(at::STAGE, state.map_or(Value::Unknown("starting"), |s| Value::Word(::automaton::stage(s))));
    let why = match state {
        Some(State::BurningOff(Some(why)) | State::Fault(why)) => Some(why),
        _ => None,
    };
    put_value(
        at::DOING,
        or_none(state.map(|s| {
            Value::Word(match s {
                State::BurningOff(Some(_)) => "burning_off_because",
                other => ::automaton::stage(other),
            })
        })),
    );
    put_value(
        at::DOING_ATTEMPT,
        or_none(match state {
            Some(State::Kindling(n)) => Some(Value::Number(n as f32)),
            _ => None,
        }),
    );
    put_value(at::DOING_WHY, or_none(why.map(|w| Value::Word(::automaton::why_name(w)))));
    put_value(
        at::DOING_TRIES,
        or_none(match why {
            Some(Why::NeverLit(n) | Why::FlameNotBack(n)) => Some(Value::Number(n as f32)),
            _ => None,
        }),
    );
    put_value(
        at::DOING_WATER,
        or_none(match why {
            Some(Why::Overheated(c)) => Some(Value::Number(c)),
            _ => None,
        }),
    );
    put_value(at::BROKE_AT, or_none(now.broke_at.map(Value::Word)));
    // what a person is told while the fault for want of fronts waits: the seconds owed, the faults let go of today, and how many a day there are
    put_value(at::RELIEF_IN, or_none(now.relief.left_s.map(Value::Seconds)));
    put_value(at::RELIEF_TODAY, or_none(now.relief.today.map(|n| Value::Number(n as f32))));
    put_value(at::RELIEF_LIMIT, Value::Number(::automaton::RELIEF_PER_DAY as f32));
    let s = now.silent;
    for (address, silent) in [(at::SILENT_DRAUGHT, s.draught), (at::SILENT_FLAME, s.flame), (at::SILENT_FLUE, s.flue), (at::SILENT_WATER, s.water), (at::SILENT_FRONTS, s.fronts)] {
        put_value(address, or_none(silent.map(Value::Seconds)));
    }
    put_value(at::FOR, or_none(now.doing.map(|_| Value::Seconds(now.lasted_s))));
    // THE CLOCKS COUNT DOWN: whole seconds up, so a clock never reads nought while its thing still runs; one whose thing has not begun has no number
    for what in ::automaton::CLOCKS {
        let Some(address) = clock_at(what) else { continue };
        let left = now.clocks.iter().find(|c| c.what == what).map(|c| c.left_s.map_or(Value::Unknown(NOT_BEGUN), |left| Value::Seconds(left.ceil() as u32)));
        put_value(address, or_none(left));
    }
    let doing = now.doing;
    put_value(at::AUGER, or_none(doing.map(|d| Value::On(d.auger))));
    put_value(at::FEED, Value::On(now.feed_allowed));
    put_value(at::SEMI, Value::On(now.semi));
    put_value(at::IGNITER, or_none(doing.map(|d| Value::On(d.igniter))));
    put_value(at::PUMP, or_none(doing.map(|d| Value::On(d.pump))));
    put_value(at::PUMP_BY, or_none(doing.map(|d| Value::Word(::automaton::pump_word(d.pump_why)))));
    put_value(
        at::PUMP_BY_WATER,
        or_none(doing.and_then(|d| match d.pump_why {
            PumpWhy::Water(c) | PumpWhy::Overheated(c) => Some(Value::Number(c)),
            PumpWhy::NoNumber => None,
        })),
    );
    put_value(
        at::FAN,
        or_none(doing.map(|d| match d.fan {
            Asked::Stop => Value::Word("off"),
            Asked::Purge => Value::Word("full"),
            Asked::Hold(pa) => Value::Number(pa),
            Asked::ByHand => Value::Word("by_hand"),
        })),
    );
    let alarm = doing.is_some_and(|d| d.alarm);
    put_value(at::ALARM, if alarm { Value::On(true) } else { Value::None });
    put_value(at::ALARM_PAST, if alarm { Value::Number(now.alarm_c) } else { Value::None });
    put_value(at::ORGANS, Value::Words(&::automaton::ORGANS));
    let refused = now.refused;
    put(at::REFUSED, Said::Value(or_none(refused.map(|r| Value::Word(r.code)))));
    put(at::REFUSED_FIELD, refused.map_or(Said::Value(Value::None), |r| Said::Text(&r.field)));
    put(at::REFUSED_VALUE, Said::Value(or_none(refused.and_then(|r| r.value).map(Value::Number))));
    put(at::REFUSED_THAN, refused.and_then(|r| r.than).map_or(Said::Value(Value::None), Said::Text));
    put(at::REFUSED_LIMIT, Said::Value(or_none(refused.and_then(|r| r.limit).map(Value::Number))));
}
