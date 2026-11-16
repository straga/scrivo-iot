//! The planned restart, expected from what the task says: once a day from the set hour, only with the burner standing with no fire, never twice a day, never with the count owed; and one note for each case of a silent draught.

use daily::{Clues, Daily, Hour, Moment, Postponed, Request, Settled, Verdict, Why};
use schedule::Wall;

const THREE: u16 = 3 * 60;
const DAY_S: u64 = 24 * 3600;

fn daily() -> Daily {
    Daily { from_minute: THREE }
}

fn at(minute: u16, up_s: u64, standing: bool) -> Moment {
    Moment { wall: Some(Wall { day: 2, minute }), up_s, standing, fault: false, owed: false, auger_closed: false, igniter_closed: false }
}

#[test]
fn a_board_that_stands_with_no_fire_restarts_at_the_hour() {
    assert_eq!(daily().judge(at(THREE, DAY_S, true)), Verdict::Restart);
    assert_eq!(daily().judge(at(THREE + 40, DAY_S + 2400, true)), Verdict::Restart, "still due when the hour has gone");
}

#[test]
fn a_board_before_the_hour_does_not_restart() {
    assert_eq!(daily().judge(at(THREE - 1, DAY_S, true)), Verdict::Not);
}

#[test]
fn a_burner_with_fire_puts_the_restart_off_until_it_stands_and_then_it_happens() {
    assert_eq!(daily().judge(at(THREE, DAY_S, false)), Verdict::Waits(Why::Fire));
    assert_eq!(daily().judge(at(THREE + 90, DAY_S + 5400, false)), Verdict::Waits(Why::Fire));
    assert_eq!(daily().judge(at(THREE + 91, DAY_S + 5460, true)), Verdict::Restart);
}

#[test]
fn a_board_that_has_just_restarted_does_not_restart_again_in_the_same_hour_or_day() {
    assert_eq!(daily().judge(at(THREE, 5, true)), Verdict::Not, "up for seconds at the hour it restarted at");
    assert_eq!(daily().judge(at(THREE + 30, 1800, true)), Verdict::Not);
    assert_eq!(daily().judge(at(15 * 60, 12 * 3600, true)), Verdict::Not, "twelve hours after the restart at the hour");
    assert_eq!(daily().judge(at(23 * 60, 20 * 3600, true)), Verdict::Not, "before the next night");
    assert_eq!(daily().judge(at(THREE + 40, 40 * 60 + 20, true)), Verdict::Not, "a restart the fire put off to 03:40 is not repeated at 04:20");
}

#[test]
fn a_board_that_started_a_moment_before_the_hour_waits_for_the_next_night() {
    assert_eq!(daily().judge(at(THREE, 5 * 60, true)), Verdict::Not);
    assert_eq!(daily().judge(at(THREE, 11 * 60, true)), Verdict::Restart, "started eleven minutes before the hour");
}

#[test]
fn a_board_started_by_hand_at_noon_restarts_the_next_night() {
    assert_eq!(daily().judge(at(THREE, 15 * 3600, true)), Verdict::Restart);
}

#[test]
fn a_count_the_flash_has_not_taken_holds_the_restart_back() {
    let owed = Moment { owed: true, ..at(THREE, DAY_S, true) };
    assert_eq!(daily().judge(owed), Verdict::Waits(Why::CountOwed));
}

#[test]
fn a_fault_is_not_restarted_under_it_and_is_not_waited_out() {
    let fault = Moment { fault: true, standing: true, ..at(THREE, DAY_S, true) };
    assert_eq!(daily().judge(fault), Verdict::Waits(Why::Fault));
    assert_eq!(daily().judge(Moment { fault: true, ..at(THREE - 1, DAY_S, true) }), Verdict::Not, "nothing is due before the hour, fault or not");
}

#[test]
fn a_board_with_no_clock_restarts_by_its_uptime_alone_and_only_standing() {
    let blind = |up_s, standing| Moment { wall: None, up_s, standing, fault: false, owed: false, auger_closed: false, igniter_closed: false };
    assert_eq!(daily().judge(blind(26 * 3600, true)), Verdict::Restart);
    assert_eq!(daily().judge(blind(26 * 3600, false)), Verdict::Waits(Why::Fire));
    assert_eq!(daily().judge(blind(5 * 3600, true)), Verdict::Not);
}

#[test]
fn a_silence_of_ten_seconds_is_not_noted_and_one_second_more_is_noted_once() {
    let mut clues = Clues::default();
    assert!(!clues.see(None));
    assert!(!clues.see(Some(10)));
    assert!(clues.see(Some(11)), "the first second past ten");
    for s in 12..120 {
        assert!(!clues.see(Some(s)), "second {s} of the same case");
    }
}

#[test]
fn a_draught_that_speaks_and_goes_silent_again_is_a_new_case() {
    let mut clues = Clues::default();
    assert!(clues.see(Some(42)));
    assert!(!clues.see(None));
    assert!(!clues.see(Some(3)));
    assert!(clues.see(Some(11)));
}

#[test]
fn a_stop_the_automaton_decided_but_with_the_auger_closed_by_a_hand_does_not_restart() {
    let by_hand = Moment { auger_closed: true, ..at(THREE, DAY_S, true) };
    assert_eq!(daily().judge(by_hand), Verdict::Waits(Why::Organ), "the auger turns where the automaton says nothing");
    assert_eq!(daily().judge(Moment { auger_closed: false, ..by_hand }), Verdict::Restart, "the same stop with the auger open goes");
}

#[test]
fn a_stop_with_the_plug_closed_by_a_hand_does_not_restart() {
    let by_hand = Moment { igniter_closed: true, ..at(THREE, DAY_S, true) };
    assert_eq!(daily().judge(by_hand), Verdict::Waits(Why::Organ));
    assert_eq!(daily().judge(Moment { igniter_closed: false, ..by_hand }), Verdict::Restart);
}

#[test]
fn a_relay_closed_before_the_hour_changes_nothing_and_a_fire_is_still_a_fire() {
    assert_eq!(daily().judge(Moment { auger_closed: true, ..at(THREE - 1, DAY_S, true) }), Verdict::Not, "nothing is due before the hour");
    assert_eq!(daily().judge(Moment { auger_closed: true, ..at(THREE, DAY_S, false) }), Verdict::Waits(Why::Fire), "the fire is named first");
}

#[test]
fn a_restart_nobody_asked_for_is_idle_and_an_asked_one_goes_when_the_journal_is_down_and_no_relay_is_closed() {
    let mut request = Request::new();
    assert_eq!(request.settle(true, false), Settled::Idle);
    request.ask();
    assert_eq!(request.settle(true, false), Settled::Go);
}

#[test]
fn a_journal_the_flash_refused_holds_the_restart_and_the_next_pass_with_the_journal_down_lets_it_go() {
    let mut request = Request::new();
    request.ask();
    assert_eq!(request.settle(false, false), Settled::Waits { why: Postponed::Journal, new: true });
    assert_eq!(request.settle(false, false), Settled::Waits { why: Postponed::Journal, new: false }, "said once");
    assert_eq!(request.settle(true, false), Settled::Go);
}

#[test]
fn a_relay_closed_after_the_restart_was_asked_for_holds_it_and_an_opened_one_lets_it_go() {
    let mut request = Request::new();
    request.ask();
    assert_eq!(request.settle(true, true), Settled::Waits { why: Postponed::Organ, new: true });
    assert_eq!(request.settle(true, false), Settled::Go);
}

#[test]
fn a_cancelled_request_is_idle_and_a_journal_that_refused_is_not_forgotten_by_a_new_reason() {
    let mut request = Request::new();
    request.ask();
    assert_eq!(request.settle(false, false), Settled::Waits { why: Postponed::Journal, new: true });
    assert_eq!(request.settle(false, true), Settled::Waits { why: Postponed::Journal, new: false }, "the journal is the first reason: it is not said again as a relay");
    request.cancel();
    assert_eq!(request.settle(true, false), Settled::Idle);
}

#[test]
fn a_relay_that_held_the_restart_and_then_a_journal_that_refused_are_two_reasons_said_in_turn() {
    let mut request = Request::new();
    request.ask();
    assert_eq!(request.settle(true, true), Settled::Waits { why: Postponed::Organ, new: true });
    assert_eq!(request.settle(false, true), Settled::Waits { why: Postponed::Journal, new: true });
    assert_eq!(request.settle(true, false), Settled::Go);
    request.ask();
    assert_eq!(request.settle(true, true), Settled::Waits { why: Postponed::Organ, new: true }, "a request that went starts again from nothing said");
}

#[test]
fn the_hour_off_never_restarts_and_an_hour_does_at_that_hour() {
    let blind = |up_s, standing| Moment { wall: None, up_s, standing, fault: false, owed: false, auger_closed: false, igniter_closed: false };
    assert_eq!(Hour::Off.judge(at(THREE, DAY_S, true)), Verdict::Not, "the hour, the day up, standing - and nothing is due");
    assert_eq!(Hour::Off.judge(blind(40 * 3600, true)), Verdict::Not, "no clock and forty hours up");
    assert_eq!(Hour::At(THREE).judge(at(THREE, DAY_S, true)), Verdict::Restart);
    assert_eq!(Hour::At(THREE).judge(blind(40 * 3600, true)), Verdict::Restart);
}

#[test]
fn the_setting_is_read_as_off_an_hour_or_the_default_hour() {
    assert_eq!(Hour::of("off"), Hour::Off);
    assert_eq!(Hour::of("3"), Hour::At(180));
    assert_eq!(Hour::of("0"), Hour::At(0));
    assert_eq!(Hour::of("23"), Hour::At(23 * 60));
    assert_eq!(Hour::of("27"), Hour::At(23 * 60), "past the last hour is the last hour");
    assert_eq!(Hour::of("tomorrow"), Hour::At(THREE), "not a setting: the default stands, not off");
    assert_eq!(Hour::of(""), Hour::At(THREE));
}
