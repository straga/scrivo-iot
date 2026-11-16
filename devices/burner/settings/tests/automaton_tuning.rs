//! The automaton's tuning out of the settings (`automaton_tuning`): which setting is which number of the fire, which numbers are the image's own, and which a person's numbers the automaton's door refuses.
//! Expected from what each number is and what the first burner burns by - the fixed numbers as its dictionary lists them, a level of power as the owner's table makes it: a dose, a pause and a draught on
//! each level; 2 s and the pause from the period 136 / kW and the draught in proportion to the power from 14 kW at 51 Pa, but for the 3 kW row the burner held on 27.09 - and, for the rest, from that burner's own
//! mapping (`automaton_card::tuning`) played on a sample of texts, accepted and refused, whose answer is fixed here before the run. The first burner burned a temperature's hold by a pair of its own: what it is
//! handed is the numbers of the chosen step, so that the two are held to the same burning.

use std::collections::HashMap;

use automaton::Refused;
use settings::{Setting};
use burner_settings::{automaton_tuning, thermostat_tuning, ALL};
use thermostat::Hold;

fn defaults(s: &Setting) -> String {
    s.default.to_string()
}

/// The texts the settings keep: the defaults, over which what a row changes.
fn texts(changed: &[(&str, &str)]) -> impl Fn(&Setting) -> String {
    let changed: HashMap<String, String> = changed.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |s: &Setting| changed.get(s.name).cloned().unwrap_or_else(|| s.default.to_string())
}

/// The numbers of the fire and the thermostat's, as the two doors let them through.
type Numbers = (automaton::Tuning, thermostat::Tuning);

/// Both doors: the fire's, and the thermostat's with the overheating the fire holds.
fn doors(fire: automaton::Tuning, heat: thermostat::Tuning) -> Result<Numbers, Refused> {
    let fire_sound = fire.sound()?;
    let heat_sound = heat.sound(fire.too_hot_c)?;
    Ok((*fire_sound.numbers(), *heat_sound.numbers()))
}

fn ours(text: &dyn Fn(&Setting) -> String) -> Result<Numbers, Refused> {
    doors(automaton_tuning(text), thermostat_tuning(text))
}

/// The first burner, handed what it reads: its pair is the numbers of the step the new burner has chosen - what a temperature's hold burns by here - and everything else the same texts.
fn first_burners(changed: &[(&str, &str)]) -> Result<Numbers, Refused> {
    let ours = texts(&with_the_first_burners_smouldering(changed));
    let chosen = ours(&burner_settings::AUTO_STEP);
    let row = burner_settings::STEP_TABLE.iter().find(|r| r.kw == chosen);
    let text = move |s: &Setting| match (s.name, row) {
        ("auto_pa", Some(r)) => ours(r.pa),
        ("auto_feed_on_s", Some(r)) => ours(r.on_s),
        ("auto_feed_off_s", Some(r)) => ours(r.off_s),
        ("auto_pa" | "auto_feed_on_s" | "auto_feed_off_s", None) => String::new(),
        _ => ours(s),
    };
    // THE NUMBERS THE FIRST BURNER FIXED ARE THE SETTINGS OF THE NEW ONE: what a text says of one lands in the number it was
    doors(
        automaton_card::tuning(&text, &|name: &str| fixed_from_the_texts(&texts(&with_the_first_burners_smouldering(changed)), name)),
        automaton_card::heat_tuning(&text),
    )
}

/// The first burner smouldered for 300 s: the new one is asked for the same, explicitly, since its own default is 900 (the owner's).
fn with_the_first_burners_smouldering<'a>(changed: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut all = vec![("auto_smoulder_s", "300")];
    all.extend_from_slice(changed);
    all
}

/// A number the first burner fixed, by the name its dictionary gives it, read from the setting of the new burner that is that number now.
fn fixed_from_the_texts(text: &dyn Fn(&Setting) -> String, name: &str) -> f32 {
    let setting = match name {
        "purge_after_s" => "auto_purge_end",
        "burn_out_min_s" => "auto_out_min_s",
        "smoulder_s" => "auto_smoulder_s",
        "smoulder_pa" => "auto_smould_pa",
        "smoulder_on_s" => "auto_smould_on",
        "smoulder_off_s" => "auto_smould_off",
        "flue_holds_c" => "auto_flue_c",
        "flue_cold_c" => "auto_flue_cold",
        "flue_fall_c" => "auto_flue_fall",
        "lit_s" => "auto_lit_s",
        "too_hot_c" => "auto_too_hot_c",
        "alarm_c" => "auto_alarm_c",
        "sure_s" => "auto_sure_s",
        "relight_hot_c" => "auto_ember_c",
        "spin_up_s" => "auto_spin_up_s",
        other => return burner_api::first_burner::fixed_number(other),
    };
    match ALL.iter().find(|s| s.name == setting) {
        Some(s) => text(s).parse::<f32>().unwrap_or(f32::NAN),
        None => f32::NAN,
    }
}

/// The settings of the automaton's card the first burner has too, by name: the term of the low pressure is the new burner's alone (`the_term_of_low_pressure_is_a_number_of_the_new_burner_alone`).
fn card() -> Vec<&'static str> {
    ALL.iter().filter(|s| s.card == Some("automaton") && s.name != "auto_low_pa_s").map(|s| s.name).collect()
}

/// The twelve numbers the image once fixed that are still numbers of the fire (the thirteenth, the rise of the flue, is not among them: the cold mark and the fall of its own replaced it), each now the setting of the card that names it: setting, the number it lands in, what the burner burned by before - and what the owner chose for smouldering (900 s, the boiler is slow).
const FIRE: [(&str, &str, f32); 12] = [
    ("auto_purge_end", "purge_after_s", 20.0),
    ("auto_out_min_s", "burn_out_min_s", 60.0),
    ("auto_smoulder_s", "smoulder_s", 900.0),
    ("auto_smould_pa", "smoulder.pa", 0.0),
    ("auto_smould_on", "smoulder.feed.on_s", 1.0),
    ("auto_smould_off", "smoulder.feed.off_s", 50.0),
    ("auto_flue_c", "flue_holds_c", 100.0),
    ("auto_too_hot_c", "too_hot_c", 88.0),
    ("auto_alarm_c", "alarm_c", 95.0),
    ("auto_sure_s", "sure_s", 10.0),
    ("auto_ember_c", "relight_hot_c", 45.0),
    ("auto_spin_up_s", "spin_up_s", 10.0),
];

/// The number of the tuning a field name stands for.
fn number_of(t: &automaton::Tuning, field: &str) -> f32 {
    match field {
        "purge_after_s" => t.purge_after_s,
        "burn_out_min_s" => t.burn_out_min_s,
        "smoulder_s" => t.smoulder_s,
        "smoulder.pa" => t.smoulder.pa,
        "smoulder.feed.on_s" => t.smoulder.feed.on_s,
        "smoulder.feed.off_s" => t.smoulder.feed.off_s,
        "flue_holds_c" => t.flue_holds_c,
        "too_hot_c" => t.too_hot_c,
        "alarm_c" => t.alarm_c,
        "sure_s" => t.sure_s,
        "relight_hot_c" => t.relight_hot_c,
        "spin_up_s" => t.spin_up_s,
        other => panic!("no number called {other}"),
    }
}

/// The numbers by the card's default: what a board that has kept nothing reads (27.09: a cold flue is under 70 °C, 10 °C is a fall in ten
/// minutes, a flame is a fire after five seconds), each on the card of the automaton and in the number it names.
#[test]
fn the_flue_and_the_flame_of_a_low_fire_stand_at_the_numbers_of_the_test_burn_by_default() {
    let t = automaton_tuning(&defaults);
    for (name, field, want) in [("auto_flue_cold", "flue_cold_c", 70.0), ("auto_flue_fall", "flue_fall_c", 10.0), ("auto_flue_win_s", "flue_fall_s", 600.0), ("auto_lit_s", "lit_s", 5.0)] {
        let setting = ALL.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no setting {name}"));
        assert_eq!(setting.card, Some("automaton"), "{name} stands on another card");
        let got = match field {
            "flue_cold_c" => t.flue_cold_c,
            "flue_fall_c" => t.flue_fall_c,
            "flue_fall_s" => t.flue_fall_s,
            _ => t.lit_s,
        };
        assert_eq!(got, want, "{field} by the default of {name}");
    }
    // what the burner no longer asks is no setting of the image: the window and the rise of the fire that did not climb
    for gone in ["auto_no_fuel_s", "auto_flue_rise"] {
        assert!(ALL.iter().all(|s| s.name != gone), "{gone} is still a setting: the rule it numbered is gone");
    }
}

#[test]
fn every_number_of_the_fire_is_a_setting_of_the_automatons_card_and_stands_at_what_the_burner_burned_by() {
    let t = automaton_tuning(&defaults);
    for (name, field, factory) in FIRE {
        let setting = ALL.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("{field} is no setting yet: there is no {name}"));
        assert_eq!(setting.card, Some("automaton"), "{name} stands on another card");
        assert_eq!(number_of(&t, field), factory, "{field} by the default of {name}");
    }
}

#[test]
fn each_setting_of_the_card_becomes_the_number_it_names_and_no_other() {
    // the numbers a person turns, each a different one, so a setting that lands in another field cannot hide; the step is the sixth, which is the chosen one
    let texts_of_a_person = texts(&[
        ("auto_purge", "31"),
        ("auto_igniter", "32"),
        ("auto_wait", "33"),
        ("auto_tries", "4"),
        ("auto_dose_again", "35"),
        ("auto_rise_s", "36"),
        ("auto_no_flame_s", "37"),
        ("auto_burn_off_s", "238"),
        ("auto_pump_on_c", "39"),
        ("auto_pump_off_c", "40"),
        ("auto_flue_win_s", "41"),
        ("auto_flue_cold", "58"),
        ("auto_flue_fall", "14"),
        ("auto_lit_s", "15"),
        ("auto_hold_c", "72"),
        ("auto_drop_c", "5"),
        ("auto_relight_1", "11"),
        ("auto_relight_2", "12"),
        ("auto_relight_3", "13"),
        ("auto_kindle_pa", "44"),
        ("auto_dose", "45"),
        ("auto_6_pa", "46"),
        ("auto_6_on_s", "47"),
        ("auto_6_off_s", "48"),
        ("auto_purge_end", "51"),
        ("auto_out_min_s", "52"),
        ("auto_smoulder_s", "53"),
        ("auto_smould_pa", "54"),
        ("auto_smould_on", "55"),
        ("auto_smould_off", "56"),
        ("auto_flue_c", "57"),
        ("auto_too_hot_c", "89"),
        ("auto_alarm_c", "97"),
        ("auto_sure_s", "61"),
        ("auto_ember_c", "62"),
        ("auto_spin_up_s", "63"),
    ]);
    let (t, h) = (automaton_tuning(&texts_of_a_person), thermostat_tuning(&texts_of_a_person));
    assert_eq!(
        (t.purge_before_s, t.igniter_s, t.wait_for_flame_s, t.tries, t.rise_s, t.flame_lost_s, t.burn_off_s),
        (31.0, 32.0, 33.0, 4, 36.0, 37.0, 238.0)
    );
    assert_eq!((t.pump_on_c, t.pump_off_below_c, t.flue_fall_s, h.want_c, h.hysteresis_c), (39.0, 40.0, 41.0, 72.0, 5.0));
    // the flue and the flame of 6.6: each setting lands in the number it names and in no other
    assert_eq!((t.flue_cold_c, t.flue_fall_c, t.lit_s), (58.0, 14.0, 15.0));
    assert_eq!(t.relight_s, [11.0, 12.0, 13.0]);
    assert_eq!(
        [t.purge_after_s, t.burn_out_min_s, t.smoulder_s, t.smoulder.pa, t.smoulder.feed.on_s, t.smoulder.feed.off_s, t.flue_holds_c, t.too_hot_c, t.alarm_c, t.sure_s, t.relight_hot_c, t.spin_up_s],
        [51.0, 52.0, 53.0, 54.0, 55.0, 56.0, 57.0, 89.0, 97.0, 61.0, 62.0, 63.0]
    );
    assert_eq!((t.kindling.pa, t.kindling.dose_s), (44.0, 45.0));
    assert_eq!((h.step.pa, h.step.feed.on_s, h.step.feed.off_s), (46.0, 47.0, 48.0));
    // a share of the lighting's dose: whole per cent in the setting, a fraction in the number
    assert!((t.dose_after_first - 0.35).abs() < 1e-6, "{}", t.dose_after_first);
}

/// The owner's table (the 3 kW row is what the burner held on 27.09): level, dose, pause, draught.
const TABLE: [(u32, f32, f32, f32); 7] = [(14, 2.0, 8.0, 51.0), (10, 2.0, 12.0, 36.0), (8, 2.0, 15.0, 29.0), (6, 2.0, 21.0, 22.0), (5, 2.0, 25.0, 18.0), (4, 2.0, 32.0, 15.0), (3, 1.0, 21.0, 11.0)];

#[test]
fn a_level_of_power_is_the_row_of_the_owners_table_in_every_hold() {
    for hold in ["power", "boiler", "return", "room"] {
        for (kw, on_s, off_s, pa) in TABLE {
            let t = thermostat_tuning(&texts(&[("auto_hold", hold), ("auto_step", &kw.to_string())]));
            for (name, step) in [("step", t.step), ("power", t.power)] {
                assert_eq!((step.feed.on_s, step.feed.off_s, step.pa), (on_s, off_s, pa), "{name} at {kw} kW in {hold}");
            }
        }
    }
}

#[test]
fn a_level_the_table_has_not_is_no_number_and_the_door_refuses_it_by_name() {
    for odd in ["7", "", "fourteen"] {
        let t = thermostat_tuning(&texts(&[("auto_step", odd)]));
        assert!(t.power.pa.is_nan() && t.power.feed.on_s.is_nan() && t.power.feed.off_s.is_nan(), "{odd:?} made a level");
        let refused = ours(&texts(&[("auto_step", odd)])).expect_err("a level that is none was taken");
        assert_eq!(refused.field, "step.pa", "{odd:?}");
    }
}

#[test]
fn what_the_burner_holds_is_read_off_the_choice_and_a_text_it_does_not_know_holds_the_boiler() {
    for (text, hold) in [("power", Hold::Power), ("boiler", Hold::Boiler), ("return", Hold::Return), ("room", Hold::Boiler), ("sky", Hold::Boiler), ("", Hold::Boiler)] {
        assert_eq!(thermostat_tuning(&texts(&[("auto_hold", text)])).hold, hold, "{text:?}");
    }
}

#[test]
fn the_factorys_own_numbers_are_taken_by_the_door() {
    assert!(ours(&defaults).is_ok(), "{:?}", ours(&defaults).err());
}

#[test]
fn a_number_the_door_cannot_take_is_refused_by_the_name_of_the_field_it_lands_in() {
    for (setting, bad, field) in [
        ("auto_purge", "0", "purge_before_s"),
        ("auto_igniter", "abc", "igniter_s"),
        ("auto_wait", "-3", "wait_for_flame_s"),
        ("auto_tries", "0", "tries"),
        ("auto_tries", "x", "tries"),
        ("auto_dose_again", "150", "dose_after_first"),
        ("auto_dose_again", "0", "dose_after_first"),
        ("auto_rise_s", "0", "rise_s"),
        ("auto_no_flame_s", "", "flame_lost_s"),
        ("auto_burn_off_s", "5", "burn_off_s"),
        ("auto_hold_c", "abc", "want_c"),
        ("auto_drop_c", "90", "hysteresis_c"),
        ("auto_kindle_pa", "0", "kindling.pa"),
        ("auto_dose", "0", "kindling.dose_s"),
        ("auto_6_pa", "-1", "step.pa"),
        ("auto_6_on_s", "0", "step.feed.on_s"),
        ("auto_6_off_s", "0", "step.feed.off_s"),
        ("auto_pump_on_c", "0", "pump_on_c"),
        ("auto_pump_off_c", "abc", "pump_off_below_c"),
        ("auto_flue_win_s", "0", "flue_fall_s"),
        ("auto_flue_cold", "0", "flue_cold_c"),
        ("auto_flue_fall", "0", "flue_fall_c"),
        ("auto_lit_s", "0", "lit_s"),
        ("auto_purge_end", "0", "purge_after_s"),
        ("auto_purge_end", "abc", "purge_after_s"),
        ("auto_smould_on", "0", "smoulder.feed.on_s"),
        ("auto_smould_off", "0", "smoulder.feed.off_s"),
        ("auto_smould_pa", "-1", "smoulder.pa"),
        ("auto_flue_c", "0", "flue_holds_c"),
        ("auto_too_hot_c", "60", "too_hot_c"),
        ("auto_alarm_c", "80", "alarm_c"),
        ("auto_sure_s", "0", "sure_s"),
        ("auto_ember_c", "0", "relight_hot_c"),
        ("auto_spin_up_s", "0", "spin_up_s"),
        ("auto_out_min_s", "700", "burn_off_s"),
    ] {
        let refused = ours(&texts(&[(setting, bad)])).expect_err(&format!("{setting} = {bad:?} was taken"));
        assert_eq!(refused.field, field, "{setting} = {bad:?}: {}", refused.words());
    }
}

#[test]
fn the_alarm_stands_strictly_above_the_overheating_and_an_equal_pair_is_refused_by_the_alarms_name() {
    // the two protections are each inside their own span at 90/90 and at 88/88, and it is the pair the door refuses: an alarm that sounds at the very water the pump is let run at is no second line
    for (too_hot, alarm) in [("90", "90"), ("88", "88"), ("75", "75"), ("89", "88")] {
        let refused = ours(&texts(&[("auto_too_hot_c", too_hot), ("auto_alarm_c", alarm)])).expect_err(&format!("{too_hot} / {alarm} was taken"));
        assert_eq!(refused.field, "alarm_c", "{too_hot} / {alarm}: {}", refused.words());
    }
    // one degree over is a pair
    for (too_hot, alarm) in [("88", "89"), ("90", "91"), ("70", "71"), ("89", "99")] {
        assert!(ours(&texts(&[("auto_too_hot_c", too_hot), ("auto_alarm_c", alarm)])).is_ok(), "{too_hot} / {alarm} was refused");
    }
}

#[test]
fn a_number_in_its_place_is_taken_whatever_it_is_moved_to() {
    for (setting, good) in [("auto_purge", "45"), ("auto_tries", "3"), ("auto_dose_again", "20"), ("auto_hold_c", "70"), ("auto_kindle_pa", "30"), ("auto_6_pa", "40"), ("auto_step", "6"), ("auto_hold", "room"), ("auto_smoulder_s", "0"), ("auto_smoulder_s", "7200"), ("auto_too_hot_c", "90"), ("auto_alarm_c", "99")] {
        assert!(ours(&texts(&[(setting, good)])).is_ok(), "{setting} = {good:?} was refused");
    }
}

/// The sample the first burner's mapping and this one are played on: the defaults, each setting of the card put to each of ten texts, and every level of power under every hold.
fn sample() -> Vec<Vec<(&'static str, &'static str)>> {
    let mut all: Vec<Vec<(&'static str, &'static str)>> = vec![vec![]];
    for name in card() {
        for text in ["0", "1", "7", "50", "100", "abc", "", "-3", "14", "3"] {
            all.push(vec![(name, text)]);
        }
    }
    for hold in ["power", "boiler", "return", "room"] {
        for level in ["14", "10", "8", "6", "5", "4", "3", "7"] {
            all.push(vec![("auto_hold", hold), ("auto_step", level)]);
        }
    }
    all
}

/// What the two are compared on: the Ok numbers, or the refusal. THE FIRST BURNER'S `power` IS ITS FACTORY RULE FOR A HOLD OF POWER and the new burner's is the chosen row of its table - the same row as `step` -, so what
/// is compared is the one `step` both burn on, and the table's own numbers are held to the owner's table by `a_level_of_power_is_the_row_of_the_owners_table_in_every_hold`.
fn view(result: Result<Numbers, Refused>) -> Result<Numbers, Refused> {
    result.map(|(fire, mut heat)| {
        heat.power = heat.step;
        (fire, heat)
    })
}

#[test]
fn the_tuning_is_the_first_burners_on_every_text_of_the_sample_taken_or_refused_alike() {
    let sample = sample();
    let wrong: Vec<String> = sample
        .iter()
        .filter(|changed| view(ours(&texts(&with_the_first_burners_smouldering(changed)))) != view(first_burners(changed)))
        .map(|changed| format!("{changed:?}: ours {:?}, the first burner's {:?}", ours(&texts(&with_the_first_burners_smouldering(changed))).err(), first_burners(changed).err()))
        .collect();
    assert!(sample.len() > 200, "the sample is {} texts", sample.len());
    assert!(wrong.is_empty(), "{} of {} texts differ:\n{}", wrong.len(), sample.len(), wrong.join("\n"));
}

#[test]
fn the_term_of_low_pressure_is_a_number_of_the_new_burner_alone_and_nought_is_refused_by_the_door() {
    for good in ["1", "20", "120"] {
        let (fire, _) = ours(&texts(&[("auto_low_pa_s", good)])).unwrap_or_else(|refused| panic!("{good} was refused: {}", refused.words()));
        assert_eq!(fire.low_pressure_s, good.parse::<f32>().unwrap());
    }
    // the default is the 20 s the record of 16-29.10 gives (its longest dip that came back was 9 s)
    assert_eq!(ours(&texts(&[])).unwrap().0.low_pressure_s, 20.0);
    for bad in ["0", "-3", "abc", ""] {
        assert!(ours(&texts(&[("auto_low_pa_s", bad)])).is_err(), "{bad:?} was taken");
    }
}
