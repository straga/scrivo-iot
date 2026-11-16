//! The wall switch as a row of a device's dictionary: read, never commanded, and a count of its toggles.

use page::dictionary::{self as d, Kind};

#[test]
fn a_wall_switch_is_read_and_never_commanded_and_counts_its_toggles_as_a_part_of_its_row() {
    let table = [switch::entry::row("switches", "switch1"), switch::entry::toggles("switches", "switch1")];
    assert_eq!(table[0].kinds, &[Kind::On]);
    assert_eq!(table[1].part_of, Some("switch1"));
    assert_eq!((table[1].name, table[1].kinds), ("toggles", &[Kind::Number][..]));
    assert_eq!((table[1].unit, table[1].decimals), ("toggles", Some(0)), "a count of presses is counted in toggles and has no digits after the point");
    assert_eq!(d::check(&table), Ok(()));
    assert_eq!(d::command(&table, "on", "0", "").unwrap_err(), "switch1 takes no switch: it takes no command");
    assert!(d::command(&table, "toggle", "0", "").is_err());
}
