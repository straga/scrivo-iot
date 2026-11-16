//! The journal on flash keeps a line once and counts its repeats: the night of 24-25.10 pushed one warning of the analog unit through the flash every 180 ms,
//! and by 04:26 the journal held nothing from before 04:21. The lines of the black box (a leap of the clock, a leap of the tick, the fronts lost and back) are
//! never folded: each is written whole with its own time and number, and a storm of them is held by a limit a minute with the count of what it skipped.
//!
//! The rows run on the lines of that night (tests/fixtures/journal_storm.txt, the journal of `.33` as it was read at 04:26), not on lines made up for them.

use journal::{Fold, Verdict, BLACK_BOX, FOLD_BURST, FOLD_PERIOD_S};

/// One line of the fixture as `Tee` hands it to the fold: the target, the text after it, the local time, and the seconds of a clock that only goes forward.
struct Line {
    target: String,
    text: String,
    stamp: String,
    at_s: u64,
}

fn lines_of_fixture() -> Vec<Line> {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/journal_storm.txt")).unwrap();
    text.lines()
        .map(|row| {
            // "2026-10-25 04:21:32 W (53273370) analog::board: draught: a window ..."
            let (stamp, rest) = row.split_at(19);
            let rest = rest.trim_start();
            let after_uptime = rest.split_once(") ").unwrap().1;
            let (target, text) = after_uptime.split_once(": ").unwrap();
            let clock: Vec<u64> = stamp[11..].split(':').map(|n| n.parse().unwrap()).collect();
            Line { target: target.into(), text: text.into(), stamp: stamp.into(), at_s: clock[0] * 3600 + clock[1] * 60 + clock[2] }
        })
        .collect()
}

fn storm() -> Vec<Line> {
    lines_of_fixture().into_iter().filter(|line| line.target == "analog::board").collect()
}

#[test]
fn the_storm_of_the_analog_window_is_written_a_burst_of_each_word_and_counted_not_sixty_times() {
    let lines = storm();
    assert_eq!(lines.len(), 60, "the fixture holds sixty lines of the cut window");
    let mut fold = Fold::new();
    let mut written = 0;
    let mut held = 0;
    for line in &lines {
        match fold.put(&line.target, &line.text, &line.stamp, line.at_s) {
            Verdict::Write => written += 1,
            Verdict::Hold => held += 1,
            other => panic!("inside the first period nothing is due: {other:?}"),
        }
    }
    // two lines are told apart by their words - the draught's and the flame's - and each is written its burst, then counted
    let burst = FOLD_BURST as usize;
    assert_eq!((written, held), (2 * burst, 60 - 2 * burst));
}

#[test]
fn what_was_held_comes_out_as_one_line_each_with_the_count_and_the_last_text_whole_when_the_storm_goes_quiet() {
    let lines = storm();
    let mut fold = Fold::new();
    lines.iter().for_each(|line| {
        fold.put(&line.target, &line.text, &line.stamp, line.at_s);
    });
    let last_at = lines.last().unwrap().at_s;
    assert!(fold.quiet(last_at + FOLD_PERIOD_S - 1).is_empty(), "a storm that has not been quiet for a period is not over");
    let said = fold.quiet(last_at + FOLD_PERIOD_S);
    assert_eq!(said.iter().map(|(_, s)| s.times).sum::<u32>(), 60 - 2 * FOLD_BURST, "every line held is in a count");
    let (_, draught) = said.iter().find(|(_, s)| s.last.starts_with("draught")).expect("the draught's run");
    let last_draught = lines.iter().rev().find(|l| l.text.starts_with("draught")).unwrap();
    assert_eq!(draught.last, last_draught.text, "the last text comes with its numbers, not masked");
    assert_eq!(draught.last_at, last_draught.stamp);
    assert!(fold.quiet(last_at + 10 * FOLD_PERIOD_S).is_empty(), "a run is told once");
}

#[test]
fn the_summary_line_names_the_count_the_time_of_the_last_and_its_text() {
    let mut fold = Fold::new();
    for n in 0..FOLD_BURST as u64 + 1 {
        fold.put("analog::board", &format!("draught: cut after {n} of 20 samples"), &format!("2026-10-25 04:21:{:02}", 30 + n), 100 + n);
    }
    let last = 100 + FOLD_BURST as u64;
    let said = fold.quiet(last + FOLD_PERIOD_S);
    assert_eq!(said.len(), 1);
    let line = said[0].1.line(&said[0].0);
    assert!(line.contains("x1"), "{line}");
    assert!(line.contains(&format!("2026-10-25 04:21:{:02}", 30 + FOLD_BURST)), "{line}");
    assert!(line.contains(&format!("draught: cut after {FOLD_BURST} of 20 samples")), "{line}");
}

#[test]
fn a_hundred_equal_lines_are_a_burst_of_lines_and_a_count() {
    let mut fold = Fold::new();
    let verdicts: Vec<Verdict> = (0..100).map(|n| fold.put("fan", "no crossings", "stamp", 1000 + n / 10)).collect();
    assert_eq!(verdicts.iter().filter(|v| **v == Verdict::Write).count() as u32, FOLD_BURST);
    assert_eq!(verdicts.iter().filter(|v| **v == Verdict::Hold).count() as u32, 100 - FOLD_BURST);
}

#[test]
fn lines_that_differ_only_in_their_numbers_are_one_line() {
    let mut fold = Fold::new();
    for n in 0..FOLD_BURST as u64 {
        assert_eq!(fold.put("analog::board", &format!("draught: cut after {n} of 20 samples"), "s", 10 + n), Verdict::Write, "inside the burst");
    }
    assert_eq!(fold.put("analog::board", "draught: cut after 17 of 20 samples", "s", 20), Verdict::Hold);
}

#[test]
fn a_line_that_flips_between_two_states_inside_the_period_is_written_whole() {
    // A, B, A, B: the order of events is what the journal is read for, and three of each are not a storm
    let mut fold = Fold::new();
    for round in 0..FOLD_BURST as u64 {
        assert_eq!(fold.put("automaton", "stage: kindling", "s", 10 + round * 2), Verdict::Write);
        assert_eq!(fold.put("automaton", "stage: burning", "s", 11 + round * 2), Verdict::Write);
    }
}

#[test]
fn lines_that_differ_in_their_words_target_or_level_words_are_not_folded() {
    let mut fold = Fold::new();
    assert_eq!(fold.put("analog::board", "draught: cut after 5 of 20 samples", "s", 10), Verdict::Write);
    assert_eq!(fold.put("analog::board", "flame: cut after 5 of 20 samples", "s", 11), Verdict::Write, "another word");
    assert_eq!(fold.put("fan_gate", "draught: cut after 5 of 20 samples", "s", 12), Verdict::Write, "another target");
    assert_eq!(fold.put("analog::board", "draught: cut after 5 of 20 windows", "s", 13), Verdict::Write, "another word after the numbers");
}

#[test]
fn a_line_that_comes_once_a_period_is_written_every_time_and_never_held() {
    let mut fold = Fold::new();
    // `fan: 29 crossings lost by gaps ...` came every minute of that journal: a period apart it is a new line, and there is nothing to count between
    let lost: Vec<Line> = lines_of_fixture().into_iter().filter(|line| line.text.contains("crossings lost by gaps")).collect();
    assert!(lost.len() >= 6, "the fixture holds the minutes of fan_gate");
    for line in &lost {
        assert_eq!(fold.put(&line.target, &line.text, &line.stamp, line.at_s), Verdict::Write, "{}", line.stamp);
    }
}

#[test]
fn a_line_that_comes_again_after_the_period_is_written_with_how_many_went_unsaid() {
    let mut fold = Fold::new();
    // the burst, then four counted
    for n in 0..FOLD_BURST as u64 {
        assert_eq!(fold.put("fan", "no crossings", "s", 100 + n), Verdict::Write);
    }
    for n in 0..4u64 {
        assert_eq!(fold.put("fan", "no crossings", "s", 100 + FOLD_BURST as u64 + n), Verdict::Hold);
    }
    let due = 100 + FOLD_BURST as u64 + FOLD_PERIOD_S;
    assert_eq!(fold.put("fan", "no crossings", "s", due), Verdict::Repeated { times: 5 });
}

#[test]
fn after_a_run_is_told_the_next_line_is_the_first_of_another() {
    let mut fold = Fold::new();
    for n in 0..FOLD_BURST as u64 + 1 {
        fold.put("fan", "no crossings", "s", 100 + n);
    }
    let last = 100 + FOLD_BURST as u64;
    assert_eq!(fold.quiet(last + FOLD_PERIOD_S).len(), 1);
    assert_eq!(fold.put("fan", "no crossings", "s", last + FOLD_PERIOD_S + 1), Verdict::Write);
}

#[test]
fn more_kinds_of_line_than_the_fold_holds_never_lose_a_line() {
    let mut fold = Fold::new();
    for n in 0..64 {
        assert_eq!(fold.put("fan", &format!("kind{}x", "a".repeat(n)), "s", 10 + n as u64), Verdict::Write);
    }
}

// ---- the black box ----

fn box_target(kind: &str) -> String {
    format!("{BLACK_BOX}{kind}")
}

#[test]
fn a_line_of_the_black_box_is_never_folded_however_equal_the_lines() {
    let mut fold = Fold::new();
    let target = box_target("clock");
    for at in 0..5 {
        assert_eq!(fold.put(&target, "clock leap back 3773 ms", "s", 100 + at), Verdict::Write, "second {at}");
    }
}

#[test]
fn a_storm_of_the_black_box_is_held_after_the_limit_and_counted() {
    let mut fold = Fold::new();
    let target = box_target("tick");
    let verdicts: Vec<Verdict> = (0..40).map(|n| fold.put(&target, &format!("tick leap forward {n} ms"), "s", 100 + n / 10)).collect();
    let written = verdicts.iter().filter(|v| **v == Verdict::Write).count();
    assert_eq!(written as u32, journal::BLACK_BOX_PER_MINUTE);
    assert_eq!(verdicts.iter().filter(|v| **v == Verdict::Hold).count() as u32, 40 - journal::BLACK_BOX_PER_MINUTE);
}

#[test]
fn the_first_line_of_the_black_box_after_a_held_storm_comes_with_how_many_were_skipped() {
    let mut fold = Fold::new();
    let target = box_target("tick");
    (0..40).for_each(|n| {
        fold.put(&target, &format!("tick leap forward {n} ms"), "s", 100 + n / 10);
    });
    assert_eq!(fold.put(&target, "tick leap forward 1 ms", "s", 100 + 60), Verdict::WriteAfterSkipped { skipped: 40 - journal::BLACK_BOX_PER_MINUTE });
    assert_eq!(fold.put(&target, "tick leap forward 2 ms", "s", 100 + 61), Verdict::Write, "the skipped are told once");
}

#[test]
fn a_storm_of_one_kind_of_the_black_box_does_not_hold_the_others() {
    let mut fold = Fold::new();
    let tick = box_target("tick");
    let fronts = box_target("fronts");
    (0..40).for_each(|n| {
        fold.put(&tick, "tick leap forward 5 ms", "s", 100 + n / 10);
    });
    assert_eq!(fold.put(&fronts, "fronts lost", "s", 104), Verdict::Write);
}

#[test]
fn the_lines_skipped_over_the_limit_are_told_once_when_no_line_of_the_kind_follows() {
    let mut fold = Fold::new();
    let target = box_target("clock");
    (0..30).for_each(|n| {
        fold.put(&target, "clock leap back 5 ms", "s", 100 + n);
    });
    assert!(fold.quiet(100 + 59).is_empty(), "inside the minute of the limit nothing is over");
    let said = fold.quiet(100 + 60);
    assert_eq!(said.len(), 1);
    let (said_target, summary) = &said[0];
    assert_eq!((said_target.as_str(), summary.skipped, summary.times), (target.as_str(), true, 30 - journal::BLACK_BOX_PER_MINUTE));
    assert!(summary.line(said_target).contains("skipped"), "{}", summary.line(said_target));
    assert!(fold.quiet(100 + 600).is_empty(), "told once");
    assert_eq!(fold.put(&target, "clock leap back 5 ms", "s", 100 + 601), Verdict::Write, "and not again by the next line");
}
