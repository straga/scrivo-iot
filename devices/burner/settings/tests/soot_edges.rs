//! The two edges of the soot scale after the count moved to one window: a clean boiler stands at 36 degrees over the water now and a dirty one at 51, and what a board holds under the old numbers comes to the new ones.
//! Expected from what a person on the board sees: a board that holds the old factory number of an edge reads the new one after the image, whoever put it there; a number that is not the old factory one stays; a new board reads the new numbers.

use burner_settings::{ALL, RULES, SOOT_CLEAN_C, SOOT_DIRTY_C};
use settings::{carry_over, change, seed, value, Setting, Store};
use std::collections::BTreeMap;

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

/// What the firmware does at every start: the carry, then the seed.
fn start(nvs: &mut Nvs) -> Vec<String> {
    let mut said = carry_over(&ALL, &RULES, nvs);
    said.extend(seed(&ALL, &RULES, nvs, &[]));
    said
}

/// A board of the image before the window (the meaning 4 of the settings): the soot card holds the two edges as that image seeded or a person left them.
fn old_board(clean: &str, dirty: &str) -> Nvs {
    let mut nvs = Nvs::default();
    nvs.0.insert("s.soot".into(), format!("soot_clean_c={clean}\nsoot_dirty_c={dirty}\n@meaning=4"));
    nvs
}

fn edges(nvs: &Nvs) -> (String, String) {
    (value(&SOOT_CLEAN_C, nvs), value(&SOOT_DIRTY_C, nvs))
}

fn setting(s: &Setting) -> &Setting {
    s
}

#[test]
fn the_scale_starts_from_36_and_51() {
    assert_eq!((SOOT_CLEAN_C.default, SOOT_DIRTY_C.default), ("36", "51"));
    assert_eq!(setting(&SOOT_CLEAN_C).card, Some("soot"));
}

#[test]
fn a_new_board_reads_the_new_edges() {
    let mut nvs = Nvs::default();
    start(&mut nvs);
    assert_eq!(edges(&nvs), ("36".into(), "51".into()));
}

#[test]
fn a_board_that_holds_the_old_defaults_reads_the_new_edges_after_the_image() {
    let mut nvs = old_board("40", "60");
    let said = start(&mut nvs);
    assert_eq!(edges(&nvs), ("36".into(), "51".into()));
    assert!(said.iter().any(|l| l.contains("soot_clean_c")) && said.iter().any(|l| l.contains("soot_dirty_c")), "the journal says what was moved: {said:?}");
}

#[test]
fn an_edge_that_is_not_the_old_factory_number_stays_and_the_one_that_is_comes_to_the_new_number() {
    let mut nvs = old_board("40", "58");
    start(&mut nvs);
    assert_eq!(edges(&nvs), ("36".into(), "58".into()));
    let mut nvs = old_board("38", "60");
    start(&mut nvs);
    assert_eq!(edges(&nvs), ("38".into(), "51".into()));
}

#[test]
fn both_edges_that_are_not_the_old_factory_numbers_stay() {
    let mut nvs = old_board("38", "58");
    start(&mut nvs);
    assert_eq!(edges(&nvs), ("38".into(), "58".into()));
}

#[test]
fn the_old_numbers_typed_after_the_image_are_a_choice_and_stay() {
    let mut nvs = old_board("40", "60");
    start(&mut nvs);
    change(&ALL, &RULES, &mut nvs, "soot_clean_c", "40").unwrap();
    change(&ALL, &RULES, &mut nvs, "soot_dirty_c", "60").unwrap();
    start(&mut nvs);
    assert_eq!(edges(&nvs), ("40".into(), "60".into()), "a second start does not carry again what the first one carried");
}

#[test]
fn the_edges_are_the_only_numbers_the_move_touches() {
    let mut nvs = old_board("40", "60");
    nvs.0.insert("s.pellets".into(), "pellet_rate=250\n@meaning=4".into());
    start(&mut nvs);
    assert_eq!(value(ALL.iter().copied().find(|s| s.name == "pellet_rate").unwrap(), &nvs), "250");
}

// ---- the contract of the carry: the number of the old factory is the factory's, whoever typed it ----

#[test]
fn a_person_who_set_38_and_set_it_back_to_40_reads_36_after_the_image() {
    // the store keeps a name and a value and no mark of a hand: the old image's save of 40 over 38 is a plain overwrite
    let mut nvs = old_board("38", "60");
    let set = nvs.0.get_mut("s.soot").unwrap();
    *set = set.replace("soot_clean_c=38", "soot_clean_c=40");
    assert_eq!(nvs.0, old_board("40", "60").0, "nothing in the store tells this board from one the seeding wrote");
    start(&mut nvs);
    assert_eq!(edges(&nvs), ("36".into(), "51".into()), "the contract, not a defect: the old 40 meant the flue over the water of the whole burn");
}

#[test]
fn a_person_who_set_55_and_set_it_back_to_60_reads_51_after_the_image() {
    let mut nvs = old_board("40", "55");
    let set = nvs.0.get_mut("s.soot").unwrap();
    *set = set.replace("soot_dirty_c=55", "soot_dirty_c=60");
    assert_eq!(nvs.0, old_board("40", "60").0);
    start(&mut nvs);
    assert_eq!(edges(&nvs), ("36".into(), "51".into()));
}

#[test]
fn an_edge_set_to_38_stays_38_and_one_set_to_55_stays_55() {
    let mut nvs = old_board("38", "55");
    start(&mut nvs);
    assert_eq!(edges(&nvs), ("38".into(), "55".into()));
}
