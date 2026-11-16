//! What the chip's reset code means to a person. No hardware here: the code comes from esp_reset_reason().

#[test]
fn the_three_that_decide_where_to_look_are_named_apart() {
    // THE WHOLE POINT. A board that merely "restarted" leaves three repairs open at once, and they are not the
    // same repair: a brownout is wires and a power supply, a watchdog is our code holding a core too long, a
    // panic is our code being wrong. The words have to pick one.
    assert!(reset_why::words(9).contains("power dipped"));
    assert!(reset_why::words(4).contains("panic"));
    assert!(reset_why::words(5).contains("watchdog"));
    assert!(reset_why::words(6).contains("watchdog"));
    // and the two watchdogs are still told apart, because one is an interrupt held too long and the other a task
    assert_ne!(reset_why::words(5), reset_why::words(6));
}

#[test]
fn an_ordinary_start_says_so_and_does_not_look_like_a_fault() {
    assert!(reset_why::words(1).contains("power"));
    assert!(reset_why::words(3).contains("software"));
    assert!(!reset_why::words(1).contains("dipped"));
}

#[test]
fn a_code_we_do_not_know_comes_out_as_the_number_and_not_as_a_guess() {
    // the pair to every line above: a reason invented to fit a familiar word sends a person to the wrong repair
    assert_eq!(reset_why::words(77), "reset for a reason this firmware does not know, code 77");
    assert_eq!(reset_why::words(0), "reset for a reason the chip did not record, code 0");
}

#[test]
fn a_fault_is_told_from_an_ordinary_start() {
    // what a card colours red, and what it leaves quiet
    for ordinary in [1, 2, 3, 8] {
        assert!(!reset_why::is_fault(ordinary), "{ordinary}");
    }
    for fault in [4, 5, 6, 7, 9, 14, 15] {
        assert!(reset_why::is_fault(fault), "{fault}");
    }
    // an unknown code is not called a fault: we do not know that it is one
    assert!(!reset_why::is_fault(77));
}
