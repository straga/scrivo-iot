//! What a recorded fall says to a person. No hardware here: the numbers come from the chip's panic handler
//! through the panic_note component, and this says what they mean.

use fell_where::{exception_words, lines, trace_line, when, words, Fall, When};

/// A fall as the note carries one: the chip's numbers, and what our code said if it said anything.
fn fall(seq: u32) -> Fall {
    Fall {
        seq,
        exception: 4,
        core: 0,
        addr: 0x42009abc,
        data: 0,
        reason: "StoreProhibited".into(),
        said: String::new(),
        trace: vec![0x42009abc, 0x4200912c, 0x4037a1c4],
        other_core: Some(1),
        other_trace: vec![0x4037f26f, 0x42130c79],
        same_image: true,
    }
}

#[test]
fn a_fault_names_the_address_it_read_or_wrote() {
    // 6.4: a LoadProhibited in Journal::subscribe says where the code was, not what it touched - and what it touched
    // (internal RAM, external RAM, nothing at all) is what tells a corrupted pointer from memory that was not there
    let loaded = Fall { exception: 4, reason: "LoadProhibited".into(), data: 0x3c1a2b3c, ..fall(7) };
    assert!(words(&loaded, 8).contains("touched 0x3c1a2b3c"), "{}", words(&loaded, 8));
}

#[test]
fn a_fall_without_a_data_address_names_none() {
    let aborted = Fall { exception: 3, reason: "abort() was called".into(), data: 0, ..fall(7) };
    assert!(!words(&aborted, 8).contains("touched"), "{}", words(&aborted, 8));
    let faulted = Fall { data: 0, ..fall(7) };
    assert!(!words(&faulted, 8).contains("touched"), "{}", words(&faulted, 8));
}

#[test]
fn the_fall_that_brought_this_start_is_told_from_an_older_one() {
    // THE WHOLE POINT of the count. A note left on the board outlives the fall that wrote it, and a person
    // reading "it fell at 0x42009abc" has no way to know whether that was a minute ago or last week - so the
    // note that belongs to THIS start has to say so, and every other one has to say how far back it is.
    assert_eq!(when(7, 8), When::ThisStart);
    assert_eq!(when(6, 8), When::StartsBack(2));
    assert_eq!(when(1, 8), When::StartsBack(7));
}

#[test]
fn a_note_from_a_start_the_board_has_not_had_is_not_believed() {
    // the pair to the line above: RTC memory comes up as whatever the power left in it, and a note that
    // claims a start still to come is the one case where nobody should trust our words
    assert_eq!(when(8, 8), When::Impossible);
    assert_eq!(when(99, 8), When::Impossible);
    // and seq 0 is what an empty note carries: no start is numbered 0
    assert_eq!(when(0, 8), When::Impossible);
}

#[test]
fn the_line_of_this_start_names_where_and_what() {
    let said = words(&fall(7), 8);
    assert!(said.contains("0x42009abc"), "{said:?}");
    assert!(said.contains("StoreProhibited"), "{said:?}");
    assert!(said.contains("core 0"), "{said:?}");
    // and it says the fall is the one that brought this start, in words and not in a number to decode
    assert!(said.contains("this start"), "{said:?}");
}

#[test]
fn an_older_fall_says_how_far_back_it_is_in_the_same_line() {
    let said = words(&fall(5), 8);
    assert!(said.contains("3 starts ago"), "{said:?}");
    assert!(!said.contains("this start"), "{said:?}");
}

#[test]
fn a_note_that_cannot_be_true_says_so_instead_of_naming_a_place() {
    let said = words(&fall(99), 8);
    assert!(said.contains("not to be trusted"), "{said:?}");
}

#[test]
fn what_our_own_code_said_is_in_the_line_when_it_said_anything() {
    // a Rust panic knows the file and the line, which no address does; the chip never sees it
    let mut f = fall(7);
    f.exception = 3;
    f.reason = String::new();
    f.said = "devices/burner/src/fan.rs:88:21 attempt to divide by zero".into();
    let said = words(&f, 8);
    assert!(said.contains("fan.rs:88:21"), "{said:?}");
    assert!(said.contains("divide by zero"), "{said:?}");
}

#[test]
fn a_fall_our_code_said_nothing_about_carries_no_empty_quote() {
    // the pair: a hardware fault never goes through our code, and an empty pair of quotes reads as a message
    let said = words(&fall(7), 8);
    assert!(!said.contains("''"), "{said:?}");
    assert!(!said.contains("\"\""), "{said:?}");
    assert!(!said.contains("said"), "{said:?}");
}

#[test]
fn the_words_of_an_exception_tell_the_four_kinds_apart() {
    // which repair a person goes to: a watchdog is our code holding a core, an abort is our code giving up,
    // a fault is our code being wrong about an address
    assert!(exception_words(1).contains("interrupt watchdog"));
    assert!(exception_words(2).contains("task watchdog"));
    assert!(exception_words(3).contains("abort"));
    assert!(exception_words(4).contains("fault"));
    assert_ne!(exception_words(1), exception_words(2));
}

#[test]
fn an_exception_we_do_not_know_comes_out_as_the_number() {
    // the same discipline as reset_why: a kind invented to fit a familiar word sends a person to the wrong repair
    assert!(exception_words(77).contains("77"), "{:?}", exception_words(77));
    assert!(!exception_words(77).contains("fault"), "{:?}", exception_words(77));
}

#[test]
fn the_trace_is_the_addresses_and_the_way_to_turn_them_into_lines() {
    let line = trace_line(&fall(7), "burner").expect("a fall with a trace has a trace line");
    assert!(line.contains("0x42009abc"), "{line:?}");
    assert!(line.contains("0x4200912c"), "{line:?}");
    assert!(line.contains("0x4037a1c4"), "{line:?}");
    // the addresses are nothing to a person without the command that names them
    assert!(line.contains("where_fell.py"), "{line:?}");
    assert!(line.contains("burner"), "{line:?}");
    // and they stand in the order the stack gives them: the instruction that faulted first
    let first = line.find("0x42009abc").unwrap();
    assert!(first < line.find("0x4037a1c4").unwrap(), "{line:?}");
}

#[test]
fn a_fall_without_a_trace_has_no_trace_line_at_all() {
    // the pair: an empty "trace:" line reads as a stack of nothing, which is not what happened
    let mut f = fall(7);
    f.trace.clear();
    assert!(trace_line(&f, "burner").is_none());
}

#[test]
fn a_note_left_by_another_image_says_so_before_it_names_a_place() {
    // THE WORST WAY A NOTE CAN LIE. RTC memory outlives a firmware update, and an address from the build
    // before it points at whatever the new build put there - a real function, in the wrong code, and the line
    // reads exactly as confident as a true one.
    let mut f = fall(7);
    f.same_image = false;
    let said = words(&f, 8);
    assert!(said.contains("another image"), "{said:?}");
    // the addresses are still the evidence, so they are not thrown away - they are named as that build's
    assert!(said.contains("0x42009abc"), "{said:?}");
}

#[test]
fn a_note_from_the_running_image_says_nothing_about_images_at_all() {
    // the pair: a warning on every ordinary note is a warning nobody reads
    let said = words(&fall(7), 8);
    assert!(!said.contains("image"), "{said:?}");
}

// ---------------------------------------------------------------- both cores

#[test]
fn a_fall_on_a_two_core_chip_names_the_stack_of_each_core() {
    let said = lines(&fall(8), 8, "burner");
    assert_eq!(said.len(), 3, "a fall with two stacks said {said:#?}");
    assert!(said[1].contains("0x42009abc"), "the stack of the core that fell is not the first trace: {}", said[1]);
    assert!(said[2].contains("core 1") && said[2].contains("0x4037f26f 0x42130c79"), "the other core's stack: {}", said[2]);
    assert!(said[2].contains("tools/where_fell.py burner"), "the other core's addresses have no way to lines: {}", said[2]);
}

#[test]
fn a_core_that_left_no_stack_is_said_to_have_none_rather_than_left_out() {
    // the pair: a core that never reached the panic handler has nothing to walk, and silence would read as a core
    // nobody asked about
    let f = Fall { other_trace: vec![], ..fall(8) };
    let said = lines(&f, 8, "burner");
    assert_eq!(said.len(), 3, "a core without a stack went unmentioned: {said:#?}");
    assert!(said[2].contains("core 1") && said[2].contains("no stack"), "the other core: {}", said[2]);
}

#[test]
fn a_chip_with_one_core_says_nothing_about_another() {
    let f = Fall { other_core: None, other_trace: vec![], ..fall(8) };
    assert_eq!(lines(&f, 8, "burner").len(), 2, "a one-core chip was given a second core");
}
