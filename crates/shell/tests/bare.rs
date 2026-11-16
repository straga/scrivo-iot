//! What the shell says on a board that has nothing but the engine, and that it names no device in its own help.
//!
//! The shell is the engine's: it knows the words every board has. A board with no meter, no relay and no button has none of `meter`,
//! `toggle`, `press`, `on`, `off`, and for such a board each is a word nobody knows - with or without arguments, and answered alike
//! - while a word the board does have keeps its answer. The help the shell writes carries no name of a device: an example in it is a
//! placeholder, or it comes with the module that brings the word.

use std::cell::RefCell;
use std::convert::Infallible;
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

/// A board that is the engine and nothing more: no word of its own, and none of the optional words of the shell.
struct Bare;

impl Board for Bare {
    fn status(&mut self, line: &mut dyn FnMut(&str)) {
        line("version bare");
    }

    fn restart(&mut self) {}

    fn down(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn takes(&self, _name: &str) -> bool {
        false
    }
}

/// A board with things: it switches, presses and goes on and off, and has a word of its own, `meter`, that a module brings.
struct Furnished;

impl Board for Furnished {
    fn status(&mut self, line: &mut dyn FnMut(&str)) {
        line("version furnished");
    }

    fn restart(&mut self) {}

    fn down(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn toggle(&mut self, name: &str) -> Result<String, String> {
        Ok(format!("{name} switched over"))
    }

    fn words(&self) -> Vec<Word> {
        vec![Word { name: "meter", help: "What the meter reads" }]
    }

    fn word(&mut self, name: &str, words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
        match (name, words) {
            ("meter", []) => {
                line("voltage 230");
                Some(Ok(()))
            }
            _ => None,
        }
    }
}

fn run(input: &str, board: &mut dyn Board) -> String {
    let out = Out::default();
    let mut shell = Shell::new(out.clone()).unwrap();
    for &b in input.as_bytes() {
        shell.feed(b, board).unwrap();
    }
    let text = String::from_utf8_lossy(&out.0.borrow()).into_owned();
    text.replace('\r', "")
}

/// The word each line of `help` starts with.
fn listed(help: &str) -> Vec<String> {
    help.lines()
        .skip_while(|l| !l.contains("Commands:"))
        .skip(1)
        .filter_map(|l| l.split_whitespace().next())
        .filter(|w| !w.ends_with(':') && *w != "rust>")
        .map(str::to_string)
        .collect()
}

/// Names of things a device has and the engine does not, which no help of the shell may carry.
const DEVICES: [&str; 8] = ["burner", "igniter", "role_ntc", "relay1", "buffer tank", "fan", "pellet", "energy"];

#[test]
fn a_bare_board_lists_only_the_words_every_board_has() {
    let out = run("help\n", &mut Bare);
    let words = listed(&out);
    assert_eq!(words, ["status", "restart", "down", "fell", "log", "settings", "set"], "{out}");
}

#[test]
fn meter_is_unknown_on_a_bare_board_and_a_word_of_the_board_where_a_module_brings_it() {
    let bare = run("meter\n", &mut Bare);
    assert!(bare.contains("error: unknown command"), "{bare}");
    assert!(!bare.contains("meter refused"), "{bare}");

    let furnished = run("help\nmeter\n", &mut Furnished);
    assert!(listed(&furnished).contains(&"meter".to_string()), "{furnished}");
    assert!(furnished.contains("voltage 230"), "{furnished}");
}

#[test]
fn every_form_of_an_optional_word_a_board_lacks_answers_alike() {
    for form in ["toggle", "toggle a", "toggle a b", "press", "press a", "on", "on a", "off", "off a", "off a b"] {
        let out = run(&format!("{form}\n"), &mut Bare);
        assert!(out.contains("error: unknown command"), "{form:?} -> {out}");
        assert!(!out.contains("missing required argument") && !out.contains("unexpected argument"), "{form:?} -> {out}");
    }
}

#[test]
fn an_unknown_command_names_the_words_the_board_has() {
    let bare = run("bogus\n", &mut Bare);
    let line = bare.lines().find(|l| l.contains("unknown command")).unwrap_or_default();
    for word in ["status", "restart", "down", "fell", "log", "settings", "set"] {
        assert!(line.contains(word), "{word} missing from {line:?}");
    }
    assert!(!line.contains("toggle") && !line.contains("meter"), "{line:?}");

    let furnished = run("toggle\ndim 40\n", &mut Furnished);
    let line = furnished.lines().find(|l| l.contains("unknown command")).unwrap_or_default();
    assert!(line.contains("meter") && line.contains("toggle"), "{line:?}");
}

#[test]
fn a_word_the_board_has_keeps_its_own_answer_to_a_missing_argument() {
    let out = run("toggle\n", &mut Furnished);
    assert!(out.contains("missing required argument"), "{out}");
}

#[test]
fn no_help_of_the_shell_carries_the_name_of_a_device() {
    for (kind, board) in [("bare", &mut Bare as &mut dyn Board), ("furnished", &mut Furnished as &mut dyn Board)] {
        for ask in ["help\n", "help toggle\n", "help press\n", "help on\n", "help off\n", "help log\n", "help set\n", "log\nlog a\nset a\nfell x\n"] {
            let out = run(ask, board);
            for name in DEVICES {
                assert!(!out.to_lowercase().contains(name), "{kind}: {ask:?} names {name}: {out}");
            }
        }
    }
}

/// A board that counts how often the shell asks it for its words and for the optional words it takes.
#[derive(Default)]
struct Counted {
    words_asked: std::cell::Cell<usize>,
    takes_asked: std::cell::Cell<usize>,
}

impl Board for Counted {
    fn status(&mut self, line: &mut dyn FnMut(&str)) {
        line("version counted");
    }

    fn restart(&mut self) {}

    fn down(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn words(&self) -> Vec<Word> {
        self.words_asked.set(self.words_asked.get() + 1);
        vec![Word { name: "meter", help: "What the meter reads" }]
    }

    fn word(&mut self, name: &str, words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
        match (name, words) {
            ("meter", []) => {
                line("voltage 230");
                Some(Ok(()))
            }
            _ => None,
        }
    }

    fn takes(&self, name: &str) -> bool {
        self.takes_asked.set(self.takes_asked.get() + 1);
        name == "toggle"
    }

    fn toggle(&mut self, name: &str) -> Result<String, String> {
        Ok(format!("{name} switched over"))
    }
}

#[test]
fn the_words_of_a_board_are_asked_once_for_a_shell_however_many_bytes_it_is_fed() {
    let mut board = Counted::default();
    let out = Out::default();
    let mut shell = Shell::new(out.clone()).unwrap();
    for &b in "status\nmeter\nbogus\nhelp\ntoggle x\npress x\n".as_bytes() {
        shell.feed(b, &mut board).unwrap();
    }
    assert_eq!(board.words_asked.get(), 1, "the board's words were collected again on a byte");
    let text = String::from_utf8_lossy(&out.0.borrow()).replace('\r', "");
    assert!(text.contains("version counted") && text.contains("voltage 230") && text.contains("x switched over"), "{text}");
    assert!(text.contains("error: unknown command; the words of this board: status, restart, down, toggle, fell, log, settings, set, meter"), "{text}");
    assert!(text.contains("meter     What the meter reads"), "help lost the board's word: {text}");

    // another shell is another stream with its own ask: the connections of the console are many
    let mut second = Shell::new(Out::default()).unwrap();
    for &b in b"status\n" {
        second.feed(b, &mut board).unwrap();
    }
    assert_eq!(board.words_asked.get(), 2);
}
