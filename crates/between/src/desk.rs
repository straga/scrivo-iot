//! An owner's desk: where another task asks the owner of some state to do something with it, and
//! waits for the answer. The owner is the only one that touches the state; everybody else asks.

use core::time::Duration;

use core::sync::atomic::{AtomicU32, Ordering};

use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{with_deadline, Instant, Timer};

use crate::Lock;

/// Why an asking was not answered - a code, turned into words where it leaves the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The owner's queue is full: it stands, and asking more would only pile up.
    Busy,
    /// No answer within the limit the asker gave.
    Silent,
}

impl core::fmt::Display for Refused {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Refused::Busy => "owner_busy",
            Refused::Silent => "owner_silent",
        })
    }
}

struct Asked<Q> {
    /// Where the answer goes.
    place: usize,
    /// Which asking of that place it answers: a place is taken again after its asker gave up, and the answer to the
    /// one who gave up must not reach the one after.
    number: u32,
    what: Q,
}

/// The desk: LEN requests waiting at most, PLACES askers waiting for an answer at once.
///
/// **THE OWNER'S STATE IS TOUCHED IN THE OWNER'S TASK AND NOWHERE ELSE.** An asker hands over what it wants and
/// waits; the owner carries requests out one after another between its own rounds, so two commands never meet
/// half way and a command never meets the owner's own work - with no lock over the state at all.
///
/// Every place is a `Signal`, and a place is an asker waiting now, not a task that may ask: an asker that finds all
/// taken waits for one inside its own limit.
///
/// **A REQUEST WHOSE ASKER GAVE UP IS NOT CARRIED OUT**: a person told "no answer" must not find the relay switched a
/// second later. One window stays, and it is said here rather than hidden: a request the owner has already taken in
/// hand when the limit runs out is carried out, and its answer is lost. The asker does not wait past its limit for it,
/// because an owner that has stuck would take the asker down with it.
pub struct Desk<Q, A, const LEN: usize, const PLACES: usize> {
    queue: Channel<Lock, Asked<Q>, LEN>,
    places: [Signal<Lock, (u32, A)>; PLACES],
    /// Per place: `FREE`, the number of the asking waiting there, or that number with `IN_HAND` once the owner took it.
    state: [AtomicU32; PLACES],
    numbers: AtomicU32,
}

const FREE: u32 = 0x8000_0000;
const IN_HAND: u32 = 0x4000_0000;
/// The numbers of askings run under both marks and wrap there.
const NUMBER: u32 = IN_HAND - 1;

/// How often an asker that found every place taken looks again.
const PLACE_LOOK: Duration = Duration::from_millis(5);

impl<Q: Send, A: Send, const LEN: usize, const PLACES: usize> Desk<Q, A, LEN, PLACES> {
    pub const fn new() -> Self {
        Self {
            queue: Channel::new(),
            places: [const { Signal::new() }; PLACES],
            state: [const { AtomicU32::new(FREE) }; PLACES],
            numbers: AtomicU32::new(0),
        }
    }

    /// Asks the owner WHAT and waits for its answer up to WITHIN, asleep: the answer, or why there is none.
    pub fn ask(&self, what: Q, within: Duration) -> Result<A, Refused> {
        crate::block_on(self.asked(what, within))
    }

    /// The same, for an asker that is itself a task of an executor.
    pub async fn asked(&self, what: Q, within: Duration) -> Result<A, Refused> {
        let until = Instant::now() + time(within);
        let number = self.numbers.fetch_add(1, Ordering::Relaxed) & NUMBER;
        let place = loop {
            if let Some(place) = self.take_place(number) {
                break place;
            }
            if Instant::now() + time(PLACE_LOOK) > until {
                return Err(Refused::Silent);
            }
            Timer::after(time(PLACE_LOOK)).await;
        };
        let answer = self.wait_at(place, number, what, until).await;
        // given up or answered, the place is free again; a request of this number still queued is skipped
        self.state[place].store(FREE, Ordering::Release);
        answer
    }

    fn take_place(&self, number: u32) -> Option<usize> {
        self.state
            .iter()
            .position(|s| s.compare_exchange(FREE, number, Ordering::AcqRel, Ordering::Relaxed).is_ok())
    }

    async fn wait_at(&self, place: usize, number: u32, what: Q, until: Instant) -> Result<A, Refused> {
        let signal = &self.places[place];
        // what the place's last asker left there is not this asking's
        signal.reset();
        self.queue.try_send(Asked { place, number, what }).map_err(|_| Refused::Busy)?;
        let answered = async {
            loop {
                let (to, answer) = signal.wait().await;
                if to == number {
                    return answer;
                }
            }
        };
        with_deadline(until, answered).await.map_err(|_| Refused::Silent)
    }

    /// Answers what is asked for LEFT, each by EACH, in the owner's own task, asleep in between: what the owner does
    /// instead of sleeping to the start of its next round.
    pub fn serve_for(&self, left: Duration, each: impl FnMut(Q) -> A) {
        crate::block_on(self.served_for(left, each))
    }

    /// The same, for an owner that is itself a task of an executor.
    pub async fn served_for(&self, left: Duration, mut each: impl FnMut(Q) -> A) {
        let until = Instant::now() + time(left);
        let serving = async {
            loop {
                let asked = self.queue.receive().await;
                let waiting = &self.state[asked.place];
                let in_hand =
                    waiting.compare_exchange(asked.number, asked.number | IN_HAND, Ordering::AcqRel, Ordering::Relaxed);
                if in_hand.is_err() {
                    continue; // its asker gave up: not carried out
                }
                let answer = each(asked.what);
                self.places[asked.place].signal((asked.number, answer));
            }
        };
        let _ = with_deadline(until, serving).await;
    }
}

/// A span of std's in embassy-time's ticks.
fn time(span: Duration) -> embassy_time::Duration {
    embassy_time::Duration::from_micros(span.as_micros().min(u128::from(u64::MAX)) as u64)
}

impl<Q: Send, A: Send, const LEN: usize, const PLACES: usize> Default for Desk<Q, A, LEN, PLACES> {
    fn default() -> Self {
        Self::new()
    }
}
