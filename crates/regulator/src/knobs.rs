//! What turning each knob of the regulator does, as numbers: the arithmetic a person's line under a knob is made of. The page words these numbers and works out none of them.

use crate::{Holding, Tuning};

/// The size of a miss, in pascals, the lines of the gains are said for: a miss that grows by it, and a miss of it that stays.
pub const MISS_PA: f32 = 10.0;
/// The size of a miss, in pascals, the line of the band's share is said for.
pub const NEAR_PA: f32 = 3.0;
/// The jump in the tract the answer of the band is given for: the size of the one the igniter made going off on 26.09 (45 -> 68 Pa).
pub const JUMP_PA: f32 = 20.0;

/// What the knobs do on tuning T, at the point the regulator is asked about.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Consequences {
    /// How far a gain moves the gate, in percent of travel: the proportional ones at once for a miss growing by [`MISS_PA`], the integral ones in a second for a miss of [`MISS_PA`] that stays.
    pub up_kp: f32,
    pub up_ki: f32,
    pub down_kp: f32,
    pub down_ki: f32,
    /// How long the gate takes over the whole of its travel from the lowest to the highest, in seconds; None where the step is none.
    pub ramp_s: Option<f32>,
    /// The second from which the draught is back near its setpoint after a jump of [`JUMP_PA`], and how far under it the draught went; None - not back within the answer's time.
    pub back_s: Option<u32>,
    pub dip_pa: f32,
    /// What each way's integral gain comes to in a second inside the band, for a miss of [`NEAR_PA`], where only the share of it acts.
    pub near_up: f32,
    pub near_down: f32,
}

/// The consequences of T on the fan standing at PA pascals and ANGLE percent.
///
/// A gain is in thousandths of a percent of travel per percent of the miss (`Gains`), and a miss in pascals becomes percents of travel through the steepness of the fan where it stands ([`Holding::slope`]).
/// A point with no pressure, no angle or no number answers None: a fan that holds nothing has no answer to give, and a made-up one would be read as the board's word.
pub fn consequences(t: &Tuning, pa: f32, angle: f32) -> Option<Consequences> {
    if !(pa > 0.0 && angle > 0.0) {
        return None;
    }
    let steepness = Holding::slope(pa, angle);
    let moves = |gain_milli: f32, miss_pa: f32| gain_milli / 1000.0 * miss_pa / steepness;
    let jump = crate::answer_to_a_jump(t, pa, angle, JUMP_PA, &mut |_, _| {});
    Some(Consequences {
        up_kp: moves(t.up.kp_milli as f32, MISS_PA),
        up_ki: moves(t.up.ki_milli as f32, MISS_PA),
        down_kp: moves(t.down.kp_milli as f32, MISS_PA),
        down_ki: moves(t.down.ki_milli as f32, MISS_PA),
        ramp_s: (t.step > 0.0).then(|| (t.max - t.min) / t.step),
        back_s: jump.back_s,
        dip_pa: jump.dip_pa,
        near_up: moves(t.up.ki_milli as f32 * t.band_share, NEAR_PA),
        near_down: moves(t.down.ki_milli as f32 * t.band_share, NEAR_PA),
    })
}

/// The point the knobs are answered for: what THIS fan was last seen holding, or the fan like it until it has held anything - a point that stays put, so the words under the knobs move with the knobs and not with the breathing.
pub fn known(holding: Option<&Holding>) -> (f32, f32) {
    holding.and_then(|h| h.knows()).unwrap_or(crate::A_FAN_LIKE_THIS)
}
