//! The numbers the rows of the automaton's card burn by: the card's settings and the numbers an image fixes, written
//! here, handed through the mapping the board and the stand share (`automaton_card::tuning`). What a board holds is a
//! person's to set; the rows are about what the automaton does with such numbers.

#![allow(dead_code)]

use std::collections::HashMap;

use settings::Setting;

#[path = "../../../automaton/tests/common/mod.rs"]
pub mod pair;

use pair::Sound;

/// The automaton card these rows burn by: every setting it has, by name.
pub const CARD: [(&str, f32); 22] = [
    ("auto_hold_c", 60.0),
    ("auto_drop_c", 4.0),
    ("auto_step", 6.0),
    ("auto_pump_off_c", 4.0),
    ("auto_purge", 30.0),
    ("auto_igniter", 240.0),
    ("auto_wait", 90.0),
    ("auto_tries", 3.0),
    ("auto_kindle_pa", 51.0),
    ("auto_pa", 51.0),
    ("auto_dose", 32.0),
    ("auto_dose_again", 10.0),
    ("auto_rise_s", 60.0),
    ("auto_no_flame_s", 120.0),
    ("auto_flue_win_s", 600.0),
    ("auto_burn_off_s", 600.0),
    ("auto_feed_on_s", 2.0),
    ("auto_feed_off_s", 20.0),
    ("auto_pump_on_c", 35.0),
    ("auto_relight_1", 20.0),
    ("auto_relight_2", 10.0),
    ("auto_relight_3", 10.0),
];

/// The numbers fixed in an image, as these rows fix them.
pub const FIXED: [(&str, f32); 15] = [
    ("purge_after_s", 20.0),
    ("burn_out_min_s", 60.0),
    ("smoulder_s", 300.0),
    ("smoulder_pa", 0.0),
    ("smoulder_on_s", 1.0),
    ("smoulder_off_s", 50.0),
    ("flue_holds_c", 100.0),
    ("flue_cold_c", 70.0),
    ("flue_fall_c", 10.0),
    ("lit_s", 5.0),
    ("too_hot_c", 88.0),
    ("alarm_c", 95.0),
    ("sure_s", 10.0),
    ("relight_hot_c", 45.0),
    ("spin_up_s", 10.0),
];

/// The rows' card with CHANGED over it; a name neither holds is NaN, which the door refuses by name.
pub fn sound(changed: &[(&str, f32)]) -> Sound {
    let card: HashMap<&str, f32> = CARD.iter().chain(changed.iter()).copied().collect();
    let fixed: HashMap<&str, f32> = FIXED.iter().copied().collect();
    let card = |s: &Setting| card.get(s.name).map_or(String::new(), |n| n.to_string());
    let fixed = |name: &str| fixed.get(name).copied().unwrap_or(f32::NAN);
    Sound::of(automaton_card::tuning(&card, &fixed), automaton_card::heat_tuning(&card)).unwrap_or_else(|refused| panic!("refused: {}", refused.words()))
}

