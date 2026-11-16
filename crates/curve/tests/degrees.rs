//! A temperature in a ring of `u16`: tenths of a degree lifted by 100 °C, so that a reading below zero is still a number. Expected from the lift - -100.0 °C is 0, 0.0 °C
//! is 1000, 1000.0 °C is 11000 - and from what a person reads back off a curve, not from the code that makes the points.

use curve::degrees::{celsius, point};

#[test]
fn the_bottom_the_freezing_point_and_a_boiling_kettle_stand_where_the_lift_puts_them() {
    assert_eq!(point(-100.0), 0);
    assert_eq!(point(0.0), 1000);
    assert_eq!(point(100.0), 2000);
    assert_eq!(point(1000.0), 11000);
}

#[test]
fn a_tenth_of_a_degree_is_one_step_and_a_reading_below_zero_is_a_number() {
    assert_eq!(point(20.7), 1207);
    assert_eq!(point(-23.5), 765);
}

#[test]
fn a_point_reads_back_as_the_degrees_it_was() {
    for degrees in [-23.5_f32, 0.0, 20.7, 61.5, 148.5, 250.0] {
        assert!((celsius(point(degrees)) - degrees).abs() < 0.051, "{degrees}");
    }
}

#[test]
fn a_reading_past_either_end_of_the_ring_stops_at_the_end_and_does_not_wrap() {
    assert_eq!(point(-500.0), 0);
    assert_eq!(point(100_000.0), u16::MAX);
}
