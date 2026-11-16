//! The relays of a board and the one owner that switches them. A wall switch, the page, the console and the automaton
//! never touch a relay's pin: they ask at the owner's desk (between::Desk) and wait for its answer, which comes once
//! the relay is switched and the change announced. A full queue or a silent owner refuses the command by a code
//! instead of blocking whoever asked.
//!
//! The relays switch mains: at start every relay is open, and a relay moves only on a command, its contacts open
//! whatever the pin was before the restart. A relay may carry a limit: closed, it
//! opens by itself once the limit has run, so a forgotten button does not keep a heater on. The limit counts from
//! the last command that closed it.
//!
//! Time is whatever the caller counts in milliseconds since start; it is kept in 32 bits and wraps after 49 days,
//! which a limit never comes near.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use between::Desk;

/// The relays on the chip's pins and their owner's task: built for the board only.
#[cfg(target_os = "espidf")]
pub mod board;

/// Commands waiting for the owner at most. A hand on a wall key or on the page is never this fast; a full queue
/// means the owner stands, and asking more would only pile up.
pub const QUEUE: usize = 4;

/// Askers waiting for an answer at once: a person (the page or a console), the automaton, a switch on the wall.
pub const PLACES: usize = 3;

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// No relay of that number on this board.
    NoSuchRelay(usize),
    /// QUEUE commands wait already.
    Busy,
    /// The owner did not answer within the asker's limit.
    Silent,
}

impl core::fmt::Display for Refused {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Refused::NoSuchRelay(n) => write!(f, "no relay {n} on this board"),
            Refused::Busy => write!(f, "the relays are busy: {QUEUE} commands wait already, this one is refused"),
            Refused::Silent => write!(f, "the relays' owner did not answer in time"),
        }
    }
}

/// What a command asks of a relay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    Toggle(usize),
    Set(usize, bool),
    /// Look at the feeding relays now: what the automaton told has changed what they are to do.
    Look,
}

/// Opens the pins of a relay board so that none closes for a moment on the way: the open level is written to each
/// pin first, and only then is the pin made an output. After a reset the output latch of a pin holds low, and a
/// relay closed by low would close for as long as it takes to write the level afterwards. WRITE (pin, high) sets a
/// pin's level, OUTPUT (pin) makes it an output; CLOSED_ON_LOW says which level closes the relays.
pub fn open_pins(pins: usize, closed_on_low: bool, write: &mut impl FnMut(usize, bool), output: &mut impl FnMut(usize)) {
    for pin in 0..pins {
        write(pin, level(false, closed_on_low));
    }
    for pin in 0..pins {
        output(pin);
    }
}

/// The level of a pin that holds its relay CLOSED, high or low.
pub fn level(closed: bool, closed_on_low: bool) -> bool {
    closed != closed_on_low
}

/// What a relay says of itself to a person: off, on, or on with the time its limit leaves, in minutes and seconds
/// rounded up - "on - off by itself in 4:12".
pub fn words(closed: bool, left_ms: Option<u32>) -> String {
    match (closed, left_ms) {
        (false, _) => "off".into(),
        (true, None) => "on".into(),
        (true, Some(ms)) => {
            let s = ms.div_ceil(1000);
            format!("on - off by itself in {}:{:02}", s / 60, s % 60)
        }
    }
}

/// How long a plan of the automaton holds, in ms, counted from the moment the relay's task gets it: a plan whose end has come stands for nothing and the relay is open, and nothing extends it - only a new plan, which the automaton builds
/// every second from its own count. The number is the longest the automaton can stand between two plans, by the code: after the word to the auger a turn of its loop can wait for the relays' task twice (1 s each, `board::WITHIN`: the igniter, the pump),
/// for the fan 2 s (`modules/fan.rs`), for the igniter's guard 1 s, for the flash 3 s (`flash_owner::WITHIN`), and for the word of the auger itself 1 s, nine in all; ten leave a second over for the scheduler. A silent automaton leaves the auger
/// going for ten seconds at the most, and only in what it had planned.
pub const HORIZON_MS: u32 = 10_000;

/// What a relay that feeds is to do from the moment it is told, as the automaton plans it: it is the automaton that knows where it stands in a dose and in a rhythm, so a plan says that, and the relay's task keeps nothing of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feeding {
    /// Open. What a relay with no plan, or with a plan that has run out, does.
    Stop,
    /// Closed for LEFT_MS from now, then open: what is left of a dose by the automaton's count.
    Dose { left_ms: u32 },
    /// Cycles of ON_MS closed and OFF_MS open, the cycle standing INTO_MS into itself now.
    Rhythm { into_ms: u32, on_ms: u32, off_ms: u32 },
}

impl Feeding {
    /// Whether the plan has the relay closed REL_MS after it came.
    fn closed_after(self, rel_ms: u32) -> bool {
        match self {
            Feeding::Stop => false,
            Feeding::Dose { left_ms } => rel_ms < left_ms,
            Feeding::Rhythm { into_ms, on_ms, off_ms } => {
                let (on, cycle) = (u64::from(on_ms.max(1)), u64::from(on_ms.max(1)) + u64::from(off_ms.max(1)));
                (u64::from(into_ms) + u64::from(rel_ms)) % cycle < on
            }
        }
    }

    /// How long after REL_MS the plan changes what it has the relay do; None when it never will.
    fn edge_after(self, rel_ms: u32) -> Option<u32> {
        match self {
            Feeding::Stop => None,
            Feeding::Dose { left_ms } => (rel_ms < left_ms).then(|| left_ms - rel_ms),
            Feeding::Rhythm { into_ms, on_ms, off_ms } => {
                let (on, cycle) = (u64::from(on_ms.max(1)), u64::from(on_ms.max(1)) + u64::from(off_ms.max(1)));
                let at = (u64::from(into_ms) + u64::from(rel_ms)) % cycle;
                Some((if at < on { on - at } else { cycle - at }) as u32)
            }
        }
    }
}

/// The plan a relay holds, and where it came from.
#[derive(Clone, Copy)]
struct Feed {
    plan: Feeding,
    /// When the plan came and when it runs out, in the caller's ms.
    from: u32,
    end: u32,
    /// The plan came as a command: the relay goes to the state the plan has it in, whatever a hand left it in.
    retake: bool,
    /// A plan came after the end of the one before and before the task had looked: the end is honoured first, the relay opens, and the new plan follows from there.
    end_first: bool,
    /// What the plan had the relay do when the task last looked: the relay is written when this changes, not at every look, so a hand's switch holds to the next edge.
    last: bool,
}

impl Feed {
    const OPEN: Feed = Feed { plan: Feeding::Stop, from: 0, end: 0, retake: false, end_first: false, last: false };
}

/// Whether the time END has come at NOW, over the wrap of the 32 bits.
fn due(now_ms: u32, end: u32) -> bool {
    now_ms.wrapping_sub(end) as i32 >= 0
}

/// How long since FROM, in ms, over the wrap of the 32 bits; 0 for a time ahead of NOW_MS.
fn since(now_ms: u32, from: u32) -> u32 {
    (now_ms.wrapping_sub(from) as i32).max(0) as u32
}

/// What is said to the relay of an organ that feeds: a word that commands it, or one that only confirms the word before (the automaton gives it every second, and a hand's switch is not taken back by it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Told {
    Say(Feeding),
    Still(Feeding),
}

pub struct Relays<const N: usize> {
    desk: Desk<Order, (), QUEUE, PLACES>,
    closed: [AtomicBool; N],
    /// How long a relay may stay closed, in ms; 0 - as long as it is told.
    limit_ms: [AtomicU32; N],
    /// When the relay was last closed, in the caller's ms.
    closed_at: [AtomicU32; N],
    /// The portions of the relays that are told a rhythm.
    feeds: Mutex<[Feed; N]>,
    /// Schedules cut off for want of a word.
    cuts: AtomicU32,
    /// The switches since they were last taken (`take_switches`).
    switches: Mutex<Switches>,
}

/// How many switches are kept between two takings: the fan and the plug and the auger at a start are a handful, and the auger's portions come four to a minute at most.
const SWITCHES: usize = 8;

/// The relays that really changed since the last taking, in order, and how many more there were than fit.
struct Switches {
    kept: [(u8, bool); SWITCHES],
    len: usize,
    dropped: u32,
}

impl Switches {
    const EMPTY: Self = Self { kept: [(0, false); SWITCHES], len: 0, dropped: 0 };
}

impl<const N: usize> Relays<N> {
    pub const fn new() -> Self {
        Self {
            switches: Mutex::new(Switches::EMPTY),
            desk: Desk::new(),
            closed: [const { AtomicBool::new(false) }; N],
            limit_ms: [const { AtomicU32::new(0) }; N],
            closed_at: [const { AtomicU32::new(0) }; N],
            feeds: Mutex::new([Feed::OPEN; N]),
            cuts: AtomicU32::new(0),
        }
    }

    /// Hands EACH the relays that really changed since the last taking, as (relay, closed), in the order they changed, and forgets them; how many more changed than the notes hold.
    pub fn take_switches(&self, each: &mut impl FnMut(usize, bool)) -> u32 {
        let mut switches = self.switches.lock().unwrap_or_else(|e| e.into_inner());
        for (relay, closed) in &switches.kept[..switches.len] {
            each(usize::from(*relay), *closed);
        }
        let dropped = switches.dropped;
        *switches = Switches::EMPTY;
        dropped
    }

    /// RELAY really changed to CLOSED: noted for [`Relays::take_switches`], or counted if the notes are full. The relay is written before this is called, and a relay that was told what it already was is not noted.
    fn note(&self, relay: usize, closed: bool) {
        let mut switches = self.switches.lock().unwrap_or_else(|e| e.into_inner());
        if switches.len < SWITCHES {
            let at = switches.len;
            switches.kept[at] = (relay as u8, closed);
            switches.len += 1;
        } else {
            switches.dropped += 1;
        }
    }

    /// Opens every relay through WRITE (relay, closed): the first thing the owner does, before any command.
    pub fn start(&self, write: &mut impl FnMut(usize, bool)) {
        for (relay, closed) in self.closed.iter().enumerate() {
            closed.store(false, Ordering::Relaxed);
            write(relay, false);
        }
    }

    fn ask(&self, order: Order, relay: usize, within: Duration) -> Result<(), Refused> {
        if relay >= N {
            return Err(Refused::NoSuchRelay(relay));
        }
        self.desk.ask(order, within).map_err(|refused| match refused {
            between::Refused::Busy => Refused::Busy,
            between::Refused::Silent => Refused::Silent,
        })
    }

    /// Asks the owner to switch relay RELAY over, and waits up to WITHIN for it to be done.
    pub fn toggle(&self, relay: usize, within: Duration) -> Result<(), Refused> {
        self.ask(Order::Toggle(relay), relay, within)
    }

    /// Asks the owner to close or open relay RELAY, and waits up to WITHIN for it to be done. Closing a closed relay
    /// starts its limit anew.
    pub fn set(&self, relay: usize, closed: bool, within: Duration) -> Result<(), Refused> {
        self.ask(Order::Set(relay, closed), relay, within)
    }

    /// Lets relay RELAY stay closed LIMIT_MS at most; 0 lifts the limit. A relay closed now keeps its start: a limit
    /// shorter than it has been closed opens it on the owner's next look.
    pub fn set_limit(&self, relay: usize, limit_ms: u32) {
        if let Some(limit) = self.limit_ms.get(relay) {
            limit.store(limit_ms, Ordering::Relaxed);
        }
    }

    /// How long relay RELAY stays closed before its limit opens it, at NOW_MS; None when it is open or has no limit.
    pub fn left_ms(&self, relay: usize, now_ms: u32) -> Option<u32> {
        if !self.closed(relay) {
            return None;
        }
        let limit = self.limit_ms[relay].load(Ordering::Relaxed);
        if limit == 0 {
            return None;
        }
        let closed_for = now_ms.wrapping_sub(self.closed_at[relay].load(Ordering::Relaxed));
        Some(limit.saturating_sub(closed_for))
    }

    /// How long the owner may wait for a command at NOW_MS before a limit runs out, a portion ends or a word's lease does; None when none runs.
    pub fn wait_ms(&self, now_ms: u32) -> Option<u32> {
        let limits = (0..N).filter_map(|relay| self.left_ms(relay, now_ms)).min();
        match (limits, self.feed_wait_ms(now_ms)) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Opens through WRITE every relay whose limit has run out at NOW_MS; whether any was opened.
    pub fn expire(&self, now_ms: u32, write: &mut impl FnMut(usize, bool)) -> bool {
        let mut opened = false;
        for relay in 0..N {
            if self.left_ms(relay, now_ms) == Some(0) {
                self.closed[relay].store(false, Ordering::Relaxed);
                write(relay, false);
                self.note(relay, false);
                opened = true;
            }
        }
        opened
    }

    /// Carries out the commands asked for LEFT, each at the time NOW gives: switches the relay through WRITE, then
    /// calls ANNOUNCE, then answers its asker. A command that closes a relay takes its limit from LIMIT_MS (relay) at
    /// that moment, so a limit changed just before counts from the first second.
    pub fn serve_for(
        &self,
        left: Duration,
        now: &impl Fn() -> u32,
        limit_ms: &impl Fn(usize) -> u32,
        write: &mut impl FnMut(usize, bool),
        announce: &impl Fn(),
    ) {
        self.desk.serve_for(left, |order| {
            let (relay, closed) = match order {
                Order::Toggle(relay) => (relay, !self.closed[relay].load(Ordering::Relaxed)),
                Order::Set(relay, closed) => (relay, closed),
                Order::Look => {
                    if self.step(now(), write) {
                        announce();
                    }
                    return;
                }
            };
            if closed {
                self.limit_ms[relay].store(limit_ms(relay), Ordering::Relaxed);
                self.closed_at[relay].store(now(), Ordering::Relaxed);
            }
            // only the owner writes: the state and the pin cannot part
            let was = self.closed[relay].swap(closed, Ordering::Relaxed);
            write(relay, closed);
            if was != closed {
                self.note(relay, closed);
            }
            announce();
        });
    }

    /// Hands relay RELAY a plan at NOW_MS as a COMMAND: the decision is new, or the automaton takes the organ back from a hand, so the relay goes to the state the plan has it in at the next look ([`Relays::step`]).
    /// Whether the previous plan had run out before this one came - the caller then has the task look now, so that the end is honoured before the plan follows.
    pub fn feed(&self, relay: usize, feeding: Feeding, now_ms: u32) -> bool {
        self.plan(relay, feeding, now_ms, true)
    }

    /// Hands relay RELAY a plan at NOW_MS for the decision that stands: the automaton builds one every second. It takes nothing back from a hand, and extends nothing: the plan replaces the one before, which counted
    /// from its own moment and its own end. Whether the previous plan had run out before this one came (as [`Relays::feed`]).
    pub fn still(&self, relay: usize, feeding: Feeding, now_ms: u32) -> bool {
        self.plan(relay, feeding, now_ms, false)
    }

    fn plan(&self, relay: usize, feeding: Feeding, now_ms: u32, command: bool) -> bool {
        let mut feeds = self.feeds.lock().unwrap_or_else(|e| e.into_inner());
        let Some(f) = feeds.get_mut(relay) else { return false };
        let ran_out = f.plan != Feeding::Stop && due(now_ms, f.end);
        if ran_out {
            self.cuts.fetch_add(1, Ordering::Relaxed);
            f.end_first = true;
        }
        (f.plan, f.from, f.end) = (feeding, now_ms, now_ms.wrapping_add(HORIZON_MS));
        f.retake |= command;
        ran_out
    }

    /// How many times a plan ran out with no new one since start: the task says it once in its journal for each.
    pub fn cuts(&self) -> u32 {
        self.cuts.load(Ordering::Relaxed)
    }

    /// Carries out the plans at NOW_MS through WRITE (relay, closed): a plan whose end has come is over and the relay opens; the relay is written when what its plan has it do changes, or at once for a command.
    /// Whether any relay was written.
    pub fn step(&self, now_ms: u32, write: &mut impl FnMut(usize, bool)) -> bool {
        let mut feeds = self.feeds.lock().unwrap_or_else(|e| e.into_inner());
        let mut wrote = false;
        for (relay, f) in feeds.iter_mut().enumerate() {
            let mut set = |closed: bool| {
                if self.closed[relay].load(Ordering::Relaxed) != closed {
                    if closed {
                        self.closed_at[relay].store(now_ms, Ordering::Relaxed);
                    }
                    self.closed[relay].store(closed, Ordering::Relaxed);
                    write(relay, closed);
                    self.note(relay, closed);
                    wrote = true;
                }
            };
            if f.end_first {
                f.end_first = false;
                set(false);
                f.last = false;
            }
            // **A PLAN THAT HAS RUN OUT STANDS FOR NOTHING**: the automaton has not planned, and the relay is open whatever the plan had it do
            if f.plan != Feeding::Stop && due(now_ms, f.end) {
                self.cuts.fetch_add(1, Ordering::Relaxed);
                f.plan = Feeding::Stop;
            }
            let wanted = f.plan.closed_after(since(now_ms, f.from));
            if f.retake || wanted != f.last {
                f.retake = false;
                f.last = wanted;
                set(wanted);
            }
        }
        wrote
    }

    /// How long until the next thing a plan does at NOW_MS - an edge or its end; None when no relay has a plan. 0 for a command not yet looked at.
    fn feed_wait_ms(&self, now_ms: u32) -> Option<u32> {
        let feeds = self.feeds.lock().unwrap_or_else(|e| e.into_inner());
        feeds
            .iter()
            .filter_map(|f| match f.plan {
                _ if f.retake || f.end_first => Some(0),
                Feeding::Stop => None,
                plan => {
                    let to_end = (f.end.wrapping_sub(now_ms) as i32).max(0) as u32;
                    Some(plan.edge_after(since(now_ms, f.from)).map_or(to_end, |edge| edge.min(to_end)))
                }
            })
            .min()
    }

    /// Asks the owner to look at the schedules now, and waits up to WITHIN for it to have done so.
    pub fn look(&self, within: Duration) -> Result<(), Refused> {
        self.ask(Order::Look, 0, within)
    }

    pub fn closed(&self, relay: usize) -> bool {
        self.closed.get(relay).is_some_and(|c| c.load(Ordering::Relaxed))
    }
}

impl<const N: usize> Default for Relays<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// The rows of this module in a device's dictionary (`page::dictionary`): what a client reads of it and what a person may do. The card it stands on and its name are the device's.
pub mod entry {
    use page::dictionary::{Entry, Kind};

    /// The row of the relay NAME on the card CARD: on or off, and a person may turn it on, turn it off or turn it over.
    pub const fn row(card: &'static str, name: &'static str) -> Entry {
        Entry { can: &["switch", "toggle"], ..Entry::row(card, name, &[Kind::On]) }
    }
}
