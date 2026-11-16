//! The load of the two cores is said by two rows of the board's card, which stand LAST in the burner's table: the card's first rows are the same on every board, and a row put among them would move the address of everything after.

use page::dictionary::{address, Kind};

#[test]
fn the_load_of_each_core_is_a_row_of_the_system_card_in_per_cent_and_stands_last_in_the_table() {
    let all = &burner_api::ALL;
    let (core_0, core_1) = (address(all, "system", "core_0").expect("core_0"), address(all, "system", "core_1").expect("core_1"));
    assert_eq!((core_0, core_1), (all.len() - 2, all.len() - 1));
    assert!(dictionary::system::cores_stand_last(all));
    for at in [core_0, core_1] {
        let row = &all[at];
        assert_eq!((row.unit, row.kinds, row.can.len()), ("%", &[Kind::Number][..], 0), "a load is a number of per cent and no one's to command");
    }
}

#[test]
fn the_first_rows_of_the_board_s_card_are_where_every_board_has_them() {
    let all = &burner_api::ALL;
    assert_eq!(address(all, "system", "console"), Some(0));
    assert_eq!(address(all, "system", "uptime"), Some(dictionary::system::UPTIME));
}
