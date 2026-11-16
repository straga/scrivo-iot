//! What the board calls itself on the network: `<device>-<last two bytes of the station MAC>`, `burner-f950`, in DHCP and in mDNS - at most 30 bytes, what the DHCP
//! client's hostname holds, and always ending in the tail. The recovery image names itself `scrivo-safe-<the same tail>` (scrivo-safe/main/host_name.c), so one board is one tail in both images.
//!
//! The expected names are written out from the device and the MAC, not computed from the code under test.

const MAC: [u8; 6] = [0x34, 0x85, 0x18, 0xab, 0xf9, 0x50];

#[test]
fn the_name_of_a_board_is_the_device_and_the_last_two_bytes_of_its_mac() {
    assert_eq!(board_name::of("burner", MAC).as_deref(), Some("burner-f950"));
}

#[test]
fn the_tail_is_lower_case_and_two_digits_to_a_byte_as_the_recovery_image_writes_it() {
    // scrivo-safe/main/host_name.c: snprintf("scrivo-safe-%02x%02x", mac[4], mac[5])
    assert_eq!(board_name::of("burner", [0, 0, 0, 0, 0x00, 0x0a]).as_deref(), Some("burner-000a"));
    assert_eq!(board_name::of("burner", [0, 0, 0, 0, 0xAB, 0xCD]).as_deref(), Some("burner-abcd"));
    assert_eq!(board_name::tail(MAC), "f950");
}

#[test]
fn an_underscore_of_the_device_becomes_a_hyphen_and_a_capital_a_small_letter() {
    // a host name has no underscore; ESPHome does the same with a device's name
    assert_eq!(board_name::of("bench_s3", MAC).as_deref(), Some("bench-s3-f950"));
    assert_eq!(board_name::of("Burner", MAC).as_deref(), Some("burner-f950"));
}

#[test]
fn a_device_name_that_cannot_be_a_host_name_gives_no_name_and_none_is_made_up() {
    assert_eq!(board_name::of("", MAC), None, "no device named");
    assert_eq!(board_name::of("-burner", MAC), None, "a label does not begin with a hyphen");
    assert_eq!(board_name::of("burner-", MAC), None, "a label does not end with a hyphen before the tail is added to it");
    assert_eq!(board_name::of("горелка", MAC), None, "only letters, digits and hyphens");
    assert_eq!(board_name::of("burner 2", MAC), None);
    assert_eq!(board_name::of("burner.v2", MAC), None, "a dot would be a second label");
}

#[test]
fn a_name_is_at_most_thirty_bytes_the_most_a_dhcp_hostname_holds() {
    assert_eq!(board_name::NAME_MAX, 30);
    // 25 bytes of the device, a hyphen and four digits are thirty: whole
    let fits = "d".repeat(25);
    assert_eq!(board_name::of(&fits, MAC), Some(format!("{fits}-f950")));
    assert_eq!(board_name::of(&fits, MAC).map(|n| n.len()), Some(30));
}

#[test]
fn a_longer_device_name_is_cut_so_that_the_tail_of_the_mac_stays() {
    // a device of forty bytes: its first twenty-five, then the tail - the board is still found by its tail
    let long = "d".repeat(40);
    assert_eq!(board_name::of(&long, MAC), Some(format!("{}-f950", "d".repeat(25))));
    // one over the line is cut by that one
    assert_eq!(board_name::of(&"d".repeat(26), MAC), Some(format!("{}-f950", "d".repeat(25))));
    // and whatever the device, the name ends in the tail and is not longer than thirty
    for device in ["a", "burner", "bench_s3", &"x".repeat(25), &"x".repeat(26), &"x".repeat(58), &"x".repeat(200)] {
        let name = board_name::of(device, MAC).unwrap();
        assert!(name.len() <= 30, "{name}");
        assert!(name.ends_with("-f950"), "{name}");
    }
}

#[test]
fn a_cut_that_leaves_a_hyphen_at_the_end_drops_it_so_that_the_name_has_no_double_hyphen() {
    // twenty-five bytes of this one end in a hyphen
    assert_eq!(board_name::of("aaaa-aaaa-aaaa-aaaa-aaaa-bbbb", MAC).as_deref(), Some("aaaa-aaaa-aaaa-aaaa-aaaa-f950"));
    // a name the device has in capitals and underscores is cut after it is made a host name's label
    assert_eq!(board_name::of(&format!("{}_X", "A".repeat(24)), MAC), Some(format!("{}-f950", "a".repeat(24))));
}
