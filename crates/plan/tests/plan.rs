//! The two plans against what was measured, and the choice between them against what the build was told.

use plan::{Plan, LARGE, PLAN, SMALL};

/// The room a stack keeps under load (`firmware::board::ROOMY`).
const ROOMY: usize = 1536;

/// Bytes each stack was seen to use under an open page with both browser consoles, the streams, the state, the page's
/// files and a relay clicked every ten seconds, on the S3 built without PSRAM. The ones the small
/// plan shrinks.
const USED_NET: usize = 10_100;
const USED_FLASH: usize = 3_300;
const USED_USB_CONSOLE: usize = 3_200;

/// The head of a request a browser sends the page: the longest one a page's own requests make.
const LONGEST_HEAD: usize = 900;

/// A stack is a multiple of 512, as every stack of this project is.
fn rounded(n: usize) -> bool {
    n % 512 == 0
}

#[test]
fn the_large_plan_is_what_the_burner_ran_with_before_the_plans_were() {
    assert_eq!(
        LARGE,
        Plan {
            net_stack: 16_896,
            flash_stack: 8_192,
            usb_console_stack: 8_192,
            head_buffer: 2_048,
            log_ring: 64,
            to_flash_lines: 32,
            handlers: 6,
        }
    );
}

#[test]
fn the_small_plan_takes_less_than_the_large_in_every_field_it_changes_and_never_more() {
    let (s, l) = (SMALL, LARGE);
    assert!(s.net_stack < l.net_stack);
    assert!(s.flash_stack < l.flash_stack);
    assert!(s.usb_console_stack < l.usb_console_stack);
    assert!(s.head_buffer < l.head_buffer);
    assert!(s.log_ring < l.log_ring);
    assert!(s.to_flash_lines < l.to_flash_lines);
    assert!(s.handlers < l.handlers);
}

#[test]
fn the_small_stacks_keep_the_room_a_stack_keeps_over_what_the_task_was_seen_to_use() {
    assert!(SMALL.net_stack >= USED_NET + ROOMY, "net {}", SMALL.net_stack);
    assert!(SMALL.flash_stack >= USED_FLASH + ROOMY, "flash {}", SMALL.flash_stack);
    assert!(SMALL.usb_console_stack >= USED_USB_CONSOLE + ROOMY, "usb console {}", SMALL.usb_console_stack);
    for stack in [SMALL.net_stack, SMALL.flash_stack, SMALL.usb_console_stack] {
        assert!(rounded(stack), "{stack} is not a multiple of 512");
    }
}

#[test]
fn the_small_head_buffer_takes_the_longest_head_a_page_request_makes() {
    assert!(SMALL.head_buffer >= LONGEST_HEAD, "{}", SMALL.head_buffer);
}

#[test]
fn the_small_log_has_room_for_a_burst_of_the_start_and_a_console_that_connects_late() {
    // the lines of a start the loop takes between two polls, and the last of the log for a console that comes in
    assert!(SMALL.log_ring >= 16);
    assert!(SMALL.to_flash_lines >= 8);
}

#[test]
fn the_plan_of_the_image_is_the_small_one_when_the_build_was_told_there_is_no_psram_and_the_large_otherwise() {
    let told = std::env::var("STRAGA_PSRAM").ok();
    let want = if told.as_deref() == Some("0") { SMALL } else { LARGE };
    assert_eq!(PLAN, want, "the build was told STRAGA_PSRAM={told:?}");
}

/// What a browser holds on the page at once: its stream and, in a tab that opens it, its console - for each of two people - and the short
/// requests beside them (six handlers for six connections of a browser).
const STREAMS_AND_CONSOLES_AT_MOST: usize = 4;

#[test]
fn a_handler_is_always_left_for_the_short_requests_whatever_the_long_ones_hold() {
    for plan in [LARGE, SMALL] {
        assert!(plan.long_holders() < plan.handlers, "{plan:?}");
        assert!(plan.long_holders() >= 1, "no stream could be held: {plan:?}");
    }
}

#[test]
fn the_small_plan_has_three_handlers_a_stream_a_console_and_the_short_requests_and_the_large_keeps_its_six() {
    assert_eq!(SMALL.handlers, 3);
    assert_eq!(SMALL.long_holders(), 2);
    assert_eq!(LARGE.handlers, 6);
}

#[test]
fn the_large_plan_holds_every_long_connection_the_burner_ever_admitted() {
    // two streams and two browser consoles are admitted by the burner's page (web::HANDLERS = 6); the rule of the long holders must not refuse a fifth of them
    assert!(LARGE.long_holders() >= STREAMS_AND_CONSOLES_AT_MOST, "{}", LARGE.long_holders());
}
