//! The settings against a store that is a map: what each takes and refuses, what a person reads of it, and that a
//! value set in place outlives the seeding of a new image.

use burner_settings::{ALL, AUGER_LIMIT, PELLET_RATE, RULES};
use std::collections::HashMap;

use settings::{change, change_all, check, check_description, find, seed, value, words, key_of, Kind, Setting, Store, TIME, ZONE};

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

#[test]
fn every_default_is_a_value_its_setting_takes() {
    for s in ALL {
        assert_eq!(takes(s, s.default), s.default, "{}", s.name);
    }
}

#[test]
fn every_name_fits_a_key_of_nvs() {
    for s in ALL {
        assert!(s.name.len() <= 15 && s.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'), "{}", s.name);
    }
}

#[test]
fn a_person_reads_words_and_no_keys() {
    assert_eq!(words(&TIME, "auto"), "auto");
    assert_eq!(words(&TIME, "server:time.google.com"), "from the server time.google.com");
    for s in ALL {
        let w = words(s, s.default);
        assert!(!w.contains(s.name) && !w.contains("gateway") && !w.contains("server:"), "{}: {w}", s.name);
    }
}


#[test]
fn a_span_of_seconds_is_taken_as_a_whole_number_within_its_bounds() {
    assert_eq!(check(&AUGER_LIMIT, "600"), Ok("600".to_string()));
    assert_eq!(check(&AUGER_LIMIT, " 10 "), Ok("10".to_string()));
}

#[test]
fn a_span_out_of_bounds_or_not_a_number_is_refused_with_its_name_and_its_bounds() {
    for text in ["9", "601", "0", "-5", "5 min", "", "3e2", "0600", "600.0"] {
        let why = check(&AUGER_LIMIT, text).expect_err(text).wire();
        assert_eq!(why, serde_json::json!({"code": "not_seconds", "name": "auger_limit", "text": text, "from": 10, "to": 600}), "{text}");
    }
}

#[test]
fn a_span_reads_in_minutes_and_seconds() {
    assert_eq!(words(&AUGER_LIMIT, "300"), "5 min");
    assert_eq!(words(&AUGER_LIMIT, "90"), "1 min 30 s");
    assert_eq!(words(&AUGER_LIMIT, "45"), "45 s");
    assert_eq!(words(&AUGER_LIMIT, "600"), "10 min");
}

#[test]
fn the_augers_limit_starts_from_a_minute() {
    let mut store = Map::default();
    let list = [&AUGER_LIMIT];
    seed(&list, &RULES, &mut store, &[]);
    assert_eq!(value(&AUGER_LIMIT, &store), "60");
}

#[test]
fn a_limit_set_in_place_outlives_the_seeding_of_a_new_image() {
    let mut store = Map::default();
    let list = [&AUGER_LIMIT];
    seed(&list, &RULES, &mut store, &[]);
    change(&list, &RULES, &mut store, "auger_limit", "10").unwrap();
    seed(&list, &RULES, &mut store, &[("auger_limit", "60")]);
    assert_eq!(value(&AUGER_LIMIT, &store), "10");
}

#[test]
fn a_card_saves_its_own_settings_and_leaves_the_rest() {
    let mut store = Map::default();
    let list = [&TIME, &ZONE, &AUGER_LIMIT];
    seed(&list, &RULES, &mut store, &[("zone", "utc")]);
    let said = change_all(&list, &RULES, &mut store, &[("auger_limit", "30")]).unwrap();
    assert_eq!(said.iter().map(|m| (m.name, m.value.as_str())).collect::<Vec<_>>(), [("auger_limit", "30")]);
    assert_eq!(value(&ZONE, &store), "utc");
}

#[test]
fn the_augers_limit_lies_in_the_card_of_the_relays_and_the_board_s_settings_in_none() {
    assert_eq!(AUGER_LIMIT.card, Some("relays"));
    assert!(settings::ENGINE.iter().all(|s| s.card.is_none()));
}

#[test]
fn a_description_may_seed_the_limits_and_is_refused_a_limit_out_of_bounds() {
    assert_eq!(check_description(&ALL, &[("auger_limit", "250")]), Ok(()));
    assert!(check_description(&ALL, &[("auger_limit", "900")]).is_err());
}

// --- a whole number of its own: a module's address on a fieldbus ---

#[test]
fn an_address_in_the_range_is_kept_as_it_was_typed() {
    for good in ["1", "2", "247"] {
        assert_eq!(settings::check(&burner_settings::NTC_ADDRESS, good), Ok(good.to_string()), "{good}");
    }
}

#[test]
fn an_address_outside_the_range_or_not_a_number_is_refused() {
    // 0 is the broadcast address and no slave answers on it; 248 is past what Modbus allows
    // " 2 " is not here on purpose: every setting trims what a person typed, and that is the same for all of them
    for bad in ["0", "248", "-1", "2.0", "two", "", " ", "0x02"] {
        let said = settings::check(&burner_settings::NTC_ADDRESS, bad);
        assert!(said.is_err(), "{bad:?} was taken as an address: {said:?}");
    }
}

#[test]
fn the_range_of_a_field_is_said_in_the_refusal_and_not_over_the_field() {
    // THE RULE CHANGED, by the owner's word of 21.09: a range printed above a field says the same thing all day
    // to somebody who is not doing anything wrong, and a page of such lines stops being read at all. Where it
    // belongs is the moment it matters - the refusal - and the refusal already says it.
    // the refusal carries the two ends of the range, as numbers
    let why = refuses(&burner_settings::NTC_ADDRESS, "300");
    assert!(why.contains('1') && why.contains("247"), "{why}");
}

#[test]
fn an_address_reads_as_the_plain_number() {
    assert_eq!(settings::words(&burner_settings::NTC_ADDRESS, "2"), "2");
}

#[test]
fn the_poll_span_is_seconds_and_refuses_what_is_not() {
    assert_eq!(settings::check(&burner_settings::POLL_EVERY, "2"), Ok("2".to_string()));
    assert_eq!(settings::words(&burner_settings::POLL_EVERY, "2"), "2 s");
    for bad in ["0", "61", "half", ""] {
        assert!(settings::check(&burner_settings::POLL_EVERY, bad).is_err(), "{bad:?} was taken as a span");
    }
}

// --- what a channel measures: a role from the list, or a name of one's own ---

#[test]
fn a_role_out_of_the_list_is_kept_as_it_is() {
    for good in ["free", "boiler", "supply", "return", "flue", "hopper", "room"] {
        assert_eq!(settings::check(&burner_settings::ROLE_NTC_1, good), Ok(good.to_string()), "{good}");
    }
}

#[test]
fn a_role_that_is_none_of_them_is_refused_with_the_list() {
    // `water` was a role until the water of the boiler and the water leaving it were told apart
    let why = settings::check(&burner_settings::ROLE_NTC_1, "water").expect_err("water is no role any more").wire();
    let listed: Vec<String> = why["choices"].as_array().expect("the refusal carries the choices").iter().map(|c| c.as_str().unwrap().to_string()).collect();
    for named in ["boiler", "supply", "return", "flue", "hopper", "room", "own"] {
        assert!(listed.iter().any(|c| c == named), "the refusal does not name {named}: {why}");
    }
}

#[test]
fn own_without_a_name_is_refused_because_a_role_of_ones_own_needs_one() {
    for bad in ["own", "own:", "own:  "] {
        assert!(settings::check(&burner_settings::ROLE_NTC_1, bad).is_err(), "{bad:?} was taken as a role");
    }
}

#[test]
fn own_with_a_name_keeps_the_name() {
    let kept = settings::check(&burner_settings::ROLE_NTC_1, "own:buffer tank").expect("a name of one's own");
    assert_eq!(kept, "own:buffer tank");
    assert_eq!(burner_settings::role_words(&kept), "buffer tank");
}

#[test]
fn a_role_reads_as_words_and_free_says_it_is_free() {
    assert_eq!(burner_settings::role_words("boiler"), "water in the boiler");
    assert_eq!(burner_settings::role_words("supply"), "water leaving the boiler");
    assert_eq!(burner_settings::role_words("free"), "nothing plugged in");
}

#[test]
fn every_channel_of_the_map_has_a_role_of_its_own_and_starts_free() {
    let roles = [
        &burner_settings::ROLE_NTC_1, &burner_settings::ROLE_NTC_2, &burner_settings::ROLE_NTC_3, &burner_settings::ROLE_NTC_4,
        &burner_settings::ROLE_NTC_5, &burner_settings::ROLE_NTC_6, &burner_settings::ROLE_NTC_7,
    ];
    for role in roles {
        assert_eq!(role.default, burner_settings::FREE, "{}", role.name);
        // no card: a role is set once at the mounting, so it stands on the tab Settings, not among the knobs of
        // the card a person reads while the boiler runs
        assert_eq!(role.card, None, "{}", role.name);
        assert!(role.name.len() <= 15, "{} is longer than nvs takes", role.name);
    }
    let named: std::collections::BTreeSet<&str> = roles.iter().map(|r| r.name).collect();
    assert_eq!(named.len(), roles.len(), "two channels share a key");
}

#[test]
fn the_card_of_temperatures_holds_only_what_is_turned_while_it_runs() {
    assert_eq!(burner_settings::NTC_ADDRESS.card, Some("temperatures"));
    assert_eq!(burner_settings::POLL_EVERY.card, Some("temperatures"));
}

#[test]
fn a_role_of_ones_own_is_read_back_as_the_name_that_was_written() {
    // not "own:buffer tank": a person reads what they typed, not the key it is kept behind
    assert_eq!(settings::words(&burner_settings::ROLE_NTC_1, "own:buffer tank"), "buffer tank");
    assert_eq!(settings::words(&burner_settings::ROLE_NTC_1, "supply"), "water leaving the boiler");
    // and the server of time still reads as it did
    assert_eq!(settings::words(&settings::TIME, "server:time.google.com"), "from the server time.google.com");
}

// ---- the divider of the draught: two resistances a person reads off a voltmeter ----

#[test]
fn the_divider_takes_two_resistances_in_ohms() {
    // TWO NUMBERS AND NOT ONE COEFFICIENT. A voltmeter measures resistances, and that is what a person can write
    // down without thinking; a single coefficient would make them divide in their head, and a slip in that
    // division looks exactly like a board working properly on a wrong pressure.
    assert_eq!(takes(&burner_settings::DIVIDER_TOP, "10000"), "10000");
    assert_eq!(takes(&burner_settings::DIVIDER_BOTTOM, "9870"), "9870");
    assert_eq!(words(&burner_settings::DIVIDER_BOTTOM, "9870"), "9870 Ω");
}

#[test]
fn a_resistance_that_is_not_one_is_refused_and_the_divider_keeps_what_it_had() {
    // A zero in the bottom arm is the setting fault crates/draught answers Unconfigured to; it must not get as
    // far as being stored. Nor must a word, nor a number written any way but plainly.
    for bad in ["0", "", "10k", "10 000", "-10000", "1000000000", "0x2710"] {
        let why = refuses(&burner_settings::DIVIDER_BOTTOM, bad);
        assert!(why.contains("whole number"), "{bad}: {why}");
    }
}

#[test]
fn both_arms_of_the_divider_stand_in_the_card_of_the_draught() {
    // the card is where a person is when they are looking at a pressure that is wrong - not on the board's own tab
    assert_eq!(burner_settings::DIVIDER_TOP.card, Some("draught"));
    assert_eq!(burner_settings::DIVIDER_BOTTOM.card, Some("draught"));
}

#[test]
fn a_resistance_no_divider_has_never_reaches_the_reading() {
    // THE SECOND LAYER UNDER crates/draught, and it stands where the numbers come in rather than where they are
    // used. The arms that made the reading overflow - 4 292 115 452 Ω over 35 Ω - are not resistances anybody
    // solders; the field refuses them, so that pair cannot be stored at all. The crate still checks its own
    // arithmetic, because `read` is public and its honesty may not depend on who called it.
    for absurd in ["4292115452", "2147483649", "1000001"] {
        let why = refuses(&burner_settings::DIVIDER_TOP, absurd);
        assert!(why.contains("whole number from 100 to 1000000"), "{absurd}: {why}");
    }
    // and the largest one anybody would solder is still taken
    assert_eq!(takes(&burner_settings::DIVIDER_TOP, "1000000"), "1000000");
}

// ---- the two thresholds of the flame: numbers a lamp and a palm give ----

#[test]
fn the_flame_takes_two_thresholds_in_millivolts() {
    assert_eq!(takes(&burner_settings::FLAME_LIT, "2400"), "2400");
    assert_eq!(takes(&burner_settings::FLAME_STEADY, "1100"), "1100");
    assert_eq!(words(&burner_settings::FLAME_LIT, "2400"), "2400 mV");
}

#[test]
fn a_threshold_outside_what_the_pin_can_hold_is_refused() {
    // the pin cannot sit under 0 or over its 3.3 V supply, and a threshold out there divides nothing
    for bad in ["0", "", "3301", "-100", "2.4", "2400 mV"] {
        let why = refuses(&burner_settings::FLAME_STEADY, bad);
        assert!(why.contains("whole number from 100 to 3300"), "{bad}: {why}");
    }
}

#[test]
fn both_thresholds_stand_in_the_card_of_the_flame() {
    assert_eq!(burner_settings::FLAME_LIT.card, Some("flame"));
    assert_eq!(burner_settings::FLAME_STEADY.card, Some("flame"));
}

// ---- the fan holds the draught ----

#[test]
fn the_setpoint_is_in_pascals_on_the_fan_s_card() {
    // A PERCENT IS NOT A PRESSURE, and that is a measurement and not an opinion: 30 % read as 89 Pa and, within
    // the hour, as 47 Pa on the same cold burner. Only pascals repeat.
    assert!(matches!(burner_settings::DRAUGHT_PA.kind, Kind::Whole { unit: "Pa", .. }));
    assert_eq!(burner_settings::DRAUGHT_PA.card, Some("fan"));
}

#[test]
fn the_setpoint_takes_a_pressure_this_sensor_can_say_and_refuses_one_it_cannot() {
    let mut map = Map::default();
    assert!(change(&ALL, &RULES, &mut map, "draught_pa", "74").is_ok());
    assert_eq!(value(&burner_settings::DRAUGHT_PA, &map), "74");
    // the sensor ends at 300 Pa; past it there would be no reading to regulate on at all
    let refused = change(&ALL, &RULES, &mut map, "draught_pa", "301");
    assert!(refused.is_err(), "301 Pa was taken; the sensor ends at {} Pa", draught_top());
    assert_eq!(value(&burner_settings::DRAUGHT_PA, &map), "74", "a refused value was stored anyway");
}

/// The top of the sensor's scale, for the message of the case above.
fn draught_top() -> u32 {
    match burner_settings::DRAUGHT_PA.kind {
        Kind::Whole { to, .. } => to,
        _ => 0,
    }
}

#[test]
fn a_floor_over_the_ceiling_is_refused_by_each_setting_s_own_span() {
    let mut map = Map::default();
    // each span is its own; a pair that crosses is a fault the regulator answers, not one the tab can spell
    assert!(change(&ALL, &RULES, &mut map, "fan_min", "4").is_err(), "a floor under what any fan holds was taken");
    assert!(change(&ALL, &RULES, &mut map, "fan_max", "101").is_err(), "a ceiling over full travel was taken");
    assert!(change(&ALL, &RULES, &mut map, "fan_min", "60").is_ok());
    assert!(change(&ALL, &RULES, &mut map, "fan_max", "20").is_ok());
}

#[test]
fn the_four_gains_act_on_a_miss_in_angle_and_not_in_pascals() {
    // the unit a person reads says what the number acts on: a miss in percents of angle, not pascals
    for s in [&burner_settings::UP_KP, &burner_settings::UP_KI, &burner_settings::DOWN_KP, &burner_settings::DOWN_KI] {
        let settings::Kind::Whole { unit, .. } = s.kind else {
            panic!("{} is not a whole number any more", s.name)
        };
        assert!(!unit.contains("Pa"), "{} still says it acts on pascals: {unit}", s.name);
    }
    for s in [&burner_settings::UP_KP, &burner_settings::UP_KI, &burner_settings::DOWN_KP, &burner_settings::DOWN_KI] {
        assert_eq!(s.card, Some("fan"), "{} is not on the fan's card", s.name);
        // a gain of nothing is a legal answer: it switches that half of the regulator off while tuning
        let mut map = Map::default();
        assert!(change(&ALL, &RULES, &mut map, s.name, "0").is_ok(), "{} refused a gain of nothing", s.name);
        assert!(change(&ALL, &RULES, &mut map, s.name, "2001").is_err(), "{} took a gain past its span", s.name);
    }
}

#[test]
fn every_setting_of_the_fan_is_on_the_fan_s_card_and_in_the_list_the_tab_draws() {
    let fan: Vec<&str> = ALL.iter().filter(|s| s.card == Some("fan")).map(|s| s.name).collect();
    assert_eq!(
        fan,
        vec!["draught_pa", "fan_min", "fan_max", "fan_step", "up_kp", "up_ki", "down_kp", "down_ki", "fan_band",
             "fan_band_share"],
        "the fan's card does not draw the settings the regulator reads"
    );
}

#[test]
fn the_numbers_a_setting_shares_with_its_crate_are_the_same_numbers() {
    // TWO CAPS THAT DISAGREED WOULD MEAN A PAIR A PERSON CAN TYPE AND THE READING CANNOT USE - and nothing
    // would say so. A default written as a word cannot be built from a number at build time, so the spans come
    // from the crates directly and the defaults are held to them here.
    assert!(matches!(burner_settings::DIVIDER_TOP.kind, Kind::Whole { to, .. } if to == draught::MOST_OHM));
    assert!(matches!(burner_settings::DIVIDER_BOTTOM.kind, Kind::Whole { to, .. } if to == draught::MOST_OHM));
    assert!(matches!(burner_settings::DRAUGHT_PA.kind, Kind::Whole { to, .. } if to == draught::FULL_PA));
}

// ---------------------------------------------------------------------------------------------------------
// A setting whose value stops meaning what it meant: the four gains went from thousandths of a percent
// of angle per PASCAL to a plain ratio, because the regulator now divides the error by the steepness of the
// fan's curve. Flashing a new image does not touch what a person set in place - so without a carry, a tuned
// board comes up five times gentler than it was and says nothing about it.
// ---------------------------------------------------------------------------------------------------------

/// A store holding the set as a board tuned before the change holds it.
fn tuned_before() -> Map {
    let mut map = Map::default();
    let set = ["up_kp=30", "up_ki=9", "down_kp=30", "down_ki=6", "draught_pa=31", "fan_step=5"];
    map.put(settings::KEY, &set.join("\n")).unwrap();
    map
}

#[test]
fn a_gain_stored_under_the_old_unit_is_carried_to_the_new_one() {
    let mut map = tuned_before();
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert_eq!(value(&burner_settings::UP_KP, &map), "150", "30 in the old unit is 150 in the new one");
    assert_eq!(value(&burner_settings::UP_KI, &map), "45");
    assert_eq!(value(&burner_settings::DOWN_KP, &map), "150");
    assert_eq!(value(&burner_settings::DOWN_KI, &map), "30");
    assert!(
        said.iter().any(|l| l.contains("up_kp") && l.contains("30") && l.contains("150")),
        "the carry passed without a word about it: {said:?}"
    );
    // and what did NOT change meaning is left exactly as the person set it
    assert_eq!(value(&burner_settings::DRAUGHT_PA, &map), "31");
}

#[test]
fn a_carry_already_done_is_not_done_again() {
    // **THE PAIR THAT MATTERS MOST, because the second carry is silent and multiplies a good value by five.**
    let mut map = tuned_before();
    settings::carry_over(&ALL, &RULES, &mut map);
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert_eq!(value(&burner_settings::UP_KP, &map), "150", "the gain was carried twice");
    assert!(said.is_empty(), "the second carry spoke, so it did something: {said:?}");
}

#[test]
fn a_board_that_never_had_settings_is_not_carried_at_all() {
    // Negative: an empty store is a NEW board, and its defaults are already written in the new unit. Carrying
    // them would multiply the seed by five before anybody ever set anything.
    let mut map = Map::default();
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert!(said.is_empty(), "an empty store was carried: {said:?}");
    seed(&ALL, &RULES, &mut map, &[]);
    assert_eq!(value(&burner_settings::UP_KP, &map), burner_settings::UP_KP.default, "the default is already in the new unit");
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert!(said.is_empty(), "a freshly seeded store was carried: {said:?}");
    assert_eq!(value(&burner_settings::UP_KP, &map), burner_settings::UP_KP.default);
}

#[test]
fn a_value_that_cannot_be_carried_falls_back_to_the_default_and_says_so() {
    // Negative: whatever is stored is not a number the carry can work with - a hand-edited nvs, a value from a
    // firmware that is not ours. Silence here would leave a gain of nothing on the fan.
    let mut map = Map::default();
    map.put(settings::KEY, "up_kp=nonsense\nup_ki=9\ndown_kp=30\ndown_ki=6").unwrap();
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert_eq!(value(&burner_settings::UP_KP, &map), burner_settings::UP_KP.default);
    assert!(said.iter().any(|l| l.contains("up_kp")), "a value that could not be carried passed in silence: {said:?}");
}

#[test]
fn a_carry_that_would_leave_the_span_is_clamped_rather_than_refused() {
    // Negative: the old unit allowed 2000, and five times that is past the new span. A value outside its own
    // span reads as the default (`value`), so an uncarried maximum would come back as 150 - quietly undoing
    // what a person deliberately set to the top.
    let mut map = Map::default();
    map.put(settings::KEY, "up_kp=2000\nup_ki=9\ndown_kp=30\ndown_ki=6").unwrap();
    settings::carry_over(&ALL, &RULES, &mut map);
    assert_eq!(value(&burner_settings::UP_KP, &map), "2000", "the carry left the span and the value fell to the default");
}

#[test]
fn the_carry_survives_a_person_changing_a_setting_afterwards() {
    // **THE TRAP THIS DESIGN EXISTS TO AVOID.** `change_all` rebuilds the whole set out of the list of
    // settings, so anything kept INSIDE the set that is not a setting is dropped the moment a person saves
    // one. Were the meaning kept there, the next start would carry the gains a second time.
    let mut map = tuned_before();
    settings::carry_over(&ALL, &RULES, &mut map);
    change(&ALL, &RULES, &mut map, "draught_pa", "40").unwrap();
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert!(said.is_empty(), "after a person saved a setting, the carry ran again: {said:?}");
    assert_eq!(value(&burner_settings::UP_KP, &map), "150");
}

#[test]
fn a_carry_cut_off_partway_never_multiplies_a_value_twice() {
    // **THE BOARD LOSES POWER MID-CARRY, WHICH IS HOW A BOILER IS TURNED OFF - not an accident.** If the carry
    // needs more than one write, the cut can land between them: the new numbers lie in nvs under the OLD
    // meaning, and the next start carries them AGAIN. 30 becomes 150 becomes 750 and slams into the top of the
    // span, and a regulator that was five times too gentle becomes one that is five times too fierce.
    //
    // The check does not care HOW MANY writes the carry makes - it names what must be true whatever it does:
    // after a cut and a restart, the gain is either untouched or carried exactly once. Never twice.
    for cut in 1..=3 {
        let mut interrupted = Failing { map: tuned_before(), puts: 0, fail_at: Some(cut) };
        settings::carry_over(&ALL, &RULES, &mut interrupted);

        // the power came back: whatever reached nvs is what the board reads now
        let mut after = interrupted.map;
        settings::carry_over(&ALL, &RULES, &mut after);
        let got = value(&burner_settings::UP_KP, &after);
        assert!(
            got == "150" || got == "30",
            "cut at write {cut}: the gain came back as {got}, where 30 is untouched and 150 is carried once"
        );
        // and the state it landed in is a WHOLE one: every gain carried, or none of them
        let all_four: Vec<String> =
            [&burner_settings::UP_KP, &burner_settings::UP_KI, &burner_settings::DOWN_KP, &burner_settings::DOWN_KI]
                .iter()
                .map(|s| value(s, &after))
                .collect();
        let carried = ["150", "45", "150", "30"];
        let untouched = ["30", "9", "30", "6"];
        assert!(
            all_four == carried || all_four == untouched,
            "cut at write {cut}: the gains came back half carried: {all_four:?}"
        );
    }
}

#[test]
fn a_carry_that_could_not_be_written_leaves_the_values_it_found() {
    // The pair: a store that refuses every write must not report a carry it did not make.
    let mut store = Failing { map: tuned_before(), puts: 0, fail_at: Some(1) };
    let said = settings::carry_over(&ALL, &RULES, &mut store);
    assert_eq!(value(&burner_settings::UP_KP, &store), "30", "the gain moved although no write succeeded");
    assert!(
        said.iter().any(|l| l.contains("not carried")),
        "a carry that did not happen passed in silence: {said:?}"
    );
}

// ---------------------------------------------------------------------------------------------------------
// `water` named one of two places: the water at the top of the boiler, which the burner holds, and
// the water leaving it a little further along the pipe. A board that kept `water` kept it on the socket of the
// supply, and comes up reading `supply` - once, and in the same write as the meaning it is now in.
// ---------------------------------------------------------------------------------------------------------

/// The board's own settings as a board kept them before the boiler and the supply were told apart.
fn roles_before() -> Map {
    let mut map = Map::default();
    let set = ["role_ntc_1=water", "role_ntc_2=return", "role_ntc_3=own:buffer tank", "role_ntc_4=room", "@meaning=1"];
    map.put(&settings::key_of(None), &set.join("\n")).unwrap();
    map
}

#[test]
fn a_kept_water_comes_up_as_the_supply() {
    let mut map = roles_before();
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert_eq!(value(&burner_settings::ROLE_NTC_1, &map), burner_settings::SUPPLY);
    assert!(said.iter().any(|l| l.contains("role_ntc_1") && l.contains("water") && l.contains("supply")), "{said:?}");
}

#[test]
fn the_other_roles_are_left_as_a_person_set_them_and_not_mentioned() {
    let mut map = roles_before();
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert_eq!(value(&burner_settings::ROLE_NTC_2, &map), "return");
    assert_eq!(value(&burner_settings::ROLE_NTC_3, &map), "own:buffer tank");
    assert_eq!(value(&burner_settings::ROLE_NTC_4, &map), "room");
    assert!(!said.iter().any(|l| l.contains("role_ntc_2") || l.contains("role_ntc_3")), "{said:?}");
}

#[test]
fn water_is_carried_once_and_a_supply_set_later_is_not_touched_again() {
    let mut map = roles_before();
    settings::carry_over(&ALL, &RULES, &mut map);
    change(&ALL, &RULES, &mut map, "role_ntc_1", "boiler").unwrap();
    let said = settings::carry_over(&ALL, &RULES, &mut map);
    assert!(said.is_empty(), "the carry ran a second time: {said:?}");
    assert_eq!(value(&burner_settings::ROLE_NTC_1, &map), burner_settings::BOILER);
}

#[test]
fn a_new_board_is_seeded_in_the_present_meaning_and_carries_nothing() {
    let mut map = Map::default();
    seed(&ALL, &RULES, &mut map, &[]);
    assert!(settings::carry_over(&ALL, &RULES, &mut map).is_empty());
    assert_eq!(value(&burner_settings::ROLE_NTC_1, &map), "free");
}

// ---------------------------------------------------------------- the automaton idles unless a person says otherwise

#[test]
fn every_number_of_the_automaton_stands_on_its_card() {
    let mut listed = vec![&burner_settings::AUTO_PURGE, &burner_settings::AUTO_IGNITER, &burner_settings::AUTO_WAIT,
        &burner_settings::AUTO_TRIES, &burner_settings::AUTO_KINDLE_PA, &burner_settings::AUTO_DOSE, &burner_settings::AUTO_DOSE_AGAIN,
        &burner_settings::AUTO_RISE_S, &burner_settings::AUTO_NO_FLAME_S, &burner_settings::AUTO_FLUE_FALL_S, &burner_settings::AUTO_LIT_S, &burner_settings::AUTO_BURN_OFF_S,
        &burner_settings::AUTO_PUMP_ON_C, &burner_settings::AUTO_RELIGHT_1, &burner_settings::AUTO_RELIGHT_2,
        &burner_settings::AUTO_RELIGHT_3,
        &burner_settings::AUTO_PUMP_OFF_C, &burner_settings::AUTO_PURGE_END, &burner_settings::AUTO_OUT_MIN_S, &burner_settings::AUTO_SMOULDER_S,
        &burner_settings::AUTO_SMOULD_PA, &burner_settings::AUTO_SMOULD_ON, &burner_settings::AUTO_SMOULD_OFF, &burner_settings::AUTO_FLUE_C,
        &burner_settings::AUTO_FLUE_COLD_C, &burner_settings::AUTO_FLUE_FALL_C, &burner_settings::AUTO_TOO_HOT_C, &burner_settings::AUTO_ALARM_C, &burner_settings::AUTO_SURE_S,
        &burner_settings::AUTO_EMBER_C, &burner_settings::AUTO_SPIN_UP_S, &burner_settings::AUTO_LOW_PRESSURE_S];
    // the table of steps: a dose, a pause and a draught on each of the seven levels
    for row in &burner_settings::STEP_TABLE {
        listed.extend([row.on_s, row.off_s, row.pa]);
    }
    for s in &listed {
        assert_eq!(s.card, Some("automaton"), "{}", s.name);
        assert!(ALL.iter().any(|a| a.name == s.name), "{} is not in ALL, so no device can seed it", s.name);
    }
    // the mode, the setpoint, the hysteresis and the level are the thermostat's: on its card, and not on the automaton's
    for s in [&burner_settings::AUTO_HOLD, &burner_settings::AUTO_HOLD_C, &burner_settings::AUTO_DROP_C, &burner_settings::AUTO_STEP] {
        assert_eq!(s.card, Some("thermostat"), "{}", s.name);
        assert!(ALL.iter().any(|a| a.name == s.name), "{} is not in ALL, so no device can seed it", s.name);
    }
    // the other way round, so the list above cannot fall behind the card: every setting ALL puts on the card is in it
    let unlisted: Vec<&str> =
        ALL.iter().filter(|a| a.card == Some("automaton") && !listed.iter().any(|s| s.name == a.name)).map(|a| a.name).collect();
    assert!(unlisted.is_empty(), "settings of the automaton's card this test does not know: {unlisted:?}");
}

#[test]
fn lighting_holds_a_draught_of_its_own_beside_the_step() {
    // the owner's word 23.09: lighting is turned down without touching the burning
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    let step = value(&burner_settings::AUTO_6_PA, &store);
    let other = if value(&burner_settings::AUTO_KINDLE_PA, &store) == "42" { "43" } else { "42" };
    change(&ALL, &RULES, &mut store, "auto_kindle_pa", other).unwrap();
    assert_eq!(value(&burner_settings::AUTO_KINDLE_PA, &store), other);
    assert_eq!(value(&burner_settings::AUTO_6_PA, &store), step, "turning the lighting down moved the step too");
}

#[test]
fn the_draught_of_lighting_takes_no_more_than_the_sensor_reads_and_no_word() {
    for word in ["301", "-1", "fifty", "", "51 Pa"] {
        refuses(&burner_settings::AUTO_KINDLE_PA, word);
    }
    assert_eq!(takes(&burner_settings::AUTO_KINDLE_PA, "0"), "0", "a lighting in still air is a number a person may choose");
}

#[test]
fn a_person_moves_the_pump_s_threshold() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    change(&ALL, &RULES, &mut store, "auto_pump_on_c", "55").unwrap();
    assert_eq!(value(&burner_settings::AUTO_PUMP_ON_C, &store), "55");
}

#[test]
fn the_pump_takes_no_threshold_outside_the_water_a_boiler_holds() {
    for word in ["19", "86", "-5", "warm", ""] {
        refuses(&burner_settings::AUTO_PUMP_ON_C, word);
    }
}

#[test]
fn the_automaton_has_no_mode_to_set() {
    // the owner, 23.09: the automaton switches the organs for real, always; the mode where it did not is gone, and a
    // board that kept its word in nvs keeps it among the names this image does not know
    assert!(!ALL.iter().any(|s| s.name == "auto_mode"), "the automaton's card still offers a mode");
}

// ---------------------------------------------------------------- what the burner holds
//
// The mode and its numbers are settings of the automaton's card: seeded with what the burner did before the mode
// existed, kept when a person chose otherwise, and a mode by a sensor is not taken while no socket plays its role.

/// A setting of the card by its name, or the plain reason it is missing.
fn on_the_card(name: &str) -> &'static Setting {
    find(&ALL, name).unwrap_or_else(|| panic!("no setting '{name}' on the automaton's card"))
}

#[test]
fn a_new_board_holds_the_boiler_at_60_and_lights_again_4_under_it() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    assert_eq!(value(on_the_card("auto_hold"), &store), "boiler");
    assert_eq!(value(on_the_card("auto_hold_c"), &store), "60");
    assert_eq!(value(on_the_card("auto_drop_c"), &store), "4");
    assert_eq!(value(on_the_card("auto_pump_off_c"), &store), "4", "the pump stood off 4 under its threshold before it had a number");
}

#[test]
fn a_mode_chosen_in_place_outlives_the_seeding_of_a_new_image() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    change(&ALL, &RULES, &mut store, "role_ntc_2", "return").unwrap();
    change(&ALL, &RULES, &mut store, "auto_hold", "return").unwrap();
    change(&ALL, &RULES, &mut store, "auto_hold_c", "40").unwrap();
    seed(&ALL, &RULES, &mut store, &[]);
    assert_eq!(value(on_the_card("auto_hold"), &store), "return", "the seeding took the mode a person chose");
    assert_eq!(value(on_the_card("auto_hold_c"), &store), "40");
}

#[test]
fn a_mode_by_a_sensor_is_taken_when_a_socket_plays_its_role_and_refused_when_none_does() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    on_the_card("auto_hold");
    let refused = change(&ALL, &RULES, &mut store, "auto_hold", "return").expect_err("no socket measures the return and the mode was taken");
    assert!(refused.contains("coming back"), "the console's line does not name the role: {refused}");
    assert_eq!(value(on_the_card("auto_hold"), &store), "boiler", "a refused mode changed the setting");
    change(&ALL, &RULES, &mut store, "role_ntc_4", "return").unwrap();
    assert!(change(&ALL, &RULES, &mut store, "auto_hold", "return").is_ok(), "the return is measured now and the mode was refused");
    assert_eq!(value(on_the_card("auto_hold"), &store), "return");
}

#[test]
fn a_room_that_asks_is_taken_when_a_socket_plays_the_room_and_refused_when_none_does() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    let refused = change(&ALL, &RULES, &mut store, "auto_asked_by", "room").expect_err("no socket measures the room and the source was taken");
    assert!(refused.contains("room"), "the console's line does not name the role: {refused}");
    assert_eq!(value(on_the_card("auto_asked_by"), &store), "none", "a refused source changed the setting");
    change(&ALL, &RULES, &mut store, "role_ntc_4", "room").unwrap();
    assert!(change(&ALL, &RULES, &mut store, "auto_asked_by", "room").is_ok(), "the room is measured now and the source was refused");
    assert_eq!(value(on_the_card("auto_asked_by"), &store), "room");
    assert!(change(&ALL, &RULES, &mut store, "auto_asked_by", "schedule").is_ok(), "the schedule needs no sensor");
}

#[test]
fn the_mode_of_a_level_needs_no_sensor() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    on_the_card("auto_hold");
    let taken = change(&ALL, &RULES, &mut store, "auto_hold", "power");
    assert!(taken.is_ok(), "power holds no temperature and was refused: {taken:?}");
    assert_eq!(value(on_the_card("auto_hold"), &store), "power");
}

#[test]
fn the_numbers_of_a_mode_take_their_limits_and_no_word() {
    for (name, ok, bad) in [("auto_hold_c", ["5", "85"], ["4", "86", "sixty"]), ("auto_drop_c", ["1", "20"], ["0", "21", "4 C"])] {
        let s = on_the_card(name);
        for text in ok {
            assert_eq!(check(s, text), Ok(text.to_string()), "{name}: {text}");
        }
        for text in bad {
            assert!(check(s, text).is_err(), "{name}: '{text}' was taken");
        }
    }
}

#[test]
fn the_mode_and_its_numbers_are_work_on_the_thermostats_card_and_the_automatons_card_is_tuning() {
    let work = ["auto_hold", "auto_hold_c", "auto_drop_c", "auto_step"];
    for name in work {
        assert_eq!(on_the_card(name).level, settings::Level::Work, "{name}");
        assert_eq!(on_the_card(name).card, Some("thermostat"), "{name}");
    }
    let tuning: Vec<&str> = ALL
        .iter()
        .filter(|s| s.card == Some("automaton") && !work.contains(&s.name))
        .filter(|s| s.level != settings::Level::Tuning)
        .map(|s| s.name)
        .collect();
    assert!(tuning.is_empty(), "settings of the automaton's card standing among the everyday ones: {tuning:?}");
    // the pump's number is the pump's: it stands with its threshold, in the tuning
    assert_eq!(on_the_card("auto_pump_off_c").level, settings::Level::Tuning);
    assert_eq!(on_the_card("auto_pump_off_c").card, Some("automaton"));
}

#[test]
fn a_setting_of_another_card_stays_where_it_was_the_day_before_the_level() {
    // the engine's settings and the burner's other cards are everyday settings: only the automaton's card folds
    let folded: Vec<&str> = ALL.iter().filter(|s| s.card != Some("automaton") && s.level == settings::Level::Tuning).map(|s| s.name).collect();
    assert!(folded.is_empty(), "settings of other cards folded away: {folded:?}");
}

#[test]
fn a_mode_by_a_sensor_is_taken_in_the_same_save_that_gives_a_socket_its_role() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    on_the_card("auto_hold");
    let said = change_all(&ALL, &RULES, &mut store, &[("role_ntc_2", "return"), ("auto_hold", "return")]);
    assert!(said.is_ok(), "the role and the mode came in one save: {said:?}");
    assert_eq!(value(on_the_card("auto_hold"), &store), "return");
}

#[test]
fn a_save_that_takes_the_role_away_from_the_mode_it_holds_is_taken_and_the_mode_stays() {
    // the burner then burns off by the silent sensor and says its name: a setting that snapped back to another mode
    // by itself would light a boiler by a sensor nobody chose
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    on_the_card("auto_hold");
    change(&ALL, &RULES, &mut store, "role_ntc_2", "return").unwrap();
    change(&ALL, &RULES, &mut store, "auto_hold", "return").unwrap();
    change(&ALL, &RULES, &mut store, "role_ntc_2", "free").expect("the socket was let go while its mode was held");
    assert_eq!(value(on_the_card("auto_hold"), &store), "return");
}

#[test]
fn the_steps_of_the_level_are_the_ones_the_research_gives_and_no_others() {
    let s = on_the_card("auto_step");
    for kw in ["14", "10", "8", "6", "5", "4", "3"] {
        assert_eq!(check(s, kw), Ok(kw.to_string()), "{kw} kW");
    }
    for kw in ["2", "15", "20", "six", ""] {
        assert!(check(s, kw).is_err(), "'{kw}' was taken as a step");
    }
    assert_eq!(value(s, &{
        let mut store = Map::default();
        seed(&ALL, &RULES, &mut store, &[]);
        store
    }), "6");
}

// ---- what a choice bears on ----

fn offers(setting: &Setting, choice: &str) -> bool {
    matches!(&setting.kind, Kind::Choice(list) if list.iter().any(|c| c.value == choice))
}

#[test]
fn every_bearing_names_a_setting_of_the_list_a_choice_it_offers_and_settings_of_its_own_card() {
    for b in burner_settings::BEARINGS.iter().chain(burner_settings::BEARINGS_WITH_LEVELS.iter()) {
        let s = ALL.iter().find(|s| s.name == b.setting).unwrap_or_else(|| panic!("no setting {}", b.setting));
        assert!(offers(s, b.choice), "{} never offers {}", b.setting, b.choice);
        for shown in b.shows {
            let other = ALL.iter().find(|o| o.name == *shown).unwrap_or_else(|| panic!("{} / {} shows {shown}, which is no setting", b.setting, b.choice));
            assert_eq!(other.card, s.card, "{} / {} shows {shown} of another card", b.setting, b.choice);
        }
        if let Some(role) = b.role {
            for socket in burner_settings::SOCKET_ROLES {
                let s = ALL.iter().find(|s| s.name == socket).unwrap_or_else(|| panic!("no socket {socket}"));
                assert!(offers(s, role), "{socket} never holds {role}");
            }
        }
    }
}

#[test]
fn a_mode_by_a_sensor_needs_the_role_of_its_sensor_and_a_level_of_power_brings_nothing() {
    let needs = |mode: &str| burner_settings::bearing("auto_hold", mode).map(|b| b.role);
    // the level the burner burns on stands in every mode, so the mode that holds no temperature brings no number of its own
    assert_eq!(needs("power"), None);
    for b in burner_settings::BEARINGS.iter().chain(burner_settings::BEARINGS_WITH_LEVELS.iter()) {
        assert!(!b.shows.contains(&"auto_step"), "{} / {} shows the level, which stands in every mode", b.setting, b.choice);
    }
    assert_eq!(needs("boiler"), Some(Some("boiler")));
    assert_eq!(needs("return"), Some(Some("return")));
    assert_eq!(needs("room"), None, "the room is no mode of the water");
}

#[test]
fn a_room_that_asks_needs_the_role_of_the_room_and_the_schedule_and_nobody_need_none() {
    let by = |source: &str| burner_settings::bearing_with_levels("auto_asked_by", source).map(|b| b.role);
    assert_eq!(by("room"), Some(Some("room")));
    assert_eq!(by("schedule"), Some(None));
    assert_eq!(by("none"), None, "nobody asking brings nothing");
}

#[test]
fn a_choice_no_bearing_names_bears_on_nothing() {
    assert!(burner_settings::bearing("auto_hold", "nonsense").is_none());
    assert!(burner_settings::bearing("zone", "kyiv").is_none());
    assert!(burner_settings::bearing("auto_step", "6").is_none());
}

// ---- the numbers of the fire are settings ----

fn named(name: &str) -> &'static Setting {
    find(&ALL, name).unwrap_or_else(|| panic!("no setting {name}"))
}

#[test]
fn the_protections_take_a_value_inside_their_span_and_refuse_one_outside_it() {
    for (name, good, bad) in [
        ("auto_too_hot_c", ["70", "88", "90"], ["69", "91", "0"]),
        ("auto_alarm_c", ["71", "95", "99"], ["70", "100", "150"]),
    ] {
        for text in good {
            assert_eq!(check(named(name), text), Ok(text.to_string()), "{name} = {text}");
        }
        for text in bad {
            assert!(check(named(name), text).is_err(), "{name} = {text} was taken");
        }
    }
}

#[test]
fn the_times_and_the_thresholds_of_the_fire_take_their_span_and_refuse_what_is_no_number_of_it() {
    for (name, good, bad) in [
        ("auto_smoulder_s", ["0", "900", "7200"], ["7201", "-1", "abc"]),
        ("auto_smould_on", ["1", "2", "60"], ["0", "61", ""]),
        ("auto_sure_s", ["1", "10", "120"], ["0", "121", "x"]),
        ("auto_flue_c", ["50", "100", "200"], ["49", "201", "hot"]),
        // the cold mark, the fall and its window, and the hold of a flame: the span of each, and what is no number of it
        ("auto_flue_cold", ["30", "70", "150"], ["29", "151", "cold"]),
        ("auto_flue_fall", ["2", "10", "50"], ["1", "51", "x"]),
        ("auto_flue_win_s", ["60", "600", "1800"], ["59", "1801", "x"]),
        ("auto_lit_s", ["1", "5", "60"], ["0", "61", "x"]),
    ] {
        for text in good {
            assert_eq!(check(named(name), text), Ok(text.to_string()), "{name} = {text}");
        }
        for text in bad {
            assert!(check(named(name), text).is_err(), "{name} = {text} was taken");
        }
    }
}

#[test]
fn a_board_of_the_image_before_gets_the_new_numbers_seeded_with_what_it_burned_by_and_keeps_what_a_person_set() {
    let before: Vec<&'static Setting> = ALL.iter().copied().filter(|s| !NEW_FIRE.contains(&s.name)).collect();
    let mut store = Map::default();
    seed(&before, &RULES, &mut store, &[]);
    change(&before, &RULES, &mut store, "auto_hold_c", "55").unwrap();
    for name in NEW_FIRE {
        assert_eq!(settings::Values::of(&ALL, &store).get(named(name)), named(name).default, "{name} before the seeding is the default, nothing is stored");
    }
    seed(&ALL, &RULES, &mut store, &[]);
    let now = settings::Values::of(&ALL, &store);
    assert_eq!(now.get(named("auto_hold_c")), "55", "what a person set is kept");
    assert_eq!(now.get(named("auto_smoulder_s")), "900");
    for name in NEW_FIRE {
        assert_eq!(now.get(named(name)), named(name).default, "{name}");
    }
}

#[test]
fn a_number_of_the_fire_a_person_set_is_kept_through_a_seeding_of_a_new_image() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    change(&ALL, &RULES, &mut store, "auto_smoulder_s", "1200").unwrap();
    change(&ALL, &RULES, &mut store, "auto_alarm_c", "97").unwrap();
    seed(&ALL, &RULES, &mut store, &[("auto_smoulder_s", "300"), ("auto_alarm_c", "90")]);
    let now = settings::Values::of(&ALL, &store);
    assert_eq!((now.get(named("auto_smoulder_s")).as_str(), now.get(named("auto_alarm_c")).as_str()), ("1200", "97"));
}

#[test]
fn a_board_of_the_image_before_6_6_reads_the_new_flue_and_flame_numbers_by_default_and_keeps_what_it_held_of_the_old_ones_aside() {
    // 27.09: the board holds `auto_no_fuel_s` 1800 (the safeguard of that day) and the rise of the flue; the image that asks the flue another question has neither, and must not read the 1800 as the window it asks now
    let key = key_of(Some("automaton"));
    let mut store = Map::default();
    store.put(&key, "auto_no_fuel_s=1800\nauto_flue_rise=3\nauto_flue_c=90").unwrap();
    seed(&ALL, &RULES, &mut store, &[]);
    for (name, default) in [("auto_flue_cold", "70"), ("auto_flue_fall", "10"), ("auto_flue_win_s", "600"), ("auto_lit_s", "5")] {
        assert_eq!(value(named(name), &store), default, "{name} on a board that kept the old numbers");
    }
    assert_eq!(value(named("auto_flue_c"), &store), "90", "what a person set of a setting that stayed is kept");
    let left = store.0.get(settings::LEFT).cloned().unwrap_or_default();
    assert!(left.contains("auto_no_fuel_s=1800") && left.contains("auto_flue_rise=3"), "the numbers of the old rule were not kept aside: {left:?}");
    assert!(ALL.iter().all(|s| s.name != "auto_no_fuel_s" && s.name != "auto_flue_rise"), "the image still lists the old rule's numbers");
}

const NEW_FIRE: [&str; 12] = [
    "auto_purge_end", "auto_out_min_s", "auto_smoulder_s", "auto_smould_pa", "auto_smould_on", "auto_smould_off", "auto_flue_c", "auto_too_hot_c", "auto_alarm_c", "auto_sure_s", "auto_ember_c", "auto_spin_up_s",
];

// ---- a role is played when a socket is set to it ----

#[test]
fn a_role_is_played_when_a_socket_is_set_to_it_and_by_nobody_otherwise_and_one_role_is_never_taken_for_another() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    let played = |store: &Map, role: &str| burner_settings::plays(role, &|s: &Setting| value(s, store));
    assert!(!played(&store, "return") && !played(&store, "room"), "a board that has set no socket plays no role");
    change(&ALL, &RULES, &mut store, "role_ntc_3", "return").unwrap();
    assert!(played(&store, "return"));
    assert!(!played(&store, "room"), "a role nobody has set is not played");
    change(&ALL, &RULES, &mut store, "role_ntc_2", "supply").unwrap();
    assert!(!played(&store, "boiler"), "the supply was taken for the boiler");
    change(&ALL, &RULES, &mut store, "role_ntc_3", "free").unwrap();
    assert!(!played(&store, "return"), "a socket let go still plays its role");
    change(&ALL, &RULES, &mut store, "role_ntc_7", "return").unwrap();
    assert!(played(&store, "return"), "the seventh socket is a socket too");
}

#[test]
fn the_pellet_rate_starts_from_four_grams_a_second_the_owner_weighed() {
    let mut store = Map::default();
    seed(&[&PELLET_RATE], &RULES, &mut store, &[]);
    assert_eq!(value(&PELLET_RATE, &store), "240", "grams a minute: 4 g/s of 6.3");
}

#[test]
fn the_pellet_rate_takes_a_whole_number_of_grams_a_minute_from_a_minute_of_a_slow_auger_to_ten_grams_a_second() {
    for text in ["60", "240", " 600 "] {
        assert!(check(&PELLET_RATE, text).is_ok(), "{text}");
    }
    for text in ["0", "59", "601", "-5", "3.5", "", "4 g/s"] {
        let why = check(&PELLET_RATE, text).expect_err(text).wire();
        assert_eq!(why["name"], "pellet_rate", "{text}");
    }
}

#[test]
fn the_pellet_rate_lies_in_the_card_of_the_pellets_and_a_description_may_seed_it() {
    assert_eq!(PELLET_RATE.card, Some("pellets"));
    assert_eq!(check_description(&ALL, &[("pellet_rate", "216")]), Ok(()));
    assert!(check_description(&ALL, &[("pellet_rate", "30")]).is_err());
}

#[test]
fn a_rate_set_in_place_outlives_the_seeding_of_a_new_image() {
    let mut store = Map::default();
    let list = [&PELLET_RATE];
    seed(&list, &RULES, &mut store, &[]);
    change(&list, &RULES, &mut store, "pellet_rate", "216").unwrap();
    seed(&list, &RULES, &mut store, &[("pellet_rate", "240")]);
    assert_eq!(value(&PELLET_RATE, &store), "216");
}

#[test]
fn the_term_of_low_pressure_is_taken_from_a_second_to_two_minutes_and_by_nothing_else() {
    let term = &burner_settings::AUTO_LOW_PRESSURE_S;
    assert_eq!(term.name, "auto_low_pa_s");
    assert_eq!(term.default, "20");
    for good in ["1", "20", "120"] {
        takes(term, good);
    }
    for bad in ["0", "121", "-3", "abc", ""] {
        refuses(term, bad);
    }
}

#[test]
fn the_edges_of_the_soot_scale_start_from_the_clean_boiler_and_the_dirty_one_and_lie_in_the_soot_card() {
    assert_eq!(burner_settings::SOOT_CLEAN_C.default, "36");
    assert_eq!(burner_settings::SOOT_DIRTY_C.default, "51");
    assert_eq!(burner_settings::SOOT_CLEAN_C.card, Some("soot"));
    assert_eq!(burner_settings::SOOT_DIRTY_C.card, Some("soot"));
    assert_eq!(check_description(&ALL, &[("soot_clean_c", "38"), ("soot_dirty_c", "65")]), Ok(()));
}

#[test]
fn the_clean_edge_can_never_stand_at_or_above_the_dirty_one() {
    // the spans do not meet, so whatever pair a person types, the scale has a width and the page never divides by nothing
    assert_eq!(takes(&burner_settings::SOOT_CLEAN_C, "50"), "50");
    assert_eq!(takes(&burner_settings::SOOT_DIRTY_C, "51"), "51");
    refuses(&burner_settings::SOOT_CLEAN_C, "51");
    refuses(&burner_settings::SOOT_DIRTY_C, "50");
    refuses(&burner_settings::SOOT_CLEAN_C, "forty");
    refuses(&burner_settings::SOOT_DIRTY_C, "");
}
