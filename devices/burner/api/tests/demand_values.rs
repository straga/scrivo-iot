//! What the demand's module hands the stream for a second: what the house said and why, how long the room's sensor has been silent, the profile that stands with what chose it, the target of the room - each by code, the same addresses in every
//! state. Expected from what a person is told - cold or warm and the reason; none while nobody asks - and not from the code that makes the values.

use burner_api::demand::{put_values, Now};
use demand::{Asked, Demand, Why};
use page::dictionary::Value;
use schedule::{Profile, Source};
use std::collections::HashMap;

fn nothing() -> Now {
    Now { asked: None, silent_room: None, profile: None, source: None, until_min: None, target_c: None }
}

/// What the stream is told for NOW, by the name of the row.
fn said(now: &Now) -> HashMap<&'static str, Value> {
    let mut by_address: HashMap<usize, Value> = HashMap::new();
    put_values(now, &mut |at, value| {
        by_address.insert(at, value);
    });
    ["asked", "why", "silent_room", "profile", "source", "window", "until", "target"]
        .into_iter()
        .filter_map(|name| {
            // the window's number and the minute it ends at are parts of the source row
            let at = match name {
                "window" | "until" => page::dictionary::part_at(&burner_api::ALL, "demand", "source", name),
                _ => page::dictionary::address(&burner_api::ALL, "demand", name)?,
            };
            Some((name, *by_address.get(&at)?))
        })
        .collect()
}

#[test]
fn every_word_and_every_reason_is_said_by_its_own_code() {
    for (demand, word) in [(Demand::Cold, "cold"), (Demand::Warm, "warm")] {
        for (why, reason) in [
            (Why::BelowTarget, "below_target"),
            (Why::TargetReached, "target_reached"),
            (Why::ProfileOff, "profile_off"),
            (Why::ScheduledOn, "scheduled_on"),
            (Why::ScheduledOff, "scheduled_off"),
            (Why::SourceSilent, "source_silent"),
        ] {
            let v = said(&Now { asked: Some(Asked { demand, why }), ..nothing() });
            assert_eq!((v.get("asked"), v.get("why")), (Some(&Value::Word(word)), Some(&Value::Word(reason))), "{demand:?} {why:?}");
        }
    }
}

#[test]
fn where_nobody_asks_the_word_and_the_reason_are_none_by_a_code_of_their_own() {
    let v = said(&nothing());
    assert_eq!((v.get("asked"), v.get("why")), (Some(&Value::Unknown("none")), Some(&Value::Unknown("none"))));
}

#[test]
fn the_same_eight_rows_are_said_in_every_state_and_nothing_else() {
    for now in [nothing(), Now { asked: Some(Asked { demand: Demand::Cold, why: Why::BelowTarget }), profile: Some(Profile::Eco), source: Some(Source::Window(2)), until_min: Some(360), target_c: Some(17.0), silent_room: Some(40), ..nothing() }] {
        let mut names: Vec<&str> = said(&now).keys().copied().collect();
        names.sort();
        assert_eq!(names, ["asked", "profile", "silent_room", "source", "target", "until", "why", "window"]);
    }
}

#[test]
fn every_profile_is_said_by_its_own_code_and_no_profile_is_none() {
    for (profile, word) in [(Profile::Comfort, "comfort"), (Profile::Eco, "eco"), (Profile::Off, "off")] {
        let v = said(&Now { profile: Some(profile), source: Some(Source::Main), ..nothing() });
        assert_eq!(v.get("profile"), Some(&Value::Word(word)), "{profile:?}");
    }
    assert_eq!(said(&nothing()).get("profile"), Some(&Value::Unknown("none")), "nobody asking has no profile to say");
}

#[test]
fn a_window_is_said_as_the_word_window_and_its_number_and_the_minute_it_ends_at() {
    let v = said(&Now { profile: Some(Profile::Eco), source: Some(Source::Window(2)), until_min: Some(360), ..nothing() });
    assert_eq!(v.get("source"), Some(&Value::Word("window")));
    assert_eq!(v.get("window"), Some(&Value::Number(2.0)));
    assert_eq!(v.get("until"), Some(&Value::Number(360.0)));
}

#[test]
fn the_main_profile_names_no_window_and_no_end_and_a_board_that_does_not_know_the_time_says_so_by_a_code_of_its_own() {
    let v = said(&Now { profile: Some(Profile::Comfort), source: Some(Source::Main), ..nothing() });
    assert_eq!((v.get("source"), v.get("window"), v.get("until")), (Some(&Value::Word("main")), Some(&Value::Unknown("none")), Some(&Value::Unknown("none"))));
    let v = said(&Now { profile: Some(Profile::Comfort), source: Some(Source::NoClock), ..nothing() });
    assert_eq!((v.get("source"), v.get("profile")), (Some(&Value::Word("no_clock")), Some(&Value::Word("comfort"))), "the main profile stands, and the card still says which");
}

#[test]
fn the_target_is_degrees_and_none_where_no_room_is_held() {
    assert_eq!(said(&Now { target_c: Some(21.0), ..nothing() }).get("target"), Some(&Value::Number(21.0)));
    assert_eq!(said(&nothing()).get("target"), Some(&Value::Unknown("none")));
}

#[test]
fn the_silence_of_the_room_is_said_by_the_seconds_and_by_nothing_while_it_speaks() {
    assert_eq!(said(&Now { silent_room: Some(40), ..nothing() }).get("silent_room"), Some(&Value::Seconds(40)));
    assert_eq!(said(&nothing()).get("silent_room"), Some(&Value::None), "a sensor that speaks, or none given the role, says nothing");
}
