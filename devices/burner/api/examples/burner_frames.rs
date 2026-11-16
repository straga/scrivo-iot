//! The frames a stand plays for the new burner, one a second: what its API says of the modules laid through - the flame, the water's temperatures, the flue - for the readings a
//! scenario gives.
//!
//!     cargo run -q -p burner_api --example burner_frames SCENARIO > FRAMES
//!
//! SCENARIO is the file the first burner's scenarios are (`tools/stand_scenarios`): `seconds`, the readings as [[second, value or null]], straight lines between the points, null where
//! the sensor reads nothing - `flame_mv` the millivolts on the flame's pin, `boiler_c`, `return_c`, `room_c` and `flue_c` degrees -, and `settings` - {"name": "text"} - the roles of the
//! sockets (`role_ntc_1`...), which say whose reading each socket gives. What the first burner's scenarios do not have, and only this one reads: `supply_c`; `socket_c` - {"ntc_5": line} -
//! the line of a socket whose role has no line of its own; `empty_sockets` - ["ntc_6"] - sockets with nothing plugged in; `bus_silent`, `flue_open` and `flue_silent` - [[from, to]] -
//! the seconds the module of the bus does not answer, the thermocouple is open, and the chip says nothing; `draught_under` and `draught_gone` - [[from, to]] - the seconds the draught sensor answers under its own zero (with the automaton playing), and the seconds it does not answer as a sensor that is gone; `draught_mv` - [[second, millivolts or null]], each point holding until the next, null
//! where the pin reads nothing - the millivolts on the draught's pin, read through the divider of crates/settings' defaults or the scenario's `settings` (`div_top`, `div_bottom`).
//!
//! `soot` - {"soot_c": degrees or null, "burns": n, "grams": g, "cleaned_day": days since 1970 or null} - the four numbers of the card `soot`, the same in every second: the stand counts no burns, so the card says what the scenario holds.
//!
//! `clock` - {"start": "mon 21:50", "minutes_a_second": 1} - the wall time of the board the schedule follows its windows by: it is `start` at second 0 and goes on by `minutes_a_second` a second (an accelerated
//! day: 1440 seconds at one minute a second). A scenario without it is a board that does not know the time, and the card says so.
//!
//! The thermostat is told what the fire does as the board tells it: whether the automaton stood in Stop at the second before, and that each second stands for one - so `auto_lock_min` (the wait before a light) is counted here by the seconds of the scenario.
//!
//! `organs` - {"auger": [[second, "on" | "off" | "toggle"], ...], ...} - what a person or the automaton commands of each organ at each second: the commands go to the machine of crates/relay, run here on
//! a clock of the scenario's seconds, which closes the organs, counts their limits down and opens them when a limit runs out; the limits are the auger's own setting and the igniter's, the settings'
//! defaults or the scenario's `settings`.
//!
//! `fan` - [[second, "hold", pascals] | [second, "stop"] | [second, "purge"] | [second, "by_hand", percent]] - what a person asks of the fan at each second: the round of crates/regulator (`round`, the one the board runs) decides on the
//! draught the scenario's `draught_mv` makes through the divider, a gate that takes every angle carries it, and the mains is followed to the pulse; the fan's settings are the scenario's `settings` or the defaults. The
//! draught does NOT follow the fan here: it is the scenario's line, so the regulator's answer to a given draught is what is played, not a boiler's answer to the regulator.
//!
//! `restart` - [second, ...] - the seconds the board comes up again in the middle of the run: the fire is what `Burning::resumed` makes of the stage the board wrote down and of what the sensors say, the organs are open, and the thermostat comes up
//! with the switch the last press left in the flash - a press is written once, and `switch_writes` in the output counts the writes. Only with `press`: it is the automaton that restarts.
//!
//! `flash_refuses` - [[from, to]] - the seconds the flash of the stand refuses a write of the switch: the word waits and is tried again once a second until the flash takes it; a restart inside the range finds the word it held before.
//!
//! `press` - [[second, "start" | "stop"]] - and `feed` - [[second, true | false]] - the first burner's own keys: with `press` in the scenario the AUTOMATON plays (crates/automaton, on the numbers of crates/settings' `automaton_tuning`)
//! and commands the organs and the fan itself, and what a scenario says of `organs`, `fan` and `draught_mv` is not read: the draught is what the stand makes of what the automaton asked of the fan, as the first burner's stand does -
//! full air reads over the top of the scale, a setpoint is reached, no air is under the sensor's zero - and the sensors the automaton sees by are the scenario's lines through the board's own `module::Role`, so their silence is the board's code.
//!
//! The values are the modules' own mapping (`burner_api::flame::put_values`, `burner_api::temperatures::put_values`, `burner_api::flue::value`), on the thresholds of crates/settings' defaults;
//! the first burner's frames come from another code (crates/automaton_card, example `scenario`), which is what `tools/module_compare.py` holds them against. WHAT IS MADE UP HERE is
//! what a stand makes up: the sensors as the scenario reads them, and the counts of the bus, one asking a second.

use burner_api::flame;
use burner_api::{automaton as api_automaton, demand as api_demand, draught, fan, flue, relays, thermostat as api_thermostat};
use ::automaton::{Automaton, Burning, Doing, Heard, Seen, Silence, Water};
use ::schedule::Wall;
use ::thermostat::{Press, Temperatures, Thermostat};
use module::{Quiet, Role};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
// the critical section of std and the clock of std the machine of crates/relay waits by on the computer: only linked in when something names them
use critical_section as _;
use embassy_time as _;
use burner_api::temperatures;
use burner_map::{Readings, COUNT, UNREAD};
use ::flame::Thresholds;
use modbus::{Counts, Reading};
use page::dictionary::Said;
use serde_json::{json, Value};

/// The reading at T on a line of POINTS - [second, value] -, straight between them; None from the second a null point is passed until the next point.
/// The same reading of a scenario as the first burner's generator makes (crates/automaton_card, example `scenario`): what is compared is what each side says, not how each reads the file.
fn line_at(points: &Value, t: f32) -> Option<f32> {
    let points: Vec<(f32, Option<f32>)> =
        points.as_array()?.iter().map(|p| (p[0].as_f64().unwrap() as f32, p[1].as_f64().map(|v| v as f32))).collect();
    let (first, last) = (points.first()?, points.last()?);
    if t <= first.0 {
        return first.1;
    }
    if t >= last.0 {
        return last.1;
    }
    let w = points.windows(2).find(|w| t >= w[0].0 && t <= w[1].0)?;
    let (a, b) = (w[0].1?, w[1].1.unwrap_or(w[0].1?));
    Some(a + (b - a) * (t - w[0].0) / (w[1].0 - w[0].0))
}

/// The value at T of POINTS - [second, value or null] -, each holding until the next point: None where the last point passed is null or none has been passed.
fn step_at(points: &Value, t: u32) -> Option<f32> {
    let held = points.as_array()?.iter().filter(|p| p[0].as_u64().unwrap() <= u64::from(t)).last()?;
    held[1].as_f64().map(|v| v as f32)
}

/// The wall time of the board at the second T by the scenario's CLOCK - {"start": "mon 21:50", "minutes_a_second": N} -, none for a scenario that has none.
fn wall_at(clock: &Value, t: u32) -> Option<Wall> {
    const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
    let (day, time) = clock.get("start")?.as_str()?.split_once(' ')?;
    let (h, m) = time.split_once(':')?;
    let day = DAYS.iter().position(|d| *d == day).expect("a day of the week in the clock's start") as u32;
    let minutes = h.parse::<u32>().ok()? * 60 + m.parse::<u32>().ok()? + t * clock.get("minutes_a_second").and_then(Value::as_u64).unwrap_or(1) as u32;
    Some(Wall { day: ((day + minutes / 1440) % 7) as u8, minute: (minutes % 1440) as u16 })
}

/// The local time of the board at the second T by the scenario's CLOCK, as the pellets' count takes it: the days of the clock are the calendar's, from Monday the 5th of October 2026 - the clock knows
/// the day of the week and no date -, none for a scenario that has no clock.
fn now_at(clock: &Value, t: u32) -> Option<pellets::Now> {
    const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
    let (day, time) = clock.get("start")?.as_str()?.split_once(' ')?;
    let (h, m) = time.split_once(':')?;
    let day = DAYS.iter().position(|d| *d == day).expect("a day of the week in the clock's start") as u32;
    let minutes = day * 1440 + h.parse::<u32>().ok()? * 60 + m.parse::<u32>().ok()? + t * clock.get("minutes_a_second").and_then(Value::as_u64).unwrap_or(1) as u32;
    let date = pellets::Date { year: 2026, month: 10, day: (5 + minutes / 1440) as u8 };
    assert!(date.day <= 31, "the scenario's clock runs out of October");
    Some(pellets::Now { date, minute: (minutes % 1440) as u16, offset_min: 0 })
}

/// The organs on the computer: the machine of crates/relay and the clock of the scenario's seconds. An organ is commanded as a person would, in a thread of its own, while this thread is the owner.
struct Organs {
    relays: relay::Relays<4>,
    limits: [u32; 4],
}

impl Organs {
    fn up(limits: [u32; 4]) -> Self {
        let organs = Organs { relays: relay::Relays::new(), limits };
        organs.relays.start(&mut |_, _| {});
        for (n, limit) in limits.iter().enumerate() {
            organs.relays.set_limit(n, *limit);
        }
        organs
    }

    /// COMMAND on the organ N at the second T, and the limits that have run out at it opened.
    fn command(&self, n: usize, command: &str, t: u32) {
        let (now, limits) = (t * 1000, self.limits);
        std::thread::scope(|s| {
            let asker = s.spawn(|| match command {
                "on" => self.relays.set(n, true, std::time::Duration::from_secs(1)),
                "off" => self.relays.set(n, false, std::time::Duration::from_secs(1)),
                "toggle" => self.relays.toggle(n, std::time::Duration::from_secs(1)),
                other => panic!("no command {other} of an organ"),
            });
            while !asker.is_finished() {
                self.relays.serve_for(std::time::Duration::from_millis(10), &|| now, &|r| limits[r], &mut |_, _| {}, &|| {});
            }
            asker.join().unwrap().expect("the machine refused a command");
        });
    }

    fn second(&self, t: u32) -> [relays::Organ; 4] {
        let now = t * 1000;
        self.relays.expire(now, &mut |_, _| {});
        std::array::from_fn(|n| relays::Organ { closed: self.relays.closed(n), left_s: self.relays.left_ms(n, now).map(|ms| ms.div_ceil(1000)) })
    }
}

/// Whether T falls in one of the spans [[from, to]] of SPANS, the end included.
fn within(spans: &Value, t: u32) -> bool {
    spans.as_array().is_some_and(|all| all.iter().any(|s| s[0].as_u64().unwrap() <= u64::from(t) && u64::from(t) <= s[1].as_u64().unwrap()))
}

/// The fan on the computer: the regulator of crates/regulator run a second at a time on the draught the scenario gives, a gate that takes every angle, and a mains followed to the pulse.
struct FanSim {
    holding: Option<regulator::Holding>,
    asked_to: regulator::Asked,
    said: Option<(regulator::Held, fan::Gate)>,
    percent: Option<u32>,
}

impl FanSim {
    fn new() -> Self {
        FanSim { holding: None, asked_to: regulator::Asked::Stop, said: None, percent: None }
    }

    /// COMMAND at the second T - what a person asks of the fan.
    fn command(&mut self, command: &Value) {
        self.asked_to = match command[1].as_str().expect("a command of the fan") {
            "hold" => regulator::Asked::Hold(command[2].as_f64().expect("pascals") as f32),
            "stop" => regulator::Asked::Stop,
            "purge" => regulator::Asked::Purge,
            "by_hand" => {
                // by hand the regulator lets go, and the angle is the person's, set at once
                self.percent = Some(command[2].as_u64().expect("percent") as u32);
                regulator::Asked::ByHand
            }
            other => panic!("no command {other} of the fan"),
        };
    }

    /// One second: the regulator decides on READING - None before the sensor has said a second - and the gate carries it.
    fn second(&mut self, reading: Option<::draught::Reading>, tuning: &regulator::Tuning) {
        let Some(reading) = reading else { return };
        let mut holding = self.holding.take().unwrap_or_else(|| regulator::Holding::from(self.percent.unwrap_or(0) as f32));
        let mut set = self.percent;
        let round = regulator::round(&mut holding, self.asked_to, reading, 1.0, tuning, &mut |percent| {
            set = Some(percent);
            Ok(format!("{percent}%"))
        });
        self.percent = set;
        self.holding = Some(holding);
        self.said = Some((round.held, fan::gate_of(&round.carried, &[])));
    }

    fn knows(&self) -> (f32, f32) {
        regulator::knobs::known(self.holding.as_ref())
    }
}

/// How often the round of the temperatures publishes, in seconds: `poll_every` of the scenario, as the board takes it.
static ROUND_S: AtomicU64 = AtomicU64::new(2);

fn round() -> Duration {
    Duration::from_secs(ROUND_S.load(Ordering::Relaxed))
}

static BOILER: Role<f32> = Role::sensor(burner_settings::BOILER, round);
static RETURN: Role<f32> = Role::sensor(burner_settings::RETURN, round);
static ROOM: Role<f32> = Role::sensor(burner_settings::ROOM, round);

/// A role's silence as the automaton reads it (devices/burner, `src/modules/automaton.rs`).
fn heard(quiet: Quiet) -> Heard {
    match quiet {
        Quiet::Speaks => Heard::Speaks,
        Quiet::Silent(_) => Heard::Silent,
        Quiet::TooLong(_) => Heard::TooLong,
    }
}

/// The seconds a role has been silent, as the card of the automaton says them (devices/burner, `src/modules/automaton.rs`); None while it speaks.
fn silent_for(quiet: Quiet) -> Option<u32> {
    match quiet {
        Quiet::Speaks => None,
        Quiet::Silent(span) | Quiet::TooLong(span) => Some(span.as_secs() as u32),
    }
}

/// What the scenario's line KEY gives ROLE at second AT, and how the role is heard: published as the round would, read as the automaton would. A key with no line is not simulated - nothing to read, and never silent.
fn through(role: &Role<f32>, scenario: &Value, key: &str, at: u32) -> (Option<f32>, Quiet) {
    if scenario[key].is_null() {
        return (None, Quiet::Speaks);
    }
    let now = between::clock::Moment::default() + Duration::from_secs(u64::from(at));
    role.publish_at(line_at(&scenario[key], at as f32), now);
    (role.latest_at(now), role.quiet_at(now))
}

/// What the draught reads under the fan asked ASKED, as the stand makes it up: over the scale at full air, the setpoint once held, nothing under the sensor's zero with no air.
fn draught_of(asked: regulator::Asked) -> Option<f32> {
    match asked {
        regulator::Asked::Purge => Some(::draught::FULL_PA as f32),
        regulator::Asked::Hold(pa) => Some(pa),
        regulator::Asked::Stop | regulator::Asked::ByHand => None,
    }
}

/// The millivolts on the draught's pin that make what the stand says the draught is, through the divider: past the top of the scale, the pascals asked, under the sensor's own zero.
fn pin_of(asked: regulator::Asked, divider: (u32, u32)) -> f32 {
    let sensor_mv = match asked {
        regulator::Asked::Purge => ::draught::FULL_MV + 300,
        regulator::Asked::Hold(pa) => ::draught::ZERO_MV + (pa * (::draught::FULL_MV - ::draught::ZERO_MV) as f32 / ::draught::FULL_PA as f32).round() as u32,
        regulator::Asked::Stop | regulator::Asked::ByHand => ::draught::MISSING_BELOW_MV + 70,
    };
    ::draught::pin_mv(sensor_mv, divider.0, divider.1).expect("a divider the pin is made through") as f32
}

/// The automaton on the computer: the machine of crates/automaton second by second on what the scenario reads, the organs it commands, and the card it makes.
struct Auto {
    machine: Automaton,
    thermostat: Thermostat,
    /// The players of the demand, and the numbers they say by: the word of the house the thermostat orders on, and what the demand's card says of it.
    player: ::demand::Player,
    demand: ::demand::Tuning,
    verdict: Option<::demand::Verdict>,
    sound: ::automaton::Sound,
    heat: ::thermostat::Sound,
    /// What the water said in the last order, for the thermostat's card.
    water: Option<Water>,
    last: Option<Doing>,
    since: u32,
    broke_at: Option<&'static str>,
    asked: regulator::Asked,
    held: [bool; 3],
    /// What the board's flash holds of the switch: the word of the last press, none before any.
    kept: Option<&'static str>,
    /// How many times the flash was written for the switch: one for a press that changed the word, none for a second.
    writes: u32,
}

/// The flash of the stand: it takes WORD and counts the write, or - in a second of the scenario's `flash_refuses` - refuses it and holds what it held.
fn flash_takes(kept: &mut Option<&'static str>, writes: &mut u32, refused: bool, word: &str) -> Result<(), String> {
    if refused {
        return Err("the flash refuses the write".into());
    }
    *kept = ::thermostat::switch_kept(::thermostat::kept_switch(Some(word)));
    *writes += 1;
    Ok(())
}

fn json_of(said: Said<'_>) -> Value {
    match said {
        Said::Value(value) => serde_json::to_value(value).unwrap(),
        Said::Text(text) => json!({ "text": text }),
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: burner_frames SCENARIO > FRAMES");
    let scenario: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("the scenario")).expect("the scenario is JSON");
    let seconds = scenario["seconds"].as_u64().expect("seconds") as u32;
    let thresholds = Thresholds {
        lit_mv: burner_settings::FLAME_LIT.default.parse().unwrap(),
        steady_mv: burner_settings::FLAME_STEADY.default.parse().unwrap(),
    };
    // the role of each socket as the scenario sets it; a socket nobody gave one is free
    let roles: Vec<String> = temperatures::SETTINGS[..COUNT]
        .iter()
        .map(|s| scenario["settings"][s.name].as_str().unwrap_or(s.default).to_string())
        .collect();
    let roles: [&str; COUNT] = std::array::from_fn(|n| roles[n].as_str());
    let line_of = |n: usize| -> &Value {
        match roles[n] {
            "boiler" => &scenario["boiler_c"],
            "supply" => &scenario["supply_c"],
            "return" => &scenario["return_c"],
            "room" => &scenario["room_c"],
            _ => &Value::Null,
        }
    };
    let divider: (u32, u32) = {
        let arm = |setting: &settings::Setting| scenario["settings"][setting.name].as_str().unwrap_or(setting.default).parse().expect("an arm of the divider in ohms");
        (arm(&burner_settings::DIVIDER_TOP), arm(&burner_settings::DIVIDER_BOTTOM))
    };
    let setting = |setting: &settings::Setting| scenario["settings"][setting.name].as_str().unwrap_or(setting.default).to_string();
    let organs = Organs::up([
        setting(&burner_settings::AUGER_LIMIT).parse::<u32>().unwrap() * 1000,
        burner_settings::igniter_limit_s(&|s| setting(s)) * 1000,
        0,
        0,
    ]);
    let tuning = burner_settings::fan_tuning(&|s| setting(s));
    let seeded_pa: f32 = setting(&burner_settings::DRAUGHT_PA).parse().unwrap_or(0) as f32;
    // the automaton plays when the scenario has buttons for it
    ROUND_S.store(setting(&burner_settings::POLL_EVERY).parse().unwrap_or(2), Ordering::Relaxed);
    let presses: std::collections::HashMap<u32, Press> = scenario["press"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|p| {
                    let press = match p[1].as_str() {
                        Some("start") => Press::Start,
                        Some("stop") => Press::Stop,
                        Some("light") => Press::Light,
                        other => panic!("no button {other:?}"),
                    };
                    (p[0].as_u64().expect("the second of a press") as u32, press)
                })
                .collect()
        })
        .unwrap_or_default();
    let restarts: Vec<u32> = scenario["restart"].as_array().map(|all| all.iter().map(|s| s.as_u64().expect("the second of a restart") as u32).collect()).unwrap_or_default();
    let feeds: std::collections::HashMap<u32, bool> = scenario["feed"]
        .as_array()
        .map(|all| all.iter().map(|f| (f[0].as_u64().expect("the second of a feed") as u32, f[1].as_bool().expect("the feed is true or false"))).collect())
        .unwrap_or_default();
    let mut auto: Option<Auto> = scenario.get("press").map(|_| Auto {
        machine: Automaton::new(Burning::new()),
        thermostat: Thermostat::new(),
        player: ::demand::Player::new(),
        demand: burner_settings::demand_tuning(&|s| setting(s)),
        verdict: None,
        sound: burner_settings::automaton_tuning(&|s| setting(s)).sound().unwrap_or_else(|refused| panic!("the card's numbers are refused: {}", refused.words())),
        heat: burner_settings::thermostat_tuning(&|s| setting(s))
            .sound(burner_settings::automaton_tuning(&|s| setting(s)).too_hot_c)
            .unwrap_or_else(|refused| panic!("the card's numbers are refused: {}", refused.words())),
        last: None,
        water: None,
        since: 0,
        broke_at: None,
        asked: regulator::Asked::Stop,
        held: [false; 3],
        kept: None,
        writes: 0,
    });
    let (mut fan_sim, mut counts, mut frames) = (FanSim::new(), Counts::default(), Vec::new());
    // the pellets burned: the board's own count, on the seconds the auger of this scenario turns and the rate its settings give
    let mut burned = pellets::Pellets::default();
    let pellet_rate: u32 = setting(&burner_settings::PELLET_RATE).parse().expect("the rate of the pellets in grams a minute");
    for at_second in 0..seconds {
        let t = at_second as f32;
        let mut values: Vec<Value> = Vec::new();
        // the automaton's second: what it sees by the scenario's lines and the draught the stand makes of what it asked last, one step, and what it wants of the organs and the fan
        let mut wants: Option<(Doing, [bool; 3])> = None;
        // what the thermostat's card says of the silence of the return and the air this second: a role no socket is set to is not silent, it is not there
        let mut card_silent = api_thermostat::Silent::default();
        let mut silent_room = None;
        let mut pin_asked: Option<regulator::Asked> = None;
        if let Some(a) = auto.as_mut() {
            let (boiler_c, water) = through(&BOILER, &scenario, "boiler_c", at_second);
            let (return_c, returns) = through(&RETURN, &scenario, "return_c", at_second);
            let (room_c, room) = through(&ROOM, &scenario, "room_c", at_second);
            let blowing = draught_of(a.asked);
            let seen = Seen {
                flame: line_at(&scenario["flame_mv"], t).map_or(::flame::Flame::Out, |mv| ::flame::read(mv.round() as u32, thresholds)),
                boiler_c,
                flue_c: line_at(&scenario["flue_c"], t),
                // the sensor under its zero or gone: the stand's draught follows the fan except in the spans the scenario names
                draught_pa: if within(&scenario["draught_under"], at_second) || within(&scenario["draught_gone"], at_second) { None } else { blowing },
                draught_low: within(&scenario["draught_under"], at_second),
                // the stand's gate is open exactly while the fan is asked to blow
                gate: if blowing.is_some_and(|pa| pa > 0.0) { 30 } else { 0 },
                silence: Silence { water: heard(water), ..Silence::HEARD },
            };
            // THE BOARD COMES UP AGAIN: what it remembers is what it writes down - the stage of the fire, and the switch by the word of the last press -, the organs are open and the fan asked for nothing, and the fire is
            // `Burning::resumed` over what the sensors say now; a thermostat that finds a word in the flash comes up with that switch and no press of its own
            if restarts.contains(&at_second) {
                let was = ::automaton::recalled(&::automaton::remembered(a.machine.state())).unwrap_or(::automaton::State::Stop);
                for n in 0..3 {
                    if a.held[n] {
                        organs.command(n, "off", at_second);
                    }
                }
                a.held = [false; 3];
                a.machine = Automaton::new(Burning::resumed(was, &seen, &a.sound));
                a.thermostat = Thermostat::restored(::thermostat::kept_switch(a.kept));
                a.player = ::demand::Player::new();
                a.water = None;
                a.last = None;
                a.since = at_second;
                a.broke_at = None;
                a.asked = regulator::Asked::Stop;
            }
            // THE HOUSE SPEAKS BEFORE THE THERMOSTAT ORDERS, as on the board: the room by its role and the clock of the scenario, through the players the card's source picks
            let verdict = a.player.ask(&a.demand, room_c, wall_at(&scenario["clock"], at_second));
            a.verdict = Some(verdict);
            let readings = Temperatures { boiler_c, return_c, return_heard: heard(returns), asked: verdict.asked, room_c, room_target_c: verdict.target_c, fire: ::thermostat::Fire { out: a.machine.state() == ::automaton::State::Stop, steady: seen.flame == ::flame::Flame::Steady, elapsed_s: if at_second == 0 { 0.0 } else { 1.0 } } };
            // the role is played by the sockets set to it - the rule of the board (`burner_settings::plays`) - and not by a line the scenario happens to have
            card_silent = api_thermostat::Silent { return_c: api_thermostat::Silent::said(silent_for(returns), burner_settings::plays(burner_settings::RETURN, &setting)) };
            silent_room = api_thermostat::Silent::said(silent_for(room), burner_settings::plays(burner_settings::ROOM, &setting));
            // a word the flash refused is tried again once a second until it is taken, as the board does (`Thermostat::retry`)
            a.thermostat.retry(1.0, &mut |word| flash_takes(&mut a.kept, &mut a.writes, within(&scenario["flash_refuses"], at_second), word));
            if let Some(allowed) = feeds.get(&at_second) {
                a.machine.feed(*allowed, &a.thermostat.order(&readings, &a.heat), seen, &a.sound);
            }
            a.thermostat.faulted(matches!(a.machine.state(), ::automaton::State::Fault(_)));
            let doing = match presses.get(&at_second) {
                Some(press) => {
                    // a Light is judged against the readings of this second, as the board's: a refused one is not counted and the fire is not asked
                    match press {
                        Press::Light => {
                            let _ = a.thermostat.light(&readings, &a.heat);
                        }
                        other => {
                            let _ = a.thermostat.press(*other);
                        }
                    }
                    let refused = within(&scenario["flash_refuses"], at_second);
                    a.thermostat.write_now(&mut |word| flash_takes(&mut a.kept, &mut a.writes, refused, word));
                    let order = a.thermostat.order(&readings, &a.heat);
                    a.water = Some(order.water);
                    a.machine.heat(&order, seen, &a.sound)
                }
                None => {
                    let order = a.thermostat.order(&readings, &a.heat);
                    a.water = Some(order.water);
                    a.machine.second(&order, seen, 1.0, &a.sound)
                }
            };
            if a.last.map(|d| d.state) != Some(doing.state) {
                a.broke_at = ::automaton::broken_at(a.broke_at, a.last.map(|d| d.state), doing.state);
                a.since = at_second;
            }
            a.last = Some(doing);
            pin_asked = Some(a.asked);
            a.asked = doing.fan;
            wants = Some((doing, [doing.auger, doing.igniter, doing.pump]));
        }
        // the flame
        let reading = line_at(&scenario["flame_mv"], t)
            .map(|mv| flame::Reading { window: analog::window(&[mv.round() as u16; flame::SAMPLES]).unwrap(), at_ceiling: false });
        flame::put_values(reading, thresholds, &mut |name, value| values.push(json!([flame::address(name), serde_json::to_value(value).unwrap()])));
        values.push(json!([flame::at::STEADY_ORDER, serde_json::to_value(flame::order_value(thresholds)).unwrap()]));
        // the water: the bus asked once a second, answered unless the scenario has the module silent
        let silent = within(&scenario["bus_silent"], at_second);
        counts.count(if silent { modbus::Outcome::Missed } else { modbus::Outcome::Accepted });
        let mut read: Readings = UNREAD;
        if !silent {
            for n in 0..COUNT {
                let name = format!("ntc_{}", n + 1);
                let empty = scenario["empty_sockets"].as_array().is_some_and(|all| all.iter().any(|s| s == name.as_str()));
                let line = if line_of(n).is_null() { &scenario["socket_c"][name.as_str()] } else { line_of(n) };
                read[n] = match (empty, line_at(line, t)) {
                    (true, _) => Reading::Empty,
                    (false, Some(celsius)) => Reading::Is(celsius),
                    (false, None) => Reading::Unknown,
                };
            }
        }
        temperatures::put_values(&read, &roles, &counts, &mut |a, said| values.push(json!([a, json_of(said)])));
        // the settings of the roles, as the board keeps them: the scenario's
        for (n, role) in roles.iter().enumerate() {
            let setting = page::dictionary::address(&burner_api::ALL, page::SETTINGS, temperatures::SETTINGS[n].name).expect("a role setting in the table");
            values.push(json!([setting, { "text": role }]));
        }
        // the organs: the commands of this second carried out, the limits that have run out opened
        if let (Some(a), Some((_, want))) = (auto.as_mut(), wants.as_ref()) {
            // the automaton's hand on the organs: a command where what it wants changes
            for n in 0..3 {
                if want[n] != a.held[n] {
                    organs.command(n, if want[n] { "on" } else { "off" }, at_second);
                    a.held[n] = want[n];
                }
            }
        } else {
            for (n, name) in relays::NAMES.iter().enumerate() {
                for command in scenario["organs"][*name].as_array().into_iter().flatten().filter(|c| c[0].as_u64() == Some(u64::from(at_second))) {
                    organs.command(n, command[1].as_str().expect("a command"), at_second);
                }
            }
        }
        let organ_states = organs.second(at_second);
        relays::put_values(&organ_states, &mut |a, value| values.push(json!([a, serde_json::to_value(value).unwrap()])));
        // the pellets: counted from the auger's relay in the second, placed in its day by the clock, said by the API's mapping
        burned.observe(u64::from(at_second) * 1000, organ_states[0].closed, pellet_rate);
        burned.at(now_at(&scenario["clock"], at_second));
        burner_api::pellets::put_values(&burned.kilograms().map(|(_, kg)| kg), burned.yesterday_kg(), true, &mut |a, value| values.push(json!([a, serde_json::to_value(value).unwrap()])));
        // the soot: the numbers the scenario holds - a stand has no burns of its own to count - said by the API's mapping
        if let Some(soot) = scenario.get("soot") {
            burner_api::soot::put_values(
                soot["soot_c"].as_f64().map(|c| c as f32),
                soot["burns"].as_u64().unwrap_or(0) as usize,
                soot["grams"].as_u64().unwrap_or(0),
                soot["cleaned_day"].as_i64(),
                &mut |a, value| values.push(json!([a, serde_json::to_value(value).unwrap()])),
            );
        }
        // the draught: the pin behind its divider
        let pin_mv = match pin_asked {
            Some(asked) => Some(pin_of(asked, divider)),
            None => step_at(&scenario["draught_mv"], at_second),
        };
        let pin = pin_mv
            .map(|mv| burner_api::pin::Reading { window: analog::window(&[mv.round() as u16; burner_api::pin::SAMPLES]).unwrap(), at_ceiling: false });
        draught::put_values(pin, divider, &mut |a, value| values.push(json!([a, serde_json::to_value(value).unwrap()])));
        values.push(json!([draught::at::ZERO_AT, serde_json::to_value(draught::zero_at(divider)).unwrap()]));
        // the fan: what a person asked of it this second, the regulator's round on the draught the divider makes of the scenario's pin, the gate's second
        match wants.as_ref() {
            // the automaton asks for what it wants of the fan, as a person's command would
            Some((doing, _)) => fan_sim.asked_to = doing.fan,
            None => {
                for command in scenario["fan"].as_array().into_iter().flatten().filter(|c| c[0].as_u64() == Some(u64::from(at_second))) {
                    fan_sim.command(command);
                }
            }
        }
        let said_second = pin_mv.map(|mv| ::draught::read(mv.round() as u32, divider.0, divider.1));
        fan_sim.second(said_second, &tuning);
        let percent = fan_sim.percent;
        fan::put_values(
            &fan::Now {
                asked_to: fan_sim.asked_to,
                said: fan_sim.said,
                draught: said_second,
                // the first second after a start is not counted yet; after it the mains is followed to the pulse, and a gate held down fires none
                second: (at_second > 0).then_some(fan::Second { crossings: 100, pulses: if percent.is_some_and(|p| p > 0) { 100 } else { 0 } }),
                counted_this: at_second > 0,
                percent,
                seeded_pa,
            },
            &mut |a, value| values.push(json!([a, serde_json::to_value(value).unwrap()])),
        );
        fan::put_setting_values(&tuning, fan_sim.knows(), &mut |a, value| values.push(json!([a, serde_json::to_value(value).unwrap()])));
        // the automaton's card, by the mapping of the API, as the board's module makes it
        if let (Some(a), Some((doing, _))) = (auto.as_ref(), wants.as_ref()) {
            let clocks = a.machine.clocks(&a.sound);
            api_automaton::put_values(
                &api_automaton::Now {
                    doing: Some(*doing),
                    lasted_s: at_second - a.since,
                    refused: None,
                    clocks: &clocks,
                    broke_at: a.broke_at,
                    feed_allowed: a.machine.feed_allowed(),
                    semi: a.machine.semi(),
                    silent: api_automaton::Silent::default(),
                    relief: api_automaton::ReliefNow::default(),
                    alarm_c: setting(&burner_settings::AUTO_ALARM_C).parse().unwrap_or(f32::NAN),
                },
                &mut |address, said| values.push(json!([address, json_of(said)])),
            );
        }
        // the thermostat's card, by the mapping of the API, as the board's module makes it: whether a person asked, and what the water said in the second's order
        if let Some(a) = auto.as_ref() {
            api_thermostat::put_values(
                &api_thermostat::Now {
                    switch: a.thermostat.switch(),
                    from_memory: a.thermostat.from_memory(),
                    water: a.water,
                    rung: a.thermostat.rung(),
                    left_c: a.thermostat.left_c(),
                    setpoint_c: a.thermostat.setpoint_c(),
                    lock_left_s: a.thermostat.lock_left_s(),
                    silent: card_silent,
                    manual: a.thermostat.cycle(),
                }, &mut |address, value| values.push(json!([address, serde_json::to_value(value).unwrap()])),
            );
        }
        // the demand's card, by the mapping of the API, as the board's module makes it: what the house said and why, the profile that stands, the target of the room
        if let Some(verdict) = auto.as_ref().and_then(|a| a.verdict) {
            let said = verdict.asked.is_some();
            api_demand::put_values(
                &api_demand::Now {
                    asked: verdict.asked,
                    silent_room,
                    profile: said.then_some(verdict.profile),
                    source: said.then_some(verdict.source),
                    until_min: verdict.until.filter(|_| said),
                    target_c: verdict.target_c,
                },
                &mut |address, value| values.push(json!([address, serde_json::to_value(value).unwrap()])),
            );
        }
        // the flue
        let chip = if within(&scenario["flue_open"], at_second) {
            Some(max6675::Reading::Open)
        } else if within(&scenario["flue_silent"], at_second) {
            Some(max6675::Reading::Silent)
        } else {
            line_at(&scenario["flue_c"], t).map(max6675::Reading::Is)
        };
        values.push(json!([flue::at::FLUE_TEMP, serde_json::to_value(flue::value(chip)).unwrap()]));
        frames.push(json!({ "at": at_second, "values": values }));
    }
    println!("{}", json!({ "frames": frames, "switch_writes": auto.as_ref().map(|a| a.writes) }));
}
