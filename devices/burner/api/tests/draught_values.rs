//! What the draught's module hands the stream for a window of its pin: the pressure in pascals, or why there is none - under its own zero, over its scale, nothing on the pin, a divider that is
//! no divider, nothing read yet -, the pin under it, and where the sensor's own zero lands through the divider. Expected from the circuit and the sensor (0.5 V is no pressure, 4.5 V is 300 Pa,
//! the pin sees a share of the sensor that the two arms make) and from the first burner's board, which says the same of the same millivolts - not from the code that makes them.

use burner_api::draught::{at, put_values, zero_at};
use burner_api::pin::Reading;
use page::dictionary::Value;

/// The divider of the defaults: a half.
const HALF: (u32, u32) = (10_000, 10_000);

fn window(mv: u16) -> Option<Reading> {
    Some(Reading { window: analog::window(&[mv; 20]).unwrap(), at_ceiling: false })
}

fn said(reading: Option<Reading>, divider: (u32, u32)) -> Vec<(usize, Value)> {
    let mut out = Vec::new();
    put_values(reading, divider, &mut |at, value| out.push((at, value)));
    out
}

fn get(said: &[(usize, Value)], at: usize) -> Value {
    said.iter().find(|(a, _)| *a == at).map(|(_, v)| *v).unwrap_or_else(|| panic!("nothing said at {at}: {said:?}"))
}

fn pressure(reading: Option<Reading>, divider: (u32, u32)) -> Value {
    get(&said(reading, divider), at::BLAST_PRESSURE)
}

#[test]
fn a_pin_that_sees_a_share_of_the_sensor_reads_the_pascals_the_sensor_put_out() {
    // 1000 mV on the pin through a half is 2000 mV out of the sensor: 1.5 V over its zero, 112.5 Pa of 300
    let Value::Number(pa) = pressure(window(1000), HALF) else { panic!("no number for 1000 mV") };
    assert!((pa - 112.5).abs() < 0.2, "{pa}");
    // the same pin through another divider is another pressure: the arms are what the number is made through
    let Value::Number(more) = pressure(window(1000), (10_000, 30_000)) else { panic!("no number") };
    assert!(more < pa, "a bigger share on the pin must mean less pressure: {more} against {pa}");
}

#[test]
fn a_pin_a_little_over_the_sensors_zero_is_a_still_pressure_and_one_under_it_is_its_own_zero_not_a_number() {
    // 161 mV on the pin - what the owner measured on the stopped fan (21.09) - is 322 mV out of the sensor: alive, under its 500 mV zero
    assert_eq!(pressure(window(161), HALF), Value::Unknown("own_zero"));
    // 250 mV on the pin is the sensor's own zero exactly: no pressure
    let Value::Number(zero) = pressure(window(250), HALF) else { panic!("no number at the zero") };
    assert!(zero.abs() < 0.2, "{zero}");
}

#[test]
fn a_pin_past_the_top_of_the_scale_is_at_least_the_top_and_never_a_clamped_number() {
    assert_eq!(pressure(window(2400), HALF), Value::AtLeast(300.0));
}

#[test]
fn a_sensor_that_puts_out_nothing_is_missing_and_a_dead_wire_is_not_a_pressure_of_zero() {
    assert_eq!(pressure(window(80), HALF), Value::Unknown("missing"));
    assert_eq!(pressure(window(0), HALF), Value::Unknown("missing"));
}

#[test]
fn a_pair_of_arms_that_is_no_divider_is_unconfigured_and_not_a_sensor_that_fell_off() {
    // nothing at the bottom is a setting nobody filled in; an arm past a megaohm is past what the board's own input leaves of a divider
    assert_eq!(pressure(window(1000), (10_000, 0)), Value::Unknown("unconfigured"));
    assert_eq!(pressure(window(1000), (2_000_000, 10_000)), Value::Unknown("unconfigured"));
    assert_eq!(pressure(window(1000), (10_000, 2_000_000)), Value::Unknown("unconfigured"));
}

#[test]
fn a_pin_that_has_read_nothing_says_not_read_and_leaves_its_parts_empty() {
    let s = said(None, HALF);
    assert_eq!(get(&s, at::BLAST_PRESSURE), Value::Unknown("not_read"));
    assert_eq!(get(&s, at::PIN), Value::Unknown("not_read"));
    for part in [at::SPREAD, at::EDGE, at::WINDOW] {
        assert_eq!(get(&s, part), Value::None);
    }
}

#[test]
fn the_pin_says_its_millivolts_its_spread_and_where_the_number_stands() {
    let s = said(window(1450), HALF);
    assert_eq!(get(&s, at::PIN), Value::Number(1450.0));
    assert_eq!(get(&s, at::SPREAD), Value::Number(0.0));
    assert_eq!((get(&s, at::EDGE), get(&s, at::WINDOW)), (Value::Word("inside"), Value::Word("inside")));
    let low = said(window(100), HALF);
    assert_eq!(get(&low, at::WINDOW), Value::Word("under"));
}

#[test]
fn the_zero_of_the_sensor_lands_on_the_pin_at_the_share_the_arms_make() {
    assert_eq!(zero_at(HALF), Value::Number(250.0));
    assert_eq!(zero_at((10_000, 30_000)), Value::Number(375.0));
    assert_eq!(zero_at((10_000, 0)), Value::Unknown("not_a_divider"));
}
