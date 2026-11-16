//! The target of the room goes by tenths of a degree: 22.1 is a target, from 6.0 to 35.0 in the comfort profile and in the eco one; what has more than one digit after the point, or lies outside the span, or is no number, is refused with the
//! span in words; a target kept as a whole number by an image before reads as itself with a nought tenth; the hysteresis stays whole; the times of the auger stay in seconds. Expected from what a person does at the panel (22.1, 22.2) and from
//! what the stage says the board takes - not from how a setting is parsed.

use burner_settings::{demand_tuning, ALL};
use serde_json::json;
use settings::{check, words, Refusal, Setting};

fn card(name: &str) -> &'static Setting {
    ALL.iter().copied().find(|s| s.name == name).unwrap_or_else(|| panic!("no setting {name}"))
}

const TARGETS: [&str; 2] = ["auto_room_c", "auto_room_eco_c"];

#[test]
fn a_target_takes_one_digit_after_the_point_from_six_to_thirty_five_and_is_kept_as_typed() {
    for name in TARGETS {
        for typed in ["6", "6.0", "6.1", "22", "22.0", "22.1", "22.9", "34.9", "35", "35.0"] {
            assert_eq!(check(card(name), typed), Ok(typed.to_string()), "{name} {typed}");
        }
    }
}

#[test]
fn what_is_outside_the_span_or_has_two_digits_or_is_no_number_is_refused_as_not_tenths() {
    for name in TARGETS {
        for bad in ["5.9", "5", "35.1", "36", "22.15", "22.", ".", "warm", "", "22,1", " 5x", "-1", "1e1", "22.1.1", "+22"] {
            assert!(matches!(check(card(name), bad), Err(Refusal::NotTenths { .. })), "{name} {bad:?}: {:?}", check(card(name), bad));
        }
    }
}

#[test]
fn the_refusal_says_the_span_in_degrees_and_not_in_seconds() {
    let why = check(card("auto_room_c"), "5.9").unwrap_err();
    assert_eq!(why.wire(), json!({"code": "not_tenths", "name": "auto_room_c", "text": "5.9", "from": "6", "to": "35", "unit": "\u{b0}C"}));
}

#[test]
fn the_hysteresis_stays_whole_from_one_to_five() {
    for ok in ["1", "3", "5"] {
        assert_eq!(check(card("auto_room_gap_c"), ok), Ok(ok.to_string()));
    }
    for bad in ["0", "6", "1.5", "2.0", "warm", ""] {
        assert!(matches!(check(card("auto_room_gap_c"), bad), Err(Refusal::NotWhole { .. })), "{bad:?}");
    }
}

#[test]
fn the_thermostat_reads_a_target_with_a_tenth_as_it_was_set_and_a_whole_one_of_an_image_before_as_itself() {
    let read = |comfort: &'static str, eco: &'static str| {
        demand_tuning(&|s: &Setting| match s.name {
            "auto_asked_by" => "room".to_string(),
            "auto_room_c" => comfort.to_string(),
            "auto_room_eco_c" => eco.to_string(),
            _ => s.default.to_string(),
        })
    };
    let t = read("22.1", "17.5");
    assert_eq!((t.room_c, t.room_eco_c), (22.1, 17.5));
    let old = read("22", "17");
    assert_eq!((old.room_c, old.room_eco_c), (22.0, 17.0), "a number kept as 22 by the image before is 22.0");
}

#[test]
fn a_target_is_said_in_degrees_and_a_time_of_the_auger_still_in_seconds() {
    assert_eq!(words(card("auto_room_c"), "22.1"), "22.1 \u{b0}C");
    assert_eq!(words(card("auto_dose"), "0.8"), "0.8 s");
}

#[test]
fn the_defaults_are_still_what_a_new_board_has_and_pass_their_own_check() {
    for (name, default) in [("auto_room_c", "21"), ("auto_room_eco_c", "17")] {
        assert_eq!(card(name).default, default);
        assert_eq!(check(card(name), default), Ok(default.to_string()));
    }
}
