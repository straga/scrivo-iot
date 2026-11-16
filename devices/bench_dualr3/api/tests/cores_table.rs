//! How busy each core is is a property of the board and not of what is hung on it: the bench_dualr3 says it by the same two rows the burner does (`dictionary::system::CORES`), last in its table, so that no address before them moves.

use page::dictionary::address;

#[test]
fn the_load_of_each_core_is_said_by_the_two_rows_every_device_puts_last() {
    let all = &bench_dualr3_api::ALL;
    assert!(dictionary::system::cores_stand_last(all), "the rows of the cores do not end the table");
    assert_eq!(address(all, "system", "core_0"), Some(all.len() - 2));
    assert_eq!(address(all, "system", "core_1"), Some(all.len() - 1));
}

#[test]
fn the_first_rows_of_the_board_s_card_are_where_every_board_has_them() {
    let all = &bench_dualr3_api::ALL;
    assert_eq!(address(all, "system", "console"), Some(0));
    assert_eq!(address(all, "system", "uptime"), Some(dictionary::system::UPTIME));
}
