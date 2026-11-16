//! The automaton's entities as the new burner's API says them, held to the first burner's table (crates/dictionary, `burner`): the same stage, doing, clocks, organs and switches, the same
//! settings with the same limits (and the table of power steps the new burner alone has) - and, on purpose, nothing of how the page draws them. Six things differ and each is held here as what it is now: the two buttons and whether heat is wanted are the thermostat's card (tests/thermostat_table.rs); the numbers the image fixes stand in
//! the card of the automaton itself and not in a card of their own; the door's refusal is a code with the field and the numbers and not a text; it says the silence of four sensors and the first burner's card of none of them, while the return and the air are the thermostat's to say (`thermostat_table.rs`); the dictionary carries no `machine` (no card order, no rows of the summary, no table of stages, no `effect`); and no setting names the
//! card it stands on.

use serde_json::Value;

/// What the new burner says that the first one does not: how many digits a number is worth and which counts go on by themselves are the stream's, so a held-to-the-first-burner row leaves them out.
fn new() -> Value {
    let mut said: Value = serde_json::from_str(&burner_api::json("stragabases3-26.10.01.5")).unwrap();
    for t in said["things"].as_array_mut().unwrap() {
        let thing = t.as_object_mut().unwrap();
        thing.remove("decimals");
        thing.remove("counts");
    }
    said
}

fn reference() -> Value {
    serde_json::from_str(&burner_api::first_burner::json("stragabases3-26.10.01.5")).unwrap()
}

fn things_of(json: &Value, card: &str) -> Vec<Value> {
    json["things"].as_array().unwrap().iter().filter(|t| t["card"] == card).cloned().collect()
}

fn named<'a>(things: &'a [Value], name: &str, part_of: Option<&str>) -> Option<&'a Value> {
    things.iter().find(|t| t["name"] == name && t.get("part_of").and_then(|p| p.as_str()) == part_of)
}

/// The numbers the image fixes, as the first burner's dictionary lists them in its card `automaton_fixed`: name, value, unit.
fn fixed() -> Vec<(String, f64, String)> {
    things_of(&reference(), "automaton_fixed").iter().map(|t| (t["name"].as_str().unwrap().to_string(), t["fixed"].as_f64().unwrap(), t["unit"].as_str().unwrap().to_string())).collect()
}

#[test]
fn the_automatons_rows_are_the_first_burners_rows_but_for_the_numbers_the_image_fixed() {
    let (ours, theirs) = (things_of(&new(), "automaton"), things_of(&reference(), "automaton"));
    assert!(!ours.is_empty(), "the new burner has no row of the automaton");
    let fixed: Vec<String> = fixed().into_iter().map(|(name, _, _)| name).collect();
    let ours_rows: Vec<Value> = ours.iter().filter(|t| !fixed.contains(&t["name"].as_str().unwrap().to_string()) && !refusal(t) && !laid_later(t)).cloned().collect();
    let theirs_rows: Vec<Value> = theirs.iter().filter(|t| !refusal(t) && !moved_to_the_thermostat(t)).cloned().collect();
    assert_eq!(ours_rows, theirs_rows);
    assert!(ours.iter().all(|t| t.get("fixed").is_none()), "the new burner fixes a number of the automaton in its image");
}

/// A row laid on the automaton's card after the first burner, at the end of the table so that no address moves: the first burner never had it.
fn laid_later(thing: &Value) -> bool {
    matches!(thing["name"].as_str(), Some("silent_fronts" | "relief_in" | "relief_today" | "relief_limit"))
}

/// The two buttons: the first burner's automaton carries them, and the new burner's thermostat does.
fn moved_to_the_thermostat(thing: &Value) -> bool {
    matches!(thing["name"].as_str(), Some("start" | "stop"))
}

#[test]
fn the_two_buttons_are_the_thermostats_in_the_new_burner_and_the_automatons_in_the_first_one() {
    let (ours, theirs) = (things_of(&new(), "automaton"), things_of(&reference(), "automaton"));
    for name in ["start", "stop"] {
        assert!(named(&theirs, name, None).is_some(), "the first burner has no button {name}");
        assert!(named(&ours, name, None).is_none(), "the new burner's automaton still has the button {name}");
        assert!(named(&things_of(&new(), "thermostat"), name, None).is_some(), "the new burner's thermostat has no button {name}");
    }
}

/// The door's refusal: the row and its parts, which the new burner says as a code and the first one as a text.
fn refusal(thing: &Value) -> bool {
    thing["name"] == "refused" || thing.get("part_of").and_then(|p| p.as_str()) == Some("refused")
}

#[test]
fn the_doors_refusal_is_a_code_of_the_list_with_the_field_and_the_numbers_where_the_first_burner_says_a_text() {
    let (ours, theirs) = (things_of(&new(), "automaton"), things_of(&reference(), "automaton"));
    assert_eq!(named(&theirs, "refused", None).map(|t| t["kinds"].clone()), Some(serde_json::json!(["text"])), "the first burner's refusal is no longer a text");
    let row = named(&ours, "refused", None).expect("the new burner has no refusal row");
    assert_eq!(row["kinds"], serde_json::json!(["word"]));
    assert_eq!(row["words"], serde_json::json!(automaton::REFUSALS));
    for (part, kind) in [("field", "text"), ("value", "number"), ("than", "text"), ("limit", "number")] {
        let thing = named(&ours, part, Some("refused")).unwrap_or_else(|| panic!("the refusal has no part {part}"));
        assert_eq!(thing["kinds"], serde_json::json!([kind]), "{part}");
    }
}

/// The setting of the new burner that is each number the first burner fixes in its image.
const SETTING_OF: [(&str, &str); 15] = [
    ("purge_after_s", "auto_purge_end"),
    ("burn_out_min_s", "auto_out_min_s"),
    ("smoulder_s", "auto_smoulder_s"),
    ("smoulder_pa", "auto_smould_pa"),
    ("smoulder_on_s", "auto_smould_on"),
    ("smoulder_off_s", "auto_smould_off"),
    ("flue_holds_c", "auto_flue_c"),
    ("flue_cold_c", "auto_flue_cold"),
    ("flue_fall_c", "auto_flue_fall"),
    ("lit_s", "auto_lit_s"),
    ("too_hot_c", "auto_too_hot_c"),
    ("alarm_c", "auto_alarm_c"),
    ("sure_s", "auto_sure_s"),
    ("relight_hot_c", "auto_ember_c"),
    ("spin_up_s", "auto_spin_up_s"),
];

#[test]
fn every_number_the_first_burner_fixes_is_a_setting_of_the_new_one_at_the_same_value_and_unit_but_the_smouldering_the_owner_lengthened() {
    let theirs = fixed();
    assert_eq!(theirs.len(), 15, "the first burner fixes {} numbers: {theirs:?}", theirs.len());
    let served = new();
    let settings = served["settings"].as_array().expect("the dictionary lists no settings");
    for (name, value, unit) in &theirs {
        let setting_name = SETTING_OF.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("{name} has no setting named for it here")).1;
        let one = settings.iter().find(|s| s["name"] == setting_name).unwrap_or_else(|| panic!("{name} is not a setting of the new burner: no {setting_name}"));
        assert!(one.get("card").is_none(), "{setting_name} says which card of the page it stands on: {one}");
        let want = if setting_name == "auto_smoulder_s" { 900.0 } else { *value };
        assert_eq!(one["default"].as_str().and_then(|d| d.parse::<f64>().ok()), Some(want), "{setting_name}");
        // seconds say themselves as seconds, a whole number as the unit the first burner gave it
        let said = one.get("unit").and_then(|u| u.as_str()).unwrap_or("s");
        assert_eq!(said, unit.as_str(), "{setting_name}");
    }
    assert_eq!(things_of(&new(), "automaton_fixed").len(), 0, "the numbers have a card of their own again");
}

#[test]
fn the_feed_and_the_mode_are_switched() {
    let ours = things_of(&new(), "automaton");
    for name in ["feed", "semi"] {
        let one = named(&ours, name, None).unwrap_or_else(|| panic!("no switch {name}"));
        assert_eq!((&one["kinds"], &one["can"]), (&serde_json::json!(["on"]), &serde_json::json!(["switch"])), "{name}");
    }
}

#[test]
fn the_twelve_clocks_count_down_and_the_time_a_state_lasts_and_a_silence_count_up() {
    let ours = things_of(&new(), "automaton");
    let counting: Vec<&str> = ours.iter().filter(|t| t.get("clock") == Some(&Value::Bool(true))).map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(counting.len(), automaton::CLOCKS.len(), "{counting:?}");
    for name in ["for", "silent_draught", "silent_flame", "silent_flue", "silent_water"] {
        assert!(!counting.contains(&name), "{name} counts up and is no clock");
    }
}

#[test]
fn every_clock_the_automaton_counts_has_a_row_to_be_said_in_and_a_clock_it_does_not_count_has_none() {
    let json = new();
    let all = json["things"].as_array().unwrap();
    let mut seen = Vec::new();
    for what in automaton::CLOCKS {
        let at = burner_api::automaton::clock_at(what).unwrap_or_else(|| panic!("no row for the clock {what:?}"));
        assert_eq!((all[at]["card"].as_str(), all[at]["clock"].as_bool()), (Some("automaton"), Some(true)), "{what}");
        assert!(!seen.contains(&at), "two clocks say themselves in one row: {what}");
        seen.push(at);
    }
    assert_eq!(burner_api::automaton::clock_at("the moon in"), None);
}

#[test]
fn the_stage_and_the_reasons_are_the_words_the_automaton_has() {
    let ours = things_of(&new(), "automaton");
    let words = |row: &str, part: Option<&str>| -> Vec<String> { named(&ours, row, part).unwrap()["words"].as_array().unwrap().iter().map(|w| w.as_str().unwrap().to_string()).collect() };
    assert_eq!(words("stage", None), automaton::STAGES);
    assert_eq!(words("broke_at", None), automaton::STAGES);
    assert_eq!(words("why", Some("doing")), automaton::WHYS);
}

#[test]
fn the_twenty_settings_of_the_automaton_the_first_burner_has_are_its_own_with_no_card_and_no_effect_and_the_table_of_steps_and_the_numbers_of_the_fire_are_the_new_ones() {
    let find = |json: &Value, name: &str| json["settings"].as_array().unwrap().iter().find(|s| s["name"] == name).cloned();
    let table: Vec<&str> = burner_settings::STEP_TABLE.iter().flat_map(|r| [r.on_s.name, r.off_s.name, r.pa.name]).collect();
    // the mode, the setpoint, the hysteresis and the level stand on the thermostat's card now and are held to the same first burner's numbers
    let names: Vec<&str> = burner_settings::ALL.iter().filter(|s| matches!(s.card, Some("automaton" | "thermostat"))).map(|s| s.name).collect();
    let numbers_of_the_fire: Vec<&str> = SETTING_OF.iter().map(|(_, setting)| *setting).collect();
    // the levels of the way up to the setpoint are the new burner's thermostat alone: the first burner has no steps down; nor does it follow a clock or ask by a room: no profiles, no windows, no source of the demand; nor wait before a light
    let levels = [
        "auto_step_mid", "auto_step_small", "auto_mid_c", "auto_small_c", "auto_lock_min", "auto_lock_c", "auto_asked_by", "auto_room_c", "auto_room_eco_c", "auto_room_gap_c", "auto_profile", "window_1", "window_2", "window_3", "window_4", "window_5", "window_6", "auto_low_pa_s",
    ];
    assert_eq!((names.len(), table.len(), numbers_of_the_fire.len()), (63, 21, 15));
    for gone in levels {
        assert!(find(&reference(), gone).is_none(), "{gone} is a setting of the first burner");
        assert!(find(&new(), gone).is_some(), "{gone} is no setting of the new burner");
    }
    for name in names.iter().filter(|n| !table.contains(n) && !numbers_of_the_fire.contains(n) && !levels.contains(n)) {
        let mut ours = find(&new(), name).unwrap_or_else(|| panic!("no setting {name} in the new burner's dictionary"));
        let mut theirs = find(&reference(), name).unwrap();
        assert!(ours.get("card").is_none(), "{name} says which card of the page it stands on: {ours}");
        assert!(ours.get("effect").is_none(), "{name} says what its line is worked out of");
        for key in ["card", "effect"] {
            theirs.as_object_mut().unwrap().remove(key);
            ours.as_object_mut().unwrap().remove(key);
        }
        // the level the burner burns on stands in every mode: the choices of the mode say what each brings, and that is the new burner's word now
        if *name == "auto_hold" {
            continue;
        }
        assert_eq!(ours, theirs, "{name}");
    }
    // the pair is not the new burner's; each number of the table has the span of the number it replaces
    for gone in ["auto_pa", "auto_feed_on_s", "auto_feed_off_s"] {
        assert!(find(&new(), gone).is_none(), "{gone} is still a setting of the new burner");
        assert!(find(&reference(), gone).is_some());
    }
    let limits = |json: &Value, name: &str| {
        let s = find(json, name).unwrap();
        (s["unit"].clone(), s["seconds"].clone(), s["whole"].clone())
    };
    for row in &burner_settings::STEP_TABLE {
        assert_eq!(limits(&new(), row.on_s.name), limits(&reference(), "auto_feed_on_s"), "{}", row.on_s.name);
        assert_eq!(limits(&new(), row.off_s.name), limits(&reference(), "auto_feed_off_s"), "{}", row.off_s.name);
        assert_eq!(limits(&new(), row.pa.name), limits(&reference(), "auto_pa"), "{}", row.pa.name);
    }
}

#[test]
fn a_setting_of_the_automaton_is_a_row_of_the_settings_card_that_keeps_its_text() {
    let rows = things_of(&new(), "settings");
    for s in burner_settings::ALL.iter().filter(|s| s.card == Some("automaton")) {
        let one = named(&rows, s.name, None).unwrap_or_else(|| panic!("no row of the settings card for {}", s.name));
        assert_eq!(one["kinds"], serde_json::json!(["text"]), "{}", s.name);
    }
}

fn keys(json: &Value, out: &mut Vec<String>) {
    match json {
        Value::Object(map) => {
            for (k, v) in map {
                out.push(k.clone());
                keys(v, out);
            }
        }
        Value::Array(all) => all.iter().for_each(|v| keys(v, out)),
        _ => {}
    }
}

#[test]
fn the_dictionary_says_nothing_of_how_the_automaton_is_drawn() {
    let json = new();
    assert!(!things_of(&json, "automaton").is_empty(), "the dictionary has no row of the automaton to say nothing of");
    assert!(json.get("machine").is_none(), "the dictionary carries a `machine`: {}", json["machine"]);
    let mut all = Vec::new();
    keys(&json, &mut all);
    for layout in ["machine", "now", "sections", "effect", "aside", "press", "says", "cause"] {
        assert!(!all.contains(&layout.to_string()), "the dictionary carries `{layout}`, which is the page's to say");
    }
    let cards: Vec<&str> = json["cards"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    assert!(cards.contains(&"automaton") && !cards.contains(&"automaton_fixed"), "{cards:?}");
}

#[test]
fn the_automaton_stands_first_of_the_modules_cards_and_the_table_holds_together() {
    let cards: Vec<String> = new()["cards"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect();
    let at = |name: &str| cards.iter().position(|c| c == name).unwrap_or_else(|| panic!("no card {name}: {cards:?}"));
    // the board's own cards come first at every board; of the modules' the automaton leads
    assert!(at("other") < at("automaton") && at("automaton") < at("relays"), "{cards:?}");
    page::dictionary::check(&burner_api::ALL).expect("the burner's table does not hold together");
}

#[test]
fn the_three_clocks_that_may_not_have_begun_may_say_so_with_their_reason_and_the_nine_others_may_not() {
    let served: Value = serde_json::from_str(&burner_api::json("stragabases3-26.10.01.5")).unwrap();
    let ours = things_of(&served, "automaton");
    let may_not_have_begun = ["blowing_clear_ends_in", "fed_after_flame_for", "full_step_in"];
    for (_, code) in burner_api::automaton::CLOCKS {
        let one = named(&ours, code, None).unwrap_or_else(|| panic!("no row for the clock {code}"));
        assert_eq!(one["counts"], "down", "{code}: a clock that runs is counted on by the page");
        if may_not_have_begun.contains(&code) {
            assert_eq!(one["kinds"], serde_json::json!(["seconds", "unknown"]), "{code}");
            assert_eq!(one["reasons"], serde_json::json!(["not_begun"]), "{code}");
        } else {
            assert_eq!(one["kinds"], serde_json::json!(["seconds"]), "{code}: a clock that is always running says no unknown");
            assert!(one.get("reasons").is_none(), "{code}");
        }
    }
}
