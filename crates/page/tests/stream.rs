//! The stream of contract 3 sends what changed and nothing else: a reader that opens gets every
//! value, a quiet board sends nothing, one value changed is one record, and a second reader mid-work gets everything.

use std::sync::Mutex;

use page::dictionary::{Said, Value};
use page::{Sent, Values, Wanted};

/// A board of four values on two cards, changed by the test between sends.
struct Board(Mutex<Vec<Value>>);

impl Board {
    fn new() -> Self {
        Board(Mutex::new(vec![Value::Number(21.5), Value::On(false), Value::Word("out"), Value::Number(7774.0)]))
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
        &["relays", "flame"]
    }
    fn values(&self, wanted: &Wanted, each: &mut dyn FnMut(usize, Said<'_>)) {
        let all = self.0.lock().unwrap();
        if wanted.has("system") {
            each(0, all[0].into());
            each(3, all[3].into());
        }
        if wanted.has("relays") {
            each(1, all[1].into());
        }
        if wanted.has("flame") {
            each(2, all[2].into());
        }
    }
}

/// One send of the cards WANTED to the reader who has SENT: what went out, as text.
fn send(board: &Board, sent: &mut Sent, wanted: &[&str]) -> String {
    let mut out = Vec::new();
    let marks = match wanted {
        ["*"] => Wanted::ALL,
        named => Wanted::of(board.cards(), named),
    };
    page::write_changed(&mut out, board, &marks, sent).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn a_reader_that_opens_gets_every_value() {
    let board = Board::new();
    let mut sent = Sent::new(4);
    assert_eq!(send(&board, &mut sent, &["*"]), "data: [[0,21.5],[3,7774.0],[1,false],[2,\"out\"]]\n\n");
}

#[test]
fn a_quiet_board_sends_nothing_at_all() {
    let board = Board::new();
    let mut sent = Sent::new(4);
    send(&board, &mut sent, &["*"]);
    assert_eq!(send(&board, &mut sent, &["*"]), "", "a card marked with nothing changed in it");
    assert_eq!(send(&board, &mut sent, &["flame"]), "");
}

#[test]
fn one_value_changed_is_one_record() {
    let board = Board::new();
    let mut sent = Sent::new(4);
    send(&board, &mut sent, &["*"]);
    board.set(1, Value::On(true));
    assert_eq!(send(&board, &mut sent, &["*"]), "data: [[1,true]]\n\n");
    board.set(2, Value::Unknown("not_read"));
    assert_eq!(send(&board, &mut sent, &["flame"]), "data: [[2,{\"unknown\":\"not_read\"}]]\n\n");
}

#[test]
fn a_second_reader_mid_work_gets_every_value_and_the_first_only_what_changed() {
    let board = Board::new();
    let mut first = Sent::new(4);
    send(&board, &mut first, &["*"]);
    board.set(1, Value::On(true));
    let mut second = Sent::new(4);
    assert_eq!(send(&board, &mut second, &["*"]), "data: [[0,21.5],[3,7774.0],[1,true],[2,\"out\"]]\n\n");
    assert_eq!(send(&board, &mut first, &["*"]), "data: [[1,true]]\n\n");
}

#[test]
fn a_value_sent_only_when_a_reader_opens_is_not_sent_again_when_it_moves() {
    let board = Board::new();
    let mut sent = Sent::new(4);
    sent.once(3);
    send(&board, &mut sent, &["*"]);
    board.set(3, Value::Number(8020.0));
    assert_eq!(send(&board, &mut sent, &["*"]), "");
}
