//! The lockout of a light as settings of the thermostat's card: how long after the fire goes out it is not lit again by the thermostat, in minutes, and how cold the held water may get meanwhile. Expected from what the
//! stage says: nought minutes is no lockout and is what a new board has, the floor is 30, each takes its span and no word, and the thermostat reads them as seconds and degrees.

use std::collections::HashMap;

use settings::{check, Refusal, Setting};
use burner_settings::{bearing_with_levels, thermostat_tuning, AUTO_LOCK_C, AUTO_LOCK_MIN};

fn texts(changed: &[(&str, &str)]) -> impl Fn(&Setting) -> String {
    let changed: HashMap<String, String> = changed.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |s: &Setting| changed.get(s.name).cloned().unwrap_or_else(|| s.default.to_string())
}

#[test]
fn a_new_board_has_no_lockout_and_the_floor_the_stage_names() {
    let t = thermostat_tuning(&texts(&[]));
    assert_eq!((t.lockout.wait_s, t.lockout.floor_c), (0.0, 30.0));
}

#[test]
fn the_minutes_of_the_card_are_the_seconds_of_the_thermostat() {
    let t = thermostat_tuning(&texts(&[("auto_lock_min", "120"), ("auto_lock_c", "35")]));
    assert_eq!((t.lockout.wait_s, t.lockout.floor_c), (7200.0, 35.0));
    assert_eq!(thermostat_tuning(&texts(&[("auto_lock_min", "1")])).lockout.wait_s, 60.0);
}

#[test]
fn the_minutes_take_their_span_and_no_word() {
    for ok in ["0", "1", "120", "720"] {
        assert_eq!(check(&AUTO_LOCK_MIN, ok), Ok(ok.to_string()));
    }
    for bad in ["-1", "721", "", "two hours", "1.5", " 5x"] {
        assert!(matches!(check(&AUTO_LOCK_MIN, bad), Err(Refusal::NotWhole { name: "auto_lock_min", .. })), "{bad:?}");
    }
}

#[test]
fn the_floor_takes_its_span_and_no_word() {
    for ok in ["5", "30", "60"] {
        assert_eq!(check(&AUTO_LOCK_C, ok), Ok(ok.to_string()));
    }
    for bad in ["4", "61", "", "cold"] {
        assert!(matches!(check(&AUTO_LOCK_C, bad), Err(Refusal::NotWhole { name: "auto_lock_c", .. })), "{bad:?}");
    }
}

#[test]
fn both_matter_in_a_mode_by_a_sensor_and_in_no_other() {
    for mode in ["boiler", "return"] {
        let shows = bearing_with_levels("auto_hold", mode).expect("a mode by a sensor says what it brings").shows;
        assert!(shows.contains(&"auto_lock_min") && shows.contains(&"auto_lock_c"), "{mode}: {shows:?}");
    }
    assert!(bearing_with_levels("auto_hold", "power").is_none(), "the mode of power brings nothing, and has no water to withhold a light from");
}

#[test]
fn the_seconds_a_flame_must_stand_to_be_a_fire_are_the_automatons_own_sure_s_and_no_setting_of_the_thermostat() {
    // one measure of a flame that is believed: the card of the automaton says it, and the automaton and the thermostat both read it
    for sure in ["3", "10", "25"] {
        let text = texts(&[("auto_sure_s", sure)]);
        let seconds: f32 = sure.parse().unwrap();
        assert_eq!(burner_settings::automaton_tuning(&text).sure_s, seconds);
        assert_eq!(thermostat_tuning(&text).lockout.sure_s, seconds, "the thermostat read another number than the automaton's {sure}");
    }
    assert!(!burner_settings::ALL.iter().any(|s| s.card == Some("thermostat") && s.name.contains("sure")), "a second setting for the same measure");
}
