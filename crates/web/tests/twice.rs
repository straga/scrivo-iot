//! The stream sends what the value is when it sends, not every value it had: changed twice before a send, it goes
//! once and as it is last.

mod common;

use std::time::Duration;

/// The events of TEXT - the `data:` lines, without the head of the answer or the beats.
fn events(text: &str) -> Vec<&str> {
    text.split("\n\n").filter_map(|e| e.trim_start_matches(|c| c != 'd').strip_prefix("data: ")).collect()
}

#[test]
fn a_value_changed_twice_before_a_send_goes_once_and_last() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut s = common::stream(run.http);
    let first = common::read_for(&mut s, Duration::from_millis(500));
    assert_eq!(events(&first).len(), 1, "the first event, the whole board: {first:?}");

    run.board.set(1, 5.0);
    run.board.set(1, 6.0);
    web::announce("flame");
    let next = common::read_for(&mut s, Duration::from_millis(500));
    assert_eq!(events(&next), ["[[1,6.0]]"], "what came after two changes and one announcement: {next:?}");
}

#[test]
fn a_value_announced_without_a_change_is_not_sent_again() {
    let run = common::start(common::limits(Duration::from_millis(500)));
    let mut s = common::stream(run.http);
    common::read_for(&mut s, Duration::from_millis(500));

    run.board.set(1, 6.0);
    web::announce("flame");
    assert_eq!(events(&common::read_for(&mut s, Duration::from_millis(500))), ["[[1,6.0]]"]);
    web::announce("flame");
    let quiet = common::read_for(&mut s, Duration::from_millis(500));
    assert_eq!(events(&quiet), Vec::<&str>::new(), "an announcement with nothing changed sent {quiet:?}");
}
