//! The card's settings as the automaton takes them: the mode, the setpoint and its hysteresis, the level, the
//! pump's own number. **THE CARD IS THE ONE THE BOARD IS SEEDED WITH** - the settings' own defaults, changed in the
//! rows that change them - so a default nobody set on the card is a default the rows see, and the numbers the burner
//! held before the mode existed are asserted as what a new board holds.

use std::collections::HashMap;

use automaton::{Feed, Step, Tuning};
use settings::Setting;
use thermostat::Hold;

/// The tuning of a board seeded with the defaults and CHANGED over them: the fire's, and the thermostat's of the same card.
fn of(changed: &[(&str, &str)]) -> (Tuning, thermostat::Tuning) {
    let changed: HashMap<&str, &str> = changed.iter().copied().collect();
    let card = |s: &Setting| changed.get(s.name).map_or_else(|| s.default.to_string(), |v| v.to_string());
    (automaton_card::tuning(&card, &burner_api::first_burner::fixed_number), automaton_card::heat_tuning(&card))
}

#[test]
fn a_board_seeded_with_the_defaults_burns_as_it_burned_before_the_mode() {
    let (fire, heat) = of(&[]);
    assert_eq!(heat.hold, Hold::Boiler);
    assert_eq!((heat.want_c, heat.hysteresis_c), (60.0, 4.0));
    assert_eq!(fire.pump_off_below_c, 4.0);
    assert!(fire.sound().is_ok(), "the defaults of the card are not a burner the door takes");
    assert!(heat.sound(fire.too_hot_c).is_ok(), "the defaults of the card are not a thermostat the door takes");
}

#[test]
fn the_mode_and_its_numbers_come_from_the_card_and_not_from_the_image() {
    let (_, t) = of(&[("auto_hold", "return"), ("auto_hold_c", "40"), ("auto_drop_c", "5")]);
    assert_eq!(t.hold, Hold::Return);
    assert_eq!((t.want_c, t.hysteresis_c), (40.0, 5.0));
    assert_eq!(of(&[("auto_hold", "room")]).1.hold, Hold::Boiler, "a text the card does not take - the room was a mode once - reads as the boiler");
    assert_eq!(of(&[("auto_hold", "power")]).1.hold, Hold::Power);
}

#[test]
fn the_pump_number_is_its_own_and_the_setpoints_hysteresis_does_not_move_it() {
    let (fire, heat) = of(&[("auto_drop_c", "10"), ("auto_pump_off_c", "3")]);
    assert_eq!(heat.hysteresis_c, 10.0);
    assert_eq!(fire.pump_off_below_c, 3.0);
}

#[test]
fn every_step_of_the_level_is_the_pair_and_the_draught_the_research_gives() {
    // 2 s of dose, the pause from the period, the draught from 14 kW at 51 Pa
    for (kw, on, off, pa) in [("14", 2.0, 8.0, 51.0), ("10", 2.0, 12.0, 36.0), ("8", 2.0, 15.0, 29.0), ("6", 2.0, 21.0, 22.0),
        ("5", 2.0, 25.0, 18.0), ("4", 2.0, 32.0, 15.0), ("3", 2.0, 43.0, 11.0)]
    {
        assert_eq!(of(&[("auto_step", kw)]).1.power, Step { pa, feed: Feed { on_s: on, off_s: off } }, "{kw} kW");
    }
}

#[test]
fn the_setpoint_and_its_hysteresis_are_no_longer_fixed_in_the_image() {
    assert!(burner_api::first_burner::fixed_number("want_c").is_nan(), "the setpoint is still a number of the image");
    assert!(burner_api::first_burner::fixed_number("hysteresis_c").is_nan(), "the hysteresis is still a number of the image");
    assert_eq!(burner_api::first_burner::fixed_number("too_hot_c"), 88.0, "the protections stay where they were");
}
