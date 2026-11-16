//! The thermostat does not light the fire again sooner than a set time after it went out, on rows of readings: the time counts from the end of burning, `Keep` stands in the place of `Burn` while it runs,
//! a Light, or the held water under the floor, lights at once, and nought is the thermostat as it was.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: the fire going out is the second the thermostat is told it is out and the one before told it was not; the count is of the seconds the readings stand for, from that second; a
//! light is withheld only while the fire is OUT - a smouldering fire is not lit, it works again, and that is not the lockout's; a Light is the person's word and ends the wait for good; a Start is the switch and cuts nothing; the floor is a number the water must not
//! reach on the way, and "under" is under. The time lives in the thermostat, so the house turning warm or cold in the middle of it moves the order and not the clock.
//!
//! Numbers: setpoint 60, hysteresis 4 (the water lights the fire at 56 and under), the lockout two hours, the floor 30.

use automaton::{Feed, Heard, Held, Step, Switch, Water};
use thermostat::{Fire, Hold, Lockout, Press, Temperatures, Thermostat, Tuning};

const WAIT: f32 = 7200.0;

fn step() -> Step {
    Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } }
}

fn tuning(hold: Hold, wait_s: f32, floor_c: f32) -> Tuning {
    Tuning { hold, want_c: 60.0, hysteresis_c: 4.0, step: step(), mid: step(), small: step(), mid_c: 8.0, small_c: 3.0, power: step(), lockout: Lockout { wait_s, floor_c, sure_s: 0.0 } }
}

fn sound(wait_s: f32) -> thermostat::Sound {
    tuning(Hold::Boiler, wait_s, 30.0).sound(88.0).expect("the numbers are sound")
}

/// What the thermostat is given: the boiler at C, the fire out or not, and the seconds this reading stands for.
fn reading(boiler_c: f32, out: bool, elapsed_s: f32) -> Temperatures {
    Temperatures { boiler_c: Some(boiler_c), return_c: Some(30.0), return_heard: Heard::Speaks, asked: None, room_c: None, room_target_c: None, fire: Fire { out, elapsed_s, steady: !out } }
}

/// A thermostat that has watched a fire burn and has just been told it is out, over water at C. Its first look at the fire out is the end of burning.
fn just_out(sound: &thermostat::Sound, c: f32) -> Thermostat {
    let mut t = Thermostat::new();
    t.order(&reading(c, false, 1.0), sound);
    t.order(&reading(c, true, 1.0), sound);
    t
}

fn water(t: &mut Thermostat, sound: &thermostat::Sound, c: f32, out: bool, elapsed_s: f32) -> Water {
    t.order(&reading(c, out, elapsed_s), sound).water
}

/// The seconds of the lockout left, to the second: a count is of whole seconds on these rows.
fn left(t: &Thermostat) -> Option<f32> {
    t.lock_left_s()
}

// ---------------------------------------------------------------- a fire that went out is not lit again before the time

#[test]
fn cooled_water_under_the_line_is_kept_until_the_time_is_up_and_burned_after_it() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    for second in 1..7200 {
        assert_eq!(water(&mut t, &s, 50.0, true, 1.0), Water::Keep, "second {second} of 7200: still waiting");
    }
    assert_eq!(water(&mut t, &s, 50.0, true, 1.0), Water::Burn, "the 7200th second: the wait is over");
    assert_eq!(water(&mut t, &s, 50.0, true, 1.0), Water::Burn, "and stays over");
}

#[test]
fn what_the_water_says_is_kept_whatever_it_is_when_it_does_not_ask_for_a_light() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 58.0);
    assert_eq!(water(&mut t, &s, 58.0, true, 1.0), Water::Keep, "in the band: the lockout has nothing to withhold");
    assert_eq!(water(&mut t, &s, 61.0, true, 1.0), Water::Enough, "over the setpoint: enough, not kept");
}

#[test]
fn nought_is_the_thermostat_as_it_was_and_says_no_time() {
    let s = sound(0.0);
    let mut t = just_out(&s, 50.0);
    assert_eq!(water(&mut t, &s, 50.0, true, 1.0), Water::Burn, "no lockout: the second the water is under the line the order is a light");
    assert_eq!(left(&t), None);
}

// ---------------------------------------------------------------- what ends it early

#[test]
fn a_start_when_nothing_was_waited_for_says_nothing() {
    let s = sound(WAIT);
    let mut t = Thermostat::new();
    t.order(&reading(50.0, true, 1.0), &s);
    assert_eq!(t.press(Press::Start), None, "a board that has not seen a fire end has no wait to cut short");
    let mut after = just_out(&sound(0.0), 50.0);
    assert_eq!(after.press(Press::Start), None, "no lockout is set");
    let mut done = just_out(&s, 50.0);
    for _ in 0..7200 {
        water(&mut done, &s, 50.0, true, 1.0);
    }
    assert_eq!(done.press(Press::Start), None, "the wait is over");
}

#[test]
fn a_stop_does_not_end_the_wait_and_does_not_start_one() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    assert_eq!(t.press(Press::Stop), None);
    assert_eq!(water(&mut t, &s, 50.0, true, 1.0), Water::Keep);
    assert!(left(&t).is_some());
}

#[test]
fn water_under_the_floor_lights_at_once_whatever_is_left_and_water_on_the_floor_does_not() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    assert_eq!(water(&mut t, &s, 30.0, true, 1.0), Water::Keep, "30 is the floor itself: not under it");
    assert_eq!(water(&mut t, &s, 29.9, true, 1.0), Water::Burn, "under the floor: waiting longer would let the boiler go cold");
    assert_eq!(water(&mut t, &s, 40.0, true, 1.0), Water::Keep, "and when the water is over it again the wait goes on");
}

#[test]
fn the_floor_is_a_setting_and_a_lower_one_lets_the_water_cool_further() {
    let s = tuning(Hold::Boiler, WAIT, 20.0).sound(88.0).unwrap();
    let mut t = just_out(&s, 50.0);
    assert_eq!(water(&mut t, &s, 25.0, true, 1.0), Water::Keep, "under 30 and over 20");
    assert_eq!(water(&mut t, &s, 19.0, true, 1.0), Water::Burn);
}

// THE FLOOR IS THE BOILER'S WATER IN EVERY MODE: "the boiler has gone cold, and waiting longer would let it" is about the boiler, whatever the burner holds. In the mode of the return the
// held 25 degrees are under a floor of 30, so a floor read off the held sensor would never let the wait hold.

#[test]
fn in_the_mode_of_the_return_the_floor_is_the_boilers_water_too() {
    let s = tuning(Hold::Return, WAIT, 30.0).sound(88.0).unwrap();
    let at = |boiler_c: f32, out: bool| Temperatures { boiler_c: Some(boiler_c), return_c: Some(25.0), ..reading(boiler_c, out, 1.0) };
    let mut t = Thermostat::new();
    t.order(&at(45.0, false), &s);
    t.order(&at(45.0, true), &s);
    assert_eq!(t.order(&at(45.0, true), &s).water, Water::Keep, "the return at 25 is under the floor and the boiler is not: the wait holds");
    assert_eq!(t.order(&at(29.0, true), &s).water, Water::Burn);
}

#[test]
fn a_boiler_that_says_nothing_does_not_end_the_wait() {
    let s = tuning(Hold::Return, WAIT, 30.0).sound(88.0).unwrap();
    let at = |boiler_c: Option<f32>, out: bool| Temperatures { boiler_c, ..reading(45.0, out, 1.0) };
    let mut t = Thermostat::new();
    t.order(&at(Some(45.0), false), &s);
    t.order(&at(Some(45.0), true), &s);
    assert_eq!(t.order(&at(None, true), &s).water, Water::Keep, "no reading of the boiler is not a cold boiler");
}

// ---------------------------------------------------------------- what is counted from, and what is left

#[test]
fn the_time_left_counts_down_from_the_end_of_burning_by_the_seconds_the_readings_stand_for() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    assert_eq!(left(&t), Some(WAIT), "the second the fire is told out is the start");
    water(&mut t, &s, 50.0, true, 1.0);
    assert_eq!(left(&t), Some(WAIT - 1.0));
    water(&mut t, &s, 50.0, true, 60.0);
    assert_eq!(left(&t), Some(WAIT - 61.0), "a reading that stands for a minute counts a minute");
    water(&mut t, &s, 50.0, true, 0.0);
    assert_eq!(left(&t), Some(WAIT - 61.0), "a second look at the same second counts nothing");
}

#[test]
fn nothing_is_left_while_the_fire_burns_when_the_wait_is_over_and_where_no_fire_was_seen_to_end() {
    let s = sound(WAIT);
    let mut burning = Thermostat::new();
    burning.order(&reading(50.0, false, 1.0), &s);
    assert_eq!(left(&burning), None, "a fire that burns has no wait");
    let mut boot = Thermostat::new();
    boot.order(&reading(50.0, true, 1.0), &s);
    assert_eq!(left(&boot), None, "a board that came up with the fire out has seen no end of burning: nothing is counted from the power");
    assert_eq!(water(&mut boot, &s, 50.0, true, 1.0), Water::Burn, "and a cooled boiler is lit as it always was");
    let mut done = just_out(&s, 50.0);
    for _ in 0..7200 {
        water(&mut done, &s, 50.0, true, 1.0);
    }
    assert_eq!(left(&done), None);
}

#[test]
fn a_fire_that_was_lit_and_has_gone_out_again_starts_the_wait_anew() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    for _ in 0..500 {
        water(&mut t, &s, 50.0, true, 1.0);
    }
    t.press(Press::Start);
    water(&mut t, &s, 50.0, false, 1.0);
    assert_eq!(left(&t), None, "the fire is lit: no wait");
    water(&mut t, &s, 50.0, true, 1.0);
    assert_eq!(left(&t), Some(WAIT), "it went out again: a whole wait from this end of burning");
    assert_eq!(water(&mut t, &s, 50.0, true, 1.0), Water::Keep);
}

#[test]
fn a_fire_that_is_not_out_is_never_withheld_a_smouldering_one_works_again_when_the_water_asks() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    assert_eq!(water(&mut t, &s, 50.0, false, 1.0), Water::Burn, "the fire is not out: this is no light, it is the fire working again");
}

#[test]
fn the_wait_is_a_setting_and_a_longer_one_taken_in_the_middle_of_it_is_the_new_total() {
    let two = sound(WAIT);
    let three = sound(10800.0);
    let mut t = just_out(&two, 50.0);
    for _ in 0..1000 {
        water(&mut t, &two, 50.0, true, 1.0);
    }
    water(&mut t, &three, 50.0, true, 1.0);
    assert_eq!(left(&t), Some(10800.0 - 1001.0), "the seconds since the end of burning stay, the total is the card's now");
}

// ---------------------------------------------------------------- the demand moves the order and not the clock

#[test]
fn the_house_turning_warm_and_cold_in_the_middle_of_the_wait_does_not_restart_it() {
    let s = tuning(Hold::Boiler, WAIT, 30.0).sound(88.0).unwrap();
    let asking = |demand: demand::Demand, elapsed_s: f32| Temperatures { asked: Some(demand::Asked { demand, why: demand::Why::BelowTarget }), ..reading(50.0, true, elapsed_s) };
    let mut t = Thermostat::new();
    t.order(&reading(50.0, false, 1.0), &s);
    t.order(&reading(50.0, true, 1.0), &s);
    assert_eq!(t.order(&asking(demand::Demand::Warm, 3600.0), &s).water, Water::Enough, "warm: nothing to light, and the wait is counting");
    let cold = t.order(&asking(demand::Demand::Cold, 1.0), &s);
    assert_eq!(left(&t), Some(WAIT - 3601.0), "the demand changed the order, not the clock");
    assert_eq!(cold.water, Water::Keep, "50 is inside the band of the water; a light the wait is holding is not asked for");
    let colder = t.order(&Temperatures { boiler_c: Some(40.0), ..asking(demand::Demand::Cold, 1.0) }, &s);
    assert_eq!(colder.water, Water::Keep, "40 is under the line of 56: a light the wait is holding");
}

// ---------------------------------------------------------------- where there is nothing to withhold

#[test]
fn the_mode_of_power_has_no_water_and_no_wait() {
    let s = tuning(Hold::Power, WAIT, 30.0).sound(88.0).unwrap();
    let mut t = Thermostat::new();
    t.order(&reading(50.0, false, 1.0), &s);
    let order = t.order(&reading(50.0, true, 1.0), &s);
    assert_eq!((order.water, left(&t)), (Water::Level, None));
}

#[test]
fn a_held_sensor_that_is_silent_is_said_silent_and_not_kept() {
    let s = tuning(Hold::Return, WAIT, 30.0).sound(88.0).unwrap();
    let mut t = Thermostat::new();
    t.order(&reading(50.0, false, 1.0), &s);
    t.order(&reading(50.0, true, 1.0), &s);
    let silent = Temperatures { return_c: None, return_heard: Heard::TooLong, ..reading(50.0, true, 1.0) };
    assert_eq!(t.order(&silent, &s).water, Water::Silent(Held::Return));
}

#[test]
fn the_switch_of_the_order_is_the_persons_and_the_wait_does_not_touch_it() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    t.press(Press::Stop);
    let order = t.order(&reading(50.0, true, 1.0), &s);
    assert_eq!((order.switch, order.water), (Switch::Off, Water::Keep));
}

// ---------------------------------------------------------------- the door

#[test]
fn a_wait_that_is_no_number_or_under_nought_is_refused_by_its_name_and_a_floor_at_nought_too() {
    for wait in [f32::NAN, f32::INFINITY, -1.0] {
        let refused = tuning(Hold::Boiler, wait, 30.0).sound(88.0).expect_err("a wait that is not a span of time");
        assert_eq!(refused.field, "lock_s", "{wait}");
    }
    for floor in [f32::NAN, 0.0, -5.0] {
        let refused = tuning(Hold::Boiler, WAIT, floor).sound(88.0).expect_err("a floor that is no temperature");
        assert_eq!(refused.field, "lock_floor_c", "{floor}");
    }
    assert!(tuning(Hold::Boiler, 0.0, 30.0).sound(88.0).is_ok(), "nought is no lockout, and sound");
}

// ---------------------------------------------------------------- Start switches on and the wait goes on; a Light ends it

#[test]
fn a_start_does_not_end_the_wait_and_the_light_stays_withheld() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    for _ in 0..100 {
        water(&mut t, &s, 50.0, true, 1.0);
    }
    assert_eq!(t.press(Press::Start), None, "a Start is the switch: it cuts no wait, so it has nothing left to say");
    assert_eq!(water(&mut t, &s, 50.0, true, 0.0), Water::Keep, "the order of the very moment of the press is still the wait's");
    assert!(left(&t).is_some(), "and the wait is still running");
    assert_eq!(water(&mut t, &s, 50.0, true, 1.0), Water::Keep);
}

#[test]
fn a_light_before_the_time_is_up_ends_the_wait_and_says_how_much_was_left() {
    let s = sound(WAIT);
    let mut t = just_out(&s, 50.0);
    for _ in 0..100 {
        water(&mut t, &s, 50.0, true, 1.0);
    }
    assert_eq!(water(&mut t, &s, 50.0, true, 1.0), Water::Keep);
    let cut = t.press(Press::Light).expect("a light in the middle of the wait ends it, and says what was left");
    assert!((cut - (WAIT - 101.0)).abs() < 1.5, "left {cut}");
    assert_eq!(water(&mut t, &s, 50.0, true, 0.0), Water::Burn, "the order of the very moment of the press");
    assert_eq!(left(&t), None);
}

#[test]
fn a_light_when_nothing_was_waited_for_says_nothing() {
    let s = sound(WAIT);
    let mut t = Thermostat::new();
    t.order(&reading(50.0, true, 1.0), &s);
    assert_eq!(t.press(Press::Light), None, "a board that has not seen a fire end has no wait to cut short");
    let mut after = just_out(&sound(0.0), 50.0);
    assert_eq!(after.press(Press::Light), None, "no lockout is set");
}
