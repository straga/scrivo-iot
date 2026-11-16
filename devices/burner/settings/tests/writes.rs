//! What `status` says the flash was written with: a person's settings and a module's memory are two counts, so
//! a start that only wrote down the automaton's state does not read as a start that changed a setting.

use burner_settings::RULES;
use std::collections::BTreeMap;

use settings::{change, memory_key, seed, Store, Writes};
use burner_settings::{ALL};

/// A store that counts every write the way the board does, by the key written.
#[derive(Default)]
struct Nvs {
    kept: BTreeMap<String, String>,
    writes: Writes,
}

impl Store for Nvs {
    fn get(&self, name: &str) -> Option<String> {
        self.kept.get(name).cloned()
    }

    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.writes.count(name);
        self.kept.insert(name.into(), value.into());
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<bool, String> {
        self.writes.count(name);
        Ok(self.kept.remove(name).is_some())
    }
}

/// A board already seeded by an earlier start, its counts back at nought as after a restart.
fn seeded() -> Nvs {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    nvs.writes = Writes::default();
    nvs
}

#[test]
fn a_start_that_changes_nothing_writes_no_setting_and_says_the_memory_apart() {
    let mut nvs = seeded();
    // the start: the settings are seeded again, then the automaton writes down the state it came up in
    seed(&ALL, &RULES, &mut nvs, &[]);
    nvs.put(&memory_key("automaton"), "stop").unwrap();
    assert_eq!(nvs.writes, Writes { settings: 0, memory: 1 });
    assert_eq!(
        nvs.writes.lines(),
        ["settings written to flash: 0 since start".to_string(), "modules' memory written to flash: 1 since start".to_string()]
    );
}

#[test]
fn a_changed_setting_is_counted_as_a_setting_and_not_as_memory() {
    let mut nvs = seeded();
    change(&ALL, &RULES, &mut nvs, "draught_pa", "40").unwrap();
    assert_eq!(nvs.writes, Writes { settings: 1, memory: 0 });
}

#[test]
fn a_card_whose_name_starts_like_memory_is_still_a_setting() {
    // `s.m` and `s.m.x` are settings' keys; only `m.<card>` is memory
    let mut writes = Writes::default();
    for key in ["s", "s.m", "s.m.x", "s-left", "settings"] {
        writes.count(key);
    }
    assert_eq!(writes, Writes { settings: 5, memory: 0 });
}
