//! The power steps as a table a person edits: seven steps, and on each a dose, a pause and a draught, kept as settings of the automaton's card; the burner burns on the step that is chosen in every hold, and the pair it
//! used to work by is gone, its numbers carried into the chosen step once. Expected from what a step is - the owner's table, 14 to 3 kW - and from what a board must keep when the image changes under it: what a person set
//! stays, what is empty is seeded, a number outside its span is refused by its name.

use burner_settings::RULES;
use std::collections::{BTreeMap, HashMap};

use thermostat::Hold;
use settings::{carry_over, change, change_all, seed, value, Setting, Store, LEFT};
use burner_settings::{thermostat_tuning, ALL};

#[derive(Default, Clone)]
struct Nvs {
    kept: BTreeMap<String, String>,
    writes: usize,
    cut_at: Option<usize>,
}

impl Store for Nvs {
    fn get(&self, name: &str) -> Option<String> {
        self.kept.get(name).cloned()
    }
    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        if self.cut_at.is_some_and(|at| self.writes + 1 >= at) {
            return Err("the power went off".into());
        }
        self.writes += 1;
        self.kept.insert(name.into(), value.into());
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<bool, String> {
        if self.cut_at.is_some_and(|at| self.writes + 1 >= at) {
            return Err("the power went off".into());
        }
        self.writes += 1;
        Ok(self.kept.remove(name).is_some())
    }
}

const STEPS: [u32; 7] = [14, 10, 8, 6, 5, 4, 3];

/// The owner's table (and the run of 27.09 for the 3 kW row): the dose, the pause, the draught.
const TABLE: [(u32, &str, &str, &str); 7] =
    [(14, "2", "8", "51"), (10, "2", "12", "36"), (8, "2", "15", "29"), (6, "2", "21", "22"), (5, "2", "25", "18"), (4, "2", "32", "15"), (3, "1", "21", "11")];

fn on(kw: u32) -> String {
    format!("auto_{kw}_on_s")
}
fn off(kw: u32) -> String {
    format!("auto_{kw}_off_s")
}
fn pa(kw: u32) -> String {
    format!("auto_{kw}_pa")
}

fn setting(name: &str) -> &'static Setting {
    ALL.iter().copied().find(|s| s.name == name).unwrap_or_else(|| panic!("no setting {name} in ALL"))
}

/// What the firmware does at every start: the carry, then the seed.
fn start(nvs: &mut Nvs) -> Vec<String> {
    let mut said = carry_over(&ALL, &RULES, nvs);
    said.extend(seed(&ALL, &RULES, nvs, &[]));
    said
}

/// A board of the image before this one: the automaton's card holds the pair it worked by, and the meaning the image before wrote.
fn old_board(hold: &str, step: &str, pair: (&str, &str, &str)) -> Nvs {
    let mut lines: Vec<String> = ALL
        .iter()
        .filter(|s| s.card == Some("automaton") && !STEPS.iter().any(|kw| [on(*kw), off(*kw), pa(*kw)].contains(&s.name.to_string())))
        .map(|s| format!("{}={}", s.name, s.default))
        .filter(|l| !l.starts_with("auto_hold=") && !l.starts_with("auto_step="))
        .collect();
    lines.push(format!("auto_hold={hold}"));
    lines.push(format!("auto_step={step}"));
    lines.push(format!("auto_pa={}", pair.0));
    lines.push(format!("auto_feed_on_s={}", pair.1));
    lines.push(format!("auto_feed_off_s={}", pair.2));
    lines.push("@meaning=2".into());
    let mut nvs = Nvs::default();
    nvs.kept.insert("s.automaton".into(), lines.join("\n"));
    nvs
}

fn texts(changed: &[(String, String)]) -> impl Fn(&Setting) -> String {
    let changed: HashMap<String, String> = changed.iter().cloned().collect();
    move |s: &Setting| changed.get(s.name).cloned().unwrap_or_else(|| s.default.to_string())
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

// ---- the table is settings ----

#[test]
fn every_step_has_a_dose_a_pause_and_a_draught_on_the_automatons_card_with_the_owners_numbers() {
    for (kw, d, p, a) in TABLE {
        for (name, want) in [(on(kw), d), (off(kw), p), (pa(kw), a)] {
            let s = setting(&name);
            assert_eq!(s.card, Some("automaton"), "{name}");
            assert_eq!(s.default, want, "{name} at {kw} kW");
        }
    }
}

#[test]
fn the_pair_the_burner_worked_by_is_not_a_setting_any_more() {
    for gone in ["auto_pa", "auto_feed_on_s", "auto_feed_off_s"] {
        assert!(!ALL.iter().any(|s| s.name == gone), "{gone} is still a setting");
    }
}

#[test]
fn a_new_board_is_seeded_with_the_whole_table_and_a_set_number_is_not_touched() {
    let mut nvs = Nvs::default();
    start(&mut nvs);
    for (kw, d, p, a) in TABLE {
        assert_eq!((value(setting(&on(kw)), &nvs), value(setting(&off(kw)), &nvs), value(setting(&pa(kw)), &nvs)), (d.into(), p.into(), a.into()), "{kw} kW");
    }
    change(&ALL, &RULES, &mut nvs, "auto_8_pa", "33").unwrap();
    start(&mut nvs);
    assert_eq!(value(setting("auto_8_pa"), &nvs), "33", "a start put a default over a number a person set");
}

#[test]
fn an_empty_number_of_the_table_is_seeded_from_the_description_and_a_kept_one_is_not() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[("auto_5_off_s", "27")]);
    assert_eq!(value(setting("auto_5_off_s"), &nvs), "27");
    seed(&ALL, &RULES, &mut nvs, &[("auto_5_off_s", "99")]);
    assert_eq!(value(setting("auto_5_off_s"), &nvs), "27", "the description overwrote what is kept");
}

// ---- the burner burns on the chosen step, in every hold ----

#[test]
fn in_every_hold_the_burner_burns_on_the_numbers_of_the_chosen_step() {
    // every number of every step different, so a number read from another step or another field cannot pass
    let mut all: Vec<(String, String)> = Vec::new();
    for (i, kw) in STEPS.iter().enumerate() {
        let i = i as u32;
        all.push((on(*kw), (3 + i).to_string()));
        all.push((off(*kw), (30 + i).to_string()));
        all.push((pa(*kw), (60 + i).to_string()));
    }
    for hold in ["power", "boiler", "return", "room"] {
        for (i, kw) in STEPS.iter().enumerate() {
            let mut set = all.clone();
            set.extend(pairs(&[("auto_hold", hold), ("auto_step", &kw.to_string())]));
            let t = thermostat_tuning(&texts(&set));
            let i = i as f32;
            for (name, step) in [("step", t.step), ("power", t.power)] {
                assert_eq!((step.feed.on_s, step.feed.off_s, step.pa), (3.0 + i, 30.0 + i, 60.0 + i), "{name} in {hold} on {kw} kW");
            }
        }
    }
}

#[test]
fn the_hold_is_still_what_the_texts_say() {
    for (text, hold) in [("power", Hold::Power), ("boiler", Hold::Boiler), ("return", Hold::Return), ("room", Hold::Boiler)] {
        assert_eq!(thermostat_tuning(&texts(&pairs(&[("auto_hold", text)]))).hold, hold, "{text}");
    }
}

#[test]
fn a_number_of_the_chosen_step_reaches_the_automaton_and_one_of_another_step_does_not() {
    let base = thermostat_tuning(&texts(&pairs(&[("auto_step", "6")])));
    let edited = thermostat_tuning(&texts(&pairs(&[("auto_step", "6"), ("auto_6_off_s", "40")])));
    assert_eq!((base.step.feed.off_s, edited.step.feed.off_s), (21.0, 40.0));
    let elsewhere = thermostat_tuning(&texts(&pairs(&[("auto_step", "6"), ("auto_8_off_s", "40"), ("auto_3_pa", "5")])));
    assert_eq!((elsewhere.step.feed.off_s, elsewhere.step.pa), (21.0, 22.0), "a step that is not chosen moved the burning");
}

#[test]
fn a_chosen_step_with_a_number_the_door_cannot_take_is_refused_by_the_field_it_lands_in() {
    for (name, bad, field) in [("auto_6_on_s", "0", "step.feed.on_s"), ("auto_6_off_s", "", "step.feed.off_s"), ("auto_6_pa", "-1", "step.pa")] {
        let t = thermostat_tuning(&texts(&pairs(&[("auto_step", "6"), (name, bad)])));
        let refused = t.sound(88.0).err().unwrap_or_else(|| panic!("{name} = {bad:?} was taken by the door"));
        assert_eq!(refused.field, field, "{name} = {bad:?}: {}", refused.words());
    }
    // the same bad number on a step nobody burns on is not the door's business
    assert!(thermostat_tuning(&texts(&pairs(&[("auto_step", "6"), ("auto_8_off_s", "")]))).sound(88.0).is_ok());
}

// ---- a person edits the table ----

#[test]
fn a_number_inside_its_span_is_taken_and_one_outside_is_refused_by_its_name() {
    let mut nvs = Nvs::default();
    start(&mut nvs);
    for (name, good) in [("auto_14_on_s", "1"), ("auto_14_on_s", "120"), ("auto_10_off_s", "600"), ("auto_3_pa", "0"), ("auto_3_pa", "300")] {
        change(&ALL, &RULES, &mut nvs, name, good).unwrap_or_else(|why| panic!("{name} = {good}: {why}"));
        assert_eq!(value(setting(name), &nvs), good);
    }
    for (name, bad) in [("auto_14_on_s", "0"), ("auto_14_on_s", "121"), ("auto_10_off_s", "0"), ("auto_10_off_s", "601"), ("auto_3_pa", "301"), ("auto_3_pa", "-1"), ("auto_3_pa", "abc"), ("auto_4_pa", "")] {
        let was = value(setting(name), &nvs);
        assert!(change(&ALL, &RULES, &mut nvs, name, bad).is_err(), "{name} = {bad:?} was taken");
        assert_eq!(value(setting(name), &nvs), was, "{name} moved on a refusal");
    }
}

#[test]
fn a_save_of_a_whole_row_is_one_and_a_bad_number_in_it_changes_nothing() {
    let mut nvs = Nvs::default();
    start(&mut nvs);
    change_all(&ALL, &RULES, &mut nvs, &[("auto_5_on_s", "3"), ("auto_5_off_s", "26"), ("auto_5_pa", "19")]).unwrap();
    assert_eq!((value(setting("auto_5_on_s"), &nvs), value(setting("auto_5_off_s"), &nvs), value(setting("auto_5_pa"), &nvs)), ("3".into(), "26".into(), "19".into()));
    assert!(change_all(&ALL, &RULES, &mut nvs, &[("auto_5_on_s", "4"), ("auto_5_off_s", "9999"), ("auto_5_pa", "20")]).is_err());
    assert_eq!(value(setting("auto_5_on_s"), &nvs), "3", "the good number of a refused save was kept");
}

#[test]
fn the_automatons_card_still_fits_the_buffer_nvs_is_read_through() {
    let mut nvs = Nvs::default();
    start(&mut nvs);
    let card = nvs.kept.get("s.automaton").expect("the card has a key");
    assert!(card.len() < 1024 - 128, "the card is {} bytes of a buffer of 1024", card.len());
}

// ---- the pair is carried into the step whose defaults it equals, once ----

/// What a step holds at the end of a start, as a person reads the table: (dose, pause, draught).
fn row_of(nvs: &Nvs, kw: u32) -> (String, String, String) {
    (value(setting(&on(kw)), nvs), value(setting(&off(kw)), nvs), value(setting(&pa(kw)), nvs))
}

fn table_row(kw: u32) -> (String, String, String) {
    let (_, d, p, a) = TABLE.iter().copied().find(|r| r.0 == kw).unwrap();
    (d.into(), p.into(), a.into())
}

#[test]
fn the_pair_that_equals_the_defaults_of_a_step_makes_that_step_the_chosen_one_whatever_was_chosen_before() {
    // the owner, 02.10, on the boiler: the old pair 1 s / 21 s / 11 Pa went into the chosen "6 kW" and made it a small fire
    for hold in ["boiler", "return", "room"] {
        for chosen in ["14", "6", "3"] {
            let mut nvs = old_board(hold, chosen, ("11", "1", "21"));
            let said = start(&mut nvs);
            assert_eq!(value(setting("auto_step"), &nvs), "3", "{hold}/{chosen}: {said:?}");
            for kw in STEPS {
                assert_eq!(row_of(&nvs, kw), table_row(kw), "{hold}/{chosen}: the row of {kw} kW is not the table's");
            }
            // what the burner burns by is what it burned by before
            let t = thermostat_tuning(&|s: &Setting| value(s, &nvs));
            assert_eq!((t.step.pa, t.step.feed.on_s, t.step.feed.off_s), (11.0, 1.0, 21.0), "{hold}/{chosen}");
        }
    }
}

#[test]
fn a_pair_that_equals_the_defaults_of_the_step_already_chosen_changes_nothing() {
    let mut nvs = old_board("boiler", "6", ("22", "2", "21"));
    start(&mut nvs);
    assert_eq!(value(setting("auto_step"), &nvs), "6");
    for kw in STEPS {
        assert_eq!(row_of(&nvs, kw), table_row(kw), "{kw} kW");
    }
}

#[test]
fn a_pair_no_step_has_as_its_defaults_goes_into_the_chosen_step_and_the_other_steps_stay_the_tables() {
    for (chosen, kw) in [("6", 6), ("3", 3), ("14", 14)] {
        let mut nvs = old_board("boiler", chosen, ("40", "3", "30"));
        start(&mut nvs);
        assert_eq!(row_of(&nvs, kw), ("3".into(), "30".into(), "40".into()), "chosen {chosen}");
        assert_eq!(value(setting("auto_step"), &nvs), chosen, "a pair of nobody's defaults moved the choice");
        for other in STEPS.iter().filter(|k| **k != kw) {
            assert_eq!(row_of(&nvs, *other), table_row(*other), "chosen {chosen}: {other} kW changed");
        }
    }
}

#[test]
fn a_step_a_person_already_set_is_not_written_over_by_a_pair_that_equals_its_defaults() {
    let mut nvs = old_board("boiler", "6", ("11", "1", "21"));
    let card = nvs.kept.get_mut("s.automaton").unwrap();
    card.push_str("\nauto_3_off_s=25");
    start(&mut nvs);
    assert_eq!(value(setting("auto_3_off_s"), &nvs), "25", "the number a person set was written over");
    assert_eq!((value(setting("auto_3_on_s"), &nvs), value(setting("auto_3_pa"), &nvs)), ("1".into(), "11".into()));
    assert_eq!(value(setting("auto_step"), &nvs), "6", "the choice moved to a step whose row is not the pair's");
    assert_eq!(row_of(&nvs, 6), table_row(6), "the pair landed in the chosen step instead");
}

#[test]
fn the_pair_a_power_hold_never_burned_by_is_not_carried() {
    let mut nvs = old_board("power", "6", ("40", "3", "30"));
    start(&mut nvs);
    assert_eq!((value(setting("auto_6_pa"), &nvs), value(setting("auto_6_on_s"), &nvs), value(setting("auto_6_off_s"), &nvs)), ("22".into(), "2".into(), "21".into()));
}

#[test]
fn the_old_names_are_put_aside_with_their_values_and_leave_the_card() {
    let mut nvs = old_board("boiler", "6", ("11", "1", "21"));
    start(&mut nvs);
    let card = nvs.kept.get("s.automaton").unwrap();
    for gone in ["auto_pa=", "auto_feed_on_s=", "auto_feed_off_s="] {
        assert!(!card.lines().any(|l| l.starts_with(gone)), "{gone} is still on the card");
    }
    let left = nvs.kept.get(LEFT).expect("nothing was put aside");
    for kept in ["auto_pa=11", "auto_feed_on_s=1", "auto_feed_off_s=21"] {
        assert!(left.lines().any(|l| l == kept), "{kept} is not under {LEFT}: {left:?}");
    }
}

#[test]
fn a_number_a_person_sets_after_the_carry_is_never_carried_over_again() {
    let mut nvs = old_board("boiler", "6", ("11", "1", "21"));
    start(&mut nvs);
    change(&ALL, &RULES, &mut nvs, "auto_3_off_s", "35").unwrap();
    start(&mut nvs);
    start(&mut nvs);
    assert_eq!(value(setting("auto_3_off_s"), &nvs), "35");
    assert_eq!(value(setting("auto_3_pa"), &nvs), "11");
    assert_eq!(value(setting("auto_step"), &nvs), "3");
}

#[test]
fn a_pair_number_this_image_does_not_take_is_left_out_of_the_carry_and_the_table_keeps_its_own() {
    let mut nvs = old_board("boiler", "6", ("11", "1", "9999"));
    start(&mut nvs);
    assert_eq!((value(setting("auto_6_pa"), &nvs), value(setting("auto_6_on_s"), &nvs), value(setting("auto_6_off_s"), &nvs)), ("11".into(), "1".into(), "21".into()));
}

#[test]
fn a_board_with_nothing_kept_carries_nothing() {
    let mut nvs = Nvs::default();
    let said = start(&mut nvs);
    assert!(!said.iter().any(|l| l.contains("carried")), "{said:?}");
}

#[test]
fn a_cut_of_the_power_at_any_write_of_the_start_ends_in_the_same_place_as_a_start_that_was_not_cut() {
    // a pair of nobody's defaults, which goes into the chosen step, and the old pair, which moves the choice
    for (step, pair) in [("5", ("17", "2", "26")), ("6", ("11", "1", "21"))] {
        let mut whole = old_board("boiler", step, pair);
        start(&mut whole);
        let want = whole.kept.clone();
        for at in 1..12 {
            let mut nvs = old_board("boiler", step, pair);
            nvs.cut_at = Some(at);
            start(&mut nvs);
            nvs.cut_at = None;
            nvs.writes = 0;
            start(&mut nvs);
            assert_eq!(nvs.kept, want, "{step}: a cut at write {at} ended in another place");
        }
    }
}

// ---- a board that ran the image with the table already: meaning 3 ----
//
// The first carry put the pair into the chosen step and set the old names aside under LEFT. A board of that image - the boiler's - holds the pair numbers in a row they were never meant for, and a start of
// this image has to move them to the step they equal - only when nothing a person did stands in the way.

/// Adds to the card of OLD the whole table at its defaults, as the seeding of an image with the table wrote it, so a key is there whether or not anybody touched it.
fn with_default_rows(nvs: &mut Nvs) {
    let card = nvs.kept.get_mut("s.automaton").unwrap();
    for (kw, d, p, a) in TABLE {
        card.push_str(&format!("\n{}={d}\n{}={p}\n{}={a}", on(kw), off(kw), pa(kw)));
    }
}

/// A board of the image after 9.14 and before this one: meaning 3, the table in the card with the CHOSEN step holding ROW (dose, pause, draught), the pair set aside under LEFT.
fn board_at_3(hold: &str, step: &str, row: (&str, &str, &str), pair: (&str, &str, &str)) -> Nvs {
    let mut nvs = old_board(hold, step, ("0", "0", "0"));
    let kw: u32 = step.parse().unwrap();
    let chosen = [on(kw), off(kw), pa(kw)];
    let card = nvs.kept.get("s.automaton").unwrap();
    // the card of the old image without the pair and without its meaning, then the table at its defaults, then the chosen row as the first carry left it
    let mut lines: Vec<String> = card.lines().filter(|l| !["auto_pa=", "auto_feed_on_s=", "auto_feed_off_s=", "@meaning="].iter().any(|n| l.starts_with(n))).map(str::to_string).collect();
    for (k, d, p, a) in TABLE {
        if k != kw {
            lines.extend([format!("{}={d}", on(k)), format!("{}={p}", off(k)), format!("{}={a}", pa(k))]);
        }
    }
    lines.extend([format!("{}={}", chosen[0], row.0), format!("{}={}", chosen[1], row.1), format!("{}={}", chosen[2], row.2), "@meaning=3".to_string()]);
    nvs.kept.insert("s.automaton".into(), lines.join("\n"));
    nvs.kept.insert(LEFT.into(), format!("auto_pa={}\nauto_feed_on_s={}\nauto_feed_off_s={}", pair.0, pair.1, pair.2));
    nvs
}

/// A person's number in the card of NVS: the line of NAME made VALUE, or added when there is none.
fn person_sets(nvs: &mut Nvs, name: &str, value: &str) {
    let card = nvs.kept.get_mut("s.automaton").unwrap();
    let mut lines: Vec<String> = card.lines().filter(|l| !l.starts_with(&format!("{name}="))).map(str::to_string).collect();
    lines.push(format!("{name}={value}"));
    *card = lines.join("\n");
}

fn meaning_of_card(nvs: &Nvs) -> String {
    nvs.kept.get("s.automaton").unwrap().lines().find(|l| l.starts_with("@meaning=")).unwrap_or("@meaning=").to_string()
}

#[test]
fn a_board_of_the_image_with_the_table_that_holds_the_old_pair_in_the_chosen_step_has_it_moved_to_the_step_it_equals() {
    // the boiler of 02.10: the pair 1 s / 21 s / 11 Pa, carried by the first start into the chosen "6 kW"
    for hold in ["boiler", "return", "room"] {
        let mut nvs = board_at_3(hold, "6", ("1", "21", "11"), ("11", "1", "21"));
        let said = start(&mut nvs);
        assert_eq!(value(setting("auto_step"), &nvs), "3", "{hold}: {said:?}");
        for kw in STEPS {
            assert_eq!(row_of(&nvs, kw), table_row(kw), "{hold}: the row of {kw} kW is not the table's");
        }
        assert_eq!(meaning_of_card(&nvs), format!("@meaning={}", burner_settings::MEANING), "{hold}");
        let left = nvs.kept.get(LEFT).expect("the pair set aside was lost");
        assert!(left.lines().any(|l| l == "auto_pa=11"), "{left:?}");
    }
}

#[test]
fn a_row_the_owner_set_by_hand_is_not_touched_even_when_it_is_near_the_pair() {
    // the boiler: "6 kW" set to 22 Pa / 1 s / 21 s by hand against the pair 11 / 1 / 21
    let mut nvs = board_at_3("boiler", "6", ("1", "21", "22"), ("11", "1", "21"));
    start(&mut nvs);
    assert_eq!(value(setting("auto_step"), &nvs), "6");
    assert_eq!(row_of(&nvs, 6), ("1".into(), "21".into(), "22".into()), "the owner's row was written over");
    assert_eq!(row_of(&nvs, 3), table_row(3));
    assert_eq!(meaning_of_card(&nvs), format!("@meaning={}", burner_settings::MEANING));
}

#[test]
fn nothing_is_moved_when_the_row_of_the_step_the_pair_equals_was_set_by_a_person_or_when_the_pair_equals_no_step() {
    let mut edited = board_at_3("boiler", "6", ("1", "21", "11"), ("11", "1", "21"));
    person_sets(&mut edited, "auto_3_off_s", "25");
    start(&mut edited);
    assert_eq!(value(setting("auto_step"), &edited), "6", "the choice moved to a row a person had set");
    assert_eq!(row_of(&edited, 6), ("1".into(), "21".into(), "11".into()));
    assert_eq!(value(setting("auto_3_off_s"), &edited), "25");
    let mut nobodys = board_at_3("boiler", "6", ("3", "30", "40"), ("40", "3", "30"));
    start(&mut nobodys);
    assert_eq!(value(setting("auto_step"), &nobodys), "6");
    assert_eq!(row_of(&nobodys, 6), ("3".into(), "30".into(), "40".into()));
    let mut already = board_at_3("boiler", "6", ("2", "21", "22"), ("22", "2", "21"));
    start(&mut already);
    assert_eq!(value(setting("auto_step"), &already), "6");
    assert_eq!(row_of(&already, 6), table_row(6));
}

#[test]
fn what_the_carry_moved_stays_where_a_person_puts_it_at_every_start_after() {
    let mut nvs = board_at_3("boiler", "6", ("1", "21", "11"), ("11", "1", "21"));
    start(&mut nvs);
    change(&ALL, &RULES, &mut nvs, "auto_step", "6").unwrap();
    change(&ALL, &RULES, &mut nvs, "auto_6_pa", "11").unwrap();
    change(&ALL, &RULES, &mut nvs, "auto_6_on_s", "1").unwrap();
    start(&mut nvs);
    start(&mut nvs);
    assert_eq!(value(setting("auto_step"), &nvs), "6", "a choice made after the carry was carried again");
    assert_eq!(row_of(&nvs, 6), ("1".into(), "21".into(), "11".into()));
}

#[test]
fn a_cut_of_the_power_at_any_write_of_the_start_of_a_board_with_the_table_ends_where_a_whole_start_does() {
    let mut whole = board_at_3("boiler", "6", ("1", "21", "11"), ("11", "1", "21"));
    start(&mut whole);
    let want = whole.kept.clone();
    for at in 1..12 {
        let mut nvs = board_at_3("boiler", "6", ("1", "21", "11"), ("11", "1", "21"));
        nvs.cut_at = Some(at);
        start(&mut nvs);
        nvs.cut_at = None;
        nvs.writes = 0;
        start(&mut nvs);
        assert_eq!(nvs.kept, want, "a cut at write {at} ended in another place");
    }
}

// ---- a key is no edit: a value that differs from the default is ----

#[test]
fn a_pair_that_equals_a_step_moves_the_choice_when_the_rows_of_the_card_lie_at_their_defaults() {
    // the seeding of an image before wrote every step's defaults as keys; a person had touched none
    let mut nvs = old_board("boiler", "6", ("11", "1", "21"));
    with_default_rows(&mut nvs);
    start(&mut nvs);
    assert_eq!(value(setting("auto_step"), &nvs), "3", "keys at their defaults were taken for a person's hand");
    for kw in STEPS {
        assert_eq!(row_of(&nvs, kw), table_row(kw), "{kw} kW");
    }
}

#[test]
fn a_pair_that_equals_no_step_lands_in_the_chosen_row_when_its_keys_lie_at_their_defaults_and_not_when_a_person_changed_one() {
    let mut defaults = old_board("boiler", "6", ("40", "3", "30"));
    with_default_rows(&mut defaults);
    start(&mut defaults);
    assert_eq!(row_of(&defaults, 6), ("3".into(), "30".into(), "40".into()), "the pair was dropped because the keys were there");
    let mut touched = old_board("boiler", "6", ("40", "3", "30"));
    with_default_rows(&mut touched);
    person_sets(&mut touched, "auto_6_pa", "23");
    start(&mut touched);
    assert_eq!(row_of(&touched, 6), ("3".into(), "30".into(), "23".into()), "the number a person set stays, the two at their defaults take the pair's");
}
