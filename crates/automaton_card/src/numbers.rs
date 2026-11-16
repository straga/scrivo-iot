//! The numbers of the fire, from where they are kept: the automaton's card of settings, changed on the page, and the
//! numbers fixed in the image.
//!
//! **ONE MAPPING FOR THE BOARD AND THE STAND**, as the card is: the board reads the values from nvs and its fixed list,
//! the stand from a board's `/state`, and both hand them in here, so which setting feeds which part of the fire is
//! written once.

use automaton::{Feed, Lighting, Step, Tuning};
use settings::Setting;
use thermostat::Hold;

/// Every number of the fire: CARD gives a setting of the automaton's card as it is kept, in text - one a number does not
/// read from is NaN, which the automaton's door refuses by name -, FIXED a number fixed in the image by its name.
pub fn tuning(text: &dyn Fn(&Setting) -> String, fixed: &dyn Fn(&str) -> f32) -> Tuning {
    let card = |s: &Setting| text(s).parse::<f32>().unwrap_or(f32::NAN);
    // THE LIGHTING POURS ONE DOSE AND RISING FEEDS NOTHING; THE WORK FEEDS BY A PAIR OF ITS OWN:
    // the dose fills the pot above the plug, and the fire eats it before anything is added
    let kindling = Lighting { pa: card(&burner_settings::AUTO_KINDLE_PA), dose_s: card(&burner_settings::AUTO_DOSE) };
    let pair = |named: &str| Step {
        pa: fixed(&format!("{named}_pa")),
        feed: Feed { on_s: fixed(&format!("{named}_on_s")), off_s: fixed(&format!("{named}_off_s")) },
    };
    Tuning {
        purge_before_s: card(&burner_settings::AUTO_PURGE),
        purge_after_s: fixed("purge_after_s"),
        igniter_s: card(&burner_settings::AUTO_IGNITER),
        wait_for_flame_s: card(&burner_settings::AUTO_WAIT),
        // a count the card could not take is NaN, and NaN is no attempts at all: the door refuses it
        tries: card(&burner_settings::AUTO_TRIES) as u32,
        // a setting in whole per cent, a share of the lighting's dose to the automaton
        dose_after_first: card(&burner_settings::AUTO_DOSE_AGAIN) / 100.0,
        rise_s: card(&burner_settings::AUTO_RISE_S),
        flame_lost_s: card(&burner_settings::AUTO_NO_FLAME_S),
        smoulder_s: fixed("smoulder_s"),
        burn_off_s: card(&burner_settings::AUTO_BURN_OFF_S),
        burn_out_min_s: fixed("burn_out_min_s"),
        pump_on_c: card(&burner_settings::AUTO_PUMP_ON_C),
        flue_holds_c: fixed("flue_holds_c"),
        flue_cold_c: fixed("flue_cold_c"),
        flue_fall_c: fixed("flue_fall_c"),
        flue_fall_s: card(&burner_settings::AUTO_FLUE_FALL_S),
        lit_s: fixed("lit_s"),
        pump_off_below_c: card(&burner_settings::AUTO_PUMP_OFF_C),
        too_hot_c: fixed("too_hot_c"),
        alarm_c: fixed("alarm_c"),
        sure_s: fixed("sure_s"),
        relight_s: [card(&burner_settings::AUTO_RELIGHT_1), card(&burner_settings::AUTO_RELIGHT_2), card(&burner_settings::AUTO_RELIGHT_3)],
        relight_hot_c: fixed("relight_hot_c"),
        spin_up_s: fixed("spin_up_s"),
        // the first burner has no such number on its card: the term the new one starts from
        low_pressure_s: 20.0,
        kindling,
        smoulder: pair("smoulder"),
    }
}

/// What the thermostat decides by: the setpoint, the hysteresis and the mode of the same card, the pair the first burner works a hold by a temperature on (its own, beside the table), and the level of power of its table.
pub fn heat_tuning(text: &dyn Fn(&Setting) -> String) -> thermostat::Tuning {
    let card = |s: &Setting| text(s).parse::<f32>().unwrap_or(f32::NAN);
    let step = Step {
        pa: card(&burner_api::first_burner::AUTO_PA),
        feed: Feed { on_s: card(&burner_api::first_burner::AUTO_FEED_ON_S), off_s: card(&burner_api::first_burner::AUTO_FEED_OFF_S) },
    };
    thermostat::Tuning {
        // a mode the card cannot hold never arrives: what a setting keeps is what its check took, and a kept text it does not
        // take reads as the default (`settings::value`)
        hold: match text(&burner_settings::AUTO_HOLD).as_str() {
            "power" => Hold::Power,
            "return" => Hold::Return,
            _ => Hold::Boiler,
        },
        want_c: card(&burner_settings::AUTO_HOLD_C),
        hysteresis_c: card(&burner_settings::AUTO_DROP_C),
        step,
        // THE FIRST BURNER HAS NO STEPS DOWN: its three levels are the one it works on, and the two distances are numbers nothing reads
        mid: step,
        small: step,
        mid_c: 8.0,
        small_c: 3.0,
        power: power_step(&text(&burner_settings::AUTO_STEP)),
        // the seconds of a steady flame that make a fire are the automaton's own `sure_s`, from the same setting
        lockout: thermostat::Lockout { sure_s: card(&burner_settings::AUTO_SURE_S), ..thermostat::Lockout::default() },
    }
}

/// The pair and the draught of the level KW kilowatts of fuel: 2 s of dose, the pause from the period `136 / kW`, the draught
/// in proportion to the power from the factory's point of 14 kW at 51 Pa. A
/// level the table has not is NaN in every number, which the automaton's door refuses by name.
fn power_step(kw: &str) -> Step {
    let (pa, off_s) = match kw {
        "14" => (51.0, 8.0),
        "10" => (36.0, 12.0),
        "8" => (29.0, 15.0),
        "6" => (22.0, 21.0),
        "5" => (18.0, 25.0),
        "4" => (15.0, 32.0),
        "3" => (11.0, 43.0),
        _ => (f32::NAN, f32::NAN),
    };
    Step { pa, feed: Feed { on_s: if pa.is_nan() { f32::NAN } else { 2.0 }, off_s } }
}
