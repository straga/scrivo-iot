//! The checks a door asks of a number, one by one: a number before an order, and never an order alone. Every record that has a door - the
//! fire's `Tuning`, the thermostat's - asks them here, so that a refusal says the same code whoever the field belongs to.

use crate::Refused;

/// A number of seconds or of degrees that has to BE a number, and has to be above nothing at all.
///
/// The finiteness is not a formality: **every "not less than zero" lets `NaN` through**, because a
/// comparison with it is false whichever way it is written, and an infinity passes the same test while
/// meaning "never". Both arrive from a card the same way an ordinary number does.
pub fn above_nought(field: &str, value: f32) -> Result<(), Refused> {
    a_number(field, value)?;
    if value <= 0.0 {
        return Err(Refused::of(field, "above_nought", value));
    }
    Ok(())
}

/// The same, where nought itself means something - and the field says what it means.
pub fn not_below_nought(field: &str, value: f32) -> Result<(), Refused> {
    a_number(field, value)?;
    if value < 0.0 {
        return Err(Refused::of(field, "not_below_nought", value));
    }
    Ok(())
}

pub fn a_number(field: &str, value: f32) -> Result<(), Refused> {
    if value.is_nan() {
        return Err(Refused::of(field, "not_a_number", value));
    }
    if value.is_infinite() {
        return Err(Refused::of(field, "infinite", value));
    }
    Ok(())
}

/// One number of the record standing above another: the only kind of upper limit this crate has, and it is
/// about the ORDER of two numbers rather than about how large either may be.
pub fn above(field: &str, value: f32, than: &'static str, floor: f32) -> Result<(), Refused> {
    if value <= floor {
        return Err(Refused::against(field, "above", value, than, floor));
    }
    Ok(())
}

pub fn not_below(field: &str, value: f32, than: &'static str, floor: f32) -> Result<(), Refused> {
    if value < floor {
        return Err(Refused::against(field, "not_below", value, than, floor));
    }
    Ok(())
}

pub fn below(field: &str, value: f32, than: &'static str, ceiling: f32) -> Result<(), Refused> {
    if value >= ceiling {
        return Err(Refused::against(field, "below", value, than, ceiling));
    }
    Ok(())
}

