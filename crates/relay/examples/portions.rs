//! The task of a feeding relay on the real clock of this computer: a model of the automaton plans a rhythm every second (where the cycle stands now, built a moment before it is delivered), the task sleeps as the board's
//! does (for what `wait_ms` says, a second at most) and writes down every edge with the time it came at. Prints the journal and the length of each portion and pause.
//!
//!     cargo run -q -p relay --example portions -- ON_MS OFF_MS SECONDS [SAY_FOR_SECONDS]
//!
//! With SAY_FOR_SECONDS the model plans for that long and then falls silent: the plan it left runs out at the horizon and the relay opens.

use std::sync::Arc;
use std::time::{Duration, Instant};

use relay::{Feeding, Relays};

fn main() {
    let args: Vec<u32> = std::env::args().skip(1).map(|a| a.parse().expect("a number")).collect();
    let (on_ms, off_ms, seconds) = (args[0], args[1], args[2]);
    let say_for = args.get(3).copied();
    let relays = Arc::new(Relays::<1>::new());
    let began = Instant::now();
    let now_ms = || began.elapsed().as_millis() as u32;
    let mut journal: Vec<(u32, bool)> = Vec::new();
    let mut write = |_: usize, closed: bool| journal.push((now_ms(), closed));
    relays.start(&mut write);
    // the model of the automaton: one plan a second, the cycle standing where its own clock puts it
    let plan = move |at: u32| Feeding::Rhythm { into_ms: at % (on_ms + off_ms), on_ms, off_ms };
    relays.feed(0, plan(0), now_ms());
    {
        let (relays, began) = (relays.clone(), began);
        std::thread::spawn(move || {
            let mut said = 1;
            while say_for.is_none_or(|s| said <= s) {
                std::thread::sleep(Duration::from_millis(1000));
                let at = began.elapsed().as_millis() as u32;
                relays.still(0, plan(at), at);
                said += 1;
            }
        });
    }
    while now_ms() < seconds * 1000 {
        relays.step(now_ms(), &mut write);
        let wait = relays.wait_ms(now_ms()).unwrap_or(1000).min(1000);
        std::thread::sleep(Duration::from_millis(u64::from(wait)));
    }
    let mut closed_at = None;
    let mut opened_at = None;
    for (at, closed) in &journal {
        println!("{at:>7} ms  {}", if *closed { "closed" } else { "open" });
        if let (true, Some(open)) = (closed, opened_at) {
            println!("          pause {} ms", at - open);
        }
        if *closed {
            closed_at = Some(*at);
        } else if let Some(c) = closed_at.take() {
            println!("          portion {} ms", at - c);
            opened_at = Some(*at);
        }
    }
}
