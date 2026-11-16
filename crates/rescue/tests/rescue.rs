//! A station that cannot join: two joins in a row that fail at an access point put it off and
//! the next try goes to the next strongest of the same network after a fresh scan; when every access point is put off a
//! new lap begins; two laps without a join reset the radio, two more restart the board; a join starts everything over.
//! 05.10 the burner took the near access point on channel 1 every ~74 s from 02:00 to the morning, the far one standing by.

use rescue::{Next, Rescue, Rules};
use roam::Found;

const R: Rules = Rules { tries: 2, reset_laps: 2, reboot_laps: 2 };
const NEAR: [u8; 6] = [0xdc, 0x2c, 0x6e, 0x7f, 0x39, 0x4f];
const FAR: [u8; 6] = [0xdc, 0x2c, 0x6e, 0x7f, 0x39, 0x50];
const OTHER: [u8; 6] = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60];

fn ap(ssid: &str, bssid: [u8; 6], channel: u8, rssi: i8) -> Found {
    Found { ssid: ssid.into(), bssid, channel, rssi }
}

/// The scan of 05.10: the near access point strong on channel 1, the far one weaker on channel 13, a neighbour's
/// network stronger than both.
fn scan() -> Vec<Found> {
    vec![ap("GZ103", FAR, 13, -71), ap("GZ103", NEAR, 1, -55), ap("neighbour", OTHER, 6, -40)]
}

/// What a step looks like in a sequence.
#[derive(Debug, PartialEq, Clone, Copy)]
enum S {
    Near,
    Far,
    Any,
    Reset,
    Restart,
}

fn name(next: &Next) -> S {
    match next {
        Next::Try(f) if f.bssid == NEAR => S::Near,
        Next::Try(f) if f.bssid == FAR => S::Far,
        Next::Try(f) => panic!("a try at {:?}, which is no access point of the network", f.bssid),
        Next::Any => S::Any,
        Next::ResetRadio => S::Reset,
        Next::Restart => S::Restart,
    }
}

/// N steps asked for in a row, each join failing: what each step was. A reset or a restart is a step, and no join
/// follows it that could fail.
fn fail_all(rescue: &mut Rescue, found: &[Found], n: usize, r: &Rules) -> Vec<S> {
    (0..n)
        .map(|_| {
            let next = rescue.next("GZ103", found, r);
            match &next {
                Next::Try(f) => {
                    rescue.failed(Some(f.bssid), r);
                }
                Next::Any => {
                    rescue.failed(None, r);
                }
                Next::ResetRadio | Next::Restart => {}
            }
            name(&next)
        })
        .collect()
}

use S::{Any, Far, Near, Reset, Restart};

#[test]
fn the_first_try_goes_to_the_strongest_access_point_of_the_network() {
    assert_eq!(fail_all(&mut Rescue::new(), &scan(), 1, &R), [Near]);
}

#[test]
fn one_failure_does_not_put_an_access_point_off_and_two_in_a_row_do() {
    assert_eq!(fail_all(&mut Rescue::new(), &scan(), 4, &R), [Near, Near, Far, Far]);
}

#[test]
fn another_network_is_not_taken_though_it_is_stronger() {
    // the pair of the two above: the neighbour at -40 is the strongest of the scan; fail_all panics on a try at it
    assert_eq!(fail_all(&mut Rescue::new(), &scan(), 8, &R)[..4], [Near, Near, Far, Far]);
}

#[test]
fn when_every_access_point_is_put_off_a_new_lap_begins_with_fresh_counts() {
    assert_eq!(fail_all(&mut Rescue::new(), &scan(), 8, &R), [Near, Near, Far, Far, Near, Near, Far, Far]);
}

#[test]
fn two_laps_without_a_join_reset_the_radio_and_two_more_restart_the_board() {
    assert_eq!(
        fail_all(&mut Rescue::new(), &scan(), 18, &R),
        [Near, Near, Far, Far, Near, Near, Far, Far, Reset, Near, Near, Far, Far, Near, Near, Far, Far, Restart],
    );
}

#[test]
fn each_step_is_taken_once() {
    let steps = fail_all(&mut Rescue::new(), &scan(), 60, &R);
    assert_eq!(steps.iter().filter(|s| **s == Reset).count(), 1);
    assert_eq!(steps.iter().filter(|s| **s == Restart).count(), 1);
}

#[test]
fn a_join_starts_every_count_over() {
    let mut rescue = Rescue::new();
    // the near one is put off and a lap and a half done, then the station joins
    assert_eq!(fail_all(&mut rescue, &scan(), 6, &R), [Near, Near, Far, Far, Near, Near]);
    rescue.joined();
    assert_eq!(fail_all(&mut rescue, &scan(), 1, &R), [Near], "the first try goes to the strongest again");
    // and the ladder starts from nothing: two laps are 8 tries again, not the 2 that were left
    let mut rescue = Rescue::new();
    fail_all(&mut rescue, &scan(), 6, &R);
    rescue.joined();
    assert_eq!(fail_all(&mut rescue, &scan(), 9, &R), [Near, Near, Far, Far, Near, Near, Far, Far, Reset]);
}

#[test]
fn a_join_after_the_reset_puts_the_ladder_back_on_its_first_step() {
    let mut rescue = Rescue::new();
    assert_eq!(fail_all(&mut rescue, &scan(), 9, &R)[8], Reset);
    rescue.joined();
    // the restart does not come two laps after the reset's: the radio is reset first
    assert_eq!(fail_all(&mut rescue, &scan(), 9, &R)[8], Reset);
}

#[test]
fn a_network_with_one_access_point_is_tried_in_laps_of_its_tries() {
    let one = vec![ap("GZ103", NEAR, 1, -55)];
    assert_eq!(fail_all(&mut Rescue::new(), &one, 5, &R), [Near, Near, Near, Near, Reset]);
}

#[test]
fn a_network_the_scan_does_not_show_is_left_to_the_driver_and_each_failure_is_a_lap() {
    let seen = vec![ap("neighbour", OTHER, 6, -40)];
    assert_eq!(fail_all(&mut Rescue::new(), &seen, 3, &R), [Any, Any, Reset]);
    assert_eq!(fail_all(&mut Rescue::new(), &[], 1, &R), [Any], "an empty scan");
}

#[test]
fn a_put_off_access_point_that_is_gone_from_the_scan_does_not_hold_the_lap_back() {
    let mut rescue = Rescue::new();
    assert_eq!(fail_all(&mut rescue, &scan(), 2, &R), [Near, Near]);
    // the near one vanished: the far one is tried, and its two failures end the lap with nothing else to wait for
    let only_far = vec![ap("GZ103", FAR, 13, -71)];
    assert_eq!(fail_all(&mut rescue, &only_far, 3, &R), [Far, Far, Far]);
}

#[test]
fn the_numbers_come_from_the_rules() {
    let once = Rules { tries: 1, reset_laps: 1, reboot_laps: 1 };
    assert_eq!(fail_all(&mut Rescue::new(), &scan(), 6, &once), [Near, Far, Reset, Near, Far, Restart]);
}

#[test]
fn a_failure_says_whether_it_put_the_access_point_off() {
    let mut rescue = Rescue::new();
    assert!(!rescue.failed(Some(NEAR), &R), "the first of two");
    assert!(rescue.failed(Some(NEAR), &R), "the second of two");
    assert!(!rescue.failed(Some(NEAR), &R), "a third at a point that is put off already says nothing new");
    assert!(!rescue.failed(None, &R), "a network the scan does not show has no access point to put off");
}
