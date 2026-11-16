//! The tuning the burner's automaton and its thermostat are handed: the settings of their cards, as `automaton::Tuning` and `thermostat::Tuning`. Every number of the fire is a setting, and the image fixes none. The one place the
//! two are put together, for the board and for the stand on a computer, so which setting feeds which part of the fire and of the heat is written once. What a number means is crates/automaton; what a person is told of it is the page's.
//!
//! **A NUMBER OF THE FIRE IS THE OWNER'S, NOT THE AUTOMATON'S**: the crate of the automaton holds none of them (its header says why), and the API of the device only gives each a name and a unit.

use automaton::{Feed, Lighting, Step, Tuning};
use demand::By;
use schedule::{Profile, Schedule, Window, WINDOWS};
use thermostat::Hold;

use crate::{WINDOW_SETTINGS, Setting, AUTO_ALARM_C, AUTO_BURN_OFF_S, AUTO_EMBER_C, AUTO_FLUE_C, AUTO_FLUE_COLD_C, AUTO_FLUE_FALL_C, AUTO_FLUE_FALL_S, AUTO_LIT_S, AUTO_OUT_MIN_S, AUTO_PURGE_END, AUTO_SMOULDER_S, AUTO_SMOULD_OFF, AUTO_SMOULD_ON, AUTO_SMOULD_PA, AUTO_SPIN_UP_S, AUTO_LOW_PRESSURE_S, AUTO_SURE_S, AUTO_TOO_HOT_C, AUTO_DOSE, AUTO_DOSE_AGAIN, AUTO_DROP_C, AUTO_HOLD, AUTO_HOLD_C, AUTO_IGNITER, AUTO_KINDLE_PA, AUTO_NO_FLAME_S, AUTO_PUMP_OFF_C, AUTO_PUMP_ON_C, AUTO_PURGE, AUTO_RELIGHT_1, AUTO_RELIGHT_2, AUTO_RELIGHT_3, AUTO_RISE_S, AUTO_ASKED_BY, AUTO_LOCK_C, AUTO_LOCK_MIN, AUTO_MID_C, AUTO_PROFILE, AUTO_ROOM_C, AUTO_ROOM_GAP_C, AUTO_ROOM_ECO_C, AUTO_SMALL_C, AUTO_STEP, AUTO_STEP_MID, AUTO_STEP_SMALL, AUTO_TRIES, CHOSEN, AUTO_WAIT, STEP_TABLE};

/// Every number of the fire: the settings of the automaton's card as VALUE reads them - one a number does not read from is none, which the automaton's door refuses by name.
pub fn automaton_tuning(value: &dyn Fn(&Setting) -> String) -> Tuning {
    let number = |s: &Setting| value(s).parse::<f32>().unwrap_or(f32::NAN);
    Tuning {
        purge_before_s: number(&AUTO_PURGE),
        purge_after_s: number(&AUTO_PURGE_END),
        igniter_s: number(&AUTO_IGNITER),
        wait_for_flame_s: number(&AUTO_WAIT),
        // a count the card could not take is none, and none is no attempts at all: the door refuses it
        tries: number(&AUTO_TRIES) as u32,
        // a setting in whole per cent, a share of the lighting's dose to the automaton
        dose_after_first: number(&AUTO_DOSE_AGAIN) / 100.0,
        rise_s: number(&AUTO_RISE_S),
        flame_lost_s: number(&AUTO_NO_FLAME_S),
        smoulder_s: number(&AUTO_SMOULDER_S),
        burn_off_s: number(&AUTO_BURN_OFF_S),
        burn_out_min_s: number(&AUTO_OUT_MIN_S),
        pump_on_c: number(&AUTO_PUMP_ON_C),
        flue_holds_c: number(&AUTO_FLUE_C),
        flue_cold_c: number(&AUTO_FLUE_COLD_C),
        flue_fall_c: number(&AUTO_FLUE_FALL_C),
        flue_fall_s: number(&AUTO_FLUE_FALL_S),
        lit_s: number(&AUTO_LIT_S),
        pump_off_below_c: number(&AUTO_PUMP_OFF_C),
        too_hot_c: number(&AUTO_TOO_HOT_C),
        alarm_c: number(&AUTO_ALARM_C),
        sure_s: number(&AUTO_SURE_S),
        relight_s: [number(&AUTO_RELIGHT_1), number(&AUTO_RELIGHT_2), number(&AUTO_RELIGHT_3)],
        relight_hot_c: number(&AUTO_EMBER_C),
        spin_up_s: number(&AUTO_SPIN_UP_S),
        low_pressure_s: number(&AUTO_LOW_PRESSURE_S),
        // THE LIGHTING POURS ONE DOSE AND RISING FEEDS NOTHING: the dose fills the pot above the plug, and the fire eats it before anything is added; the work feeds by the level the thermostat orders
        kindling: Lighting { pa: number(&AUTO_KINDLE_PA), dose_s: number(&AUTO_DOSE) },
        smoulder: Step { pa: number(&AUTO_SMOULD_PA), feed: Feed { on_s: number(&AUTO_SMOULD_ON), off_s: number(&AUTO_SMOULD_OFF) } },
    }
}

/// What the thermostat decides by: the settings of its card as VALUE reads them - one a number does not read from is none, which the thermostat's door refuses by name.
pub fn thermostat_tuning(value: &dyn Fn(&Setting) -> String) -> thermostat::Tuning {
    let number = |s: &Setting| value(s).parse::<f32>().unwrap_or(f32::NAN);
    // THE BURNER BURNS ON THE CHOSEN STEP IN EVERY HOLD: a temperature held decides when the fire works and when it rests, not what a working fire is fed - there is no second pair beside the table
    let chosen = chosen_step(&value(&AUTO_STEP), &number);
    thermostat::Tuning {
        // a mode the card cannot hold never arrives: what a setting keeps is what its check took, and a kept text it does not take reads as the default
        hold: match value(&AUTO_HOLD).as_str() {
            "power" => Hold::Power,
            "return" => Hold::Return,
            _ => Hold::Boiler,
        },
        want_c: number(&AUTO_HOLD_C),
        hysteresis_c: number(&AUTO_DROP_C),
        step: chosen,
        mid: level_or(&value(&AUTO_STEP_MID), chosen, &number),
        small: level_or(&value(&AUTO_STEP_SMALL), chosen, &number),
        mid_c: number(&AUTO_MID_C),
        small_c: number(&AUTO_SMALL_C),
        power: chosen,
        // the card keeps minutes, the thermostat counts seconds
        lockout: thermostat::Lockout { wait_s: number(&AUTO_LOCK_MIN) * 60.0, floor_c: number(&AUTO_LOCK_C), sure_s: number(&AUTO_SURE_S) },
    }
}

/// What the demand says by: who plays it, the targets of the room and its own hysteresis, and the profiles with the windows that name the target - the settings of the demand's card as VALUE reads them. One a number does not read from is none; the ranges of
/// the settings keep the hysteresis under the lowest target, so no order between them is left to a door.
pub fn demand_tuning(value: &dyn Fn(&Setting) -> String) -> demand::Tuning {
    let number = |s: &Setting| value(s).parse::<f32>().unwrap_or(f32::NAN);
    demand::Tuning {
        by: match value(&AUTO_ASKED_BY).as_str() {
            "room" => By::Room,
            "schedule" => By::Schedule,
            _ => By::Nobody,
        },
        room_c: number(&AUTO_ROOM_C),
        room_eco_c: number(&AUTO_ROOM_ECO_C),
        room_drop_c: number(&AUTO_ROOM_GAP_C),
        schedule: schedule_of(value),
    }
}

/// The profiles and the windows of the card as VALUE reads them. A window kept as `off`, or as a text the schedule does not read, is no window: what a setting keeps is what its check took, and a kept text
/// it does not take reads as the default (`settings::value`).
fn schedule_of(value: &dyn Fn(&Setting) -> String) -> Schedule {
    let mut windows: [Option<Window>; WINDOWS] = [None; WINDOWS];
    for (at, setting) in WINDOW_SETTINGS.iter().enumerate() {
        windows[at] = value(setting).strip_prefix("set:").and_then(|text| Window::parse(text).ok());
    }
    Schedule {
        main: match value(&AUTO_PROFILE).as_str() {
            "eco" => Profile::Eco,
            "off" => Profile::Off,
            _ => Profile::Comfort,
        },
        windows,
    }
}

/// The level a row of the table names, or CHOSEN when the setting names none: a middle or a small level nobody chose is the level of power.
fn level_or(kw: &str, chosen: Step, number: &dyn Fn(&Setting) -> f32) -> Step {
    if kw == CHOSEN {
        chosen
    } else {
        chosen_step(kw, number)
    }
}

/// The dose, the pause and the draught of the level KW kilowatts of fuel, out of the table of steps as NUMBER reads it. A level the table has not is none in every number, which the automaton's door refuses by name.
fn chosen_step(kw: &str, number: &dyn Fn(&Setting) -> f32) -> Step {
    match STEP_TABLE.iter().find(|row| row.kw == kw) {
        Some(row) => Step { pa: number(row.pa), feed: Feed { on_s: number(row.on_s), off_s: number(row.off_s) } },
        None => Step { pa: f32::NAN, feed: Feed { on_s: f32::NAN, off_s: f32::NAN } },
    }
}
