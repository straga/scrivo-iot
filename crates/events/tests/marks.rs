//! What a subscriber is told, and not only that it was woken. A change announces WHICH card changed, every
//! subscriber keeps its own set of marked cards, and it takes its own set away when it sends - so two changes of
//! one card before a send are one card to send, and a card announced while another subscriber was asleep is not
//! lost to it.

use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use events::{Changes, Marks};

/// Three places on purpose: two for the pages a person may open and one for a subscriber that is not a page -
/// a sending outward.
const PLACES: usize = 3;

/// Opens a subscriber in its own thread, as the network's loop holds one: it sends its marks, then waits for the next
/// announcement or for EVERY to pass. The set of marks of each send comes out of the receiver.
fn open(changes: &'static Changes<PLACES>, every: Duration) -> Receiver<Marks> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let Some(mut listener) = changes.listen() else { return };
        loop {
            if tx.send(listener.take()).is_err() {
                return;
            }
            let within = embassy_time::Duration::from_micros(every.as_micros() as u64);
            let _ = between::block_on(embassy_time::with_timeout(within, listener.changed()));
        }
    });
    rx
}

fn took(rx: &Receiver<Marks>) -> Marks {
    rx.recv_timeout(Duration::from_secs(1)).expect("a send within a second")
}

#[test]
fn a_subscriber_starts_with_every_card_marked() {
    static OWN: Changes<PLACES> = Changes::new();
    let rx = open(&OWN, Duration::from_secs(5));
    assert_eq!(took(&rx), Marks::ALL, "the first send of a new subscriber is not the whole of it");
}

#[test]
fn a_card_announced_comes_as_that_card_and_nothing_else() {
    static OWN: Changes<PLACES> = Changes::new();
    let rx = open(&OWN, Duration::from_secs(5));
    took(&rx);
    std::thread::sleep(Duration::from_millis(50));
    OWN.announce(Marks::part(3));
    assert_eq!(took(&rx), Marks::part(3));
}

#[test]
fn two_changes_of_one_card_before_a_send_are_one_card_to_send() {
    static OWN: Changes<PLACES> = Changes::new();
    let listener = OWN.listen().expect("a free place");
    assert_eq!(listener.take(), Marks::ALL);
    OWN.announce(Marks::part(2));
    OWN.announce(Marks::part(2));
    assert_eq!(listener.take(), Marks::part(2), "one card announced twice came as something other than that one card");
    assert_eq!(listener.take(), Marks::NONE, "the second announcement of one card is to be sent again");
}

#[test]
fn two_cards_announced_before_a_send_come_together() {
    static OWN: Changes<PLACES> = Changes::new();
    let listener = OWN.listen().expect("a free place");
    listener.take();
    OWN.announce(Marks::part(0));
    OWN.announce(Marks::part(5));
    assert_eq!(listener.take(), Marks::part(0).with(Marks::part(5)));
}

#[test]
fn one_card_announced_reaches_every_subscriber() {
    static OWN: Changes<PLACES> = Changes::new();
    let a = open(&OWN, Duration::from_secs(5));
    let b = open(&OWN, Duration::from_secs(5));
    let c = open(&OWN, Duration::from_secs(5));
    for rx in [&a, &b, &c] {
        took(rx);
    }
    std::thread::sleep(Duration::from_millis(50));
    OWN.announce(Marks::part(4));
    for (n, rx) in [&a, &b, &c].into_iter().enumerate() {
        assert_eq!(took(rx), Marks::part(4), "subscriber {n} was not told which card changed");
    }
}

#[test]
fn a_third_subscriber_takes_its_own_marks_and_leaves_the_streams_theirs() {
    static OWN: Changes<PLACES> = Changes::new();
    let page = open(&OWN, Duration::from_secs(5));
    took(&page);
    std::thread::sleep(Duration::from_millis(50));
    OWN.announce(Marks::part(1));
    assert_eq!(took(&page), Marks::part(1));
    // the subscriber that is not a page opens later and gets the whole of it, not the one card the page has had
    let outward = open(&OWN, Duration::from_secs(5));
    assert_eq!(took(&outward), Marks::ALL);
    std::thread::sleep(Duration::from_millis(50));
    OWN.announce(Marks::part(6));
    assert_eq!(took(&page), Marks::part(6));
    assert_eq!(took(&outward), Marks::part(6));
}

#[test]
fn a_tick_with_nothing_announced_sends_nothing_marked() {
    static OWN: Changes<PLACES> = Changes::new();
    let rx = open(&OWN, Duration::from_millis(300));
    took(&rx);
    assert_eq!(took(&rx), Marks::NONE, "a quiet board sent a card as changed");
}

#[test]
fn announcing_nothing_wakes_nobody() {
    static OWN: Changes<PLACES> = Changes::new();
    let rx = open(&OWN, Duration::from_secs(5));
    took(&rx);
    std::thread::sleep(Duration::from_millis(50));
    OWN.announce(Marks::NONE);
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "an empty announcement woke a subscriber");
}

#[test]
fn a_subscriber_that_ended_gives_its_place_and_its_marks_to_the_next() {
    static OWN: Changes<PLACES> = Changes::new();
    for _ in 0..PLACES {
        drop(OWN.listen().expect("a free place"));
    }
    OWN.announce(Marks::part(7));
    let rx = open(&OWN, Duration::from_secs(5));
    // the place is free, and what the one before it was told is not this one's business: it starts whole
    assert_eq!(took(&rx), Marks::ALL);
    std::thread::sleep(Duration::from_millis(50));
    OWN.announce(Marks::part(2));
    assert_eq!(took(&rx), Marks::part(2));
}

#[test]
fn a_subscriber_past_the_places_there_are_gets_none() {
    static OWN: Changes<PLACES> = Changes::new();
    let held: Vec<_> = (0..PLACES).map(|_| OWN.listen().expect("a free place")).collect();
    assert!(OWN.listen().is_none(), "a place was given past the places there are");
    drop(held);
    assert!(OWN.listen().is_some(), "the places were not given back");
}

#[test]
fn a_part_beyond_the_places_of_the_set_is_refused_rather_than_folded_onto_another() {
    // 32 bits, so parts 0..=31 have one each; a 32nd part would land on a bit already spoken for, and a part
    // silently sharing another's bit sends someone else's rows
    assert!(Marks::part_at(31).is_some());
    assert!(Marks::part_at(32).is_none());
    assert!(Marks::part_at(usize::MAX).is_none());
}
