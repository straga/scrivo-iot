//! The count of electricity on the computer: known power over known time, the turn of a day, a month and a year, a
//! power cut in between, and what must not move - a count without the time, a clock that jumps back.

use energy::{file_of, reset_row, Date, Energy, Keeper, Now, Row, Slot, CSV_HEADER, MAX_STEP_MS};
use tally::Tally;

const WH: u64 = 3_600_000; // millijoules in a watt-hour

fn date(year: u16, month: u8, day: u8) -> Date {
    Date { year, month, day }
}

fn now(d: Date, hh: u16, mm: u16) -> Option<Now> {
    Some(Now { date: d, minute: hh * 60 + mm, offset_min: 180 })
}

/// Asks the meter every second from FROM_MS for SECONDS seconds, all answered with WATTS.
fn run(e: &mut Energy, from_ms: u64, seconds: u64, watts: [f32; 2]) -> u64 {
    for s in 0..=seconds {
        e.poll(from_ms + s * 1000, Some(watts));
    }
    from_ms + seconds * 1000
}

/// A count that knows the time: 17:05 of 19.09.2026, nothing counted yet.
fn known() -> Energy {
    let mut e = Energy::default();
    e.at(now(date(2026, 9, 19), 17, 5), 60);
    e
}

#[test]
fn constant_power_for_an_hour_gives_its_watt_hours() {
    let mut e = known();
    run(&mut e, 0, 3600, [100.0, 20.0]);
    e.at(now(date(2026, 9, 19), 18, 4), 60);
    assert_eq!(e.tally.sums.total, [100 * WH, 20 * WH]);
    assert_eq!(e.tally.sums.today, [100 * WH, 20 * WH]);
    assert_eq!(e.tally.sums.month, [100 * WH, 20 * WH]);
}

#[test]
fn the_first_asking_counts_nothing() {
    let mut e = known();
    e.poll(5000, Some([100.0, 100.0]));
    assert_eq!(e.tally.sums.total, [0, 0]);
}

#[test]
fn a_missed_asking_adds_nothing() {
    let mut e = known();
    e.poll(0, Some([100.0, 0.0]));
    e.poll(1000, None);
    e.poll(2000, Some([100.0, 0.0]));
    // one second of 100 W, not two: the missed second is not the next packet's power
    assert_eq!(e.tally.sums.total, [100_000, 0]);
}

#[test]
fn a_stalled_asking_counts_at_most_its_longest_step() {
    let mut e = known();
    e.poll(0, Some([100.0, 0.0]));
    e.poll(60_000, Some([100.0, 0.0]));
    assert_eq!(e.tally.sums.total, [100 * MAX_STEP_MS, 0]);
}

#[test]
fn fractions_of_a_step_are_kept() {
    let mut e = known();
    // 0.3 W for 1 s is 300 mJ each step: nothing is rounded away between steps
    run(&mut e, 0, 10, [0.3, 0.0]);
    assert_eq!(e.tally.sums.total, [3000, 0]);
}

#[test]
fn negative_power_counts_nothing_and_is_timed() {
    let mut e = known();
    run(&mut e, 0, 10, [-5.0, 10.0]);
    assert_eq!(e.tally.sums.total, [0, 100_000]);
    let row = e.at(now(date(2026, 9, 19), 18, 0), 60).expect("the hour closed");
    assert_eq!(row.negative_ms, [10_000, 0]);
    assert_eq!(row.mj, [0, 100_000]);
}

#[test]
fn without_the_time_nothing_is_placed_in_a_day() {
    let mut e = Energy::default();
    run(&mut e, 0, 10, [100.0, 0.0]);
    assert_eq!(e.at(None, 60), None);
    assert_eq!(e.tally.sums.total, [1_000_000, 0]);
    assert_eq!(e.tally.sums.today, [0, 0]);
    assert_eq!(e.tally.pending, [1_000_000, 0]);
    assert_eq!(e.tally.day, None);
}

#[test]
fn when_the_time_comes_what_was_counted_goes_in_today() {
    let mut e = Energy::default();
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(None, 60);
    e.at(now(date(2026, 9, 19), 17, 5), 60);
    assert_eq!(e.tally.sums.today, [1_000_000, 0]);
    assert_eq!(e.tally.sums.month, [1_000_000, 0]);
    assert_eq!(e.tally.pending, [0, 0]);
    assert_eq!(e.tally.day, Some(date(2026, 9, 19)));
}

#[test]
fn the_day_turns_and_today_starts_from_zero() {
    let mut e = known();
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(now(date(2026, 9, 19), 23, 59), 60);
    e.at(now(date(2026, 9, 20), 0, 0), 60);
    assert_eq!(e.tally.sums.today, [0, 0], "the day before is closed");
    run(&mut e, 11_000, 9, [100.0, 0.0]);
    e.at(now(date(2026, 9, 20), 0, 1), 60);
    assert_eq!(e.tally.sums.today, [1_000_000, 0], "only what came after midnight");
    assert_eq!(e.tally.sums.month, [2_000_000, 0]);
    assert_eq!(e.tally.sums.total, [2_000_000, 0]);
    assert_eq!(e.tally.day, Some(date(2026, 9, 20)));
}

#[test]
fn the_month_turns_and_the_month_starts_from_zero() {
    let mut e = Energy::default();
    e.at(now(date(2026, 9, 30), 23, 0), 60);
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(now(date(2026, 9, 30), 23, 59), 60);
    e.at(now(date(2026, 10, 1), 0, 1), 60);
    assert_eq!((e.tally.sums.today, e.tally.sums.month, e.tally.sums.total), ([0, 0], [0, 0], [1_000_000, 0]));
}

#[test]
fn the_year_turns_and_the_month_starts_from_zero() {
    let mut e = Energy::default();
    e.at(now(date(2026, 12, 31), 23, 0), 60);
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(now(date(2026, 12, 31), 23, 30), 60);
    e.at(now(date(2027, 1, 1), 0, 1), 60);
    assert_eq!((e.tally.sums.month, e.tally.sums.total), ([0, 0], [1_000_000, 0]));
}

#[test]
fn a_clock_that_jumps_back_closes_no_day_and_takes_nothing_away() {
    let mut e = Energy::default();
    e.at(now(date(2026, 9, 20), 0, 30), 60);
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(now(date(2026, 9, 20), 0, 40), 60);
    assert_eq!(e.at(now(date(2026, 9, 19), 23, 50), 60), None, "no stretch closes backwards");
    run(&mut e, 11_000, 9, [100.0, 0.0]);
    e.at(now(date(2026, 9, 19), 23, 55), 60);
    assert_eq!(e.tally.day, Some(date(2026, 9, 20)));
    assert_eq!(e.tally.sums.today, [2_000_000, 0]);
    assert_eq!(e.tally.sums.total, [2_000_000, 0]);
}

#[test]
fn an_hour_closes_into_a_row_of_what_it_held() {
    let mut e = known();
    run(&mut e, 0, 3600, [100.0, 20.0]);
    assert_eq!(e.at(now(date(2026, 9, 19), 17, 59), 60), None);
    let row = e.at(now(date(2026, 9, 19), 18, 0), 60).expect("17:00 closed");
    assert_eq!(row.slot, Slot { date: date(2026, 9, 19), minute: 17 * 60, offset_min: 180 });
    assert_eq!(row.mj, [100 * WH, 20 * WH]);
    assert_eq!(e.in_slot, [0, 0]);
    assert_eq!(e.slot.map(|s| s.minute), Some(18 * 60));
}

#[test]
fn a_shorter_stretch_closes_on_its_own_minutes() {
    let mut e = Energy::default();
    e.at(now(date(2026, 9, 19), 17, 14), 15);
    assert_eq!(e.slot.map(|s| s.minute), Some(17 * 60));
    let row = e.at(now(date(2026, 9, 19), 17, 15), 15).expect("17:00-17:15 closed");
    assert_eq!(row.slot.minute, 17 * 60);
    assert_eq!(e.slot.map(|s| s.minute), Some(17 * 60 + 15));
}

#[test]
fn an_hour_with_nothing_used_still_gives_its_row() {
    let mut e = known();
    let row = e.at(now(date(2026, 9, 19), 18, 0), 60).expect("an empty hour is a row of zeros, not a gap");
    assert_eq!(row.mj, [0, 0]);
}

#[test]
fn the_same_day_after_a_power_cut_goes_on() {
    let mut e = known();
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(now(date(2026, 9, 19), 17, 10), 60);
    let mut back = Energy::restore(&e.snapshot()).expect("its own snapshot");
    run(&mut back, 0, 10, [100.0, 0.0]);
    back.at(None, 60);
    back.at(now(date(2026, 9, 19), 17, 20), 60);
    assert_eq!(back.tally.sums.today, [2_000_000, 0]);
    assert_eq!(back.tally.sums.total, [2_000_000, 0]);
}

#[test]
fn another_day_after_a_power_cut_closes_the_one_that_was() {
    let mut e = known();
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(now(date(2026, 9, 19), 17, 10), 60);
    let mut back = Energy::restore(&e.snapshot()).expect("its own snapshot");
    run(&mut back, 0, 10, [100.0, 0.0]);
    back.at(None, 60);
    let row = back.at(now(date(2026, 9, 20), 8, 3), 60).expect("the hour cut short closes");
    assert_eq!(row.slot.minute, 17 * 60);
    assert_eq!(row.mj, [1_000_000, 0], "the stored hour, not what came before the time did");
    assert_eq!(back.tally.sums.today, [1_000_000, 0], "what came since the restart, in the new day");
    assert_eq!(back.tally.sums.month, [2_000_000, 0]);
    assert_eq!(back.tally.sums.total, [2_000_000, 0]);
}

#[test]
fn a_snapshot_gives_back_the_count() {
    let mut e = known();
    run(&mut e, 0, 10, [100.0, -3.0]);
    e.at(now(date(2026, 9, 19), 17, 10), 60);
    run(&mut e, 20_000, 5, [7.5, 0.0]);
    let back = Energy::restore(&e.snapshot()).expect("its own snapshot");
    assert_eq!(back, Energy { last_asked_ms: None, tally: Tally { timed: false, ..e.tally }, ..e });
}

#[test]
fn a_snapshot_of_a_count_that_never_knew_the_time_gives_it_back() {
    let mut e = Energy::default();
    run(&mut e, 0, 10, [100.0, 0.0]);
    let back = Energy::restore(&e.snapshot()).expect("its own snapshot");
    assert_eq!(back, Energy { last_asked_ms: None, tally: Tally { timed: false, ..e.tally }, ..e });
}

#[test]
fn text_that_is_not_a_snapshot_is_refused() {
    let good = {
        let mut e = known();
        run(&mut e, 0, 10, [100.0, 0.0]);
        e.snapshot()
    };
    assert!(Energy::restore(&good).is_ok());
    let lines: Vec<&str> = good.lines().collect();
    let cases = [
        String::new(),
        "garbage".to_string(),
        good.replacen("energy 1", "energy 2", 1),
        lines[..lines.len() - 1].join("\n"),
        good.replacen("today ", "today x", 1),
        good.replacen("day 2026-09-19", "day 2026-13-19", 1),
        good.replace("total 1000000", "total -1000000"),
        format!("{good}extra 1 2\n"),
    ];
    for (n, text) in cases.iter().enumerate() {
        assert!(Energy::restore(text).is_err(), "case {n} taken: {text:?}");
    }
}

#[test]
fn a_reset_zeroes_the_counts_and_the_open_hour() {
    let mut e = known();
    run(&mut e, 0, 10, [100.0, 50.0]);
    e.at(now(date(2026, 9, 19), 17, 10), 60);
    e.reset();
    assert_eq!(e.tally.sums, Default::default());
    assert_eq!(e.in_slot, [0, 0]);
    assert_eq!(e.tally.pending, [0, 0]);
    assert_eq!(e.tally.day, Some(date(2026, 9, 19)), "the day stays: the reset is not a new day");
}

#[test]
fn a_row_reads_as_the_csv_has_it() {
    let row = Row {
        slot: Slot { date: date(2026, 9, 19), minute: 17 * 60, offset_min: 180 },
        mj: [19_412 * 3600, 110_700 * 3600 + 3599],
        negative_ms: [0, 0],
    };
    assert_eq!(row.csv(), "2026-09-19 17:00+03:00,19.412,110.700,130.112");
    let west = Row { slot: Slot { date: date(2026, 1, 2), minute: 5, offset_min: -330 }, mj: [0, 0], negative_ms: [0, 0] };
    assert_eq!(west.csv(), "2026-01-02 00:05-05:30,0.000,0.000,0.000");
}

#[test]
fn a_month_has_its_file_and_every_file_its_header() {
    assert_eq!(file_of(date(2026, 9, 19)), "2026-09.csv");
    assert_eq!(file_of(date(2027, 1, 1)), "2027-01.csv");
    assert_eq!(CSV_HEADER.split(',').count(), 4);
}

#[test]
fn a_reset_leaves_its_mark_in_the_csv() {
    let at = Now { date: date(2026, 9, 19), minute: 17 * 60 + 23, offset_min: 180 };
    assert_eq!(reset_row(at), "2026-09-19 17:23+03:00,reset,,");
}

#[test]
fn a_person_reads_kilowatt_hours_per_output_and_their_sum() {
    let mut e = known();
    e.tally.sums.today = [12 * WH, 110 * WH];
    e.tally.sums.month = [1500 * WH, 0];
    e.tally.sums.total = [0, 0];
    let words = e.words();
    assert_eq!(words[0], ("today", "L1 0.012 · L2 0.110 · sum 0.122 kWh".to_string()));
    assert_eq!(words[1], ("month", "L1 1.500 · L2 0.000 · sum 1.500 kWh".to_string()));
    assert_eq!(words[2], ("total", "L1 0.000 · L2 0.000 · sum 0.000 kWh".to_string()));
}

#[test]
fn each_row_carries_its_sum_as_a_number_beside_its_words() {
    let mut e = known();
    e.tally.sums.today = [12 * WH, 110 * WH];
    e.tally.sums.month = [1500 * WH, 0];
    e.tally.sums.total = [0, 0];
    let sums = e.sums_kwh();
    assert_eq!(sums[0].0, "today");
    assert!((sums[0].1 - 0.122).abs() < 1e-6, "{sums:?}");
    assert!((sums[1].1 - 1.5).abs() < 1e-6, "{sums:?}");
    assert_eq!(sums[2], ("total", 0.0), "the pair: nothing counted is nought, and a number");
}

/// What the meter's thread does with the count: a snapshot taken under its lock, written after it is let go.
#[test]
fn a_snapshot_taken_before_a_reset_never_lands_after_it() {
    use std::sync::{Arc, Barrier, Mutex};
    let keeper = Arc::new(Keeper::default());
    let file = Arc::new(Mutex::new(String::new()));
    let count = Arc::new(Mutex::new(known()));
    {
        let mut e = count.lock().unwrap();
        run(&mut e, 0, 10, [100.0, 0.0]);
        e.at(now(date(2026, 9, 19), 17, 10), 60);
    }
    let taken = Arc::new(Barrier::new(2));
    let reset_written = Arc::new(Barrier::new(2));
    let meter = {
        let (keeper, file, count, taken, reset_written) =
            (keeper.clone(), file.clone(), count.clone(), taken.clone(), reset_written.clone());
        std::thread::spawn(move || {
            let stale = keeper.take(count.lock().unwrap().snapshot());
            taken.wait();
            reset_written.wait();
            keeper.write(stale, |t| -> Result<(), ()> {
                *file.lock().unwrap() = t.to_string();
                Ok(())
            })
        })
    };
    taken.wait();
    let zero = {
        let mut e = count.lock().unwrap();
        e.reset();
        keeper.take(e.snapshot())
    };
    assert_eq!(keeper.write(zero, |t| -> Result<(), ()> { *file.lock().unwrap() = t.to_string(); Ok(()) }), Ok(true));
    reset_written.wait();
    assert_eq!(meter.join().unwrap(), Ok(false), "the snapshot from before the reset is not written");
    let back = Energy::restore(&file.lock().unwrap()).expect("a snapshot");
    assert_eq!(back.tally.sums.total, [0, 0], "after the restart the count is the one the reset left");
}

#[test]
fn snapshots_written_in_the_order_they_were_taken_all_land() {
    let keeper = Keeper::default();
    let mut file = String::new();
    let mut e = known();
    run(&mut e, 0, 10, [100.0, 0.0]);
    let first = keeper.take(e.snapshot());
    run(&mut e, 11_000, 9, [100.0, 0.0]);
    let second = keeper.take(e.snapshot());
    assert_eq!(keeper.write(first, |t| -> Result<(), ()> { file = t.to_string(); Ok(()) }), Ok(true));
    assert_eq!(keeper.write(second, |t| -> Result<(), ()> { file = t.to_string(); Ok(()) }), Ok(true));
    assert_eq!(Energy::restore(&file).unwrap().tally.sums.total, [2_000_000, 0]);
}

#[test]
fn a_snapshot_whose_write_failed_does_not_hold_back_the_next() {
    let keeper = Keeper::default();
    let e = known();
    let first = keeper.take(e.snapshot());
    let second = keeper.take(e.snapshot());
    assert_eq!(keeper.write(second, |_| Err("vfs not mounted")), Err("vfs not mounted"));
    assert_eq!(keeper.write(first, |_| Ok::<(), &str>(())), Ok(true), "nothing newer landed, so the older one may");
}

#[test]
fn a_year_on_in_the_same_month_starts_the_month_from_zero() {
    let mut e = Energy::default();
    e.at(now(date(2026, 9, 30), 23, 0), 60);
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(now(date(2026, 9, 30), 23, 30), 60);
    e.at(now(date(2027, 9, 1), 0, 1), 60);
    assert_eq!((e.tally.sums.today, e.tally.sums.month, e.tally.sums.total), ([0, 0], [0, 0], [1_000_000, 0]));
}

/// A good snapshot with its day and its stretch set to DAY and SLOT.
fn dated(day: &str, slot: &str) -> String {
    let mut e = known();
    run(&mut e, 0, 10, [100.0, 0.0]);
    e.at(now(date(2026, 9, 19), 17, 10), 60);
    e.snapshot().replace("day 2026-09-19", &format!("day {day}")).replace("slot 2026-09-19", &format!("slot {slot}"))
}

#[test]
fn a_snapshot_with_a_day_no_calendar_has_is_refused() {
    for (day, slot) in [
        ("2026-02-31", "2026-09-19"),
        ("2026-04-31", "2026-09-19"),
        ("2026-02-29", "2026-09-19"),
        ("2026-09-19", "2026-02-30"),
        ("2026-09-19", "2026-06-31"),
        ("2026-09-19", "2026-02-29"),
        ("2100-02-29", "2026-09-19"),
        ("2026-09-00", "2026-09-19"),
    ] {
        assert!(Energy::restore(&dated(day, slot)).is_err(), "day {day}, slot {slot} taken");
    }
}

#[test]
fn a_snapshot_with_days_the_calendar_has_is_taken() {
    for (day, slot) in [
        ("2028-02-29", "2028-02-29"),
        ("2000-02-29", "2026-09-19"),
        ("2026-02-28", "2026-02-28"),
        ("2026-04-30", "2026-12-31"),
        ("2026-01-31", "2026-03-31"),
    ] {
        assert!(Energy::restore(&dated(day, slot)).is_ok(), "day {day}, slot {slot} refused");
    }
}

#[test]
fn a_snapshot_with_an_offset_no_zone_has_is_refused() {
    let good = dated("2026-09-19", "2026-09-19");
    for offset in ["841", "-841", "9999"] {
        let text = good.replace(" 1020 180\n", &format!(" 1020 {offset}\n"));
        assert!(Energy::restore(&text).is_err(), "offset {offset} taken");
    }
    for offset in ["840", "-720", "0", "330"] {
        let text = good.replace(" 1020 180\n", &format!(" 1020 {offset}\n"));
        assert!(Energy::restore(&text).is_ok(), "offset {offset} refused");
    }
}

/// What a board running before the count was shared has in /data/energy/state: the file of a restart must still be taken, and the text written now must read the same, line for line.
const KEPT_BEFORE: &str = "energy 1\nday 2026-09-19\nslot 2026-09-19 1020 180\ntoday 100 20\nmonth 300 40\ntotal 900 80\nin_slot 7 8\nnegative 1 2\npending 3 4\n";

#[test]
fn a_snapshot_written_before_the_count_was_shared_is_taken_and_written_the_same() {
    let back = Energy::restore(KEPT_BEFORE).unwrap();
    assert_eq!(back.tally.sums.today, [100, 20]);
    assert_eq!(back.tally.sums.month, [300, 40]);
    assert_eq!(back.tally.sums.total, [900, 80]);
    assert_eq!(back.tally.pending, [3, 4]);
    assert_eq!(back.in_slot, [7, 8]);
    assert_eq!(back.negative_ms, [1, 2]);
    assert_eq!(back.tally.day, Some(date(2026, 9, 19)));
    assert_eq!(back.snapshot(), KEPT_BEFORE);
}

/// What a page draws by when it says each output apart: the same three rows as `sums_kwh`, each output's own kilowatt-hours.
#[test]
fn the_sums_are_told_for_each_output_apart_in_kilowatt_hours() {
    let mut e = Energy::default();
    e.tally.sums.today = [12 * WH, 110 * WH];
    e.tally.sums.month = [1500 * WH, 0];
    e.tally.sums.total = [0, 0];
    let by_output = e.sums_by_output_kwh();
    assert_eq!(by_output.map(|(name, _)| name), ["today", "month", "total"]);
    assert!((by_output[0].1[0] - 0.012).abs() < 1e-6 && (by_output[0].1[1] - 0.110).abs() < 1e-6, "{by_output:?}");
    assert!((by_output[1].1[0] - 1.5).abs() < 1e-6 && by_output[1].1[1] == 0.0, "{by_output:?}");
    assert_eq!(by_output[2].1, [0.0, 0.0], "nothing counted is nought, and a number");
}

#[test]
fn each_output_s_kilowatt_hours_add_up_to_the_sum_the_page_has_always_drawn() {
    let mut e = Energy::default();
    e.tally.sums.today = [7 * WH, 31 * WH];
    e.tally.sums.month = [1234 * WH, 5678 * WH];
    e.tally.sums.total = [9_000_000 * WH, 1 * WH];
    for ((name, apart), (same, whole)) in e.sums_by_output_kwh().iter().zip(e.sums_kwh()) {
        assert_eq!(*name, same);
        assert!((apart[0] + apart[1] - whole).abs() <= whole * 1e-6 + 1e-6, "{name}: {apart:?} against {whole}");
    }
}

#[test]
fn a_day_and_a_stretch_recorded_from_a_false_time_are_put_right_and_close_no_row() {
    // the kept text of a count that stood in a day that never came: its day and its open stretch are of 2028
    let mut e = Energy::restore(&dated("2028-02-27", "2028-02-27")).unwrap();
    let (month, total) = (e.tally.sums.month, e.tally.sums.total);
    assert_eq!(e.at(now(date(2026, 9, 19), 17, 20), 60), None, "a stretch of a day that never came is not written");
    assert_eq!(e.tally.day, Some(date(2026, 9, 19)));
    assert_eq!(e.tally.sums.today, [0, 0], "today starts from nought");
    assert_eq!((e.tally.sums.month, e.tally.sums.total), (month, total), "the month and the total stay");
    assert_eq!(e.slot, Some(Slot { date: date(2026, 9, 19), minute: 17 * 60, offset_min: 180 }), "the open stretch is the real one");
    // and the stretches close as they always did
    run(&mut e, 0, 10, [100.0, 0.0]);
    let closed = e.at(now(date(2026, 9, 19), 18, 5), 60).expect("the hour closed");
    assert_eq!((closed.slot.date, closed.slot.minute, closed.mj), (date(2026, 9, 19), 17 * 60, [1_000_000, 0]));
}

#[test]
fn a_stretch_a_day_ahead_is_left_as_it_was_a_zone_or_summer_time_goes_back_by_a_day_at_most() {
    let mut e = Energy::restore(&dated("2026-09-20", "2026-09-20")).unwrap();
    assert_eq!(e.at(now(date(2026, 9, 19), 17, 20), 60), None);
    assert_eq!(e.tally.day, Some(date(2026, 9, 20)));
    assert_eq!(e.slot.map(|s| s.date), Some(date(2026, 9, 20)), "a day ahead closes nothing and puts nothing right");
}

#[test]
fn a_kept_energy_count_is_not_sure_of_its_day_until_a_time_is_handed_in() {
    let mut e = Energy::restore(&dated("2026-09-19", "2026-09-19")).unwrap();
    assert!(!e.day_sure(), "what the flash kept is of a day the board has not seen");
    e.at(None, 60);
    assert!(!e.day_sure());
    e.at(now(date(2026, 9, 19), 17, 20), 60);
    assert!(e.day_sure());
    e.at(None, 60);
    assert!(!e.day_sure(), "the time is lost: not sure again");
}

#[test]
fn the_words_a_person_reads_say_a_dash_for_today_and_the_month_while_the_count_is_not_sure_of_its_day_and_the_total_as_it_is() {
    // what the flash kept: today and the month are of a day the board has not seen
    let mut e = Energy::restore(&dated("2026-09-19", "2026-09-19")).unwrap();
    assert!(!e.day_sure());
    let held = e.words();
    let shown = e.words_shown();
    assert_eq!(shown[0], ("today", "-".to_string()));
    assert_eq!(shown[1], ("month", "-".to_string()));
    assert_eq!(shown[2], held[2], "the total is the number it is, said as it always was");
    // the time is handed in: the count is sure and says what it counted
    e.at(now(date(2026, 9, 19), 17, 20), 60);
    assert!(e.day_sure());
    assert_eq!(e.words_shown(), e.words(), "sure of its day: the words are the numbers");
    // and the time is lost: not sure again
    e.at(None, 60);
    assert_eq!(e.words_shown()[0], ("today", "-".to_string()));
}

#[test]
fn the_words_the_start_logs_are_the_numbers_the_flash_kept_whether_or_not_the_day_is_sure() {
    let e = Energy::restore(&dated("2026-09-19", "2026-09-19")).unwrap();
    assert!(!e.day_sure());
    assert!(e.words()[0].1.starts_with("L1 "), "{:?}", e.words()[0]);
}
