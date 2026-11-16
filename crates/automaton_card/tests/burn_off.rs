//! Burning off ends by the fire and not by the clock. The live lighting of 24.09 burnt out to its
//! ceiling of 120 s, blew 20 s, and the stop switched the fan off over a pot with most of its dose alight. So: the fire
//! burns out on the step's draught until the flame has been gone a minute in a row; the ceiling is a setting, ten
//! minutes, and at it the pot is blown clear and stopped whatever the flame says; and in no state - a stop and
//! a fault too - does the fan stand while the sensor sees a flame. The numbers are the rows' own (tests/common), through the
//! mapping the board and the stand share (`automaton_card::tuning`).


mod common;

use automaton::{Asked, Doing, State};
use common::pair::{Automaton, Burning, Press, Seen, Silence, Sound};
use flame::Flame;


fn sound() -> Sound {
    common::sound(&[])
}

/// Lit at 90 s, at work from 154 s, and a person presses Stop at 200 s; FLAME gives each second after that.
const STOP: usize = 200;

fn run(seconds: u32, flame: impl Fn(u32) -> Flame) -> Vec<Doing> {
    let t = sound();
    let mut a = Automaton::new(Burning::new());
    (0..seconds)
        .map(|at| {
            let flame = if at < 90 { Flame::Out } else if (at as usize) < STOP { Flame::Steady } else { flame(at) };
            let seen = Seen { flame, boiler_c: Some(40.0), return_c: None, flue_c: Some(20.0 + at as f32 * 0.2), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD };
            match at as usize {
                2 => a.press(Press::Start, seen, &t),
                STOP => a.press(Press::Stop, seen, &t),
                _ => a.second(seen, 1.0, &t),
            }
        })
        .collect()
}

/// Of the burning off: how many seconds it held the step's draught, then how many it blew at full, and what came after.
fn burning_off(seconds: &[Doing]) -> (usize, usize, Option<State>) {
    let off: Vec<&Doing> = seconds[STOP..].iter().take_while(|d| matches!(d.state, State::BurningOff(_))).collect();
    let held = off.iter().take_while(|d| matches!(d.fan, Asked::Hold(_))).count();
    let blown = off.iter().skip(held).take_while(|d| d.fan == Asked::Purge).count();
    (held, blown, seconds.get(STOP + off.len()).map(|d| d.state))
}

#[test]
fn a_pot_that_burns_on_five_minutes_after_stop_is_burnt_out_until_a_minute_without_a_flame() {
    let seconds = run(900, |at| if (at as usize) < STOP + 300 { Flame::Steady } else { Flame::Out });
    assert_eq!(seconds[154].state, State::Working, "the rows expect the work from 154 s");
    let (held, blown, after) = burning_off(&seconds);
    assert!((359..=361).contains(&held), "burnt out {held} s on the step's draught, and the fire burnt 300 s and then 60 dark");
    assert_eq!((blown, after), (20, Some(State::Stop)), "then the pot is blown clear and the burner stops");
}

#[test]
fn a_pot_dark_from_the_stop_is_burnt_out_a_minute() {
    // the pair: no flame from the first second of burning off, and the minute without one is all it gets
    let seconds = run(900, |_| Flame::Out);
    let (held, blown, after) = burning_off(&seconds);
    assert!((59..=61).contains(&held), "burnt out {held} s over a dark pot, and the least is a minute");
    assert_eq!((blown, after), (20, Some(State::Stop)));
}

#[test]
fn a_fire_still_alight_at_the_ceiling_is_blown_clear_and_stopped_and_the_stop_keeps_its_fan_while_the_flame_shows() {
    // Plum 850P2, 8.7: the ceiling ends burning out whatever the flame says - a blow
    // and a stop, no fault; the stop holds the step's draught over the flame until it goes
    let gone = STOP + 700;
    let seconds = run(1200, |at| if (at as usize) < gone { Flame::Steady } else { Flame::Out });
    let (held, blown, after) = burning_off(&seconds);
    assert!((599..=601).contains(&held), "burnt out {held} s over a flame, and the ceiling is the card's ten minutes");
    assert_eq!((blown, after), (20, Some(State::Stop)), "at the ceiling the pot is blown clear and the burner stops");
    assert!(seconds.iter().all(|d| !matches!(d.state, State::Fault(_))), "a fire alight at the ceiling was made a fault");
    let stop = STOP + held + blown;
    assert!(seconds[stop..gone].iter().all(|d| d.state == State::Stop && d.fan == Asked::Hold(51.0) && !d.auger),
        "a stop over a flame: {:?}", seconds[stop..gone].iter().map(|d| (d.state, d.fan)).collect::<Vec<_>>());
    assert!(seconds[gone..].iter().all(|d| d.fan == Asked::Stop), "the flame is gone and the stop's fan still turns");
}

#[test]
fn a_pot_gone_dark_just_before_the_ceiling_is_blown_clear_and_stopped() {
    // the pair: dark 30 s before the ceiling, not yet the minute that ends burning out - the ceiling ends it, and a dark
    // pot is blown clear and stopped as before
    let seconds = run(1200, |at| if (at as usize) < STOP + 570 { Flame::Steady } else { Flame::Out });
    let (held, blown, after) = burning_off(&seconds);
    assert!((599..=601).contains(&held), "burnt out {held} s, and the ceiling is the card's ten minutes");
    assert_eq!((blown, after), (20, Some(State::Stop)), "a pot dark at the ceiling is blown clear and stopped");
}

#[test]
fn a_flame_seen_in_a_stop_keeps_the_fan_turning_and_a_dark_stop_rests_it() {
    // burnt out and stopped as in the pair above; then the pot flares at 400 s for twenty seconds
    let seconds = run(600, |at| if (400..420).contains(&at) { Flame::Steady } else { Flame::Out });
    assert_eq!(seconds[399].state, State::Stop);
    assert_eq!(seconds[399].fan, Asked::Stop, "a dark stop rests its fan");
    assert!(seconds[400..420].iter().all(|d| d.state == State::Stop && d.fan != Asked::Stop && !d.auger),
        "a stop over a flame: {:?}", seconds[400..420].iter().map(|d| (d.state, d.fan)).collect::<Vec<_>>());
    assert_eq!(seconds[421].fan, Asked::Stop, "the flame is gone and the fan rests again");
}
