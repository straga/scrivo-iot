//! The round of a sensor (`Sensor`, the half of this crate that needs the chip): read it every period, keep what it read, publish its card, announce
//! it when what the card shows has changed, and measure the step the rounds actually take - one order for every
//! sensor of every device, as ESPHome's PollingComponent calls `update` and its Sensor publishes.

use std::time::Duration;

use esp_idf_svc::sys::EspError;
use crate::sensor::{Kept, Sensor, Tally};

/// What a sensor's round is asked by the others: a curve it draws from what it keeps, its lines for the console,
pub enum Asked {
    Curve(String),
    Lines,
}

/// What it answers.
pub enum Said {
    Curve(Option<page::Curve>),
    Lines(Result<Vec<String>, String>),
}

/// Where a sensor's round is asked (between::Desk): the page's /curve and the console's lines. What it keeps is
/// read in its own task and nowhere else.
pub type Desk = between::Desk<Asked, Said, 2, 2>;

/// What a sensor's round answers with, from what it keeps: the curve called NAME (None - not its own), and its lines.
pub struct Answers {
    pub curve: fn(&str) -> Option<page::Curve>,
    pub lines: fn(&mut dyn FnMut(&str)) -> Result<(), String>,
}

/// How long the page or the console waits for a round: it answers between its readings, and the slowest round - the
/// bus of the temperatures, a frame a channel - reads for a couple of seconds.
const WITHIN: Duration = Duration::from_secs(5);

/// The curve NAME from the round at DESK, for a module's `curve` once it knows the name is its own.
pub fn curve(desk: &Desk, name: &str) -> Option<page::Curve> {
    match desk.ask(Asked::Curve(name.to_string()), WITHIN) {
        Ok(Said::Curve(curve)) => curve,
        Ok(Said::Lines(_)) => None,
        Err(refused) => {
            log::warn!("{name}: the round did not answer for its curve: {refused}");
            None
        }
    }
}

/// The lines of the round at DESK for the console, handed to LINE.
pub fn lines(desk: &Desk, line: &mut dyn FnMut(&str)) -> Result<(), String> {
    match desk.ask(Asked::Lines, WITHIN).map_err(|refused| refused.to_string())? {
        Said::Lines(lines) => lines.map(|lines| lines.iter().for_each(|one| line(one))),
        Said::Curve(_) => Err("the round answered its lines with a curve".into()),
    }
}

/// Starts the round of SENSOR as a task called NAME with STACK bytes of stack - the sensor's own number, measured on the board (`status`, "least stack ever free"), and not the one every round is given: a stack is the internal heap's, and a round that
/// leaves five of its eight kilobytes untouched holds three of them for nothing. A reading every EVERY (asked afresh each round, so a period
/// that is a setting works from the next round), kept in KEPT, the card published by PUBLISH every round and CARD
/// announced when it has news. Between readings it answers DESK by ANSWERS - so what it keeps is only ever read in
/// its own task.
///
/// Published every round, announced only on news: the card carries numbers that change without being news - the
/// bus's counts, the step of the rounds - and /state reads the card as published, so it must not lag behind them. A
/// publish is a copy into the card's place, without the heap.
pub fn poll<S: Sensor>(
    name: &'static str,
    stack: usize,
    card: &'static str,
    kept: &'static Kept<S>,
    every: fn() -> Duration,
    mut sensor: S,
    publish: fn(),
    desk: &'static Desk,
    answers: Answers,
) -> Result<(), EspError> {
    // a sensor's round keeps time with its hardware: core 1
    round::round(name, stack, round::Core::Hardware, move || {
        let mut tally = Tally::new();
        loop {
            let round = between::clock::now();
            let reading = sensor.read();
            let news = kept.take(reading);
            if let Some(step) = tally.round() {
                kept.measured(step);
            }
            publish();
            if news {
                sensor.told(reading.as_ref());
                web::announce(card);
            }
            desk.serve_for(every().saturating_sub(round.elapsed()), |asked| match asked {
                Asked::Curve(name) => Said::Curve((answers.curve)(&name)),
                Asked::Lines => {
                    let mut lines = Vec::new();
                    let said = (answers.lines)(&mut |one| lines.push(one.to_string()));
                    Said::Lines(said.map(|()| lines))
                }
            });
        }
    })
}
