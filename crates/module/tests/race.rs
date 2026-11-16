//! Two persons pressing toggle at once: the other side of what the thing is must
//! be decided where the switching is - by the thing's owner, one command at a time -, or both read off, both ask for on,
//! and two presses leave one switch-over.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;

use between::Desk;
use module::{toggle, Module, Modules, Order};

const WITHIN: Duration = Duration::from_secs(2);

/// A relay slow to switch, as a real one is between a command and its answer: long enough for a second press to read
/// the state while the first is still on its way. Its owner is a task of its own, and only the owner writes it.
struct SlowRelay {
    on: AtomicBool,
    asked: Mutex<Vec<bool>>,
    desk: Desk<Order, Result<(), String>, 4, 4>,
}

impl SlowRelay {
    /// The owner's work: ORDER carried out on the relay.
    fn carry(&self, order: Order) -> Result<(), String> {
        let on = match order {
            Order::Switch(on) => on,
            Order::Toggle => !self.on.load(Ordering::SeqCst),
            other => return Err(format!("a relay does not {other:?}")),
        };
        thread::sleep(Duration::from_millis(100));
        self.asked.lock().unwrap().push(on);
        self.on.store(on, Ordering::SeqCst);
        Ok(())
    }
}

impl Module for SlowRelay {
    fn card(&self) -> &'static str {
        "relays"
    }

    fn order(&self, name: &str, order: Order) -> Option<Result<(), String>> {
        (name == "pump").then(|| self.desk.ask(order, WITHIN).unwrap_or_else(|refused| Err(refused.to_string())))
    }
}

/// A keeper of the pump that lets every command through, carried out in its own task, as the automaton does in a stop.
struct Keeper {
    desk: Desk<(&'static Modules, String, Order), Result<(), String>, 4, 4>,
}

impl Module for Keeper {
    fn card(&self) -> &'static str {
        "automaton"
    }


    fn through(&self, modules: &'static Modules, name: &str, order: Order) -> Option<Result<(), String>> {
        (name == "pump").then(|| {
            self.desk.ask((modules, name.to_string(), order), WITHIN).unwrap_or_else(|refused| Err(refused.to_string()))
        })
    }
}

/// Starts the task that owns DESK and carries out what is asked there by EACH, for as long as the tests run.
fn owner<Q: Send + 'static, A: Send + 'static>(
    desk: &'static Desk<Q, A, 4, 4>,
    each: impl Fn(Q) -> A + Send + 'static,
) {
    thread::spawn(move || loop {
        desk.serve_for(Duration::from_secs(1), &each);
    });
}

fn relay(on: bool) -> &'static SlowRelay {
    let r: &'static SlowRelay =
        Box::leak(Box::new(SlowRelay { on: AtomicBool::new(on), asked: Mutex::new(Vec::new()), desk: Desk::new() }));
    owner(&r.desk, move |order| r.carry(order));
    r
}

fn keeper() -> &'static Keeper {
    let k: &'static Keeper = Box::leak(Box::new(Keeper { desk: Desk::new() }));
    owner(&k.desk, |(modules, name, order): (&'static Modules, String, Order)| module::carry(modules, &name, order));
    k
}

/// Two toggles of the pump started together; what the relay was asked, in order.
fn two_at_once(modules: &'static Modules, r: &'static SlowRelay) -> Vec<bool> {
    let start = Arc::new(Barrier::new(2));
    let presses: Vec<_> = (0..2)
        .map(|_| {
            let start = start.clone();
            thread::spawn(move || {
                start.wait();
                toggle(modules, "pump").expect("toggled")
            })
        })
        .collect();
    for press in presses {
        press.join().unwrap();
    }
    r.asked.lock().unwrap().clone()
}

#[test]
fn two_toggles_at_once_of_a_thing_nobody_keeps_are_two_switch_overs() {
    let r = relay(false);
    let modules: &'static Modules = Box::leak(Box::new([r as &dyn Module]));
    let asked = two_at_once(modules, r);
    assert_eq!(asked, [true, false], "two presses switched it over once");
    assert!(!r.on.load(Ordering::SeqCst), "two switch-overs leave it as it was");
}

#[test]
fn two_toggles_at_once_of_a_kept_thing_are_two_switch_overs() {
    let r = relay(false);
    let k = keeper();
    let modules: &'static Modules = Box::leak(Box::new([k as &dyn Module, r as &dyn Module]));
    let asked = two_at_once(modules, r);
    assert_eq!(asked, [true, false], "two presses switched it over once");
    assert!(!r.on.load(Ordering::SeqCst), "two switch-overs leave it as it was");
}

#[test]
fn one_toggle_switches_it_over() {
    let r = relay(false);
    let modules: &'static Modules = Box::leak(Box::new([r as &dyn Module]));
    toggle(modules, "pump").expect("toggled");
    assert!(r.on.load(Ordering::SeqCst), "one toggle leaves it on");
    assert_eq!(*r.asked.lock().unwrap(), [true]);
}
