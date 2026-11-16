//! The profiles by the clock as settings of the demand's card: who asks, the targets of the room and its hysteresis, the profile that stands outside the windows, and six
//! windows, each one line of text. Expected from what the stage says a window is, and from what a board must keep: a window written wrong is refused by its name with the choices that are left, one
//! written right comes back as it went in, the card the six windows make fits the buffer the card is read through, and with none written and nobody asking the boiler is the one there was.

use burner_settings::RULES;
use std::collections::HashMap;

use demand::By;
use schedule::{Profile, Span, Window};
use settings::{change, check, find, seed, value, Refusal, Setting, Store};
use burner_settings::{demand_tuning, thermostat_tuning, ALL, WINDOW_SETTINGS};

fn texts(changed: &[(&str, &str)]) -> impl Fn(&Setting) -> String {
    let changed: HashMap<String, String> = changed.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |s: &Setting| changed.get(s.name).cloned().unwrap_or_else(|| s.default.to_string())
}

fn card(name: &str) -> &'static Setting {
    find(&ALL, name).unwrap_or_else(|| panic!("no setting {name}"))
}

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

// ---------------------------------------------------------------- what a new board has

#[test]
fn a_new_board_asks_by_nothing_follows_no_clock_and_stands_on_comfort() {
    let t = demand_tuning(&texts(&[]));
    assert_eq!(t.by, By::Nobody, "a boiler that has not been told who asks works from button to button, as it did");
    assert_eq!(t.schedule.main, Profile::Comfort);
    assert_eq!(t.schedule.windows, [None; 6], "no window written");
    assert_eq!((t.room_c, t.room_eco_c, t.room_drop_c), (21.0, 17.0, 1.0));
}

#[test]
fn the_setpoint_of_the_water_is_the_one_number_the_card_always_had() {
    let t = thermostat_tuning(&texts(&[("auto_hold_c", "57")]));
    assert_eq!(t.want_c, 57.0, "the key and the number of the setpoint are the ones an owner already has, and no profile moves it");
}

// ---------------------------------------------------------------- the settings are read into the tuning

#[test]
fn who_asks_the_targets_and_the_main_profile_are_read_as_set() {
    let t = demand_tuning(&texts(&[("auto_asked_by", "room"), ("auto_room_c", "22"), ("auto_room_eco_c", "18"), ("auto_room_gap_c", "2"), ("auto_profile", "eco")]));
    assert_eq!((t.by, t.room_c, t.room_eco_c, t.room_drop_c, t.schedule.main), (By::Room, 22.0, 18.0, 2.0, Profile::Eco));
    assert_eq!(demand_tuning(&texts(&[("auto_asked_by", "schedule")])).by, By::Schedule);
    assert_eq!(demand_tuning(&texts(&[("auto_asked_by", "contact")])).by, By::Nobody, "a text the card does not take reads as nobody");
}

#[test]
fn a_window_written_is_read_into_the_place_it_stands_in() {
    let t = demand_tuning(&texts(&[("window_2", "set:mon-fri 22:00-06:00 eco"), ("window_5", "set:daily 07:00+3h/20m comfort")]));
    let w = t.schedule.windows;
    assert_eq!(w[0], None);
    assert_eq!(w[1], Some(Window { days: 0b001_1111, span: Span::Between { from: 1320, to: 360 }, profile: Some(Profile::Eco) }));
    assert_eq!(w[2], None);
    assert_eq!(w[3], None);
    assert_eq!(w[4], Some(Window { days: 0b111_1111, span: Span::Every { from: 420, every_h: 3, for_min: 20 }, profile: Some(Profile::Comfort) }));
    assert_eq!(w[5], None);
}

#[test]
fn a_window_that_is_not_used_is_no_window_and_a_text_the_card_cannot_take_is_none_too() {
    let t = demand_tuning(&texts(&[("window_1", "off"), ("window_2", "set:nonsense")]));
    assert_eq!((t.schedule.windows[0], t.schedule.windows[1]), (None, None));
}

// ---------------------------------------------------------------- what a window takes

#[test]
fn a_window_is_taken_as_written_and_kept_with_what_it_is() {
    for text in ["daily 22:00-06:00 eco", "mon-fri 06:30-08:00 comfort", "sat-sun 08:00-10:00 off", "daily 07:00+3h/20m comfort", "mon,wed,fri 07:00-09:00 none"] {
        for setting in WINDOW_SETTINGS {
            assert_eq!(check(setting, text).as_deref(), Ok(format!("set:{text}").as_str()), "{} takes {text:?}", setting.name);
        }
    }
}

#[test]
fn a_window_not_used_is_taken_by_its_value_and_is_what_a_new_board_has() {
    assert_eq!(check(&burner_settings::WINDOW_1, "off"), Ok("off".to_string()));
    assert_eq!(burner_settings::WINDOW_1.default, "off");
}

#[test]
fn the_capitals_of_a_window_are_not_a_different_window() {
    assert_eq!(check(&burner_settings::WINDOW_1, " Mon-Fri 06:30-08:00 COMFORT "), Ok("set:mon-fri 06:30-08:00 comfort".to_string()));
}

#[test]
fn what_is_no_window_is_refused_by_the_name_of_the_setting_with_the_choices_there_are() {
    for text in ["", "set", "daily", "daily 22:00-22:00 eco", "daily 22:00-06:00", "daily 22:00-06:00 warm", "daily 07:00+0h/20m comfort", "someday 22:00-06:00 eco"] {
        match check(&burner_settings::WINDOW_3, text) {
            Err(Refusal::NoneOf { name, choices, own, .. }) => {
                assert_eq!(name, "window_3", "{text:?}");
                assert_eq!(choices, ["off", "set"]);
                assert_eq!(own, Some("set"), "the page is told which choice takes a line of its own");
            }
            other => panic!("{text:?} is no window and was not refused: {other:?}"),
        }
    }
}

#[test]
fn a_window_longer_than_a_line_is_refused_even_when_it_is_a_window() {
    let padded = format!("daily{}22:00-06:00 eco", " ".repeat(80));
    assert!(schedule::Window::parse(&padded).is_ok(), "the grammar alone takes any run of spaces");
    assert!(matches!(check(&burner_settings::WINDOW_1, &padded), Err(Refusal::NoneOf { .. })), "the card does not keep it");
}

#[test]
fn a_window_a_person_saves_comes_back_the_way_it_was_kept() {
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    change(&ALL, &RULES, &mut store, "window_4", "sat-sun 08:00-10:00 comfort").expect("a window is saved");
    assert_eq!(value(&burner_settings::WINDOW_4, &store), "set:sat-sun 08:00-10:00 comfort");
    assert_eq!(value(&burner_settings::WINDOW_1, &store), "off", "the other windows are as they were");
    let t = demand_tuning(&|s: &Setting| value(s, &store));
    assert_eq!(t.schedule.windows[3].map(|w| w.days), Some(0b110_0000));
}

#[test]
fn the_hysteresis_of_the_room_takes_its_span_as_a_whole_number_and_no_word() {
    // the targets of the room go by tenths of a degree (tests/target_tenths.rs); the hysteresis stays whole, because a tenth of it would light the fire oftener and a light costs
    for (name, from, to) in [("auto_room_gap_c", 1, 5)] {
        assert_eq!(check(card(name), &from.to_string()), Ok(from.to_string()), "{name}");
        assert_eq!(check(card(name), &to.to_string()), Ok(to.to_string()), "{name}");
        for bad in [(from - 1).to_string(), (to + 1).to_string(), "".into(), "warm".into(), "2.5".into(), " 5x".into()] {
            assert!(matches!(check(card(name), &bad), Err(Refusal::NotWhole { .. })), "{name} {bad:?}");
        }
    }
}

#[test]
fn the_hysteresis_of_the_room_can_never_reach_the_lowest_target_so_the_line_to_say_cold_is_over_nought() {
    let (drop_to, target_from) = (5, 6);
    assert!(drop_to < target_from, "the span of the hysteresis ends under the span of the targets: no order between them is left to a door");
}

#[test]
fn who_asks_takes_a_word_of_its_three_and_no_other() {
    for word in ["none", "room", "schedule"] {
        assert_eq!(check(&burner_settings::AUTO_ASKED_BY, word), Ok(word.into()));
    }
    assert!(matches!(check(&burner_settings::AUTO_ASKED_BY, "contact"), Err(Refusal::NoneOf { .. })), "a contact on a leg is a player of a later stage");
}

#[test]
fn the_main_profile_takes_a_profile_and_no_other_word() {
    for word in ["comfort", "eco", "off"] {
        assert_eq!(check(&burner_settings::AUTO_PROFILE, word), Ok(word.into()));
    }
    assert!(matches!(check(&burner_settings::AUTO_PROFILE, "none"), Err(Refusal::NoneOf { .. })), "none is for a window, which can be written and off; the main profile always stands");
}

// ---------------------------------------------------------------- the card has to fit its buffer

#[test]
fn a_card_with_every_setting_at_its_longest_still_fits_the_buffer_it_is_read_through() {
    // the keys are read through one buffer of 1024 bytes (firmware nvs_settings, MAX_READ): the card of the demand, every window as long as the card allows
    const MAX_READ: usize = 1024;
    let longest_window = format!("mon,tue,wed,thu,fri,sat,sun{}22:00+24h/1440m comfort", " ".repeat(64 - "mon,tue,wed,thu,fri,sat,sun".len() - " 22:00+24h/1440m comfort".len()));
    assert!(longest_window.len() <= 64);
    let mut store = Map::default();
    seed(&ALL, &RULES, &mut store, &[]);
    let mut pairs: Vec<(&str, String)> = WINDOW_SETTINGS.iter().map(|s| (s.name, longest_window.clone())).collect();
    pairs.extend([("auto_asked_by", "schedule".to_string()), ("auto_room_c", "35".into()), ("auto_room_eco_c", "35".into()), ("auto_room_gap_c", "5".into())]);
    let pairs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (*n, v.as_str())).collect();
    settings::change_all(&ALL, &RULES, &mut store, &pairs).expect("saved");
    let kept = store.get("s.demand").expect("the card is kept under its key");
    assert!(kept.len() + 200 <= MAX_READ, "the card is {} bytes of {MAX_READ}: no room left for the settings that come after it", kept.len());
}
