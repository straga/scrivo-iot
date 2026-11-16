//! When the board writes the count: what the flash holds, and when it is asked again. Expected from what a person would lose by a power cut.

use soot::{Keeper, Saving, Soot, RETRY_MS, SAVE_EVERY_MS};
use tally::Date;

fn burn(s: &mut Soot) {
    for _ in 0..1200 {
        s.second(true, true, Some(100.0), Some(60.0));
    }
    s.second(false, true, None, None);
}

#[test]
fn a_count_that_changed_nothing_is_not_written() {
    let (s, keeper) = (Soot::default(), Keeper::new());
    let mut saving = Saving::new(&s, 0);
    assert!(saving.due(&keeper, &s, SAVE_EVERY_MS * 10).is_none());
    assert!(!saving.owed(&s));
}

#[test]
fn grams_wait_for_the_interval() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    s.observe_total_g(100);
    let mut saving = Saving::new(&s, 0);
    s.observe_total_g(200);
    assert!(saving.owed(&s));
    assert!(saving.due(&keeper, &s, SAVE_EVERY_MS - 1).is_none(), "grams are a slow loss: they wait their interval");
    assert!(saving.due(&keeper, &s, SAVE_EVERY_MS).is_some());
}

#[test]
fn a_burn_that_ended_is_written_at_once() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    let mut saving = Saving::new(&s, 0);
    burn(&mut s);
    assert!(saving.due(&keeper, &s, 1).is_some(), "a median of a burn cannot be had again");
}

#[test]
fn a_cleaning_is_written_at_once() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    let mut saving = Saving::new(&s, 0);
    s.cleaned(Some(Date { year: 2026, month: 11, day: 9 }));
    assert!(saving.due(&keeper, &s, 1).is_some(), "a restart must not bring the grams back");
}

#[test]
fn what_the_flash_refused_is_tried_again_after_the_retry_term_and_stays_owed() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    let mut saving = Saving::new(&s, 0);
    burn(&mut s);
    let pending = saving.due(&keeper, &s, 1).unwrap();
    saving.finish(&pending, false);
    assert!(saving.owed(&s));
    assert!(saving.due(&keeper, &s, 1 + RETRY_MS - 1).is_none());
    let again = saving.due(&keeper, &s, 1 + RETRY_MS).expect("tried again");
    saving.finish(&again, true);
    assert!(!saving.owed(&s));
    assert!(saving.due(&keeper, &s, 10_000_000).is_none(), "written is written");
}

#[test]
fn a_change_made_while_the_flash_was_writing_stays_owed() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    s.observe_total_g(0);
    let mut saving = Saving::new(&s, 0);
    burn(&mut s);
    let pending = saving.due(&keeper, &s, 1).unwrap();
    s.observe_total_g(500); // moved after the snapshot was taken
    saving.finish(&pending, true);
    assert!(saving.owed(&s), "what came after the snapshot is not in the flash");
}

#[test]
fn the_snapshot_written_is_the_one_the_count_was_taken_as() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    let mut saving = Saving::new(&s, 0);
    burn(&mut s);
    let pending = saving.due(&keeper, &s, 1).unwrap();
    let back = Soot::restore(&pending.snapshot.text).unwrap();
    assert_eq!(back.soot_c(), s.soot_c());
}

#[test]
fn a_count_taken_back_from_the_flash_owes_nothing() {
    let mut s = Soot::default();
    burn(&mut s);
    let back = Soot::restore(&s.snapshot()).unwrap();
    let saving = Saving::new(&back, 0);
    assert!(!saving.owed(&back));
}

// ---- the cleaning is answered by the flash: the board takes the snapshot of it at once, whatever is due, and says what the flash answered ----

#[test]
fn a_cleaning_snapshot_is_taken_at_once_whatever_is_due_and_holds_the_cleaned_count() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    burn(&mut s);
    s.observe_total_g(1000);
    s.observe_total_g(6000);
    let mut saving = Saving::new(&s, 0);
    s.cleaned(Some(Date { year: 2026, month: 11, day: 9 }));
    let pending = saving.now(&keeper, &s, 5);
    let back = Soot::restore(&pending.snapshot.text).unwrap();
    assert_eq!((back.grams_since_clean(), back.soot_c(), back.burns_kept()), (0, None, 0), "what the flash is given is the count after the cleaning");
    assert_eq!(back.cleaned_on(), Some(Date { year: 2026, month: 11, day: 9 }));
}

#[test]
fn a_cleaning_the_flash_took_is_written_and_one_it_refused_stays_owed_and_is_tried_again() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    let mut saving = Saving::new(&s, 0);
    s.cleaned(None);
    let refused = saving.now(&keeper, &s, 5);
    saving.finish(&refused, false);
    assert!(saving.owed(&s), "a cleaning the flash did not take is owed, and the person was told so");
    assert!(saving.due(&keeper, &s, 5 + RETRY_MS - 1).is_none());
    let again = saving.due(&keeper, &s, 5 + RETRY_MS).expect("the pass tries again by itself");
    saving.finish(&again, true);
    assert!(!saving.owed(&s));
}

#[test]
fn a_snapshot_taken_after_another_is_numbered_after_it_so_the_older_never_lands_over_the_newer() {
    let (mut s, keeper) = (Soot::default(), Keeper::new());
    let mut saving = Saving::new(&s, 0);
    burn(&mut s);
    let older = saving.due(&keeper, &s, 1).unwrap();
    s.cleaned(None);
    let newer = saving.now(&keeper, &s, 2);
    assert!(newer.snapshot.generation > older.snapshot.generation);
    // the newer is written first, as the desk of the flash may serve it first; the older is then passed over
    let mut flash = String::new();
    assert_eq!(keeper.write(newer.snapshot.clone(), |t| -> Result<(), ()> { flash = t.to_string(); Ok(()) }), Ok(true));
    assert_eq!(keeper.write(older.snapshot.clone(), |_| -> Result<(), ()> { panic!("the older snapshot was written over the newer") }), Ok(false));
    assert_eq!(Soot::restore(&flash).unwrap().burns_kept(), 0);
}
