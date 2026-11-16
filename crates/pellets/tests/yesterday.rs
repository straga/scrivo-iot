//! What the count of pellets says of yesterday, and what it keeps of it. The day before today is what a person reads on the card; it is kept in a file
//! of its own so that the file of the count stays what an older image reads (06.10: an image that finds a line it does not know in the count counts from nought).
//! Expected from what a person would want after a restart in the night and from what an older image reads - not from the code that keeps it.

use pellets::{Date, Now, Pellets, Saving, RETRY_MS};

const RATE: u32 = 240;

fn date(year: u16, month: u8, day: u8) -> Date {
    Date { year, month, day }
}

fn now(d: Date) -> Option<Now> {
    Some(Now { date: d, minute: 12 * 60, offset_min: 180 })
}

/// The auger on for SECONDS from FROM_MS, asked every second; the ms of the last asking.
fn run(p: &mut Pellets, from_ms: u64, seconds: u64) -> u64 {
    let mut at = from_ms;
    for _ in 0..=seconds {
        p.observe(at, true, RATE);
        at += 1000;
    }
    at - 1000
}

/// 22.10 with 40 grams burned, then the clock reaches 23.10.
fn after_midnight() -> Pellets {
    let mut p = Pellets::default();
    p.at(now(date(2026, 10, 22)));
    run(&mut p, 0, 10);
    p.at(now(date(2026, 10, 22)));
    p.at(now(date(2026, 10, 23)));
    p
}

/// What the boiler's file of the count held before this work, as the burner keeps it now: six lines.
const OLD_COUNT: &str = "pellets 1\nday 2026-10-22\ntoday 2400000\nmonth 9000000\ntotal 30000000\npending 0\n";

#[test]
fn the_card_says_what_the_day_before_came_to_in_kilograms() {
    let p = after_midnight();
    assert_eq!(p.yesterday_kg(), Some(0.04));
    assert_eq!(p.kilograms().map(|(_, kg)| kg), [0.0, 0.04, 0.04], "today starts from nought, the month and the total go on");
}

#[test]
fn before_a_day_has_turned_there_is_no_yesterday_and_it_is_not_nought() {
    let mut p = Pellets::default();
    p.at(now(date(2026, 10, 22)));
    assert_eq!(p.yesterday_kg(), None);
    assert_eq!(p.yesterday_text(), None);
}

#[test]
fn the_file_of_the_count_is_the_six_lines_an_older_image_reads_with_or_without_yesterday() {
    let mut p = Pellets::restore(OLD_COUNT).unwrap();
    assert_eq!(p.snapshot(), OLD_COUNT, "the old file comes back byte for byte");
    p.at(now(date(2026, 10, 22)));
    p.at(now(date(2026, 10, 23)));
    assert!(p.yesterday_kg().is_some());
    let text = p.snapshot();
    assert_eq!(text.lines().count(), 6, "{text}");
    assert!(text.starts_with("pellets 1\n"), "{text}");
    assert!(Pellets::restore(&text).is_ok());
}

#[test]
fn a_count_in_the_old_file_comes_back_whole_and_says_no_yesterday() {
    let mut p = Pellets::restore(OLD_COUNT).unwrap();
    p.at(now(date(2026, 10, 22)));
    assert_eq!(p.grams().map(|(_, g)| g), [40, 150, 500]);
    assert_eq!(p.yesterday_kg(), None, "the old file has no yesterday: empty until the first midnight, not nought");
}

#[test]
fn yesterday_is_kept_as_text_and_comes_back_after_a_restart() {
    let p = after_midnight();
    let text = p.yesterday_text().expect("a day has turned");
    assert_eq!(text, "pellets-yesterday 1\nday 2026-10-22\nsums 2400000\n");
    let mut back = Pellets::restore(&p.snapshot()).unwrap();
    assert_eq!(back.yesterday_kg(), None, "not until the file of yesterday is read and the time is believed");
    back.restore_yesterday(&text).unwrap();
    back.at(now(date(2026, 10, 23)));
    assert_eq!(back.yesterday_kg(), Some(0.04));
}

#[test]
fn a_restart_across_midnight_says_the_day_that_ended_not_the_one_before() {
    // the count was kept on 22.10 (40 g that day), the board comes back on 23.10 with the file of yesterday of 21.10
    let mut p = Pellets::restore(OLD_COUNT).unwrap();
    p.restore_yesterday("pellets-yesterday 1\nday 2026-10-21\nsums 600000\n").unwrap();
    assert_eq!(p.yesterday_kg(), None);
    p.at(now(date(2026, 10, 23)));
    assert_eq!(p.yesterday_kg(), Some(0.04), "22.10 is yesterday now, and it is what the count of 22.10 held");
}

#[test]
fn a_file_of_yesterday_that_is_not_one_is_refused_and_the_count_is_not_touched() {
    let mut p = Pellets::restore(OLD_COUNT).unwrap();
    let before = p.snapshot();
    for bad in ["", "pellets 1\nday 2026-10-21\nsums 6\n", "pellets-yesterday 1\nday -\nsums 6\n", "pellets-yesterday 1\nday 2026-10-21\nsums x\n"] {
        assert!(p.restore_yesterday(bad).is_err(), "{bad:?}");
    }
    assert_eq!(p.snapshot(), before);
    assert_eq!(p.yesterday_kg(), None);
}

/// The round of modules/pellets.rs for yesterday: the text if one is due, written, the answer told.
fn round(p: &Pellets, saving: &mut Saving, held: &mut Option<String>, refuse: &mut u32, tries: &mut u32, at_ms: u64) {
    if let Some(pending) = saving.yesterday_due(p, at_ms) {
        *tries += 1;
        let written = if *refuse > 0 {
            *refuse -= 1;
            false
        } else {
            // a text is the file written whole; no text is the file taken off
            *held = pending.text.clone();
            true
        };
        saving.yesterday_finish(&pending, written);
    }
}

#[test]
fn yesterday_is_written_once_when_the_day_turns_and_not_again() {
    let (mut held, mut refuse, mut tries) = (None, 0, 0);
    let mut p = Pellets::default();
    p.at(now(date(2026, 10, 22)));
    let mut saving = Saving::new(&p, 0);
    run(&mut p, 0, 10);
    p.at(now(date(2026, 10, 22)));
    for s in 0..100u64 {
        round(&p, &mut saving, &mut held, &mut refuse, &mut tries, s * 1000);
    }
    assert_eq!(tries, 0, "nothing to write while no day has turned, whatever is counted");
    p.at(now(date(2026, 10, 23)));
    assert!(saving.yesterday_owed(&p), "a restart now would lose yesterday");
    for s in 100..1000u64 {
        round(&p, &mut saving, &mut held, &mut refuse, &mut tries, s * 1000);
    }
    assert_eq!(tries, 1, "once for the day that turned");
    assert!(!saving.yesterday_owed(&p));
    assert_eq!(held.as_deref(), p.yesterday_text().as_deref());
    run(&mut p, 2_000_000, 600);
    p.at(now(date(2026, 10, 23)));
    for s in 1000..2000u64 {
        round(&p, &mut saving, &mut held, &mut refuse, &mut tries, s * 1000);
    }
    assert_eq!(tries, 1, "a day of burning later the file is the same: the flash is not worn");
}

#[test]
fn a_refused_write_of_yesterday_is_tried_again_after_the_wait_and_the_count_stays_owed_until_then() {
    let (mut held, mut refuse, mut tries) = (None, 2, 0);
    let p = after_midnight();
    let mut saving = Saving::new(&Pellets::default(), 0);
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 1000);
    assert_eq!((tries, held.is_some()), (1, false));
    assert!(saving.yesterday_owed(&p));
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 1000 + RETRY_MS - 1);
    assert_eq!(tries, 1, "not before the wait");
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 1000 + RETRY_MS);
    assert_eq!(tries, 2);
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 1000 + 2 * RETRY_MS);
    assert_eq!((tries, held.is_some()), (3, true));
    assert!(!saving.yesterday_owed(&p), "the file is the flash's now");
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 1000 + 3 * RETRY_MS);
    assert_eq!(tries, 3);
}

#[test]
fn yesterday_taken_back_from_the_file_is_not_written_again_at_start() {
    let text = after_midnight().yesterday_text().unwrap();
    let mut p = Pellets::restore(OLD_COUNT).unwrap();
    p.restore_yesterday(&text).unwrap();
    let mut saving = Saving::new(&p, 0);
    let (mut held, mut refuse, mut tries) = (None, 0, 0);
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 1_000_000);
    assert_eq!(tries, 0);
}

#[test]
fn a_reset_forgets_yesterday_with_the_count_it_was_a_day_of() {
    let mut p = after_midnight();
    assert_eq!(p.yesterday_kg(), Some(0.04));
    p.reset();
    assert_eq!(p.yesterday_kg(), None);
    assert_eq!(p.yesterday_text(), None);
    assert_eq!(p.grams().map(|(_, g)| g), [0, 0, 0]);
}

#[test]
fn the_file_of_yesterday_is_taken_off_after_a_reset_once_and_a_refusal_is_tried_again() {
    let (mut held, mut refuse, mut tries) = (None, 0, 0);
    let mut p = after_midnight();
    let mut saving = Saving::new(&Pellets::default(), 0);
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 1000);
    assert!(held.is_some(), "the file of yesterday stands on the flash");
    assert_eq!(tries, 1);
    p.reset();
    assert!(saving.yesterday_owed(&p), "the flash still holds a day of the count that was");
    refuse = 1;
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 2000);
    assert_eq!((tries, held.is_some()), (2, true), "the flash refused: the file stands");
    assert!(saving.yesterday_owed(&p));
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 2000 + RETRY_MS - 1);
    assert_eq!(tries, 2, "not before the wait");
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 2000 + RETRY_MS);
    assert_eq!((tries, held.is_some()), (3, false), "taken off");
    assert!(!saving.yesterday_owed(&p));
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 2000 + 2 * RETRY_MS);
    assert_eq!(tries, 3, "nothing to take off twice");
}

#[test]
fn the_day_that_turns_after_a_reset_writes_the_yesterday_of_the_count_since() {
    let (mut held, mut refuse, mut tries) = (None, 0, 0);
    let mut p = after_midnight();
    let mut saving = Saving::new(&Pellets::default(), 0);
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 1000);
    p.observe(11_000, false, RATE); // the auger stops, so the first asking after the reset finds it standing
    p.reset();
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 2000);
    assert_eq!(held, None);
    run(&mut p, 5_000_000, 20);
    p.at(now(date(2026, 10, 23)));
    p.at(now(date(2026, 10, 24)));
    assert_eq!(p.yesterday_kg(), Some(0.08), "20 s of the auger since the reset, not the 10 s before it");
    round(&p, &mut saving, &mut held, &mut refuse, &mut tries, 3000);
    assert_eq!(held.as_deref(), Some("pellets-yesterday 1\nday 2026-10-23\nsums 4800000\n"));
}

fn civil(days: i64) -> Date {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    Date { year: (yoe + era * 400 + i64::from(m <= 2)) as u16, month: m as u8, day: d as u8 }
}

#[test]
fn a_jump_of_the_wall_clock_with_no_answer_leaves_the_kept_yesterday_and_its_file_alone() {
    const DAY_MS: i64 = 86_400_000;
    let day10 = Date { year: 2026, month: 10, day: 10 }.days() * DAY_MS;
    let wall = |steady_ms: i64, shift_ms: i64| {
        let ms = day10 + 12 * 3_600_000 + steady_ms + shift_ms;
        Some(Now { date: civil(ms.div_euclid(DAY_MS)), minute: (ms.rem_euclid(DAY_MS) / 60_000) as u16, offset_min: 0 })
    };
    let mut p = Pellets::default();
    p.restore_yesterday("pellets-yesterday 1\nday 2026-10-09\nsums 420000\n").unwrap();
    let kept = p.yesterday_text().unwrap();
    let mut saving = Saving::new(&p, 0);
    let mut watch = pellets::Watch::new();
    let (mut tries, mut shift) = (0, 0);
    for second in 1..=4200i64 {
        if second == 3601 {
            shift = 3 * DAY_MS; // the time of day leaps three days while the chip's counter stands, and no server answers
        }
        p.at(watch.believe(wall(second * 1000, shift), (second * 1000) as u64, true, 0));
        if saving.yesterday_due(&p, (second * 1000) as u64).is_some() {
            tries += 1;
        }
    }
    assert_eq!(p.tally.day, Some(Date { year: 2026, month: 10, day: 10 }), "the count does not stand in the day of the jump");
    assert_eq!(p.yesterday_text().as_deref(), Some(kept.as_str()), "the kept yesterday is whole");
    assert_eq!(tries, 0, "the file of yesterday is neither rewritten nor taken off");
    assert!(!saving.yesterday_owed(&p));
}
