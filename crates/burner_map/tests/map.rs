//! The map of the burner's readings, and the one thing the whole of this file is for: that adding a reading
//! is a line in the map and nothing else.
//!
//! The test that matters here is `an_eighth_reading_is_a_line_and_touches_no_size`. It walks the burner's OWN map
//! with one line added and takes it the whole way - channels, askings, the array of what was read, the rows a
//! person sees - calling exactly the same functions with exactly the same arguments. Anything written as 7 rather
//! than counted from the map stops it: a length that does not fit fails to compile, and a loop bound that does not
//! follow drops the eighth row.

use burner_map::{addresses, asks, channels, rows, Spot, COUNT, MAP, MAX_ASKS, UNREAD};
use modbus::{Ask, Channel, Reading, Scale};

/// Where each module answers, as the board would say it: the NT18B07 on 2, the PTA9B01 on 1.
fn address(setting: &settings::Setting) -> u8 {
    match setting.name {
        "ntc_address" => 2,
        _ => 1,
    }
}

/// The burner's own map with the flue gas of the PTA9B01 added - the line that goes in the day its probe arrives.
const WITH_FLUE: [Spot; MAP.len() + 1] = {
    let mut with = [MAP[0]; MAP.len() + 1];
    let mut n = 0;
    while n < MAP.len() {
        with[n] = MAP[n];
        n += 1;
    }
    with[MAP.len()] = Spot {
        name: "flue",
        address: &settings::TIME, // stands for the flue module's own address setting, which arrives with it
        role: &burner_settings::ROLE_NTC_7, // and for its own role setting, likewise
        reg: 0x0000,
        unit: "°C",
        scale: Scale::Tenths,
        empty: None,
    };
    with
};

#[test]
fn every_size_is_counted_from_the_map() {
    assert_eq!(COUNT, MAP.len());
    assert_eq!(MAX_ASKS, MAP.len());
    assert_eq!(UNREAD.len(), MAP.len());
    assert_eq!(channels(&MAP, address).len(), MAP.len());
    // every socket named, so the card shows one row per channel and the count follows the map
    assert_eq!(rows(&MAP, &UNREAD, |_| "room".to_string()).len(), MAP.len());
}

#[test]
fn an_eighth_reading_is_a_line_and_touches_no_size() {
    // the same calls as the burner makes, over a map one line longer - nothing here is told how long it is
    let channels = channels(&WITH_FLUE, address);
    assert_eq!(channels.len(), MAP.len() + 1);

    let mut built = [Ask::read(0, 0, 0); WITH_FLUE.len()];
    let made = asks(&channels, &mut built);
    assert_eq!(made, 2, "seven of one module in one asking, and the new module in its own: {built:?}");
    assert!(built[..made].contains(&Ask::read(2, 0x0000, 7)), "the old module's asking is untouched");
    assert!(built[..made].contains(&Ask::read(1, 0x0000, 1)), "the new one is asked for");

    let read = [Reading::Unknown; WITH_FLUE.len()];
    let rows = rows(&WITH_FLUE, &read, |_| "own:whatever it is".to_string());
    assert_eq!(rows.len(), MAP.len() + 1);
    assert_eq!(rows.last().map(|(name, _)| name.as_str()), Some("whatever it is"));

    // and the new module brings its own address setting, found without being told
    assert_eq!(addresses(&WITH_FLUE).len(), 2);
}

#[test]
fn the_map_of_the_burner_asks_one_module_once() {
    let channels = channels(&MAP, address);
    let mut built = [Ask::read(0, 0, 0); MAX_ASKS];
    assert_eq!(asks(&channels, &mut built), 1);
    assert_eq!(built[0], Ask::read(2, 0x0000, 7));
    assert_eq!(addresses(&MAP).len(), 1);
}

#[test]
fn an_empty_socket_reads_as_words_and_a_number_as_its_unit() {
    let mut read = UNREAD;
    read[0] = Reading::Is(21.1);
    read[4] = Reading::Empty;
    // named by socket number so the rows can be told apart whatever the roles are
    let rows = rows(&MAP, &read, |s| format!("own:{}", s.name));
    assert_eq!(rows[0], ("role_ntc_1".to_string(), "21.1 °C".to_string()));
    assert_eq!(rows[4], ("role_ntc_5".to_string(), "no sensor".to_string()));
    assert_eq!(rows[6], ("role_ntc_7".to_string(), "-".to_string()), "nothing read yet is not zero");
}

#[test]
fn the_address_of_a_reading_comes_from_its_setting_and_not_from_the_map() {
    // the module remembers where it answers; the map only says which setting asks that question
    let asked: Vec<u8> = channels(&MAP, address).iter().map(|c: &Channel| c.addr).collect();
    assert!(asked.iter().all(|a| *a == 2), "{asked:?}");
    let moved: Vec<u8> = channels(&MAP, |_| 9).iter().map(|c: &Channel| c.addr).collect();
    assert!(moved.iter().all(|a| *a == 9), "{moved:?}");
}

// --- what a channel measures, and what the card does with a socket nobody uses ---

/// Roles as nvs would give them back: four sockets named, three left free.
fn four_named(setting: &'static settings::Setting) -> String {
    match setting.name {
        "role_ntc_1" => "supply".into(),
        "role_ntc_2" => "return".into(),
        "role_ntc_3" => "own:buffer tank".into(),
        "role_ntc_4" => "room".into(),
        _ => burner_settings::FREE.into(),
    }
}

fn all_free(_: &'static settings::Setting) -> String {
    burner_settings::FREE.into()
}

fn read_four() -> [Reading; MAP.len()] {
    let mut read = UNREAD;
    for (n, value) in [21.1, 20.4, 55.0, 19.8].into_iter().enumerate() {
        read[n] = Reading::Is(value);
    }
    read[4] = Reading::Empty;
    read[5] = Reading::Empty;
    read[6] = Reading::Empty;
    read
}

#[test]
fn only_the_sockets_a_person_named_are_shown() {
    let rows = rows(&MAP, &read_four(), four_named);
    assert_eq!(rows.len(), 4, "three free sockets are not readings: {rows:?}");
    let names: Vec<&str> = rows.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["water leaving the boiler", "water coming back", "buffer tank", "the boiler room"]);
}

#[test]
fn a_name_of_ones_own_stands_where_the_role_would() {
    let rows = rows(&MAP, &read_four(), four_named);
    assert_eq!(rows[2], ("buffer tank".to_string(), "55.0 °C".to_string()));
}

#[test]
fn a_card_with_nothing_assigned_says_so_and_says_what_to_do() {
    let rows = rows(&MAP, &UNREAD, all_free);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "no sensors");
    assert!(rows[0].1.contains("settings"), "{:?} does not say where to go", rows[0].1);
}

#[test]
fn the_map_still_holds_every_socket_the_module_has() {
    // hiding is what the page does; the module has seven sockets whatever a person uses, and the asking reads them
    // all in one go
    assert_eq!(MAP.len(), 7);
    let channels = channels(&MAP, address);
    let mut built = [Ask::read(0, 0, 0); MAX_ASKS];
    assert_eq!(asks(&channels, &mut built), 1);
    assert_eq!(built[0], Ask::read(2, 0x0000, 7));
}

#[test]
fn every_socket_has_a_role_setting_of_its_own() {
    let named: std::collections::BTreeSet<&str> = burner_map::roles(&MAP).iter().map(|s| s.name).collect();
    assert_eq!(named.len(), MAP.len(), "two sockets share a role setting");
}

// --- a value by its role: the automaton asks for the water of the boiler, not for a socket ---

/// The board as it stands: socket 1 the supply, socket 3 the boiler.
fn boiler_and_supply(setting: &'static settings::Setting) -> String {
    match setting.name {
        "role_ntc_1" => burner_settings::SUPPLY.into(),
        "role_ntc_2" => "return".into(),
        "role_ntc_3" => burner_settings::BOILER.into(),
        "role_ntc_4" => "room".into(),
        _ => burner_settings::FREE.into(),
    }
}

#[test]
fn the_water_of_the_boiler_is_the_socket_set_to_boiler() {
    let found = burner_map::by_role(&MAP, &read_four(), boiler_and_supply, burner_settings::BOILER);
    assert_eq!(found.map(|(spot, reading)| (spot.name, reading)), Some((MAP[2].name, Reading::Is(55.0))));
}

#[test]
fn the_supply_is_not_taken_for_the_water_of_the_boiler() {
    // the board before the boiler was assigned: only the supply is there, and it is not the boiler
    let found = burner_map::by_role(&MAP, &read_four(), four_named, burner_settings::BOILER);
    assert!(found.is_none(), "the supply was taken for the boiler: {:?}", found.map(|(s, r)| (s.name, r)));
}

// the page draws a reading by its value, never by its words; the values stand row for row with the
// words, so the nth value is the nth row's.
#[test]
fn every_row_has_its_value_beside_it_in_the_same_order() {
    let read = {
        let mut read = read_four();
        read[1] = Reading::Unknown;
        read
    };
    let values = burner_map::values(&MAP, &read, four_named);
    assert_eq!(values.len(), rows(&MAP, &read, four_named).len(), "a value per row");
    assert_eq!(values, vec![Some((21.1, "°C")), None, Some((55.0, "°C")), Some((19.8, "°C"))]);
}

#[test]
fn a_card_with_nothing_assigned_has_no_value_to_draw() {
    // the pair: the one row of an empty card is words, not a reading
    assert_eq!(burner_map::values(&MAP, &UNREAD, all_free), vec![None]);
}
