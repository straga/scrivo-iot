//! When the count goes to flash, and that what the board counts as written is what the flash took. A flash that refuses is played by a store that says no for a number of tries. Expected from what a person would want after a power cut: the count
//! they last saw - or the zero they last asked for - is what comes back, whatever the flash did on the way.

use pellets::{Date, Now, Pellets, Saving, RETRY_MS, SAVE_EVERY_MS};
use tally::Keeper;

#[derive(Default)]
struct Flash {
    refuse: u32,
    tries: u32,
    held: Option<String>,
}

impl Flash {
    fn put(&mut self, text: &str) -> Result<(), String> {
        self.tries += 1;
        if self.refuse > 0 {
            self.refuse -= 1;
            return Err("NOT_ENOUGH_SPACE".into());
        }
        self.held = Some(text.to_string());
        Ok(())
    }
}

fn now() -> Option<Now> {
    Some(Now { date: Date { year: 2026, month: 10, day: 4 }, minute: 720, offset_min: 180 })
}

/// A count of 40 g that knows its day, and a saving that has written it.
fn counted() -> (Pellets, Saving, Keeper, Flash) {
    let mut p = Pellets::default();
    p.at(now());
    let (keeper, mut flash) = (Keeper::new(), Flash::default());
    let mut saving = Saving::new(&p, 0);
    for s in 0..=10u64 {
        p.observe(s * 1000, true, 240);
    }
    p.at(now());
    round(&p, &mut saving, &keeper, &mut flash, SAVE_EVERY_MS);
    assert_eq!(flash.tries, 1);
    (p, saving, keeper, flash)
}

/// The round of modules/pellets.rs: the snapshot if one is due, written, and the answer told.
fn round(p: &Pellets, saving: &mut Saving, keeper: &Keeper, flash: &mut Flash, at_ms: u64) {
    if let Some(pending) = saving.due(keeper, p, at_ms) {
        let written = keeper.write(pending.snapshot.clone(), |text| flash.put(text)).is_ok();
        saving.finish(&pending, written);
    }
}

/// The reset of modules/pellets.rs: zeroed, written at once, the answer told and given back.
fn reset(p: &mut Pellets, saving: &mut Saving, keeper: &Keeper, flash: &mut Flash, at_ms: u64) -> Result<(), String> {
    p.reset();
    let pending = saving.now(keeper, p, at_ms);
    let done = keeper.write(pending.snapshot.clone(), |text| flash.put(text)).map(|_| ());
    saving.finish(&pending, done.is_ok());
    done
}

#[test]
fn a_count_that_moved_is_written_once_the_interval_is_gone_and_not_before() {
    let (mut p, mut saving, keeper, mut flash) = counted();
    p.observe(11_000, true, 240);
    p.observe(12_000, true, 240);
    round(&p, &mut saving, &keeper, &mut flash, SAVE_EVERY_MS + 1000);
    assert_eq!(flash.tries, 1, "a minute after the last write is too soon");
    round(&p, &mut saving, &keeper, &mut flash, 2 * SAVE_EVERY_MS);
    assert_eq!(flash.tries, 2);
    assert_eq!(Pellets::restore(flash.held.as_deref().unwrap()).unwrap().grams()[2].1, 48);
}

#[test]
fn a_count_that_did_not_move_is_never_written_again() {
    let (p, mut saving, keeper, mut flash) = counted();
    for k in 2..20 {
        round(&p, &mut saving, &keeper, &mut flash, k * SAVE_EVERY_MS);
    }
    assert_eq!(flash.tries, 1);
}

#[test]
fn a_reset_the_flash_refused_is_written_again_and_the_restart_finds_the_zero() {
    let (mut p, mut saving, keeper, mut flash) = counted();
    flash.refuse = 1;
    let said = reset(&mut p, &mut saving, &keeper, &mut flash, 400_000);
    assert!(said.is_err(), "a refused write was answered as done");
    assert_eq!(Pellets::restore(flash.held.as_deref().unwrap()).unwrap().grams()[2].1, 40, "the flash still holds the old count");
    // the burner stands still: nothing moves the count, and the board must write the zero it owes all the same
    round(&p, &mut saving, &keeper, &mut flash, 400_000 + RETRY_MS);
    assert_eq!(Pellets::restore(flash.held.as_deref().unwrap()).unwrap().grams(), [("today", 0), ("month", 0), ("total", 0)]);
}

#[test]
fn a_reset_the_flash_took_is_written_once() {
    let (mut p, mut saving, keeper, mut flash) = counted();
    assert!(reset(&mut p, &mut saving, &keeper, &mut flash, 400_000).is_ok());
    let tries = flash.tries;
    for k in 1..40 {
        round(&p, &mut saving, &keeper, &mut flash, 400_000 + k * RETRY_MS);
    }
    assert_eq!(flash.tries, tries);
}

#[test]
fn a_write_the_flash_refuses_is_tried_again_every_retry_and_not_every_second() {
    let (mut p, mut saving, keeper, mut flash) = counted();
    p.observe(11_000, true, 240);
    p.observe(12_000, true, 240);
    flash.refuse = 2;
    let start = 2 * SAVE_EVERY_MS;
    for s in 0..(2 * RETRY_MS / 1000 + 1) {
        round(&p, &mut saving, &keeper, &mut flash, start + s * 1000);
    }
    assert_eq!(flash.tries, 1 + 3, "one try at the start, one after each retry: three, not one a second");
    assert_eq!(Pellets::restore(flash.held.as_deref().unwrap()).unwrap().grams()[2].1, 48);
}

#[test]
fn a_new_day_is_written_at_once_while_the_flash_takes_it_and_after_a_retry_when_it_refuses() {
    let (mut p, mut saving, keeper, mut flash) = counted();
    let tomorrow = Some(Now { date: Date { year: 2026, month: 10, day: 5 }, minute: 0, offset_min: 180 });
    p.at(tomorrow);
    flash.refuse = 1;
    round(&p, &mut saving, &keeper, &mut flash, SAVE_EVERY_MS + 1);
    assert_eq!(flash.tries, 2, "the turn is written at once");
    round(&p, &mut saving, &keeper, &mut flash, SAVE_EVERY_MS + 2);
    assert_eq!(flash.tries, 2, "a refusal is not hammered every second");
    round(&p, &mut saving, &keeper, &mut flash, SAVE_EVERY_MS + 1 + RETRY_MS);
    assert_eq!(flash.tries, 3);
    assert_eq!(Pellets::restore(flash.held.as_deref().unwrap()).unwrap().tally.day, Some(Date { year: 2026, month: 10, day: 5 }));
}

#[test]
fn a_count_that_moved_is_owed_until_the_flash_took_it() {
    let (mut p, mut saving, keeper, mut flash) = counted();
    assert!(!saving.owed(&p), "written and not moved since");
    p.observe(11_000, true, 240);
    p.observe(12_000, true, 240);
    assert!(saving.owed(&p), "moved since the write");
    flash.refuse = 1;
    let pending = saving.now(&keeper, &p, 13_000);
    let refused = keeper.write(pending.snapshot.clone(), |text| flash.put(text));
    saving.finish(&pending, refused.is_ok());
    assert!(saving.owed(&p), "a refused write leaves the count owed");
    let pending = saving.now(&keeper, &p, 14_000);
    let taken = keeper.write(pending.snapshot.clone(), |text| flash.put(text));
    saving.finish(&pending, taken.is_ok());
    assert!(!saving.owed(&p), "the flash took it");
    let back = Pellets::restore(flash.held.as_deref().unwrap()).unwrap();
    assert!(back.grams()[2].1 >= p.grams()[2].1, "the count after a restart is not less than before: {:?} against {:?}", back.grams(), p.grams());
}
