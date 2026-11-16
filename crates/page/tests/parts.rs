//! The address of a part of the state. A module says WHICH of its own changed, and the name it says it by is the
//! card its rows stand on in the dictionary - so the address existed before this and nothing had to be invented.
//!
//! What these hold is that the address is the SAME address every time: across rounds, across restarts of the
//! board, and across a device gaining a card. A bit that moved would send one module's rows under another
//! module's name, and nothing about the answer would look wrong.

use page::{mark_of, Marks, Wanted, OTHER, SETTINGS, SYSTEM};

const BURNER: [&str; 6] = ["relays", "temperatures", "flue", "fan", "draught", "flame"];

#[test]
fn a_card_has_the_same_address_every_time_it_is_asked() {
    let once = mark_of(&BURNER, "draught");
    for _ in 0..100 {
        assert_eq!(mark_of(&BURNER, "draught"), once, "the address of a card moved between askings");
    }
}

#[test]
fn no_two_parts_of_a_board_share_an_address() {
    let mut seen: Vec<(String, Marks)> = Vec::new();
    for name in [SYSTEM, SETTINGS, OTHER].iter().chain(BURNER.iter()) {
        let mark = mark_of(&BURNER, name);
        if let Some((other, _)) = seen.iter().find(|(_, m)| *m == mark) {
            panic!("'{name}' and '{other}' share one address");
        }
        seen.push((name.to_string(), mark));
    }
    assert_eq!(seen.len(), 9);
}

#[test]
fn the_parts_every_board_has_keep_their_addresses_when_a_device_gains_a_card() {
    // the kept places come first on purpose: a device's list grows at its end, and the addresses of what every
    // board has must not walk with it - a stream opened before a change and read after it would swap two cards
    let grown = ["relays", "temperatures", "flue", "fan", "draught", "flame", "boiler"];
    for name in [SYSTEM, SETTINGS, OTHER] {
        assert_eq!(mark_of(&BURNER, name), mark_of(&grown, name), "the address of '{name}' moved");
    }
    for name in BURNER {
        assert_eq!(mark_of(&BURNER, name), mark_of(&grown, name), "the address of '{name}' moved");
    }
}

#[test]
fn a_name_the_board_does_not_carry_addresses_everything_rather_than_something() {
    // there is no right answer for a card nobody declared, and "the first card" would be a wrong one that works
    assert_eq!(mark_of(&BURNER, "boiler"), Marks::ALL);
    assert_eq!(mark_of(&BURNER, ""), Marks::ALL);
    assert_eq!(mark_of(&[], "draught"), Marks::ALL);
}

#[test]
fn a_board_with_more_cards_than_a_set_has_places_addresses_everything() {
    // thirty-two places, three of them kept: the thirtieth card of a device is the last that fits, and past it
    // the honest answer is the whole of the state rather than a bit that means another card
    let many: &[&'static str] = Vec::leak((0..40).map(|n| &*format!("card{n}").leak()).collect::<Vec<_>>());
    assert_ne!(mark_of(many, "card28"), Marks::ALL, "the twenty-ninth card of a device did not fit");
    assert_eq!(mark_of(many, "card29"), Marks::ALL, "a card past the places there are took one of them");
    assert_eq!(mark_of(many, "card39"), Marks::ALL);
}

#[test]
fn a_set_of_one_card_holds_that_card_and_refuses_the_others() {
    let wanted = Wanted::of(&BURNER, &["flue"]);
    assert!(!wanted.everything());
    assert!(wanted.has("flue"));
    for other in [SYSTEM, SETTINGS, OTHER, "draught", "fan", "relays", "temperatures", "flame"] {
        assert!(!wanted.has(other), "a set of the flue alone holds '{other}' as well");
    }
}

#[test]
fn a_set_of_no_part_at_all_holds_nothing() {
    let wanted = Wanted::new(&BURNER, Marks::NONE);
    for name in [SYSTEM, SETTINGS, OTHER, "draught", "flue"] {
        assert!(!wanted.has(name), "an empty set holds '{name}'");
    }
    assert!(!wanted.everything());
}

#[test]
fn the_whole_of_the_state_holds_every_part_and_says_it_is_the_whole() {
    let wanted = Wanted::ALL;
    assert!(wanted.everything());
    for name in [SYSTEM, SETTINGS, OTHER, "draught", "boiler", "no_such_card"] {
        assert!(wanted.has(name), "the whole of the state left out '{name}'");
    }
}
