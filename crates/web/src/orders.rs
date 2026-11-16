//! What the network's loop hands over rather than does itself: every request whose answer waits on another task -
//! a command to a module, a saved setting, a curve, a line typed into a console. Its owner is the task `orders`;
//! the loop asks on its desk and awaits the answer, so a command that waits three seconds on a deciding module holds
//! that one connection and nobody else.
//!
//! **A LONG ORDER IN HAND IS SAID AT ONCE, NOT WAITED OUT.** `bus scan` holds the owner eighteen seconds; a command
//! from the page queued behind it would wait all of that and come back refused anyway. So an asker whose order is not
//! the one in hand looks at how long that one has been there: past [`LONG`] it is refused `Busy` at once, and while it
//! waits in the queue it keeps looking.
//!
//! A console's output does not come back as an answer - `bus scan` writes three hundred lines as it goes. It goes
//! into the console's own pipe, which the loop drains into the socket.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use between::{Desk, Lock, Refused};
use embassy_futures::select::{select, Either};
use embassy_sync::pipe::Pipe;
use embassy_time::Timer;

/// Consoles at once: two in browsers, two on TCP port 23, and the one on USB.
pub const CONSOLES: usize = 5;
/// The console places of the browsers come first, then the port's, then USB's.
pub const BROWSER_CONSOLES: usize = 2;
/// Consoles on TCP port 23.
pub const PORT_CONSOLES: usize = 2;
/// The place of the console on USB, which the loop never takes: its bridge does (firmware::console).
pub const USB: usize = BROWSER_CONSOLES + PORT_CONSOLES;
/// What a console holds of the shell's output that its socket has not taken yet.
pub const PIPE: usize = 1024;
/// The bytes fed to a console's shell at once: a line and the return after it.
pub const LINE: usize = shell::MAX_LINE + 1;
/// How long another order may have been in hand before an asker is refused at once.
pub const LONG: Duration = Duration::from_millis(500);
/// How often a queued asker looks at the order in hand.
const LOOK: Duration = Duration::from_millis(50);

/// What the loop asks of the owner.
#[derive(Debug)]
pub enum Order {
    /// A command to the thing NAME, the dictionary having found it by its address.
    Act { name: &'static str, command: page::dictionary::Command },
    /// The tab Settings' form.
    Save(String),
    /// The curve called so.
    Curve(String),
    /// The lines of the journal on flash not written yet.
    Waiting,
    /// The end of the journal as the board holds it in memory.
    Tail,
    /// A console opened at this place: a shell of its own, its prompt into the pipe.
    Open(usize),
    /// Bytes typed into the console at this place.
    Feed(usize, heapless::Vec<u8, LINE>),
}

/// What the owner answers.
#[derive(Debug)]
pub enum Answer {
    Done,
    Refused(String),
    Json(serde_json::Value),
    Curve(Option<page::Curve>),
    /// The lines of the journal not on the flash yet; None - the board keeps no journal.
    Waiting(Option<String>),
    /// The end of the journal in memory; None - the board keeps no journal.
    Tail(Option<journal::kept::Tail>),
}

/// What carries the orders out, in the owner's task: the firmware's, and a record in the tests.
pub trait Hands {
    fn carry(&mut self, order: Order, out: &Orders) -> Answer;
}

struct Asked {
    ticket: u32,
    order: Order,
}

/// The owner's desk and what the loop reads about it.
pub struct Orders {
    desk: Desk<Asked, Answer, 4, 4>,
    tickets: AtomicU32,
    /// The ticket of the order in hand, 0 when none is.
    in_hand: AtomicU32,
    /// When it was taken, milliseconds of embassy-time, truncated.
    since: AtomicU32,
    pipes: [Pipe<Lock, PIPE>; CONSOLES],
    /// Whether the console at a place has gone: what the owner writes for it is dropped. One writer, the loop.
    gone: [AtomicBool; CONSOLES],
}

impl Default for Orders {
    fn default() -> Self {
        Self::new()
    }
}

impl Orders {
    pub const fn new() -> Self {
        Self {
            desk: Desk::new(),
            tickets: AtomicU32::new(1),
            in_hand: AtomicU32::new(0),
            since: AtomicU32::new(0),
            pipes: [const { Pipe::new() }; CONSOLES],
            gone: [const { AtomicBool::new(true) }; CONSOLES],
        }
    }

    /// Asks ORDER and waits for its answer up to WITHIN, refused `Busy` at once when another order has been in hand
    /// longer than [`LONG`].
    pub async fn ask(&self, order: Order, within: Duration) -> Result<Answer, Refused> {
        let ticket = loop {
            let t = self.tickets.fetch_add(1, Ordering::Relaxed);
            if t != 0 {
                break t;
            }
        };
        if self.long_other_than(ticket) {
            return Err(Refused::Busy);
        }
        let asked = self.desk.asked(Asked { ticket, order }, within);
        let watching = async {
            loop {
                Timer::after(time(LOOK)).await;
                if self.long_other_than(ticket) {
                    return;
                }
            }
        };
        match select(asked, watching).await {
            Either::First(answer) => answer,
            Either::Second(()) => Err(Refused::Busy),
        }
    }

    fn long_other_than(&self, ticket: u32) -> bool {
        let held = self.in_hand.load(Ordering::Acquire);
        if held == 0 || held == ticket {
            return false;
        }
        let now = between::clock::now().as_millis() as u32;
        now.wrapping_sub(self.since.load(Ordering::Acquire)) > LONG.as_millis() as u32
    }

    /// Serves the desk for good, each order by HANDS: the body of the task `orders`.
    pub fn serve(&self, hands: &mut impl Hands) -> ! {
        loop {
            self.desk.serve_for(Duration::from_secs(3600), |asked| {
                self.since.store(between::clock::now().as_millis() as u32, Ordering::Release);
                self.in_hand.store(asked.ticket, Ordering::Release);
                let answer = hands.carry(asked.order, self);
                self.in_hand.store(0, Ordering::Release);
                answer
            });
        }
    }

    /// The console at PLACE opened: its pipe emptied of what the last one left, its output taken again.
    pub fn opened(&self, place: usize) {
        self.pipes[place].clear();
        self.gone[place].store(false, Ordering::Release);
    }

    /// The console at PLACE has gone: what is written for it from now on is dropped.
    pub fn closed(&self, place: usize) {
        self.gone[place].store(true, Ordering::Release);
    }

    /// Writes BYTES for the console at PLACE, from the owner's task: waits while the pipe is full, up to a second -
    /// a console whose socket takes nothing for that long loses the rest rather than holding the owner.
    pub fn say(&self, place: usize, bytes: &[u8]) {
        let pipe = &self.pipes[place];
        let mut left = bytes;
        while !left.is_empty() && !self.gone[place].load(Ordering::Acquire) {
            let wrote = between::block_on(embassy_time::with_timeout(time(Duration::from_secs(1)), pipe.write(left)));
            match wrote {
                Ok(n) => left = &left[n..],
                Err(_) => return,
            }
        }
    }

    /// Writes WORDS for the console at PLACE from the loop itself - a refusal said to it - into the same pipe, so they
    /// come out between the shell's lines and not through them.
    pub async fn tell(&self, place: usize, words: &[u8]) {
        let _ = embassy_time::with_timeout(time(Duration::from_secs(1)), self.pipes[place].write_all(words)).await;
    }

    /// What the owner wrote for the console at PLACE, awaited: bytes into BUF, their count.
    pub async fn heard(&self, place: usize, buf: &mut [u8]) -> usize {
        self.pipes[place].read(buf).await
    }
}

/// A console's writer for the shell: the owner's bytes into the console's pipe.
pub struct Said<'a> {
    pub orders: &'a Orders,
    pub place: usize,
}

impl embedded_io::ErrorType for Said<'_> {
    type Error = core::convert::Infallible;
}

impl embedded_io::Write for Said<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.orders.say(self.place, buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub(crate) fn time(d: Duration) -> embassy_time::Duration {
    embassy_time::Duration::from_micros(d.as_micros() as u64)
}
