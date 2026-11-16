//! The two settings of the energy's history - the minutes per row and the months kept - and the room they take: what each takes and
//! refuses, what a person reads of it, how many kilobytes a month is. They are the Dual's, so they are tested where they live.

use std::collections::HashMap;

use bench_dualr3_api::energy::{history_room, HISTORY_EVERY, HISTORY_KEEP, SETTINGS};
use settings::{change_all, check, seed, value, words, Setting, Store, NO_RULES};

#[derive(Default)]
struct Map(HashMap<String, String>);

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

fn takes(s: &Setting, text: &str) -> String {
    check(s, text).unwrap_or_else(|why| panic!("{} refused '{text}': {why}", s.name))
}

fn refuses(s: &Setting, text: &str) -> String {
    match check(s, text) {
        Ok(v) => panic!("{} took '{text}' as '{v}'", s.name),
        Err(why) => why.to_string(),
    }
}

#[test]
fn the_history_takes_a_row_every_15_30_or_60_min_and_1_to_120_months() {
    for good in ["15", "30", "60"] {
        assert_eq!(takes(&HISTORY_EVERY, good), good);
    }
    for bad in ["", "5", "45", "1h", "-15"] {
        refuses(&HISTORY_EVERY, bad);
    }
    for good in ["1", "12", "120"] {
        assert_eq!(takes(&HISTORY_KEEP, good), good);
    }
    for bad in ["", "0", "121", "-1", "twelve", "1.5", "012"] {
        refuses(&HISTORY_KEEP, bad);
    }
}

#[test]
fn the_history_says_how_much_room_it_takes_in_kilobytes() {
    let room = history_room(60, 12, 896 * 1024);
    assert_eq!((room.month_kb, room.all_kb, room.fs_kb), (36, 432, 896));
    let room = history_room(15, 12, 896 * 1024);
    assert_eq!((room.month_kb, room.all_kb, room.fs_kb), (140, 1680, 896));
}

#[test]
fn a_row_a_minute_is_no_division_by_nothing() {
    assert_eq!(history_room(0, 1, 4096).month_kb, history_room(1, 1, 4096).month_kb, "no minutes between rows counts as one");
}

#[test]
fn a_person_reads_the_history_in_words() {
    assert_eq!(words(&HISTORY_EVERY, "60"), "1 hour");
    assert_eq!(words(&HISTORY_KEEP, "12"), "12 months");
}

#[test]
fn the_history_is_seeded_with_an_hour_and_a_year_and_a_bad_months_save_is_refused_by_its_code() {
    let mut store = Map::default();
    seed(&SETTINGS, &NO_RULES, &mut store, &[]);
    assert_eq!((value(&HISTORY_EVERY, &store), value(&HISTORY_KEEP, &store)), ("60".to_string(), "12".to_string()));
    let why = change_all(&SETTINGS, &NO_RULES, &mut store, &[("keep", "0")]).unwrap_err();
    assert_eq!(why.wire(), serde_json::json!({"code": "not_months", "name": "keep", "text": "0", "from": 1, "to": 120}));
    assert_eq!(value(&HISTORY_KEEP, &store), "12", "a refused save stores nothing");
}
