//! What the burner's rules of the settings do to the same inputs, written down as a transcript and compared with the
//! one taken before the rules moved out of the mechanism: the carry across a change of meaning,
//! the values renamed, the refusal of a mode whose sensor no socket plays, the words of a role.
//!
//! The expected transcript is `tests/golden/rules.txt`. It is rewritten only by a person who means the burner to behave
//! otherwise: `UPDATE_GOLDEN=1 cargo test -p settings --test rules_golden`.

use burner_settings::RULES;
use std::collections::BTreeMap;
use std::fmt::Write as _;

use settings::{change_all, carry_over, key_of, seed, value, words, Setting, Store, ZONE};
use burner_settings::{plays, role_words, ALL, AUTO_HOLD, STEP_TABLE};

#[derive(Default)]
struct Map(BTreeMap<String, String>);

impl Store for Map {
    fn get(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }

    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.0.insert(name.into(), value.into());
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<bool, String> {
        Ok(self.0.remove(name).is_some())
    }
}

fn dump(out: &mut String, map: &Map) {
    for (key, text) in &map.0 {
        let lines: Vec<&str> = text.split('\n').collect();
        writeln!(out, "  store {key}: {}", lines.join(" | ")).unwrap();
    }
}

fn carried(out: &mut String, title: &str, keys: &[(&str, String)]) {
    writeln!(out, "carry: {title}").unwrap();
    let mut map = Map::default();
    for (key, text) in keys {
        map.put(key, text).unwrap();
    }
    let said = carry_over(&ALL, &RULES, &mut map);
    for line in &said {
        writeln!(out, "  said {line}").unwrap();
    }
    dump(out, &map);
    let again = carry_over(&ALL, &RULES, &mut map);
    writeln!(out, "  again {}", again.len()).unwrap();
}

fn row(kw: &str) -> &'static burner_settings::StepNumbers {
    STEP_TABLE.iter().find(|r| r.kw == kw).expect("a row of the table")
}

fn pair_of(kw: &str) -> String {
    let r = row(kw);
    format!("auto_pa={}\nauto_feed_on_s={}\nauto_feed_off_s={}", r.pa.default, r.on_s.default, r.off_s.default)
}

fn saved(out: &mut String, title: &str, kept: &[(&str, &str)], pairs: &[(&str, &str)]) {
    writeln!(out, "save: {title}").unwrap();
    let mut map = Map::default();
    seed(&ALL, &RULES, &mut map, &[]);
    for (name, text) in kept {
        let s = ALL.iter().find(|s| s.name == *name).expect("a setting");
        let key = key_of(s.card);
        let was = map.get(&key).unwrap_or_default();
        let mut lines: Vec<String> = was.split('\n').filter(|l| !l.starts_with(&format!("{name}="))).map(String::from).collect();
        lines.push(format!("{name}={text}"));
        map.put(&key, &lines.join("\n")).unwrap();
    }
    match change_all(&ALL, &RULES, &mut map, pairs) {
        Ok(moved) => {
            let moved: Vec<String> = moved.iter().map(|m| format!("{}={}", m.name, m.value)).collect();
            writeln!(out, "  moved {}", moved.join(", ")).unwrap();
        }
        Err(why) => {
            writeln!(out, "  refused {} / {} / {}", why.code(), why.wire(), why).unwrap();
        }
    }
    let now: Vec<String> = ALL
        .iter()
        .filter(|s| pairs.iter().any(|(n, _)| *n == s.name) || kept.iter().any(|(n, _)| *n == s.name))
        .map(|s| format!("{}={}", s.name, value(s, &map)))
        .collect();
    writeln!(out, "  now {}", now.join(", ")).unwrap();
}

fn transcript() -> String {
    let mut out = String::new();

    carried(&mut out, "gains under the old unit, one set", &[(settings::KEY, "up_kp=30\nup_ki=9\ndown_kp=30\ndown_ki=6\ndraught_pa=31\nfan_step=5".into())]);
    carried(&mut out, "a gain nobody can carry", &[(settings::KEY, "up_kp=nonsense\nup_ki=9\ndown_kp=30\ndown_ki=6".into())]);
    carried(&mut out, "a gain past the span is clamped", &[(settings::KEY, "up_kp=2000\nup_ki=9\ndown_kp=30\ndown_ki=6".into())]);
    carried(&mut out, "a role kept as water", &[(settings::KEY, "role_ntc_1=water\nrole_ntc_2=supply\nrole_ntc_3=own:tank\n@meaning=1".into())]);
    carried(&mut out, "nothing kept", &[]);
    for (title, hold, step, kw) in [
        ("a pair that is the defaults of another step", "boiler", "6", "3"),
        ("a pair that is the defaults of the step chosen", "boiler", "3", "3"),
        ("a pair in a hold of power", "power", "6", "3"),
    ] {
        carried(&mut out, title, &[(&key_of(Some("automaton")), format!("auto_hold={hold}\nauto_step={step}\n{}\n@meaning=2", pair_of(kw)))]);
    }
    carried(
        &mut out,
        "a pair that is no step's, into the step chosen",
        &[(&key_of(Some("automaton")), "auto_hold=boiler\nauto_step=6\nauto_pa=44\nauto_feed_on_s=7\nauto_feed_off_s=25\n@meaning=2".into())],
    );
    carried(
        &mut out,
        "a step a person set is not written over",
        &[(&key_of(Some("automaton")), format!("auto_hold=boiler\nauto_step=6\nauto_3_pa=19\n{}\n@meaning=2", pair_of("3")))],
    );
    carried(
        &mut out,
        "meaning 3 with the pair put aside",
        &[
            (&key_of(Some("automaton")), format!("auto_hold=boiler\nauto_step=6\nauto_6_pa={}\nauto_6_on_s={}\nauto_6_off_s={}\n@meaning=3", row("3").pa.default, row("3").on_s.default, row("3").off_s.default)),
            (settings::LEFT, pair_of("3")),
        ],
    );

    saved(&mut out, "a mode by the boiler with no socket playing it", &[("auto_hold", "power")], &[("auto_hold", "boiler")]);
    saved(&mut out, "the role and the mode in one save", &[], &[("role_ntc_1", "boiler"), ("auto_hold", "boiler")]);
    saved(&mut out, "a mode by the return with another role set", &[("role_ntc_2", "boiler")], &[("auto_hold", "return")]);
    saved(&mut out, "the room that asks with no socket", &[], &[("auto_asked_by", "room")]);
    saved(&mut out, "the room that asks with its socket in the same save", &[], &[("role_ntc_4", "room"), ("auto_asked_by", "room")]);
    saved(&mut out, "the schedule asks", &[], &[("auto_asked_by", "schedule")]);
    saved(&mut out, "a level of power", &[], &[("auto_hold", "power")]);
    saved(&mut out, "a mode kept and saved as it stood is not asked again", &[("auto_hold", "boiler")], &[("auto_hold", "boiler")]);
    saved(&mut out, "a city kept for the zone, saved as the zone", &[("zone", "kyiv")], &[("zone", "gmt+2")]);
    saved(&mut out, "a setting nobody has", &[], &[("nonsense", "1")]);
    saved(&mut out, "a mode that is none", &[], &[("auto_hold", "nonsense")]);
    saved(&mut out, "a refusal in the middle of a save changes nothing", &[], &[("fan_step", "7"), ("auto_hold", "return")]);

    for kept in ["kyiv", "riga", "warsaw", "london", "istanbul", "gmt+3", "nonsense"] {
        let mut map = Map::default();
        map.put(&key_of(ZONE.card), &format!("zone={kept}")).unwrap();
        let read = value(&ZONE, &map);
        writeln!(out, "zone kept {kept}: value {read}, words {}, posix {}", words(&ZONE, &read), settings::posix(&read)).unwrap();
    }

    for stored in ["boiler", "supply", "return", "room", "free", "own:tank", "own:", "nonsense"] {
        writeln!(out, "role_words {stored}: {}", role_words(stored)).unwrap();
    }
    let mut map = Map::default();
    seed(&ALL, &RULES, &mut map, &[("role_ntc_1", "boiler"), ("role_ntc_5", "own:tank")]);
    let read = |s: &Setting| value(s, &map);
    for role in ["boiler", "return", "room", "supply", "own:tank", "free"] {
        writeln!(out, "plays {role}: {}", plays(role, &read)).unwrap();
    }
    writeln!(out, "auto_hold default {}", AUTO_HOLD.default).unwrap();
    out
}

#[test]
fn the_rules_of_the_burner_answer_the_inputs_as_they_answered_before_they_moved() {
    let now = transcript();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/rules.txt");
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden")).unwrap();
        std::fs::write(path, &now).unwrap();
        return;
    }
    let then = std::fs::read_to_string(path).expect("tests/golden/rules.txt: take it with UPDATE_GOLDEN=1 on the code before the move");
    assert_eq!(now, then, "the rules of the burner answer otherwise than they did");
}
