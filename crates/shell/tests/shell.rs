//! The shell fed the bytes a terminal sends, against a board that only records what it was asked.

use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;

use shell::{Board, Shell, MAX_LINE};

#[derive(Clone, Default)]
struct Out(Rc<RefCell<Vec<u8>>>);

impl Out {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.borrow()).into_owned()
    }
}

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

#[derive(Default)]
struct Record {
    calls: Vec<&'static str>,
    down_fails: Option<String>,
    toggled: Vec<String>,
    meter_lines: bool,
    set: Vec<(String, String)>,
    switched: Vec<(String, bool)>,
    fell_lines: bool,
    fell_hard: Vec<bool>,
    loud: Vec<(String, String)>,
    room: Option<(usize, usize)>,
}

impl Board for Record {
    fn room(&self) -> Option<shell::Room> {
        self.room.map(|(free, largest)| shell::Room { free, largest })
    }

    fn status(&mut self, line: &mut dyn FnMut(&str)) {
        self.calls.push("status");
        line("version stragabases3-26.09.19.1");
        line("running ota_0");
    }

    fn restart(&mut self) {
        self.calls.push("restart");
    }

    fn down(&mut self) -> Result<(), String> {
        self.calls.push("down");
        match self.down_fails.take() {
            Some(why) => Err(why),
            None => Ok(()),
        }
    }

    /// `meter` is a word a module brings: this board has it when `meter_lines` says a meter is there.
    fn words(&self) -> Vec<shell::Word> {
        match self.meter_lines {
            true => vec![shell::Word { name: "meter", help: "What the meter reads" }],
            false => Vec::new(),
        }
    }

    fn word(&mut self, name: &str, words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
        match (name, words) {
            ("meter", []) if self.meter_lines => {
                self.calls.push("meter");
                line("voltage 238.7 V");
                line("packets 12 accepted, 0 rejected, 1 missed");
                Some(Ok(()))
            }
            _ => None,
        }
    }

    fn settings(&mut self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        self.calls.push("settings");
        line("Time comes from: auto");
        line("Time zone: Kyiv, Chisinau, Helsinki, Riga - UTC+2, +3 in summer");
        Ok(())
    }

    fn set(&mut self, name: &str, value: &str) -> Result<String, String> {
        self.calls.push("set");
        self.set.push((name.into(), value.into()));
        match name {
            "zone" if value.contains('/') => Err(format!("Time zone: '{value}' is none of kyiv, utc")),
            _ => Ok(format!("{name} {value}")),
        }
    }

    fn switch(&mut self, name: &str, on: bool) -> Result<String, String> {
        self.calls.push(if on { "on" } else { "off" });
        self.switched.push((name.to_string(), on));
        match name {
            "igniter" => Ok(format!("igniter {}", if on { "on - off by itself in 5:00" } else { "off" })),
            _ => Err(format!("this board carries no {name}; it carries: igniter")),
        }
    }

    fn fell(&mut self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        self.calls.push("fell");
        match self.fell_lines {
            true => {
                line("fell just before this start: a fault on core 0 at 0x42009abc (StoreProhibited)");
                line("trace 0x42009abc 0x4200912c - files and lines with: tools/where_fell.py burner <these addresses>");
                Ok(())
            }
            false => Err("nothing has fallen since the power came on".into()),
        }
    }

    fn loudness(&mut self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        self.calls.push("loudness");
        line("burner::fan info - as the board starts");
        line("burner::flue debug - asked for");
        Ok(())
    }

    fn set_loudness(&mut self, module: &str, level: &str) -> Result<String, String> {
        self.calls.push("set loudness");
        self.loud.push((module.into(), level.into()));
        match (module, level) {
            (_, "loud") => Err("'loud' is not a level; they are: off, error, warn, info, debug, trace".into()),
            ("burner::fna", _) => Err("this board has no module burner::fna; it has: burner::fan".into()),
            _ => Ok(format!("{module} {level}")),
        }
    }

    fn fall_now(&mut self, hard: bool) {
        self.calls.push("fall now");
        self.fell_hard.push(hard);
    }

    fn press(&mut self, name: &str) -> Result<String, String> {
        self.calls.push("press");
        match name {
            "stop" => Ok("stop pressed".into()),
            _ => Err(format!("{name} is no button")),
        }
    }

    fn toggle(&mut self, name: &str) -> Result<String, String> {
        self.calls.push("toggle");
        self.toggled.push(name.to_string());
        match name {
            "relay1" => Ok("relay1 asked to switch over".into()),
            _ => Err(format!("this board carries no {name}; it carries: relay1")),
        }
    }
}

fn run(input: &[u8], board: &mut Record) -> String {
    let out = Out::default();
    let mut shell = Shell::new(out.clone()).unwrap();
    for &b in input {
        shell.feed(b, board).unwrap();
    }
    out.text()
}

#[test]
fn status_prints_what_the_board_says() {
    let mut board = Record::default();
    let out = run(b"status\n", &mut board);
    assert_eq!(board.calls, ["status"]);
    assert!(out.contains("version stragabases3-26.09.19.1"), "{out:?}");
    assert!(out.contains("running ota_0"), "{out:?}");
}

#[test]
fn restart_and_down_reach_the_board_once_per_line_even_with_crlf() {
    let mut board = Record::default();
    run(b"restart\r\ndown\r\n", &mut board);
    assert_eq!(board.calls, ["restart", "down"]);
}

#[test]
fn down_that_fails_says_why_and_the_shell_goes_on() {
    let mut board = Record { down_fails: Some("no factory partition".into()), ..Default::default() };
    let out = run(b"down\nstatus\n", &mut board);
    assert_eq!(board.calls, ["down", "status"]);
    assert!(out.contains("no factory partition"), "{out:?}");
}

#[test]
fn unknown_command_is_refused_and_touches_nothing() {
    let mut board = Record::default();
    let out = run(b"reboot\nstatus\n", &mut board);
    assert_eq!(board.calls, ["status"]);
    assert!(out.contains("unknown command"), "{out:?}");
}

#[test]
fn command_with_argument_it_does_not_take_is_refused() {
    let mut board = Record::default();
    let out = run(b"down now\n", &mut board);
    assert!(board.calls.is_empty(), "{:?}", board.calls);
    assert!(!out.is_empty());
}

#[test]
fn line_of_max_length_still_runs() {
    let mut board = Record::default();
    let mut line = b"status".to_vec();
    line.resize(MAX_LINE, b' ');
    line.push(b'\n');
    run(&line, &mut board);
    assert_eq!(board.calls, ["status"]);
}

#[test]
fn longer_line_is_refused_whole_even_when_its_start_is_a_command() {
    // Cut to the buffer, "down" plus spaces would read as "down".
    let mut board = Record::default();
    let mut line = b"down".to_vec();
    line.resize(MAX_LINE + 1, b' ');
    line.push(b'\n');
    let out = run(&line, &mut board);
    assert!(board.calls.is_empty(), "{:?}", board.calls);
    assert!(out.contains(&format!("longer than {MAX_LINE}")), "{out:?}");
}

#[test]
fn after_a_refused_long_line_the_next_line_runs() {
    let mut board = Record::default();
    let mut input = vec![b'x'; MAX_LINE * 3];
    input.extend_from_slice(b"\nstatus\n");
    run(&input, &mut board);
    assert_eq!(board.calls, ["status"]);
}

#[test]
fn help_names_every_command() {
    let mut board = Record::default();
    let out = run(b"help\n", &mut board);
    for name in ["status", "restart", "down", "toggle", "on", "off", "settings", "set"] {
        assert!(out.contains(name), "{name} missing from {out:?}");
    }
    assert!(board.calls.is_empty());
}

#[test]
fn toggle_asks_the_board_for_the_thing_named_and_prints_its_answer() {
    let mut board = Record::default();
    let out = run(b"toggle relay1\n", &mut board);
    assert_eq!(board.toggled, ["relay1"]);
    assert!(out.contains("relay1 asked to switch over"), "{out:?}");
}

#[test]
fn toggle_of_a_thing_the_board_refuses_says_why_and_the_shell_goes_on() {
    let mut board = Record::default();
    let out = run(b"toggle pump\nstatus\n", &mut board);
    assert_eq!(board.calls, ["toggle", "status"]);
    assert!(out.contains("toggle refused: this board carries no pump"), "{out:?}");
}

#[test]
fn toggle_without_a_name_is_refused_and_touches_nothing() {
    let mut board = Record::default();
    let out = run(b"toggle\n", &mut board);
    assert!(board.calls.is_empty());
    assert!(out.contains("toggle"), "{out:?}");
}

#[test]
fn toggle_with_two_names_is_refused_and_touches_nothing() {
    let mut board = Record::default();
    run(b"toggle relay1 relay2\n", &mut board);
    assert!(board.calls.is_empty());
}

#[test]
fn meter_prints_what_the_board_reads() {
    let mut board = Record { meter_lines: true, ..Default::default() };
    let out = run(b"meter\n", &mut board);
    assert_eq!(board.calls, ["meter"]);
    assert!(out.contains("voltage 238.7 V\r\npackets 12 accepted"), "{out:?}");
}

#[test]
fn meter_on_a_board_without_one_is_an_unknown_word_and_the_shell_goes_on() {
    let mut board = Record::default();
    let out = run(b"meter\nstatus\n", &mut board);
    assert_eq!(board.calls, ["status"]);
    assert!(out.contains("error: unknown command"), "{out:?}");
}

#[test]
fn meter_with_an_argument_is_refused_and_touches_nothing() {
    let mut board = Record { meter_lines: true, ..Default::default() };
    run(b"meter now\n", &mut board);
    assert!(board.calls.is_empty());
}

#[test]
fn every_answer_ends_its_lines_with_one_carriage_return() {
    let mut board = Record { down_fails: Some("no factory partition".into()), meter_lines: true, ..Default::default() };
    let mut input = b"status\ndown\ntoggle relay1\ntoggle pump\nmeter\nsettings\nset zone utc\nrestart\n".to_vec();
    input.extend(vec![b'x'; MAX_LINE + 1]);
    input.push(b'\n');
    let out = run(&input, &mut board);
    assert!(out.contains("refused"), "{out:?}");
    assert!(!out.contains("\r\r"), "{out:?}");
}

#[test]
fn settings_prints_what_the_board_says() {
    let mut board = Record::default();
    let out = run(b"settings\n", &mut board);
    assert_eq!(board.calls, ["settings"]);
    assert!(out.contains("Time comes from: auto\r\nTime zone: Kyiv"), "{out:?}");
}

#[test]
fn settings_on_a_board_without_them_says_so() {
    struct Bare;
    impl Board for Bare {
        fn status(&mut self, _line: &mut dyn FnMut(&str)) {}
        fn restart(&mut self) {}
        fn down(&mut self) -> Result<(), String> {
            Ok(())
        }
    }
    let out = Out::default();
    let mut shell = Shell::new(out.clone()).unwrap();
    for &b in b"settings\nset zone utc\n" {
        shell.feed(b, &mut Bare).unwrap();
    }
    let out = out.text();
    assert!(out.contains("settings refused: this board has no settings"), "{out:?}");
    assert!(out.contains("set refused: this board has no settings"), "{out:?}");
}

#[test]
fn a_board_that_keeps_no_note_and_cannot_be_made_to_fall_says_both_rather_than_going_quiet() {
    // The neighbouring surface of `fell`: a board that carries neither. The second half is the one that would
    // go unnoticed - fall_now that does nothing looks exactly like a board about to restart, and a person
    // would sit waiting for it to come back.
    struct Bare;
    impl Board for Bare {
        fn status(&mut self, _line: &mut dyn FnMut(&str)) {}
        fn restart(&mut self) {}
        fn down(&mut self) -> Result<(), String> {
            Ok(())
        }
    }
    let out = Out::default();
    let mut shell = Shell::new(out.clone()).unwrap();
    for &b in b"fell\nfell now\n" {
        shell.feed(b, &mut Bare).unwrap();
    }
    let out = out.text();
    assert!(out.contains("fell refused: this board keeps no note of a fall"), "{out:?}");
    assert!(out.contains("fell now refused: this board cannot be made to fall"), "{out:?}");
}

#[test]
fn set_hands_the_name_and_the_value_to_the_board_and_prints_its_answer() {
    let mut board = Record::default();
    let out = run(b"set time time.google.com\n", &mut board);
    assert_eq!(board.set, [("time".to_string(), "time.google.com".to_string())]);
    assert!(out.contains("time time.google.com\r\n"), "{out:?}");
}

#[test]
fn set_of_a_value_the_board_refuses_says_why() {
    let mut board = Record::default();
    let out = run(b"set zone Europe/Kyiv\nstatus\n", &mut board);
    assert_eq!(board.calls, ["set", "status"]);
    assert!(out.contains("set refused: Time zone: 'Europe/Kyiv' is none of kyiv, utc"), "{out:?}");
}

#[test]
fn set_without_a_value_touches_nothing() {
    let mut board = Record::default();
    let out = run(b"set zone\nset\n", &mut board);
    assert!(board.calls.is_empty(), "{:?}", board.calls);
    assert!(out.contains("set"), "the refusal says what was expected: {out:?}");
}

#[test]
fn a_value_of_several_words_is_the_boards_to_judge_not_the_shells() {
    // the shell no longer counts the words: which values a setting takes is the setting's business, and a name a
    // person writes is one value however many words it has
    let mut board = Record::default();
    run(b"set time router internet\n", &mut board);
    assert_eq!(board.set, [("time".to_string(), "router internet".to_string())]);
}


#[test]
fn on_and_off_ask_the_board_for_the_thing_named_and_print_its_answer() {
    let mut board = Record::default();
    let out = run(b"on igniter\noff igniter\n", &mut board);
    assert_eq!(board.switched, [("igniter".to_string(), true), ("igniter".to_string(), false)]);
    assert!(out.contains("igniter on - off by itself in 5:00"), "{out:?}");
    assert!(out.contains("igniter off"), "{out:?}");
}

#[test]
fn on_of_a_thing_the_board_refuses_says_why_and_the_shell_goes_on() {
    let mut board = Record::default();
    let out = run(b"on pump\nstatus\n", &mut board);
    assert_eq!(board.calls, ["on", "status"]);
    assert!(out.contains("on refused: this board carries no pump"), "{out:?}");
}

#[test]
fn on_or_off_without_a_name_or_with_two_touches_nothing() {
    for line in [&b"on\n"[..], b"off\n", b"on igniter auger\n", b"off igniter now\n", b"on --igniter\n"] {
        let mut board = Record::default();
        run(line, &mut board);
        assert!(board.calls.is_empty(), "{:?}: {:?}", String::from_utf8_lossy(line), board.calls);
    }
}

#[test]
fn a_value_of_several_words_reaches_the_board_whole() {
    // a setting can take a name a person writes: "own:buffer tank" is one value, not a value and a stray word
    let mut board = Record::default();
    run(b"set role_ntc_3 own:buffer tank\n", &mut board);
    assert_eq!(board.set, [("role_ntc_3".to_string(), "own:buffer tank".to_string())]);
}

#[test]
fn a_value_of_one_word_is_unchanged_by_that() {
    let mut board = Record::default();
    run(b"set zone kyiv\n", &mut board);
    assert_eq!(board.set, [("zone".to_string(), "kyiv".to_string())]);
}

#[test]
fn fell_prints_the_note_the_board_kept() {
    // The one fact a board with no wire on it cannot otherwise give: where it faulted. A person who joins
    // after the boot lines have scrolled away asks for it by name.
    let mut board = Record { fell_lines: true, ..Default::default() };
    let out = run(b"fell\n", &mut board);
    assert_eq!(board.calls, ["fell"]);
    assert!(out.contains("0x42009abc"), "{out:?}");
    assert!(out.contains("where_fell.py"), "{out:?}");
}

#[test]
fn fell_on_a_board_that_has_not_fallen_says_so_and_invents_nothing() {
    // The pair. A note that is empty, or last week's, read as this week's sends somebody to the wrong repair,
    // so nothing at all has to read as nothing at all.
    let mut board = Record::default();
    let out = run(b"fell\n", &mut board);
    assert_eq!(board.calls, ["fell"]);
    assert!(out.contains("nothing has fallen"), "{out:?}");
    assert!(!out.contains("0x"), "{out:?}");
}

#[test]
fn fell_now_makes_the_board_fall_the_way_our_code_gives_up() {
    let mut board = Record::default();
    let out = run(b"fell now\n", &mut board);
    assert_eq!(board.calls, ["fall now"]);
    assert_eq!(board.fell_hard, [false]);
    assert!(out.contains("on purpose"), "{out:?}");
}

#[test]
fn fell_now_hard_takes_the_way_a_hardware_fault_takes() {
    // The two ways reach the note differently - our code speaks on its way to abort, a fault never speaks at
    // all - so proving one proves nothing about the other.
    let mut board = Record::default();
    run(b"fell now hard\n", &mut board);
    assert_eq!(board.calls, ["fall now"]);
    assert_eq!(board.fell_hard, [true]);
}

#[test]
fn a_word_after_fell_that_is_not_now_touches_nothing_and_names_what_is_allowed() {
    // The pair to both lines above: a board is not to be made to fall by a typo, and "fell" alone reads as a
    // question, not as an order.
    let mut board = Record::default();
    let out = run(b"fell soon\nfell now softly\n", &mut board);
    assert!(board.calls.is_empty(), "{:?}", board.calls);
    assert!(out.contains("now"), "{out:?}");
}

#[test]
fn log_alone_lists_the_modules_and_their_levels() {
    // So that a name is never read out of the code: the board says which modules it has.
    let mut board = Record::default();
    let out = run(b"log\n", &mut board);
    assert_eq!(board.calls, ["loudness"]);
    assert!(out.contains("burner::fan info"), "{out:?}");
    assert!(out.contains("burner::flue debug"), "{out:?}");
}

#[test]
fn log_with_a_module_and_a_level_turns_that_one_module_up() {
    let mut board = Record::default();
    let out = run(b"log burner::fan debug\n", &mut board);
    assert_eq!(board.calls, ["set loudness"]);
    assert_eq!(board.loud, [("burner::fan".to_string(), "debug".to_string())]);
    assert!(out.contains("burner::fan debug"), "{out:?}");
}

#[test]
fn a_level_that_is_not_one_is_refused_with_the_levels_there_are() {
    // The pair. A word the board does not know must not read as done: a person would watch a console that
    // says nothing and believe the details are on.
    let mut board = Record::default();
    let out = run(b"log burner::fan loud\n", &mut board);
    assert_eq!(board.calls, ["set loudness"]);
    assert!(out.contains("refused"), "{out:?}");
    assert!(out.contains("debug"), "the refusal names the levels: {out:?}");
}

#[test]
fn a_module_the_board_does_not_have_is_refused_with_the_ones_it_has() {
    let mut board = Record::default();
    let out = run(b"log burner::fna debug\n", &mut board);
    assert!(out.contains("refused"), "{out:?}");
    assert!(out.contains("burner::fan"), "{out:?}");
}

#[test]
fn log_with_a_module_and_no_level_touches_nothing_and_says_what_it_takes() {
    // Half a command is not a command: a module named without a level could as easily mean "show me this one"
    // as "turn it up", and guessing between them changes what the board does.
    let mut board = Record::default();
    let out = run(b"log burner::fan\nlog a b c\n", &mut board);
    assert!(board.calls.is_empty(), "{:?}", board.calls);
    assert!(out.contains("log"), "{out:?}");
}

#[test]
fn flame_is_no_command_the_burner_has_no_pretended_flame_and_the_shell_goes_on() {
    // the automaton burns by the sensor's flame and nothing else
    let mut board = Record::default();
    let out = run(b"flame pretend steady\nstatus\n", &mut board);
    assert_eq!(board.calls, ["status"]);
    assert!(out.contains("unknown command"), "{out:?}");
}

#[test]
fn help_names_no_flame() {
    let out = run(b"help\n", &mut Record::default());
    assert!(!out.lines().any(|l| l.trim_start().starts_with("flame")), "{out}");
    assert!(out.lines().any(|l| l.trim_start().starts_with("press")), "{out}");
}

#[test]
fn press_hands_the_board_the_button_named_and_prints_its_answer() {
    // a button of the automaton is pressed, not switched over
    let mut board = Record::default();
    let out = run(b"press stop\npress pump\n", &mut board);
    assert_eq!(board.calls, ["press", "press"]);
    assert!(out.contains("stop pressed"), "{out:?}");
    assert!(out.contains("press refused: pump is no button"), "{out:?}");
}

#[test]
fn press_without_a_name_is_refused_and_touches_nothing() {
    let mut board = Record::default();
    run(b"press\n", &mut board);
    assert!(board.calls.is_empty(), "{:?}", board.calls);
}

#[test]
fn status_on_a_heap_too_tight_to_print_it_says_the_numbers_and_does_not_ask_the_board() {
    // the report allocates: asked on a heap with no piece to spare it takes the board down (rust_oom, 08.10)
    let mut board = Record { room: Some((3000, 1200)), ..Record::default() };
    let out = run(b"status\n", &mut board);
    assert!(board.calls.is_empty(), "the board was asked: {:?}", board.calls);
    assert!(out.contains("1200"), "{out:?}");
    assert!(out.contains("3000"), "{out:?}");
    assert!(out.contains(&shell::STATUS_NEEDS.to_string()), "{out:?}");
}

#[test]
fn status_on_a_heap_with_a_piece_to_spare_prints_the_report() {
    let mut board = Record { room: Some((60_000, 40_000)), ..Record::default() };
    let out = run(b"status\n", &mut board);
    assert_eq!(board.calls, ["status"]);
    assert!(out.contains("running ota_0"), "{out:?}");
}

#[test]
fn status_runs_with_the_biggest_piece_exactly_the_need_and_not_one_byte_less() {
    let mut at = Record { room: Some((50_000, shell::STATUS_NEEDS)), ..Record::default() };
    run(b"status\n", &mut at);
    assert_eq!(at.calls, ["status"]);
    let mut under = Record { room: Some((50_000, shell::STATUS_NEEDS - 1)), ..Record::default() };
    run(b"status\n", &mut under);
    assert!(under.calls.is_empty());
}

#[test]
fn after_a_status_refused_for_room_the_next_command_runs() {
    let mut board = Record { room: Some((3000, 1200)), ..Record::default() };
    run(b"status\nrestart\n", &mut board);
    assert_eq!(board.calls, ["restart"]);
}
