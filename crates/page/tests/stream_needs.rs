//! The stream says what a reader needs and no more: a number goes out with the digits it is worth, a count of seconds goes out when
//! it jumps and not every second, and a reader's `?every=` is read the way the board answers it.
//!
//! The expected numbers are worked out of what a reader sees: the rounded reading, the seconds it can count on by itself.

use std::sync::Mutex;

use page::dictionary::{Count, Entry, Kind, Said, Value};
use page::{Sent, Values, Wanted};

fn number(decimals: Option<u8>) -> Entry {
    Entry { unit: "u", decimals, ..Entry::row("c", "n", &[Kind::Number, Kind::AtLeast]) }
}

fn rounded(entry: &Entry, value: Value) -> Value {
    match page::rounded(entry, value.into()) {
        Said::Value(v) => v,
        Said::Text(_) => panic!("a number came back as text"),
    }
}

fn written(value: Value) -> String {
    serde_json::to_string(&value).unwrap()
}

// ---- the digits a number is worth ----

#[test]
fn a_number_goes_out_with_the_digits_the_entry_keeps() {
    assert_eq!(written(rounded(&number(Some(0)), Value::Number(17.149_593))), "17.0", "an angle of 17.149593 degrees is 17");
    assert_eq!(written(rounded(&number(Some(1)), Value::Number(98.26))), "98.3");
    assert_eq!(written(rounded(&number(Some(1)), Value::Number(98.24))), "98.2");
    assert_eq!(written(rounded(&number(Some(2)), Value::Number(8.5549))), "8.55", "two digits keep two");
    assert_eq!(written(rounded(&number(Some(0)), Value::AtLeast(300.4))), "{\"at_least\":300.0}", "the edge of a scale is rounded too");
}

#[test]
fn a_number_that_rounds_to_nothing_is_zero_and_not_minus_zero() {
    assert_eq!(written(rounded(&number(Some(1)), Value::Number(-0.04))), "0.0");
    assert_eq!(written(rounded(&number(Some(0)), Value::Number(-0.4))), "0.0");
}

#[test]
fn a_negative_number_rounds_to_the_nearest_digit_too() {
    assert_eq!(written(rounded(&number(Some(1)), Value::Number(-12.26))), "-12.3");
}

#[test]
fn what_is_no_number_is_sent_as_it_is_and_so_is_a_number_the_entry_does_not_round() {
    assert_eq!(rounded(&number(Some(0)), Value::On(true)), Value::On(true));
    assert_eq!(rounded(&number(Some(0)), Value::Seconds(52)), Value::Seconds(52));
    assert_eq!(rounded(&number(Some(0)), Value::Unknown("not_read")), Value::Unknown("not_read"));
    assert_eq!(rounded(&number(Some(0)), Value::None), Value::None);
    assert_eq!(rounded(&number(None), Value::Number(17.149_593)), Value::Number(17.149_593), "no digits said, none taken");
}

// ---- a count of seconds that goes on by itself ----

const FOR: usize = 0;
const IN: usize = 1;
const SILENT: usize = 2;

fn table() -> [Entry; 4] {
    [
        Entry { counts: Some(Count::Up), ..Entry::row("a", "for", &[Kind::Seconds]) },
        Entry { counts: Some(Count::Down), clock: true, ..Entry::row("a", "lighting_in", &[Kind::Seconds]) },
        Entry::row("a", "silent_flame", &[Kind::Seconds]),
        Entry { unit: "u", ..Entry::row("a", "temp", &[Kind::Number]) },
    ]
}

struct Board(Mutex<[Value; 4]>);

impl Board {
    fn new() -> Self {
        Board(Mutex::new([Value::Seconds(0), Value::None, Value::Seconds(0), Value::Number(20.0)]))
    }
    fn set(&self, at: usize, value: Value) {
        self.0.lock().unwrap()[at] = value;
    }
}

impl Values for Board {
    fn version(&self) -> &str {
        "v+00000000"
    }
    fn cards(&self) -> &'static [&'static str] {
        &["a"]
    }
    fn values(&self, wanted: &Wanted, each: &mut dyn FnMut(usize, Said<'_>)) {
        if wanted.has("a") {
            for (at, v) in self.0.lock().unwrap().iter().enumerate() {
                each(at, (*v).into());
            }
        }
    }
}

/// One send at the board's clock NOW_MS: the records that went out, `[[address, value], ...]`, or empty.
fn send(board: &Board, sent: &mut Sent, now_ms: u64) -> Vec<(usize, serde_json::Value)> {
    let mut out = Vec::new();
    sent.at(now_ms);
    page::write_changed(&mut out, board, &Wanted::ALL, sent).unwrap();
    let text = String::from_utf8(out).unwrap();
    match text.strip_prefix("data: ") {
        None => Vec::new(),
        Some(body) => serde_json::from_str::<Vec<(usize, serde_json::Value)>>(body.trim()).unwrap(),
    }
}

/// What went out about the address AT alone: the board of these tests keeps its other counts still, and a count that stands still while the clock goes
/// is rightly said again.
fn about(at: usize, sent: Vec<(usize, serde_json::Value)>) -> Vec<(usize, serde_json::Value)> {
    sent.into_iter().filter(|(a, _)| *a == at).collect()
}

fn addresses(sent: &[(usize, serde_json::Value)]) -> Vec<usize> {
    sent.iter().map(|(at, _)| *at).collect()
}

#[test]
fn a_count_up_that_goes_on_as_the_reader_counts_it_is_not_sent_again() {
    let (table, board) = (table(), Board::new());
    let mut sent = Sent::of(&table);
    assert!(addresses(&send(&board, &mut sent, 0)).contains(&FOR), "a reader that opens gets the count");
    let mut sends = 0;
    for second in 1..=300u32 {
        board.set(FOR, Value::Seconds(second));
        sends += send(&board, &mut sent, u64::from(second) * 1000).iter().filter(|(at, _)| *at == FOR).count();
    }
    assert_eq!(sends, 0, "300 s of a count that goes on by itself, and the reader counts it: nothing to say");
}

#[test]
fn a_count_up_that_starts_over_is_sent_once_and_the_reader_counts_on_from_it() {
    let (table, board) = (table(), Board::new());
    let mut sent = Sent::of(&table);
    send(&board, &mut sent, 0);
    for second in 1..=100u32 {
        board.set(FOR, Value::Seconds(second));
        send(&board, &mut sent, u64::from(second) * 1000);
    }
    board.set(FOR, Value::Seconds(0));
    let at_start = send(&board, &mut sent, 101_000);
    assert_eq!(at_start, vec![(FOR, serde_json::json!(0))], "a new stage: the count is 0 and the reader is told");
    for second in 1..=50u32 {
        board.set(FOR, Value::Seconds(second));
        let more = send(&board, &mut sent, 101_000 + u64::from(second) * 1000);
        assert!(!addresses(&more).contains(&FOR), "second {second} after the start is counted by the reader");
    }
}

#[test]
fn a_count_down_runs_to_nothing_unsaid_and_a_new_target_is_said() {
    let (table, board) = (table(), Board::new());
    let mut sent = Sent::of(&table);
    send(&board, &mut sent, 0);
    board.set(IN, Value::Seconds(40));
    assert_eq!(about(IN, send(&board, &mut sent, 1000)), vec![(IN, serde_json::json!(40))], "a clock that starts is said");
    for second in 1..=40u32 {
        board.set(IN, Value::Seconds(40 - second));
        let said = send(&board, &mut sent, 1000 + u64::from(second) * 1000);
        assert!(!addresses(&said).contains(&IN), "second {second}: the reader counts {} down itself", 40 - second);
    }
    for second in 41..=60u32 {
        let said = send(&board, &mut sent, 1000 + u64::from(second) * 1000);
        assert!(!addresses(&said).contains(&IN), "a count that reached 0 and stays there says nothing more");
    }
    board.set(IN, Value::Seconds(25));
    assert_eq!(about(IN, send(&board, &mut sent, 62_000)), vec![(IN, serde_json::json!(25))], "a new target: 25 where the reader counts 0");
}

#[test]
fn a_count_that_stops_having_a_number_is_said_and_said_again_when_it_comes_back() {
    let (table, board) = (table(), Board::new());
    let mut sent = Sent::of(&table);
    board.set(IN, Value::Seconds(30));
    send(&board, &mut sent, 0);
    board.set(IN, Value::None);
    assert_eq!(about(IN, send(&board, &mut sent, 1000)), vec![(IN, serde_json::json!(null))], "the clock ran out: nothing to count");
    board.set(IN, Value::Seconds(30));
    assert_eq!(about(IN, send(&board, &mut sent, 2000)), vec![(IN, serde_json::json!(30))], "the same 30 as the first time is said again: the reader had none");
}

#[test]
fn a_count_that_the_reader_has_wrong_by_two_seconds_is_said_and_by_one_is_not() {
    let (table, board) = (table(), Board::new());
    let mut sent = Sent::of(&table);
    send(&board, &mut sent, 0);
    board.set(FOR, Value::Seconds(11));
    assert!(!addresses(&send(&board, &mut sent, 10_000)).contains(&FOR), "11 where the reader counts 10: the board's second and the reader's differ by a tick");
    board.set(FOR, Value::Seconds(23));
    assert_eq!(send(&board, &mut sent, 20_000), vec![(FOR, serde_json::json!(23))], "23 where the reader counts 20: it is wrong, and told");
}

#[test]
fn seconds_that_are_no_count_go_out_whenever_they_change() {
    let (table, board) = (table(), Board::new());
    let mut sent = Sent::of(&table);
    send(&board, &mut sent, 0);
    for second in 1..=5u32 {
        board.set(SILENT, Value::Seconds(second));
        assert_eq!(about(SILENT, send(&board, &mut sent, u64::from(second) * 1000)), vec![(SILENT, serde_json::json!(second))], "0 means the sensor answers: a reader counting on from 0 would show a silence that is not there");
    }
}

#[test]
fn a_second_reader_that_opens_mid_count_gets_the_count_as_it_is_now() {
    let (table, board) = (table(), Board::new());
    let mut first = Sent::of(&table);
    send(&board, &mut first, 0);
    board.set(FOR, Value::Seconds(77));
    let mut second = Sent::of(&table);
    let got = send(&board, &mut second, 77_000);
    assert!(got.contains(&(FOR, serde_json::json!(77))), "a reader that opens has no count to go on from: {got:?}");
}

// ---- what a reader asks of the pace ----

#[test]
fn a_reader_asks_for_its_pace_in_whole_seconds_and_zero_is_live() {
    assert_eq!(page::every_asked("/events?every=1"), 1);
    assert_eq!(page::every_asked("/events?every=30"), 30);
    assert_eq!(page::every_asked("/events?every=0"), 0);
    assert_eq!(page::every_asked("/events?x=1&every=10&y=2"), 10);
}

#[test]
fn a_reader_that_asks_nothing_or_nonsense_gets_the_pace_of_everyone() {
    let everyone = page::dictionary::STREAM_EVERY_S;
    assert_eq!(everyone, 5);
    for uri in ["/events", "/events?", "/events?every=", "/events?every=abc", "/events?every=-3", "/events?every=2.5", "/events?every=61", "/events?every=99999999999", "/events?every=%30"] {
        assert_eq!(page::every_asked(uri), everyone, "{uri}");
    }
    assert_eq!(page::every_asked("/events?every=60"), 60, "the longest pace there is is still asked for");
}
