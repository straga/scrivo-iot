//! A key of nvs per card: saving one module's setting leaves every other module's key alone, and the one set
//! boards kept before is split into them once - and a cut in the middle of the split loses nothing.

use burner_settings::RULES;
use std::collections::BTreeMap;

use settings::{change, change_all, key_of, seed, split, value, Setting, Store, LEFT};
use burner_settings::{ALL};

/// A store that keeps what was written, in order, and can be told to fail from a given write on - the power going
/// off between two writes, which for a boiler is an ordinary way to stop.
#[derive(Default, Clone)]
struct Nvs {
    kept: BTreeMap<String, String>,
    writes: Vec<String>,
    cut_at: Option<usize>,
}

impl Store for Nvs {
    fn get(&self, name: &str) -> Option<String> {
        self.kept.get(name).cloned()
    }

    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        if self.cut_at.is_some_and(|at| self.writes.len() + 1 >= at) {
            return Err("the power went off".into());
        }
        self.writes.push(format!("put {name}"));
        self.kept.insert(name.into(), value.into());
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<bool, String> {
        if self.cut_at.is_some_and(|at| self.writes.len() + 1 >= at) {
            return Err("the power went off".into());
        }
        self.writes.push(format!("remove {name}"));
        Ok(self.kept.remove(name).is_some())
    }
}

fn setting(name: &str) -> &'static Setting {
    ALL.iter().copied().find(|s| s.name == name).unwrap_or_else(|| panic!("no setting {name} in ALL"))
}

/// A board as it was before: every setting of ALL under the one key, some of them set by a person.
fn one_set() -> Nvs {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    change(&ALL, &RULES, &mut nvs, "draught_pa", "40").unwrap();
    change(&ALL, &RULES, &mut nvs, "div_top", "8200").unwrap();
    change(&ALL, &RULES, &mut nvs, "zone", "london").unwrap();
    let whole: Vec<String> = ALL.iter().map(|s| format!("{}={}", s.name, value(s, &nvs))).chain(["@meaning=1".into()]).collect();
    let mut before = Nvs::default();
    before.kept.insert("settings".into(), whole.join("\n"));
    before
}

// ---- one key a card ----

#[test]
fn saving_one_modules_setting_writes_that_modules_key_alone() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    let others: BTreeMap<String, String> =
        nvs.kept.iter().filter(|(k, _)| **k != key_of(setting("draught_pa").card)).map(|(k, v)| (k.clone(), v.clone())).collect();
    nvs.writes.clear();
    change(&ALL, &RULES, &mut nvs, "draught_pa", "40").unwrap();
    assert_eq!(nvs.writes, vec![format!("put {}", key_of(setting("draught_pa").card))]);
    for (key, was) in &others {
        assert_eq!(nvs.kept.get(key), Some(was), "{key} was rewritten by a save of the fan's setting");
    }
    assert_eq!(value(setting("draught_pa"), &nvs), "40");
}

#[test]
fn a_refused_value_writes_no_key_at_all() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    nvs.writes.clear();
    assert!(change_all(&ALL, &RULES, &mut nvs, &[("draught_pa", "40"), ("zone", "Atlantis")]).is_err());
    assert!(nvs.writes.is_empty(), "a save with a bad value wrote {:?}", nvs.writes);
}

#[test]
fn a_save_across_two_cards_writes_each_of_them_once() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    nvs.writes.clear();
    change_all(&ALL, &RULES, &mut nvs, &[("draught_pa", "40"), ("fan_min", "9"), ("zone", "london")]).unwrap();
    let mut wrote = nvs.writes.clone();
    wrote.sort();
    let mut want = vec![format!("put {}", key_of(setting("draught_pa").card)), format!("put {}", key_of(None))];
    want.sort();
    assert_eq!(wrote, want);
}

#[test]
fn every_key_is_one_nvs_takes() {
    for s in ALL.iter() {
        let key = key_of(s.card);
        assert!(!key.is_empty() && key.len() <= 15, "{}: the key '{key}' is not one nvs takes (1..=15 bytes)", s.name);
        assert_ne!(key, "settings", "{}: a card's key is the key of the one set boards kept before", s.name);
    }
}

// ---- the split of the one set ----

#[test]
fn the_one_set_is_split_into_the_cards_with_every_value_kept() {
    let before = one_set();
    let mut nvs = before.clone();
    let said = split(&ALL, &mut nvs);
    assert!(nvs.get("settings").is_none(), "the one set is still there after the split");
    assert_eq!(value(setting("draught_pa"), &nvs), "40");
    assert_eq!(value(setting("div_top"), &nvs), "8200");
    // the zone the one set kept is a city, and a city is read as the offset it had
    assert_eq!(value(setting("zone"), &nvs), "gmt");
    for s in ALL.iter() {
        assert!(nvs.get(&key_of(s.card)).is_some_and(|set| set.contains(&format!("{}=", s.name))), "{} was lost", s.name);
    }
    assert!(said.iter().any(|l| l.contains("split")), "the split passed in silence: {said:?}");
}

#[test]
fn a_board_with_nothing_to_split_is_not_written() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    nvs.writes.clear();
    assert!(split(&ALL, &mut nvs).is_empty());
    assert!(nvs.writes.is_empty(), "a board with no one set was written: {:?}", nvs.writes);
}

#[test]
fn a_cut_anywhere_in_the_split_loses_nothing_and_the_next_start_finishes_it() {
    let mut whole = one_set();
    split(&ALL, &mut whole);
    let steps = whole.writes.len();
    assert!(steps >= 2, "a split of one write cannot be cut in the middle: {:?}", whole.writes);
    for cut in 1..=steps {
        let mut nvs = one_set();
        nvs.cut_at = Some(cut);
        split(&ALL, &mut nvs);
        assert!(nvs.get("settings").is_some(), "cut at write {cut}: the one set was gone before every card was written");
        for s in ALL.iter() {
            assert_eq!(value(s, &nvs), value(s, &one_set()), "cut at write {cut}: {} reads another value", s.name);
        }
        nvs.cut_at = None;
        split(&ALL, &mut nvs);
        assert_eq!(nvs.kept, whole.kept, "cut at write {cut}: the next start did not finish the split the same way");
    }
}

// ---- names the list no longer has: a module taken off, a setting renamed ----

/// The one set of a board whose older image had a setting this one has not.
fn one_set_with_a_name_gone() -> Nvs {
    let mut nvs = one_set();
    let whole = nvs.kept.get("settings").unwrap().clone();
    nvs.kept.insert("settings".into(), format!("{whole}\nold_gain=42"));
    nvs
}

#[test]
fn a_name_the_list_no_longer_has_is_kept_after_the_split_and_the_log_says_where() {
    let mut nvs = one_set_with_a_name_gone();
    let said = split(&ALL, &mut nvs);
    assert!(nvs.get("settings").is_none(), "the one set is still there");
    let left = nvs.get(LEFT).unwrap_or_default();
    assert!(left.lines().any(|l| l == "old_gain=42"), "old_gain=42 was lost in the split; kept: {:?}", nvs.kept);
    assert!(said.iter().any(|l| l.contains("old_gain") && l.contains(LEFT)), "the log does not say where it went: {said:?}");
}

#[test]
fn a_cut_anywhere_in_the_split_keeps_a_name_the_list_no_longer_has() {
    let mut whole = one_set_with_a_name_gone();
    split(&ALL, &mut whole);
    for cut in 1..=whole.writes.len() {
        let mut nvs = one_set_with_a_name_gone();
        nvs.cut_at = Some(cut);
        split(&ALL, &mut nvs);
        let somewhere = ["settings", LEFT]
            .iter()
            .any(|key| nvs.get(key).is_some_and(|set| set.lines().any(|l| l == "old_gain=42")));
        assert!(somewhere, "cut at write {cut}: old_gain=42 is nowhere: {:?}", nvs.kept);
        nvs.cut_at = None;
        split(&ALL, &mut nvs);
        assert!(
            nvs.get(LEFT).is_some_and(|set| set.lines().any(|l| l == "old_gain=42")),
            "cut at write {cut}: after the next start finished the split, old_gain=42 is gone: {:?}",
            nvs.kept
        );
        assert_eq!(nvs.kept, whole.kept, "cut at write {cut}: the next start did not finish the split the same way");
    }
}

#[test]
fn a_set_with_no_name_gone_writes_nothing_beside_the_cards() {
    let mut nvs = one_set();
    split(&ALL, &mut nvs);
    assert!(nvs.get(LEFT).is_none(), "a key of what was left was written with nothing left");
    assert!(nvs.writes.iter().all(|w| !w.contains(LEFT)), "{:?}", nvs.writes);
}

#[test]
fn saving_a_card_keeps_a_name_of_that_card_the_list_no_longer_has() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    let key = key_of(setting("draught_pa").card);
    let was = nvs.kept.get(&key).unwrap().clone();
    nvs.kept.insert(key.clone(), format!("{was}\ngone_gain=7"));
    change(&ALL, &RULES, &mut nvs, "draught_pa", "40").unwrap();
    let now = nvs.kept.get(&key).unwrap();
    assert!(now.lines().any(|l| l == "gone_gain=7"), "a save of the fan's card erased gone_gain=7: {now:?}");
    assert_eq!(value(setting("draught_pa"), &nvs), "40");
}

#[test]
fn saving_a_card_with_no_name_gone_writes_only_the_lists_names() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    change(&ALL, &RULES, &mut nvs, "draught_pa", "40").unwrap();
    let key = key_of(setting("draught_pa").card);
    for line in nvs.kept.get(&key).unwrap().lines() {
        let name = line.split_once('=').map_or(line, |(n, _)| n);
        assert!(name.starts_with('@') || ALL.iter().any(|s| s.name == name), "{name} is in the card and in no list");
    }
}

// ---- what a module writes down for itself ----

#[test]
fn a_modules_memory_has_a_key_nvs_takes_and_no_setting_has() {
    let key = settings::memory_key("automaton");
    assert!(!key.is_empty() && key.len() <= 15, "'{key}' is not a key nvs takes");
    let taken: Vec<String> = ALL.iter().map(|s| key_of(s.card)).chain([LEFT.to_string(), "settings".to_string()]).collect();
    assert!(!taken.contains(&key), "the automaton's memory would be written over a key of settings: {key}");
}

#[test]
fn a_card_whose_memory_key_nvs_would_not_take_is_refused() {
    assert!(settings::memory_key_checked("automaton").is_ok());
    let why = settings::memory_key_checked("a_card_far_too_long").expect_err("a key past 15 bytes was taken");
    assert!(why.contains("a_card_far_too_long"), "{why}");
}

// ---- a setting a newer image took off a card that has its own key (auto_mode) ----

/// A board an older image seeded, whose automaton card still had `auto_mode`, set by a person to idle.
fn card_with_auto_mode() -> Nvs {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    let key = key_of(Some("automaton"));
    let was = nvs.kept.get(&key).expect("the automaton's card has a key").clone();
    nvs.kept.insert(key, format!("{was}\nauto_mode=idle"));
    nvs.writes.clear();
    nvs
}

#[test]
fn a_setting_the_image_no_longer_has_leaves_its_card_for_the_left_key_at_the_start_and_the_log_names_it() {
    let mut nvs = card_with_auto_mode();
    let said = seed(&ALL, &RULES, &mut nvs, &[]);
    let card = nvs.get(&key_of(Some("automaton"))).unwrap();
    assert!(!card.lines().any(|l| l.starts_with("auto_mode=")), "auto_mode is still in the automaton's card: {card:?}");
    assert!(
        nvs.get(LEFT).is_some_and(|left| left.lines().any(|l| l == "auto_mode=idle")),
        "auto_mode=idle was not kept under {LEFT}: {:?}",
        nvs.kept
    );
    assert!(said.iter().any(|l| l.contains("auto_mode") && l.contains(LEFT)), "the log does not name it: {said:?}");
    assert_eq!(value(setting("auto_wait"), &nvs), setting("auto_wait").default, "the card's own settings stay");
}

#[test]
fn a_start_with_no_setting_gone_from_any_card_writes_nothing_and_says_nothing_of_it() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    nvs.writes.clear();
    let said = seed(&ALL, &RULES, &mut nvs, &[]);
    assert!(nvs.writes.is_empty(), "a start with nothing to move wrote {:?}", nvs.writes);
    assert!(nvs.get(LEFT).is_none());
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn a_cut_anywhere_in_the_move_keeps_auto_mode_and_the_next_start_finishes_it() {
    let mut whole = card_with_auto_mode();
    seed(&ALL, &RULES, &mut whole, &[]);
    for cut in 1..=whole.writes.len() {
        let mut nvs = card_with_auto_mode();
        nvs.cut_at = Some(cut);
        seed(&ALL, &RULES, &mut nvs, &[]);
        let somewhere = [key_of(Some("automaton")), LEFT.to_string()]
            .iter()
            .any(|key| nvs.get(key).is_some_and(|set| set.lines().any(|l| l == "auto_mode=idle")));
        assert!(somewhere, "cut at write {cut}: auto_mode=idle is nowhere: {:?}", nvs.kept);
        nvs.cut_at = None;
        seed(&ALL, &RULES, &mut nvs, &[]);
        assert_eq!(nvs.kept, whole.kept, "cut at write {cut}: the next start did not end where one whole start ends");
    }
}

// ---- what was set aside comes back to the image that knows it again (6.6, 25.09: the roles of the sockets) ----

/// ALL without the setting NAME: an image that does not know it.
fn all_but(name: &str) -> Vec<&'static Setting> {
    ALL.iter().copied().filter(|s| s.name != name).collect()
}

/// A board a person set role_ntc_1 on, then started once by an image that does not know the roles' first socket.
fn role_set_aside() -> Nvs {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    change(&ALL, &RULES, &mut nvs, "role_ntc_1", "supply").unwrap();
    seed(&all_but("role_ntc_1"), &RULES, &mut nvs, &[]);
    nvs.writes.clear();
    nvs
}

#[test]
fn a_setting_an_image_did_not_know_comes_back_with_its_value_to_the_next_image_that_knows_it() {
    let mut nvs = role_set_aside();
    assert!(nvs.get(LEFT).is_some_and(|l| l.lines().any(|x| x == "role_ntc_1=supply")), "not set aside: {:?}", nvs.kept);
    let said = seed(&ALL, &RULES, &mut nvs, &[]);
    assert_eq!(value(setting("role_ntc_1"), &nvs), "supply", "the value a person set did not come back");
    assert!(!nvs.get(LEFT).unwrap_or_default().lines().any(|x| x.starts_with("role_ntc_1=")), "it stays set aside too");
    assert!(said.iter().any(|l| l.contains("role_ntc_1") && l.contains(LEFT)), "the log does not say it came back: {said:?}");
}

#[test]
fn a_value_the_image_knows_and_keeps_itself_is_not_overwritten_by_what_was_set_aside() {
    let mut nvs = role_set_aside();
    // the image knowing role_ntc_1 again has already kept one of its own - the person set it anew
    let key = key_of(setting("role_ntc_1").card);
    let was = nvs.kept.get(&key).cloned().unwrap_or_default();
    nvs.kept.insert(key, format!("{was}\nrole_ntc_1=boiler"));
    seed(&ALL, &RULES, &mut nvs, &[]);
    assert_eq!(value(setting("role_ntc_1"), &nvs), "boiler", "what was set aside overwrote the value kept");
    assert!(!nvs.get(LEFT).unwrap_or_default().lines().any(|x| x.starts_with("role_ntc_1=")), "the stale copy stays aside");
}

#[test]
fn a_setting_moved_to_another_card_keeps_the_value_it_had_on_the_old_one() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    change(&ALL, &RULES, &mut nvs, "poll_every", "5").unwrap();
    // the next image keeps poll_every on the fan's card: another key of nvs, while the temperatures' key stays
    let was = setting("poll_every");
    let moved: &'static Setting = Box::leak(Box::new(Setting {
        name: was.name,
        label: was.label,
        default: was.default,
        kind: settings::Kind::Seconds { from: 1, to: 60 },
        level: settings::Level::Work,
        card: Some("fan"),
    }));
    let list: Vec<&'static Setting> = ALL.iter().map(|s| if s.name == "poll_every" { moved } else { *s }).collect();
    seed(&list, &RULES, &mut nvs, &[]);
    assert_eq!(value(moved, &nvs), "5", "the value stayed under the old card's key and the new one got the default");
}

#[test]
fn a_cut_anywhere_in_bringing_it_back_loses_nothing_and_the_next_start_finishes_it() {
    let mut whole = role_set_aside();
    seed(&ALL, &RULES, &mut whole, &[]);
    assert!(!whole.writes.is_empty(), "bringing it back wrote nothing");
    for cut in 1..=whole.writes.len() {
        let mut nvs = role_set_aside();
        nvs.cut_at = Some(cut);
        seed(&ALL, &RULES, &mut nvs, &[]);
        let somewhere = [key_of(setting("role_ntc_1").card), LEFT.to_string()]
            .iter()
            .any(|k| nvs.get(k).is_some_and(|set| set.lines().any(|l| l == "role_ntc_1=supply")));
        assert!(somewhere, "a cut at write {cut} lost role_ntc_1: {:?}", nvs.kept);
        nvs.cut_at = None;
        seed(&ALL, &RULES, &mut nvs, &[]);
        assert_eq!(value(setting("role_ntc_1"), &nvs), "supply", "the start after a cut at write {cut} did not finish it");
    }
}

// ---- a kept value this image no longer takes (6.5: auto_no_flame_s from 5 to 60 s, boards keep 120) ----

/// A board an older image seeded, whose person or default left `auto_no_flame_s` at WAS.
fn card_with_no_flame(was: &str) -> Nvs {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    let key = key_of(Some("automaton"));
    let set = nvs.kept.get(&key).expect("the automaton's card has a key").clone();
    let set: Vec<String> = set
        .lines()
        .map(|l| if l.starts_with("auto_no_flame_s=") { format!("auto_no_flame_s={was}") } else { l.to_string() })
        .collect();
    nvs.kept.insert(key, set.join("\n"));
    nvs.writes.clear();
    nvs
}

#[test]
fn a_kept_value_the_image_does_not_take_is_put_aside_and_the_default_reads() {
    let mut nvs = card_with_no_flame("120");
    let said = seed(&ALL, &RULES, &mut nvs, &[]);
    let s = setting("auto_no_flame_s");
    assert_eq!(value(s, &nvs), s.default, "a kept 120 still reads");
    let card = nvs.get(&key_of(Some("automaton"))).unwrap();
    assert!(card.lines().any(|l| l == format!("auto_no_flame_s={}", s.default)), "the card was not seeded with the default: {card:?}");
    assert!(
        nvs.get(LEFT).is_some_and(|left| left.lines().any(|l| l == "auto_no_flame_s=120")),
        "auto_no_flame_s=120 was not kept under {LEFT}: {:?}",
        nvs.kept
    );
    assert!(said.iter().any(|l| l.contains("auto_no_flame_s") && l.contains("120") && l.contains(LEFT)), "the log does not name it: {said:?}");
    // the next start neither brings it back nor drops it, and writes nothing
    nvs.writes.clear();
    let said = seed(&ALL, &RULES, &mut nvs, &[]);
    assert!(nvs.writes.is_empty(), "the next start wrote {:?} ({said:?})", nvs.writes);
    assert_eq!(value(s, &nvs), s.default);
    assert!(nvs.get(LEFT).is_some_and(|left| left.lines().any(|l| l == "auto_no_flame_s=120")), "the put-aside line was dropped");
}

#[test]
fn a_kept_value_the_image_takes_stays_where_it_is() {
    let mut nvs = card_with_no_flame("30");
    seed(&ALL, &RULES, &mut nvs, &[]);
    assert_eq!(value(setting("auto_no_flame_s"), &nvs), "30");
    assert!(nvs.get(LEFT).is_none(), "a sound value was put aside: {:?}", nvs.kept);
}

#[test]
fn a_cut_anywhere_in_putting_aside_keeps_the_value_and_the_next_start_finishes_it() {
    let mut whole = card_with_no_flame("120");
    seed(&ALL, &RULES, &mut whole, &[]);
    for cut in 1..=whole.writes.len() {
        let mut nvs = card_with_no_flame("120");
        nvs.cut_at = Some(cut);
        seed(&ALL, &RULES, &mut nvs, &[]);
        let somewhere = [key_of(Some("automaton")), LEFT.to_string()]
            .iter()
            .any(|key| nvs.get(key).is_some_and(|set| set.lines().any(|l| l == "auto_no_flame_s=120")));
        assert!(somewhere, "cut at write {cut}: auto_no_flame_s=120 is nowhere: {:?}", nvs.kept);
        nvs.cut_at = None;
        seed(&ALL, &RULES, &mut nvs, &[]);
        assert_eq!(nvs.kept, whole.kept, "cut at write {cut}: the next start did not end where one whole start ends");
    }
}
