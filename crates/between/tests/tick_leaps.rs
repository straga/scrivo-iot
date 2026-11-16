//! A leap of the tick's counter is counted in the interrupt of the tick (crates/firmware/systick/systick.c), where no line of the log can be written; what is left to the
//! board is to look at the count once a second and to say what has come since. These rows hold that look on the numbers the interrupt keeps.
//! On the night of 24-25.10 `.33` took 18 of them and `status` kept the count and the largest two, so the order against the leaps of the clock and the loss of the fronts could not be read.

use between::tick::{Leap, TickLeaps, Watch};

fn leaps(count: u32, last_ticks: u32, last_was_back: bool) -> TickLeaps {
    TickLeaps { count, last_ticks, last_was_back, largest_back: 0, largest_forward: 0, rearms: 0 }
}

#[test]
fn a_count_that_has_not_moved_says_nothing() {
    let mut watch = Watch::new();
    assert_eq!(watch.look(leaps(0, 0, false)), None);
    assert_eq!(watch.look(leaps(0, 0, false)), None);
}

#[test]
fn a_leap_is_told_with_its_size_and_its_way() {
    let mut watch = Watch::new();
    watch.look(leaps(0, 0, false));
    assert_eq!(watch.look(leaps(1, 2620, true)), Some(Leap { new: 1, last_ticks: 2620, last_was_back: true }));
    assert_eq!(watch.look(leaps(1, 2620, true)), None, "told once");
}

#[test]
fn several_leaps_between_two_looks_are_one_line_with_how_many_and_the_last() {
    let mut watch = Watch::new();
    watch.look(leaps(2, 100, false));
    assert_eq!(watch.look(leaps(5, 10_737_000, false)), Some(Leap { new: 3, last_ticks: 10_737_000, last_was_back: false }));
}

#[test]
fn leaps_that_were_there_before_the_first_look_are_told_at_the_first() {
    // a board that is looked at for the first time after a storm in its first seconds: they are in the journal too
    let mut watch = Watch::new();
    assert_eq!(watch.look(leaps(3, 400, true)), Some(Leap { new: 3, last_ticks: 400, last_was_back: true }));
}

#[test]
fn a_count_that_went_down_is_a_new_count_and_not_a_negative_number_of_leaps() {
    let mut watch = Watch::new();
    watch.look(leaps(5, 1, false));
    assert_eq!(watch.look(leaps(1, 70, true)), Some(Leap { new: 1, last_ticks: 70, last_was_back: true }));
}
