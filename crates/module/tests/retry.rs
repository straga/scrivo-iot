//! A task that does not start is tried again with a pause and every failure is said: on rows of a start that fails so many times, and no board anywhere.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: a start that works is not said, not slept for and not repeated; a start that fails is tried again up to the tries given, with the pause between two tries and none after the last,
//! and each failure is one line naming the task, the try and the reason; a start that never works answers false, so the caller can say what it does without a round.

use std::time::Duration;

use module::retry::start_with_retries;

const PAUSE: Duration = Duration::from_secs(2);

/// A start that fails FAILS times and then works, as the rows call it: what it answered, how many times it was called, the pauses taken and the lines said.
fn run(tries: u32, fails: u32) -> (bool, u32, Vec<Duration>, Vec<String>) {
    let (mut calls, mut slept, mut lines) = (0u32, Vec::new(), Vec::new());
    let started = start_with_retries(
        "temperatures",
        tries,
        PAUSE,
        || {
            calls += 1;
            if calls <= fails { Err(format!("no memory for the stack ({calls})")) } else { Ok(()) }
        },
        |pause| slept.push(pause),
        |line| lines.push(line),
    );
    (started, calls, slept, lines)
}

#[test]
fn a_start_that_works_is_called_once_and_says_nothing() {
    let (started, calls, slept, lines) = run(5, 0);
    assert_eq!((started, calls, slept.len(), lines.len()), (true, 1, 0, 0));
}

#[test]
fn a_start_that_fails_twice_and_then_works_is_said_twice_and_waited_for_twice() {
    let (started, calls, slept, lines) = run(5, 2);
    assert_eq!((started, calls), (true, 3), "the third try works");
    assert_eq!(slept, vec![PAUSE, PAUSE], "a pause after each failure that is followed by another try");
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("temperatures") && lines[0].contains("1 of 5") && lines[0].contains("no memory for the stack (1)"), "{}", lines[0]);
    assert!(lines[1].contains("2 of 5"), "{}", lines[1]);
}

#[test]
fn a_start_that_never_works_is_tried_all_the_tries_said_every_time_and_answers_false() {
    let (started, calls, slept, lines) = run(3, 99);
    assert_eq!((started, calls), (false, 3));
    assert_eq!(slept.len(), 2, "no pause after the last try: nobody is waiting for it");
    assert_eq!(lines.len(), 3, "one line for each failure, the last included");
}

#[test]
fn a_start_that_works_on_the_last_try_answers_true() {
    let (started, calls, _, lines) = run(3, 2);
    assert_eq!((started, calls, lines.len()), (true, 3, 2));
}
