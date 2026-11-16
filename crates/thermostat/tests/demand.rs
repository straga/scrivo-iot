//! What the thermostat orders when the house asks for heat, on rows of readings and the word of the demand, and no board anywhere.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: warm is the ordinary way to a stop whatever the water says; cold is the water's own band, and in the mode of power - where the
//! water never says burn - it is burn; the lockout of a light stands after the demand and withholds its lights too; and with nobody playing the role the water decides as it did.

use automaton::{Feed, Heard, Held, Step, Water};
use demand::{Asked, Demand, Why};
use thermostat::{Fire, Hold, Lockout, Temperatures, Thermostat, Tuning};

const WANT: f32 = 60.0;
const DROP: f32 = 4.0;
const WAIT_S: f32 = 600.0;
const FLOOR_C: f32 = 30.0;

fn step() -> Step {
    Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } }
}

fn power() -> Step {
    Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } }
}

fn sound(hold: Hold, wait_s: f32) -> thermostat::Sound {
    let tuning = Tuning { hold, want_c: WANT, hysteresis_c: DROP, step: step(), mid: step(), small: step(), mid_c: 8.0, small_c: 3.0, power: power(), lockout: Lockout { wait_s, floor_c: FLOOR_C, sure_s: 0.0 } };
    tuning.sound(88.0).expect("the numbers are sound")
}

fn cold() -> Option<Asked> {
    Some(Asked { demand: Demand::Cold, why: Why::BelowTarget })
}

fn warm() -> Option<Asked> {
    Some(Asked { demand: Demand::Warm, why: Why::TargetReached })
}

/// Every sensor speaking: the boiler and the return at C, the room far from anything a row asks.
fn at(c: f32, asked: Option<Asked>) -> Temperatures {
    Temperatures { boiler_c: Some(c), return_c: Some(c), return_heard: Heard::Speaks, asked, room_c: None, room_target_c: None, fire: Fire::default() }
}

fn water(hold: Hold, readings: Temperatures) -> Water {
    Thermostat::new().order(&readings, &sound(hold, 0.0)).water
}

// ---------------------------------------------------------------- row 1: warm is the ordinary way to a stop

#[test]
fn warm_says_enough_whatever_the_water_says() {
    for hold in [Hold::Boiler, Hold::Return, Hold::Power] {
        for c in [30.0, WANT - DROP, 58.0, WANT, 75.0] {
            assert_eq!(water(hold, at(c, warm())), Water::Enough, "{hold:?} at {c}: a warm house wants no burning, the water cold or not");
        }
    }
}

#[test]
fn a_silent_held_sensor_is_still_said_silent_under_warm() {
    let readings = Temperatures { return_c: None, return_heard: Heard::TooLong, ..at(40.0, warm()) };
    assert_eq!(water(Hold::Return, readings), Water::Silent(Held::Return), "the protection is not a request, and a warm house does not hide it");
}

// ---------------------------------------------------------------- row 2: cold is the water's own band

#[test]
fn cold_is_the_water_by_its_own_band_in_a_mode_by_a_sensor() {
    for hold in [Hold::Boiler, Hold::Return] {
        assert_eq!(water(hold, at(WANT - DROP, cold())), Water::Burn, "{hold:?}");
        assert_eq!(water(hold, at(58.0, cold())), Water::Keep, "{hold:?}");
        assert_eq!(water(hold, at(WANT, cold())), Water::Enough, "{hold:?}: cold water that is at the setpoint is no reason to burn");
    }
}

#[test]
fn cold_in_the_mode_of_power_asks_to_burn_whatever_the_water_says() {
    for c in [30.0, 58.0, 75.0] {
        assert_eq!(water(Hold::Power, at(c, cold())), Water::Burn, "power at {c}: the water never says burn there, and a stop does not light itself");
    }
}

#[test]
fn a_source_that_is_silent_says_cold_and_is_told_so() {
    let silent = Some(Asked { demand: Demand::Cold, why: Why::SourceSilent });
    let mut thermostat = Thermostat::new();
    let order = thermostat.order(&at(30.0, silent), &sound(Hold::Power, 0.0));
    assert_eq!(order.water, Water::Burn);
    assert_eq!(thermostat.asked(), silent, "the card says who asked and why, as it was told");
}

#[test]
fn the_word_of_the_last_order_is_kept_for_the_card() {
    let mut thermostat = Thermostat::new();
    assert_eq!(thermostat.asked(), None, "before the first order nothing was asked");
    thermostat.order(&at(50.0, warm()), &sound(Hold::Boiler, 0.0));
    assert_eq!(thermostat.asked(), warm());
    thermostat.order(&at(50.0, None), &sound(Hold::Boiler, 0.0));
    assert_eq!(thermostat.asked(), None, "nobody playing the role is not a warm house");
}

// ---------------------------------------------------------------- row 4: the lockout stands after the demand

/// The fire burned and goes out, the wait runs for SECONDS: the last order. The boiler is at C, a number over the floor.
fn after_burning(hold: Hold, asked: Option<Asked>, c: f32, seconds: f32) -> (Thermostat, Water) {
    let sound = sound(hold, WAIT_S);
    let mut thermostat = Thermostat::new();
    thermostat.order(&Temperatures { fire: Fire { out: false, steady: true, elapsed_s: 1.0 }, ..at(c, asked) }, &sound);
    thermostat.order(&Temperatures { fire: Fire { out: true, steady: false, elapsed_s: 1.0 }, ..at(c, asked) }, &sound);
    let order = thermostat.order(&Temperatures { fire: Fire { out: true, steady: false, elapsed_s: seconds }, ..at(c, asked) }, &sound);
    (thermostat, order.water)
}

#[test]
fn a_light_the_demand_asks_for_waits_out_the_lockout() {
    for hold in [Hold::Boiler, Hold::Power] {
        let (thermostat, water) = after_burning(hold, cold(), 45.0, 100.0);
        assert_eq!(water, Water::Keep, "{hold:?}: cold, and the fire went out 100 s of 600 ago");
        assert!(thermostat.lock_left_s().is_some_and(|left| left > 400.0), "{hold:?}: the page counts the wait down, and it is {:?}", thermostat.lock_left_s());
    }
}

#[test]
fn the_demand_lights_when_the_lockout_is_over() {
    for hold in [Hold::Boiler, Hold::Power] {
        let (thermostat, water) = after_burning(hold, cold(), 45.0, WAIT_S + 1.0);
        assert_eq!(water, Water::Burn, "{hold:?}");
        assert_eq!(thermostat.lock_left_s(), None, "{hold:?}");
    }
}

#[test]
fn a_boiler_that_has_gone_cold_lights_at_once_under_the_demand_in_every_mode() {
    for hold in [Hold::Boiler, Hold::Power] {
        let (_, water) = after_burning(hold, cold(), FLOOR_C - 5.0, 100.0);
        assert_eq!(water, Water::Burn, "{hold:?}: under the floor waiting longer lets the boiler go cold");
    }
}

#[test]
fn warm_is_not_a_light_and_the_lockout_has_nothing_to_withhold() {
    let (thermostat, water) = after_burning(Hold::Boiler, warm(), 45.0, 100.0);
    assert_eq!(water, Water::Enough);
    assert_eq!(thermostat.lock_left_s(), Some(WAIT_S - 100.0), "the wait runs on its own clock whatever the house says");
}

// ---------------------------------------------------------------- row 7: with nobody playing the role the water decides as it did

#[test]
fn with_nobody_asking_the_water_decides_as_before() {
    for (c, expected) in [(30.0, Water::Burn), (58.0, Water::Keep), (75.0, Water::Enough)] {
        assert_eq!(water(Hold::Boiler, at(c, None)), expected, "boiler at {c}");
    }
    for c in [30.0, 58.0, 75.0] {
        assert_eq!(water(Hold::Power, at(c, None)), Water::Level, "power at {c}");
    }
    let (thermostat, water) = after_burning(Hold::Power, None, 45.0, 100.0);
    assert_eq!((water, thermostat.lock_left_s()), (Water::Level, None), "the mode of power has no water and no wait while nobody asks");
}

// ---------------------------------------------------------------- row 8: a room not read once since the board came up leaves the water to the floor alone

fn not_read() -> Option<Asked> {
    Some(Asked { demand: Demand::Warm, why: Why::NotRead })
}

#[test]
fn a_room_not_read_yet_lights_a_boiler_under_the_floor_in_every_mode() {
    for hold in [Hold::Boiler, Hold::Return, Hold::Power] {
        assert_eq!(water(hold, at(FLOOR_C - 1.0, not_read())), Water::Burn, "{hold:?}: the house has said nothing, and a boiler gone under its floor is not left to go colder");
    }
}

#[test]
fn a_room_not_read_yet_leaves_a_boiler_over_the_floor_standing_whatever_the_setpoint() {
    // the pair: the floor itself is no light ("under" is under), and neither is the water under the setpoint less the hysteresis (56) - the water of a restart is not a reason
    for hold in [Hold::Boiler, Hold::Return, Hold::Power] {
        for c in [FLOOR_C, 40.0, WANT - DROP, 58.0, 75.0] {
            assert_eq!(water(hold, at(c, not_read())), Water::Enough, "{hold:?} at {c}");
        }
    }
}

#[test]
fn a_room_not_read_yet_and_a_boiler_that_says_nothing_light_nothing() {
    let readings = Temperatures { boiler_c: None, ..at(0.0, not_read()) };
    assert_eq!(water(Hold::Boiler, readings), Water::Enough, "no number under the floor is no number under the floor");
}

#[test]
fn a_house_warm_for_a_reason_is_not_a_light_under_the_floor() {
    // the pair of the pair: a room that was read and is warm is the house's word, and the floor does not overrule it
    assert_eq!(water(Hold::Boiler, at(FLOOR_C - 1.0, warm())), Water::Enough);
}

#[test]
fn the_order_says_whether_the_house_is_cold_whatever_the_water_says() {
    let said = |asked| Thermostat::new().order(&at(58.0, asked), &sound(Hold::Boiler, 0.0)).house_cold;
    assert!(said(cold()), "58 is inside the water's hysteresis: the word of the water is keep, and the house is cold");
    assert!(!said(warm()));
    assert!(!said(not_read()), "a room not read says neither");
    assert!(!said(None), "nobody plays the role");
}

// ---------------------------------------------------------------- row 9: before the first round of the sensors nothing lights, the floor included

fn starting() -> Option<Asked> {
    Some(Asked { demand: Demand::Warm, why: Why::Starting })
}

#[test]
fn the_word_starting_asks_nothing_of_the_water_in_any_mode() {
    // the room's word before the first round is as if nobody played the role: the water is what it says - and that no stop is lit in that time is the barrier of the thermostat over every source (tests/first_round.rs), not this word
    for hold in [Hold::Boiler, Hold::Return, Hold::Power] {
        for c in [5.0, FLOOR_C - 1.0, 40.0, 58.0, 75.0] {
            assert_eq!(water(hold, at(c, starting())), water(hold, at(c, None)), "{hold:?} at {c}");
        }
    }
}

#[test]
fn a_silent_held_sensor_is_said_silent_before_the_first_round_too() {
    let readings = Temperatures { return_c: None, return_heard: Heard::TooLong, ..at(40.0, starting()) };
    assert_eq!(water(Hold::Return, readings), Water::Silent(Held::Return), "the protection is not a request, and it does not wait for a round");
}
