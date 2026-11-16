//! API of the fan: its entities - what it is asked to hold and what the regulator made of it (the mode, the state by code, the angle, the draught that came out, why the gate stopped - by code), the two
//! sliders and the purge it takes commands on, the crossings of the mains and the pulses of the gate, the curves it has -, the regulator's settings with the numbers that say what turning each knob does,
//! and what the module hands the stream for a second. Names, kinds, units, limits, commands, codes of words and reasons; the address of each.
//! NEVER HERE: a card, an order, a label, a level, a phrase for a person, the limits of a picture: page/menu.json says them. Nor an interrupt, a pin or an angle being fired: that is src/hardware; what
//! angle holds the draught is crates/regulator and what a percent of air is, crates/dimmer, which this file only gives names to. A refusal of the gate is a code of REFUSALS, never its words.
//! CHECKS: tests/fan_table.rs, tests/fan_values.rs.

use ::draught::Reading as Pressure;
use ::regulator::knobs::{self, Consequences};
use ::regulator::{Asked, Blind, Carried, Held, Tuning};
use page::dictionary::{joined, Entry, Kind, Range, Value};

/// The name this module's rows go by, and the name of its entry in page/menu.json and in burner.yaml.
pub const CARD: &str = "fan";

/// The names of the rows a person moves the fan by: the purge button, the gate's slider and the draught's. The automaton keeps them while it runs the burner.
pub const PURGE_ROW: &str = "purge";
pub const GATE_ROW: &str = "gate";
pub const DRAUGHT_ROW: &str = "draught";
pub const BY_HAND: [&str; 3] = [PURGE_ROW, GATE_ROW, DRAUGHT_ROW];

/// The curves of the fan: what was asked and what came out by the angle, what the regulator did with the miss, and what it would do with a jump.
pub const CURVES: [&str; 3] = ["fan_holding", "fan_regulator", "fan_response"];

/// What a mains of 50 Hz gives: two crossings of zero in every cycle, a hundred a second. A second of crossings that is not a hundred, give or take one, is not a mains being followed.
pub const CROSSINGS_PER_SECOND: u32 = 100;

/// The settings of this module: the draught the board comes up at, how far the angle may travel and how fast, the two sets of gains and the band near the setpoint.
pub const SETTINGS: [&settings::Setting; 10] = [
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
];

/// The state of the holding, by code: what the fan is doing now. The order is the card's.
const STATES: [&str; 12] = [
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
];

/// Why the gate did not take an angle, by code. NOT THE WORDS: the words are the menu's, and a refusal that carries its numbers in prose is a string the board builds for every reader on every change.
pub const REFUSALS: [&str; 5] = ["mains_unknown", "mains_wrong", "over_scale", "no_gate", "gate_refused"];

/// Why the gate refused an angle, as the module knows it where it refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The mains has not been measured yet, and the detector does not give a mains' crossings either: nothing is fired over a phase nobody knows.
    MainsUnknown,
    /// The mains was measured and is no mains: the crossings are being missed or the edges bounce.
    MainsWrong,
    /// A percent of air over a hundred.
    OverScale,
    /// The gate was never built: see the log at start.
    NoGate,
    /// The peripheral refused: the gate would not be forced down, or the comparator would not take the moment.
    GateRefused,
}

impl Refusal {
    pub const fn code(self) -> &'static str {
        match self {
            Refusal::MainsUnknown => REFUSALS[0],
            Refusal::MainsWrong => REFUSALS[1],
            Refusal::OverScale => REFUSALS[2],
            Refusal::NoGate => REFUSALS[3],
            Refusal::GateRefused => REFUSALS[4],
        }
    }
}

/// Why the half cycle a percent is fired against is not to be had, from what the last second said of the mains: `Some(Err(words))` with words is a mains that was measured and is none, anything else - not measured yet, or a
/// detector that is not alive - is a mains nobody knows yet.
pub fn mains_refusal(measured: Option<&Result<u32, String>>) -> Refusal {
    match measured {
        Some(Err(words)) if !words.is_empty() => Refusal::MainsWrong,
        _ => Refusal::MainsUnknown,
    }
}

/// What became of the angle a round decided on, by code (`regulator::Carried` says it in words for the log).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gate {
    /// The gate took the angle - or nothing was commanded.
    Out,
    /// The gate refused the angle and the fan was put to zero instead.
    Stopped { angle: u32, why: Refusal },
    /// The gate refused the angle AND refused to stop: where the triac stands is not known.
    Lost { angle: u32, why: Refusal, stop: Refusal },
}

/// The gate's part in a round: what `regulator::carry` decided, with the refusals in the order the gate gave them - the angle's first, then, if it was asked, the stop's.
pub fn gate_of(carried: &Carried, refusals: &[Refusal]) -> Gate {
    match (carried, refusals) {
        (Carried::Out, _) => Gate::Out,
        (Carried::Stopped { angle, .. }, [why, ..]) => Gate::Stopped { angle: *angle, why: *why },
        (Carried::Lost { angle, .. }, [why, stop, ..]) => Gate::Lost { angle: *angle, why: *why, stop: *stop },
        // a gate that refused without saying why: the one reason that needs no word of its own
        (Carried::Stopped { angle, .. }, []) => Gate::Stopped { angle: *angle, why: Refusal::GateRefused },
        (Carried::Lost { angle, .. }, rest) => Gate::Lost { angle: *angle, why: *rest.first().unwrap_or(&Refusal::GateRefused), stop: Refusal::GateRefused },
    }
}

/// A number of the fan fixed in the image: part of ROW, never sent.
const fn constant(card: &'static str, row: &'static str, name: &'static str, value: f32, unit: &'static str) -> Entry {
    Entry { unit, fixed: Some(value), ..Entry::part(card, row, name, &[Kind::Number]) }
}

/// The rows of the fan: the holding and what it is made of, the draught and the purge a person moves it by, the crossings and the gate under them.
const ROWS: [Entry; 26] = [
    Entry { words: &["off", "hold", "full", "by_hand"], curves: &CURVES, ..Entry::row(CARD, "holding", &[Kind::Word]) },
    Entry { words: &STATES, ..Entry::part(CARD, "holding", "state", &[Kind::Word]) },
    Entry { unit: "%", decimals: Some(0), ..Entry::part(CARD, "holding", "angle", &[Kind::Number]) },
    Entry { unit: "Pa", decimals: Some(0), reasons: &["under", "missing", "unconfigured", "not_read"], ..Entry::part(CARD, "holding", "came_out", &[Kind::Number, Kind::AtLeast, Kind::Unknown]) },
    Entry { unit: "Pa", decimals: Some(0), ..Entry::part(CARD, "holding", "asked", &[Kind::Number]) },
    Entry { unit: "%", decimals: Some(0), ..Entry::part(CARD, "holding", "refused_angle", &[Kind::Number]) },
    Entry { words: &REFUSALS, ..Entry::part(CARD, "holding", "gate_why", &[Kind::Word]) },
    Entry { words: &REFUSALS, ..Entry::part(CARD, "holding", "stop_why", &[Kind::Word]) },
    constant(CARD, "holding", "zero_mv", ::draught::ZERO_MV as f32, "mV"),
    constant(CARD, "holding", "missing_below_mv", ::draught::MISSING_BELOW_MV as f32, "mV"),
    constant(CARD, "holding", "kicks", ::regulator::KICKS as f32, "kicks"),
    constant(CARD, "holding", "kick_percent", ::regulator::KICK_PERCENT, "%"),
    constant(CARD, "holding", "full_pa", ::draught::FULL_PA as f32, "Pa"),
    Entry {
        unit: "Pa", decimals: Some(0),
        words: &["full", "by_hand", "off"],
        can: &["hold"],
        range: Some(Range { min: 0, max: ::draught::FULL_PA, step: 1, at: "slider" }),
        ..Entry::row(CARD, DRAUGHT_ROW, &[Kind::Number, Kind::Word])
    },
    Entry { unit: "Pa", decimals: Some(0), ..Entry::part(CARD, DRAUGHT_ROW, "slider", &[Kind::Number]) },
    // what the board comes up at when it is not what is asked now; none when the two are one
    Entry { unit: "Pa", decimals: Some(0), ..Entry::part(CARD, DRAUGHT_ROW, "comes_up", &[Kind::Number]) },
    Entry { can: &["switch", "toggle"], ..Entry::row(CARD, PURGE_ROW, &[Kind::On]) },
    Entry { unit: "/s", decimals: Some(0), ..Entry::row(CARD, "zero_crossings", &[Kind::Number]) },
    Entry { words: &["ok", "bad"], ..Entry::part(CARD, "zero_crossings", "mains", &[Kind::Word]) },
    Entry { unit: "%", decimals: Some(0), can: &["hold"], range: Some(Range { min: 0, max: 100, step: 1, at: "slider" }), ..Entry::row(CARD, GATE_ROW, &[Kind::Number]) },
    Entry { unit: "%", decimals: Some(0), ..Entry::part(CARD, GATE_ROW, "slider", &[Kind::Number]) },
    Entry { words: &["off", "wont_start", "not_firing", "firing"], ..Entry::part(CARD, GATE_ROW, "state", &[Kind::Word]) },
    Entry { words: &["first_second", "ok", "bad"], ..Entry::part(CARD, GATE_ROW, "judged", &[Kind::Word]) },
    Entry { unit: "/s", decimals: Some(0), ..Entry::part(CARD, GATE_ROW, "pulses", &[Kind::Number]) },
    Entry { unit: "/s", decimals: Some(0), ..Entry::part(CARD, GATE_ROW, "crossings", &[Kind::Number]) },
    constant(CARD, GATE_ROW, "starts_at", ::dimmer::STARTS_AT as f32, "%"),
];

/// The settings as rows of the settings card: what each keeps, as it is kept - and under the knobs of the regulator the numbers that say what turning each does (crates/regulator, `knobs`): the page words them and works out none.
const SETTING_ROWS: [Entry; 29] = {
    use page::SETTINGS as S;
    const fn gain(name: &'static str) -> [Entry; 3] {
        [
            Entry::row(S, name, &[Kind::Text]),
            // what the gain moves the gate by, in percent of travel, for a miss of `miss`
            Entry { unit: "%", decimals: Some(1), ..Entry::part(S, name, "moves", &[Kind::Number]) },
            constant(S, name, "miss", knobs::MISS_PA, "Pa"),
        ]
    }
    let (up_kp, up_ki, down_kp, down_ki) = (gain("up_kp"), gain("up_ki"), gain("down_kp"), gain("down_ki"));
    [
        Entry::row(S, "draught_pa", &[Kind::Text]),
        Entry::row(S, "fan_min", &[Kind::Text]),
        Entry::row(S, "fan_max", &[Kind::Text]),
        Entry::row(S, "fan_step", &[Kind::Text]),
        // the whole travel of the gate, lowest to highest, at that step: where it starts, where it ends and how long it takes
        Entry { unit: "%", decimals: Some(0), ..Entry::part(S, "fan_step", "from", &[Kind::Number]) },
        Entry { unit: "%", decimals: Some(0), ..Entry::part(S, "fan_step", "to", &[Kind::Number]) },
        Entry { unit: "s", decimals: Some(0), reasons: &["no_step"], ..Entry::part(S, "fan_step", "takes", &[Kind::Number, Kind::Unknown]) },
        up_kp[0],
        up_kp[1],
        up_kp[2],
        up_ki[0],
        up_ki[1],
        up_ki[2],
        down_kp[0],
        down_kp[1],
        down_kp[2],
        down_ki[0],
        down_ki[1],
        down_ki[2],
        Entry::row(S, "fan_band", &[Kind::Text]),
        // what the regulator does with a jump of `jump`: the band it settles in, when it is back, how far under the setpoint the draught went
        Entry { unit: "Pa", decimals: Some(1), ..Entry::part(S, "fan_band", "band", &[Kind::Number]) },
        constant(S, "fan_band", "jump", knobs::JUMP_PA, "Pa"),
        Entry { unit: "s", decimals: Some(0), reasons: &["not_back"], ..Entry::part(S, "fan_band", "back_in", &[Kind::Number, Kind::Unknown]) },
        Entry { unit: "Pa", decimals: Some(1), ..Entry::part(S, "fan_band", "dip", &[Kind::Number]) },
        Entry::row(S, "fan_band_share", &[Kind::Text]),
        // what each way's gain comes to inside the band, for a miss of `near`
        Entry { unit: "Pa", decimals: Some(1), ..Entry::part(S, "fan_band_share", "band", &[Kind::Number]) },
        constant(S, "fan_band_share", "near", knobs::NEAR_PA, "Pa"),
        Entry { unit: "%", decimals: Some(0), ..Entry::part(S, "fan_band_share", "up", &[Kind::Number]) },
        Entry { unit: "%", decimals: Some(0), ..Entry::part(S, "fan_band_share", "down", &[Kind::Number]) },
    ]
};

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; 55] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &burner_settings::RULES)).collect()
}

/// The addresses of this module's values in the burner's table (`crate::TABLE`).
pub mod at {
    use page::dictionary::{at, part_at};
    const C: &str = super::CARD;
    pub const HOLDING: usize = at(&crate::ALL, C, "holding");
    pub const HOLDING_STATE: usize = part_at(&crate::ALL, C, "holding", "state");
    pub const HOLDING_ANGLE: usize = part_at(&crate::ALL, C, "holding", "angle");
    pub const HOLDING_CAME_OUT: usize = part_at(&crate::ALL, C, "holding", "came_out");
    pub const HOLDING_ASKED: usize = part_at(&crate::ALL, C, "holding", "asked");
    pub const HOLDING_REFUSED_ANGLE: usize = part_at(&crate::ALL, C, "holding", "refused_angle");
    pub const HOLDING_GATE_WHY: usize = part_at(&crate::ALL, C, "holding", "gate_why");
    pub const HOLDING_STOP_WHY: usize = part_at(&crate::ALL, C, "holding", "stop_why");
    pub const DRAUGHT: usize = at(&crate::ALL, C, super::DRAUGHT_ROW);
    pub const DRAUGHT_SLIDER: usize = part_at(&crate::ALL, C, super::DRAUGHT_ROW, "slider");
    pub const DRAUGHT_COMES_UP: usize = part_at(&crate::ALL, C, super::DRAUGHT_ROW, "comes_up");
    pub const PURGE: usize = at(&crate::ALL, C, super::PURGE_ROW);
    pub const ZERO_CROSSINGS: usize = at(&crate::ALL, C, "zero_crossings");
    pub const ZERO_CROSSINGS_MAINS: usize = part_at(&crate::ALL, C, "zero_crossings", "mains");
    pub const GATE: usize = at(&crate::ALL, C, super::GATE_ROW);
    pub const GATE_SLIDER: usize = part_at(&crate::ALL, C, super::GATE_ROW, "slider");
    pub const GATE_STATE: usize = part_at(&crate::ALL, C, super::GATE_ROW, "state");
    pub const GATE_JUDGED: usize = part_at(&crate::ALL, C, super::GATE_ROW, "judged");
    pub const GATE_PULSES: usize = part_at(&crate::ALL, C, super::GATE_ROW, "pulses");
    pub const GATE_CROSSINGS: usize = part_at(&crate::ALL, C, super::GATE_ROW, "crossings");

    /// What a gain moves the gate by, by setting: up_kp, up_ki, down_kp, down_ki.
    pub const GAINS_MOVE: [usize; 4] = [
        part_at(&crate::ALL, page::SETTINGS, "up_kp", "moves"),
        part_at(&crate::ALL, page::SETTINGS, "up_ki", "moves"),
        part_at(&crate::ALL, page::SETTINGS, "down_kp", "moves"),
        part_at(&crate::ALL, page::SETTINGS, "down_ki", "moves"),
    ];
    pub const STEP_FROM: usize = part_at(&crate::ALL, page::SETTINGS, "fan_step", "from");
    pub const STEP_TO: usize = part_at(&crate::ALL, page::SETTINGS, "fan_step", "to");
    pub const STEP_TAKES: usize = part_at(&crate::ALL, page::SETTINGS, "fan_step", "takes");
    pub const BAND_BAND: usize = part_at(&crate::ALL, page::SETTINGS, "fan_band", "band");
    pub const BAND_BACK_IN: usize = part_at(&crate::ALL, page::SETTINGS, "fan_band", "back_in");
    pub const BAND_DIP: usize = part_at(&crate::ALL, page::SETTINGS, "fan_band", "dip");
    pub const SHARE_BAND: usize = part_at(&crate::ALL, page::SETTINGS, "fan_band_share", "band");
    pub const SHARE_UP: usize = part_at(&crate::ALL, page::SETTINGS, "fan_band_share", "up");
    pub const SHARE_DOWN: usize = part_at(&crate::ALL, page::SETTINGS, "fan_band_share", "down");
}

/// What the gate counted in the last whole second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Second {
    pub crossings: u32,
    pub pulses: u32,
}

/// What the fan is now, as the module knows it: the API gives each of it its code and no more.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Now {
    /// What the fan is asked to do - by a person from the card or the console, or by the automaton.
    pub asked_to: Asked,
    /// What the last round decided and what became of it at the gate; None before the first round.
    pub said: Option<(Held, Gate)>,
    /// The draught of the last second, as the sensor says it; None when no module can say it now.
    pub draught: Option<Pressure>,
    /// The last second the gate counted; None before the first.
    pub second: Option<Second>,
    /// Whether that second is about the percent held now: straight after the slider moves it is not.
    pub counted_this: bool,
    /// The percent last set at the gate; None before anything was.
    pub percent: Option<u32>,
    /// The draught the setting holds, in pascals: where the board comes up and where the slider stands while the fan is off.
    pub seeded_pa: f32,
}

/// What the stream is told of the fan: the holding, the two sliders and the purge, the crossings and the gate, each by its address as `put` takes it.
pub fn put_values(now: &Now, put: &mut dyn FnMut(usize, Value)) {
    let Now { asked_to, said, draught, second, counted_this, percent, seeded_pa } = *now;
    let (crossings, pulses) = second.map_or((0, 0), |s| (s.crossings, s.pulses));
    // Zero percent is the gate held down ON PURPOSE, so no pulse is the right answer and counting them says nothing.
    let off = percent == Some(0);
    // Below the percent this fan breaks away on, a hundred pulses a second mean nothing to a fan standing still - but only while it is not turning: a draught is the one proof of turning this board has.
    let blowing = matches!(draught, Some(Pressure::Is(pa)) if pa > 0.0) || matches!(draught, Some(Pressure::Over));
    let wont_start = !blowing && matches!(percent, Some(p) if p > 0 && p < ::dimmer::STARTS_AT);
    // A hundred of each is a mains being followed, give or take one: a crossing can fall on either side of where the second is cut.
    let mains = crossings.abs_diff(CROSSINGS_PER_SECOND) <= 1;
    let firing = counted_this && pulses.abs_diff(crossings) <= 1;
    let gate = match percent {
        None => "off",
        Some(_) if off => "off",
        Some(_) if wont_start => "wont_start",
        Some(_) if counted_this && !firing => "not_firing",
        Some(_) => "firing",
    };
    // Nothing is held against a second counted before the angle moved: the count is about the old angle.
    let judged = percent.is_none() || off || !counted_this || firing;
    let judged = if percent.is_some() && !off && second.is_none() {
        "first_second"
    } else if mains && judged {
        "ok"
    } else {
        "bad"
    };

    put(
        at::HOLDING,
        Value::Word(match asked_to {
            Asked::Hold(_) => "hold",
            Asked::Purge => "full",
            Asked::ByHand => "by_hand",
            Asked::Stop => "off",
        }),
    );
    let (mut angle, mut came_out, mut held_pa, mut refused) = (Value::None, Value::None, Value::None, Value::None);
    let (mut gate_why, mut stop_why) = (Value::None, Value::None);
    // THE GATE COMES FIRST, whatever the regulator decided: a command that did not get through is the worst thing the row can say, and it must not be painted over by an opinion.
    let state = match said {
        None => "not_holding_yet",
        Some((_, Gate::Stopped { angle: a, why })) => {
            (refused, gate_why) = (Value::Number(a as f32), Value::Word(why.code()));
            "gate_stopped"
        }
        Some((_, Gate::Lost { angle: a, why, stop })) => {
            (refused, gate_why, stop_why) = (Value::Number(a as f32), Value::Word(why.code()), Value::Word(stop.code()));
            "gate_lost"
        }
        Some((Held::Blind(why), Gate::Out)) => match why {
            Blind::Under => "blind_under",
            Blind::Missing => "blind_missing",
            Blind::Unconfigured => "blind_unconfigured",
            Blind::NeverStarted => "blind_never_started",
        },
        Some((_, Gate::Out)) if asked_to == Asked::ByHand => "by_hand_free",
        Some((Held::Alone, Gate::Out)) => "alone",
        Some((Held::At(_), Gate::Out)) if asked_to == Asked::Stop => "off",
        Some((Held::At(a), Gate::Out)) => {
            angle = Value::Number(a);
            match asked_to {
                Asked::Hold(pa) => {
                    held_pa = Value::Number(pa);
                    came_out = match draught {
                        Some(Pressure::Is(pa)) => Value::Number(pa),
                        Some(Pressure::Over) => Value::AtLeast(::draught::FULL_PA as f32),
                        Some(Pressure::Under) => Value::Unknown("under"),
                        Some(Pressure::Missing) => Value::Unknown("missing"),
                        Some(Pressure::Unconfigured) => Value::Unknown("unconfigured"),
                        None => Value::Unknown("not_read"),
                    };
                    "holding"
                }
                _ => "purging",
            }
        }
    };
    put(at::HOLDING_STATE, Value::Word(state));
    put(at::HOLDING_ANGLE, angle);
    put(at::HOLDING_CAME_OUT, came_out);
    put(at::HOLDING_ASKED, held_pa);
    put(at::HOLDING_REFUSED_ANGLE, refused);
    put(at::HOLDING_GATE_WHY, gate_why);
    put(at::HOLDING_STOP_WHY, stop_why);

    // THE KNOB IN PASCALS, and nothing else a person turns for the burner's air. Zero is stop.
    let seeded = seeded_pa.round();
    put(
        at::DRAUGHT,
        match asked_to {
            Asked::Hold(pa) => Value::Number(pa),
            Asked::Purge => Value::Word("full"),
            Asked::ByHand => Value::Word("by_hand"),
            Asked::Stop => Value::Word("off"),
        },
    );
    // WHERE THE SLIDER STANDS WHEN THE FAN IS OFF IS THE KEPT SETPOINT, not zero: a person pushing it off the floor would otherwise start from one pascal and drag the whole way.
    put(
        at::DRAUGHT_SLIDER,
        Value::Number(match asked_to {
            Asked::Hold(pa) => pa.round(),
            _ => seeded,
        }),
    );
    put(
        at::DRAUGHT_COMES_UP,
        match asked_to {
            Asked::Hold(pa) if pa.round() != seeded => Value::Number(seeded),
            _ => Value::None,
        },
    );
    put(at::PURGE, Value::On(asked_to == Asked::Purge));
    put(at::ZERO_CROSSINGS, Value::Number(crossings as f32));
    put(at::ZERO_CROSSINGS_MAINS, Value::Word(if mains { "ok" } else { "bad" }));
    // THE ANGLE BY HAND, AND IT IS FOR PROBING: a person moving it takes the regulator off. The number the burner's air is set by is the draught, in pascals.
    let percent = percent.unwrap_or(0) as f32;
    put(at::GATE, Value::Number(percent));
    put(at::GATE_SLIDER, Value::Number(percent));
    put(at::GATE_STATE, Value::Word(gate));
    put(at::GATE_JUDGED, Value::Word(judged));
    put(at::GATE_PULSES, Value::Number(pulses as f32));
    put(at::GATE_CROSSINGS, Value::Number(crossings as f32));
}

/// What turning each knob of the regulator does, by the address of the setting it is a part of: how far each gain moves the gate, how long the travel takes, what the band does with a jump, what each way acts at
/// inside it. NONE where the fan has no point to be answered for - nothing then says what a made-up fan would do.
pub fn put_setting_values(tuning: &Tuning, point: (f32, f32), put: &mut dyn FnMut(usize, Value)) {
    let Some(Consequences { up_kp, up_ki, down_kp, down_ki, ramp_s, back_s, dip_pa, near_up, near_down }) = knobs::consequences(tuning, point.0, point.1) else {
        return;
    };
    for (at, moves) in at::GAINS_MOVE.into_iter().zip([up_kp, up_ki, down_kp, down_ki]) {
        put(at, Value::Number(moves));
    }
    put(at::STEP_FROM, Value::Number(tuning.min));
    put(at::STEP_TO, Value::Number(tuning.max));
    put(at::STEP_TAKES, ramp_s.map_or(Value::Unknown("no_step"), Value::Number));
    put(at::BAND_BAND, Value::Number(tuning.band_pa));
    put(at::BAND_BACK_IN, back_s.map_or(Value::Unknown("not_back"), |s| Value::Number(s as f32)));
    put(at::BAND_DIP, Value::Number(dip_pa));
    put(at::SHARE_BAND, Value::Number(tuning.band_pa));
    put(at::SHARE_UP, Value::Number(near_up));
    put(at::SHARE_DOWN, Value::Number(near_down));
}
