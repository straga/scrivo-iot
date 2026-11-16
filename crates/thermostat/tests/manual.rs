//! A person's Light carries the fire to the nominal of whatever holds the burner, and not just lights it: the target of the room where the room is the source of the demand, the setpoint of the held water everywhere else. Neither the band of the room, nor
//! the band of the water, nor the wait after a fire that went out holds a manual cycle; a protection does (that is the fire's and never this crate's). The cycle ends at the nominal, when the fire that burned has gone out, and on a person's Stop. A Light
//! when the nominal is reached already lights nothing and is counted for nothing.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: 29.10 15:31 the room stood at 21.1 °C under a target of 22 and a band of 2; the person pressed Light, the fire lit, and one second into work it went to smoulder because the room, coming down from above,
//! was still `warm` inside its band. "Light = one full cycle to the nominal of whoever holds the burner" is the person's word of 29.10 and 30.10.
//!
//! Numbers: room target 22, band 2, setpoint 60, hysteresis 4 (the water burns at 56 and under), floor 30.

use automaton::{Feed, Heard, Step, Switch, Water};
use demand::{Asked, By, Player, Tuning as Asks};
use schedule::Schedule;
use thermostat::{Ended, Fire, Hold, Lockout, Nominal, Press, Refusal, Temperatures, Thermostat, Tuning};

const TARGET: f32 = 22.0;
const GAP: f32 = 2.0;
const WANT: f32 = 60.0;

fn step() -> Step {
    Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } }
}

fn sound_with(hold: Hold, wait_s: f32) -> thermostat::Sound {
    Tuning { hold, want_c: WANT, hysteresis_c: 4.0, step: step(), mid: step(), small: step(), mid_c: 8.0, small_c: 3.0, power: step(), lockout: Lockout { wait_s, floor_c: 30.0, sure_s: 0.0 } }
        .sound(88.0)
        .expect("the numbers are sound")
}

fn sound() -> thermostat::Sound {
    sound_with(Hold::Boiler, 0.0)
}

/// The house as the room's player says it: the room at ROOM_C after it stood at FROM_C - the memory of the band is what the two readings make.
fn house(from_c: f32, room_c: f32) -> Option<Asked> {
    let tuning = Asks { by: By::Room, room_c: TARGET, room_eco_c: 17.0, room_drop_c: GAP, schedule: Schedule::default() };
    let mut player = Player::new();
    player.ask(&tuning, Some(from_c), None);
    player.ask(&tuning, Some(room_c), None).asked
}

/// What the thermostat is given in the room's source: the boiler at BOILER_C, the room at ROOM_C and the house's word, the fire out or burning.
fn at_room(boiler_c: f32, room_c: f32, asked: Option<Asked>, out: bool) -> Temperatures {
    Temperatures { boiler_c: Some(boiler_c), return_c: Some(boiler_c), return_heard: Heard::Speaks, asked, room_c: Some(room_c), room_target_c: Some(TARGET), fire: Fire { out, steady: !out, elapsed_s: 1.0 } }
}

/// What the thermostat is given by the water alone: no room, nobody asking.
fn at_water(boiler_c: f32, out: bool) -> Temperatures {
    Temperatures { boiler_c: Some(boiler_c), return_c: Some(boiler_c), return_heard: Heard::Speaks, asked: None, room_c: None, room_target_c: None, fire: Fire { out, steady: !out, elapsed_s: 1.0 } }
}

// ---------------------------------------------------------------- (a) the room, inside its band, coming down from above

#[test]
fn a_light_with_the_room_under_its_target_inside_the_band_burns_until_the_room_reaches_the_target_and_then_the_house_decides_again() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    let warm = house(23.0, 21.1);
    assert_eq!(warm.map(|a| a.demand), Some(demand::Demand::Warm), "the scaffolding: 29.10, the room 21.1 inside the band, coming down, is warm");
    // without a Light the house's word stands: enough
    assert_eq!(thermostat.order(&at_room(40.0, 21.1, warm, true), &t).water, Water::Enough);
    thermostat.light(&at_room(40.0, 21.1, warm, true), &t).expect("21.1 is under 22");
    assert_eq!(thermostat.cycle().map(|c| (c.nominal, c.to_c)), Some((Nominal::Room, TARGET)), "the cycle is the room's, to its target");
    // the fire lit, the room climbs: burn all the way, whatever the band says
    for room_c in [21.1, 21.4, 21.7, 21.9, 21.99] {
        assert_eq!(thermostat.order(&at_room(45.0, room_c, house(23.0, room_c), false), &t).water, Water::Burn, "the room at {room_c} is under its target");
    }
    // at the target the cycle ends and the order is the house's again
    let order = thermostat.order(&at_room(45.0, 22.0, house(23.0, 22.0), false), &t);
    assert_eq!(order.water, Water::Enough, "at the target the house says warm and the fire goes the ordinary way down");
    assert_eq!(thermostat.ended(), Some(Ended::Reached));
    assert_eq!(thermostat.cycle(), None);
    assert_eq!(thermostat.ended(), None, "an end is read once");
}

#[test]
fn after_a_cycle_the_next_fall_of_the_room_inside_the_band_is_warm_as_the_band_says() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_room(40.0, 21.1, house(23.0, 21.1), true), &t).unwrap();
    thermostat.order(&at_room(45.0, 22.1, house(21.1, 22.1), false), &t);
    // the room cools from 22.1 to 21.1: the band keeps its word, warm, and so does the order
    assert_eq!(thermostat.order(&at_room(45.0, 21.1, house(22.1, 21.1), false), &t).water, Water::Enough);
}

// ---------------------------------------------------------------- (b) the water, inside its band

#[test]
fn a_light_with_the_water_inside_its_band_burns_until_the_water_reaches_the_setpoint() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    assert_eq!(thermostat.order(&at_water(57.0, true), &t).water, Water::Keep, "the water in its band, the fire out: nothing to light");
    thermostat.light(&at_water(57.0, true), &t).expect("57 is under 60");
    assert_eq!(thermostat.cycle().map(|c| (c.nominal, c.to_c)), Some((Nominal::Water, WANT)));
    for c in [57.0, 58.5, 59.9] {
        assert_eq!(thermostat.order(&at_water(c, false), &t).water, Water::Burn, "the water at {c}: under the setpoint, inside the band");
    }
    assert_eq!(thermostat.order(&at_water(60.0, false), &t).water, Water::Enough, "at the setpoint the water says enough");
    assert_eq!(thermostat.ended(), Some(Ended::Reached));
}

#[test]
fn a_light_with_the_schedule_out_of_its_window_burns_to_the_setpoint_of_the_water() {
    // the schedule's profile that holds no room: warm, no target, and the cycle is the water's
    let off = Some(Asked { demand: demand::Demand::Warm, why: demand::Why::ProfileOff });
    let t = sound();
    let mut thermostat = Thermostat::new();
    let readings = |c: f32, out: bool| Temperatures { asked: off, ..at_water(c, out) };
    thermostat.light(&readings(50.0, true), &t).expect("50 is under 60");
    assert_eq!(thermostat.cycle().map(|c| c.nominal), Some(Nominal::Water));
    assert_eq!(thermostat.order(&readings(55.0, false), &t).water, Water::Burn);
    assert_eq!(thermostat.order(&readings(60.0, false), &t).water, Water::Enough);
}

// ---------------------------------------------------------------- (c) the wait after a fire that went out

#[test]
fn a_light_in_the_wait_after_a_fire_lights_and_the_cycle_is_not_held_by_it() {
    let t = sound_with(Hold::Boiler, 7200.0);
    let mut thermostat = Thermostat::new();
    thermostat.order(&at_water(50.0, false), &t);
    thermostat.order(&at_water(50.0, true), &t);
    assert_eq!(thermostat.order(&at_water(50.0, true), &t).water, Water::Keep, "the scaffolding: the wait holds a light the water asks for");
    assert!(thermostat.lock_left_s().is_some());
    thermostat.light(&at_water(50.0, true), &t).unwrap();
    // the wait is cut by the press (it always was); what the cycle adds is that the band does not hold it: the water rises into its band and the fire goes on
    for c in [50.0, 55.0, 57.0, 58.0] {
        assert_eq!(thermostat.order(&at_water(c, false), &t).water, Water::Burn, "the water at {c}");
    }
    assert_eq!(thermostat.lock_left_s(), None, "while the cycle goes the wait is not shown");
}

#[test]
fn a_light_from_a_stop_before_the_first_round_of_the_sensors_lights_a_person_is_not_a_restart() {
    let t = sound();
    let mut thermostat = Thermostat::new().waiting_for_first_round();
    assert_ne!(thermostat.order(&at_water(50.0, true), &t).water, Water::Burn, "the scaffolding: the barrier holds the water's own light");
    thermostat.light(&at_water(50.0, true), &t).unwrap();
    assert_eq!(thermostat.order(&at_water(50.0, true), &t).water, Water::Burn);
}

// ---------------------------------------------------------------- (d) the nominal is reached already: the pair

#[test]
fn a_light_with_the_room_at_or_over_its_target_lights_nothing_and_counts_for_nothing() {
    let t = sound();
    for room_c in [22.0, 22.3, 24.0] {
        let mut thermostat = Thermostat::new();
        let readings = at_room(40.0, room_c, house(23.0, room_c), true);
        let counts = thermostat.order(&readings, &t).presses;
        let refused = thermostat.light(&readings, &t);
        assert_eq!(refused, Err(Refusal { nominal: Nominal::Room, now_c: room_c, to_c: TARGET }), "the room at {room_c}");
        assert_eq!(thermostat.cycle(), None);
        assert_eq!(thermostat.order(&readings, &t).presses, counts, "a refused Light was counted: the fire would take it");
    }
}

#[test]
fn a_light_with_the_water_at_or_over_the_setpoint_lights_nothing_and_counts_for_nothing() {
    let t = sound();
    for c in [60.0, 61.0, 75.0] {
        let mut thermostat = Thermostat::new();
        let counts = thermostat.order(&at_water(c, true), &t).presses;
        assert_eq!(thermostat.light(&at_water(c, true), &t), Err(Refusal { nominal: Nominal::Water, now_c: c, to_c: WANT }), "the water at {c}");
        assert_eq!(thermostat.order(&at_water(c, true), &t).presses, counts);
    }
}

#[test]
fn a_light_in_the_mode_of_power_is_what_it_always_was_it_lights_and_has_no_cycle_to_end_and_no_refusal() {
    let t = sound_with(Hold::Power, 0.0);
    let mut thermostat = Thermostat::new();
    let before = thermostat.order(&at_water(90.0, true), &t).presses;
    thermostat.light(&at_water(90.0, true), &t).expect("no setpoint stands, so nothing is reached");
    assert_eq!(thermostat.cycle(), None, "the mode of power holds no temperature: no cycle");
    assert_eq!(thermostat.order(&at_water(90.0, true), &t).presses.light, before.light + 1, "the press is counted, as before");
}

// ---------------------------------------------------------------- the end of a cycle

#[test]
fn a_stop_ends_the_cycle_and_the_order_is_the_ordinary_one() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(57.0, true), &t).unwrap();
    assert_eq!(thermostat.order(&at_water(57.0, false), &t).water, Water::Burn);
    thermostat.press(Press::Stop);
    assert_eq!(thermostat.ended(), Some(Ended::Stopped));
    assert_eq!(thermostat.order(&at_water(57.0, false), &t).water, Water::Keep, "the water in its band, the fire burning: the band's own word");
    assert_eq!(thermostat.cycle(), None);
}

#[test]
fn a_fire_that_burned_and_went_out_ends_the_cycle_and_it_is_not_lit_again_by_it() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(57.0, true), &t).unwrap();
    // the second of the press: the automaton still stands in Stop (it takes the press the same breath) - that is not an end
    assert_eq!(thermostat.order(&at_water(57.0, true), &t).water, Water::Burn);
    assert_eq!(thermostat.ended(), None);
    thermostat.order(&at_water(57.0, false), &t);
    // a protection puts the fire out: the cycle ends with the fire
    assert_ne!(thermostat.order(&at_water(57.0, true), &t).water, Water::Burn, "the fire went out; the cycle does not light it again");
    assert_eq!(thermostat.ended(), Some(Ended::Out));
}

#[test]
fn a_fault_ends_the_cycle_it_waits_for_a_person_and_a_cycle_over_it_would_say_what_is_not_so() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(57.0, true), &t).unwrap();
    thermostat.order(&at_water(57.0, false), &t);
    thermostat.faulted(true);
    assert_ne!(thermostat.order(&at_water(57.0, false), &t).water, Water::Burn);
    assert_eq!(thermostat.ended(), Some(Ended::Out));
    assert_eq!(thermostat.cycle(), None);
}

#[test]
fn a_silent_held_sensor_stays_silent_under_a_cycle_it_is_a_protection_and_not_a_request() {
    let t = sound_with(Hold::Return, 0.0);
    let mut thermostat = Thermostat::new();
    let readings = |out: bool| Temperatures { return_c: None, return_heard: Heard::TooLong, ..at_water(40.0, out) };
    thermostat.light(&at_water(40.0, true), &t).unwrap();
    assert!(matches!(thermostat.order(&readings(false), &t).water, Water::Silent(_)), "a cycle does not speak for a sensor that is gone");
}

#[test]
fn a_cycle_of_the_room_does_not_ask_for_a_fire_over_the_setpoint_of_the_water_the_ceiling_stays() {
    // the person's word: "as the ordinary mode" - and in the ordinary mode the water at its setpoint is enough, whatever the room says
    let t = sound();
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_room(55.0, 21.0, house(23.0, 21.0), true), &t).unwrap();
    for c in [55.0, 58.0, 59.9] {
        assert_eq!(thermostat.order(&at_room(c, 21.0, house(23.0, 21.0), false), &t).water, Water::Burn, "the water at {c} is under its setpoint of 60");
    }
    let order = thermostat.order(&at_room(61.0, 21.0, house(23.0, 21.0), false), &t);
    assert_ne!(order.water, Water::Burn, "the room at 21 and the water at 61 against a setpoint of 60: the cycle asked for a fire over the ceiling");
    assert_eq!(order.water, Water::Enough, "the ordinary word at the setpoint");
    assert_eq!(thermostat.ended(), Some(Ended::Water));
    assert_eq!(thermostat.cycle(), None, "the cycle ended; it does not sit on over a burner it does not ask");
    // and the water falling back into its band does not bring the cycle back
    assert_ne!(thermostat.order(&at_room(58.0, 21.0, house(23.0, 21.0), false), &t).water, Water::Burn);
}

#[test]
fn a_cycle_of_the_room_ends_when_the_room_is_no_longer_read() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_room(45.0, 21.0, house(23.0, 21.0), true), &t).unwrap();
    assert_eq!(thermostat.order(&at_room(45.0, 21.0, house(23.0, 21.0), false), &t).water, Water::Burn);
    // the house still says warm (the memory of its band): without the cycle that is `Enough`, and it is what the order goes back to
    let unread = Temperatures { room_c: None, ..at_room(45.0, 21.0, house(23.0, 21.0), false) };
    assert_eq!(thermostat.order(&unread, &t).water, Water::Enough, "a cycle that cannot see where it is going went on");
    assert_eq!(thermostat.ended(), Some(Ended::Unread));
    assert_eq!(thermostat.cycle(), None);
}

#[test]
fn a_cycle_of_the_water_ends_when_the_held_water_is_no_longer_read() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(50.0, true), &t).unwrap();
    let blind = Temperatures { boiler_c: None, ..at_water(50.0, false) };
    assert_ne!(thermostat.order(&blind, &t).water, Water::Burn);
    assert_eq!(thermostat.ended(), Some(Ended::Unread));
}

#[test]
fn a_light_over_a_cold_room_and_water_at_its_setpoint_is_refused_by_the_water_there_is_nothing_to_light_for() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    let readings = at_room(61.0, 21.0, house(23.0, 21.0), true);
    let counts = thermostat.order(&readings, &t).presses;
    assert_eq!(thermostat.light(&readings, &t), Err(Refusal { nominal: Nominal::Water, now_c: 61.0, to_c: WANT }), "the room is cold, the boiler is hot: the ordinary mode would light nothing");
    assert_eq!(thermostat.cycle(), None);
    assert_eq!(thermostat.order(&readings, &t).presses, counts);
}

#[test]
fn a_light_with_the_room_not_read_lights_as_a_plain_light_and_begins_no_cycle() {
    let t = sound();
    let mut thermostat = Thermostat::new();
    let unread = Temperatures { room_c: None, ..at_room(45.0, 21.0, None, true) };
    let counts = thermostat.order(&unread, &t).presses;
    assert!(thermostat.light(&unread, &t).is_ok(), "a refusal needs a number to refuse by");
    assert_eq!(thermostat.cycle(), None, "a cycle needs a number to be carried to");
    assert_eq!(thermostat.order(&unread, &t).presses.light, counts.light + 1, "the press is counted, as a plain light is");
}

#[test]
fn the_switch_is_not_a_cycles_to_move_a_light_leaves_it_where_it_stood() {
    let t = sound();
    let mut thermostat = Thermostat::restored(Switch::Off);
    thermostat.light(&at_water(50.0, true), &t).unwrap();
    assert_eq!(thermostat.order(&at_water(50.0, false), &t).switch, Switch::Off);
}

// ---------------------------------------------------------------- what the board says of a cycle and of a refusal

#[test]
fn a_refused_light_is_said_to_the_page_as_a_code_and_two_numbers_and_to_the_journal_in_words() {
    let room = Refusal { nominal: Nominal::Room, now_c: 22.3, to_c: 22.0 };
    assert_eq!(room.wire(), "light_refused_room now=22.3 to=22.0", "the board gives data: the page has the words");
    let water = Refusal { nominal: Nominal::Water, now_c: 61.04, to_c: 60.0 };
    assert_eq!(water.wire(), "light_refused_water now=61.0 to=60.0");
    assert_eq!(room.line(), "a Light was refused: the room is at 22.3 °C, its target is 22.0 °C");
    assert_eq!(water.line(), "a Light was refused: the water is at 61.0 °C, its setpoint is 60.0 °C");
}

#[test]
fn the_journal_says_when_a_cycle_begins_and_how_it_ended() {
    assert_eq!(thermostat::Cycle { nominal: Nominal::Room, to_c: 22.0 }.began_line(), "a Light begins a cycle to the room's target of 22.0 °C");
    assert_eq!(thermostat::Cycle { nominal: Nominal::Water, to_c: 60.0 }.began_line(), "a Light begins a cycle to the water's setpoint of 60.0 °C");
    assert_eq!(Ended::Reached.line(), "the cycle ended: the nominal was reached");
    assert_eq!(Ended::Out.line(), "the cycle ended: the fire is out");
    assert_eq!(Ended::Stopped.line(), "the cycle ended: a person pressed Stop");
}

// ---------------------------------------------------------------- the mode changed under the cycle (the review of 30.10)

/// A cycle of the room begun over the boiler's water, at 50 °C against a setpoint of 60.
fn cycling_by_the_room() -> Thermostat {
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_room(50.0, 21.0, house(23.0, 21.0), true), &sound()).unwrap();
    assert_eq!(thermostat.order(&at_room(50.0, 21.0, house(23.0, 21.0), false), &sound()).water, Water::Burn, "the scaffolding: the cycle goes");
    thermostat
}

#[test]
fn a_change_of_what_the_burner_holds_ends_the_cycle_and_the_new_nominal_is_not_carried_to() {
    // a cycle begun over the boiler's water, a nominal of the water: the burner is switched to hold the return, whose water stands inside its band
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(50.0, true), &sound_with(Hold::Boiler, 0.0)).unwrap();
    thermostat.order(&at_water(50.0, false), &sound_with(Hold::Boiler, 0.0));
    let held_by_the_return = Temperatures { return_c: Some(58.0), ..at_water(50.0, false) };
    let order = thermostat.order(&held_by_the_return, &sound_with(Hold::Return, 0.0));
    assert_eq!(order.water, Water::Keep, "the return in its band: the ordinary word, not the old cycle's Burn");
    assert_eq!(thermostat.ended(), Some(Ended::ModeChanged));
    assert_eq!(thermostat.cycle(), None);
}

#[test]
fn a_change_of_the_source_of_the_demand_ends_the_cycle() {
    // the room was the source and is not any more (the house is asked by the schedule or by nobody): there is no room's target to carry the fire to
    let mut thermostat = cycling_by_the_room();
    let no_room = Temperatures { room_target_c: None, ..at_room(50.0, 21.0, house(23.0, 21.0), false) };
    assert_ne!(thermostat.order(&no_room, &sound()).water, Water::Burn, "the cycle of the room went on to the water's setpoint after the room stopped being the source");
    assert_eq!(thermostat.ended(), Some(Ended::ModeChanged));
}

#[test]
fn a_change_to_the_mode_of_power_ends_the_cycle_as_a_change_of_mode_and_not_as_a_nominal_reached() {
    let mut thermostat = cycling_by_the_room();
    thermostat.order(&at_room(50.0, 21.0, house(23.0, 21.0), false), &sound_with(Hold::Power, 0.0));
    assert_eq!(thermostat.ended(), Some(Ended::ModeChanged), "the journal said the nominal was reached, and it was not");
    assert_eq!(thermostat.cycle(), None);
}

#[test]
fn the_target_of_the_room_moving_under_the_cycle_is_not_a_change_of_mode_the_cycle_goes_to_the_new_target() {
    // the pair: the schedule moves the room's target from 22 to 21.5 (a profile of the same source); the cycle is the room's still
    let mut thermostat = cycling_by_the_room();
    let moved = Temperatures { room_target_c: Some(21.5), ..at_room(50.0, 21.0, house(23.0, 21.0), false) };
    assert_eq!(thermostat.order(&moved, &sound()).water, Water::Burn);
    assert_eq!(thermostat.ended(), None);
    assert_eq!(thermostat.cycle().map(|c| (c.nominal, c.to_c)), Some((Nominal::Room, 21.5)));
}

#[test]
fn the_journal_names_a_change_of_mode() {
    assert_eq!(Ended::ModeChanged.line(), "the cycle ended: the mode that held the burner was changed");
}

// ---------------------------------------------------------------- the inputs of a cycle that can change under it (the inventory of tools/cycle_inputs.py)
//
// Every input the cycle reads - `Tuning.hold`, `Tuning.want_c`, `Temperatures.{room_c, room_target_c, boiler_c, return_c}`, the fire's `out`, the fault - and every input the word of the water below it is made of
// (`asked`, `return_heard`, the wait and its floor, the hysteresis, the barrier) is held by one row here or above. The page saves a setting and the board takes it at the next second (`Heat::take`): the rows swap the SOUND between two looks.

fn sound_by(hold: Hold, want_c: f32, hysteresis_c: f32, wait_s: f32) -> thermostat::Sound {
    Tuning { hold, want_c, hysteresis_c, step: step(), mid: step(), small: step(), mid_c: 8.0, small_c: 3.0, power: step(), lockout: Lockout { wait_s, floor_c: 30.0, sure_s: 0.0 } }.sound(88.0).expect("the numbers are sound")
}

#[test]
fn the_setpoint_changed_under_a_cycle_of_the_water_is_the_new_number_to_carry_the_fire_to() {
    // raised: the water at 62 would have ended the cycle at 60, and it goes on to 65
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(50.0, true), &sound_by(Hold::Boiler, 60.0, 4.0, 0.0)).unwrap();
    thermostat.order(&at_water(50.0, false), &sound_by(Hold::Boiler, 60.0, 4.0, 0.0));
    assert_eq!(thermostat.order(&at_water(62.0, false), &sound_by(Hold::Boiler, 65.0, 4.0, 0.0)).water, Water::Burn, "the setpoint was raised to 65");
    assert_eq!(thermostat.cycle().map(|c| c.to_c), Some(65.0));
    // lowered under the water: reached
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(50.0, true), &sound_by(Hold::Boiler, 60.0, 4.0, 0.0)).unwrap();
    thermostat.order(&at_water(50.0, false), &sound_by(Hold::Boiler, 60.0, 4.0, 0.0));
    assert_ne!(thermostat.order(&at_water(57.0, false), &sound_by(Hold::Boiler, 55.0, 4.0, 0.0)).water, Water::Burn, "the setpoint was lowered to 55 under the water at 57");
    assert_eq!(thermostat.ended(), Some(Ended::Reached));
}

#[test]
fn the_setpoint_lowered_under_a_cycle_of_the_room_moves_its_ceiling_with_it() {
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_room(50.0, 21.0, house(23.0, 21.0), true), &sound_by(Hold::Boiler, 60.0, 4.0, 0.0)).unwrap();
    assert_eq!(thermostat.order(&at_room(58.0, 21.0, house(23.0, 21.0), false), &sound_by(Hold::Boiler, 60.0, 4.0, 0.0)).water, Water::Burn);
    assert_ne!(thermostat.order(&at_room(58.0, 21.0, house(23.0, 21.0), false), &sound_by(Hold::Boiler, 57.0, 4.0, 0.0)).water, Water::Burn, "the ceiling is 57 now, the water is at 58");
    assert_eq!(thermostat.ended(), Some(Ended::Water));
}

#[test]
fn the_target_of_the_room_lowered_under_the_room_by_a_profile_or_a_setting_ends_the_cycle_reached() {
    // the schedule moves the room from the comfort target (22) to the eco one (20.5) while the room stands at 21: nothing to carry the fire to
    let mut thermostat = cycling_by_the_room();
    let eco = Temperatures { room_target_c: Some(20.5), ..at_room(50.0, 21.0, house(23.0, 21.0), false) };
    assert_ne!(thermostat.order(&eco, &sound()).water, Water::Burn);
    assert_eq!(thermostat.ended(), Some(Ended::Reached));
}

#[test]
fn a_room_that_becomes_the_source_under_a_cycle_of_the_water_ends_it_as_a_change_of_source() {
    // the reverse of "the room stopped being the source": the house was asked by nobody and is asked by the room now
    // the water stands inside its band (58 of a setpoint of 60), where the ordinary word is Keep and only a cycle says Burn
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(58.0, true), &sound()).unwrap();
    thermostat.order(&at_water(58.0, false), &sound());
    let by_the_room = Temperatures { room_c: Some(21.0), room_target_c: Some(TARGET), asked: house(23.0, 21.0), ..at_water(58.0, false) };
    assert_ne!(thermostat.order(&by_the_room, &sound()).water, Water::Burn, "the cycle of the water went on to the room's target");
    assert_eq!(thermostat.ended(), Some(Ended::ModeChanged));
}

#[test]
fn every_other_hold_ends_a_cycle_begun_by_one_and_the_ones_it_was_not_begun_by_are_all_the_same_to_it() {
    let holds = [Hold::Boiler, Hold::Return];
    for from in holds {
        for to in [Hold::Boiler, Hold::Return, Hold::Power] {
            let mut thermostat = Thermostat::new();
            thermostat.light(&at_water(50.0, true), &sound_with(from, 0.0)).unwrap();
            thermostat.order(&at_water(50.0, false), &sound_with(from, 0.0));
            thermostat.order(&at_water(50.0, false), &sound_with(to, 0.0));
            let want = if to == from { None } else { Some(Ended::ModeChanged) };
            assert_eq!(thermostat.ended(), want, "begun by {from:?}, looked at as {to:?}");
            assert_eq!(thermostat.cycle().is_some(), to == from, "{from:?} -> {to:?}");
        }
    }
}

#[test]
fn a_held_return_that_says_nothing_ends_a_cycle_of_the_return_as_the_boiler_does() {
    let t = sound_with(Hold::Return, 0.0);
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(50.0, true), &t).unwrap();
    let blind = Temperatures { return_c: None, ..at_water(50.0, false) };
    assert_ne!(thermostat.order(&blind, &t).water, Water::Burn);
    assert_eq!(thermostat.ended(), Some(Ended::Unread));
}

#[test]
fn what_the_house_says_does_not_matter_to_a_cycle_whatever_it_says_and_whenever_it_changes_its_mind() {
    let t = sound();
    let not_read = Some(Asked { demand: demand::Demand::Warm, why: demand::Why::NotRead });
    let starting = Some(Asked { demand: demand::Demand::Warm, why: demand::Why::Starting });
    let cold = Some(Asked { demand: demand::Demand::Cold, why: demand::Why::BelowTarget });
    for asked in [None, house(23.0, 21.0), cold, not_read, starting] {
        let mut thermostat = Thermostat::new();
        thermostat.light(&at_room(50.0, 21.0, house(23.0, 21.0), true), &t).unwrap();
        assert_eq!(thermostat.order(&at_room(50.0, 21.0, asked, false), &t).water, Water::Burn, "the house says {asked:?}");
        assert_eq!(thermostat.ended(), None);
    }
}

#[test]
fn the_hysteresis_the_wait_and_the_floor_changed_under_a_cycle_change_nothing_of_it() {
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(50.0, true), &sound_by(Hold::Boiler, 60.0, 4.0, 7200.0)).unwrap();
    thermostat.order(&at_water(50.0, false), &sound_by(Hold::Boiler, 60.0, 4.0, 7200.0));
    for (drop_c, wait_s) in [(2.0, 7200.0), (10.0, 7200.0), (4.0, 0.0), (4.0, 99999.0)] {
        let order = thermostat.order(&at_water(57.0, false), &sound_by(Hold::Boiler, 60.0, drop_c, wait_s));
        assert_eq!(order.water, Water::Burn, "hysteresis {drop_c}, wait {wait_s}");
        assert_eq!(thermostat.lock_left_s(), None, "the wait is not shown while the cycle goes");
    }
    assert_eq!(thermostat.ended(), None);
}

#[test]
fn a_cycle_that_ended_with_its_fire_is_followed_by_the_ordinary_wait_of_a_fire_that_burned() {
    // the fire of a cycle is a fire like another: it burned, and when it goes out the wait before a light begins
    let t = sound_with(Hold::Boiler, 7200.0);
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(50.0, true), &t).unwrap();
    thermostat.order(&at_water(50.0, false), &t);
    thermostat.order(&at_water(50.0, true), &t);
    assert_eq!(thermostat.ended(), Some(Ended::Out));
    assert!(thermostat.lock_left_s().is_some(), "no wait began after the fire of a cycle");
    assert_eq!(thermostat.order(&at_water(50.0, true), &t).water, Water::Keep, "the water asks, the wait holds it");
}

#[test]
fn a_restart_of_the_board_does_not_bring_a_cycle_back_it_is_a_word_of_a_press_and_not_a_memory() {
    // the switch is kept in the flash and comes back; a Light is not the switch, and the fire the board resumes goes by the ordinary rules
    let thermostat = Thermostat::restored(Switch::On);
    assert_eq!(thermostat.cycle(), None);
    let mut thermostat = thermostat;
    assert_eq!(thermostat.ended(), None);
}

// ---------------------------------------------------------------- the target is the person's to change while the cycle goes (9.35)

/// The room's readings with the target of the room at TARGET_C: a person moved it on the page, the house asks the thermostat for it in the next second.
fn at_room_to(target_c: f32, room_c: f32, out: bool) -> Temperatures {
    Temperatures { room_target_c: Some(target_c), ..at_room(45.0, room_c, house(23.0, room_c), out) }
}

#[test]
fn a_target_raised_in_the_middle_of_a_cycle_carries_the_fire_to_the_new_one_and_the_cycle_does_not_end_at_the_old() {
    let t = sound();
    let mut thermostat = cycling_by_the_room();
    assert_eq!(thermostat.cycle().map(|c| (c.nominal, c.to_c)), Some((Nominal::Room, TARGET)), "the scaffolding: the cycle goes to 22");
    // the person raises the target to 23 while the room is at 21.5; the next second the cycle is to 23
    assert_eq!(thermostat.order(&at_room_to(23.0, 21.5, false), &t).water, Water::Burn);
    assert_eq!(thermostat.cycle().map(|c| (c.nominal, c.to_c)), Some((Nominal::Room, 23.0)), "the cycle takes the target that stands now");
    // the room passes the old target: the cycle goes on
    for room_c in [21.9, 22.0, 22.4, 22.99] {
        assert_eq!(thermostat.order(&at_room_to(23.0, room_c, false), &t).water, Water::Burn, "the room at {room_c} is under the new target");
        assert_eq!(thermostat.ended(), None, "the cycle ended at {room_c}, under the new target");
    }
    // at the new target it ends the ordinary way
    assert_ne!(thermostat.order(&at_room_to(23.0, 23.0, false), &t).water, Water::Burn);
    assert_eq!(thermostat.ended(), Some(Ended::Reached));
    assert_eq!(thermostat.cycle(), None);
}

#[test]
fn a_target_lowered_under_the_room_in_the_middle_of_a_cycle_ends_the_cycle_as_a_nominal_reached() {
    let t = sound();
    let mut thermostat = cycling_by_the_room();
    assert_eq!(thermostat.order(&at_room_to(TARGET, 21.5, false), &t).water, Water::Burn, "the scaffolding: the cycle goes at 21.5 under 22");
    // the person lowers the target to 21 while the room stands at 21.5
    assert_ne!(thermostat.order(&at_room_to(21.0, 21.5, false), &t).water, Water::Burn);
    assert_eq!(thermostat.ended(), Some(Ended::Reached));
    assert_eq!(thermostat.cycle(), None);
}

#[test]
fn a_cycle_of_the_water_lives_one_second_once_the_room_is_the_source_of_the_demand() {
    // the page opens the panel of the room's target in a cycle of the water too; it cannot hold the water's cycle, which the thermostat ends in the first second the room is the source
    let t = sound();
    let mut thermostat = Thermostat::new();
    thermostat.light(&at_water(50.0, true), &t).unwrap();
    assert_eq!(thermostat.order(&at_water(50.0, false), &t).water, Water::Burn, "the scaffolding: the cycle of the water goes");
    assert_eq!(thermostat.cycle().map(|c| c.nominal), Some(Nominal::Water));
    let order = thermostat.order(&at_room_to(TARGET, 21.0, false), &t);
    assert_ne!(order.water, Water::Burn, "the water's cycle went on after the room became the source");
    assert_eq!(thermostat.ended(), Some(Ended::ModeChanged));
    assert_eq!(thermostat.cycle(), None);
}
