//! The page's stand with the automaton in it: runs the burner's own automaton second by second on the readings a
//! scenario gives, and writes what the board would send each second - the automaton's card made by the board's code
//! (this crate), the relays as the automaton drives them, the sensors as the scenario reads them - as values by the
//! addresses of the burner's dictionary (contract 3).
//!
//!     cargo run -p automaton_card --example scenario -- STATE SCENARIO > FRAMES
//!
//! STATE is a /state of contract 3 (tools/page_stand_values.json): its automaton settings are the numbers the automaton
//! runs on here, with the numbers fixed in the image, so the stand runs on the board's card and not on a copy of it.
//! SCENARIO is JSON: `seconds`, `press` - [[second, "start"|"stop"], ...], `feed` - [[second, true|false], ...],
//! the feed allowed or banned from that second - and the readings as [[second, value], ...]: `flame_mv` the millivolts
//! on the flame's pin - its word is read off them by the board's own `flame::read`, on the thresholds of the state -,
//! `boiler_c`, `return_c`, `room_c` and `flue_c` degrees, straight lines between the points; a null reads nothing until the
//! next point, as a sensor with its wire off - or a socket let go of its role. `settings` - {"name": "text"} - are settings
//! of the automaton's card the scenario runs on over the state's own (what the burner holds).
//!
//! **THE THREE TEMPERATURES THE AUTOMATON HOLDS BY GO THROUGH THE BOARD'S OWN `module::Role`**, published each second the
//! scenario has a line for them: their silence - three periods, ten, at least 3 s and 30 s - is the board's code answering
//! at the scenario's moment, and a reading that has gone silent is None as it is on the board. A temperature the scenario has
//! no line for is not simulated: it reads nothing and is never silent, as before. The draught follows the fan: full air reads over the top of the scale, a setpoint is
//! reached, no air is under the sensor's zero. FRAMES is `{"frames": [{"at": second, "values": [[address, value],
//! ...]}, ...]}`, what tools/page_stand.py plays.
//!
//! WHAT IS MADE UP HERE AND WHAT IS NOT: the automaton, its clocks and its card are the burner's; the relays, the
//! sensors, the fan's angle and the draught's answer to the fan are the stand's, by the board's addresses.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use automaton::{broken_at, Silence, Asked, Automaton, Burning, Doing, Heard, Seen, Tuning};
use thermostat::{Press, Temperatures, Thermostat};
use between::clock::Moment;
use module::{Quiet, Role};
use automaton_card::{values, Card, Silent};
use burner_api::first_burner::{at, TABLE};
use page::dictionary::Value as V;
use flame::{Flame, Thresholds};
use serde_json::{json, Value};

/// How often the round of the temperatures publishes, in seconds: `poll_every` of the scenario's card, as the board takes it.
static ROUND_S: AtomicU64 = AtomicU64::new(2);

fn round() -> Duration {
    Duration::from_secs(ROUND_S.load(Ordering::Relaxed))
}

static BOILER: Role<f32> = Role::sensor(burner_settings::BOILER, round);
static RETURN: Role<f32> = Role::sensor(burner_settings::RETURN, round);
static ROOM: Role<f32> = Role::sensor(burner_settings::ROOM, round);

/// A role's silence as the automaton reads it (devices/burner, `heard`).
fn heard(quiet: Quiet) -> Heard {
    match quiet {
        Quiet::Speaks => Heard::Speaks,
        Quiet::Silent(_) => Heard::Silent,
        Quiet::TooLong(_) => Heard::TooLong,
    }
}

/// What the scenario's line KEY gives ROLE at second AT, and how the role is heard: published as the round would, read
/// as the automaton would. A key with no line is not simulated - nothing to read, and never silent.
fn through(role: &Role<f32>, scenario: &Value, key: &str, at: u32) -> (Option<f32>, Heard) {
    if scenario[key].is_null() {
        return (None, Heard::Speaks);
    }
    let now = Moment::default() + Duration::from_secs(u64::from(at));
    role.publish_at(line_at(&scenario[key], at as f32), now);
    (role.latest_at(now), heard(role.quiet_at(now)))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [state, scenario] = args.as_slice() else {
        eprintln!("usage: scenario STATE SCENARIO > FRAMES");
        std::process::exit(2);
    };
    let state: Value = serde_json::from_str(&std::fs::read_to_string(state).expect("the state")).expect("the state is JSON");
    let scenario: Value = serde_json::from_str(&std::fs::read_to_string(scenario).expect("the scenario")).expect("the scenario is JSON");
    // a setting as the state keeps it; one the state was taken before is seeded with its default, as the board does
    let kept = |name: &str| -> Option<String> {
        // a setting the scenario names is the scenario's: the burner runs on it whatever the state kept
        if let Some(text) = scenario["settings"][name].as_str() {
            return Some(text.to_string());
        }
        let address = page::dictionary::address(&TABLE, page::SETTINGS, name)?;
        state["values"].as_array()?.iter().find(|r| r[0] == address)?[1]["text"].as_str().map(str::to_string)
    };
    let numbers: HashMap<String, String> = burner_api::first_burner::settings_of("automaton")
        .iter()
        .map(|s| (s.name.to_string(), kept(s.name).unwrap_or_else(|| s.default.into())))
        .collect();
    let fire = tuning(&numbers);
    let sound = fire.sound().unwrap_or_else(|refused| panic!("the card's numbers are refused: {}", refused.words()));
    let heat = heat_tuning(&numbers).sound(fire.too_hot_c).unwrap_or_else(|refused| panic!("the card's numbers are refused: {}", refused.words()));
    let alarm_c = burner_api::first_burner::fixed_number("alarm_c");
    let limit_s = |name: &str| -> Option<u32> { kept(name)?.parse().ok() };
    // the relays' own limits, from the card of the relays: the seconds they leave are what the board sends
    // the igniter's is no setting: it follows from the kindling, as the board's relays take it (burner_settings::igniter_limit_s)
    let igniter = burner_settings::igniter_limit_s(&|s: &settings::Setting| kept(s.name).unwrap_or_else(|| s.default.into()));
    let limits = [limit_s("auger_limit"), Some(igniter), None];
    // the flame's thresholds, from the card of the flame, as the board keeps them
    let thresholds = Thresholds {
        lit_mv: limit_s("flame_lit").unwrap_or_else(|| burner_settings::FLAME_LIT.default.parse().unwrap()),
        steady_mv: limit_s("flame_steady").unwrap_or_else(|| burner_settings::FLAME_STEADY.default.parse().unwrap()),
    };
    // which socket of the bus measures the boiler: its reading is the scenario's
    let socket = |role: &str| (1..=7).find(|n| kept(&format!("role_ntc_{n}")).as_deref() == Some(role)).map(|n| n - 1);
    // the return and the room are drawn only when the scenario has a line for them: without one the state's own reading stands
    let sockets = [
        socket(burner_settings::BOILER),
        socket(burner_settings::RETURN).filter(|_| !scenario["return_c"].is_null()),
        socket(burner_settings::ROOM).filter(|_| !scenario["room_c"].is_null()),
    ];
    ROUND_S.store(kept("poll_every").and_then(|s| s.parse().ok()).unwrap_or(2), Ordering::Relaxed);
    let mut closed_since: [Option<u32>; 3] = [None; 3];

    let seconds = scenario["seconds"].as_u64().expect("seconds") as u32;
    let presses: HashMap<u32, Press> = scenario["press"]
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

    let feeds: HashMap<u32, bool> = scenario["feed"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|f| (f[0].as_u64().expect("the second of a feed") as u32, f[1].as_bool().expect("the feed is true or false")))
                .collect()
        })
        .unwrap_or_default();

    let mut automaton = Automaton::new(Burning::new());
    let mut thermostat = Thermostat::new();
    let mut last: Option<Doing> = None;
    let mut since = 0u32;
    let mut broke_at = None;
    let mut asked = Asked::Stop;
    let mut frames = Vec::new();
    for at in 0..seconds {
        let t = at as f32;
        let flame_mv = line_at(&scenario["flame_mv"], t);
        // a pin that reads nothing gives the automaton no flame: the stand says out, the board's module gives nothing
        let (boiler_c, water) = through(&BOILER, &scenario, "boiler_c", at);
        let (return_c, returns) = through(&RETURN, &scenario, "return_c", at);
        let (room_c, _) = through(&ROOM, &scenario, "room_c", at);
        let seen = Seen {
            flame: flame_mv.map_or(Flame::Out, |mv| flame::read(mv.round() as u32, thresholds)),
            boiler_c,
            flue_c: line_at(&scenario["flue_c"], t),
            draught_pa: draught_of(asked),
            draught_low: false,
            // the stand's gate is open exactly while the fan is asked to blow
            gate: if draught_of(asked).is_some_and(|pa| pa > 0.0) { 30 } else { 0 },
            // the stand's other sensors publish on every second of the scenario; the temperatures are heard as their roles say
            silence: Silence { water, ..Silence::HEARD },
        };
        let readings = Temperatures { boiler_c, return_c, return_heard: returns, asked: None, room_c: None, room_target_c: None, fire: thermostat::Fire::default() };
        if let Some(allowed) = feeds.get(&at) {
            automaton.feed(*allowed, &thermostat.order(&readings, &heat), seen, &sound);
        }
        let doing = match presses.get(&at) {
            Some(press) => {
                thermostat.press(*press);
                automaton.heat(&thermostat.order(&readings, &heat), seen, &sound)
            }
            None => automaton.second(&thermostat.order(&readings, &heat), seen, 1.0, &sound),
        };
        if last.map(|d| d.state) != Some(doing.state) {
            broke_at = broken_at(broke_at, last.map(|d| d.state), doing.state);
            since = at;
        }
        last = Some(doing);
        asked = doing.fan;
        let clocks = automaton.clocks(&sound);
        let card = Card { doing: Some(doing), lasted_s: at - since, refused: None, clocks: &clocks, broke_at, alarm_c, feed_allowed: automaton.feed_allowed(), semi: automaton.semi(), silent: Silent::default() };
        let mut sent: Vec<Value> = Vec::new();
        values(&card, &mut |address, value| sent.push(json!([address, value])));
        let on = [doing.auger, doing.igniter, doing.pump];
        for n in 0..3 {
            closed_since[n] = match (on[n], closed_since[n]) {
                (true, Some(since)) => Some(since),
                (true, None) => Some(at),
                (false, _) => None,
            };
        }
        let left_ms: [Option<u32>; 3] = std::array::from_fn(|n| {
            let (limit, since) = (limits[n]?, closed_since[n]?);
            Some(limit.saturating_sub(at - since) * 1000)
        });
        hardware(&doing, &seen, &readings, room_c, flame_mv, left_ms, sockets, &mut |address, value| sent.push(json!([address, value])));
        frames.push(json!({ "at": at, "values": sent }));
    }
    println!("{}", json!({ "frames": frames }));
}

/// The tuning the board builds, from the same numbers and by the same code (`automaton_card::tuning`).
fn tuning(n: &HashMap<String, String>) -> Tuning {
    let card = |s: &settings::Setting| n.get(s.name).cloned().unwrap_or_default();
    automaton_card::tuning(&card, &burner_api::first_burner::fixed_number)
}

/// What the thermostat decides by, from the same numbers (`automaton_card::heat_tuning`).
fn heat_tuning(n: &HashMap<String, String>) -> thermostat::Tuning {
    let card = |s: &settings::Setting| n.get(s.name).cloned().unwrap_or_default();
    automaton_card::heat_tuning(&card)
}

/// The value of POINTS - [[second, number], ...] - at T, on straight lines between them; None without points.
/// The reading at T on a line of POINTS [second, value], straight between them; None before a point whose value is null,
/// until the next point: the sensor gives nothing then, as a board's does when its wire is off.
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

/// What the draught reads under the fan asked ASKED: over the scale at full air, the setpoint once held, nothing
/// under the sensor's zero with no air.
fn draught_of(asked: Asked) -> Option<f32> {
    match asked {
        Asked::Purge => Some(draught::FULL_PA as f32),
        Asked::Hold(pa) => Some(pa),
        Asked::Stop | Asked::ByHand => None,
    }
}

/// The values of the other cards this second, by the board's addresses: the relays as the automaton drives them and
/// the seconds their limits leave (LEFT_MS - auger, igniter, pump), the fan's mode, the draught, the flame, the flue
/// and the sockets of the boiler, the return and the room (SOCKETS) as the scenario reads them.
fn hardware(doing: &Doing, seen: &Seen, readings: &Temperatures, room_c: Option<f32>, flame_mv: Option<f32>, left_ms: [Option<u32>; 3], sockets: [Option<usize>; 3], each: &mut dyn FnMut(usize, V)) {
    let on = [doing.auger, doing.igniter, doing.pump];
    for n in 0..3 {
        each(at::RELAY[n], V::On(on[n]));
        if let Some(off_in) = at::RELAY_OFF_IN[n] {
            each(off_in, left_ms[n].map_or(V::None, |ms| V::Seconds(ms.div_ceil(1000))));
        }
    }
    let (mode, state, angle) = match doing.fan {
        Asked::Stop => ("off", "off", None),
        Asked::Purge => ("full", "purging", Some(100.0)),
        Asked::Hold(_) => ("hold", "holding", Some(40.0)),
        Asked::ByHand => ("by_hand", "by_hand_free", None),
    };
    each(at::HOLDING, V::Word(mode));
    each(at::HOLDING_STATE, V::Word(state));
    each(at::HOLDING_ANGLE, angle.map_or(V::None, V::Number));
    let (asked, came_out) = match doing.fan {
        Asked::Hold(pa) => (V::Number(pa), seen.draught_pa.map_or(V::Unknown("not_read"), V::Number)),
        _ => (V::None, V::None),
    };
    each(at::HOLDING_ASKED, asked);
    each(at::HOLDING_CAME_OUT, came_out);
    each(at::FAN_DRAUGHT, match doing.fan {
        Asked::Stop => V::Word("off"),
        Asked::Purge => V::Word("full"),
        Asked::Hold(pa) => V::Number(pa),
        Asked::ByHand => V::Word("by_hand"),
    });
    each(at::PURGE, V::On(doing.fan == Asked::Purge));
    each(at::BLAST_PRESSURE, match (doing.fan, seen.draught_pa) {
        (Asked::Purge, _) => V::AtLeast(draught::FULL_PA as f32),
        (_, Some(pa)) => V::Number(pa),
        (_, None) => V::Unknown("own_zero"),
    });
    // the flame's two rows as the board makes them (devices/burner/src/flame.rs): the fire's code and the pin
    let [pin, spread, edge, standing] = at::FLAME_PIN;
    match flame_mv {
        Some(mv) => {
            each(at::FLAME_ROW, V::Word(match seen.flame {
                Flame::Steady => "steady",
                Flame::Weak => "weak",
                _ => "out",
            }));
            let mv = mv.round();
            each(pin, V::Number(mv));
            each(spread, V::Number(0.0));
            each(edge, V::Word(analog::edge(false, mv as u16)));
            each(standing, V::Word(analog::standing(mv as u16)));
        }
        None => {
            each(at::FLAME_ROW, V::Unknown("not_read"));
            each(pin, V::Unknown("not_read"));
            for part in [spread, edge, standing] {
                each(part, V::None);
            }
        }
    }
    for (socket, reading) in sockets.into_iter().zip([readings.boiler_c, readings.return_c, room_c]) {
        if let Some(socket) = socket {
            each(at::CHANNEL[socket], reading.map_or(V::Unknown("not_read"), V::Number));
        }
    }
    each(at::FLUE_TEMP, seen.flue_c.map_or(V::Unknown("not_read"), V::Number));
}
