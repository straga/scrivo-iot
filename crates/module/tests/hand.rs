//! One point a person's command passes: every hand command of the contract - a
//! switch over, on or off, a number held - goes through the module that keeps the thing, which judges it by what it is
//! doing NOW and carries it out in its own task, so nothing it does can come between the verdict and the organ. The
//! automaton's own road to the organs is a command by role, which is not a person's and does not pass there.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, ThreadId};
use std::time::Duration;

use between::Desk;
use module::{command, hold, press, switch, toggle, Module, Modules, Order, Role};

const WITHIN: Duration = Duration::from_secs(2);

/// What the keeper's task is asked: a person's command to judge, or a press of its own button.
enum Asked {
    Through(&'static Modules, String, Order),
    Press,
}

/// The automaton as the contract sees it: running or not, the things it keeps, and the task it decides in.
struct Keeper {
    running: AtomicBool,
    desk: Desk<Asked, Result<(), String>, 4, 4>,
    task: OnceLock<ThreadId>,
}

impl Keeper {
    /// The keeper's own task: what it is asked, judged and carried out here.
    fn carry(&self, asked: Asked) -> Result<(), String> {
        match asked {
            Asked::Through(modules, name, order) => match self.running.load(Ordering::Relaxed) {
                true => Err(format!("the automaton is kindling - press stop, then {name} by hand")),
                false => module::carry(modules, &name, order),
            },
            Asked::Press => {
                self.running.store(false, Ordering::Relaxed);
                Ok(())
            }
        }
    }

    fn ask(&self, asked: Asked) -> Result<(), String> {
        self.desk.ask(asked, WITHIN).unwrap_or_else(|refused| Err(refused.to_string()))
    }
}

impl Module for Keeper {
    fn card(&self) -> &'static str {
        "automaton"
    }


    fn through(&self, modules: &'static Modules, name: &str, order: Order) -> Option<Result<(), String>> {
        (["igniter", "gate"].contains(&name)).then(|| self.ask(Asked::Through(modules, name.to_string(), order)))
    }

    fn order(&self, name: &str, order: Order) -> Option<Result<(), String>> {
        (name == "stop" && order == Order::Press).then(|| self.ask(Asked::Press))
    }
}

/// The organs: a relay and a gate, writing down every command that reached them, and whether it came from the
/// keeper's task.
struct Organs {
    keeper: &'static Keeper,
    reached: Mutex<Vec<String>>,
}

impl Organs {
    fn row(&self, what: String) -> Result<(), String> {
        let in_keeper = self.keeper.task.get() == Some(&thread::current().id());
        self.reached.lock().unwrap().push(format!("{what}{}", if in_keeper { "" } else { " outside the keeper's task" }));
        Ok(())
    }
}

static IGNITER: Role<bool> = Role::new("igniter");

impl Module for Organs {
    fn card(&self) -> &'static str {
        "relays"
    }

    fn order(&self, name: &str, order: Order) -> Option<Result<(), String>> {
        match (name, order) {
            ("igniter", Order::Switch(on)) => Some(self.row(format!("igniter {on}"))),
            // it is off, so the other side is on
            ("igniter", Order::Toggle) => Some(self.row("igniter true".to_string())),
            ("gate", Order::Hold(value)) => Some(self.row(format!("gate {value}"))),
            _ => None,
        }
    }

    fn command(&self, role: &str, what: &dyn std::any::Any) -> Option<Result<(), String>> {
        (role == IGNITER.name).then(|| {
            let on = module::take::<bool>(what, role)?;
            self.reached.lock().unwrap().push(format!("automaton igniter {on}"));
            Ok(())
        })
    }
}

struct Board {
    modules: &'static Modules,
    organs: &'static Organs,
}

fn board(running: bool) -> Board {
    let keeper: &'static Keeper =
        Box::leak(Box::new(Keeper { running: AtomicBool::new(running), desk: Desk::new(), task: OnceLock::new() }));
    thread::spawn(move || {
        keeper.task.set(thread::current().id()).unwrap();
        loop {
            keeper.desk.serve_for(Duration::from_secs(1), |asked| keeper.carry(asked));
        }
    });
    let organs: &'static Organs = Box::leak(Box::new(Organs { keeper, reached: Mutex::new(Vec::new()) }));
    Board { modules: Box::leak(Box::new([keeper as &dyn Module, organs as &dyn Module])), organs }
}

impl Board {
    fn reached(&self) -> Vec<String> {
        self.organs.reached.lock().unwrap().clone()
    }
}

#[test]
fn a_person_is_refused_by_every_hand_command_while_the_automaton_runs_and_nothing_reaches_the_organ() {
    let b = board(true);
    let refusals = [
        toggle(b.modules, "igniter").expect_err("toggle went through"),
        switch(b.modules, "igniter", true).expect_err("switch on went through"),
        hold(b.modules, "gate", 40).expect_err("hold went through"),
    ];
    for words in &refusals {
        assert!(words.contains("press stop"), "the refusal is the automaton's words: {words}");
    }
    assert!(b.reached().is_empty(), "a refused command reached the organ: {:?}", b.reached());
}

#[test]
fn in_a_stop_the_same_commands_reach_the_organ_from_the_keepers_task() {
    let b = board(false);
    toggle(b.modules, "igniter").expect("toggle");
    switch(b.modules, "igniter", true).expect("switch on");
    hold(b.modules, "gate", 40).expect("hold");
    assert_eq!(b.reached(), ["igniter true", "igniter true", "gate 40"]);
}

#[test]
fn stop_passes_while_every_organ_is_refused_and_then_the_hand_is_free() {
    let b = board(true);
    press(b.modules, "stop").expect("stop is never refused");
    switch(b.modules, "igniter", true).expect("after the stop the hand is free");
    assert_eq!(b.reached(), ["igniter true"]);
}

#[test]
fn the_automaton_commands_its_organs_by_role_past_the_point_that_stops_a_person() {
    let b = board(true);
    command(b.modules, &IGNITER, true).expect("the automaton's own command");
    assert_eq!(b.reached(), ["automaton igniter true"]);
}

#[test]
fn a_thing_nobody_keeps_is_not_refused_by_the_keeper() {
    // the pair of the refusal: the keeper speaks about its own names, not against everything while it runs
    let b = board(true);
    let words = switch(b.modules, "lamp", true).expect_err("nobody has a lamp");
    assert!(!words.contains("press stop"), "the refusal is the contract's, not the automaton's: {words}");
}
