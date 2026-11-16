//! The words a board adds to the shell's own, and the shell's own that a board without the thing does not have: listed in `help`, run with
//! the words after them, refused in words, unknown where the board has none.

use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;

use shell::{Board, Shell, Word, WORD_WORDS};

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

/// A board with one word of its own, `dim`, and a say in which of the shell's optional words it takes.
#[derive(Default)]
struct Fanned {
    heard: Vec<(String, Vec<String>)>,
    toggled: Vec<String>,
    lacks: Vec<&'static str>,
    own: bool,
}

impl Board for Fanned {
    fn status(&mut self, _line: &mut dyn FnMut(&str)) {}

    fn restart(&mut self) {}

    fn down(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn words(&self) -> Vec<Word> {
        match self.own {
            true => vec![Word { name: "dim", help: "Hold the fan at a percent of air: dim 40" }],
            false => Vec::new(),
        }
    }

    fn word(&mut self, name: &str, words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
        self.heard.push((name.to_string(), words.iter().map(|w| w.to_string()).collect()));
        match (name, words) {
            ("dim", ["40"]) => {
                line("fan held at 40%");
                line("and the gate follows");
                Some(Ok(()))
            }
            ("dim", ["500"]) => Some(Err("the fan holds 0 to 100".into())),
            _ => None,
        }
    }

    fn takes(&self, name: &str) -> bool {
        !self.lacks.contains(&name)
    }

    fn toggle(&mut self, name: &str) -> Result<String, String> {
        self.toggled.push(name.to_string());
        Ok(format!("{name} switched over"))
    }
}

fn run(input: &str, board: &mut Fanned) -> String {
    let out = Out::default();
    let mut shell = Shell::new(out.clone()).unwrap();
    for &b in input.as_bytes() {
        shell.feed(b, board).unwrap();
    }
    let text = String::from_utf8_lossy(&out.0.borrow()).into_owned();
    text.replace('\r', "")
}

#[test]
fn a_word_of_the_board_is_listed_in_help_with_its_line() {
    let out = run("help\n", &mut Fanned { own: true, ..Default::default() });
    assert!(out.lines().any(|l| l.trim_start().starts_with("dim") && l.contains("Hold the fan at a percent of air")), "{out}");
}

#[test]
fn a_board_without_the_word_does_not_list_it_and_the_word_is_unknown_on_it() {
    let mut board = Fanned::default();
    let out = run("help\ndim 40\nstatus\n", &mut board);
    assert!(!out.lines().any(|l| l.trim_start().starts_with("dim")), "{out}");
    assert!(out.contains("error: unknown command"), "{out}");
    assert!(board.heard.is_empty(), "a word nobody listed reached the board: {:?}", board.heard);
}

#[test]
fn a_word_of_the_board_gets_the_words_after_it_and_its_lines_are_written_as_they_come() {
    let mut board = Fanned { own: true, ..Default::default() };
    let out = run("dim 40\n", &mut board);
    assert_eq!(board.heard, [("dim".to_string(), vec!["40".to_string()])]);
    assert!(out.contains("fan held at 40%\nand the gate follows\n"), "{out}");
}

#[test]
fn a_word_the_board_refuses_says_why_after_its_name_and_the_shell_goes_on() {
    let mut board = Fanned { own: true, ..Default::default() };
    let out = run("dim 500\ndim 40\n", &mut board);
    assert!(out.contains("dim refused: the fan holds 0 to 100"), "{out}");
    assert!(out.contains("fan held at 40%"), "the line after a refusal did not run: {out}");
}

#[test]
fn words_nothing_takes_are_refused_in_words_naming_the_first() {
    let mut board = Fanned { own: true, ..Default::default() };
    let out = run("dim half\ndim\n", &mut board);
    assert!(out.contains("dim refused: nothing on this board takes half"), "{out}");
    assert!(out.contains("dim refused: nothing on this board takes it"), "{out}");
}

#[test]
fn a_word_with_more_words_than_the_shell_hands_over_is_refused_and_touches_nothing() {
    let mut board = Fanned { own: true, ..Default::default() };
    let line = format!("dim {}\n", ["1"; WORD_WORDS + 1].join(" "));
    let out = run(&line, &mut board);
    assert!(board.heard.is_empty(), "{:?}", board.heard);
    assert!(out.contains("unexpected argument"), "{out}");
}

#[test]
fn an_option_after_a_word_of_the_board_is_refused_and_touches_nothing() {
    let mut board = Fanned { own: true, ..Default::default() };
    let out = run("dim --fast\ndim -f\n", &mut board);
    assert!(board.heard.is_empty(), "{:?}", board.heard);
    assert!(out.contains("unexpected option"), "{out}");
}

#[test]
fn a_board_without_a_relay_a_button_or_a_switch_does_not_list_toggle_press_on_off_and_they_are_unknown() {
    let mut board = Fanned { lacks: vec!["toggle", "press", "on", "off"], ..Default::default() };
    let out = run("help\ntoggle relay1\npress stop\non igniter\noff igniter\n", &mut board);
    for word in ["toggle", "press", "on", "off"] {
        assert!(!out.lines().any(|l| l.trim_start().starts_with(word) && l.contains("  ")), "{word} is listed: {out}");
    }
    assert_eq!(out.matches("error: unknown command").count(), 4, "{out}");
    assert!(board.toggled.is_empty(), "{:?}", board.toggled);
    assert!(out.lines().any(|l| l.trim_start().starts_with("status")), "the shell's own words are gone too: {out}");
}

#[test]
fn a_board_that_takes_a_word_lists_it_and_runs_it_while_the_others_it_lacks_stay_out() {
    let mut board = Fanned { lacks: vec!["press"], ..Default::default() };
    let out = run("help\ntoggle relay1\npress stop\n", &mut board);
    assert_eq!(board.toggled, ["relay1"]);
    assert!(out.lines().any(|l| l.trim_start().starts_with("toggle")), "{out}");
    assert!(!out.lines().any(|l| l.trim_start().starts_with("press ")), "{out}");
    assert_eq!(out.matches("error: unknown command").count(), 1, "{out}");
}

#[test]
fn help_of_a_word_of_the_board_says_its_line_and_how_it_is_written_and_of_a_missing_word_says_unknown() {
    let mut board = Fanned { own: true, lacks: vec!["press"], ..Default::default() };
    let out = run("help dim\nhelp press\nhelp toggle\n", &mut board);
    assert!(out.contains("Hold the fan at a percent of air: dim 40\n\nUsage: dim [WORDS]..."), "{out}");
    assert_eq!(out.matches("error: unknown command").count(), 1, "help of a word the board lacks: {out}");
    assert!(out.contains("Usage: toggle"), "help of a word the board takes: {out}");
}

#[test]
fn tab_completes_the_words_of_the_board_and_not_the_ones_it_lacks() {
    let mut board = Fanned { own: true, lacks: vec!["press"], ..Default::default() };
    // the line is not ended: completion alone must not run the word
    let out = run("di\t", &mut board);
    assert!(out.ends_with("rust> dim "), "{out:?}");
    let out = run("pr\t", &mut board);
    assert!(out.ends_with("rust> pr"), "a word the board lacks was completed: {out:?}");
    let mut taking = Fanned::default();
    assert!(run("pr\t", &mut taking).ends_with("rust> press "), "a word the board takes is completed");
    assert!(board.heard.is_empty(), "completion alone ran a word: {:?}", board.heard);
}
