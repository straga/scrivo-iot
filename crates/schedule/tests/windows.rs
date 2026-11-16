//! How a window is written, and what is no window: `<days> <from>-<to> <profile>` and `<days> <from>+<N>h/<M>m <profile>`.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: days are `daily`, a name (`mon`), a range (`mon-fri`, `sat-mon` goes through the
//! end of the week) and lists of those; times are `HH:MM`; a window is never empty and never the whole day, so `from` is not `to`; "every"
//! is N from 1 to 24 hours and M minutes, and M is not longer than the N hours (the next one would start inside it); the profile is
//! `comfort`, `eco`, `off` - or `none`, which leaves the window written and off the schedule.

use schedule::{Profile, Span, Window};

const ALL: u8 = 0b111_1111;

fn ok(text: &str) -> Window {
    Window::parse(text).unwrap_or_else(|why| panic!("{text:?} should be a window: {why}"))
}

fn refused(text: &str) {
    assert!(Window::parse(text).is_err(), "{text:?} is no window");
}

#[test]
fn a_window_is_days_then_times_then_a_profile() {
    assert_eq!(ok("daily 22:00-06:00 eco"), Window { days: ALL, span: Span::Between { from: 1320, to: 360 }, profile: Some(Profile::Eco) });
    assert_eq!(ok("mon-fri 06:30-08:00 comfort"), Window { days: 0b001_1111, span: Span::Between { from: 390, to: 480 }, profile: Some(Profile::Comfort) });
    assert_eq!(ok("sat-sun 08:00-10:00 off"), Window { days: 0b110_0000, span: Span::Between { from: 480, to: 600 }, profile: Some(Profile::Off) });
}

#[test]
fn days_are_names_ranges_and_lists_of_them() {
    assert_eq!(ok("mon,wed,fri 07:00-09:00 eco").days, 0b001_0101);
    assert_eq!(ok("mon-wed,sat 07:00-09:00 eco").days, 0b010_0111);
    assert_eq!(ok("sun 07:00-09:00 eco").days, 0b100_0000);
    assert_eq!(ok("sat-mon 07:00-09:00 eco").days, 0b110_0001, "a range that runs past the end of the week comes round to Monday");
    assert_eq!(ok("mon-sun 07:00-09:00 eco").days, ALL);
}

#[test]
fn every_n_hours_for_m_minutes_from_a_time() {
    assert_eq!(ok("daily 07:00+3h/20m comfort"), Window { days: ALL, span: Span::Every { from: 420, every_h: 3, for_min: 20 }, profile: Some(Profile::Comfort) });
    assert_eq!(ok("mon-fri 06:15+24h/60m eco").span, Span::Every { from: 375, every_h: 24, for_min: 60 });
    assert_eq!(ok("daily 00:00+1h/60m eco").span, Span::Every { from: 0, every_h: 1, for_min: 60 }, "as long as the gap is allowed");
}

#[test]
fn a_profile_of_none_leaves_the_window_written_and_off() {
    assert_eq!(ok("daily 22:00-06:00 none").profile, None);
}

#[test]
fn capitals_and_spaces_around_the_words_are_no_matter() {
    assert_eq!(ok("  Mon-Fri   06:30-08:00   COMFORT "), ok("mon-fri 06:30-08:00 comfort"));
}

#[test]
fn what_is_no_window_is_refused() {
    for text in [
        "",
        "daily",
        "daily 22:00-06:00",
        "daily 22:00-06:00 eco extra",
        "daily 22:00-22:00 eco",
        "daily 24:00-06:00 eco",
        "daily 22:60-06:00 eco",
        "daily 7:00-9:00 eco",
        "daily 22:00-6:00 eco",
        "someday 22:00-06:00 eco",
        "mon- 22:00-06:00 eco",
        "-fri 22:00-06:00 eco",
        "mon,,fri 22:00-06:00 eco",
        "daily 22:00-06:00 warm",
        "daily 07:00+0h/20m comfort",
        "daily 07:00+25h/20m comfort",
        "daily 07:00+3h/0m comfort",
        "daily 07:00+3h/181m comfort",
        "daily 07:00+3h/20 comfort",
        "daily 07:00+3/20m comfort",
        "daily 07:00+h/20m comfort",
    ] {
        refused(text);
    }
}
