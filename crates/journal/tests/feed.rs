//! The log's way from the task that writes a line to the one that reads it: a channel of fixed size
//! that never makes its writer wait, loses the oldest when its reader falls behind, and asks the heap for nothing.

use journal::{line_of, Feed, Read, Ring, LINE};

fn all(feed: &Feed<16>) -> Vec<String> {
    std::iter::from_fn(|| feed.take()).map(|l| l.to_string()).collect()
}

#[test]
fn lines_put_faster_than_they_are_taken_lose_the_oldest_and_are_counted() {
    let feed = Feed::<16>::new();
    for n in 0..100 {
        feed.put(&format!("line {n}"));
    }
    let got = all(&feed);
    let want: Vec<String> = (84..100).map(|n| format!("line {n}")).collect();
    assert_eq!(got, want, "not the newest sixteen, in order");
    assert_eq!(feed.lost(), 84);
    assert_eq!(feed.lost(), 0, "the count was not cleared by asking");
}

#[test]
fn lines_taken_as_fast_as_they_come_are_all_there_and_none_lost() {
    let feed = Feed::<16>::new();
    let mut got = Vec::new();
    for n in 0..100 {
        feed.put(&format!("line {n}"));
        got.extend(all(&feed));
    }
    assert_eq!(got.len(), 100);
    assert_eq!(got[99], "line 99");
    assert_eq!(feed.lost(), 0);
}

#[test]
fn a_line_longer_than_a_slot_is_cut_at_a_whole_character_and_says_so() {
    let long = "ж".repeat(LINE);
    let cut = line_of(&long);
    assert!(cut.len() <= LINE, "{} bytes", cut.len());
    assert!(cut.ends_with('…'), "a cut line does not say it was cut: {cut:?}");
    assert!(cut.starts_with("жжж"));
    assert_eq!(line_of("status").as_str(), "status", "a short line was not kept as it is");
}

#[test]
fn a_console_slower_than_the_lines_loses_the_oldest_is_told_how_many_and_reads_on() {
    let mut ring: Ring<64> = Ring::new();
    let mut cursor = ring.now();
    for n in 0..200 {
        ring.push(line_of(&format!("line {n}")));
    }
    assert_eq!(ring.read(&mut cursor), Some(Read::Lost(136)));
    let mut got = Vec::new();
    while let Some(Read::Line(l)) = ring.read(&mut cursor) {
        got.push(l.to_string());
    }
    assert_eq!(got.len(), 64);
    assert_eq!(got.first().map(String::as_str), Some("line 136"));
    assert_eq!(got.last().map(String::as_str), Some("line 199"));
    assert_eq!(ring.read(&mut cursor), None);
}

#[test]
fn a_console_that_keeps_up_reads_every_line_and_loses_none() {
    let mut ring: Ring<64> = Ring::new();
    let mut cursor = ring.now();
    let mut got = Vec::new();
    for n in 0..200 {
        ring.push(line_of(&format!("line {n}")));
        while let Some(read) = ring.read(&mut cursor) {
            match read {
                Read::Line(l) => got.push(l.to_string()),
                Read::Lost(n) => panic!("a console that kept up lost {n}"),
            }
        }
    }
    assert_eq!(got.len(), 200);
}

#[test]
fn a_console_that_opens_reads_from_now_on() {
    let mut ring: Ring<64> = Ring::new();
    ring.push(line_of("before"));
    let mut cursor = ring.now();
    ring.push(line_of("after"));
    assert_eq!(ring.read(&mut cursor), Some(Read::Line("after")));
    assert_eq!(ring.read(&mut cursor), None);
}
