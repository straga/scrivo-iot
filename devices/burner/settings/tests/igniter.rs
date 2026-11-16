//! The igniter's time is one number, on the automaton's card: the relay's own limit follows
//! from it and is no setting of its own - nobody can set it apart and have the relay cut the kindling in the middle.

use burner_settings::RULES;
use std::collections::BTreeMap;

use settings::{change, change_all, key_of, seed, Keeper, Setting, Store, Values};
use burner_settings::{igniter_limit_s, ALL, AUTO_IGNITER};

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

fn limit(set: &Values) -> u32 {
    igniter_limit_s(&|s: &Setting| set.get(s))
}

fn board() -> Keeper<Nvs> {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    Keeper::new(ALL.to_vec(), &RULES, nvs)
}

#[test]
fn a_new_time_of_the_igniter_moves_the_relays_limit_by_itself() {
    let mut board = board();
    let before = limit(&board.now());
    board.change_all(&[("auto_igniter", "120")]).expect("a good time of the igniter refused");
    let after = limit(&board.now());
    assert_ne!(before, after, "the relay's limit stayed where it was");
    // the plug burns only with air: the limit is the burning and the margin, and nothing else
    assert_eq!(after, 120 + burner_settings::IGNITER_MARGIN_S, "the limit is not the burning and the margin");
}

#[test]
fn the_igniters_limit_is_no_setting_the_page_or_the_console_can_write() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    // the page's save and the console's `set` - both come to the one change of the list
    let page = change_all(&ALL, &RULES, &mut nvs, &[("igniter_limit", "100")]).expect_err("the page wrote the igniter's limit");
    let console = change(&ALL, &RULES, &mut nvs, "igniter_limit", "100").expect_err("the console wrote the igniter's limit");
    assert_eq!(page.wire(), serde_json::json!({"code": "no_setting", "name": "igniter_limit"}), "the page's refusal does not say there is no such setting");
    assert!(console.contains("no setting 'igniter_limit'"), "the console's line does not say there is no such setting: {console}");
    assert!(!ALL.iter().any(|s| s.name == "igniter_limit"), "igniter_limit is still a setting of the board");
}

#[test]
fn a_limit_an_older_image_kept_is_carried_into_nothing() {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    // an older image kept its own limit on the relays' card, set by a person to 100 s
    let key = key_of(Some("relays"));
    let was = nvs.0.get(&key).cloned().unwrap_or_default();
    nvs.0.insert(key, format!("{was}\nigniter_limit=100"));
    seed(&ALL, &RULES, &mut nvs, &[]);
    let board = Keeper::new(ALL.to_vec(), &RULES, nvs);
    let set = board.now();
    assert_eq!(set.get(&AUTO_IGNITER), AUTO_IGNITER.default, "the old limit was carried into the time of the igniter");
    assert_eq!(limit(&set), 240 + burner_settings::IGNITER_MARGIN_S, "the old limit stands in for the one that follows");
}
