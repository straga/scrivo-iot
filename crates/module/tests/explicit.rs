//! Commands that say what they want: on and off set the state asked for, the same one twice leaves
//! the same state; toggle is the other side of what the thing is now, decided by the thing's owner; a button is
//! pressed.

use std::sync::Mutex;

use module::{press, switch, toggle, Module, Order};

/// A relay that keeps whether it is on, answers only on and off, and writes down what it was asked.
struct Relay {
    on: Mutex<bool>,
    asked: Mutex<Vec<bool>>,
}

impl Module for Relay {
    fn card(&self) -> &'static str {
        "relays"
    }

    fn order(&self, name: &str, order: Order) -> Option<Result<(), String>> {
        let mut now = self.on.lock().unwrap();
        let on = match (name, order) {
            ("pump", Order::Switch(on)) => on,
            ("pump", Order::Toggle) => !*now,
            _ => return None,
        };
        self.asked.lock().unwrap().push(on);
        *now = on;
        Some(Ok(()))
    }
}

/// A card with one button.
struct Buttons;

impl Module for Buttons {
    fn card(&self) -> &'static str {
        "automaton"
    }

    fn order(&self, name: &str, order: Order) -> Option<Result<(), String>> {
        (name == "stop" && order == Order::Press).then(|| Ok(()))
    }
}

fn relay(on: bool) -> &'static Relay {
    Box::leak(Box::new(Relay { on: Mutex::new(on), asked: Mutex::new(Vec::new()) }))
}

#[test]
fn on_twice_leaves_it_on_and_off_twice_leaves_it_off() {
    let r = relay(false);
    let modules: &module::Modules = Box::leak(Box::new([r as &dyn Module]));
    for want in [true, true, false, false] {
        switch(modules, "pump", want).expect("switched");
        assert_eq!(*r.on.lock().unwrap(), want, "after asking for {want}");
    }
    assert_eq!(*r.asked.lock().unwrap(), [true, true, false, false], "each asked for what it said, never the other side");
}

#[test]
fn toggle_asks_for_the_other_side_of_what_the_thing_says_it_is_now() {
    for was in [false, true] {
        let r = relay(was);
        let modules: &module::Modules = Box::leak(Box::new([r as &dyn Module]));
        toggle(modules, "pump").expect("toggled");
        assert_eq!(*r.asked.lock().unwrap(), [!was], "it was {was}");
        assert_eq!(*r.on.lock().unwrap(), !was);
    }
}

#[test]
fn toggle_of_a_thing_that_is_not_on_or_off_is_refused_by_its_name() {
    let r = relay(false);
    let modules: &module::Modules = Box::leak(Box::new([r as &dyn Module]));
    let why = toggle(modules, "water").expect_err("a reading was toggled");
    assert!(why.contains("water"), "{why}");
    let why = toggle(modules, "lamp").expect_err("a thing nobody has was toggled");
    assert!(why.contains("lamp"), "{why}");
    assert!(r.asked.lock().unwrap().is_empty(), "the relay was asked: {:?}", r.asked.lock().unwrap());
}

#[test]
fn a_button_is_pressed_by_the_module_whose_button_it_is_and_nothing_else_is() {
    let modules: &'static module::Modules = &[&Buttons];
    assert_eq!(press(modules, "stop"), Ok(()));
    let why = press(modules, "pump").expect_err("a relay was pressed");
    assert!(why.contains("pump"), "{why}");
}
