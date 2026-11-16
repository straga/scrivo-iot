//! The burner's dictionary: one table the image serves and its modules write by.

use burner_api::first_burner::{self as burner, at, ALL, TABLE};
use page::dictionary as d;
use serde_json::Value;

fn served() -> Value {
    serde_json::from_str(&burner::json("stragabases3-26.09.24.14")).unwrap()
}

/// The thing at AT of the served dictionary, as `card/name` or `card/row.name`.
fn who(json: &Value, at: usize) -> String {
    let t = &json["things"][at];
    match t["part_of"].as_str() {
        Some(row) => format!("{}/{row}.{}", t["card"].as_str().unwrap(), t["name"].as_str().unwrap()),
        None => format!("{}/{}", t["card"].as_str().unwrap(), t["name"].as_str().unwrap()),
    }
}

#[test]
fn the_burners_description_holds_together() {
    assert_eq!(d::check(&ALL), Ok(()));
}

#[test]
fn every_address_a_module_writes_by_is_the_thing_the_served_dictionary_names_there() {
    let json = served();
    assert_eq!(json["things"].as_array().unwrap().len(), TABLE.len());
    let named = [
        (at::STAGE, "automaton/stage"),
        (at::DOING_WHY, "automaton/doing.why"),
        (at::PUMP_BY_WATER, "automaton/pump_by.water"),
        (at::RELAY[2], "relays/pump"),
        (at::RELAY_OFF_IN[1].unwrap(), "relays/igniter.off_in"),
        (at::CHANNEL[6], "temperatures/ntc_7"),
        (at::CHANNEL_ROLE[0], "temperatures/ntc_1.role"),
        (at::BUS_LAST_SILENT, "temperatures/bus.last_silent"),
        (at::FLUE_TEMP, "flue/flue_temp"),
        (at::HOLDING_CAME_OUT, "fan/holding.came_out"),
        (at::FAN_DRAUGHT_SLIDER, "fan/draught.slider"),
        (at::GATE_JUDGED, "fan/gate.judged"),
        (at::BLAST_PRESSURE, "draught/blast_pressure"),
        (at::DRAUGHT_PIN[3], "draught/pin.window"),
        (at::FLAME_PIN[0], "flame/pin"),
        (at::DIVIDER_ZERO_AT, "settings/div_bottom.zero_at"),
        (dictionary::system::WIFI_SIGNAL, "system/wifi.signal"),
    ];
    for (address, thing) in named {
        assert_eq!(who(&json, address), thing, "address {address}");
    }
    for (address, entry) in TABLE.iter().enumerate().filter(|(_, e)| e.part_of.is_none()) {
        assert_eq!(d::address(&TABLE, entry.card, entry.name), Some(address));
    }
}

#[test]
fn a_thing_the_burner_has_not_got_has_no_address() {
    assert_eq!(d::address(&TABLE, "relays", "fan"), None);
    assert_eq!(d::address(&TABLE, "meter", "voltage"), None, "the Sonoff's row is no row of the burner");
    assert_eq!(burner::clock_at("tea is ready in"), None);
}

#[test]
fn the_automaton_is_pressed_to_start_and_to_stop_and_to_nothing_else() {
    // there is no quick run, so no button and no row says one is going
    let json = served();
    let things = json["things"].as_array().unwrap();
    let of_automaton = |t: &&Value| t["card"] == "automaton" && t["part_of"].is_null();
    let pressed: Vec<&str> = things
        .iter()
        .filter(of_automaton)
        .filter(|t| t["can"].as_array().is_some_and(|can| can.iter().any(|c| c == "press")))
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(pressed, ["start", "stop"]);
    assert!(!things.iter().filter(of_automaton).any(|t| t["name"] == "run"), "a row says which run is going");
}

#[test]
fn start_and_stop_are_pressed_by_their_addresses() {
    for name in ["start", "stop"] {
        let at = d::address(&TABLE, "automaton", name).unwrap();
        assert_eq!(d::command(&TABLE, "press", &at.to_string(), ""), Ok((at, d::Command::Press)), "{name}");
    }
}

#[test]
fn every_clock_of_the_automaton_has_its_address() {
    for what in automaton::CLOCKS {
        let at = burner::clock_at(what).unwrap_or_else(|| panic!("the clock '{what}' has no address"));
        // a clock whose thing may not have begun is also unknown, with its reason
        assert!(TABLE[at].kinds == [d::Kind::Seconds] || TABLE[at].kinds == [d::Kind::Seconds, d::Kind::Unknown], "{what}");
    }
}

#[test]
fn every_setting_of_the_burner_has_the_address_its_value_goes_by() {
    for s in burner::SETTINGS {
        assert!(d::address(&TABLE, page::SETTINGS, s.name).is_some(), "{}", s.name);
    }
    let json = served();
    let names: Vec<&str> = json["settings"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, burner::SETTINGS.iter().map(|s| s.name).collect::<Vec<_>>());
}

// ---- every setting the dictionary lists is one the board keeps ----

/// The settings the burner registers, put together the way it does: the clock's two (crates/firmware, `run`), then
/// every module's own - each module of the burner answers `settings_of(its card)`.
fn kept_by_the_burner() -> Vec<&'static str> {
    let mut kept = vec![settings::TIME.name, settings::ZONE.name];
    for card in burner::CARDS {
        kept.extend(burner::settings_of(card).iter().map(|s| s.name));
    }
    kept
}

#[test]
fn every_setting_the_dictionary_lists_is_kept_by_the_burner() {
    // a setting the page offers and no module keeps is read as its default and refused on save (6.6, 25.09: the
    // roles of the sockets read "free" on the burner, the pump ran for want of the water's number)
    let kept = kept_by_the_burner();
    let unkept: Vec<&str> = burner::SETTINGS.iter().map(|s| s.name).filter(|n| !kept.contains(n)).collect();
    assert_eq!(unkept, Vec::<&str>::new(), "settings of the dictionary that no module of the burner keeps");
}

#[test]
fn no_setting_is_kept_twice_or_kept_without_the_dictionary() {
    let kept = kept_by_the_burner();
    let twice: Vec<&str> = kept.iter().copied().filter(|n| kept.iter().filter(|k| k == &n).count() > 1).collect();
    assert_eq!(twice, Vec::<&str>::new(), "settings two modules keep");
    let foreign: Vec<&str> = kept.iter().copied().filter(|n| !burner::SETTINGS.iter().any(|s| s.name == *n)).collect();
    assert_eq!(foreign, Vec::<&str>::new(), "settings kept that the dictionary does not list");
}

#[test]
fn the_first_burner_lists_the_automatons_card_but_for_the_table_of_steps_and_the_numbers_of_the_fire_it_has_not_and_the_pair_it_alone_has() {
    // a setting the burner does not list is not seeded, not kept and not on its page: the automaton reads its default and a
    // person cannot turn it (the mode and its numbers were on the card and not in this list). The table of steps
    // is the new burner's alone, and the pair the fire was fed by in the modes by a temperature the first one's.
    // The numbers of the fire are settings of the new burner's alone: the first one fixes them in its image, as its card `automaton_fixed` says.
    const FIRE: [&str; 15] = [
        "auto_purge_end", "auto_out_min_s", "auto_smoulder_s", "auto_smould_pa", "auto_smould_on", "auto_smould_off", "auto_flue_c", "auto_flue_cold", "auto_flue_fall", "auto_lit_s", "auto_too_hot_c", "auto_alarm_c", "auto_sure_s", "auto_ember_c", "auto_spin_up_s",
    ];
    // the term of the low pressure is a number the new burner has alone: the first one has no such state
    const NEW_ALONE: [&str; 1] = ["auto_low_pa_s"];
    let listed: Vec<&str> = burner::SETTINGS.iter().map(|s| s.name).collect();
    let table: Vec<&str> = burner_settings::STEP_TABLE.iter().flat_map(|r| [r.on_s.name, r.off_s.name, r.pa.name]).collect();
    assert!(FIRE.iter().all(|n| !listed.contains(n)), "the first burner lists a number it fixes in its image");
    let missing: Vec<&str> = burner_settings::ALL
        .iter()
        .filter(|s| s.card == Some("automaton") && !table.contains(&s.name) && !FIRE.contains(&s.name) && !NEW_ALONE.contains(&s.name) && !listed.contains(&s.name))
        .map(|s| s.name)
        .collect();
    assert_eq!(missing, Vec::<&str>::new(), "settings of the automaton's card the burner's dictionary does not list");
    assert!(table.iter().all(|n| !listed.contains(n)), "the first burner lists a step of the table");
    for pair in ["auto_pa", "auto_feed_on_s", "auto_feed_off_s"] {
        assert!(listed.contains(&pair), "the first burner lost {pair}");
    }
}

#[test]
fn the_dictionary_folds_the_automatons_tuning_and_nothing_of_another_card() {
    // the level is a property of the setting, and the page reads it from here - not from where it draws it
    let dictionary = served();
    let settings = dictionary["settings"].as_array().unwrap();
    let level = |name: &str| settings.iter().find(|s| s["name"] == name).unwrap_or_else(|| panic!("no setting {name}")).get("level").cloned();
    for name in ["auto_hold", "auto_hold_c", "auto_drop_c", "auto_step"] {
        assert_eq!(level(name), None, "{name} is what a person turns from day to day");
    }
    for name in ["auto_purge", "auto_pa", "auto_pump_on_c", "auto_pump_off_c", "auto_relight_3"] {
        assert_eq!(level(name), Some(Value::from("tuning")), "{name}");
    }
    let folded_elsewhere: Vec<&str> =
        settings.iter().filter(|s| s["card"] != "automaton" && s.get("level").is_some()).map(|s| s["name"].as_str().unwrap()).collect();
    assert!(folded_elsewhere.is_empty(), "{folded_elsewhere:?}");
}

// ---- what the page knew by name, said by the board ----
//
// The page does not have to know which numbers belong to which mode, and which sensor a mode needs a socket to play, by the names of
// the settings (`auto_hold_c`, `role_ntc_<n>`). The board says it in its dictionary: at each choice of the mode, the settings
// that matter while it is the one set and the sockets whose role it needs. The expectations are what the page draws:
// a level of power holds no temperature and burns on its level; a mode by a sensor holds that temperature and its pair.

fn setting<'a>(dictionary: &'a Value, name: &str) -> &'a Value {
    dictionary["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).unwrap_or_else(|| panic!("no setting {name}"))
}

fn choice<'a>(setting: &'a Value, value: &str) -> &'a Value {
    setting["choices"].as_array().unwrap().iter().find(|c| c["value"] == value).unwrap_or_else(|| panic!("no choice {value}"))
}

fn names(list: Option<&Value>) -> Vec<String> {
    let mut out: Vec<String> = list.map(|l| l.as_array().unwrap().iter().map(|n| n.as_str().unwrap().to_string()).collect()).unwrap_or_default();
    out.sort();
    out
}

#[test]
fn the_dictionary_says_which_settings_matter_with_each_mode() {
    let served = served();
    let mode = setting(&served, "auto_hold");
    // the level stands in every mode: no mode lists it, and the one that holds no temperature lists nothing
    assert_eq!(names(choice(mode, "power").get("shows")), Vec::<String>::new());
    for by_a_sensor in ["boiler", "return"] {
        assert_eq!(names(choice(mode, by_a_sensor).get("shows")), ["auto_drop_c", "auto_hold_c"], "{by_a_sensor}");
    }
}

#[test]
fn a_mode_by_a_sensor_says_which_role_a_socket_must_play_and_a_level_of_power_says_none() {
    let served = served();
    let mode = setting(&served, "auto_hold");
    assert!(choice(mode, "power").get("needs").is_none(), "a level of power holds no sensor");
    let sockets = ["role_ntc_1", "role_ntc_2", "role_ntc_3", "role_ntc_4", "role_ntc_5", "role_ntc_6", "role_ntc_7"];
    for role in ["boiler", "return"] {
        let needs = &choice(mode, role)["needs"];
        assert_eq!(needs["holds"], role, "{role}");
        assert_eq!(names(needs.get("among")), sockets, "{role}");
    }
}

#[test]
fn a_choice_that_bears_on_nothing_says_nothing() {
    // the rest of the dictionary is as it was: no choice of any other setting carries either field
    let served = served();
    for s in served["settings"].as_array().unwrap().iter().filter(|s| s["name"] != "auto_hold") {
        for c in s.get("choices").map(|c| c.as_array().unwrap().clone()).unwrap_or_default() {
            assert!(c.get("shows").is_none() && c.get("needs").is_none(), "{} / {}", s["name"], c["value"]);
        }
    }
}

#[test]
fn what_a_choice_shows_is_a_setting_of_its_own_card_and_what_it_needs_is_a_setting_that_offers_that_role() {
    let served = served();
    let all = served["settings"].as_array().unwrap();
    for s in all {
        for c in s.get("choices").map(|c| c.as_array().unwrap().clone()).unwrap_or_default() {
            for shown in names(c.get("shows")) {
                let other = setting(&served, &shown);
                assert_eq!(other["card"], s["card"], "{} / {} shows {shown}, of another card", s["name"], c["value"]);
            }
            if let Some(needs) = c.get("needs") {
                for socket in names(needs.get("among")) {
                    let offered: Vec<&str> = setting(&served, &socket)["choices"].as_array().unwrap().iter().map(|c| c["value"].as_str().unwrap()).collect();
                    assert!(offered.contains(&needs["holds"].as_str().unwrap()), "{socket} never holds {}", needs["holds"]);
                }
            }
        }
    }
}

#[test]
fn the_dictionary_marks_the_rows_that_count_down_and_no_others() {
    // the page need not know the twelve clocks by their names; the board says which of its rows count down to something. The time
    // the stage has lasted and the silence of a sensor count UP and are no clocks.
    let served = served();
    let things = served["things"].as_array().unwrap();
    let marked: Vec<&str> = things.iter().filter(|t| t["clock"] == true).map(|t| t["name"].as_str().unwrap()).collect();
    let mut want: Vec<&str> = burner::CLOCKS.iter().map(|(_, code)| *code).collect();
    want.sort();
    let mut got = marked.clone();
    got.sort();
    assert_eq!(got, want);
    assert!(things.iter().filter(|t| t["clock"] == true).all(|t| t["card"] == "automaton" && (t["kinds"] == serde_json::json!(["seconds"]) || t["kinds"] == serde_json::json!(["seconds", "unknown"]))));
    for count_up in ["for", "silent_draught", "silent_flame", "silent_flue", "silent_water"] {
        let t = things.iter().find(|t| t["card"] == "automaton" && t["name"] == count_up).unwrap_or_else(|| panic!("no row {count_up}"));
        assert!(t.get("clock").is_none(), "{count_up} counts up and is no clock");
    }
}

// ---- the automaton, as the board says it is ----
//
// The page knew the automaton by name: its card, the row that says the stage, the one that says what it does, where a fault happened,
// the buttons pressed under the scheme, and which numbers stand in which stage of its table. The board says it: `machine` at the top
// of its dictionary.

#[test]
fn the_dictionary_names_the_automaton_and_the_rows_that_say_what_it_does() {
    let machine = &served()["machine"];
    assert_eq!(machine["card"], "automaton");
    assert_eq!(machine["fixed"], "automaton_fixed", "the card of the numbers the image fixes");
    assert_eq!(machine["stage"], "stage");
    assert_eq!(machine["says"], "doing");
    assert_eq!(machine["cause"], "broke_at");
    assert_eq!(names(machine.get("press")), ["start", "stop"]);
    assert_eq!(names(machine.get("aside")), ["organs"], "what the automaton commands, for the scheme, is no row of its card's list");
}

#[test]
fn the_automatons_numbers_stand_by_stage_as_the_page_drew_them_in_6_7() {
    let machine = &served()["machine"];
    let sections: Vec<(&str, Vec<Vec<&str>>)> = machine["sections"]
        .as_array()
        .expect("sections")
        .iter()
        .map(|s| {
            let items = s["items"].as_array().unwrap().iter().map(|i| i.as_array().unwrap().iter().map(|n| n.as_str().unwrap()).collect()).collect();
            (s["name"].as_str().unwrap(), items)
        })
        .collect();
    let one = |name: &'static str| vec![name];
    assert_eq!(
        sections,
        vec![
            ("purge", vec![one("auto_purge")]),
            (
                "lighting",
                vec![
                    one("auto_dose"),
                    one("auto_igniter"),
                    one("auto_kindle_pa"),
                    one("auto_wait"),
                    one("auto_tries"),
                    one("auto_dose_again"),
                    one("auto_no_flame_s"),
                    one("lit_s"),
                    vec!["auto_relight_1", "auto_relight_2", "auto_relight_3"],
                    one("relight_hot_c"),
                ]
            ),
            ("rising", vec![one("auto_rise_s"), one("auto_kindle_pa"), one("auto_pa")]),
            (
                "working",
                vec![one("auto_hold"), one("auto_step"), one("auto_pa"), one("auto_feed_on_s"), one("auto_feed_off_s"), one("auto_hold_c"), one("auto_drop_c")]
            ),
            ("burning_off", vec![one("auto_pa"), one("burn_out_min_s"), one("auto_burn_off_s"), one("purge_after_s")]),
            ("pump", vec![one("auto_pump_on_c"), one("auto_pump_off_c"), one("too_hot_c")]),
            ("flame", vec![one("flame_lit"), one("flame_steady")]),
            ("gives_up", vec![one("auto_tries"), one("flue_cold_c"), one("flue_fall_c"), one("auto_flue_win_s"), one("too_hot_c"), one("flue_holds_c")]),
        ]
    );
}

#[test]
fn every_name_the_machine_gives_is_a_row_of_its_card_or_a_setting_or_a_number_of_the_image() {
    let served = served();
    let Some(machine) = served.get("machine") else { return };
    let things = served["things"].as_array().unwrap();
    let row = |card: &str, name: &str| things.iter().find(|t| t["card"] == card && t["name"] == name && t["part_of"].is_null());
    let card = machine["card"].as_str().unwrap();
    for one in ["stage", "says", "cause"] {
        let name = machine[one].as_str().unwrap();
        assert!(row(card, name).is_some(), "{one}: '{name}' is no row of {card}");
    }
    for name in names(machine.get("aside")).into_iter().chain(names(machine.get("press"))) {
        assert!(row(card, &name).is_some(), "'{name}' is no row of {card}");
    }
    for name in names(machine.get("press")) {
        assert!(row(card, &name).unwrap()["can"].as_array().unwrap().iter().any(|c| c == "press"), "'{name}' is pressed by nothing");
    }
    let fixed = machine["fixed"].as_str().unwrap();
    for section in machine["sections"].as_array().unwrap() {
        for item in section["items"].as_array().unwrap() {
            for name in names(Some(item)) {
                let known = setting_names(&served).contains(&name) || row(fixed, &name).is_some();
                assert!(known, "{}: '{name}' is neither a setting nor a number of {fixed}", section["name"]);
            }
        }
    }
}

fn setting_names(served: &Value) -> Vec<String> {
    served["settings"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap().to_string()).collect()
}

#[test]
fn only_the_working_stage_says_which_setting_decides_its_numbers() {
    // the draught and the auger's pair stand in the rising and the burning off whatever the burner holds, and in the working row only
    // while it holds a temperature: what the mode decides is that one stage's numbers, and the dictionary says which setting decides them
    let machine = &served()["machine"];
    let by = |name: &str| machine["sections"].as_array().unwrap().iter().find(|s| s["name"] == name).unwrap_or_else(|| panic!("no stage {name}")).get("by").cloned();
    assert_eq!(by("working"), Some(Value::from("auto_hold")));
    let others: Vec<&str> = machine["sections"].as_array().unwrap().iter().filter(|s| s["name"] != "working" && s.get("by").is_some()).map(|s| s["name"].as_str().unwrap()).collect();
    assert!(others.is_empty(), "{others:?} name a setting that decides their numbers");
}

// ---- what the card Now is made of and what a knob's line is worked out of are the board's to say ----
//
// They lay in the page's table of texts (`now`, `numbers`, a setting's `line`) as names of the board's rows and settings: the page took its
// plan of the board from the words. The board says them; the page keeps the words and what it chooses to show.

#[test]
fn the_dictionary_lists_the_rows_the_card_now_is_made_of_in_the_order_they_stand() {
    let machine = &served()["machine"];
    let now = machine["now"].as_array().unwrap_or_else(|| panic!("the machine names no rows of the card Now: {machine}"));
    let want = serde_json::json!([
        {"card": "relays", "row": "auger"},
        {"card": "relays", "row": "igniter"},
        {"card": "relays", "row": "pump"},
        {"card": "fan", "row": "gate"},
        {"card": "draught", "row": "blast_pressure"},
        {"card": "flame", "row": "flame", "with": "pin"},
        {"card": "temperatures", "part": "role", "is": "boiler"},
        {"card": "temperatures", "part": "role", "is": "supply"},
        {"card": "flue", "row": "flue_temp"},
    ]);
    assert_eq!(Value::Array(now.clone()), want);
}

#[test]
fn every_row_the_card_now_names_is_a_row_of_the_board_and_a_role_it_names_is_one_a_sensor_plays() {
    let served = served();
    let things = served["things"].as_array().unwrap();
    let Some(now) = served["machine"]["now"].as_array() else { panic!("the machine names no rows of the card Now") };
    let row = |card: &str, name: &str| things.iter().find(|t| t["card"] == card && t["name"] == name && t["part_of"].is_null());
    for one in now {
        let card = one["card"].as_str().unwrap();
        if let Some(name) = one["row"].as_str() {
            assert!(row(card, name).is_some(), "'{card}/{name}' is no row of the board");
        } else {
            let (part, is) = (one["part"].as_str().unwrap(), one["is"].as_str().unwrap());
            let plays = things.iter().any(|t| t["card"] == card && t["name"] == part && !t["part_of"].is_null() && names(t.get("words")).iter().any(|w| w == is));
            assert!(plays, "no row of {card} has a part '{part}' that can hold '{is}'");
        }
        if let Some(name) = one["with"].as_str() {
            assert!(row(card, name).is_some(), "'{card}/{name}' beside a row of Now is no row of the board");
        }
    }
}

#[test]
fn the_knobs_of_the_fans_regulator_say_which_settings_and_numbers_their_lines_are_worked_out_of() {
    let served = served();
    let effect = |name: &str| setting(&served, name).get("effect").cloned().unwrap_or_else(|| panic!("the setting {name} says no effect"));
    let steep = serde_json::json!(["fan", "holding", "steepness"]);
    for name in ["up_kp", "up_ki", "down_kp", "down_ki"] {
        assert_eq!(effect(name), serde_json::json!({"kind": "gain", "uses": {"steep": steep}}), "{name}");
    }
    assert_eq!(effect("fan_step"), serde_json::json!({"kind": "ramp", "uses": {"step": "fan_step", "min": "fan_min", "max": "fan_max"}}));
    assert_eq!(
        effect("fan_band"),
        serde_json::json!({"kind": "settle", "uses": {"band": "fan_band", "jump": ["fan", "holding", "answer_jump"], "s": ["fan", "holding", "answer_s"], "dip": ["fan", "holding", "answer_dip"]}})
    );
    assert_eq!(
        effect("fan_band_share"),
        serde_json::json!({"kind": "share", "uses": {"steep": steep, "band": "fan_band", "share": "fan_band_share", "up": "up_ki", "down": "down_ki"}})
    );
}

#[test]
fn every_setting_and_number_an_effect_names_is_one_the_board_has_and_a_setting_that_says_none_says_none() {
    let served = served();
    let things = served["things"].as_array().unwrap();
    let all = setting_names(&served);
    let mut said = 0;
    for s in served["settings"].as_array().unwrap() {
        let Some(effect) = s.get("effect") else { continue };
        said += 1;
        for (role, used) in effect["uses"].as_object().unwrap() {
            match used {
                Value::String(name) => assert!(all.contains(&name.to_string()), "{}: {role} names '{name}', which is no setting", s["name"]),
                Value::Array(at) => {
                    let (card, row, part) = (at[0].as_str().unwrap(), at[1].as_str().unwrap(), at[2].as_str().unwrap());
                    let found = things.iter().any(|t| t["card"] == card && t["part_of"] == row && t["name"] == part);
                    assert!(found, "{}: {role} names {card}/{row}.{part}, which is no number of the board", s["name"]);
                }
                other => panic!("{}: {role} is {other}", s["name"]),
            }
        }
    }
    assert_eq!(said, 7, "the fan's regulator has seven knobs whose line is worked out of numbers");
    assert!(setting(&served, "auto_hold").get("effect").is_none(), "a setting with no line to work out says no effect");
}
