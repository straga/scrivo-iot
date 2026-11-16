//! The times of the auger in tenths of a second: a dose of 0.8 s or 1.5 s is typed, kept as typed and reaches the automaton as that number; a whole 2 stays 2; what has more than one digit after the point, no digit
//! or nothing but a point is no time; nought is below the span. Expected from what the stage says a person may do (0.8 and 5.0 on one step, as the Ecoforest owner sets them), not from how a setting is parsed.

use burner_settings::{thermostat_tuning, ALL, RULES};
use settings::{change, seed, value, Setting, Store};
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
struct Nvs(BTreeMap<String, String>);

impl Store for Nvs {
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

fn seeded() -> Nvs {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    nvs
}

fn setting(name: &str) -> &'static Setting {
    ALL.iter().copied().find(|s| s.name == name).unwrap_or_else(|| panic!("no setting {name} in ALL"))
}

/// What the card reads: a setting as the store keeps it, and a number the card does not keep as its default.
fn texts(nvs: &Nvs) -> impl Fn(&Setting) -> String + '_ {
    move |s: &Setting| value(s, nvs)
}

/// Every time of the auger a person types: the dose and the pause of each step, the lighting's dose, the doses of lighting a lost flame again.
const TIMES: [&str; 12] =
    ["auto_14_on_s", "auto_8_on_s", "auto_3_on_s", "auto_14_off_s", "auto_8_off_s", "auto_3_off_s", "auto_dose", "auto_relight_1", "auto_relight_2", "auto_relight_3", "auto_6_on_s", "auto_6_off_s"];
/// The doses of lighting a lost flame again may be none: nought is a dose that is skipped.
const MAY_BE_NONE: [&str; 3] = ["auto_relight_1", "auto_relight_2", "auto_relight_3"];

#[test]
fn a_time_of_the_auger_takes_tenths_and_keeps_them_as_typed() {
    for name in TIMES {
        for good in ["0.8", "1.5", "5.0", "2", "30.1"] {
            let mut nvs = seeded();
            change(&ALL, &RULES, &mut nvs, name, good).unwrap_or_else(|why| panic!("{name} = {good}: {why}"));
            assert_eq!(value(setting(name), &nvs), good, "{name} kept something else than {good}");
        }
    }
}

#[test]
fn a_time_of_the_auger_that_is_no_tenths_or_below_the_span_is_refused_and_the_old_value_stays() {
    for name in TIMES {
        for bad in ["0", "0.0", "0.05", "0.85", "1.", ".8", "1,5", "abc", "", "-1", "1e1", "00.8", "0.8.1"] {
            if MAY_BE_NONE.contains(&name) && (bad == "0" || bad == "0.0") {
                continue;
            }
            let mut nvs = seeded();
            let was = value(setting(name), &nvs);
            assert!(change(&ALL, &RULES, &mut nvs, name, bad).is_err(), "{name} = {bad:?} was taken");
            assert_eq!(value(setting(name), &nvs), was, "{name} moved on a refusal of {bad:?}");
        }
    }
}

#[test]
fn a_time_above_its_span_is_refused_whatever_its_tenths() {
    for (name, bad) in [("auto_8_on_s", "120.1"), ("auto_8_off_s", "600.1"), ("auto_dose", "120.1"), ("auto_relight_1", "60.1")] {
        let mut nvs = seeded();
        assert!(change(&ALL, &RULES, &mut nvs, name, bad).is_err(), "{name} = {bad:?} was taken");
    }
    for (name, good) in [("auto_8_on_s", "120.0"), ("auto_8_off_s", "600.0"), ("auto_dose", "120"), ("auto_relight_1", "60.0")] {
        let mut nvs = seeded();
        change(&ALL, &RULES, &mut nvs, name, good).unwrap_or_else(|why| panic!("{name} = {good}: {why}"));
    }
}

#[test]
fn what_a_board_kept_in_whole_seconds_reads_as_the_same_seconds() {
    // a setting kept before this change is a text like "2"; there is nothing to carry over, and it is the same time
    let mut nvs = seeded();
    change(&ALL, &RULES, &mut nvs, "auto_step", "8").unwrap();
    nvs.put("auto_8_on_s", "2").unwrap();
    nvs.put("auto_8_off_s", "15").unwrap();
    let t = thermostat_tuning(&texts(&nvs));
    assert_eq!((t.step.feed.on_s, t.step.feed.off_s), (2.0, 15.0));
}

#[test]
fn the_chosen_step_hands_the_automaton_its_tenths_and_does_not_round_them() {
    for (on, off) in [("0.8", "5.0"), ("1.5", "5.0"), ("0.1", "0.1")] {
        let mut nvs = seeded();
        change(&ALL, &RULES, &mut nvs, "auto_step", "8").unwrap();
        change(&ALL, &RULES, &mut nvs, "auto_8_on_s", on).unwrap_or_else(|why| panic!("auto_8_on_s = {on}: {why}"));
        change(&ALL, &RULES, &mut nvs, "auto_8_off_s", off).unwrap_or_else(|why| panic!("auto_8_off_s = {off}: {why}"));
        let t = thermostat_tuning(&texts(&nvs));
        assert_eq!((t.step.feed.on_s, t.step.feed.off_s), (on.parse::<f32>().unwrap(), off.parse::<f32>().unwrap()), "on {on} off {off}");
    }
}

#[test]
fn the_lighting_doses_reach_the_automaton_in_tenths() {
    let mut nvs = seeded();
    for (name, text) in [("auto_dose", "0.8"), ("auto_relight_1", "1.5"), ("auto_relight_2", "0.8"), ("auto_relight_3", "2")] {
        change(&ALL, &RULES, &mut nvs, name, text).unwrap_or_else(|why| panic!("{name} = {text}: {why}"));
    }
    let fixed: HashMap<&str, f32> = HashMap::new();
    let t = automaton_card::tuning(&texts(&nvs), &|name: &str| fixed.get(name).copied().unwrap_or(1.0));
    assert_eq!(t.kindling.dose_s, 0.8);
    assert_eq!(t.relight_s, [1.5, 0.8, 2.0]);
}

#[test]
fn a_time_in_tenths_is_read_back_as_that_time_and_not_as_nought() {
    assert_eq!(settings::words(setting("auto_8_on_s"), "0.8"), "0.8 s");
    assert_eq!(settings::words(setting("auto_8_off_s"), "5.0"), "5.0 s");
    assert_eq!(settings::words(setting("auto_8_on_s"), "2"), "2 s", "a whole time reads as it did");
}

#[test]
fn the_limits_that_are_no_time_of_a_feed_stay_whole() {
    // the forgotten-button limit and the share of the second dose are not times of the feed: no tenths there
    let mut nvs = seeded();
    assert!(change(&ALL, &RULES, &mut nvs, "auger_limit", "60.5").is_err());
    assert!(change(&ALL, &RULES, &mut nvs, "auto_dose_again", "12.5").is_err());
    change(&ALL, &RULES, &mut nvs, "auger_limit", "45").unwrap();
    change(&ALL, &RULES, &mut nvs, "auto_dose_again", "12").unwrap();
}
