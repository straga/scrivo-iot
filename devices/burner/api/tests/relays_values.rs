//! What the relays' module hands the stream for the states of its organs: on or off, and the seconds a limit leaves of the auger and of the igniter. The states come from the machine the module
//! runs (crates/relay), driven here on the computer by commands at the seconds a person or the automaton would give them. Expected from what a boiler's organ does - closed on a command, open
//! at start whatever it was, open by itself when its limit runs out, the limit counting from the last closing - and from the first burner's board, which says the same - not from the code
//! that makes the values.

// the critical section of std the machine's channels lock under on the computer: only linked in when something names it
use critical_section as _;
use embassy_time as _;
use std::cell::Cell;
use std::time::Duration;

use burner_api::relays::{at, put_values, Organ, NAMES};
use page::dictionary::Value;
use relay::Relays;

const WITHIN: Duration = Duration::from_secs(1);
/// The limits in force, in ms: the auger's own setting (60 s by default), the igniter's the kindling's and a margin, the pump and the power none.
const LIMITS: [u32; 4] = [60_000, 90_000, 0, 0];

/// The organs on the computer: the machine of crates/relay, a clock a test sets, and what the machine wrote to its pins.
struct Rig {
    relays: Relays<4>,
    now: u32,
    written: Vec<(usize, bool)>,
}

impl Rig {
    fn up() -> Self {
        let relays = Relays::new();
        let mut written = Vec::new();
        relays.start(&mut |r, c| written.push((r, c)));
        for (r, limit) in LIMITS.iter().enumerate() {
            relays.set_limit(r, *limit);
        }
        Rig { relays, now: 0, written }
    }

    /// The clock at SECONDS, the limits that have run out opened.
    fn at(&mut self, seconds: f32) {
        self.now = (seconds * 1000.0) as u32;
        let written = &mut self.written;
        self.relays.expire(self.now, &mut |r, c| written.push((r, c)));
    }

    /// ASK made by a person - in a thread of its own - while this thread is the owner at the clock.
    fn ask(&mut self, ask: impl FnOnce(&Relays<4>) + Send) {
        let (now, announced) = (self.now, Cell::new(0));
        let (relays, written) = (&self.relays, &mut self.written);
        std::thread::scope(|s| {
            let asker = s.spawn(|| ask(relays));
            while !asker.is_finished() {
                relays.serve_for(Duration::from_millis(10), &|| now, &|r| LIMITS[r], &mut |r, c| written.push((r, c)), &|| announced.set(announced.get() + 1));
            }
            asker.join().unwrap();
        });
    }

    fn on(&mut self, organ: &str) {
        let at = NAMES.iter().position(|n| *n == organ).unwrap();
        self.ask(move |r| r.set(at, true, WITHIN).unwrap());
    }

    fn off(&mut self, organ: &str) {
        let at = NAMES.iter().position(|n| *n == organ).unwrap();
        self.ask(move |r| r.set(at, false, WITHIN).unwrap());
    }

    fn toggle(&mut self, organ: &str) {
        let at = NAMES.iter().position(|n| *n == organ).unwrap();
        self.ask(move |r| r.toggle(at, WITHIN).unwrap());
    }

    /// What the stream is told of the organs now.
    fn said(&self) -> Vec<(usize, Value)> {
        let organs: [Organ; 4] = std::array::from_fn(|r| Organ { closed: self.relays.closed(r), left_s: self.relays.left_ms(r, self.now).map(|ms| ms.div_ceil(1000)) });
        let mut out = Vec::new();
        put_values(&organs, &mut |at, value| out.push((at, value)));
        out
    }

    fn value(&self, at: usize) -> Value {
        self.said().iter().find(|(a, _)| *a == at).map(|(_, v)| *v).unwrap_or_else(|| panic!("nothing said at {at}"))
    }
}

#[test]
fn at_start_every_organ_is_off_whatever_it_was_before_and_none_counts_down() {
    let rig = Rig::up();
    assert_eq!(rig.written, vec![(0, false), (1, false), (2, false), (3, false)], "the pins were not all written open at start");
    for organ in 0..4 {
        assert_eq!(rig.value(at::RELAY[organ]), Value::On(false), "{}", NAMES[organ]);
    }
    for off_in in at::RELAY_OFF_IN.into_iter().flatten() {
        assert_eq!(rig.value(off_in), Value::None);
    }
}

#[test]
fn a_command_closes_its_organ_and_the_augers_limit_counts_down_from_the_closing() {
    let mut rig = Rig::up();
    rig.at(5.0);
    rig.on("auger");
    assert_eq!(rig.value(at::RELAY[0]), Value::On(true));
    assert_eq!(rig.value(at::RELAY_OFF_IN[0].unwrap()), Value::Seconds(60));
    rig.at(15.0);
    assert_eq!(rig.value(at::RELAY_OFF_IN[0].unwrap()), Value::Seconds(50));
    rig.at(64.2);
    assert_eq!(rig.value(at::RELAY_OFF_IN[0].unwrap()), Value::Seconds(1), "the seconds left are rounded up: a limit with a tenth left is not yet zero");
}

#[test]
fn the_limit_opens_the_auger_by_itself_and_the_countdown_goes_with_it() {
    let mut rig = Rig::up();
    rig.on("auger");
    rig.at(60.0);
    assert_eq!(rig.value(at::RELAY[0]), Value::On(false));
    assert_eq!(rig.value(at::RELAY_OFF_IN[0].unwrap()), Value::None);
    assert_eq!(rig.written.last(), Some(&(0, false)), "the limit did not write the pin open");
}

#[test]
fn the_igniter_has_a_limit_of_its_own_and_the_pump_and_the_power_have_none() {
    let mut rig = Rig::up();
    rig.on("igniter");
    rig.on("pump");
    rig.on("aux_power");
    assert_eq!(rig.value(at::RELAY_OFF_IN[1].unwrap()), Value::Seconds(90));
    rig.at(3600.0);
    assert_eq!(rig.value(at::RELAY[1]), Value::On(false));
    assert_eq!((rig.value(at::RELAY[2]), rig.value(at::RELAY[3])), (Value::On(true), Value::On(true)), "an organ with no limit went off by itself");
}

#[test]
fn closing_a_closed_organ_starts_its_limit_anew_and_a_toggle_opens_it() {
    let mut rig = Rig::up();
    rig.on("auger");
    rig.at(40.0);
    rig.on("auger");
    assert_eq!(rig.value(at::RELAY_OFF_IN[0].unwrap()), Value::Seconds(60), "a second On did not start the limit anew");
    rig.toggle("auger");
    assert_eq!(rig.value(at::RELAY[0]), Value::On(false));
    rig.toggle("auger");
    assert_eq!(rig.value(at::RELAY[0]), Value::On(true));
    rig.off("auger");
    assert_eq!(rig.value(at::RELAY[0]), Value::On(false));
}

#[test]
fn an_organ_named_by_nobody_is_not_one_of_the_four() {
    assert_eq!(NAMES, ["auger", "igniter", "pump", "aux_power"]);
}
