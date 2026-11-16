//! What the flame's module hands the stream for a reading of its pin, by code and number, and for the order of its thresholds. Expected from the
//! circuit and the thresholds - the first burner's board says the same for these millivolts - not from the code that makes them.

use burner_api::flame::{put_values, order_value, Reading};
use flame::Thresholds;
use page::dictionary::Value;

const T: Thresholds = Thresholds { lit_mv: 2400, steady_mv: 1100 };

fn said(reading: Option<Reading>, thresholds: Thresholds) -> Vec<(&'static str, Value)> {
    let mut out = Vec::new();
    put_values(reading, thresholds, &mut |name, value| out.push((name, value)));
    out
}

fn window(mv: u16) -> Reading {
    Reading { window: analog::window(&[mv; 20]).unwrap(), at_ceiling: false }
}

fn get(said: &[(&'static str, Value)], name: &str) -> Value {
    said.iter().find(|(n, _)| *n == name).map(|(_, v)| *v).unwrap_or_else(|| panic!("nothing said of {name}: {said:?}"))
}

#[test]
fn a_dark_pin_is_out_and_a_lit_one_weak_and_a_strong_one_steady() {
    assert_eq!(get(&said(Some(window(2800)), T), "flame"), Value::Word("out"));
    assert_eq!(get(&said(Some(window(1800)), T), "flame"), Value::Word("weak"));
    assert_eq!(get(&said(Some(window(900)), T), "flame"), Value::Word("steady"));
}

#[test]
fn the_pin_says_its_millivolts_its_spread_and_where_the_number_stands() {
    let s = said(Some(Reading { window: analog::window(&[3153; 20]).unwrap(), at_ceiling: true }), T);
    assert_eq!(get(&s, "pin"), Value::Number(3153.0));
    assert_eq!(get(&s, "spread"), Value::Number(0.0));
    assert_eq!(get(&s, "edge"), Value::Word("top"));
    assert_eq!(get(&s, "window"), Value::Word("over"));
    let inside = said(Some(window(1800)), T);
    assert_eq!((get(&inside, "edge"), get(&inside, "window")), (Value::Word("inside"), Value::Word("inside")));
}

#[test]
fn thresholds_in_the_wrong_order_are_a_fault_of_the_setting_and_no_state_of_the_fire() {
    let crossed = Thresholds { lit_mv: 1100, steady_mv: 2400 };
    assert_eq!(get(&said(Some(window(900)), crossed), "flame"), Value::Unknown("crossed"));
    assert_eq!(order_value(crossed), Value::Word("crossed"));
    assert_eq!(order_value(T), Value::Word("in_order"));
}

#[test]
fn a_pin_that_has_read_nothing_says_not_read_and_leaves_its_parts_empty() {
    let s = said(None, T);
    assert_eq!(get(&s, "flame"), Value::Unknown("not_read"));
    assert_eq!(get(&s, "pin"), Value::Unknown("not_read"));
    for part in ["spread", "edge", "window"] {
        assert_eq!(get(&s, part), Value::None, "{part}");
    }
}
