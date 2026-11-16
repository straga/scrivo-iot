//! Before the sensors have been asked all round once since the board came up, the thermostat orders no light from a stop, whoever plays the demand and whatever the water reads - on the path the board takes: the demand's player, the
//! word it says, the thermostat that orders on it.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: 12.10 14:16 a restart lit the burner on water read at 25.9 °C before the room had been asked; a restart lights nothing until the first round of the sensors has gone through, by
//! the schedule, by the room and by the water alone alike; a person's Start is not a restart and lights at once; after the round the order is the order of every other second.

use automaton::{Feed, Step, Switch, Water};
use demand::{Asked, By, Demand, Player, Tuning as Asks, Why};
use schedule::{Schedule, Wall, Window};
use thermostat::{Fire, Hold, Lockout, Press, Temperatures, Thermostat, Tuning};

const WANT: f32 = 60.0;
const DROP: f32 = 4.0;
const FLOOR_C: f32 = 30.0;
/// The water of the incident: 25.9 °C, under the line (56) and under the floor (30).
const COLD_WATER_C: f32 = 25.9;

fn step() -> Step {
    Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } }
}

fn sound(hold: Hold) -> thermostat::Sound {
    let tuning = Tuning { hold, want_c: WANT, hysteresis_c: DROP, step: step(), mid: step(), small: step(), mid_c: 8.0, small_c: 3.0, power: step(), lockout: Lockout { wait_s: 0.0, floor_c: FLOOR_C, sure_s: 0.0 } };
    tuning.sound(88.0).expect("the numbers are sound")
}

fn window(text: &str) -> Option<Window> {
    Some(Window::parse(text).expect("the row's window is written right"))
}

/// What the player of BY says on the first second of a board that has come up: the room unread, the clock at Monday 23:00 with a window open that burns.
fn asked_by(by: By) -> Option<Asked> {
    let schedule = Schedule { windows: [window("daily 22:00-06:00 eco"), None, None, None, None, None], ..Schedule::default() };
    let tuning = Asks { by, room_c: 21.0, room_eco_c: 17.0, room_drop_c: 0.5, schedule };
    Player::after_boot().ask(&tuning, None, Some(Wall { day: 0, minute: 23 * 60 })).asked
}

fn readings(boiler_c: f32, asked: Option<Asked>) -> Temperatures {
    Temperatures { boiler_c: Some(boiler_c), return_c: Some(boiler_c), return_heard: automaton::Heard::Speaks, asked, room_c: None, room_target_c: None, fire: Fire { out: true, elapsed_s: 1.0, steady: false } }
}

/// A thermostat of a board that came up with the switch on in its memory: the order of its first second at BOILER_C, before and after the round.
fn orders(hold: Hold, asked: Option<Asked>, boiler_c: f32) -> (Water, Water) {
    let t = sound(hold);
    let mut thermostat = Thermostat::restored(Switch::On).waiting_for_first_round();
    let before = thermostat.order(&readings(boiler_c, asked), &t).water;
    thermostat.first_round_done();
    let after = thermostat.order(&readings(boiler_c, asked), &t).water;
    (before, after)
}

#[test]
fn the_schedule_with_a_window_open_lights_nothing_on_cold_water_before_the_first_round_and_lights_after_it() {
    let asked = asked_by(By::Schedule);
    assert_eq!(asked, Some(Asked { demand: Demand::Cold, why: Why::ScheduledOn }), "the scaffolding: the window burns and the house is cold");
    let (before, after) = orders(Hold::Boiler, asked, COLD_WATER_C);
    assert_ne!(before, Water::Burn, "the schedule said cold on the first second of the board and the water was under the line: a restart lit the burner");
    assert_eq!(after, Water::Burn, "after the round the order is the order of every other second");
}

#[test]
fn nobody_playing_the_demand_lights_nothing_on_cold_water_before_the_first_round_and_lights_after_it() {
    let asked = asked_by(By::Nobody);
    assert_eq!(asked, None, "the scaffolding: nobody plays the role");
    let (before, after) = orders(Hold::Boiler, asked, 40.0);
    assert_ne!(before, Water::Burn, "the water alone said burn on the first second of the board");
    assert_eq!(after, Water::Burn);
}

#[test]
fn the_room_unread_lights_nothing_before_the_first_round_and_by_the_floor_only_after_it() {
    let before = asked_by(By::Room);
    assert_eq!(before, Some(Asked { demand: Demand::Warm, why: Why::Starting }), "the scaffolding: the room is unread and the round has not gone through");
    let (waiting, _) = orders(Hold::Boiler, before, COLD_WATER_C);
    assert_ne!(waiting, Water::Burn);
    // the same board after the round with the room silent: a dead sensor, and the floor lights
    let (_, after) = orders(Hold::Boiler, Some(Asked { demand: Demand::Warm, why: Why::NotRead }), COLD_WATER_C);
    assert_eq!(after, Water::Burn, "the floor of the boiler's water lights a stop whose room did not answer");
}

#[test]
fn the_mode_of_power_with_the_house_cold_lights_nothing_before_the_first_round() {
    let asked = asked_by(By::Schedule);
    let (before, after) = orders(Hold::Power, asked, 40.0);
    assert_ne!(before, Water::Burn, "power asks to burn on a cold house, and the round has not gone through");
    assert_eq!(after, Water::Burn);
}

#[test]
fn a_person_s_start_before_the_first_round_is_counted_as_always() {
    // the pair: the barrier is on the water, not on a person - the press is in the order, and the fire lights on it whatever the water says
    let t = sound(Hold::Boiler);
    let mut thermostat = Thermostat::new().waiting_for_first_round();
    thermostat.press(Press::Start);
    let order = thermostat.order(&readings(COLD_WATER_C, asked_by(By::Schedule)), &t);
    assert_eq!((order.switch, order.presses.start), (Switch::On, 1), "the Start is counted");
}

#[test]
fn a_fire_that_is_burning_is_not_sent_down_by_the_barrier() {
    // the pair: a water that says enough stays enough before the round; the barrier takes a light, not a fire
    let (before, after) = orders(Hold::Boiler, None, 75.0);
    assert_eq!((before, after), (Water::Enough, Water::Enough));
}

#[test]
fn a_fire_that_burns_is_given_the_word_of_the_water_before_the_first_round_whoever_plays_the_demand() {
    // the barrier is on a light from a stop and on nothing else: a fire that is lit or smouldering takes the word of the water as it is - a smouldering one goes back to work on water that burns
    let t = sound(Hold::Boiler);
    for (who, asked) in [("the schedule", asked_by(By::Schedule)), ("nobody", asked_by(By::Nobody)), ("the room, unread", asked_by(By::Room))] {
        let mut thermostat = Thermostat::restored(Switch::On).waiting_for_first_round();
        let burning = Temperatures { fire: Fire { out: false, steady: true, elapsed_s: 1.0 }, ..readings(40.0, asked) };
        assert_eq!(thermostat.order(&burning, &t).water, Water::Burn, "{who}: the fire burns, the water says burn, and the round has not gone through");
    }
}

#[test]
fn a_person_s_light_before_the_first_round_is_counted_as_always() {
    // the pair of the Start above: the barrier is on the water, not on a person - a Light is in the order whatever the water says
    let t = sound(Hold::Boiler);
    let mut thermostat = Thermostat::new().waiting_for_first_round();
    thermostat.press(Press::Light);
    let order = thermostat.order(&readings(COLD_WATER_C, asked_by(By::Schedule)), &t);
    assert_eq!(order.presses.light, 1, "the Light is counted");
}
