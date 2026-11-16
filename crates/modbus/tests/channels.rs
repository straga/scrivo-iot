//! The map of readings: what a person sees described as data, and the askings built from that description. The
//! numbers are the live ones of the burner's bus (scrivo-iot/runs/burner/2.1/bus_probe.hex): the NT18B07 gives seven
//! channels in tenths of a degree, an empty socket among them says 0xF555.

use modbus::{asks, reading, Ask, Channel, Reading, Scale, MAX_VALUES};

/// The seven channels of the NT18B07 as the burner describes them - the description, and nothing else, is what
/// adding a reading touches.
fn water() -> Vec<Channel> {
    (0..7)
        .map(|n| Channel {
            name: ["water", "return", "feeder", "case", "spare 5", "spare 6", "spare 7"][n],
            addr: 2,
            reg: n as u16,
            unit: "°C",
            scale: Scale::Tenths,
            empty: Some(0xF555),
        })
        .collect()
}

fn built(channels: &[Channel]) -> Vec<Ask> {
    let mut out = [Ask::read(0, 0, 0); 8];
    let n = asks(channels, &mut out);
    out[..n].to_vec()
}

#[test]
fn seven_channels_of_one_module_are_one_asking() {
    // 37 ms on the wire instead of seven askings: the whole point of describing them together
    assert_eq!(built(&water()), [Ask::read(2, 0x0000, 7)]);
}

#[test]
fn channels_apart_are_still_one_asking_while_they_fit() {
    // only the first and the last register described: the run between them is read in one go
    let ends: Vec<Channel> = water().into_iter().filter(|c| c.reg == 0 || c.reg == 6).collect();
    assert_eq!(built(&ends), [Ask::read(2, 0x0000, 7)]);
}

#[test]
fn a_channel_on_another_module_gets_an_asking_of_its_own() {
    // This is about the mechanism only: a longer list is covered by the askings built from it. That adding a
    // reading to the BURNER is a line and nothing else is a promise about the burner's own map, and it is proved
    // where that map lives - burner_map, an_eighth_reading_is_a_line_and_touches_no_size.
    let before = built(&water());
    let mut with_flue = water();
    with_flue.push(Channel {
        name: "flue",
        addr: 1,
        reg: 0x0000,
        unit: "°C",
        scale: Scale::Tenths,
        empty: None,
    });
    let after = built(&with_flue);
    assert_eq!(after.len(), before.len() + 1, "the new module needs an asking of its own");
    assert_eq!(after[0], before[0], "and the asking of the old one is untouched");
    assert!(after.contains(&Ask::read(1, 0x0000, 1)));
}

#[test]
fn registers_further_apart_than_one_asking_holds_are_two_askings() {
    let far = [
        Channel { name: "first", addr: 2, reg: 0x0000, unit: "°C", scale: Scale::Tenths, empty: None },
        Channel { name: "far", addr: 2, reg: 0x0020, unit: "°C", scale: Scale::Tenths, empty: None },
    ];
    let built = built(&far);
    assert_eq!(built.len(), 2, "{built:?}");
    assert!(built.iter().all(|a| a.count as usize <= MAX_VALUES));
}

#[test]
fn a_channel_reads_its_own_register_out_of_the_answer() {
    // live: 02 03 0E 00 D3 00 D5 00 D5 00 D4 F5 55 F5 55 F5 55 - four sensors at 21 C and three empty sockets
    let values = [0x00D3, 0x00D5, 0x00D5, 0x00D4, 0xF555, 0xF555, 0xF555];
    let ask = Ask::read(2, 0x0000, 7);
    let channels = water();
    assert_eq!(reading(&channels[0], &ask, &values), Reading::Is(21.1));
    assert_eq!(reading(&channels[3], &ask, &values), Reading::Is(21.2));
}

#[test]
fn an_empty_socket_is_empty_and_not_minus_two_hundred_and_seventy_three() {
    // 0xF555 is what the module says for a socket with nothing in it. Shown as a number it reads -273.1 and looks
    // like a broken sensor to a person; it is not a reading at all.
    let values = [0x00D3, 0x00D5, 0x00D5, 0x00D4, 0xF555, 0xF555, 0xF555];
    let ask = Ask::read(2, 0x0000, 7);
    assert_eq!(reading(&water()[4], &ask, &values), Reading::Empty);
}

#[test]
fn a_value_below_zero_keeps_its_sign() {
    // live from the PTA9B01: FF15 is -23.5, not 65301
    let flue = Channel { name: "flue", addr: 1, reg: 0, unit: "°C", scale: Scale::Tenths, empty: None };
    assert_eq!(reading(&flue, &Ask::read(1, 0x0000, 2), &[0xFF15, 0x0389]), Reading::Is(-23.5));
}

#[test]
fn a_whole_number_is_not_divided() {
    let plain = Channel { name: "address", addr: 2, reg: 0x00FE, unit: "", scale: Scale::Whole, empty: None };
    assert_eq!(reading(&plain, &Ask::read(2, 0x00FE, 1), &[2]), Reading::Is(2.0));
}

#[test]
fn a_channel_the_asking_does_not_cover_is_unknown() {
    // the neighbour's asking says nothing about our channel, and saying nothing is not saying zero
    let channels = water();
    assert_eq!(reading(&channels[0], &Ask::read(1, 0x0000, 2), &[0x0EBC, 0x0990]), Reading::Unknown);
    assert_eq!(reading(&channels[6], &Ask::read(2, 0x0000, 3), &[0x00D3, 0x00D5, 0x00D5]), Reading::Unknown);
}

#[test]
fn a_short_answer_leaves_the_channels_behind_it_unknown() {
    // the module answered fewer registers than the asking covers: what is missing is unknown, not zero
    let ask = Ask::read(2, 0x0000, 7);
    assert_eq!(reading(&water()[5], &ask, &[0x00D3, 0x00D5]), Reading::Unknown);
}
