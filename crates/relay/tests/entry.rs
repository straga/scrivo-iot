//! The relay as a row of a device's dictionary: what a person may do with it.

use page::dictionary::{self as d, Command, Kind};

#[test]
fn a_relay_is_a_row_with_an_on_and_off_a_person_may_turn_on_off_or_over() {
    let one = [relay::entry::row("relays", "relay1")];
    assert_eq!((one[0].card, one[0].name, one[0].kinds), ("relays", "relay1", &[Kind::On][..]));
    assert_eq!(d::command(&one, "on", "0", ""), Ok((0, Command::Switch(true))));
    assert_eq!(d::command(&one, "off", "0", ""), Ok((0, Command::Switch(false))));
    assert_eq!(d::command(&one, "toggle", "0", ""), Ok((0, Command::Toggle)));
}

#[test]
fn a_relay_takes_no_press_and_no_number() {
    let one = [relay::entry::row("relays", "relay1")];
    assert_eq!(d::command(&one, "press", "0", "").unwrap_err(), "relay1 takes no press: it takes switch, toggle");
    assert!(d::command(&one, "hold", "0", "5").is_err());
}
