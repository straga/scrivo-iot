//! The pellet burner: every row of its page, every number that used to stand inside a row's words, its settings, and
//! the numbers fixed in its image - by codes, for machines.

use page::dictionary::{at, joined, part_at, Entry, Kind, Range};

use dictionary::system;

pub const AUTOMATON: &str = "automaton";
pub const FIXED_CARD: &str = "automaton_fixed";
pub const RELAYS: &str = "relays";
pub const TEMPERATURES: &str = "temperatures";
pub const FLUE: &str = "flue";
pub const FAN: &str = "fan";
pub const DRAUGHT: &str = "draught";
pub const FLAME: &str = "flame";

/// The cards of the burner, in the order its page draws them.
pub const CARDS: [&str; 8] = [AUTOMATON, FIXED_CARD, RELAYS, TEMPERATURES, FLUE, FAN, DRAUGHT, FLAME];

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
    Entry { clock: true, ..Entry::row(AUTOMATON, name, &[Kind::Seconds]) }
}

/// A clock that has a state before its thing begins: `not_begun` until then, the same word the new burner's card says (`automaton::NOT_BEGUN`).
const fn clock_that_may_not_have_begun(name: &'static str) -> Entry {
    Entry { clock: true, reasons: &[crate::automaton::NOT_BEGUN], ..Entry::row(AUTOMATON, name, &[Kind::Seconds, Kind::Unknown]) }
}

const fn organ(name: &'static str) -> Entry {
    Entry::row(AUTOMATON, name, &[Kind::On])
}

const fn button(name: &'static str) -> Entry {
    Entry { can: &["press"], ..Entry::row(AUTOMATON, name, &[Kind::Button]) }
}

const fn fixed(name: &'static str, value: f32, unit: &'static str) -> Entry {
    Entry { unit, fixed: Some(value), ..Entry::row(FIXED_CARD, name, &[Kind::Number]) }
}

/// A number of the board fixed in its code that a row's words name: a part with its value in the dictionary.
const fn constant(card: &'static str, row: &'static str, name: &'static str, value: f32, unit: &'static str) -> Entry {
    Entry { unit, fixed: Some(value), ..Entry::part(card, row, name, &[Kind::Number]) }
}

/// A relay of the burner, by hand from the page and the console.
const fn relay(name: &'static str) -> Entry {
    Entry { can: &["switch", "toggle"], ..Entry::row(RELAYS, name, &[Kind::On]) }
}

/// A channel of the bus's temperatures: what it reads, the socket's role, its curve.
const fn channel(name: &'static str, curve: &'static [&'static str]) -> Entry {
    Entry { unit: "°C", reasons: &["no_sensor", "not_read"], curves: curve, ..Entry::row(TEMPERATURES, name, &[Kind::Number, Kind::Unknown]) }
}

const fn role(channel: &'static str) -> Entry {
    Entry { words: &ROLE_CODES, ..Entry::part(TEMPERATURES, channel, "role", &[Kind::Word, Kind::Text]) }
}

/// The roles a socket of the bus can play (`settings::ROLES` by value); a name of one's own comes as text.
pub const ROLE_CODES: [&str; 7] = ["free", "boiler", "supply", "return", "flue", "hopper", "room"];

/// A pin read through the converter: its millivolts, how far the samples of one window spread, and where the number
/// stands against the converter's top and the window its calibration holds in.
const fn pin(card: &'static str) -> [Entry; 7] {
    [
        Entry { unit: "mV", reasons: &["not_read"], ..Entry::row(card, "pin", &[Kind::Number, Kind::Unknown]) },
        Entry { unit: "mV", ..Entry::part(card, "pin", "spread", &[Kind::Number]) },
        constant(card, "pin", "samples", 20.0, "samples"),
        // the stronger fact of the two, said beside the number: the top of the scale, else outside the window
        Entry { words: &["top", "under", "over", "inside"], ..Entry::part(card, "pin", "edge", &[Kind::Word]) },
        // where the number stands against the calibrated window, whatever the top: what the draught's level is judged by
        Entry { words: &["under", "inside", "over"], ..Entry::part(card, "pin", "window", &[Kind::Word]) },
        constant(card, "pin", "window_from", analog::WINDOW_MV.0 as f32, "mV"),
        constant(card, "pin", "window_to", analog::WINDOW_MV.1 as f32, "mV"),
    ]
}

const DRAUGHT_PIN: [Entry; 7] = pin(DRAUGHT);
const FLAME_PIN: [Entry; 7] = pin(FLAME);

/// The burner's own rows, after the system card's.
pub const CARDED: [Entry; 130] = [
    // ---- the automaton ----
    Entry { words: automaton::STAGES, reasons: &["starting"], ..Entry::row(AUTOMATON, "stage", &[Kind::Word, Kind::Unknown]) },
    Entry {
        words: &["stop", "purge", "kindling", "rising", "working", "smouldering", "burning_off", "burning_off_because", "fault"],
        ..Entry::row(AUTOMATON, "doing", &[Kind::Word])
    },
    Entry { unit: "attempt", ..Entry::part(AUTOMATON, "doing", "attempt", &[Kind::Number]) },
    Entry { words: automaton::WHYS, ..Entry::part(AUTOMATON, "doing", "why", &[Kind::Word]) },
    Entry { unit: "attempts", ..Entry::part(AUTOMATON, "doing", "tries", &[Kind::Number]) },
    Entry { unit: "°C", ..Entry::part(AUTOMATON, "doing", "water", &[Kind::Number]) },
    Entry { words: automaton::STAGES, ..Entry::row(AUTOMATON, "broke_at", &[Kind::Word]) },
    // how long each sensor the automaton sees by has been silent; nothing while it speaks
    Entry::row(AUTOMATON, "silent_draught", &[Kind::Seconds]),
    Entry::row(AUTOMATON, "silent_flame", &[Kind::Seconds]),
    Entry::row(AUTOMATON, "silent_flue", &[Kind::Seconds]),
    Entry::row(AUTOMATON, "silent_water", &[Kind::Seconds]),
    Entry::row(AUTOMATON, "for", &[Kind::Seconds]),
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
    Entry { can: &["switch"], ..Entry::row(AUTOMATON, "feed", &[Kind::On]) },
    Entry { can: &["switch"], ..Entry::row(AUTOMATON, "semi", &[Kind::On]) },
    organ("igniter"),
    organ("pump"),
    Entry { words: &["water", "overheated", "no_number"], ..Entry::row(AUTOMATON, "pump_by", &[Kind::Word]) },
    Entry { unit: "°C", ..Entry::part(AUTOMATON, "pump_by", "water", &[Kind::Number]) },
    Entry { unit: "Pa", words: &["off", "full", "by_hand"], ..Entry::row(AUTOMATON, "fan", &[Kind::Number, Kind::Word]) },
    organ("alarm"),
    Entry { unit: "°C", ..Entry::part(AUTOMATON, "alarm", "past", &[Kind::Number]) },
    Entry { words: &automaton::ORGANS, ..Entry::row(AUTOMATON, "organs", &[Kind::Words]) },
    // the door's refusal of the card's numbers, in the door's words: the one text of the card
    Entry::row(AUTOMATON, "refused", &[Kind::Text]),
    button("start"),
    button("stop"),
    // ---- the numbers fixed in the image ----
    fixed("purge_after_s", 20.0, "s"),
    fixed("burn_out_min_s", 60.0, "s"),
    fixed("smoulder_s", 300.0, "s"),
    fixed("smoulder_pa", 0.0, "Pa"),
    fixed("smoulder_on_s", 1.0, "s"),
    fixed("smoulder_off_s", 50.0, "s"),
    fixed("flue_holds_c", 100.0, "°C"),
    fixed("flue_cold_c", 70.0, "°C"),
    fixed("flue_fall_c", 10.0, "°C"),
    fixed("lit_s", 5.0, "s"),
    fixed("too_hot_c", 88.0, "°C"),
    fixed("alarm_c", 95.0, "°C"),
    fixed("sure_s", 10.0, "s"),
    // a lost flame over a flue this hot is lit again from the embers
    fixed("relight_hot_c", 45.0, "°C"),
    fixed("spin_up_s", 10.0, "s"),
    // ---- the relays ----
    relay("auger"),
    Entry::part(RELAYS, "auger", "off_in", &[Kind::Seconds]),
    relay("igniter"),
    Entry::part(RELAYS, "igniter", "off_in", &[Kind::Seconds]),
    relay("pump"),
    relay("aux_power"),
    // ---- the temperatures on the bus ----
    channel("ntc_1", &["water_1"]),
    role("ntc_1"),
    channel("ntc_2", &["water_2"]),
    role("ntc_2"),
    channel("ntc_3", &["water_3"]),
    role("ntc_3"),
    channel("ntc_4", &["water_4"]),
    role("ntc_4"),
    channel("ntc_5", &["water_5"]),
    role("ntc_5"),
    channel("ntc_6", &["water_6"]),
    role("ntc_6"),
    channel("ntc_7", &["water_7"]),
    role("ntc_7"),
    // on while every socket is free: the card then says so rather than stand empty
    Entry::row(TEMPERATURES, "none_assigned", &[Kind::On]),
    Entry { words: &["ok", "losing"], ..Entry::row(TEMPERATURES, "bus", &[Kind::Word]) },
    Entry { unit: "askings", ..Entry::part(TEMPERATURES, "bus", "read", &[Kind::Number]) },
    Entry { unit: "askings", ..Entry::part(TEMPERATURES, "bus", "silent", &[Kind::Number]) },
    Entry { unit: "askings", ..Entry::part(TEMPERATURES, "bus", "broken", &[Kind::Number]) },
    Entry { unit: "askings", ..Entry::part(TEMPERATURES, "bus", "foreign", &[Kind::Number]) },
    // the silent askings of the last full window; none before the first window closes
    Entry { unit: "askings", ..Entry::part(TEMPERATURES, "bus", "last_silent", &[Kind::Number]) },
    // ---- the flue ----
    Entry {
        unit: "°C",
        reasons: &["open", "silent", "not_read"],
        curves: &["flue"],
        ..Entry::row(FLUE, "flue_temp", &[Kind::Number, Kind::Unknown])
    },
    // ---- the fan ----
    Entry { words: &["off", "hold", "full", "by_hand"], curves: &["fan_holding", "fan_regulator", "fan_response"], ..Entry::row(FAN, "holding", &[Kind::Word]) },
    Entry {
        words: &[
            "not_holding_yet",
            "gate_stopped",
            "gate_lost",
            "blind_under",
            "blind_missing",
            "blind_unconfigured",
            "blind_never_started",
            "by_hand_free",
            "alone",
            "off",
            "purging",
            "holding",
        ],
        ..Entry::part(FAN, "holding", "state", &[Kind::Word])
    },
    Entry { unit: "%", ..Entry::part(FAN, "holding", "angle", &[Kind::Number]) },
    Entry { unit: "Pa", reasons: &["under", "missing", "unconfigured", "not_read"], ..Entry::part(FAN, "holding", "came_out", &[Kind::Number, Kind::AtLeast, Kind::Unknown]) },
    Entry { unit: "Pa", ..Entry::part(FAN, "holding", "asked", &[Kind::Number]) },
    Entry { unit: "%", ..Entry::part(FAN, "holding", "refused_angle", &[Kind::Number]) },
    Entry::part(FAN, "holding", "gate_why", &[Kind::Text]),
    Entry::part(FAN, "holding", "stop_why", &[Kind::Text]),
    // what the knobs do, on the fan the board knows: the page says it in words under each knob
    Entry { unit: "Pa/%", ..Entry::part(FAN, "holding", "steepness", &[Kind::Number]) },
    Entry { unit: "s", reasons: &["not_back"], ..Entry::part(FAN, "holding", "answer_s", &[Kind::Number, Kind::Unknown]) },
    Entry { unit: "Pa", ..Entry::part(FAN, "holding", "answer_dip", &[Kind::Number]) },
    Entry { unit: "Pa", ..Entry::part(FAN, "holding", "answer_jump", &[Kind::Number]) },
    constant(FAN, "holding", "zero_mv", draught::ZERO_MV as f32, "mV"),
    constant(FAN, "holding", "missing_below_mv", draught::MISSING_BELOW_MV as f32, "mV"),
    constant(FAN, "holding", "kicks", regulator::KICKS as f32, "kicks"),
    constant(FAN, "holding", "kick_percent", regulator::KICK_PERCENT, "%"),
    constant(FAN, "holding", "full_pa", draught::FULL_PA as f32, "Pa"),
    Entry {
        unit: "Pa",
        words: &["full", "by_hand", "off"],
        can: &["hold"],
        range: Some(Range { min: 0, max: draught::FULL_PA, step: 1, at: "slider" }),
        ..Entry::row(FAN, "draught", &[Kind::Number, Kind::Word])
    },
    Entry { unit: "Pa", ..Entry::part(FAN, "draught", "slider", &[Kind::Number]) },
    // what the board comes up at when it is not what is asked now; none when the two are one
    Entry { unit: "Pa", ..Entry::part(FAN, "draught", "comes_up", &[Kind::Number]) },
    Entry { can: &["switch", "toggle"], ..Entry::row(FAN, "purge", &[Kind::On]) },
    Entry { unit: "/s", ..Entry::row(FAN, "zero_crossings", &[Kind::Number]) },
    Entry { words: &["ok", "bad"], ..Entry::part(FAN, "zero_crossings", "mains", &[Kind::Word]) },
    Entry {
        unit: "%",
        can: &["hold"],
        range: Some(Range { min: 0, max: 100, step: 1, at: "slider" }),
        ..Entry::row(FAN, "gate", &[Kind::Number])
    },
    Entry { unit: "%", ..Entry::part(FAN, "gate", "slider", &[Kind::Number]) },
    Entry { words: &["off", "wont_start", "not_firing", "firing"], ..Entry::part(FAN, "gate", "state", &[Kind::Word]) },
    Entry { words: &["first_second", "ok", "bad"], ..Entry::part(FAN, "gate", "judged", &[Kind::Word]) },
    Entry { unit: "/s", ..Entry::part(FAN, "gate", "pulses", &[Kind::Number]) },
    Entry { unit: "/s", ..Entry::part(FAN, "gate", "crossings", &[Kind::Number]) },
    constant(FAN, "gate", "starts_at", dimmer::STARTS_AT as f32, "%"),
    // ---- the draught ----
    Entry {
        unit: "Pa",
        reasons: &["not_read", "own_zero", "missing", "unconfigured"],
        ..Entry::row(DRAUGHT, "blast_pressure", &[Kind::Number, Kind::AtLeast, Kind::Unknown])
    },
    constant(DRAUGHT, "blast_pressure", "zero_mv", draught::ZERO_MV as f32, "mV"),
    constant(DRAUGHT, "blast_pressure", "missing_below_mv", draught::MISSING_BELOW_MV as f32, "mV"),
    constant(DRAUGHT, "blast_pressure", "full_pa", draught::FULL_PA as f32, "Pa"),
    Entry { curves: &["draught_fast", "draught_slow"], ..DRAUGHT_PIN[0] },
    DRAUGHT_PIN[1],
    DRAUGHT_PIN[2],
    DRAUGHT_PIN[3],
    DRAUGHT_PIN[4],
    DRAUGHT_PIN[5],
    DRAUGHT_PIN[6],
    // ---- the flame ----
    Entry { words: &["out", "weak", "steady"], reasons: &["not_read", "crossed"], ..Entry::row(FLAME, "flame", &[Kind::Word, Kind::Unknown]) },
    Entry { curves: &["flame_fast", "flame_slow"], ..FLAME_PIN[0] },
    FLAME_PIN[1],
    FLAME_PIN[2],
    FLAME_PIN[3],
    FLAME_PIN[4],
    FLAME_PIN[5],
    FLAME_PIN[6],
];

/// THE FIRST BURNER'S PAIR: the dose, the pause and the draught its fire was fed by in the modes by a temperature. The new burner has no pair - it burns on the chosen step of a table of settings in every hold
/// (`burner_settings::STEP_TABLE`) - so the three live here, with the first burner they belong to, and go with this crate at the switch.
pub const AUTO_PA: settings::Setting = settings::Setting {
    name: "auto_pa",
    label: "Draught of the step",
    default: "51",
    kind: settings::Kind::Whole { from: 0, to: draught::FULL_PA, unit: "Pa" },
    level: settings::Level::Tuning,
    card: Some("automaton"),
};
pub const AUTO_FEED_ON_S: settings::Setting = settings::Setting {
    name: "auto_feed_on_s",
    label: "At work the auger turns",
    default: "2",
    kind: settings::Kind::Tenths { from: 1, to: 1200, unit: "s" },
    level: settings::Level::Tuning,
    card: Some("automaton"),
};
pub const AUTO_FEED_OFF_S: settings::Setting = settings::Setting {
    name: "auto_feed_off_s",
    label: "At work the auger rests",
    default: "20",
    kind: settings::Kind::Tenths { from: 1, to: 6000, unit: "s" },
    level: settings::Level::Tuning,
    card: Some("automaton"),
};

/// The settings of the burner, in the order its page shows them: the board's own, then each module's in the order of
/// the modules. **THE ONE LIST**: a module's settings are the ones here whose card is its card.
pub const SETTINGS: [&settings::Setting; 49] = [
    &settings::TIME,
    &settings::ZONE,
    &burner_settings::AUTO_HOLD,
    &burner_settings::AUTO_HOLD_C,
    &burner_settings::AUTO_DROP_C,
    &burner_settings::AUTO_STEP,
    &burner_settings::AUTO_PURGE,
    &burner_settings::AUTO_IGNITER,
    &burner_settings::AUTO_WAIT,
    &burner_settings::AUTO_TRIES,
    &burner_settings::AUTO_KINDLE_PA,
    &AUTO_PA,
    &burner_settings::AUTO_DOSE,
    &burner_settings::AUTO_DOSE_AGAIN,
    &burner_settings::AUTO_RELIGHT_1,
    &burner_settings::AUTO_RELIGHT_2,
    &burner_settings::AUTO_RELIGHT_3,
    &burner_settings::AUTO_RISE_S,
    &burner_settings::AUTO_NO_FLAME_S,
    &burner_settings::AUTO_FLUE_FALL_S,
    &burner_settings::AUTO_BURN_OFF_S,
    &AUTO_FEED_ON_S,
    &AUTO_FEED_OFF_S,
    &burner_settings::AUTO_PUMP_ON_C,
    &burner_settings::AUTO_PUMP_OFF_C,
    &burner_settings::AUGER_LIMIT,
    &burner_settings::ROLE_NTC_1,
    &burner_settings::ROLE_NTC_2,
    &burner_settings::ROLE_NTC_3,
    &burner_settings::ROLE_NTC_4,
    &burner_settings::ROLE_NTC_5,
    &burner_settings::ROLE_NTC_6,
    &burner_settings::ROLE_NTC_7,
    &burner_settings::NTC_ADDRESS,
    &burner_settings::POLL_EVERY,
    &burner_settings::DRAUGHT_PA,
    &burner_settings::FAN_MIN,
    &burner_settings::FAN_MAX,
    &burner_settings::FAN_STEP,
    &burner_settings::UP_KP,
    &burner_settings::UP_KI,
    &burner_settings::DOWN_KP,
    &burner_settings::DOWN_KI,
    &burner_settings::FAN_BAND,
    &burner_settings::FAN_BAND_SHARE,
    &burner_settings::DIVIDER_TOP,
    &burner_settings::DIVIDER_BOTTOM,
    &burner_settings::FLAME_LIT,
    &burner_settings::FLAME_STEADY,
];

/// The roles of the sockets: what each measures, set on the tab Settings. They have no card - they are kept under the
/// board's own key, where the images before the dictionary kept them - and the temperatures keep them all the same:
/// a setting no module keeps reads as its default and is refused on save.
pub const SOCKET_ROLES: [&settings::Setting; 7] = [
    &burner_settings::ROLE_NTC_1,
    &burner_settings::ROLE_NTC_2,
    &burner_settings::ROLE_NTC_3,
    &burner_settings::ROLE_NTC_4,
    &burner_settings::ROLE_NTC_5,
    &burner_settings::ROLE_NTC_6,
    &burner_settings::ROLE_NTC_7,
];

/// The settings the module whose card is CARD keeps, in the order the page shows them: those of its card, and for the
/// temperatures the roles of the sockets besides.
pub fn settings_of(card: &str) -> Vec<&'static settings::Setting> {
    let roles = card == TEMPERATURES;
    // THE FIRST BURNER HAS NO THERMOSTAT: the mode, the setpoint, the hysteresis and the level stand on its automaton's card, as they did before the new burner moved them
    let heat = card == AUTOMATON;
    SETTINGS
        .iter()
        .copied()
        .filter(|s| s.card == Some(card) || (heat && s.card == Some("thermostat")) || (roles && SOCKET_ROLES.iter().any(|r| r.name == s.name)))
        .collect()
}

/// A row of the card of settings for each of LIST: what the setting keeps, as it is kept.
const fn setting_rows<const N: usize>(list: [&settings::Setting; N]) -> [Entry; N] {
    let mut out = [Entry::row("", "", &[]); N];
    let mut i = 0;
    while i < N {
        out[i] = Entry::row(page::SETTINGS, list[i].name, &[Kind::Text]);
        i += 1;
    }
    out
}

const SETTING_ROWS: [Entry; SETTINGS.len()] = setting_rows(SETTINGS);

/// What the settings that say something now say, by code; a setting whose words are made of its own value alone
/// (a limit in minutes) has none, the client works them out.
const NOTES: [Entry; 6] = [
    Entry { words: &["came_from", "came_from_at", "silent_time_kept", "no_answer"], ..Entry::part(page::SETTINGS, settings::TIME.name, "note", &[Kind::Word]) },
    Entry { words: &["gateway"], ..Entry::part(page::SETTINGS, settings::TIME.name, "from", &[Kind::Word, Kind::Text]) },
    Entry::part(page::SETTINGS, settings::TIME.name, "gateway", &[Kind::Text]),
    Entry::part(page::SETTINGS, settings::TIME.name, "at", &[Kind::Text]),
    // where the sensor's own zero lands on the pin through the divider now written in
    Entry { unit: "mV", reasons: &["not_a_divider"], ..Entry::part(page::SETTINGS, burner_settings::DIVIDER_BOTTOM.name, "zero_at", &[Kind::Number, Kind::Unknown]) },
    Entry { words: &["in_order", "crossed"], ..Entry::part(page::SETTINGS, burner_settings::FLAME_STEADY.name, "order", &[Kind::Word]) },
];

/// The burner's own rows, after the system card's: its cards, then its settings.
pub const OWN: [Entry; CARDED.len() + SETTING_ROWS.len() + NOTES.len()] =
    joined(&CARDED, &joined::<{ SETTING_ROWS.len() + NOTES.len() }>(&SETTING_ROWS, &NOTES));

/// The burner's table: the system card, then its own.
pub const ALL: [Entry; system::ROWS.len() + OWN.len()] = joined(&system::ROWS, &OWN);
/// The same, where the image keeps it once.
pub static TABLE: [Entry; ALL.len()] = ALL;

/// The burner's settings as a machine needs them: name, card, default, shape, the codes of the choices and what each bears on.
///
/// Made when the image is built, so the lists of choices are made once and kept for the rest of the build.
pub fn setting_shapes() -> Vec<page::dictionary::Setting> {
    use page::dictionary::{Needs, Setting, Shape, Variant};
    SETTINGS
        .iter()
        .map(|s| Setting {
            name: s.name,
            card: s.card,
            default: s.default,
            tuning: s.level == settings::Level::Tuning,
            effect: effect_of(s.name),
            shape: match &s.kind {
                settings::Kind::Choice(choices) => {
                    let variants: Vec<Variant> = choices
                        .iter()
                        .map(|c| {
                            let bears = burner_settings::bearing(s.name, c.value);
                            Variant {
                                value: c.value,
                                takes: c.takes.is_some(),
                                shows: bears.map_or(&[], |b| b.shows),
                                needs: bears.and_then(|b| b.role).map(|holds| Needs { among: &burner_settings::SOCKET_ROLES, holds }),
                            }
                        })
                        .collect();
                    Shape::Choice(Box::leak(variants.into_boxed_slice()))
                }
                settings::Kind::Months { from, to } => Shape::Months { from: *from, to: *to },
                settings::Kind::Seconds { from, to } => Shape::Seconds { from: *from, to: *to },
                settings::Kind::Tenths { from, to, unit } => Shape::Tenths { from: *from, to: *to, unit },
                settings::Kind::Whole { from, to, unit } => Shape::Whole { from: *from, to: *to, unit },
            },
        })
        .collect()
}

/// The automaton of the burner as a client draws it: its card, the rows that say the stage, what it does and where a fault
/// happened, the buttons pressed to run it, and the numbers it goes by in each stage - the order they stand in is the order a person
/// reads them. A number the image fixes stands by its name in [`FIXED_CARD`]; the rest are settings.
pub static MACHINE: page::dictionary::Machine = page::dictionary::Machine {
    card: AUTOMATON,
    fixed: FIXED_CARD,
    stage: "stage",
    says: "doing",
    cause: "broke_at",
    press: &["start", "stop"],
    aside: &["organs"],
    sections: &SECTIONS,
    now: &NOW,
};

/// The card Now, the burner's summary of what it is doing: the relays, the gate, the draught, the flame with the volts on its pin
/// beside its word, the two temperatures of the water - each the sensor that plays that role, whichever socket it is wired to - and the flue.
static NOW: [page::dictionary::Shown; 9] = {
    use page::dictionary::Shown;
    [
        Shown::row(RELAYS, "auger"),
        Shown::row(RELAYS, "igniter"),
        Shown::row(RELAYS, "pump"),
        Shown::row(FAN, "gate"),
        Shown::row(DRAUGHT, "blast_pressure"),
        Shown::row(FLAME, "flame").beside("pin"),
        Shown::found_by(TEMPERATURES, "role", "boiler"),
        Shown::found_by(TEMPERATURES, "role", "supply"),
        Shown::row(FLUE, "flue_temp"),
    ]
};

/// What each knob of the fan's regulator is worked out of, for the line a client says of what turning it does: the steepness the
/// regulator measured, the settings of its band and its travel, the answer it gave to a jump.
mod worked_out_of {
    use super::FAN;
    use page::dictionary::Use;

    pub const STEEP: Use = Use::Number(FAN, "holding", "steepness");
    pub const JUMP: Use = Use::Number(FAN, "holding", "answer_jump");
    pub const BACK_IN: Use = Use::Number(FAN, "holding", "answer_s");
    pub const DIP: Use = Use::Number(FAN, "holding", "answer_dip");
}

fn effect_of(setting: &str) -> Option<page::dictionary::Effect> {
    use page::dictionary::{Effect, Use};
    use worked_out_of::{BACK_IN, DIP, JUMP, STEEP};
    let e = |kind, uses: &'static [(&'static str, Use)]| Some(Effect { kind, uses });
    match setting {
        "up_kp" | "up_ki" | "down_kp" | "down_ki" => e("gain", &[("steep", STEEP)]),
        "fan_step" => e("ramp", &[("step", Use::Setting("fan_step")), ("min", Use::Setting("fan_min")), ("max", Use::Setting("fan_max"))]),
        "fan_band" => e("settle", &[("band", Use::Setting("fan_band")), ("jump", JUMP), ("s", BACK_IN), ("dip", DIP)]),
        "fan_band_share" => e(
            "share",
            &[
                ("steep", STEEP),
                ("band", Use::Setting("fan_band")),
                ("share", Use::Setting("fan_band_share")),
                ("up", Use::Setting("up_ki")),
                ("down", Use::Setting("down_ki")),
            ],
        ),
        _ => None,
    }
}

const fn section(name: &'static str, items: &'static [&'static [&'static str]]) -> page::dictionary::Section {
    page::dictionary::Section { name, by: "", items }
}

static SECTIONS: [page::dictionary::Section; 8] = [
    section("purge", &[&["auto_purge"]]),
    // THE PLUG BURNS ONLY WITH AIR, AND A LOST FLAME IN A HOT POT IS LIT FROM ITS EMBERS
    section(
        "lighting",
        &[
            &["auto_dose"],
            &["auto_igniter"],
            &["auto_kindle_pa"],
            &["auto_wait"],
            &["auto_tries"],
            &["auto_dose_again"],
            &["auto_no_flame_s"],
            &["lit_s"],
            &["auto_relight_1", "auto_relight_2", "auto_relight_3"],
            &["relight_hot_c"],
        ],
    ),
    // THE LIGHTING POURS ONE DOSE, RISING FEEDS NOTHING, THE WORK FEEDS BY ITS OWN PAIR
    section("rising", &[&["auto_rise_s"], &["auto_kindle_pa"], &["auto_pa"]]),
    // What the burner holds decides what the work is by: the mode's choices say which of these matter with them
    // (`burner_settings::BEARINGS`), the level of power its own number and a mode by a sensor the temperature and the fire's pair.
    page::dictionary::Section {
        name: "working",
        by: "auto_hold",
        items: &[&["auto_hold"], &["auto_step"], &["auto_pa"], &["auto_feed_on_s"], &["auto_feed_off_s"], &["auto_hold_c"], &["auto_drop_c"]],
    },
    section("burning_off", &[&["auto_pa"], &["burn_out_min_s"], &["auto_burn_off_s"], &["purge_after_s"]]),
    section("pump", &[&["auto_pump_on_c"], &["auto_pump_off_c"], &["too_hot_c"]]),
    // The flame's thresholds, in millivolts on its pin: settings of the flame's own card
    section("flame", &[&["flame_lit"], &["flame_steady"]]),
    section("gives_up", &[&["auto_tries"], &["flue_cold_c"], &["flue_fall_c"], &["auto_flue_win_s"], &["too_hot_c"], &["flue_holds_c"]]),
];

/// The dictionary the burner's image serves, the image being KIT: made when the image is built, from [`ALL`].
pub fn json(kit: &str) -> String {
    let settings = setting_shapes();
    page::dictionary::json(
        &ALL,
        &page::dictionary::Meta { device: "burner", board: "STRAGA_BASE_S3", kit, cards: &CARDS, settings: &settings, machine: Some(&MACHINE) },
    )
}

/// The number fixed in the image called NAME; NaN for one the table has not.
pub fn fixed_number(name: &str) -> f32 {
    TABLE.iter().find(|e| e.card == FIXED_CARD && e.name == name).and_then(|e| e.fixed).unwrap_or(f32::NAN)
}

/// The address of the clock the automaton calls WHAT; None for one the table has not.
pub fn clock_at(what: &str) -> Option<usize> {
    let (_, code) = CLOCKS.iter().find(|(said, _)| *said == what)?;
    page::dictionary::address(&TABLE, AUTOMATON, code)
}

/// The addresses a module writes its values by, worked out from [`ALL`] when the image is built.
pub mod at {
    use super::*;

    const fn row(card: &str, name: &str) -> usize {
        at(&ALL, card, name)
    }
    const fn part(card: &str, row: &str, name: &str) -> usize {
        part_at(&ALL, card, row, name)
    }

    pub const STAGE: usize = row(AUTOMATON, "stage");
    pub const DOING: usize = row(AUTOMATON, "doing");
    pub const DOING_ATTEMPT: usize = part(AUTOMATON, "doing", "attempt");
    pub const DOING_WHY: usize = part(AUTOMATON, "doing", "why");
    pub const DOING_TRIES: usize = part(AUTOMATON, "doing", "tries");
    pub const DOING_WATER: usize = part(AUTOMATON, "doing", "water");
    pub const BROKE_AT: usize = row(AUTOMATON, "broke_at");
    pub const SILENT_DRAUGHT: usize = row(AUTOMATON, "silent_draught");
    pub const SILENT_FLAME: usize = row(AUTOMATON, "silent_flame");
    pub const SILENT_FLUE: usize = row(AUTOMATON, "silent_flue");
    pub const SILENT_WATER: usize = row(AUTOMATON, "silent_water");
    pub const FOR: usize = row(AUTOMATON, "for");
    pub const AUGER: usize = row(AUTOMATON, "auger");
    pub const FEED: usize = row(AUTOMATON, "feed");
    pub const SEMI: usize = row(AUTOMATON, "semi");
    pub const IGNITER: usize = row(AUTOMATON, "igniter");
    pub const PUMP: usize = row(AUTOMATON, "pump");
    pub const PUMP_BY: usize = row(AUTOMATON, "pump_by");
    pub const PUMP_BY_WATER: usize = part(AUTOMATON, "pump_by", "water");
    pub const AUTOMATON_FAN: usize = row(AUTOMATON, "fan");
    pub const ALARM: usize = row(AUTOMATON, "alarm");
    pub const ALARM_PAST: usize = part(AUTOMATON, "alarm", "past");
    pub const ORGANS: usize = row(AUTOMATON, "organs");
    pub const REFUSED: usize = row(AUTOMATON, "refused");
    pub const START: usize = row(AUTOMATON, "start");
    pub const STOP: usize = row(AUTOMATON, "stop");

    pub const RELAY: [usize; 4] = [row(RELAYS, "auger"), row(RELAYS, "igniter"), row(RELAYS, "pump"), row(RELAYS, "aux_power")];
    pub const RELAY_OFF_IN: [Option<usize>; 4] = [Some(part(RELAYS, "auger", "off_in")), Some(part(RELAYS, "igniter", "off_in")), None, None];

    pub const CHANNEL: [usize; 7] = [
        row(TEMPERATURES, "ntc_1"),
        row(TEMPERATURES, "ntc_2"),
        row(TEMPERATURES, "ntc_3"),
        row(TEMPERATURES, "ntc_4"),
        row(TEMPERATURES, "ntc_5"),
        row(TEMPERATURES, "ntc_6"),
        row(TEMPERATURES, "ntc_7"),
    ];
    pub const CHANNEL_ROLE: [usize; 7] = [
        part(TEMPERATURES, "ntc_1", "role"),
        part(TEMPERATURES, "ntc_2", "role"),
        part(TEMPERATURES, "ntc_3", "role"),
        part(TEMPERATURES, "ntc_4", "role"),
        part(TEMPERATURES, "ntc_5", "role"),
        part(TEMPERATURES, "ntc_6", "role"),
        part(TEMPERATURES, "ntc_7", "role"),
    ];
    pub const NONE_ASSIGNED: usize = row(TEMPERATURES, "none_assigned");
    pub const BUS: usize = row(TEMPERATURES, "bus");
    pub const BUS_READ: usize = part(TEMPERATURES, "bus", "read");
    pub const BUS_SILENT: usize = part(TEMPERATURES, "bus", "silent");
    pub const BUS_BROKEN: usize = part(TEMPERATURES, "bus", "broken");
    pub const BUS_FOREIGN: usize = part(TEMPERATURES, "bus", "foreign");
    pub const BUS_LAST_SILENT: usize = part(TEMPERATURES, "bus", "last_silent");

    pub const FLUE_TEMP: usize = row(FLUE, "flue_temp");

    pub const HOLDING: usize = row(FAN, "holding");
    pub const HOLDING_STATE: usize = part(FAN, "holding", "state");
    pub const HOLDING_ANGLE: usize = part(FAN, "holding", "angle");
    pub const HOLDING_CAME_OUT: usize = part(FAN, "holding", "came_out");
    pub const HOLDING_ASKED: usize = part(FAN, "holding", "asked");
    pub const HOLDING_REFUSED_ANGLE: usize = part(FAN, "holding", "refused_angle");
    pub const HOLDING_GATE_WHY: usize = part(FAN, "holding", "gate_why");
    pub const HOLDING_STOP_WHY: usize = part(FAN, "holding", "stop_why");
    pub const HOLDING_STEEPNESS: usize = part(FAN, "holding", "steepness");
    pub const HOLDING_ANSWER_S: usize = part(FAN, "holding", "answer_s");
    pub const HOLDING_ANSWER_DIP: usize = part(FAN, "holding", "answer_dip");
    pub const HOLDING_ANSWER_JUMP: usize = part(FAN, "holding", "answer_jump");
    pub const FAN_DRAUGHT: usize = row(FAN, "draught");
    pub const FAN_DRAUGHT_SLIDER: usize = part(FAN, "draught", "slider");
    pub const FAN_DRAUGHT_COMES_UP: usize = part(FAN, "draught", "comes_up");
    pub const PURGE: usize = row(FAN, "purge");
    pub const ZERO_CROSSINGS: usize = row(FAN, "zero_crossings");
    pub const ZERO_CROSSINGS_MAINS: usize = part(FAN, "zero_crossings", "mains");
    pub const GATE: usize = row(FAN, "gate");
    pub const GATE_SLIDER: usize = part(FAN, "gate", "slider");
    pub const GATE_STATE: usize = part(FAN, "gate", "state");
    pub const GATE_JUDGED: usize = part(FAN, "gate", "judged");
    pub const GATE_PULSES: usize = part(FAN, "gate", "pulses");
    pub const GATE_CROSSINGS: usize = part(FAN, "gate", "crossings");

    pub const BLAST_PRESSURE: usize = row(DRAUGHT, "blast_pressure");
    /// The pin of CARD - the draught's or the flame's - and its parts, in the order `pin` lays them.
    pub const fn pin_of(card: &str) -> [usize; 4] {
        [row(card, "pin"), part(card, "pin", "spread"), part(card, "pin", "edge"), part(card, "pin", "window")]
    }
    pub const DRAUGHT_PIN: [usize; 4] = pin_of(DRAUGHT);
    pub const FLAME_ROW: usize = row(FLAME, "flame");
    pub const FLAME_PIN: [usize; 4] = pin_of(FLAME);

    pub const TIME: usize = row(page::SETTINGS, settings::TIME.name);
    pub const TIME_NOTE: usize = part(page::SETTINGS, settings::TIME.name, "note");
    pub const TIME_FROM: usize = part(page::SETTINGS, settings::TIME.name, "from");
    pub const TIME_GATEWAY: usize = part(page::SETTINGS, settings::TIME.name, "gateway");
    pub const TIME_AT: usize = part(page::SETTINGS, settings::TIME.name, "at");
    pub const DIVIDER_BOTTOM: usize = row(page::SETTINGS, burner_settings::DIVIDER_BOTTOM.name);
    pub const DIVIDER_ZERO_AT: usize = part(page::SETTINGS, burner_settings::DIVIDER_BOTTOM.name, "zero_at");
    pub const FLAME_STEADY: usize = row(page::SETTINGS, burner_settings::FLAME_STEADY.name);
    pub const FLAME_STEADY_ORDER: usize = part(page::SETTINGS, burner_settings::FLAME_STEADY.name, "order");
}
