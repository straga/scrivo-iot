//! What the regulator does with rows of numbers, and no board anywhere. Every case here is one the stand will
//! be asked to repeat: it converges, it does not swing, it stops at its
//! limits, it does not wind up, it starts from the angle the fan is already held at, and it refuses to hold a
//! draught it cannot read.
//!
//! **BOTH STANDS HERE ARE THE SAME FAN, and it is the measured one:** `pa = 0.186·angle^1.92`, the shape the
//! board read off the burner on 21.09. They differ in what else they model. `Stand` answers an angle with a
//! pressure and nothing more, which is what the arithmetic of the loop is judged against. `ColdFan` adds the
//! rotor: the angle a standing one breaks away at, the angle a cold turning one falls out at, and the tract's
//! own four seconds.
//!
//! **A STRAIGHT LINE WOULD BE ANOTHER FAN.** The regulator divides the error by the steepness of the curve, so a
//! `Stand` that were a straight line of 5.6 Pa a percent would give it a steepness its own stand does not have. A green
//! row there would mean only that the loop converges under a mismatch, which is not what anybody wanted to know.
//! `Stand` has the measured shape, and not one threshold in these tests had to move for it.
//!
//! A model this rough cannot say what the gains should BE - that is the stand's answer, read off the
//! convergence curve - and it cannot judge a fall away under the setpoint either, because `ColdFan` drops its
//! rotor below 12 % of travel while the board on 22.09 kept turning at 9 %. What they answer is that the
//! arithmetic converges at all, and that the guards hold whatever the model does.

use std::cell::RefCell;
use std::rc::Rc;

use draught::Reading;
use regulator::{Asked, Blind, Gains, Held, Holding, Tuning};

/// How long this stand takes to arrive, and where it tops out. The lag is the module header's old "10-15 s at
/// small angles", kept deliberately slower than the tract measured on the board (4 s): a loop that converges
/// against a lag twice too long converges against the real one too. The ceiling is the owner's - 80 % of
/// travel gave 279 Pa and the sensor was already pinned.
const LAG_S: f32 = 10.0;
const MOST_PA: f32 = 279.0;

/// A boiler that answers an angle with a pressure, one lag behind.
struct Stand {
    pa: f32,
}

impl Stand {
    fn new() -> Self {
        Self { pa: 0.0 }
    }

    /// A second of blowing at ANGLE, and what the sensor then says. The shape is the measured one
    /// (`STEEPNESS`, `CURVE`, beside `ColdFan`), because there is only one fan.
    fn second(&mut self, angle: f32) -> Reading {
        let wants = (STEEPNESS * angle.powf(CURVE)).clamp(0.0, MOST_PA);
        self.pa += (wants - self.pa) / LAG_S;
        match self.pa {
            pa if pa > draught::FULL_PA as f32 => Reading::Over,
            // a standing fan sits under the sensor's own zero, which is what the live board shows at rest
            pa if pa < 0.5 => Reading::Under,
            // the board rounds nothing here either: the reading carries the fraction, because a setpoint of
            // twenty pascals would have a regulator chasing a step of a whole one (crates/draught)
            pa => Reading::Is(pa),
        }
    }
}

/// Gains that hold THIS MODEL without swinging, and they are not a claim about the boiler - the stand picks the
/// real ones off the convergence curve.
///
/// They are derived rather than guessed, which is why they are written down here. An integral alone on a
/// first-order lag is a second-order loop: with the slope K and the lag T, the damping is
/// 1/(2·sqrt(K·Ki·T)), so a damping of 0.7 - little overshoot - wants Ki no larger than about
/// 1/(4·0.49·K·T) = 0.0091. Hence 9 thousandths. The first gains tried here were ninety, and the row went to
/// 104 Pa on the way to 51: ten times too hot, exactly as that arithmetic says.
fn tuning() -> Tuning {
    Tuning {
        up: Gains { kp_milli: 150, ki_milli: 45 },
        // letting go is gentler on purpose, so the row exercises the second set at all; on the boiler the two
        // are the owner's to pick apart
        down: Gains { kp_milli: 150, ki_milli: 30 },
        min: 8.0,
        max: 80.0,
        step: 5.0,
        // no band: these rows judge the arithmetic of the loop, and the band is judged in back_to_setpoint.rs
        band_pa: 0.0,
        band_share: 1.0,
    }
}

/// Runs SECONDS of holding WANT, and answers what the sensor said each second.
fn held(hold: &mut Holding, stand: &mut Stand, want: f32, seconds: usize) -> Vec<Reading> {
    let t = tuning();
    let mut said = Vec::new();
    let mut reading = stand.second(hold.angle());
    for _ in 0..seconds {
        let at = match hold.step(Asked::Hold(want), reading, 1.0, &t) {
            Held::At(angle) => angle,
            Held::Blind(why) => panic!("the regulator went blind on {why:?} while the stand was answering"),
            Held::Alone => panic!("the regulator stood aside while it was asked to hold a draught"),
        };
        reading = stand.second(at);
        said.push(reading);
    }
    said
}

/// The pascals of a reading, for a row where every reading is a number.
fn pa(said: Reading) -> f32 {
    match said {
        Reading::Is(pa) => pa,
        other => panic!("a row of numbers had {other:?} in it"),
    }
}

/// The spread of the last N of a row: the number the stage asks for by name.
fn spread(row: &[Reading], last: usize) -> f32 {
    let tail: Vec<f32> = row[row.len() - last..].iter().map(|r| pa(*r)).collect();
    tail.iter().cloned().fold(f32::MIN, f32::max) - tail.iter().cloned().fold(f32::MAX, f32::min)
}

#[test]
fn it_comes_to_the_setpoint_and_stays_there() {
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    let row = held(&mut hold, &mut stand, 51.0, 200);
    let last = pa(row[row.len() - 1]);
    assert!((last - 51.0).abs() < 2.0, "after {} s it holds {last} Pa of the 51 asked", row.len());
    assert!(spread(&row, 30) < 2.0, "the last thirty seconds spread {} Pa", spread(&row, 30));
}

#[test]
fn it_does_not_swing_when_the_setpoint_moves() {
    // Overshoot is what a person reads as "the regulator is fighting the boiler". Measured between two HOLDING
    // setpoints and not from rest: a start has the kick in it, and the kick is its own case below.
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    held(&mut hold, &mut stand, 51.0, 250);
    let row = held(&mut hold, &mut stand, 74.0, 250);
    let highest = row.iter().map(|r| pa(*r)).fold(f32::MIN, f32::max);
    assert!(highest < 74.0 + (74.0 - 51.0) * 0.25, "it went to {highest} Pa on the way from 51 to 74");
    let row = held(&mut hold, &mut stand, 51.0, 250);
    let lowest = row.iter().map(|r| pa(*r)).fold(f32::MAX, f32::min);
    assert!(lowest > 51.0 - (74.0 - 51.0) * 0.25, "it dropped to {lowest} Pa on the way back from 74 to 51");
}

#[test]
fn a_start_from_rest_does_not_pin_the_sensor_on_its_way_to_the_setpoint() {
    // THE KICK IS A JUMP BY DESIGN - a smooth ramp does not start this motor - so a start overshoots and the
    // question is only how far. Past the sensor's scale and the board is blind for as long as it takes to come
    // back, at the very moment a person is watching a burner light.
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    let row = held(&mut hold, &mut stand, 51.0, 250);
    let highest = row.iter().map(|r| pa(*r)).fold(f32::MIN, f32::max);
    println!("a start onto 51 Pa reached {highest} Pa");
    assert!(highest < draught::FULL_PA as f32, "a start onto 51 Pa pinned the sensor at {highest} Pa");
    assert!(highest < 51.0 * 2.0, "a start onto 51 Pa went to {highest} Pa, over twice the setpoint");
}

#[test]
fn it_goes_to_a_new_setpoint_and_the_time_is_a_number() {
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    held(&mut hold, &mut stand, 51.0, 200);
    let row = held(&mut hold, &mut stand, 74.0, 200);
    let arrived = row.iter().position(|r| (pa(*r) - 74.0).abs() < 2.0);
    assert!(arrived.is_some(), "it never arrived at 74 Pa: it ended at {} Pa", pa(row[row.len() - 1]));
    // THE TIME IS A NUMBER AND NOT A BOUND. The stage asks for the time of arrival to be named; what it should
    // BE is the owner's call off the curve, and a bound invented here would be a requirement nobody agreed.
    println!("51 Pa to 74 Pa took {} s on this model", arrived.unwrap() + 1);
}

#[test]
fn it_comes_down_to_a_lower_setpoint_too() {
    // THE OTHER DIRECTION IS ITS OWN CASE, by the stage: the response is not symmetric, and a regulator proved
    // only upwards is proved on half its work
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    held(&mut hold, &mut stand, 74.0, 250);
    let row = held(&mut hold, &mut stand, 51.0, 250);
    let last = pa(row[row.len() - 1]);
    assert!((last - 51.0).abs() < 2.0, "coming down it holds {last} Pa of the 51 asked");
    let lowest = row.iter().map(|r| pa(*r)).fold(f32::MAX, f32::min);
    assert!(lowest > 51.0 * 0.75, "it dropped to {lowest} Pa on the way down to 51");
}

#[test]
fn the_angle_never_leaves_the_limits_it_was_given() {
    let t = tuning();
    // a setpoint over what this fan can make at all - it tops out at MOST_PA - so the angle goes to the
    // ceiling and stops there
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    let mut reading = Reading::Is(0.0);
    // long enough to arrive: past the angle where the fan tops out the error is a small constant, so the
    // integral walks the last stretch to the ceiling at a tenth of a percent a second
    for _ in 0..900 {
        let at = match hold.step(Asked::Hold(290.0), reading, 1.0, &t) {
            Held::At(angle) => angle,
            Held::Blind(why) => panic!("blind on {why:?}"),
            Held::Alone => panic!("it stood aside where it was asked to act"),
        };
        assert!(at <= t.max + 0.001, "the angle went to {at} % above the ceiling of {} %", t.max);
        assert!(at >= t.min - 0.001, "the angle went to {at} % below the floor of {} %", t.min);
        reading = stand.second(at);
    }
    assert!((hold.angle() - t.max).abs() < 0.001, "it settled at {} % instead of the ceiling", hold.angle());
}

#[test]
fn the_integral_does_not_wind_up_while_the_angle_sits_on_a_limit() {
    // THE POINT OF THE CASE: an integral that kept growing against the ceiling would hold the fan at full for
    // as long as it took to unwind, and a person lowering the setpoint would watch nothing happen
    let t = tuning();
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    let mut reading = Reading::Is(0.0);
    for _ in 0..600 {
        let Held::At(at) = hold.step(Asked::Hold(290.0), reading, 1.0, &t) else { panic!("blind") };
        reading = stand.second(at);
    }
    // ten minutes against the ceiling; now ask for something it can hold and see how long it takes to let go
    let mut let_go = None;
    for second in 0..120 {
        let Held::At(at) = hold.step(Asked::Hold(51.0), reading, 1.0, &t) else { panic!("blind") };
        reading = stand.second(at);
        if at < t.max - 1.0 && let_go.is_none() {
            let_go = Some(second);
        }
    }
    assert_eq!(let_go, Some(0), "it stayed at the ceiling for {let_go:?} seconds after the setpoint dropped");
}

#[test]
fn it_starts_from_the_angle_the_fan_is_already_held_at() {
    // ON A LIVE FIRE THIS IS THE WHOLE POINT: switched on from zero, the fan would drop to nothing for a moment
    // and the flame would be caught or smothered
    let mut hold = Holding::from(37.0);
    assert_eq!(hold.angle(), 37.0);
    let t = tuning();
    let Held::At(at) = hold.step(Asked::Hold(51.0), Reading::Is(50.0), 1.0, &t) else { panic!("blind") };
    assert!((at - 37.0).abs() <= t.step, "the first step moved the angle from 37 to {at} %");
}

#[test]
fn one_step_never_moves_the_angle_further_than_it_is_allowed() {
    let t = tuning();
    let mut hold = Holding::from(40.0);
    // an error of two hundred pascals, which without a rate limit would throw the angle across its whole range
    let Held::At(at) = hold.step(Asked::Hold(250.0), Reading::Is(50.0), 1.0, &t) else { panic!("blind") };
    assert!((at - 40.0).abs() <= t.step + 0.001, "one step moved the angle from 40 to {at} %");
}

// ---- what it does when the draught is not a number ----

#[test]
fn a_reading_that_is_not_a_pressure_stops_the_fan_once_it_had_been_turning() {
    let t = tuning();
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    held(&mut hold, &mut stand, 51.0, 120);
    assert!(hold.turning(), "the stand answered pressures and it still thinks the fan stands still");
    for (said, why) in [
        (Reading::Under, Blind::Under),
        (Reading::Missing, Blind::Missing),
        (Reading::Unconfigured, Blind::Unconfigured),
    ] {
        let mut hold = Holding::from(40.0);
        held(&mut hold, &mut Stand { pa: 51.0 }, 51.0, 3);
        match hold.step(Asked::Hold(51.0), said, 1.0, &t) {
            Held::Blind(got) => assert_eq!(got, why, "{said:?} was answered as {got:?}"),
            Held::Alone => panic!("it stood aside where it was asked to act"),
            Held::At(at) => panic!("{said:?} held the fan at {at} % blind"),
        }
    }
}

#[test]
fn a_reading_that_is_not_a_pressure_is_no_fault_while_the_fan_is_still_standing() {
    // AND WITHOUT THIS THE FAN NEVER STARTS: the sensor puts its own zero at 500 mV, so a standing fan reads
    // UNDER that - which is what the live board shows at rest. "No draught" is the state a start begins in.
    let t = tuning();
    let mut hold = Holding::from(0.0);
    assert!(!hold.turning());
    match hold.step(Asked::Hold(51.0), Reading::Under, 1.0, &t) {
        Held::At(at) => assert!(at >= dimmer::STARTS_AT as f32, "a standing fan was asked for {at} %, under what it can start on"),
        Held::Blind(why) => panic!("a standing fan asked to start answered {why:?}"),
        Held::Alone => panic!("it stood aside where it was asked to act"),
    }
}

#[test]
fn a_fan_that_will_not_start_says_so_instead_of_kicking_for_ever() {
    let t = tuning();
    let mut hold = Holding::from(0.0);
    let mut said = None;
    for _ in 0..60 {
        match hold.step(Asked::Hold(51.0), Reading::Under, 1.0, &t) {
            Held::At(_) => continue,
            Held::Blind(why) => {
                said = Some(why);
                break;
            }
            Held::Alone => panic!("it stood aside where it was asked to act"),
        }
    }
    assert_eq!(said, Some(Blind::NeverStarted), "a fan that never made a draught was kicked for a minute");
}

#[test]
fn a_kick_that_worked_lets_go_down_to_what_holds_the_setpoint() {
    // THE CASE THE STAGE ASKS FOR BY NAME: from nothing, onto a setpoint whose holding angle is UNDER what the
    // fan can start on. 51 Pa is the first ignition's setpoint and needs about nine percent on this model -
    // well under the twenty-five it breaks away at. So it must be kicked, and then let go.
    let (mut hold, mut stand) = (Holding::from(0.0), Stand::new());
    let row = held(&mut hold, &mut stand, 51.0, 250);
    let last = pa(row[row.len() - 1]);
    assert!((last - 51.0).abs() < 2.0, "it holds {last} Pa of the 51 asked");
    assert!(
        hold.angle() < dimmer::STARTS_AT as f32,
        "a setpoint under what the fan starts on left it at {} %, so it never let go of the kick",
        hold.angle()
    );
    assert!(hold.angle() >= 8.0, "it let go to {} %, under the floor a turning fan holds on", hold.angle());
}

#[test]
fn above_the_scale_is_read_as_too_much_air_and_not_as_a_fault() {
    // the purge runs at full and the sensor ends at 300 Pa, so "over" is a normal reading of a working board;
    // and while holding it says one thing for certain - that we are above the setpoint
    let t = tuning();
    let mut hold = Holding::from(70.0);
    hold.step(Asked::Hold(51.0), Reading::Is(60.0), 1.0, &t);
    let before = hold.angle();
    let Held::At(at) = hold.step(Asked::Hold(51.0), Reading::Over, 1.0, &t) else {
        panic!("a reading over the scale was answered as a fault")
    };
    assert!(at < before, "over the scale, the angle went from {before} to {at} % instead of down");
}

// ---- the three modes ----

#[test]
fn a_purge_is_full_air_whatever_the_sensor_says() {
    let t = tuning();
    let mut hold = Holding::from(20.0);
    for said in [Reading::Over, Reading::Under, Reading::Missing, Reading::Is(10.0)] {
        match hold.step(Asked::Purge, said, 1.0, &t) {
            Held::At(at) => assert_eq!(at, 100.0, "a purge with {said:?} asked for {at} %"),
            Held::Blind(why) => panic!("a purge with {said:?} answered {why:?}"),
            Held::Alone => panic!("it stood aside where it was asked to act"),
        }
    }
}

#[test]
fn a_stop_is_zero_and_not_a_very_late_firing() {
    let t = tuning();
    let mut hold = Holding::from(60.0);
    for said in [Reading::Is(200.0), Reading::Missing] {
        match hold.step(Asked::Stop, said, 1.0, &t) {
            Held::At(at) => assert_eq!(at, 0.0, "a stop with {said:?} asked for {at} %"),
            Held::Blind(why) => panic!("a stop with {said:?} answered {why:?}"),
            Held::Alone => panic!("it stood aside where it was asked to act"),
        }
    }
}

#[test]
fn leaving_a_purge_does_not_carry_its_error_into_the_holding() {
    // the purge runs at full with no feedback; an error remembered from before it would throw the first step
    // of the holding across the rate limit
    let t = tuning();
    let (mut hold, mut stand) = (Holding::from(0.0), Stand { pa: 279.0 });
    hold.step(Asked::Purge, Reading::Over, 1.0, &t);
    assert_eq!(hold.angle(), 100.0, "a purge did not hold the fan at full");
    let Held::At(first) = hold.step(Asked::Hold(51.0), stand.second(100.0), 1.0, &t) else { panic!("blind") };
    // THE FIRST STEP LANDS ON THE CEILING, and that is the limit rather than the rate: the rate limits what the
    // ERROR may move, and coming back inside the range it may hold is not a move the error asked for. What the
    // case is against is a first step that carries a remembered error and throws the angle across the range.
    assert_eq!(first, t.max, "the first step out of a purge went from 100 to {first} %");
    let Held::At(second) = hold.step(Asked::Hold(51.0), stand.second(first), 1.0, &t) else { panic!("blind") };
    assert!((second - first).abs() <= t.step + 0.001, "the step after it moved from {first} to {second} %");
}

#[test]
fn a_setpoint_of_no_pascals_stops_the_fan_rather_than_chasing_zero() {
    // zero pascals is not a pressure to hold: the fan off is the answer, and chasing it would leave the motor
    // humming at the floor of the angle
    let t = tuning();
    let mut hold = Holding::from(40.0);
    match hold.step(Asked::Hold(0.0), Reading::Is(30.0), 1.0, &t) {
        Held::At(at) => assert_eq!(at, 0.0, "a setpoint of nothing asked for {at} %"),
        Held::Blind(why) => panic!("a setpoint of nothing answered {why:?}"),
        Held::Alone => panic!("it stood aside where it was asked to act"),
    }
}

#[test]
fn what_it_says_when_it_cannot_read_the_draught_names_the_fault_and_what_to_do() {
    for why in [Blind::Under, Blind::Missing, Blind::Unconfigured, Blind::NeverStarted] {
        let words = regulator::words(why);
        assert!(words.len() > 20, "{why:?} says only {words:?}");
        assert!(!words.contains("Blind"), "{why:?} says the name of its own enum: {words:?}");
        assert!(words.chars().next().is_some_and(|c| c.is_lowercase()), "{words:?} starts with a capital");
    }
}

#[test]
fn a_kick_is_given_its_second_of_air_before_it_is_judged() {
    // MEASURED ON THE BOARD, AND IT WAS WRONG: with a step of one second the board condemned the fan after
    // three seconds, and of those the kick had only been blowing for two - the first step counted a failure at
    // the same moment it gave the kick. A fan is judged on what it did with the air it was given, so the air
    // comes first and the verdict on the step after.
    let t = tuning();
    let mut hold = Holding::from(0.0);
    let mut kicking = 0;
    for second in 1..=60 {
        match hold.step(Asked::Hold(51.0), Reading::Under, 1.0, &t) {
            Held::At(at) => {
                assert!(at >= dimmer::STARTS_AT as f32, "second {second} asked for {at} %, under what it starts on");
                kicking += 1;
            }
            Held::Alone => panic!("it stood aside where it was asked to act"),
            Held::Blind(why) => {
                assert_eq!(why, Blind::NeverStarted);
                // three kicks, each given a full second of air: at least three seconds of blowing before the
                // board says the fan is not turning
                assert!(
                    kicking >= regulator::KICKS as usize,
                    "the fan was condemned after only {kicking} s of kick air, and there are {} kicks",
                    regulator::KICKS
                );
                return;
            }
        }
    }
    panic!("a fan that made no draught was kicked for a minute without a verdict");
}

#[test]
fn by_hand_writes_nothing_at_all_rather_than_writing_a_zero() {
    // MEASURED ON THE BOARD 21.09, AND IT WAS A DEFECT OF MINE: a person moving the percent slider took the
    // regulator off by asking it to STOP, and stopping commands zero - once a second, which took the hand-set
    // angle away before the fan had spun up. The row read "gate off" while a person was asking for 33 %.
    let t = tuning();
    let mut hold = Holding::from(40.0);
    for said in [Reading::Is(60.0), Reading::Under, Reading::Missing] {
        assert_eq!(hold.step(Asked::ByHand, said, 1.0, &t), Held::Alone, "{said:?} was answered with a command");
    }
    // and the angle it remembers is untouched, so taking the regulator back is a start from where the fan is
    assert_eq!(hold.angle(), 40.0);
}

#[test]
fn a_stop_still_commands_a_zero_and_by_hand_still_does_not() {
    // THE PAIR, and without it the case above proves nothing: a regulator that wrote nothing ever would pass it
    let t = tuning();
    let mut hold = Holding::from(40.0);
    assert_eq!(hold.step(Asked::Stop, Reading::Is(60.0), 1.0, &t), Held::At(0.0));
    let mut hold = Holding::from(40.0);
    assert_eq!(hold.step(Asked::ByHand, Reading::Is(60.0), 1.0, &t), Held::Alone);
}

#[test]
fn taking_the_regulator_back_from_a_hand_starts_from_where_the_fan_is() {
    // it must not carry an error remembered from before the hand took over: the fan may be anywhere now
    let t = tuning();
    let mut hold = Holding::from(0.0);
    let mut stand = Stand::new();
    held(&mut hold, &mut stand, 51.0, 200);
    let at = hold.angle();
    hold.step(Asked::ByHand, Reading::Is(50.0), 1.0, &t);
    let Held::At(first) = hold.step(Asked::Hold(51.0), Reading::Is(50.0), 1.0, &t) else { panic!("blind") };
    assert!((first - at).abs() <= t.step + 0.001, "coming back from a hand moved the angle from {at} to {first} %");
}

#[test]
fn the_kick_outlasts_the_time_this_boiler_takes_to_make_a_draught() {
    // MEASURED ON THE BOARD, and it is the number the kick's window hangs on: held at 33 % from rest, the
    // draught read nothing at 2 s and 27 Pa at 5 s, so it crosses the five pascals that prove turning between
    // those two. A window that ends before the crossing condemns a fan that is spinning up.
    assert!(
        regulator::KICK_SECONDS >= 4.0,
        "a kick of {} s is inside the 2..5 s this boiler takes to make a draught at {} %",
        regulator::KICK_SECONDS,
        regulator::KICK_PERCENT
    );
    // and the kick is above what the fan breaks away on, or it is not a kick at all
    assert!(regulator::KICK_PERCENT > dimmer::STARTS_AT as f32);
}

#[test]
fn a_fan_that_starts_late_is_not_condemned_by_the_kick_before_it_does() {
    // the model made to answer the board: nothing for the first three seconds of the kick, and a draught after
    let t = tuning();
    let mut hold = Holding::from(0.0);
    for second in 1..=12 {
        let said = match second <= 3 {
            true => Reading::Under,
            false => Reading::Is(27.0),
        };
        match hold.step(Asked::Hold(51.0), said, 1.0, &t) {
            Held::At(_) => {}
            Held::Blind(why) => panic!("condemned at second {second} as {why:?}, and the draught came at 4 s"),
            Held::Alone => panic!("it stood aside where it was asked to act"),
        }
    }
    assert!(hold.turning(), "the draught came and it still thinks the fan stands still");
}

// ---- what became of the decision at the triac ----

/// A gate that answers as it is told to, and writes down every percent it was asked for.
fn gate_that(answers: Vec<Result<String, String>>) -> (impl FnMut(u32) -> Result<String, String>, Rc<RefCell<Vec<u32>>>) {
    let asked = Rc::new(RefCell::new(Vec::new()));
    let seen = asked.clone();
    let mut answers = answers.into_iter();
    let gate = move |percent: u32| {
        seen.borrow_mut().push(percent);
        answers.next().unwrap_or_else(|| Ok("the probe ran out of answers".to_string()))
    };
    (gate, asked)
}

#[test]
fn a_gate_that_takes_the_angle_is_not_asked_for_anything_else() {
    let (mut gate, asked) = gate_that(vec![Ok("42% of air".into())]);
    assert_eq!(regulator::carry(42, &mut gate), regulator::Carried::Out);
    assert_eq!(*asked.borrow(), vec![42], "the gate was asked for something beyond the angle");
}

#[test]
fn a_gate_that_refuses_the_angle_is_asked_to_stop_and_the_round_says_so() {
    // The pair of the test above, and the defect of 21.09: the refusal was written to the log and the angle
    // published as held.
    let (mut gate, asked) = gate_that(vec![Err("the comparator refused 1200 µs".into()), Ok("0%".into())]);
    let became = regulator::carry(42, &mut gate);
    assert_eq!(*asked.borrow(), vec![42, 0], "a refused angle was not followed by a stop");
    match became {
        regulator::Carried::Stopped { angle, ref why } => {
            assert_eq!(angle, 42);
            assert!(why.contains("comparator"), "the reason the gate gave was lost: {why}");
        }
        other => panic!("a refused angle came back as {other:?}"),
    }
}

#[test]
fn a_gate_that_refuses_even_the_stop_leaves_the_triac_unknown() {
    let (mut gate, asked) = gate_that(vec![
        Err("the comparator refused 1200 µs".into()),
        Err("the gate would not be forced free".into()),
    ]);
    let became = regulator::carry(42, &mut gate);
    assert_eq!(*asked.borrow(), vec![42, 0]);
    match became {
        regulator::Carried::Lost { angle, ref why, ref stop_why } => {
            assert_eq!(angle, 42);
            assert!(why.contains("comparator"), "{why}");
            assert!(stop_why.contains("forced free"), "{stop_why}");
        }
        other => panic!("a gate that refused everything came back as {other:?}"),
    }
}

#[test]
fn a_regulator_whose_command_did_not_get_through_forgets_what_it_had_climbed_to() {
    // THE WINDUP THAT NOBODY SEES. The angle is an intention, and while the gate refuses it the fan is standing
    // - but the pressure is missing, so a loop left running would keep climbing on an error it can do nothing
    // about. When the gate comes back it would then command an angle nobody chose, at a fan that is cold.
    let mut stand = Stand::new();
    let mut holding = Holding::from(0.0);
    let mut angle = 0.0;
    for _ in 0..120 {
        match holding.step(Asked::Hold(60.0), stand.second(angle), 1.0, &tuning()) {
            Held::At(at) => angle = at,
            other => panic!("the model should be holding by now, and it said {other:?}"),
        }
    }
    assert!(angle > 10.0, "the regulator never climbed anywhere: {angle}");

    // the gate refuses, the round says so, and the fan really is standing
    holding.not_carried_out();
    let first = holding.step(Asked::Hold(60.0), Reading::Under, 1.0, &tuning());
    assert_eq!(first, Held::At(regulator::KICK_PERCENT),
               "after a command that did not get through the regulator carried on from where it was: {first:?}");
}

#[test]
fn a_regulator_that_was_obeyed_keeps_what_it_had_climbed_to() {
    // The pair. Forgetting after every round would make the regulator useless - it would start from the floor a
    // second at a time and never hold anything.
    let mut stand = Stand::new();
    let mut holding = Holding::from(0.0);
    let mut angle = 0.0;
    for _ in 0..120 {
        match holding.step(Asked::Hold(60.0), stand.second(angle), 1.0, &tuning()) {
            Held::At(at) => angle = at,
            other => panic!("the model should be holding by now, and it said {other:?}"),
        }
    }
    let next = holding.step(Asked::Hold(60.0), stand.second(angle), 1.0, &tuning());
    match next {
        Held::At(at) => assert!((at - angle).abs() < 2.0,
                                "a regulator that was obeyed jumped from {angle} to {at}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_command_that_did_not_get_through_says_so_and_says_what_to_do() {
    // The row a person reads. Whatever the regulator decided, these two states must not read as holding.
    let stopped = regulator::Carried::Stopped { angle: 42, why: "the comparator refused 1200 µs".into() };
    let words = regulator::gate_words(&stopped).expect("a stopped fan says something");
    assert!(words.contains("fan stopped"), "{words}");
    assert!(words.contains("42"), "the angle that was refused is not in the words: {words}");
    assert!(words.contains("comparator"), "the reason is not in the words: {words}");

    let lost = regulator::Carried::Lost {
        angle: 42,
        why: "the comparator refused 1200 µs".into(),
        stop_why: "the gate would not be forced free".into(),
    };
    let words = regulator::gate_words(&lost).expect("a lost fan says something");
    assert!(words.contains("NOT UNDER CONTROL"), "{words}");
    // the next action, which is the whole point of the row: a person has to be able to act on it
    assert!(words.contains("cut the fan's power"), "the words do not say what to do: {words}");
}

#[test]
fn a_command_that_got_through_adds_nothing_to_the_row() {
    // The pair. If this said anything, every ordinary second would carry a gate's word and the row would stop
    // being about the draught.
    assert_eq!(regulator::gate_words(&regulator::Carried::Out), None);
}

// ---- a fan that has to be spun up, and the start onto a small setpoint ----

/// A fan with a rotor, which the plain `Stand` does not have.
///
/// **THE PART THAT MAKES A SMALL SETPOINT HARD.** A standing rotor needs a big angle to break away; once it is
/// TURNING it holds down to a much smaller one, and the angle it holds at drops further as the motor warms
/// (observed on 20.09 and again on 21.09). A model without that answers every angle with a pressure and cannot show
/// what the board does wrong - which is why the defect of 21.09 lived through twenty-five green tests.
struct ColdFan {
    pa: f32,
    turning: bool,
}

impl ColdFan {
    fn new() -> Self {
        Self { pa: 0.0, turning: false }
    }

    fn second(&mut self, angle: f32) -> Reading {
        // it breaks away only above the angle a standing rotor needs
        if !self.turning && angle >= STARTS_AT {
            self.turning = true;
        }
        // and it drops out the moment the angle goes under what a turning rotor holds at - a cold motor has no
        // momentum to live on, which is the whole of the owner's "when it is cold it just keeps hammering"
        if self.turning && angle < HOLDS_AT {
            self.turning = false;
        }
        let wants = match self.turning {
            true => (STEEPNESS * angle.powf(CURVE)).clamp(0.0, MOST_PA),
            false => 0.0,
        };
        // a first-order tract with the time constant the board was measured to have
        self.pa += (wants - self.pa) / TRACT_TAU_S;
        match self.pa {
            pa if pa > draught::FULL_PA as f32 => Reading::Over,
            pa if pa < 0.5 => Reading::Under,
            pa => Reading::Is(pa),
        }
    }
}

/// What this model's rotor needs to break away, and what it holds at once turning - the board's own numbers
/// (crates/dimmer STARTS_AT, HOLDS_AT), with the holding angle where a COLD motor was measured to lose it.
const STARTS_AT: f32 = 25.0;
const HOLDS_AT: f32 = 12.0;

/// **THE SHAPE OF THIS FAN, AND IT IS NOT A STRAIGHT LINE.** Measured on the board 21.09 with the gate held by
/// hand and the regulator left out of it: 20 % settled at 54 Pa and 35 % at 158 Pa. Two points three times
/// apart in pressure for less than twice the angle - fitting `pa = k·angle^n` through them gives n ≈ 1.92,
/// near enough a square, and k ≈ 0.186.
///
/// A straight line was what the model had, and it is why the model could not show a defect the board showed
/// plainly: with a slope taken low down, a third of the mains answers with a third of the pressure and nothing
/// ever runs away. On the real fan the kick's angle asks for nearly two hundred pascals, and THAT is what
/// overshoots while the angle is being taken off.
const STEEPNESS: f32 = 0.186;
const CURVE: f32 = 1.92;

/// How long the tract takes to answer, as a first-order time constant.
///
/// **FOUR SECONDS, MEASURED, NOT THE TEN TO FIFTEEN THE MODULE'S OWN HEADER SAYS IN WORDS.**
/// `tools/tract_lag_probe.py` on the bench board, 21.09: a step of the angle from 20 % to 35 % reached 63 % of
/// its change in 4 s and 95 % in 10 s; coming back down, 3 s and 9 s. The header's number was never measured,
/// and a descent built on it would be more than twice too slow.
const TRACT_TAU_S: f32 = 4.0;

#[test]
fn a_start_from_cold_onto_a_small_setpoint_arrives_instead_of_hammering() {
    // **THE DEFECT THE OWNER SAW ON 21.09.** Asked for thirty pascals from a standing fan, the board kicked,
    // saw the draught appear and dropped the angle to the floor in the same second - and the rotor, which had
    // been turning for one second, fell out. Measured on the board: the angle went 33 → 8 and the pressure went
    // 25 → 1, after which it crawled up a percent at a time. Warm it recovered; cold it kicked again and again.
    let mut stand = ColdFan::new();
    let mut holding = Holding::from(0.0);
    let mut angle = 0.0;
    let mut kicks = 0;
    let mut last = 0.0;
    let mut gave_up = 0;
    let mut highest = 0.0f32;
    for _ in 0..180 {
        let said = stand.second(angle);
        if let Reading::Is(pa) = said {
            highest = highest.max(pa);
        }
        match holding.step(Asked::Hold(30.0), said, 1.0, &tuning()) {
            Held::At(at) => {
                if at >= regulator::KICK_PERCENT && angle < regulator::KICK_PERCENT {
                    kicks += 1;
                }
                angle = at;
            }
            // the rotor fell out and the board stopped the fan: one round of the hammering the owner saw
            Held::Blind(_) => {
                gave_up += 1;
                angle = 0.0;
            }
            Held::Alone => panic!("nobody is holding this fan by hand"),
        }
        last = match said {
            Reading::Is(pa) => pa,
            _ => 0.0,
        };
    }
    println!("cold start: kicks {kicks}, gave up {gave_up}, highest {highest:.0} Pa, last {last:.0} Pa");

    // **THE CHECK FALLS ON EITHER SIDE, BECAUSE THE FIX HAS TWO WAYS TO BE WRONG.** Too fast a descent drops
    // the rotor and the board hammers; too slow a one feeds the fire air nobody asked for while the angle
    // comes off. A check that only knew one of them would pass a repair that swapped one for the other -
    // which is what happened twice this day.

    // ONE STALL IS THE PRICE OF MEETING THE FAN, and it is not a defect: until the rotor falls out once, the
    // board has no way of knowing which angle this motor holds at - the setting's floor is a person's guess.
    // What is a defect is the second one, because by then the board has been told.
    assert!(gave_up <= 1, "the board gave the fan up {gave_up} times: it is hammering, not learning");
    assert!(kicks <= 2, "the fan was kicked {kicks} times for one start");
    assert!((last - 30.0).abs() <= 6.0, "after three minutes it is at {last:.0} Pa, and 30 was asked");
    // AND IT DOES NOT BLOW THE FIRE OUT ON THE WAY. Measured on the board 21.09: with the descent ending at
    // the first touch of the setpoint, a start onto 30 Pa went to 87 Pa and stayed over it for a minute.
    assert!(highest <= 30.0 * 2.5, "a start onto 30 Pa reached {highest:.0} Pa on the way");
}

#[test]
fn on_quick_gains_with_a_band_a_cold_start_onto_a_small_setpoint_still_learns_its_floor_once() {
    // The quick gains of 3.10 must not undo 3.7: a start from cold onto 30 Pa meets one stall at most - the price of
    // meeting the fan - and arrives.
    // the quick gains of 3.10 with their band, written here: the row is about the floor, not about what a board holds
    let board = Tuning {
        up: Gains { kp_milli: 300, ki_milli: 400 },
        down: Gains { kp_milli: 300, ki_milli: 400 },
        band_pa: 4.0,
        band_share: 0.25,
        ..tuning()
    };
    let mut stand = ColdFan::new();
    let mut holding = Holding::from(0.0);
    let mut angle = 0.0;
    let (mut gave_up, mut last) = (0, 0.0);
    for _ in 0..180 {
        let said = stand.second(angle);
        match holding.step(Asked::Hold(30.0), said, 1.0, &board) {
            Held::At(at) => angle = at,
            Held::Blind(_) => {
                gave_up += 1;
                angle = 0.0;
            }
            Held::Alone => panic!("nobody is holding this fan by hand"),
        }
        last = match said {
            Reading::Is(pa) => pa,
            _ => 0.0,
        };
    }
    assert!(gave_up <= 1, "the board gave the fan up {gave_up} times on its own numbers: it is hammering");
    assert!((last - 30.0).abs() <= 6.0, "after three minutes it is at {last:.0} Pa, and 30 was asked");
}

#[test]
fn a_start_from_cold_onto_a_big_setpoint_still_arrives() {
    // The pair, and the case that worked all along: a big setpoint pulls the angle up fast enough that the
    // rotor never falls out. A fix for the small one must not spoil this.
    let mut stand = ColdFan::new();
    let mut holding = Holding::from(0.0);
    let mut angle = 0.0;
    let mut last = 0.0;
    for _ in 0..180 {
        let said = stand.second(angle);
        if let Held::At(at) = holding.step(Asked::Hold(120.0), said, 1.0, &tuning()) {
            angle = at;
        }
        if let Reading::Is(pa) = said {
            last = pa;
        }
    }
    assert!((last - 120.0).abs() <= 12.0, "after three minutes it is at {last:.0} Pa, and 120 was asked");
}

#[test]
fn a_floor_the_fan_never_needed_is_never_invented() {
    // The pair. A fan that holds everywhere the setting allows must be left with the setting's own floor -
    // a regulator that raises its floor without being shown a stall is one that quietly refuses to go low.
    let mut stand = Stand::new();
    let mut holding = Holding::from(0.0);
    let mut angle = 0.0;
    for _ in 0..200 {
        if let Held::At(at) = holding.step(Asked::Hold(51.0), stand.second(angle), 1.0, &tuning()) {
            angle = at;
        }
    }
    // now ask for almost nothing: this model never stalls, so the regulator must be free to reach the setting's
    // own floor
    let mut lowest = 100.0f32;
    for _ in 0..300 {
        if let Held::At(at) = holding.step(Asked::Hold(1.0), stand.second(angle), 1.0, &tuning()) {
            angle = at;
            lowest = lowest.min(at);
        }
    }
    assert!((lowest - tuning().min).abs() < 0.01,
            "a fan that never fell out was held at {lowest} %, and the setting's floor is {}", tuning().min);
}

// ---- what the regulator must forget when it hands the fan to somebody else ----

/// Drives a start until the regulator is in the middle of coming down off its kick, and answers the angle.
fn caught_mid_descent(holding: &mut Holding, stand: &mut ColdFan) -> f32 {
    let mut angle = 0.0;
    for _ in 0..40 {
        let said = stand.second(angle);
        if let Held::At(at) = holding.step(Asked::Hold(30.0), said, 1.0, &tuning()) {
            // the descent is the stretch after the kick where the angle is walking DOWN from it
            if at < regulator::KICK_PERCENT && angle >= regulator::KICK_PERCENT {
                return at;
            }
            angle = at;
        }
    }
    panic!("the model never got as far as coming down off the kick");
}

#[test]
fn a_hand_taking_the_fan_leaves_no_descent_behind_it() {
    // **THE MEMORY OF A START MUST NOT OUTLIVE THE START.** A person takes the fan by hand in the middle of
    // the descent, holds it as long as they like, and hands it back with a new setpoint - and the regulator
    // carried on taking the angle DOWN, because the descent was still in its head, while the setpoint was
    // asking for more air.
    let mut stand = ColdFan::new();
    let mut holding = Holding::from(0.0);
    let angle = caught_mid_descent(&mut holding, &mut stand);
    assert!(angle < regulator::KICK_PERCENT, "not in a descent: {angle}");

    // the hand takes it, and holds it for a while at an angle of its own
    for _ in 0..20 {
        assert_eq!(holding.step(Asked::ByHand, stand.second(40.0), 1.0, &tuning()), Held::Alone);
    }
    // and hands it back asking for MORE air than the fan is making. What is judged here is the DIRECTION: a
    // regulator still in its descent walks the angle down whatever the setpoint says, and one that has
    // forgotten it does what the error asks, which is up.
    let back = holding.step(Asked::Hold(290.0), stand.second(40.0), 1.0, &tuning());
    match back {
        Held::At(at) => assert!(at > angle,
            "handed back asking for more air, the regulator took the angle from {angle} % down to {at} % - the \
             descent of the start it was in twenty seconds ago is still in its head"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_purge_leaves_no_descent_behind_it_either() {
    // The neighbour, and the same class. A purge sets full air; a descent still remembered takes the angle
    // down from a hundred the moment the setpoint comes back.
    let mut stand = ColdFan::new();
    let mut holding = Holding::from(0.0);
    let angle = caught_mid_descent(&mut holding, &mut stand);
    assert!(angle < regulator::KICK_PERCENT, "not in a descent: {angle}");

    assert_eq!(holding.step(Asked::Purge, stand.second(angle), 1.0, &tuning()), Held::At(100.0));
    let back = holding.step(Asked::Hold(290.0), stand.second(100.0), 1.0, &tuning());
    match back {
        // The ceiling of the setting is where a purge's hundred per cent lands as soon as the regulator holds
        // again - that is the setting doing its job. What must NOT happen is the descent taking it further,
        // which is what one remembered step off the purge looks like.
        Held::At(at) => assert!(at >= tuning().max,
            "out of a purge asking for more air, the regulator took the angle to {at} %, under its own ceiling \
             of {} %", tuning().max),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_stop_already_forgets_it_and_still_does() {
    // The third path, closed before this round through `stood()`. Kept as a pair so that a change to the
    // forgetting cannot quietly reopen it.
    let mut stand = ColdFan::new();
    let mut holding = Holding::from(0.0);
    caught_mid_descent(&mut holding, &mut stand);
    assert_eq!(holding.step(Asked::Stop, stand.second(20.0), 1.0, &tuning()), Held::At(0.0));
    // the fan has actually stopped by the time it is asked for again - a reading under the sensor's own zero
    // is what a standing fan gives, and it is what makes this a START rather than a continuation
    let back = holding.step(Asked::Hold(290.0), Reading::Under, 1.0, &tuning());
    // from a standstill the answer is a kick, not a descent
    assert_eq!(back, Held::At(regulator::KICK_PERCENT));
}

// ---------------------------------------------------------------------------------------------------------
// The fan looks the same all the way: the error is divided by how steeply the draught answers the angle
// HERE, so one set of gains is right at both ends of the travel.
// ---------------------------------------------------------------------------------------------------------

/// One step from a fan that has already proved it turns, and how far the angle moved.
///
/// The first step is handed a reading EQUAL to the setpoint: it proves the fan turns, the error is nothing, so
/// the angle does not move and the step under test starts from a known angle and a known previous error.
fn moved_by(angle: f32, pa: f32, want: f32) -> f32 {
    let t = tuning();
    let mut hold = Holding::from(angle);
    let Held::At(settled) = hold.step(Asked::Hold(want), Reading::Is(want), 1.0, &t) else {
        panic!("the regulator would not hold a fan answering its setpoint exactly");
    };
    let Held::At(after) = hold.step(Asked::Hold(want), Reading::Is(pa), 1.0, &t) else {
        panic!("the regulator would not hold a fan reading {pa} Pa");
    };
    after - settled
}

#[test]
fn the_same_miss_in_pascals_moves_the_angle_less_where_the_curve_is_steep() {
    // **THE WHOLE OF THE CURVE'S STEEPNESS IN ONE ROW.** Ten pascals of miss low down the travel and ten pascals of miss
    // high up it are not the same job for the fan: low down a percent of angle is worth about five pascals and
    // high up about seven, so the same miss needs a bigger move of the angle low down and a smaller one high
    // up. And the power of the curve is not one number - measured on the board 21.09, it falls from 2.07 low
    // down to 1.53 high up - so each end is read with the power of its own band.
    //
    // low:  14 % and 21 Pa,  band below 25 %  → 2.07·21/14 = 3.11 Pa a percent
    // high: 40 % and 175 Pa, band above 35 %  → 1.53·175/40 = 6.69 Pa a percent
    let low = moved_by(14.0, 21.0, 31.0);
    let high = moved_by(40.0, 175.0, 185.0);
    let steepness = (1.53 * 175.0 / 40.0) / (2.07 * 21.0 / 14.0);
    assert!(low > 0.0 && high > 0.0, "the angle did not go up on a draught under the setpoint: {low} and {high}");
    let moves = low / high;
    assert!(
        (moves - steepness).abs() < steepness * 0.1,
        "the same ten pascals of miss moved the angle by {low} % low down and {high} % high up - a ratio of \
         {moves}, where the steepnesses differ by {steepness}. A ratio of one is a regulator that has not been \
         told the fan is not a straight line; the ratio of one single power is a regulator reading the top of \
         the travel with the shape of the bottom"
    );
}

#[test]
fn the_steepness_does_not_jump_at_the_edge_of_a_band_the_regulator_works_in() {
    // **THE BANDS ARE A STEP FUNCTION, SO EVERY EDGE IS A JUMP, AND THE QUESTION IS HOW BIG.** The steepness
    // is `n·pa/angle`, and at an edge the pressure and the angle are the same on both sides - so the jump is
    // exactly the ratio of the two powers and does not depend on the draught at all.
    //
    // At 25 % that ratio is 2.07/1.92 = 1.078, inside the tenth part the stage asks for. At 35 % it is
    // 1.92/1.53 = 1.255, and it is NOT: the fan's shape really does break there, and no choice of numbers
    // inside these three bands closes it.
    //
    // Both are held here on purpose. The setpoints this regulator holds sit at 16 % and 22 % of travel, so the
    // working edge is the one at 25 %; the one at 35 % is a known limit of the banding, and a test that says
    // so is what stops it being quietly re-tuned into something else.
    let at = |angle: f32, pa: f32, power: f32| power * pa / angle;
    let low = (at(25.0, 74.5, 2.07) - at(25.0, 74.5, 1.92)).abs() / at(25.0, 74.5, 2.0);
    assert!(low < 0.1, "the steepness jumps {low} across the 25 % edge, where the regulator actually works");
    let high = (at(35.0, 141.5, 1.92) - at(35.0, 141.5, 1.53)).abs() / at(35.0, 141.5, 1.72);
    assert!(high > 0.2, "the 35 % edge no longer carries the break the board measured - the bands have moved");
}

#[test]
fn a_draught_far_under_what_the_angle_should_make_does_not_throw_the_angle() {
    // **THE BAD-DATA PAIR OF THE ROW ABOVE, AND IT IS NOT A HYPOTHETICAL.** Six pascals at twenty per cent is
    // a fan whose air is going somewhere else - a damper shut, a tube off - and the live steepness that comes
    // out of those two numbers is 1.92·6/20 = 0.58 Pa a percent. Divided by THAT, a miss of twenty-five
    // pascals asks for a move four times what the fan's own curve can justify, and the regulator spends the
    // whole of its rate limit every second while the draught it is chasing is not coming.
    //
    // The floor under the steepness is 2.4 Pa a percent: the fan's own curve at the bottom of the travel
    // (`pa = 0.186·8^1.92 = 10 Pa` at 8 %, so 1.92·10/8). Below that the curve does not go, so a smaller
    // number is noise or a fault and not a flatter fan.
    let thrown = moved_by(20.0, 6.0, 31.0);
    let gains = tuning().up;
    let by_the_floor = (31.0 - 6.0) * (gains.kp_milli + gains.ki_milli) as f32 / 1000.0 / 2.4;
    assert!(
        thrown.is_finite(),
        "the angle moved by {thrown} - a steepness of nothing divided into the error"
    );
    assert!(
        thrown <= by_the_floor * 1.1,
        "six pascals at twenty per cent moved the angle {thrown} %, where the floor under the steepness allows \
         {by_the_floor} %"
    );
}

#[test]
fn a_fan_that_makes_no_draught_at_all_neither_divides_by_nothing_nor_runs_away() {
    // The far end of the same class: the draught has gone to nothing while the angle is still up. The fan is
    // out, the stall guard lifts the floor, and what must not happen on the way is a division by zero or an
    // angle that is not a number.
    let t = tuning();
    let mut hold = Holding::from(20.0);
    let _ = hold.step(Asked::Hold(31.0), Reading::Is(31.0), 1.0, &t);
    let held = hold.step(Asked::Hold(31.0), Reading::Is(0.0), 1.0, &t);
    match held {
        Held::At(at) => {
            assert!(at.is_finite(), "the angle came out {at} on a draught of nothing");
            assert!((t.min..=t.max).contains(&at), "the angle came out {at} %, outside {} to {} %", t.min, t.max);
        }
        other => panic!("a draught of nothing answered {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------------------
// The board knows its fan: the angle that holds a setpoint is REMEMBERED from when the fan was actually
// holding it, not hunted for during a start by watching a draught that lags four seconds behind the angle.
// ---------------------------------------------------------------------------------------------------------

/// Runs SECONDS of holding WANT against the measured fan, and answers the angle each second.
fn ran(hold: &mut Holding, fan: &mut ColdFan, want: f32, seconds: usize) -> Vec<f32> {
    let t = tuning();
    let mut angles = Vec::new();
    let mut angle = hold.angle();
    for _ in 0..seconds {
        let said = fan.second(angle);
        angle = match hold.step(Asked::Hold(want), said, 1.0, &t) {
            Held::At(at) => at,
            Held::Blind(_) => 0.0,
            Held::Alone => panic!("nobody is holding this fan by hand"),
        };
        angles.push(angle);
    }
    angles
}

/// A start from cold onto WANT, and the three numbers the stage judges it by: how high the draught went, how
/// low it fell once it had been over, and when it settled within the band and stayed.
fn from_cold(hold: &mut Holding, fan: &mut ColdFan, want: f32, seconds: usize) -> (f32, f32, Option<usize>) {
    let t = tuning();
    let (mut angle, mut highest, mut lowest_after, mut over) = (hold.angle(), 0.0f32, f32::MAX, false);
    let mut inside_since = None;
    for second in 0..seconds {
        let said = fan.second(angle);
        if let Reading::Is(pa) = said {
            highest = highest.max(pa);
            over = over || pa >= want;
            if over {
                lowest_after = lowest_after.min(pa);
            }
            match over && (pa - want).abs() <= 3.0 {
                true => inside_since = inside_since.or(Some(second)),
                false => inside_since = None,
            }
        }
        angle = match hold.step(Asked::Hold(want), said, 1.0, &t) {
            Held::At(at) => at,
            Held::Blind(_) => 0.0,
            Held::Alone => panic!("nobody is holding this fan by hand"),
        };
    }
    (highest, lowest_after, inside_since)
}

#[test]
fn the_board_remembers_the_angle_that_held_the_setpoint() {
    // **THE FACT THE FAN PROVES ABOUT ITSELF, like the floor.** While the regulator is holding and the draught
    // has stopped moving, the angle and the pressure belong together: that pair says what this fan does, at
    // this temperature, on this chimney. It is the one moment when they can be believed.
    let (mut hold, mut fan) = (Holding::from(0.0), ColdFan::new());
    let angles = ran(&mut hold, &mut fan, 31.0, 240);
    let held = angles[angles.len() - 1];
    let (pa, angle) = hold.knows().expect("four minutes of holding taught the board nothing about its fan");
    assert!(
        (angle - held).abs() < 2.0,
        "it remembers holding {pa} Pa at {angle} %, and it is holding {held} % right now"
    );
    assert!((pa - 31.0).abs() <= 3.0, "it remembers a pressure of {pa} Pa, and 31 was what it held");
}

#[test]
fn a_pair_is_not_taken_while_the_draught_is_still_moving() {
    // **THE PAIR THAT MAKES THE WHOLE THING WORTH ANYTHING.** During a transient the pressure belongs to the
    // angle of four seconds ago, so an angle and a pressure taken together are two facts about different
    // moments - exactly the mistake this stage exists to stop making. A pair taken there would be worse than
    // no pair, because it would be believed.
    let (mut hold, mut fan) = (Holding::from(0.0), ColdFan::new());
    ran(&mut hold, &mut fan, 31.0, 20);
    assert!(
        hold.knows().is_none(),
        "twenty seconds in, with the draught still climbing, the board already claims to know its fan: {:?}",
        hold.knows()
    );
}

#[test]
fn a_descent_stops_at_the_angle_the_board_knows_rather_than_at_the_floor() {
    // The whole of the repair in one row. Having held 31 Pa, the board is stopped and started again: the
    // descent off the kick now has somewhere to stop that is not the floor of the setting.
    let (mut hold, mut fan) = (Holding::from(0.0), ColdFan::new());
    ran(&mut hold, &mut fan, 31.0, 240);
    let (pa, angle) = hold.knows().expect("the board learnt nothing to start from");
    let wants = angle * (31.0f32 / pa).powf(1.0 / 2.07);

    let mut hold = Holding::from(0.0);
    let mut fan = ColdFan::new();
    hold.was_holding(pa, angle);
    let angles = ran(&mut hold, &mut fan, 31.0, 90);
    let lowest = angles.iter().cloned().fold(f32::MAX, f32::min);
    assert!(
        lowest >= wants - 2.0,
        "the descent took the angle down to {lowest} %, where the fan it knows holds 31 Pa at {wants} %"
    );
}

#[test]
fn a_start_from_cold_neither_blows_past_the_setpoint_nor_falls_away_under_it() {
    // **BOTH HALVES IN ONE ROW, BECAUSE THE REPAIR HAS SWAPPED THEM TWICE.** The stage asks for all three
    // numbers off one run on the board - no more than 40 Pa on the way to 31, never under 25, settled within
    // 45 s - and this is the same pair on the measured model, with the fan the board already knows.
    let (mut hold, mut fan) = (Holding::from(0.0), ColdFan::new());
    ran(&mut hold, &mut fan, 31.0, 240);
    let (pa, angle) = hold.knows().expect("the board learnt nothing to start from");

    let mut hold = Holding::from(0.0);
    let mut fan = ColdFan::new();
    hold.was_holding(pa, angle);
    let (highest, lowest, settled) = from_cold(&mut hold, &mut fan, 31.0, 180);
    println!("from cold onto 31 Pa: highest {highest:.0}, lowest after going over {lowest:.0}, settled {settled:?}");
    assert!(highest <= 40.0, "a start onto 31 Pa reached {highest:.0} Pa on the way, and 40 is the most allowed");
    assert!(lowest >= 25.0, "having been over the setpoint it fell to {lowest:.0} Pa, and 25 is the least allowed");
    assert!(settled.is_some_and(|at| at <= 45), "it settled at {settled:?} s, and 45 s is the most allowed");
}

// ---------------------------------------------------------------------------------------------------------
// The manoeuvre is one thing (3.6, second round): everything that describes what the regulator is in the
// MIDDLE of is dropped together when the fan is handed to somebody else. A field that outlives the handover
// teaches the board a fact it never proved.
// ---------------------------------------------------------------------------------------------------------

/// Holds WANT steadily for SECONDS, without reaching the ten the board needs to believe a pair.
fn nearly_steady(hold: &mut Holding, want: f32, seconds: usize) {
    let t = tuning();
    for _ in 0..seconds {
        hold.step(Asked::Hold(want), Reading::Is(want), 1.0, &t);
    }
}

/// One second of holding after the fan comes back, and whether the board claims to know it by then.
fn knows_after_one_second(hold: &mut Holding, want: f32) -> Option<(f32, f32)> {
    hold.step(Asked::Hold(want), Reading::Is(want), 1.0, &tuning());
    hold.knows()
}

#[test]
fn a_hand_on_the_knob_drops_the_steadiness_counted_so_far() {
    // **THE SAME CLASS AS THE REMEMBERED DESCENT, ONE ROUND LATER.** The board believes a pair only after the
    // draught has stood still for ten seconds - that is what tells a fact about the fan from a snapshot of a
    // transient. Carried across a handover, nine of those seconds belong to a manoeuvre that is over, and the
    // pair is written after one second of the new one. A wrong pair is worse than none: it sets the angle the
    // next start comes down to.
    let mut hold = Holding::from(16.0);
    nearly_steady(&mut hold, 31.0, 9);
    assert!(hold.knows().is_none(), "nine seconds is not ten, and the board already believes a pair");

    assert_eq!(hold.step(Asked::ByHand, Reading::Is(31.0), 1.0, &tuning()), Held::Alone);
    assert!(
        knows_after_one_second(&mut hold, 31.0).is_none(),
        "a second after a hand gave the fan back, the board claims to know it: the nine seconds of the old \
         manoeuvre were counted towards the new one"
    );
}

#[test]
fn a_purge_drops_it_too() {
    // The neighbour: full air, no feedback, and everything the regulator was in the middle of is over.
    let mut hold = Holding::from(16.0);
    nearly_steady(&mut hold, 31.0, 9);
    assert_eq!(hold.step(Asked::Purge, Reading::Is(31.0), 1.0, &tuning()), Held::At(100.0));
    assert!(
        knows_after_one_second(&mut hold, 31.0).is_none(),
        "a second after a purge, the board claims to know its fan"
    );
}

#[test]
fn a_sensor_that_went_away_and_came_back_drops_it_as_well() {
    // The third path, and the one nobody commands: the draught stops being readable, the fan is stopped, and
    // then the reading returns. Whatever was being counted belonged to the holding that the fault ended.
    let mut hold = Holding::from(16.0);
    nearly_steady(&mut hold, 31.0, 9);
    assert_eq!(hold.step(Asked::Hold(31.0), Reading::Missing, 1.0, &tuning()), Held::Blind(Blind::Missing));
    assert!(
        knows_after_one_second(&mut hold, 31.0).is_none(),
        "a second after the sensor came back, the board claims to know its fan"
    );
}

#[test]
fn the_pair_the_fan_taught_the_board_survives_all_three_handovers() {
    // **THE PAIR, and it is what stops the fix from being 'clear everything'.** A handover ends a manoeuvre,
    // not the machine, so what the fan PROVED about itself stays true after a hand, a purge or a fault.
    // Thrown away, it would make every start reason from somebody else's fan again.
    //
    // **WHAT THIS CASE DOES NOT CHECK, AND WHY IT SAYS SO.** The other fact the fan proves is the floor it
    // needs, and it survives a handover too - but nothing outside can see it. It has no accessor, and its
    // only outward sign is the fan not falling out; reading it that way needs a setpoint low enough that the
    // floor is the only thing keeping the rotor turning, and at such a setpoint this model falls out anyway,
    // - and at such a setpoint this model falls out anyway. So the floor is left to the cases that tell it on
    // a stand they dictate (`a_stall_raises_the_floor_by_one_step_however_long_the_draught_is_gone` and the
    // two beside it) rather than claimed here: a name that promises more than the body does is what the
    // reader leans on, and it is the mistake this whole round is about.
    //
    // `turning` is not in the drawer at all - see the case below.
    let mut stand = ColdFan::new();
    let mut hold = Holding::from(0.0);
    ran(&mut hold, &mut stand, 31.0, 120);
    let learnt = hold.knows().expect("two minutes of holding taught the board nothing about its fan");

    hold.step(Asked::ByHand, Reading::Is(31.0), 1.0, &tuning());
    assert_eq!(hold.knows(), Some(learnt), "a hand on the knob made the board forget its fan");
    hold.step(Asked::Purge, Reading::Is(31.0), 1.0, &tuning());
    assert_eq!(hold.knows(), Some(learnt), "a purge made the board forget its fan");
    hold.step(Asked::Hold(31.0), Reading::Missing, 1.0, &tuning());
    assert_eq!(hold.knows(), Some(learnt), "a sensor fault made the board forget its fan");
}

#[test]
fn turning_survives_a_hand_and_a_purge_and_is_dropped_by_a_fault() {
    // **THE CASE THAT KEEPS THE ONE ABOVE HONEST.** "Whether the fan turns" is not in the same drawer as the
    // pair and the floor: those are facts about the machine, this is a fact about right now. A hand on the
    // knob and a purge leave the fan blowing, so it still turns. A sensor fault stops it - `stood` commands
    // zero - and a stopped fan proves nothing, so the board has to find out again.
    let t = tuning();
    let mut hold = Holding::from(16.0);
    hold.step(Asked::Hold(31.0), Reading::Is(31.0), 1.0, &t);
    assert!(hold.turning(), "a draught of 31 Pa is a turning fan");

    hold.step(Asked::ByHand, Reading::Is(31.0), 1.0, &t);
    assert!(hold.turning(), "a hand on the knob stopped nothing, and the board forgot the fan turns");
    hold.step(Asked::Purge, Reading::Is(31.0), 1.0, &t);
    assert!(hold.turning(), "a purge blows harder, and the board forgot the fan turns");

    hold.step(Asked::Hold(31.0), Reading::Missing, 1.0, &t);
    assert!(!hold.turning(), "the fan was stopped on a fault, and the board still believes it turns");
}

#[test]
fn a_stall_does_not_carry_its_steadiness_into_what_follows() {
    // The neighbouring surface, checked because the handover fix was about a class rather than a field. A
    // stall is not a handover - the regulator keeps working - so the manoeuvre is not dropped there. The
    // counter still must not survive it, and here it does not: the moment the draught falls away the error
    // leaves the noise band by tens of pascals, which zeroes the count on that very step.
    //
    // **This one was green the day it was written**, and it is a latch rather than a caught defect: it holds
    // the reasoning above, so that a later change to the band or to the counter cannot quietly break it.
    let mut hold = Holding::from(16.0);
    nearly_steady(&mut hold, 31.0, 9);
    hold.step(Asked::Hold(31.0), Reading::Is(1.0), 1.0, &tuning());
    assert!(
        knows_after_one_second(&mut hold, 31.0).is_none(),
        "the seconds counted before the fan fell out were counted towards what came after"
    );
}

// ---- 3.7: the floor rises once per stall, and it holds while it stands ----

/// A stand told what the sensor says, second by second, so the stall happens where the test puts it.
///
/// **THE MODELS CANNOT PLACE A STALL EXACTLY.** `ColdFan` drops its rotor under an angle and the tract empties
/// at its own pace, so the second the board notices is an outcome of the model rather than a fact of the test -
/// and both probes here are about what the regulator does in a NAMED second.
struct Told(Vec<f32>);

impl Told {
    fn second(&mut self, _angle: f32) -> Reading {
        let pa = if self.0.is_empty() { 0.0 } else { self.0.remove(0) };
        match pa {
            pa if pa < 0.5 => Reading::Under,
            pa => Reading::Is(pa),
        }
    }
}

/// The draught a rotor that has just stopped leaves behind: 20 Pa emptying through a tract whose constant the
/// board was measured to have (`tools/tract_lag_probe.py`: 63 % of a step down in 3 s).
///
/// **IT IS NUMBERS AND NOT NOTHING, AND THAT IS THE WHOLE POINT.** The stall is noticed by a reading BELOW
/// what proves the fan turns, not by the reading going away: a draught that has gone entirely takes the board
/// down another path (`Under` -> Blind -> stood). A test that feeds zeroes exercises that path instead and
/// never reaches the block it means to judge.
fn emptying(seconds: usize) -> Vec<f32> {
    (0..seconds).map(|n| (20.0 * (-(n as f32) / 3.0).exp()).max(0.6)).collect()
}

/// Runs a told stand and answers the angle commanded each second.
fn angles(hold: &mut Holding, stand: &mut Told, want: f32, seconds: usize) -> Vec<f32> {
    let t = tuning();
    let mut out = Vec::new();
    for _ in 0..seconds {
        let said = stand.second(hold.angle());
        out.push(match hold.step(Asked::Hold(want), said, 1.0, &t) {
            Held::At(at) => at,
            Held::Blind(_) => 0.0,
            Held::Alone => panic!("nobody is holding it by hand"),
        });
    }
    out
}

/// Holds 20 Pa steadily, then loses the draught for GONE seconds, then gets far more air than asked.
///
/// The last stretch is what makes the floor VISIBLE without reading it: with 80 Pa against a setpoint of 20
/// the regulator drives the angle down as hard as it may, and where it stops is the floor - read within the
/// thirty seconds before `warmed_down` may slide it.
fn stalled_for(gone: usize) -> (f32, f32) {
    let mut hold = Holding::from(0.0);
    let mut stand = Told([vec![20.0; 40], emptying(gone), vec![80.0; 25]].concat());
    let steady = angles(&mut hold, &mut stand, 20.0, 40);
    let fell_at = *steady.last().expect("the steady stretch commanded nothing");
    angles(&mut hold, &mut stand, 20.0, gone);
    let after = angles(&mut hold, &mut stand, 20.0, 25);
    let floor = after.iter().cloned().fold(f32::INFINITY, f32::min);
    (fell_at, floor)
}

#[test]
fn a_stall_raises_the_floor_by_one_step_however_long_the_draught_is_gone() {
    // ONE STALL IS ONE FACT ABOUT THE FAN: at this angle, at this temperature, it fell out. How long the tract
    // then takes to empty says nothing more about the motor, so it may not add to what was learned.
    let (fell_at, floor) = stalled_for(8);
    assert!(
        floor <= fell_at + regulator::STEP_ABOVE_A_STALL + 0.5,
        "the fan fell out at {fell_at:.2} % and the floor came out at {floor:.2} % - eight seconds without \
         draught added {:.2} % to a fact worth one step",
        floor - fell_at - regulator::STEP_ABOVE_A_STALL
    );
}

#[test]
fn a_longer_silence_does_not_find_a_higher_floor_than_a_short_one() {
    // THE PAIR, and it is what says the first is about the stall rather than about the number eight: the same
    // stall, told twice with the tract taking six seconds to empty and twelve, has to find the same floor.
    //
    // **BOTH SILENCES HAVE TO CONTAIN A STALL, or the pair compares a stall with nothing.** The tract leaves
    // 20 Pa behind and falls through the threshold that proves turning in about five seconds, so a silence of
    // one second never reaches it: measured, that told the board nothing at all and left no floor.
    let (_, after_six) = stalled_for(6);
    let (_, after_twelve) = stalled_for(12);
    assert!(
        (after_twelve - after_six).abs() <= 0.5,
        "six seconds of emptying found a floor of {after_six:.2} % and twelve found {after_twelve:.2} %: \
         the floor is being set by the silence and not by the stall"
    );
}

#[test]
fn while_the_floor_stands_the_regulator_commands_nothing_below_it() {
    // **A LATCH, NOT A CATCH, and it is green from birth on purpose.** It caught no defect: measured before
    // any of this was touched, the regulator already commanded nothing below the floor while the floor stood.
    // What it guards is the repair beside it - mending the wind-up CHANGES WHERE THE FLOOR SITS, and the
    // cheapest way to make the wind-up go away is to stop the floor holding at all. Then every case here would
    // pass and the fan would walk back into the stall it just proved.
    //
    // Read inside the thirty seconds `warmed_down` waits before it may slide: past that, going lower is the
    // warm motor being let down on purpose and not a fault.
    //
    // **THE FLOOR IS WHERE THE REGULATOR STOPS, NOT WHERE THE ANGLE WAS WHEN THE STALL WAS NOTICED.** Those
    // two differ: the draught falling pushes the angle above the floor on the way, and coming back down to
    // the floor afterwards is the regulator doing its job.
    let mut hold = Holding::from(0.0);
    let mut stand = Told([vec![20.0; 40], emptying(6), vec![80.0; 29]].concat());
    angles(&mut hold, &mut stand, 20.0, 40);
    angles(&mut hold, &mut stand, 20.0, 6);
    let after = angles(&mut hold, &mut stand, 20.0, 29);
    let (settling, standing) = after.split_at(12);
    let floor = settling.iter().cloned().fold(f32::INFINITY, f32::min);
    let lowest = standing.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(
        lowest >= floor - 0.01,
        "the regulator settled onto a floor of {floor:.2} % and then went to {lowest:.2} % within the thirty \
         seconds the floor is meant to stand"
    );
}

#[test]
fn two_stalls_find_two_steps_and_not_the_seconds_between_them() {
    // THE PAIR ON THE OTHER SIDE: one stall is one step, so two stalls are two. A fix that answers "one step,
    // ever" would pass the case above and lose what the second stall proved - the fan fell out again, higher.
    let mut hold = Holding::from(0.0);
    let mut stand = Told(
        [vec![20.0; 40], emptying(6), vec![20.0; 40], emptying(6), vec![80.0; 25]].concat(),
    );
    let steady = angles(&mut hold, &mut stand, 20.0, 40);
    let fell_at = *steady.last().expect("the steady stretch commanded nothing");
    angles(&mut hold, &mut stand, 20.0, 6);
    angles(&mut hold, &mut stand, 20.0, 40);
    angles(&mut hold, &mut stand, 20.0, 6);
    let after = angles(&mut hold, &mut stand, 20.0, 25);
    let floor = after.iter().cloned().fold(f32::INFINITY, f32::min);
    let steps = (floor - fell_at) / regulator::STEP_ABOVE_A_STALL;
    assert!(
        steps <= 2.5,
        "two stalls with twelve seconds of emptying between them took the floor {steps:.1} steps above the \
         {fell_at:.2} % the fan was holding at - two stalls are two facts, and the seconds are neither"
    );
}

/// How many times the fan fell out during a run, counted the way the board counts it, and at what angles.
///
/// **THE RUN IS SHORT ON PURPOSE, and the reason is `warmed_down`.** The floor slides back down a step every
/// thirty seconds the regulator sits on it, because a warming motor holds lower - and this model never warms,
/// so over a long run it falls out again and again BY DESIGN. Counting stalls over four hundred seconds would
/// forbid the warm-down, which is exactly the mistake the red probe of 3.6 made.
fn stalls_in(seconds: usize, want: f32) -> (usize, Vec<f32>) {
    let mut stand = ColdFan::new();
    let mut hold = Holding::from(0.0);
    let mut angle = 0.0;
    let (mut stalls, mut gone_before, mut fell) = (0, false, Vec::new());
    for _ in 0..seconds {
        let said = stand.second(angle);
        let gone = matches!(said, Reading::Under)
            || matches!(said, Reading::Is(pa) if pa < regulator::TURNS_ABOVE_PA);
        if gone && !gone_before && angle > 0.0 {
            stalls += 1;
            fell.push(angle);
        }
        gone_before = gone;
        angle = match hold.step(Asked::Hold(want), said, 1.0, &tuning()) {
            Held::At(at) => at,
            Held::Blind(_) => 0.0,
            Held::Alone => panic!("nobody is holding it by hand"),
        };
    }
    (stalls, fell)
}

#[test]
fn a_floor_found_on_a_descent_does_not_let_the_fan_fall_out_again() {
    // **A LATCH, and it is green from birth - the repeat it forbids was already forbidden.** An osmotr read
    // five stalls in one run and took them for a floor set too low; measured inside the window the floor is in
    // force, there is exactly one. The other four belong to `warmed_down`, which slides the floor back a step
    // every thirty seconds the regulator sits on it because a warming motor holds lower - and `ColdFan` never
    // warms, so over a long horizon it falls out for ever, by design.
    //
    // A delayed angle was tried against this case - remembering the angle the draught is the ANSWER to,
    // three seconds back - and it made the model fall out twice where it now falls once: the lag runs both
    // ways, and after the rotor drops the regulator pushes the angle UP, so an angle from three seconds ago
    // is the LOW one it was leaving. The case stands guard over what is already true.
    let (stalls, fell) = stalls_in(regulator::FLOOR_SLIDES_AFTER_S as usize + 40, 20.0);
    assert!(stalls >= 1, "the fan never fell out in this run: nothing is being tested");
    assert_eq!(
        stalls, 1,
        "the fan fell out {stalls} times, at {fell:?} % - the floor found at the first stall let it happen \
         again while that floor was still in force"
    );
}

#[test]
fn nothing_is_remembered_about_an_angle_the_regulator_does_not_command() {
    // **A HAND ON THE KNOB MEANS THE REGULATOR DOES NOT KNOW THE ANGLE.** Its own `angle` is whatever it left
    // behind; the fan is at whatever the person set. A stall noticed in that state writes a floor from a number
    // that is about nobody - and the floor outlives the handover, so the fiction stays.
    //
    // Read through what the floor DOES: with a low setpoint and plenty of draught the regulator drives the
    // angle down, and where it stops is the floor. With nothing remembered it stops at the tuning's own
    // minimum.
    let t = tuning();
    let mut hold = Holding::from(0.0);
    // held high first, so a floor written from the angle the regulator left behind stands out plainly
    let mut stand = Told([vec![60.0; 40], emptying(8), vec![90.0; 25]].concat());
    angles(&mut hold, &mut stand, 60.0, 40);
    // the hand takes it, and the draught dies away while the person holds it
    for _ in 0..8 {
        let said = stand.second(hold.angle());
        assert!(matches!(hold.step(Asked::ByHand, said, 1.0, &t), Held::Alone));
    }
    // read inside the thirty seconds before `warmed_down` may slide the floor away and hide the fiction
    let back = angles(&mut hold, &mut stand, 20.0, 25);
    let lowest = back.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(
        lowest <= t.min + 0.01,
        "after a stall noticed while a hand held the fan, the regulator would not go below {lowest:.2} % - it \
         remembered a floor from an angle it was not commanding, and the tuning's own minimum is {:.2} %",
        t.min
    );
}

// ---- what it made of each second, for a person tuning it ----

/// Holding 60 Pa and then 40 on the model, from a fan already turning: each second the angle before it, the angle it
/// decided and what it said it made of the second.
fn traced() -> Vec<(f32, f32, Option<regulator::Parts>, f32)> {
    let t = tuning();
    let (mut hold, mut stand) = (Holding::from(30.0), Stand { pa: 40.0 });
    let mut rows = Vec::new();
    for second in 0..300 {
        let want = if second < 150 { 60.0 } else { 40.0 };
        let before = hold.angle();
        let said = stand.second(before);
        match hold.step(Asked::Hold(want), said, 1.0, &t) {
            Held::At(at) => rows.push((before, at, hold.parts(), want - pa(said))),
            other => panic!("a turning fan on the model answered {other:?} at {second} s"),
        }
    }
    rows
}

#[test]
fn every_second_it_regulates_its_move_is_its_two_halves_cut_to_the_step_and_the_limits() {
    let t = tuning();
    let rows = traced();
    let told: Vec<_> = rows.iter().filter_map(|(before, at, parts, _)| parts.map(|p| (*before, *at, p))).collect();
    assert!(told.len() >= 290, "the regulator said what it made of {} seconds of 300 it regulated", told.len());
    for (before, at, p) in &told {
        let moved = (p.p + p.i).clamp(-t.step, t.step);
        let wanted = before + moved;
        // cut to the limits: the ceiling from above, the floor - the setting's or the fan's own, higher - from below
        let fits = (at - wanted).abs() < 1e-4 || (wanted > t.max && *at == t.max) || (wanted < *at && *at >= t.min);
        assert!(fits, "from {before} the angle went to {at}, and P {} + I {} cut to ±{} makes {wanted}", p.p, p.i, t.step);
    }
    // the pair of the rows above, so that they are not passed by a regulator that never moves: it did move, both ways
    assert!(told.iter().any(|(_, _, p)| p.p.abs() > 0.01) && told.iter().any(|(_, _, p)| p.i.abs() > 0.01));
}

#[test]
fn the_miss_it_names_is_asked_less_came_out_and_its_sign_is_the_set_of_gains() {
    let rows = traced();
    let told: Vec<_> = rows.iter().filter_map(|(_, _, parts, miss)| parts.map(|p| (p, *miss))).collect();
    assert!(!told.is_empty(), "the regulator said nothing about any second");
    for (p, miss) in &told {
        assert!((p.miss_pa - miss).abs() < 1e-4, "it named a miss of {} Pa where asked less came out is {miss}", p.miss_pa);
    }
    // both sets worked: 60 Pa from 40 goes up, 40 from 60 comes down
    assert!(told.iter().any(|(p, _)| p.miss_pa > 1.0) && told.iter().any(|(p, _)| p.miss_pa < -1.0));
}

#[test]
fn a_second_it_does_not_regulate_has_nothing_to_say_rather_than_a_zero() {
    let t = tuning();
    // after a regulated second, so that a stale record would show
    let (mut hold, mut stand) = (Holding::from(30.0), Stand { pa: 40.0 });
    let said = stand.second(30.0);
    hold.step(Asked::Hold(60.0), said, 1.0, &t);
    for asked in [Asked::Stop, Asked::Purge, Asked::ByHand, Asked::Hold(0.0)] {
        hold.step(Asked::Hold(60.0), stand.second(hold.angle()), 1.0, &t);
        hold.step(asked, Reading::Is(40.0), 1.0, &t);
        assert_eq!(hold.parts(), None, "{asked:?} left the regulator saying it had regulated the second");
    }
    // a standing fan being kicked is not regulated either
    let mut cold = Holding::from(0.0);
    cold.step(Asked::Hold(60.0), Reading::Under, 1.0, &t);
    assert_eq!(cold.parts(), None, "a kick was reported as a regulated second");
}

#[test]
fn a_second_it_did_not_regulate_is_drawn_as_a_gap_and_one_it_did_as_its_numbers() {
    // the curve breaks where the regulator had nothing to say: a zero there reads "no miss, no move", which did not happen
    let none = regulator::Parts::points(None);
    assert!(none.iter().all(|v| v.is_nan()), "a second the regulator did not regulate is drawn as {none:?}");
    let told = regulator::Parts { miss_pa: -2.5, p: 0.25, i: -0.125 };
    assert_eq!(regulator::Parts::points(Some(told)), [-2.5, 0.25, -0.125]);
}
