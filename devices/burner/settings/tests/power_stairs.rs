//! The levels of the way up to the setpoint as settings of the thermostat's card: a middle and a small level, each a row of the table of steps or
//! "the level of power", and the two distances (H2, H1) they begin at. Expected from what a level is - a row of the owner's table, 14 to 3 kW - and from what a board
//! must keep: with none chosen the burner burns on the one level it burned on before, a number outside its span is refused by its name, and the thermostat's door
//! refuses the pair that leaves the middle no water.

use burner_settings::RULES;
use std::collections::HashMap;

use settings::{change, check, find, seed, value, Setting, Store};
use burner_settings::{thermostat_tuning, ALL};

fn texts(changed: &[(&str, &str)]) -> impl Fn(&Setting) -> String {
    let changed: HashMap<String, String> = changed.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |s: &Setting| changed.get(s.name).cloned().unwrap_or_else(|| s.default.to_string())
}

/// The owner's table: level, dose, pause, draught (the rows `power_steps.rs` holds the table to).
fn row(kw: u32) -> (f32, f32, f32) {
    let (on_s, off_s, pa) = match kw {
        14 => (2.0, 8.0, 51.0),
        10 => (2.0, 12.0, 36.0),
        8 => (2.0, 15.0, 29.0),
        6 => (2.0, 21.0, 22.0),
        5 => (2.0, 25.0, 18.0),
        4 => (2.0, 32.0, 15.0),
        3 => (1.0, 21.0, 11.0),
        other => panic!("{other} kW is no row of the table"),
    };
    (on_s, off_s, pa)
}

fn numbers(step: automaton::Step) -> (f32, f32, f32) {
    (step.feed.on_s, step.feed.off_s, step.pa)
}

fn card(name: &str) -> &'static Setting {
    find(&ALL, name).unwrap_or_else(|| panic!("no setting {name}"))
}

#[test]
fn a_new_board_has_no_levels_of_its_own_and_every_level_is_the_one_chosen() {
    let t = thermostat_tuning(&texts(&[]));
    assert_eq!((t.mid, t.small), (t.step, t.step), "none chosen: the three are one level");
    let t = thermostat_tuning(&texts(&[("auto_step", "10")]));
    assert_eq!(numbers(t.step), row(10));
    assert_eq!((t.mid, t.small), (t.step, t.step), "and they follow the level of power when it is turned");
}

#[test]
fn the_distances_of_a_new_board_are_8_and_3_degrees() {
    let t = thermostat_tuning(&texts(&[]));
    assert_eq!((t.mid_c, t.small_c), (8.0, 3.0));
}

#[test]
fn the_middle_and_the_small_level_are_the_rows_of_the_owners_table() {
    let t = thermostat_tuning(&texts(&[("auto_step", "6"), ("auto_step_mid", "4"), ("auto_step_small", "3")]));
    assert_eq!(numbers(t.step), row(6));
    assert_eq!(numbers(t.mid), row(4));
    assert_eq!(numbers(t.small), row(3));
}

#[test]
fn a_level_chosen_alone_leaves_the_other_one_on_the_level_of_power() {
    let t = thermostat_tuning(&texts(&[("auto_step", "6"), ("auto_step_small", "3")]));
    assert_eq!((numbers(t.mid), numbers(t.small)), (row(6), row(3)));
    let t = thermostat_tuning(&texts(&[("auto_step", "6"), ("auto_step_mid", "4")]));
    assert_eq!((numbers(t.mid), numbers(t.small)), (row(4), row(6)));
}

#[test]
fn the_distances_are_read_as_set() {
    let t = thermostat_tuning(&texts(&[("auto_mid_c", "12"), ("auto_small_c", "5")]));
    assert_eq!((t.mid_c, t.small_c), (12.0, 5.0));
}

#[test]
fn the_levels_and_the_distances_stand_on_the_thermostats_card_among_the_everyday_settings() {
    for name in ["auto_step_mid", "auto_step_small", "auto_mid_c", "auto_small_c"] {
        let s = card(name);
        assert_eq!(s.card, Some("thermostat"), "{name}");
        assert_eq!(s.level, settings::Level::Work, "{name}");
    }
}

#[test]
fn the_levels_of_a_mode_by_a_sensor_matter_with_it_and_in_the_mode_of_power_they_do_not() {
    for choice in ["boiler", "return"] {
        let b = burner_settings::bearing_with_levels("auto_hold", choice).unwrap_or_else(|| panic!("{choice} bears on nothing"));
        for name in ["auto_step_mid", "auto_step_small", "auto_mid_c", "auto_small_c"] {
            assert!(b.shows.contains(&name), "{name} does not matter with {choice}");
        }
    }
    assert!(burner_settings::bearing_with_levels("auto_hold", "power").is_none(), "the mode of power has one level and bears on nothing");
}

#[test]
fn a_level_takes_a_row_of_the_table_or_the_level_of_power_and_no_other_word() {
    for name in ["auto_step_mid", "auto_step_small"] {
        let s = card(name);
        for ok in ["chosen", "14", "10", "8", "6", "5", "4", "3"] {
            assert_eq!(check(s, ok), Ok(ok.to_string()), "{name}: {ok}");
        }
        for bad in ["", "7", "15", "2", "6 kW", "none"] {
            assert!(check(s, bad).is_err(), "{name}: '{bad}' was taken");
        }
        assert_eq!(check(s, "CHOSEN"), Ok("chosen".to_string()), "{name}: a choice is kept as it is written in the list, whatever the case typed");
    }
}

#[test]
fn a_distance_takes_its_span_and_no_word() {
    for (name, ok, bad) in [("auto_mid_c", ["2", "40"], ["1", "41", "eight"]), ("auto_small_c", ["1", "39"], ["0", "40", "3 C"])] {
        let s = card(name);
        for text in ok {
            assert_eq!(check(s, text), Ok(text.to_string()), "{name}: {text}");
        }
        for text in bad {
            assert!(check(s, text).is_err(), "{name}: '{text}' was taken");
        }
    }
}

#[test]
fn the_door_refuses_the_pair_that_leaves_the_middle_no_water_by_the_name_of_the_small_distance() {
    for (mid_c, small_c) in [("5", "5"), ("5", "9"), ("2", "39")] {
        let t = thermostat_tuning(&texts(&[("auto_mid_c", mid_c), ("auto_small_c", small_c)]));
        let refused = t.sound(88.0).expect_err("a middle with no water was taken");
        assert_eq!((refused.field.as_str(), refused.than), ("small_c", Some("mid_c")), "{mid_c} / {small_c}");
    }
    assert!(thermostat_tuning(&texts(&[("auto_mid_c", "5"), ("auto_small_c", "4")])).sound(88.0).is_ok());
}

#[test]
fn a_level_the_table_has_not_is_refused_by_the_field_it_lands_in() {
    // the span of the choice keeps a person from it; a kept text that is none reads as no level, which the door refuses by name
    for (name, field) in [("auto_step_mid", "mid.pa"), ("auto_step_small", "small.pa")] {
        let t = thermostat_tuning(&texts(&[(name, "7")]));
        let refused = t.sound(88.0).expect_err("a level that is none was taken");
        assert_eq!(refused.field, field);
    }
}

// ---- a board that has the owner's four numbers and comes up on an image that has the levels ----

#[derive(Default)]
struct Nvs(HashMap<String, String>);

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

#[test]
fn a_board_with_the_owners_four_numbers_keeps_them_on_the_new_image_and_has_no_levels_of_its_own() {
    // the card's set as the image before wrote it: the owner's four numbers on the key of the card and nothing of the levels
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    let owner = [("auto_hold", "boiler"), ("auto_hold_c", "47"), ("auto_drop_c", "6"), ("auto_step", "8")];
    let levels = ["auto_step_mid", "auto_step_small", "auto_mid_c", "auto_small_c"];
    let before: Vec<String> = nvs
        .get("s.thermostat")
        .expect("the card has a set")
        .lines()
        .filter(|line| !levels.iter().any(|name| line.starts_with(&format!("{name}="))))
        .map(|line| match owner.iter().find(|(name, _)| line.starts_with(&format!("{name}="))) {
            Some((name, text)) => format!("{name}={text}"),
            None => line.to_string(),
        })
        .collect();
    nvs.put("s.thermostat", &before.join("\n")).unwrap();
    assert!(nvs.get("s.thermostat").unwrap().contains("auto_hold_c=47") && !nvs.get("s.thermostat").unwrap().contains("auto_mid_c"), "the set of the image before was not made");
    seed(&ALL, &RULES, &mut nvs, &[]);
    for (name, text) in owner {
        assert_eq!(value(card(name), &nvs), text, "{name}: what the owner set was moved by the image");
    }
    for (name, text) in [("auto_step_mid", "chosen"), ("auto_step_small", "chosen"), ("auto_mid_c", "8"), ("auto_small_c", "3")] {
        assert_eq!(value(card(name), &nvs), text, "{name}");
    }
    let t = thermostat_tuning(&|s: &Setting| value(s, &nvs));
    assert_eq!((t.mid, t.small), (t.step, t.step), "the burner of this board burns on the one level it did");
}

#[test]
fn a_level_set_in_place_outlives_the_seeding_of_the_next_image() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    change(&ALL, &RULES, &mut nvs, "auto_step_mid", "5").unwrap();
    change(&ALL, &RULES, &mut nvs, "auto_mid_c", "11").unwrap();
    seed(&ALL, &RULES, &mut nvs, &[("auto_step_mid", "4"), ("auto_mid_c", "9")]);
    assert_eq!(value(card("auto_step_mid"), &nvs), "5", "the description took a level a person had set");
    assert_eq!(value(card("auto_mid_c"), &nvs), "11");
}
