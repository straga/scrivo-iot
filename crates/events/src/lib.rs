//! What the board tells its subscribers, and when. A module announces the CARD of its own that changed; every
//! subscriber keeps its own set of marked cards and takes that set away when it sends, so what goes out is the
//! cards that changed and nothing else.
//!
//! **A COUNTER CANNOT CARRY THIS, AND THAT IS WHY THERE IS NONE.** A `Watch<u32>` whose `announce()`
//! bumps one number wakes every waiting stream with no way of knowing what had moved, so the only thing left
//! to send is the whole board - 7448 bytes ten times a second when the fan spins, for one number off one sensor.
//! A `Watch` holds ONE value as well, so two changes before a stream woke would lose the first of them; a set of
//! marks per subscriber collapses them instead, which is what "the same card
//! changed twice" should mean.
//!
//! A subscriber is not the same thing as an open page. There is room for pages and room beside them for a
//! subscriber that is not one - the sending outward this leaves a place for - and the two are counted
//! separately: `Changes<N>` has N places for marks, and how many of them are streams is the page server's
//! business, not this crate's.
//!
//! A subscriber is a future of the network's loop ([`Listener`]): it takes its marks when it sends, and awaits the
//! next announcement.

use core::sync::atomic::{AtomicU32, Ordering};

use between::Lock;
use embassy_sync::watch::{Receiver, Watch};

// A card is named by the page's contract, and the set of them lives there with it: this crate is the waking,
// not the naming. Kept out of here for a second reason too - the page's tests run on the host, and a crate that
// pulls embassy-time in has to answer to a time driver the host has not got.
pub use page::Marks;

/// Where a change is announced, with room for N subscribers at once.
pub struct Changes<const N: usize> {
    /// The waker and nothing else: what changed is in `marked`, one set per place.
    watch: Watch<Lock, u32, N>,
    /// The cards marked for the subscriber holding each place.
    marked: [AtomicU32; N],
    /// Which places are held, one bit each.
    places: AtomicU32,
}

impl<const N: usize> Changes<N> {
    pub const fn new() -> Self {
        assert!(N <= u32::BITS as usize, "more places than a bitmask of places has");
        Self { watch: Watch::new(), marked: [const { AtomicU32::new(0) }; N], places: AtomicU32::new(0) }
    }

    /// The cards WHAT names have changed: every subscriber is told so, and every one of them wakes now rather
    /// than on its next tick. Announcing nothing wakes nobody.
    pub fn announce(&self, what: Marks) {
        if !what.any() {
            return;
        }
        for one in &self.marked {
            one.fetch_or(what.bits(), Ordering::AcqRel);
        }
        // the number itself says nothing; bumping it is what wakes the places waiting on it
        self.watch.sender().send_modify(|n| *n = Some(n.map_or(1, |n| n.wrapping_add(1))));
    }

    /// Takes a place for one subscriber, marked with every card, or None when every place is held.
    fn subscribe(&self) -> Option<Place<'_, N>> {
        let mut taken = None;
        self.places
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |held| {
                let at = (0..N).find(|at| held & (1 << at) == 0)?;
                taken = Some(at);
                Some(held | (1 << at))
            })
            .ok()?;
        let at = taken?;
        // whole from the start: whatever was announced while the place was being taken is inside "everything"
        self.marked[at].store(Marks::ALL.bits(), Ordering::Release);
        let Some(mut ear) = self.watch.receiver() else {
            // the watch has fewer places to wait in than there are places to mark; keep neither
            self.release(at);
            return None;
        };
        // a change announced before this subscriber opened is already in the first state it sends
        ear.try_changed();
        Some(Place { changes: self, at, ear })
    }

    fn release(&self, at: usize) {
        self.marked[at].store(0, Ordering::Release);
        self.places.fetch_and(!(1 << at), Ordering::AcqRel);
    }
}

impl<const N: usize> Default for Changes<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// One subscriber's place: its own set of marks, and its own way of being woken.
struct Place<'a, const N: usize> {
    changes: &'a Changes<N>,
    at: usize,
    ear: Receiver<'a, Lock, u32, N>,
}

impl<const N: usize> Place<'_, N> {
    /// The cards marked for this subscriber; the set is cleared by the taking, because what is taken is being
    /// sent, and a change that lands after this is a mark again and a wake again.
    fn take(&self) -> Marks {
        Marks::of_bits(self.changes.marked[self.at].swap(0, Ordering::AcqRel))
    }
}

/// A subscriber that is itself a future of the network's loop: the same place, taken and waited on without a
/// thread of its own. Dropping it gives the place back.
pub struct Listener<'a, const N: usize>(Place<'a, N>);

impl<const N: usize> Changes<N> {
    /// A place for a subscriber in the loop, marked with every card, or None when every place is held.
    pub fn listen(&self) -> Option<Listener<'_, N>> {
        self.subscribe().map(Listener)
    }
}

impl<const N: usize> Listener<'_, N> {
    /// The cards marked since the last take; taking clears them, as a stream's send does.
    pub fn take(&self) -> Marks {
        self.0.take()
    }

    /// Waits for the next announcement.
    pub async fn changed(&mut self) {
        self.0.ear.changed().await;
    }
}

impl<const N: usize> Drop for Place<'_, N> {
    fn drop(&mut self) {
        self.changes.release(self.at);
    }
}
