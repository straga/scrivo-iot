//! A command by address: the address a client sends is looked up in the board's dictionary, and
//! what the thing does not take is refused in words before any module is asked.

use page::dictionary::{self, Command, Entry, Kind, Range};

static TABLE: [Entry; 5] = [
    Entry { can: &["switch", "toggle"], ..Entry::row("relays", "pump", &[Kind::On]) },
    Entry { can: &["press"], ..Entry::row("automaton", "stop", &[Kind::Button]) },
    Entry {
        unit: "Pa",
        can: &["hold"],
        range: Some(Range { min: 0, max: 300, step: 1, at: "slider" }),
        ..Entry::row("fan", "draught", &[Kind::Number])
    },
    Entry { unit: "Pa", ..Entry::part("fan", "draught", "slider", &[Kind::Number]) },
    Entry { unit: "°C", ..Entry::row("flue", "flue_temp", &[Kind::Number]) },
];

#[test]
fn a_command_a_thing_takes_is_found_by_its_address() {
    assert_eq!(dictionary::command(&TABLE, "on", "0", ""), Ok((0, Command::Switch(true))));
    assert_eq!(dictionary::command(&TABLE, "off", "0", ""), Ok((0, Command::Switch(false))));
    assert_eq!(dictionary::command(&TABLE, "toggle", "0", ""), Ok((0, Command::Toggle)));
    assert_eq!(dictionary::command(&TABLE, "press", "1", ""), Ok((1, Command::Press)));
    assert_eq!(dictionary::command(&TABLE, "hold", "2", " 45\n"), Ok((2, Command::Hold(45))));
}

#[test]
fn an_address_nobody_has_a_part_and_a_command_a_thing_does_not_take_are_refused_by_name() {
    let refused = |what, at, body| dictionary::command(&TABLE, what, at, body).expect_err("taken");
    assert!(refused("on", "x", "").contains("'x' is no address"));
    assert!(refused("on", "42", "").contains("no thing of this board has the address 42"));
    assert!(refused("hold", "3", "5").contains("address 3"), "a part is no thing a command reaches");
    let why = refused("press", "0", "");
    assert!(why.contains("pump") && why.contains("press") && why.contains("switch"), "{why}");
    assert!(refused("on", "4", "").contains("flue_temp"));
    assert!(refused("hold", "2", "301").contains("0 to 300"));
    assert!(refused("hold", "2", "loud").contains("whole number"));
    assert!(refused("jump", "0", "").contains("jump"));
}
