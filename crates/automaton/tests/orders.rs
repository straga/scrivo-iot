//! What the fire does with each word of an ORDER, and no thermostat anywhere: the order is written by the row, so a
//! row says what the fire does when it is told, whatever the water might have said to the thermostat. Every word has a pair: the state
//! it acts on and a state it does not.
//!
//! The rows of tests/rows.rs and its fellows run the real thermostat under the fire and say the same decisions are taken; these say
//! the fire's half of them in the fire's own terms.

use automaton::{Asked, Burning, Doing, Feed, Held, Lighting, Order, Presses, Seen, Silence, State, Step, Switch, Told, Tuning, Water, Why};
use flame::Flame;

fn tuning() -> Tuning {
    Tuning {
        purge_before_s: 30.0,
        purge_after_s: 20.0,
        igniter_s: 30.0,
        wait_for_flame_s: 90.0,
        tries: 3,
        dose_after_first: 0.1,
        rise_s: 60.0,
        flame_lost_s: 120.0,
        smoulder_s: 300.0,
        burn_off_s: 120.0,
        burn_out_min_s: 0.0,
        pump_on_c: 45.0,
        flue_holds_c: 100.0,
        flue_cold_c: 70.0,
        flue_fall_c: 10.0,
        flue_fall_s: 600.0,
        lit_s: 5.0,
        pump_off_below_c: 4.0,
        too_hot_c: 88.0,
        alarm_c: 95.0,
        sure_s: 10.0,
        relight_s: [20.0, 10.0, 10.0],
        relight_hot_c: 45.0,
        spin_up_s: 10.0,
        low_pressure_s: 20.0,
        kindling: Lighting { pa: 51.0, dose_s: 32.0 },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

fn sound() -> automaton::Sound {
    tuning().sound().expect("the numbers are sound")
}

fn level() -> Step {
    Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } }
}

fn another_level() -> Step {
    Step { pa: 80.0, feed: Feed { on_s: 2.0, off_s: 30.0 } }
}

fn cold() -> Seen {
    Seen { flame: Flame::Out, boiler_c: Some(40.0), flue_c: Some(20.0), draught_pa: Some(0.0), draught_low: false, gate: 0, silence: Silence::HEARD }
}

fn lit() -> Seen {
    Seen { flame: Flame::Steady, boiler_c: Some(40.0), flue_c: Some(150.0), draught_pa: Some(50.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// An order of this water with nobody asked and nothing pressed.
fn ordering(water: Water) -> Order {
    Order { water, step: level(), switch: Switch::Unasked, presses: Presses::default(), house_cold: false }
}

fn asked(water: Water) -> Order {
    Order { switch: Switch::On, presses: Presses { start: 1, stop: 0, light: 0 }, ..ordering(water) }
}

fn one_second(burning: &mut Burning, order: &Order, seen: Seen) -> Doing {
    burning.step(order, seen, 1.0, &sound())
}

fn seconds(burning: &mut Burning, order: &Order, seen: Seen, n: usize) -> Doing {
    let mut did = one_second(burning, order, seen);
    for _ in 1..n {
        did = one_second(burning, order, seen);
    }
    did
}

/// A fire taken to WORKING the short way: a Light from a stop, a purge, a kindling and a rising on an order that is `Keep` throughout.
fn working() -> Burning {
    let mut burning = Burning::new();
    let start = asked(Water::Keep);
    one_second(&mut burning, &lighted(Water::Keep), cold());
    seconds(&mut burning, &start, cold(), 30);
    seconds(&mut burning, &start, lit(), 92);
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach working");
    burning
}

fn smouldering() -> Burning {
    let mut burning = working();
    one_second(&mut burning, &asked(Water::Enough), lit());
    assert_eq!(burning.state(), State::Smouldering, "the scaffolding did not reach smouldering");
    burning
}

// ---------------------------------------------------------------- a stop

#[test]
fn burn_lights_a_stop_when_a_person_asked_for_heat() {
    let mut burning = Burning::new();
    let on = Order { switch: Switch::On, ..ordering(Water::Burn) };
    assert_eq!(one_second(&mut burning, &on, cold()).state, State::Purge, "asked, and the water says burn");
}

#[test]
fn burn_does_not_light_a_stop_nobody_asked_for_heat() {
    for switch in [Switch::Unasked, Switch::Off] {
        let mut burning = Burning::new();
        let order = Order { switch, ..ordering(Water::Burn) };
        assert_eq!(seconds(&mut burning, &order, cold(), 600).state, State::Stop, "{switch:?}: cold water and nobody asked");
    }
}

#[test]
fn keep_enough_level_and_silent_do_not_light_a_stop_that_was_asked_for_heat() {
    for water in [Water::Keep, Water::Enough, Water::Level, Water::Silent(Held::Return)] {
        let mut burning = Burning::new();
        let order = Order { switch: Switch::On, ..ordering(water) };
        assert_eq!(seconds(&mut burning, &order, cold(), 600).state, State::Stop, "{water:?}: only the water's `Burn` lights a stop by itself");
    }
}

#[test]
fn a_stop_with_no_start_in_the_order_lights_nothing() {
    let mut burning = Burning::new();
    let stop = Order { switch: Switch::Off, presses: Presses { start: 0, stop: 1, light: 0 }, ..ordering(Water::Burn) };
    assert_eq!(one_second(&mut burning, &stop, cold()).state, State::Stop);
}

// ---------------------------------------------------------------- working

#[test]
fn enough_sends_a_working_fire_down_to_smouldering() {
    let mut burning = working();
    assert_eq!(one_second(&mut burning, &asked(Water::Enough), lit()).state, State::Smouldering);
}

#[test]
fn burn_keep_and_level_leave_a_working_fire_working() {
    for water in [Water::Burn, Water::Keep, Water::Level] {
        let mut burning = working();
        assert_eq!(seconds(&mut burning, &asked(water), lit(), 30).state, State::Working, "{water:?}");
    }
}

// ---------------------------------------------------------------- smouldering

#[test]
fn burn_and_level_bring_a_smouldering_fire_back_to_work() {
    for water in [Water::Burn, Water::Level] {
        let mut burning = smouldering();
        assert_eq!(one_second(&mut burning, &asked(water), lit()).state, State::Working, "{water:?}");
    }
}

#[test]
fn keep_and_enough_leave_a_smouldering_fire_smouldering() {
    for water in [Water::Keep, Water::Enough] {
        let mut burning = smouldering();
        assert_eq!(seconds(&mut burning, &asked(water), lit(), 100).state, State::Smouldering, "{water:?}");
    }
}

#[test]
fn a_smouldering_fire_burns_itself_off_by_its_own_clock_whatever_the_order() {
    let mut burning = smouldering();
    let did = seconds(&mut burning, &asked(Water::Keep), lit(), 301);
    assert_eq!(did.state, State::BurningOff(None), "the time of smouldering is the fire's");
}

// ---------------------------------------------------------------- the silence of the held sensor

#[test]
fn a_silent_held_sensor_burns_a_live_fire_off_with_the_sensors_fault() {
    for mut burning in [working(), smouldering()] {
        let did = one_second(&mut burning, &asked(Water::Silent(Held::Return)), lit());
        assert_eq!(did.state, State::BurningOff(Some(Why::ReturnSilent)));
    }
}

#[test]
fn the_fires_own_silences_come_before_the_held_sensors() {
    let mut burning = working();
    let flame_gone = Seen { silence: Silence { flame: automaton::Heard::TooLong, ..Silence::HEARD }, ..lit() };
    let did = one_second(&mut burning, &asked(Water::Silent(Held::Return)), flame_gone);
    assert_eq!(did.state, State::BurningOff(Some(Why::FlameSilent)));
}

#[test]
fn a_silent_held_sensor_does_not_light_nor_fault_a_stop() {
    let mut burning = Burning::new();
    let order = Order { switch: Switch::On, ..ordering(Water::Silent(Held::Return)) };
    assert_eq!(seconds(&mut burning, &order, cold(), 600).state, State::Stop, "a sensor nobody hears is not a cold boiler, and a stop has nothing to burn off");
}

// ---------------------------------------------------------------- a stop pressed

#[test]
fn a_stop_burns_a_live_fire_off_and_only_once() {
    let mut burning = working();
    let stop = Order { switch: Switch::Off, presses: Presses { start: 1, stop: 1, light: 0 }, ..ordering(Water::Keep) };
    assert_eq!(one_second(&mut burning, &stop, lit()).state, State::BurningOff(None));
    assert_eq!(burning.told(&stop), Told::Nothing, "the press was taken");
}

#[test]
fn a_switch_that_says_off_without_a_press_burns_nothing_off() {
    // a board that came up to a live fire and was never asked: the switch is not a press, and a fire carried on stays
    let mut burning = working();
    let off = Order { switch: Switch::Off, presses: Presses { start: 1, stop: 0, light: 0 }, ..ordering(Water::Keep) };
    assert_eq!(seconds(&mut burning, &off, lit(), 60).state, State::Working);
}

#[test]
fn a_stop_acknowledges_a_fault_and_a_switch_alone_does_not() {
    let mut burning = working();
    let overheated = Seen { boiler_c: Some(90.0), ..lit() };
    seconds(&mut burning, &asked(Water::Keep), overheated, 1);
    seconds(&mut burning, &asked(Water::Keep), Seen { flame: Flame::Out, flue_c: Some(30.0), draught_pa: Some(0.0), draught_low: false, gate: 0, ..lit() }, 200);
    assert!(matches!(burning.state(), State::Fault(Why::Overheated(_))), "{:?}", burning.state());
    let off = Order { switch: Switch::Off, presses: Presses { start: 1, stop: 0, light: 0 }, ..ordering(Water::Burn) };
    assert!(matches!(one_second(&mut burning, &off, cold()).state, State::Fault(_)), "the switch is a level; a fault waits for a press");
    let stop = Order { presses: Presses { start: 1, stop: 1, light: 0 }, ..off };
    assert_eq!(one_second(&mut burning, &stop, cold()).state, State::Stop, "a press of Stop is the acknowledgement");
}

#[test]
fn both_buttons_moved_between_two_seconds_are_a_stop() {
    let mut burning = working();
    let both = Order { switch: Switch::On, presses: Presses { start: 2, stop: 1, light: 0 }, ..ordering(Water::Keep) };
    assert_eq!(one_second(&mut burning, &both, lit()).state, State::BurningOff(None), "Stop beats every source");
}

// ---------------------------------------------------------------- the level

#[test]
fn the_fire_burns_on_the_level_the_order_gives_it_and_follows_a_new_one_at_the_next_second() {
    let mut burning = working();
    let first = seconds(&mut burning, &asked(Water::Keep), lit(), 5);
    assert_eq!(first.fan, Asked::Hold(level().pa), "the order's draught");
    let other = Order { step: another_level(), ..asked(Water::Keep) };
    let then = seconds(&mut burning, &other, lit(), 1);
    assert_eq!(then.fan, Asked::Hold(another_level().pa), "the level is the order's of this second, not the one the fire was lit on");
}

#[test]
fn the_dose_and_the_pause_of_the_level_in_the_order_are_the_augers() {
    // 4.6 s of auger in 19.6 is about a quarter of the time, 2 s in 32 about a sixteenth: over 400 s the first feeds well over twice the second
    let turned = |step: Step| {
        let mut burning = working();
        let order = Order { step, ..asked(Water::Keep) };
        (0..400).filter(|_| one_second(&mut burning, &order, lit()).auger).count()
    };
    let (rich, lean) = (turned(level()), turned(another_level()));
    assert!(lean > 0, "the lean level feeds too");
    assert!(rich > lean * 2, "{rich} seconds of auger at 4.6 on and 15 off against {lean} at 2 on and 30 off");
}

// ---------------------------------------------------------------- a start switches on, a light lights

/// A press of Light on a board whose switch is on: the Start that switched it on was taken by the fire seconds ago, and only the light count has moved.
fn lighted(water: Water) -> Order {
    Order { switch: Switch::On, presses: Presses { start: 0, stop: 0, light: 1 }, ..ordering(water) }
}

#[test]
fn a_start_with_water_that_burns_lights_a_stop() {
    let mut burning = Burning::new();
    assert_eq!(one_second(&mut burning, &asked(Water::Burn), cold()).state, State::Purge, "the switch is on and the water says burn");
}

#[test]
fn a_start_alone_lights_no_stop_whose_water_does_not_burn() {
    for water in [Water::Keep, Water::Enough, Water::Level] {
        let mut burning = Burning::new();
        assert_eq!(seconds(&mut burning, &asked(water), cold(), 600).state, State::Stop, "{water:?}: a Start is the switch, and only the water's `Burn` lights a stop by itself");
    }
}

#[test]
fn a_light_lights_a_stop_whatever_the_water_says() {
    for water in [Water::Burn, Water::Keep, Water::Enough, Water::Level] {
        let mut burning = Burning::new();
        assert_eq!(one_second(&mut burning, &lighted(water), cold()).state, State::Purge, "{water:?}: a person lights his own boiler");
    }
}

#[test]
fn a_light_counted_once_lights_once_and_the_same_count_is_no_second_light() {
    let mut burning = Burning::new();
    let light = lighted(Water::Enough);
    assert_eq!(one_second(&mut burning, &light, cold()).state, State::Purge);
    assert_eq!(burning.told(&light), Told::Nothing, "the count was taken: it is no edge any more");
}

#[test]
fn a_light_is_not_heard_by_a_burning_off_nor_by_a_fault() {
    assert_eq!(one_second(&mut Burning::new(), &lighted(Water::Keep), cold()).state, State::Purge, "the pair: in a stop the same light is heard");
    // a burning off
    let mut burning = working();
    let stop = Order { switch: Switch::Off, presses: Presses { start: 1, stop: 1, light: 0 }, ..ordering(Water::Keep) };
    assert_eq!(one_second(&mut burning, &stop, lit()).state, State::BurningOff(None), "the scaffolding: a burning off");
    let light = Order { switch: Switch::On, presses: Presses { start: 1, stop: 1, light: 1 }, ..ordering(Water::Burn) };
    assert!(matches!(one_second(&mut burning, &light, lit()).state, State::BurningOff(_)), "a burning off may not be interrupted, by a light either");
    // a fault
    let mut burning = working();
    let overheated = Seen { boiler_c: Some(90.0), ..lit() };
    seconds(&mut burning, &asked(Water::Keep), overheated, 1);
    seconds(&mut burning, &asked(Water::Keep), Seen { flame: Flame::Out, flue_c: Some(30.0), draught_pa: Some(0.0), draught_low: false, gate: 0, ..lit() }, 200);
    assert!(matches!(burning.state(), State::Fault(Why::Overheated(_))), "the scaffolding: a fault, {:?}", burning.state());
    let light = Order { switch: Switch::On, presses: Presses { start: 1, stop: 0, light: 1 }, ..ordering(Water::Burn) };
    assert!(matches!(one_second(&mut burning, &light, cold()).state, State::Fault(_)), "a fault waits for a press of Stop and for nothing else");
}

#[test]
fn a_start_alone_is_not_heard_by_a_burning_off_nor_by_a_fault() {
    let mut burning = working();
    let stop = Order { switch: Switch::Off, presses: Presses { start: 1, stop: 1, light: 0 }, ..ordering(Water::Keep) };
    assert_eq!(one_second(&mut burning, &stop, lit()).state, State::BurningOff(None), "the scaffolding: a burning off");
    let start = Order { switch: Switch::On, presses: Presses { start: 2, stop: 1, light: 0 }, ..ordering(Water::Burn) };
    assert!(matches!(one_second(&mut burning, &start, lit()).state, State::BurningOff(_)), "a Start in a burning off asks for heat and interrupts nothing");
    let mut burning = working();
    seconds(&mut burning, &asked(Water::Keep), Seen { boiler_c: Some(90.0), ..lit() }, 1);
    seconds(&mut burning, &asked(Water::Keep), Seen { flame: Flame::Out, flue_c: Some(30.0), draught_pa: Some(0.0), draught_low: false, gate: 0, ..lit() }, 200);
    assert!(matches!(burning.state(), State::Fault(Why::Overheated(_))), "the scaffolding: a fault, {:?}", burning.state());
    let start = Order { switch: Switch::On, presses: Presses { start: 2, stop: 0, light: 0 }, ..ordering(Water::Burn) };
    assert!(matches!(one_second(&mut burning, &start, cold()).state, State::Fault(_)), "a Start does not answer a fault");
    let stop = Order { switch: Switch::Off, presses: Presses { start: 2, stop: 1, light: 0 }, ..ordering(Water::Burn) };
    assert_eq!(one_second(&mut burning, &stop, cold()).state, State::Stop, "a press of Stop does");
}
