//! The engine's own settings - the clock's and the station's - against a store that is a map: what each takes and refuses, what a
//! person reads of it, and that a value set in place outlives the seeding of a new image. Checked with the rules of a device that has
//! none (`NO_RULES`): the mechanism alone.

use std::collections::HashMap;

use settings::{change, change_all, check, check_description, find, posix, posix_ok, seed, servers, value, words, key_of, Kind, Setting, Store, Values, ENGINE, NO_RULES, TIME, ZONE};

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

/// A store whose writes fail from the FAIL_AT-th on, counted from when it is armed - nvs full, flash worn out.
struct Failing {
    map: Map,
    puts: usize,
    fail_at: Option<usize>,
}

impl Store for Failing {
    fn get(&self, name: &str) -> Option<String> {
        self.map.get(name)
    }

    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.puts += 1;
        if self.fail_at.is_some_and(|at| self.puts >= at) {
            return Err("nvs: no free pages".into());
        }
        self.map.put(name, value)
    }

    fn remove(&mut self, name: &str) -> Result<bool, String> {
        self.map.remove(name)
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

/// What a zone says of itself, read off the rule the board gets: the offset from UTC in hours in winter and in summer. POSIX writes the offset the other way round ("EET-2" is UTC+2), and a zone with a
/// summer name after the winter one (EEST) moves an hour in summer.
fn offsets_of_rule(rule: &str) -> (i32, i32) {
    let name = if rule.starts_with('<') { rule.find('>').expect("a quoted name ends") + 1 } else { rule.chars().take_while(|c| c.is_ascii_alphabetic()).count() };
    let rest = &rule[name..];
    let number: String = rest.chars().take_while(|c| *c == '-' || *c == '+' || c.is_ascii_digit()).collect();
    let winter = -number.parse::<i32>().expect("a rule has an offset");
    let summer_name = rest[number.len()..].chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    (winter, if summer_name { winter + 1 } else { winter })
}

fn zones() -> Vec<&'static str> {
    let Kind::Choice(zones) = ZONE.kind else { panic!("the zone is a choice") };
    zones.iter().map(|z| z.value).collect()
}

/// The offsets of every zone a person can choose, winter and summer, as the rules the board gets say them.
fn offsets_of_the_list() -> Vec<(i32, i32)> {
    zones().into_iter().map(|zone| offsets_of_rule(posix(zone))).collect()
}

#[test]
fn every_default_is_a_value_its_setting_takes() {
    for s in ENGINE {
        assert_eq!(takes(s, s.default), s.default, "{}", s.name);
    }
}

#[test]
fn every_name_fits_a_key_of_nvs() {
    for s in ENGINE {
        assert!(s.name.len() <= 15 && s.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'), "{}", s.name);
    }
}

#[test]
fn time_comes_automatically_from_the_internet_only_or_from_a_server() {
    assert_eq!(takes(&TIME, "auto"), "auto");
    assert_eq!(takes(&TIME, "internet"), "internet");
    assert_eq!(takes(&TIME, "Time.Google.com"), "server:time.google.com");
    assert_eq!(takes(&TIME, "192.0.2.1"), "server:192.0.2.1");
    assert_eq!(takes(&TIME, "server:time.google.com"), "server:time.google.com");
}

#[test]
fn a_server_that_is_none_is_refused_in_words() {
    for bad in ["", "   ", "server:", "a b", "pool..ntp.org", "under_score.org", "пул.org", "gateway", &"a".repeat(64)] {
        let why = refuses(&TIME, bad);
        assert!(why.contains("server"), "{bad}: {why}");
    }
}

#[test]
fn each_way_of_the_time_asks_its_servers() {
    assert_eq!(servers("auto"), ["gateway", "pool.ntp.org"]);
    assert_eq!(servers("internet"), ["pool.ntp.org"]);
    assert_eq!(servers("server:time.google.com"), ["time.google.com"]);
}

#[test]
fn the_zones_a_person_chooses_between_move_the_clock_by_the_five_rules_there_are() {
    // winter, summer: two hours that become three, one that becomes two, none that becomes one, three the year round, none the year round
    let mut have = offsets_of_the_list();
    have.sort();
    assert_eq!(have, [(0, 0), (0, 1), (1, 2), (2, 3), (3, 3)]);
}

#[test]
fn a_board_that_never_chose_a_zone_stands_where_the_owner_lives_two_hours_and_three_in_summer() {
    let mut store = Map::default();
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    assert_eq!(offsets_of_rule(posix(&value(&ZONE, &store))), (2, 3));
    assert_eq!(offsets_of_rule(posix(ZONE.default)), (2, 3));
}

#[test]
fn a_zone_is_taken_typed_in_any_case_and_what_is_no_zone_is_refused() {
    for zone in zones() {
        assert_eq!(takes(&ZONE, zone), zone);
        assert_eq!(takes(&ZONE, &zone.to_ascii_uppercase()), zone, "{zone} typed in capitals");
    }
    for bad in ["", "Europe/Riga", "EET-2EEST,M3.5.0/3,M10.5.0/4", "mars", "gmt+4", "+2/+3"] {
        refuses(&ZONE, bad);
    }
}

#[test]
fn a_zone_kept_as_a_city_leads_to_the_same_offsets_that_city_had() {
    for (kept, winter, summer) in [("kyiv", 2, 3), ("riga", 2, 3), ("warsaw", 1, 2), ("london", 0, 1), ("istanbul", 3, 3), ("utc", 0, 0)] {
        // typed or read from the store, a city is a zone of the list and its rule is the one it had
        let taken = takes(&ZONE, kept);
        assert!(zones().contains(&taken.as_str()), "{kept} is taken as {taken}, which a person can choose");
        assert_eq!(offsets_of_rule(posix(&taken)), (winter, summer), "{kept} typed");
        assert_eq!(offsets_of_rule(posix(kept)), (winter, summer), "{kept}: the board's rule for what was kept");
        // the zone the board kept in its own layout - a set of `name=value` under its card's key -, put there by the seeding as a board of the old image had it
        let mut store = Map::default();
        seed(&ENGINE, &NO_RULES, &mut store, &[("zone", kept)]);
        let read = value(&ZONE, &store);
        assert!(zones().contains(&read.as_str()), "{kept} is read as {read}, which a person can choose");
        assert_eq!(offsets_of_rule(posix(&read)), (winter, summer), "{kept} read from the store");
    }
}

#[test]
fn every_zone_s_rule_is_one_newlib_reads() {
    for zone in zones() {
        assert!(posix_ok(posix(zone)), "{zone}: {}", posix(zone));
    }
}

#[test]
fn a_rule_newlib_would_misread_is_no_rule() {
    for good in ["EET-2EEST,M3.5.0/3,M10.5.0/4", "UTC0", "<+03>-3", "GMT0BST,M3.5.0/1,M10.5.0"] {
        assert!(posix_ok(good), "{good}");
    }
    for bad in ["", "Europe/Kyiv", "EET", "EET-25", "EET-2EEST", "EET-2EEST,M13.5.0/3,M10.5.0/4", "EET-2:60"] {
        assert!(!posix_ok(bad), "{bad}");
    }
}

#[test]
fn an_empty_store_is_seeded_from_the_description_then_the_defaults() {
    let mut store = Map::default();
    let lines = seed(&ENGINE, &NO_RULES, &mut store, &[("zone", "utc")]);
    assert_eq!(value(&ZONE, &store), "utc");
    assert_eq!(value(&TIME, &store), "auto");
    assert_eq!(value(&settings::NET_RESET_LAPS, &store), "2");
    assert_eq!(lines.len(), ENGINE.len(), "{lines:?}");
}

#[test]
fn a_value_set_in_place_outlives_the_seeding() {
    let mut store = Map::default();
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    change(&ENGINE, &NO_RULES, &mut store, "time", "internet").unwrap();
    let lines = seed(&ENGINE, &NO_RULES, &mut store, &[("time", "auto")]);
    assert_eq!(value(&TIME, &store), "internet");
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn a_change_answers_in_words_and_is_stored() {
    let mut store = Map::default();
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    assert_eq!(change(&ENGINE, &NO_RULES, &mut store, "time", "time.google.com").as_deref(), Ok("Time comes from: from the server time.google.com"));
    assert_eq!(value(&TIME, &store), "server:time.google.com");
}

#[test]
fn a_refused_change_leaves_the_old_value() {
    let mut store = Map::default();
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    assert!(change(&ENGINE, &NO_RULES, &mut store, "time", "").is_err());
    assert_eq!(value(&TIME, &store), "auto");
}

#[test]
fn a_save_with_one_bad_value_stores_nothing() {
    let mut store = Map::default();
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    let why = change_all(&ENGINE, &NO_RULES, &mut store, &[("time", "internet"), ("net_reset_laps", "0")]).unwrap_err();
    assert_eq!(why.code(), "not_whole", "{why}");
    assert_eq!(value(&TIME, &store), "auto", "the good one stays unstored too");
    let why = change_all(&ENGINE, &NO_RULES, &mut store, &[("wifi", "x")]).unwrap_err();
    assert_eq!(why.wire(), serde_json::json!({"code": "no_setting", "name": "wifi"}));
}

#[test]
fn a_stored_value_gone_bad_reads_as_the_default() {
    let mut store = Map::default();
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    let key = settings::key_of(settings::NET_RESET_LAPS.card);
    let kept = store.get(&key).unwrap().replace("net_reset_laps=2", "net_reset_laps=99");
    store.put(&key, &kept).unwrap();
    assert_eq!(value(&settings::NET_RESET_LAPS, &store), "2");
}

#[test]
fn a_description_names_only_settings_with_values_they_take() {
    assert!(check_description(&ENGINE, &[("time", "auto"), ("zone", "gmt+2")]).is_ok());
    assert!(check_description(&ENGINE, &[("tz", "UTC0")]).unwrap_err().contains("tz"));
    assert!(check_description(&ENGINE, &[("zone", "Europe/Riga")]).unwrap_err().contains("Europe/Riga"));
}

#[test]
fn a_setting_is_found_by_its_name() {
    assert_eq!(find(&ENGINE, "zone").map(|s| s.name), Some("zone"));
    assert!(find(&ENGINE, "tz").is_none());
}

#[test]
fn the_keys_of_the_settings_before_are_erased_when_the_set_is_first_seeded() {
    let mut store = Map::default();
    for (k, v) in [("time", "auto"), ("zone", "gmt+2"), ("rows", "60"), ("keep", "12"), ("ntp", "gateway pool.ntp.org"),
        ("tz", "UTC0"), ("csv_min", "60"), ("csv_months", "12"), ("wifi_ssid", "GZ103")] {
        store.put(k, v).unwrap();
    }
    let lines = seed(&ENGINE, &NO_RULES, &mut store, &[]);
    for old in ["time", "zone", "rows", "keep", "ntp", "tz", "csv_min", "csv_months"] {
        assert!(store.0.get(old).is_none(), "{old} left");
    }
    assert!(lines.iter().any(|l| l.contains("erased") && l.contains("csv_months")), "{lines:?}");
    assert_eq!(store.0.get("wifi_ssid").map(String::as_str), Some("GZ103"), "a key of no setting before stays");
    assert_eq!(value(&TIME, &store), "auto");
}

#[test]
fn the_server_of_time_is_still_checked_as_a_host() {
    // the rule moved onto the choice, and it has to keep working: a role's name is free text, a server's is not
    assert!(settings::check(&settings::TIME, "server:time.google.com").is_ok());
    assert!(settings::check(&settings::TIME, "server:not a host").is_err());
}

#[test]
fn a_zone_kept_as_a_city_is_published_as_the_gmt_zone_it_stood_for_and_the_store_keeps_the_city() {
    for (kept, shown) in [("kyiv", "gmt+2"), ("riga", "gmt+2"), ("warsaw", "gmt+1"), ("london", "gmt"), ("istanbul", "gmt+3")] {
        let mut store = Map::default();
        store.put(&key_of(ZONE.card), &format!("zone={kept}")).unwrap();
        // what the page is handed is the set the owner of the flash publishes
        assert_eq!(Values::of(&ENGINE, &store).get(&ZONE), shown, "{kept} as the page sees it");
        assert_eq!(store.get(&key_of(ZONE.card)).unwrap(), format!("zone={kept}"), "{kept} stays in the store as it was");
    }
}

#[test]
fn a_person_reads_words_and_no_keys() {
    assert_eq!(words(&TIME, "auto"), "auto");
    assert_eq!(words(&TIME, "server:time.google.com"), "from the server time.google.com");
    for s in ENGINE {
        let w = words(s, s.default);
        assert!(!w.contains(s.name) && !w.contains("gateway") && !w.contains("server:"), "{}: {w}", s.name);
    }
}

#[test]
fn a_save_is_whole_or_nothing_when_the_store_fails_midway() {
    let mut store = Failing { map: Map::default(), puts: 0, fail_at: None };
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    store.puts = 0;
    store.fail_at = Some(2);
    let result = change_all(&ENGINE, &NO_RULES, &mut store, &[("time", "internet"), ("net_reset_laps", "5")]);
    let (time, laps) = (value(&TIME, &store), value(&settings::NET_RESET_LAPS, &store));
    match result {
        Ok(_) => assert_eq!((time.as_str(), laps.as_str()), ("internet", "5"), "said saved, both are"),
        Err(_) => assert_eq!((time.as_str(), laps.as_str()), ("auto", "2"), "said refused, neither is"),
    }
}

#[test]
fn a_save_the_store_refuses_changes_nothing_and_says_why() {
    let mut store = Failing { map: Map::default(), puts: 0, fail_at: None };
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    store.fail_at = Some(0);
    let why = change_all(&ENGINE, &NO_RULES, &mut store, &[("time", "internet"), ("net_reset_laps", "5")]).unwrap_err();
    assert_eq!(why.wire(), serde_json::json!({"code": "not_written"}));
    assert_eq!((value(&TIME, &store), value(&settings::NET_RESET_LAPS, &store)), ("auto".to_string(), "2".to_string()));
}

#[test]
fn a_save_of_the_tab_stores_all_or_nothing_and_names_only_what_changed() {
    let mut store = Map::default();
    seed(&ENGINE, &NO_RULES, &mut store, &[]);
    let said = change_all(&ENGINE, &NO_RULES, &mut store, &[("time", "internet"), ("zone", "gmt+2"), ("net_reset_laps", "5"), ("net_reboot_laps", "2")]).unwrap();
    assert_eq!(said.iter().map(|m| (m.name, m.value.as_str())).collect::<Vec<_>>(), [("time", "internet"), ("net_reset_laps", "5")]);
    assert_eq!(value(&settings::NET_RESET_LAPS, &store), "5");
}
