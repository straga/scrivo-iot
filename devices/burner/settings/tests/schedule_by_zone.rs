//! A window opens at its LOCAL time, before and after the clocks change: the board's wall time is the local time of its zone (`firmware::clock::now`, libc's `localtime_r` under the POSIX rule
//! of the setting `zone`), so a window from 22:00 to 06:00 opens when the clock on the wall reads 22:00 - at 20:00 UTC in winter and at 19:00 UTC in summer - and the night the clocks change is seven hours long in March and nine in October.
//!
//! The instants are synthetic and the expectations are the zone's own: GMT+2 is Eastern European time, +2 in winter and +3 from the last Sunday of March, 03:00 (01:00 UTC), to the last Sunday of October, 04:00 summer time
//! (01:00 UTC). In 2026 those Sundays are the 29th of March and the 25th of October. The local time is made by the host's libc from the rule `settings::posix` gives - the same rule the board's newlib reads, which this does not
//! run: what is held here is what the schedule makes of the wall time it is given across the change, and the day of the week from the date.

use std::ffi::CString;
use std::sync::Mutex;

use schedule::{Profile, Wall};
use settings::{posix, Setting};
use burner_settings::{demand_tuning};

// POSIX's tzset: the libc crate has it on some hosts and not on others
extern "C" {
    fn tzset();
}

/// One test at a time sets the zone: the zone is the process's.
static ZONE: Mutex<()> = Mutex::new(());

/// The local wall time of the UTC instant (year, month, day, hour, minute) under the POSIX RULE, and the day of the week libc says it is (Sunday nought).
fn local(rule: &str, (y, mo, d, h, mi): (i32, i32, i32, i32, i32)) -> (Wall, i32) {
    let rule = CString::new(rule).unwrap();
    unsafe {
        libc::setenv(c"TZ".as_ptr(), rule.as_ptr(), 1);
        tzset();
        let mut utc: libc::tm = std::mem::zeroed();
        (utc.tm_year, utc.tm_mon, utc.tm_mday, utc.tm_hour, utc.tm_min) = (y - 1900, mo - 1, d, h, mi);
        let at: libc::time_t = libc::timegm(&mut utc);
        let mut local: libc::tm = std::mem::zeroed();
        libc::localtime_r(&at, &mut local);
        let wall = Wall::of_date((local.tm_year + 1900) as u16, (local.tm_mon + 1) as u8, local.tm_mday as u8, (local.tm_hour * 60 + local.tm_min) as u16);
        (wall, local.tm_wday)
    }
}

fn window_night() -> demand::Tuning {
    let texts = |s: &Setting| match s.name {
        "window_1" => "set:daily 22:00-06:00 eco".to_string(),
        _ => s.default.to_string(),
    };
    demand_tuning(&texts)
}

/// The profile the schedule stands on at the UTC instant, by the zone RULE.
fn profile_at(rule: &str, tuning: &demand::Tuning, at: (i32, i32, i32, i32, i32)) -> Option<Profile> {
    let (wall, wday) = local(rule, at);
    assert_eq!(i32::from(wall.day), (wday + 6) % 7, "the day of the week of {at:?} is what libc says, counted from Monday");
    Some(demand::Player::new().ask(tuning, None, Some(wall)).profile)
}

const ECO: Option<Profile> = Some(Profile::Eco);
const COMFORT: Option<Profile> = Some(Profile::Comfort);

#[test]
fn the_zone_the_owner_lives_in_has_a_summer_time_and_the_one_that_does_not_move_has_none() {
    assert!(posix("gmt+2").contains("EEST"), "{}", posix("gmt+2"));
    assert!(!posix("gmt+3").contains(',') && !posix("utc").contains(','), "{} {}", posix("gmt+3"), posix("utc"));
}

#[test]
fn a_window_through_the_night_opens_at_the_local_time_before_and_after_the_clocks_go_forward() {
    let _zone = ZONE.lock().unwrap_or_else(|e| e.into_inner());
    let sound = window_night();
    let rule = posix("gmt+2");
    let at = |day: i32, h: i32, mi: i32| profile_at(rule, &sound, (2026, 3, day, h, mi));
    // Saturday the 28th, still winter time (+2): 22:00 on the wall is 20:00 UTC
    assert_eq!(at(28, 19, 59), COMFORT, "21:59 by the wall");
    assert_eq!(at(28, 20, 0), ECO, "22:00 by the wall: the window opens");
    // the night the clocks go forward: 03:00 becomes 04:00 at 01:00 UTC, and the window is still open through it
    assert_eq!(at(29, 0, 59), ECO, "02:59 by the wall");
    assert_eq!(at(29, 1, 0), ECO, "01:00 UTC is 04:00 by the wall: the hour that was skipped does not close or reopen it");
    assert_eq!(at(29, 2, 59), ECO, "05:59 by the wall, summer time");
    assert_eq!(at(29, 3, 0), COMFORT, "06:00 by the wall: it closes at 03:00 UTC, seven hours after it opened");
    // Sunday evening, summer time (+3): 22:00 on the wall is 19:00 UTC, an hour sooner by the world's clock and the same by the wall's
    assert_eq!(at(29, 18, 59), COMFORT, "21:59 by the wall");
    assert_eq!(at(29, 19, 0), ECO, "22:00 by the wall: the window opens");
    assert_eq!(at(30, 2, 59), ECO, "05:59 by the wall");
    assert_eq!(at(30, 3, 0), COMFORT, "06:00 by the wall");
}

#[test]
fn a_window_through_the_night_opens_at_the_local_time_before_and_after_the_clocks_go_back() {
    let _zone = ZONE.lock().unwrap_or_else(|e| e.into_inner());
    let sound = window_night();
    let rule = posix("gmt+2");
    let at = |day: i32, h: i32, mi: i32| profile_at(rule, &sound, (2026, 10, day, h, mi));
    // Saturday the 24th, summer time (+3): 22:00 on the wall is 19:00 UTC
    assert_eq!(at(24, 18, 59), COMFORT, "21:59 by the wall");
    assert_eq!(at(24, 19, 0), ECO, "22:00 by the wall: the window opens");
    // the night the clocks go back: 04:00 becomes 03:00 at 01:00 UTC, and the hour that comes twice is in the window both times
    assert_eq!(at(25, 0, 59), ECO, "03:59 by the wall, summer time");
    assert_eq!(at(25, 1, 0), ECO, "01:00 UTC is 03:00 by the wall again");
    assert_eq!(at(25, 3, 59), ECO, "05:59 by the wall, winter time");
    assert_eq!(at(25, 4, 0), COMFORT, "06:00 by the wall: it closes at 04:00 UTC, nine hours after it opened");
    // Sunday evening, winter time (+2): 22:00 on the wall is 20:00 UTC
    assert_eq!(at(25, 19, 59), COMFORT, "21:59 by the wall");
    assert_eq!(at(25, 20, 0), ECO, "22:00 by the wall: the window opens");
}

#[test]
fn a_zone_that_does_not_move_opens_the_window_at_the_same_world_time_all_year() {
    let _zone = ZONE.lock().unwrap_or_else(|e| e.into_inner());
    let sound = window_night();
    let rule = posix("gmt+3");
    for (month, day) in [(1, 15), (3, 29), (7, 1), (10, 25)] {
        assert_eq!(profile_at(rule, &sound, (2026, month, day, 18, 59)), COMFORT, "21:59 by the wall, {month}-{day}");
        assert_eq!(profile_at(rule, &sound, (2026, month, day, 19, 0)), ECO, "22:00 by the wall, {month}-{day}");
    }
}

#[test]
fn a_window_from_to_across_the_gap_of_the_spring_night_opens_at_the_first_minute_the_wall_reads_inside_it() {
    let _zone = ZONE.lock().unwrap_or_else(|e| e.into_inner());
    // 03:30 to 05:00 by the wall: on the night the clocks go forward the wall goes from 02:59 to 04:00 and never reads 03:30
    let texts = |s: &Setting| match s.name {
        "window_1" => "set:daily 03:30-05:00 eco".to_string(),
        _ => s.default.to_string(),
    };
    let sound = demand_tuning(&texts);
    let rule = posix("gmt+2");
    let at = |day: i32, h: i32, mi: i32| profile_at(rule, &sound, (2026, 3, day, h, mi));
    assert_eq!(at(29, 0, 59), COMFORT, "02:59 by the wall");
    assert_eq!(at(29, 1, 0), ECO, "04:00 by the wall: inside the window, which has not been 'opened' by a reading of 03:30");
    assert_eq!(at(29, 1, 59), ECO, "04:59");
    assert_eq!(at(29, 2, 0), COMFORT, "05:00 by the wall: closed");
    // an ordinary day, winter time: 03:30 by the wall is 01:30 UTC
    assert_eq!(at(28, 1, 29), COMFORT);
    assert_eq!(at(28, 1, 30), ECO);
}

#[test]
fn the_control_a_clock_that_is_the_worlds_and_not_the_zones_opens_the_window_at_other_world_times() {
    // what the rows above would say if the wall time were UTC: 22:00 by that wall is 22:00 UTC, two hours after the zone's
    let _zone = ZONE.lock().unwrap_or_else(|e| e.into_inner());
    let sound = window_night();
    assert_eq!(profile_at("UTC0", &sound, (2026, 3, 28, 20, 0)), COMFORT, "20:00 UTC is not 22:00 by a wall that keeps UTC");
    assert_eq!(profile_at("UTC0", &sound, (2026, 3, 28, 22, 0)), ECO);
    assert_eq!(profile_at(posix("gmt+2"), &sound, (2026, 3, 28, 20, 0)), ECO, "and by the zone's wall it is");
}
