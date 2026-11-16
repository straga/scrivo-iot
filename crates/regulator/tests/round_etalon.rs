//! The round of the new burner (`regulator::round`) against the round of the first one, step by step, on one deterministic run: the same draughts and the same setpoints go through both, and what each decides, commands, carries to the gate and
//! remembers of the fan is the same at every second. THE FIRST BURNER'S ROUND is written here as it stands in its `devices/burner/src/fan.rs` (`Fan::step`, f6c731f, lines 1325-1385 between the regulator's step and the curves): the step,
//! the angle by what was decided - the angle, zero when blind, nothing when a person holds it -, the gate carrying it, `not_carried_out` when it did not get through. The regulator it calls is the same crate in both: `git diff f6c731f --
//! crates/regulator/src/lib.rs` adds `round` and `mod knobs` and changes no line of `Holding`.
//!
//! The run is a boiler: a draught that follows the angle with the tract's lag and the breathing of the air, setpoints that move, a purge, a stop, a person's hand, a sensor that reads under its zero, nothing, no divider and over its scale, and
//! a gate that refuses on a schedule - the angle alone, the angle and the stop. A row that is true of every step of a run like that is true of the angle the board will hold.

use draught::Reading;
use regulator::{round, Asked, Carried, Gains, Held, Holding, Tuning};

fn tuning() -> Tuning {
    Tuning { up: Gains { kp_milli: 300, ki_milli: 400 }, down: Gains { kp_milli: 300, ki_milli: 400 }, min: 8.0, max: 80.0, step: 5.0, band_pa: 4.0, band_share: 0.25 }
}

/// What a round came to, in the form both rounds say it.
#[derive(Debug, PartialEq)]
struct Step {
    held: Held,
    commanded: Option<u32>,
    carried: Carried,
    angle: f32,
    turning: bool,
    knows: Option<(f32, f32)>,
    parts: Option<[f32; 3]>,
}

fn said(holding: &Holding, held: Held, commanded: Option<u32>, carried: Carried) -> Step {
    Step { held, commanded, carried, angle: holding.angle(), turning: holding.turning(), knows: holding.knows(), parts: holding.parts().map(|p| regulator::Parts::points(Some(p))) }
}

/// The first burner's round, as `Fan::step` made it.
fn first_burners(holding: &mut Holding, asked: Asked, reading: Reading, gate: &mut dyn FnMut(u32) -> Result<String, String>) -> Step {
    let held = holding.step(asked, reading, 1.0, &tuning());
    let angle = match held {
        Held::At(angle) => Some(angle),
        Held::Blind(_) => Some(0.0),
        Held::Alone => None,
    };
    let carried = match angle {
        Some(angle) => regulator::carry(angle.round() as u32, gate),
        None => Carried::Out,
    };
    if !matches!(carried, Carried::Out) {
        holding.not_carried_out();
    }
    said(holding, held, angle.map(|a| a.round() as u32), carried)
}

fn new_round(holding: &mut Holding, asked: Asked, reading: Reading, gate: &mut dyn FnMut(u32) -> Result<String, String>) -> Step {
    let out = round(holding, asked, reading, 1.0, &tuning(), gate);
    said(holding, out.held, out.commanded, out.carried)
}

/// The numbers of a run, from a seed: a linear congruential generator, so the run is the same on every machine.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32 / (1u64 << 31) as f32) - 0.5
    }
}

/// What the sensor says at second T of the run, from the pressure the tract made, and the setpoint asked.
fn asked_at(t: usize) -> Asked {
    match t {
        0..=39 => Asked::Stop,
        40..=159 => Asked::Hold(50.0),
        160..=199 => Asked::Hold(95.0),
        200..=229 => Asked::Purge,
        230..=329 => Asked::Hold(30.0),
        330..=359 => Asked::ByHand,
        360..=389 => Asked::Stop,
        _ => Asked::Hold(70.0),
    }
}

fn sensor_at(t: usize, pa: f32, noise: f32) -> Reading {
    match t {
        100..=109 => Reading::Under,
        260..=269 => Reading::Missing,
        290..=294 => Reading::Unconfigured,
        _ if pa > 300.0 => Reading::Over,
        _ => Reading::Is((pa + noise * 3.0).max(0.0)),
    }
}

/// The gate's schedule of refusals: the angle only at 120..123, the angle and the stop at 240..243 and 400..402, otherwise it takes what it is given.
fn gate_refuses(t: usize, percent: u32) -> bool {
    match t {
        120..=123 => percent != 0,
        240..=243 | 400..=402 => true,
        _ => false,
    }
}

const STEPS: usize = 500;

/// The run through ROUND: every step's `Step`, with the tract of the boiler answering the angle the gate was really given.
fn run(round: fn(&mut Holding, Asked, Reading, &mut dyn FnMut(u32) -> Result<String, String>) -> Step) -> Vec<Step> {
    let mut noise = Noise(7);
    let (mut holding, mut pa, mut set) = (Holding::from(0.0), 0.0f32, 0u32);
    let mut out = Vec::new();
    for t in 0..STEPS {
        let reading = sensor_at(t, pa, noise.next());
        let mut gate = |percent: u32| {
            if gate_refuses(t, percent) {
                Err("the gate refused".to_string())
            } else {
                set = percent;
                Ok(format!("{percent}%"))
            }
        };
        out.push(round(&mut holding, asked_at(t), reading, &mut gate));
        // the tract: the pressure of the fan's own curve at the angle really set, a lag behind it; a fan that is stopped makes none
        let made = if set == 0 { 0.0 } else { 0.186 * (set as f32).powf(1.92) };
        pa += (made - pa) / 4.0;
    }
    out
}

#[test]
fn the_new_round_decides_commands_carries_and_remembers_what_the_first_burners_does_at_every_step_of_a_run() {
    let (first, new) = (run(first_burners), run(new_round));
    assert_eq!(first.len(), STEPS);
    for (t, (a, b)) in first.iter().zip(&new).enumerate() {
        assert_eq!(a, b, "second {t}: the first burner's round and the new one's part");
    }
}

#[test]
fn the_run_goes_through_what_it_is_meant_to_ask_of_a_round() {
    // a run that never refused, never went blind or never held would agree with anything: the row is about the states it visits
    let first = run(first_burners);
    let has = |what: &dyn Fn(&Step) -> bool| first.iter().any(what);
    assert!(has(&|s| matches!(s.held, Held::At(_)) && s.commanded.is_some_and(|c| c > 0)), "never holding an angle");
    assert!(has(&|s| matches!(s.held, Held::Blind(_))), "never blind");
    assert!(has(&|s| matches!(s.held, Held::Alone)), "never by hand");
    assert!(has(&|s| matches!(s.carried, Carried::Stopped { .. })), "never a refused angle");
    assert!(has(&|s| matches!(s.carried, Carried::Lost { .. })), "never a gate that refused the stop too");
    assert!(has(&|s| s.turning) && has(&|s| !s.turning), "never both turning and standing");
}
