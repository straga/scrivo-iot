//! The stream sends what a reader needs, at the pace it asked for: not more often than a reader's `every=` seconds, at every change for a reader that asked for Live - and Live ends by itself -, and a
//! number goes out with the digits it is worth, so a change under them says nothing.
//!
//! A second of the pace lasts a tenth of one here (`common::limits`): `every=2` is 200 ms, the pace of everyone, 5 s, is 500 ms.

mod common;

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// One test at a time: the places of the subscribers (`web::CHANGES`) are one static of the program, three of them, and a test that holds a stream holds one until its socket is seen closed.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn alone() -> std::sync::MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The `data:` lines of TEXT, without the head of the answer or the beats.
fn events(text: &str) -> Vec<&str> {
    text.split("\n\n").filter_map(|e| e.trim_start_matches(|c| c != 'd').strip_prefix("data: ")).collect()
}

/// Changes the third reading to a new number every EVERY, COUNT times, announcing each; what the reader got over that time and a little after it.
fn burst(run: &common::Running, s: &mut std::net::TcpStream, every: Duration, count: u32) -> Vec<String> {
    let mut got = String::new();
    for n in 1..=count {
        run.board.set(2, n as f32);
        web::announce("flame");
        got += &common::read_for(s, every);
    }
    got += &common::read_for(s, Duration::from_millis(700));
    events(&got).into_iter().map(str::to_string).collect()
}

#[test]
fn a_reader_that_asked_for_a_pace_hears_a_burst_of_changes_no_more_often_than_that_and_the_last_one_as_it_is() {
    let _alone = alone();
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut s = common::stream_asking(run.http, "?every=2");
    assert_eq!(events(&common::read_for(&mut s, Duration::from_millis(300))).len(), 1, "the first event is the whole board");
    // ten changes forty milliseconds apart: 400 ms of them, a pace of 200 ms
    let got = burst(&run, &mut s, Duration::from_millis(40), 10);
    assert!(got.len() <= 3, "ten changes in 400 ms at a pace of 200 ms gave {} events: {got:?}", got.len());
    assert!(!got.is_empty(), "nothing was heard of ten changes");
    assert_eq!(got.last().unwrap(), "[[2,10.0]]", "the last thing said is the last thing that happened");
}

#[test]
fn a_reader_that_asked_for_live_hears_every_change() {
    let _alone = alone();
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut s = common::stream_asking(run.http, "?every=0");
    common::read_for(&mut s, Duration::from_millis(300));
    let got = burst(&run, &mut s, Duration::from_millis(80), 8);
    assert!(got.len() >= 7, "eight changes 80 ms apart gave {} events at Live: {got:?}", got.len());
}

#[test]
fn a_reader_that_asks_nothing_hears_at_the_pace_of_everyone_five_seconds() {
    let _alone = alone();
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut s = common::stream_asking(run.http, "");
    common::read_for(&mut s, Duration::from_millis(300));
    // a change every 100 ms for 1.2 s: at a pace of 500 ms that is three events at the most
    let got = burst(&run, &mut s, Duration::from_millis(100), 12);
    assert!(got.len() <= 4, "twelve changes in 1.2 s at the pace of everyone (500 ms here) gave {} events: {got:?}", got.len());
}

#[test]
fn live_ends_by_itself_and_the_reader_then_hears_at_the_pace_of_everyone() {
    let _alone = alone();
    let limits = web::Limits { live: Duration::from_millis(400), ..common::limits(Duration::from_millis(500)) };
    let run = common::start(limits);
    let mut s = common::stream_asking(run.http, "?every=0");
    common::read_for(&mut s, Duration::from_millis(300));
    // Live is still on: this is heard at once
    run.board.set(2, 1.0);
    web::announce("flame");
    assert_eq!(events(&common::read_for(&mut s, Duration::from_millis(150))).len(), 1, "a change inside the Live of the stream is heard at once");
    // after it, twelve changes in 1.2 s are heard at the pace of everyone
    std::thread::sleep(Duration::from_millis(500));
    common::read_for(&mut s, Duration::from_millis(1));
    let got = burst(&run, &mut s, Duration::from_millis(100), 12);
    assert!(got.len() <= 4, "after Live the reader heard {} events of twelve changes: {got:?}", got.len());
}

#[test]
fn a_number_goes_out_with_the_digits_it_is_worth_and_a_change_under_them_says_nothing() {
    let _alone = alone();
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut s = common::stream_asking(run.http, "?every=0");
    common::read_for(&mut s, Duration::from_millis(300));
    // the first reading is worth a whole number: 17.149593 is 17
    run.board.set(0, 17.149_593);
    web::announce("flame");
    let sent = events(&common::read_for(&mut s, Duration::from_millis(300))).join(" ");
    assert!(sent.contains("[0,17.0]"), "17.149593 went out as {sent:?}");
    // 17.2 is still 17: nothing to say
    run.board.set(0, 17.2);
    web::announce("flame");
    assert_eq!(events(&common::read_for(&mut s, Duration::from_millis(300))), Vec::<&str>::new(), "a change under the digits a number is worth was sent");
    // 18.4 is 18
    run.board.set(0, 18.4);
    web::announce("flame");
    assert!(events(&common::read_for(&mut s, Duration::from_millis(300))).join(" ").contains("[0,18.0]"));
    // the second is worth a tenth
    run.board.set(1, 3.14159);
    web::announce("flame");
    assert!(events(&common::read_for(&mut s, Duration::from_millis(300))).join(" ").contains("[1,3.1]"));
}

#[test]
fn the_state_a_reader_asks_for_carries_the_same_digits_as_the_stream() {
    let _alone = alone();
    let run = common::start(common::limits(Duration::from_millis(500)));
    run.board.set(0, 17.149_593);
    run.board.set(1, 3.14159);
    let said = common::ask(run.http, "GET /state HTTP/1.1\r\nHost: board\r\n\r\n", Duration::from_secs(2)).expect("no answer");
    assert!(said.contains("[0,17.0]") && said.contains("[1,3.1]"), "{said}");
}

#[test]
fn the_first_event_goes_at_once_whatever_the_pace_asked() {
    let _alone = alone();
    let run = common::start(common::limits(Duration::from_millis(500)));
    let began = Instant::now();
    let mut s = common::stream_asking(run.http, "?every=30");
    let first = common::read_for(&mut s, Duration::from_millis(400));
    assert_eq!(events(&first).len(), 1, "the whole board at once, whatever the pace");
    assert!(began.elapsed() < Duration::from_secs(2));
}
