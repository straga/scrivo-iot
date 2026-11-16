//! A temperature in a ring of `u16`: tenths of a degree lifted by 100 °C, so that a reading below zero is still a number. -100.0 °C is 0, 0.0 °C is 1000, 1000.0 °C
//! is 11000. What the water's rings and the flue's ring keep, and what a curve's points are turned back into.

/// How far the lift stands: 100 °C in tenths.
pub const BELOW_ZERO: i32 = 1_000;

/// The point of a ring for CELSIUS degrees; a reading past either end of the ring stops at that end.
pub fn point(celsius: f32) -> u16 {
    ((celsius * 10.0) as i32 + BELOW_ZERO).clamp(0, i32::from(u16::MAX)) as u16
}

/// What a point of a ring is in degrees again.
pub fn celsius(point: u16) -> f32 {
    (i32::from(point) - BELOW_ZERO) as f32 / 10.0
}
