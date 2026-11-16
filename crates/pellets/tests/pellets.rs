//! What the count of pellets says. The rate is what a person weighed: grams of pellets the auger pours in a minute of
//! turning without a stop (240 g/min is 4 g/s, the owner's weighing of 04.10). Expected from that - the seconds the
//! auger turned times the rate - and from what a person reading "today", "this month" and "in all" would say; not from
//! the code that counts.

use pellets::{Date, Now, Pellets, MAX_STEP_MS};

const RATE: u32 = 240;

fn date(year: u16, month: u8, day: u8) -> Date {
    Date { year, month, day }
}

fn now(d: Date) -> Option<Now> {
    Some(Now { date: d, minute: 12 * 60, offset_min: 180 })
}

/// A count that knows the day, asked every second from SECOND 0.
fn known() -> Pellets {
    let mut p = Pellets::default();
    p.at(now(date(2026, 10, 4)));
    p
}

/// The auger in the state ON for SECONDS from FROM_MS, asked every second; the ms of the last asking.
fn run(p: &mut Pellets, from_ms: u64, seconds: u64, on: bool, rate: u32) -> u64 {
    let mut at = from_ms;
    for _ in 0..=seconds {
        p.observe(at, on, rate);
        at += 1000;
    }
    at - 1000
}

fn grams(p: &Pellets) -> [u64; 3] {
    p.grams().map(|(_, g)| g)
}

#[test]
fn an_hour_of_the_auger_at_four_grams_a_second_is_fourteen_kilograms_and_four_hundred_grams() {
    let mut p = known();
    run(&mut p, 0, 3600, true, RATE);
    p.at(now(date(2026, 10, 4)));
    assert_eq!(grams(&p), [14_400, 14_400, 14_400]);
    let kg = p.kilograms();
    assert_eq!(kg.map(|(name, _)| name), ["today", "month", "total"]);
    for (_, k) in kg {
        assert!((k - 14.4).abs() < 1e-4, "{k}");
    }
}

#[test]
fn the_first_asking_counts_nothing_even_with_the_auger_on() {
    let mut p = known();
    p.observe(5_000, true, RATE);
    assert_eq!(grams(&p), [0, 0, 0]);
}

#[test]
fn a_standing_auger_counts_nothing() {
    let mut p = known();
    run(&mut p, 0, 600, false, RATE);
    assert_eq!(grams(&p), [0, 0, 0]);
}

#[test]
fn ten_seconds_of_turning_are_forty_grams() {
    let mut p = known();
    run(&mut p, 0, 10, true, RATE);
    p.at(now(date(2026, 10, 4)));
    assert_eq!(grams(&p), [40, 40, 40]);
}

#[test]
fn what_the_auger_did_up_to_the_asking_that_found_it_off_is_counted() {
    let mut p = known();
    let at = run(&mut p, 0, 5, true, RATE);
    p.observe(at + 400, false, RATE);
    p.observe(at + 1400, false, RATE);
    p.at(now(date(2026, 10, 4)));
    // 5 s, and the 0.4 s between the last asking with the auger on and the one that found it off; nothing after
    assert_eq!(p.grams()[2].1, (5_400 * u64::from(RATE)) / 60_000);
}

#[test]
fn a_rate_changed_in_place_counts_from_the_next_asking_on() {
    let mut p = known();
    let at = run(&mut p, 0, 10, true, 240);
    run(&mut p, at + 1000, 10, true, 120);
    p.at(now(date(2026, 10, 4)));
    // ten seconds at 4 g/s; then the asking at the new rate closes the second since the last one, and ten more at 2 g/s
    assert_eq!(p.grams()[2].1, 40 + 22);
}

#[test]
fn a_stalled_asking_counts_at_most_its_longest_step() {
    let mut p = known();
    p.observe(0, true, RATE);
    p.observe(600_000, true, RATE);
    p.at(now(date(2026, 10, 4)));
    assert_eq!(p.grams()[2].1, (MAX_STEP_MS * u64::from(RATE)) / 60_000);
}

#[test]
fn fractions_of_a_step_are_not_thrown_away() {
    let mut p = known();
    // 600 askings 100 ms apart with the auger on: a minute, 240 g, though every step weighs 0.4 g
    for k in 0..=600u64 {
        p.observe(k * 100, true, RATE);
    }
    p.at(now(date(2026, 10, 4)));
    assert_eq!(p.grams()[2].1, 240);
}

#[test]
fn the_day_turns_and_today_starts_from_zero_while_the_month_and_the_total_go_on() {
    let mut p = known();
    let at = run(&mut p, 0, 10, true, RATE);
    p.at(now(date(2026, 10, 5)));
    assert_eq!(grams(&p), [0, 40, 40], "the 40 g of the 4th are the 4th's");
    run(&mut p, at + 1000, 5, true, RATE);
    p.at(now(date(2026, 10, 5)));
    assert_eq!(grams(&p), [20 + 4, 64, 64]);
}

#[test]
fn the_month_turns_and_the_month_starts_from_zero() {
    let mut p = Pellets::default();
    p.at(now(date(2026, 10, 31)));
    run(&mut p, 0, 10, true, RATE);
    p.at(now(date(2026, 10, 31)));
    p.at(now(date(2026, 11, 1)));
    assert_eq!(grams(&p), [0, 0, 40]);
}

#[test]
fn what_is_poured_before_the_board_knows_the_time_waits_and_lands_in_the_day_it_learns() {
    let mut p = Pellets::default();
    run(&mut p, 0, 10, true, RATE);
    assert_eq!(grams(&p), [0, 0, 40], "in all at once, in no day yet");
    p.at(now(date(2026, 10, 4)));
    assert_eq!(grams(&p), [40, 40, 40]);
}

#[test]
fn a_reset_zeroes_the_three_and_what_waits_and_the_day_stays() {
    let mut p = known();
    run(&mut p, 0, 10, true, RATE);
    p.at(now(date(2026, 10, 4)));
    p.reset();
    assert_eq!(grams(&p), [0, 0, 0]);
    assert_eq!(p.tally.day, Some(date(2026, 10, 4)));
    p.at(now(date(2026, 10, 4)));
    assert_eq!(grams(&p), [0, 0, 0]);
}

#[test]
fn a_count_that_changed_says_so_and_one_that_did_not_does_not() {
    let mut p = known();
    let quiet = p.changes;
    run(&mut p, 0, 10, false, RATE);
    p.at(now(date(2026, 10, 4)));
    assert_eq!(p.changes, quiet, "a standing auger and an unchanged day change nothing");
    run(&mut p, 11_000, 3, true, RATE);
    assert_ne!(p.changes, quiet);
    let counted = p.changes;
    p.at(now(date(2026, 10, 5)));
    assert_ne!(p.changes, counted, "a turn of the day changes what today holds");
    let turned = p.changes;
    p.reset();
    assert_ne!(p.changes, turned);
}

/// What the count of 40 g on the 4th keeps: units are milliseconds times grams a minute, so 10 000 * 240 in each.
const KEPT: &str = "pellets 1\nday 2026-10-04\ntoday 2400000\nmonth 2400000\ntotal 2400000\npending 0\n";

#[test]
fn the_count_is_kept_as_text_and_comes_back() {
    let mut p = known();
    run(&mut p, 0, 10, true, RATE);
    p.at(now(date(2026, 10, 4)));
    assert_eq!(p.snapshot(), KEPT);
    let back = Pellets::restore(KEPT).unwrap();
    assert_eq!(grams(&back), [40, 40, 40]);
    assert_eq!(back.tally.day, Some(date(2026, 10, 4)));
}

#[test]
fn after_a_restart_the_first_asking_counts_nothing_and_the_gap_is_not_counted() {
    let mut p = Pellets::restore(KEPT).unwrap();
    p.observe(1_000_000, true, RATE);
    p.at(now(date(2026, 10, 4)));
    assert_eq!(grams(&p), [40, 40, 40]);
}

#[test]
fn a_count_that_never_knew_the_time_is_kept_and_comes_back() {
    let mut p = Pellets::default();
    run(&mut p, 0, 10, true, RATE);
    let back = Pellets::restore(&p.snapshot()).unwrap();
    assert_eq!(back.tally.day, None);
    assert_eq!(back.tally.pending, p.tally.pending);
    assert_eq!(grams(&back), [0, 0, 40]);
}

#[test]
fn text_that_is_not_a_snapshot_of_pellets_is_refused() {
    let bad = [
        "".to_string(),
        "energy 1\nday -\ntoday 0\nmonth 0\ntotal 0\npending 0\n".to_string(),
        KEPT.replacen("pellets 1", "pellets 2", 1),
        KEPT.replacen("today 2400000", "today lots", 1),
        KEPT.replacen("today 2400000", "today 2400000 1", 1),
        KEPT.replacen("day 2026-10-04", "day 2026-02-30", 1),
        KEPT.replacen("month", "mounth", 1),
        format!("{KEPT}total 5\n"),
        KEPT.lines().take(5).collect::<Vec<_>>().join("\n") + "\n",
    ];
    for text in bad {
        assert!(Pellets::restore(&text).is_err(), "{text:?}");
    }
}

/// What the boiler kept on 09.10, as the file `/data/pellets/state` held it: the day of a jump of the clock (2028-02-27),
/// today equal to the month, and the total of all the burning.
const BOILER: &str = "pellets 1\nday 2028-02-27\ntoday 192555360\nmonth 192555360\ntotal 301081440\npending 0\n";

#[test]
fn the_day_a_jump_of_the_clock_left_in_the_kept_text_is_put_right_and_the_total_is_not_touched() {
    let mut p = Pellets::restore(BOILER).unwrap();
    assert_eq!(grams(&p), [3209, 3209, 5018], "what the page said: 3.2 kg today on a day without burning");
    let changes = p.changes;
    p.at(now(date(2026, 10, 10)));
    assert_eq!(p.tally.day, Some(date(2026, 10, 10)));
    assert_eq!(grams(&p), [0, 3209, 5018], "today is nought, the month is what it was, the total is not touched");
    assert_ne!(p.changes, changes, "the board writes the count to the flash again");
}

#[test]
fn a_kept_day_that_is_an_ordinary_one_is_left_alone() {
    let mut p = Pellets::restore(KEPT).unwrap();
    p.at(now(date(2026, 10, 4)));
    assert_eq!(grams(&p), [40, 40, 40]);
    assert_eq!(p.snapshot(), KEPT);
}

#[test]
fn a_kept_count_is_not_sure_of_its_day_until_a_time_is_handed_in_and_is_not_again_when_the_time_is_lost() {
    let mut p = Pellets::restore(BOILER).unwrap();
    assert!(!p.day_sure(), "the numbers are what the flash kept, of a day the board has not seen");
    p.at(None);
    assert!(!p.day_sure());
    p.at(now(date(2026, 10, 10)));
    assert!(p.day_sure());
    p.at(None);
    assert!(!p.day_sure(), "the time is lost: not sure again");
}

#[test]
fn a_count_that_never_knew_the_time_is_not_sure_of_a_day_it_does_not_have() {
    let mut p = Pellets::default();
    run(&mut p, 0, 10, true, RATE);
    assert!(!p.day_sure());
    assert_eq!(grams(&p)[2], 40, "the total is the number it is");
}
