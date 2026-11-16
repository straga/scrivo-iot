//! What the console answers a burner, line by line, on the inputs a person types: written down once, before the commands that belong to
//! one device moved out of the shell into the modules that carry them out, and held to it after.
//!
//! The expected transcript is `tests/golden/console.txt`.
//! It is rewritten only by a person who means the console to answer otherwise:
//! `UPDATE_GOLDEN=1 cargo test -p shell --test console_golden`.

use std::cell::RefCell;
use std::convert::Infallible;
use std::fmt::Write as _;
use std::rc::Rc;

use shell::{Board, Shell, Word};

#[derive(Clone, Default)]
struct Out(Rc<RefCell<Vec<u8>>>);

impl embedded_io::ErrorType for Out {
    type Error = Infallible;
}

impl embedded_io::Write for Out {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Infallible> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
}

/// A board answering as the burner does: relays that switch, a button, a fan that holds a percent, pellets that are counted; no bus, no energy.
struct Burnerish;

impl Board for Burnerish {
    fn status(&mut self, line: &mut dyn FnMut(&str)) {
        line("version stragabases3-26.10.06.7");
        line("running ota_0");
    }

    fn restart(&mut self) {}

    fn down(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn toggle(&mut self, name: &str) -> Result<String, String> {
        match name {
            "igniter" => Ok("igniter on - off by itself in 5:00".into()),
            _ => Err(format!("nothing on this board switches {name} on and off")),
        }
    }

    fn press(&mut self, name: &str) -> Result<String, String> {
        match name {
            "stop" => Ok("stop pressed".into()),
            _ => Err(format!("nothing on this board presses {name}")),
        }
    }

    fn switch(&mut self, name: &str, on: bool) -> Result<String, String> {
        match name {
            "igniter" => Ok(format!("igniter {}", if on { "on" } else { "off" })),
            _ => Err(format!("nothing on this board turns {name} on or off")),
        }
    }

    fn words(&self) -> Vec<Word> {
        vec![
            Word { name: "dim", help: "Hold the fan at a percent of air: dim 40, dim 0 to stop it" },
            Word { name: "reset", help: "Zero the pellets counted: reset pellets, then reset pellets yes" },
        ]
    }

    // the words as the burner's modules say them (module::say)
    fn word(&mut self, name: &str, words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
        match (name, words) {
            ("dim", []) => Some(Err("dim takes a number: dim 40, dim 0 to stop it".into())),
            ("dim", [word]) => Some(match word.parse::<u32>() {
                Ok(0..=100) => {
                    line(&format!("fan held at {word}%"));
                    Ok(())
                }
                Ok(_) => Err("the fan holds 0 to 100".into()),
                Err(_) => Err(format!("'{word}' is no whole number: dim 40, dim 0 to stop it")),
            }),
            ("dim", [_, extra, ..]) => Some(Err(format!("dim takes one number, and '{extra}' is a second: dim 40, dim 0 to stop it"))),
            ("reset", []) => Some(Err("reset takes what to zero: reset pellets".into())),
            ("reset", ["pellets", rest @ ..]) => Some(match rest {
                [] => {
                    line("reset pellets zeroes today, month and total of the pellets burned. To do it: reset pellets yes");
                    Ok(())
                }
                ["yes"] => {
                    line("pellets zeroed: today, month and total");
                    Ok(())
                }
                [other, ..] => Err(format!("'{other}' is not yes; nothing zeroed")),
            }),
            _ => None,
        }
    }

    fn settings(&mut self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        line("Time comes from: auto");
        line("Hold until: 44 °C");
        Ok(())
    }

    fn set(&mut self, name: &str, value: &str) -> Result<String, String> {
        match name {
            "auto_hold_c" if value.parse::<u32>().is_err() => Err(format!("auto_hold_c: '{value}' is not a whole number from 5 to 85")),
            _ => Ok(format!("{name}: {value}")),
        }
    }
}

const INPUTS: &[&str] = &[
    "help",
    "status",
    "toggle igniter",
    "toggle pump",
    "toggle",
    "press stop",
    "press pump",
    "on igniter",
    "off igniter",
    "on pump",
    "dim 40",
    "dim 0",
    "dim 120",
    "dim half",
    "dim",
    "reset pellets",
    "reset pellets yes",
    "reset pellets maybe",
    "reset energy",
    "reset energy yes",
    "reset",
    "reset foo",
    "bus scan",
    "bus",
    "meter",
    "settings",
    "set auto_hold_c 50",
    "set auto_hold_c abc",
    "set",
    "fell",
    "log",
    "reboot",
    "flame pretend steady",
    "help dim",
    "help toggle",
];

fn transcript() -> String {
    let mut all = String::new();
    for input in INPUTS {
        let out = Out::default();
        let mut shell = Shell::new(out.clone()).unwrap();
        let mut board = Burnerish;
        for &b in format!("{input}\n").as_bytes() {
            shell.feed(b, &mut board).unwrap();
        }
        let text = String::from_utf8_lossy(&out.0.borrow()).replace('\r', "");
        writeln!(all, "=== {input}\n{text}").unwrap();
    }
    all
}

#[test]
fn the_console_answers_a_burner_on_the_inputs_it_answered_before_the_commands_moved() {
    let now = transcript();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/console.txt");
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden")).unwrap();
        std::fs::write(path, &now).unwrap();
        return;
    }
    let then = std::fs::read_to_string(path).expect("tests/golden/console.txt: take it with UPDATE_GOLDEN=1 on the code before the move");
    assert_eq!(now, then, "the console answers otherwise than it did");
}
