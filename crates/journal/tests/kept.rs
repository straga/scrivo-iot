//! The log kept on flash: lines go down in batches, the ring turns over without growing,
//! and a restart carries on the files it finds instead of starting them again.

use std::collections::BTreeMap;

use journal::kept::{in_order, Folder, Kept, BATCH_BYTES, FILES, FILE_BYTES, FLUSH_AFTER_MS, PENDING_MOST};

/// A folder in memory that counts what the flash would have been asked to do.
#[derive(Default)]
struct Mem {
    files: BTreeMap<String, Vec<u8>>,
    appends: usize,
    refuse: bool,
    /// Refusals of the steps before the write: listing the folder, sizing a file, removing one.
    refuse_names: bool,
    refuse_size: bool,
    refuse_remove: bool,
}

impl Folder for Mem {
    fn names(&mut self) -> Result<Vec<String>, String> {
        if self.refuse_names {
            return Err("the folder could not be listed".into());
        }
        Ok(self.files.keys().cloned().collect())
    }
    fn size(&mut self, name: &str) -> Result<usize, String> {
        if self.refuse_size {
            return Err(format!("{name} could not be sized"));
        }
        self.files.get(name).map(Vec::len).ok_or_else(|| format!("no {name}"))
    }
    fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if self.refuse {
            return Err("the flash refused".into());
        }
        self.appends += 1;
        self.files.entry(name.into()).or_default().extend_from_slice(bytes);
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<(), String> {
        if self.refuse_remove {
            return Err(format!("{name} could not be removed"));
        }
        self.files.remove(name).map(|_| ()).ok_or_else(|| format!("no {name}"))
    }
    fn read_at(&mut self, name: &str, at: usize, buf: &mut [u8]) -> Result<usize, String> {
        let bytes = self.files.get(name).ok_or_else(|| format!("no {name}"))?;
        let rest = bytes.get(at..).unwrap_or(&[]);
        let n = buf.len().min(rest.len());
        buf[..n].copy_from_slice(&rest[..n]);
        Ok(n)
    }
}

impl Mem {
    /// Everything the ring holds, read the way the board serves it.
    fn read(&self) -> String {
        let names: Vec<String> = self.files.keys().cloned().collect();
        in_order(&names).iter().map(|n| String::from_utf8_lossy(&self.files[n]).into_owned()).collect()
    }
}

#[test]
fn lines_wait_for_their_batch_and_then_go_down_in_one_write() {
    let (mut kept, mut mem) = (Kept::new(), Mem::default());
    kept.add("automaton: stopped -> purging", 1_000);
    kept.add("igniter on", 2_000);
    assert!(!kept.due(1_000 + FLUSH_AFTER_MS - 1), "a batch was due before its oldest line had waited");
    assert!(kept.due(1_000 + FLUSH_AFTER_MS), "the oldest line waited its time and the batch is not due");
    kept.flush(&mut mem).unwrap();
    assert_eq!(mem.appends, 1, "two lines took {} writes, not one batch", mem.appends);
    assert_eq!(kept.writes(), 1);
    assert_eq!(mem.read(), "automaton: stopped -> purging\nigniter on\n");
}

#[test]
fn nothing_waiting_is_never_due_and_writes_nothing() {
    let (mut kept, mut mem) = (Kept::new(), Mem::default());
    assert!(!kept.due(10 * FLUSH_AFTER_MS));
    kept.flush(&mut mem).unwrap();
    assert_eq!(mem.appends, 0, "an empty batch reached the flash");
    assert_eq!(kept.writes(), 0);
}

#[test]
fn a_full_batch_is_due_before_its_time() {
    let mut kept = Kept::new();
    let line = "x".repeat(99);
    for _ in 0..BATCH_BYTES / 100 {
        kept.add(&line, 0);
    }
    assert!(!kept.due(1), "the batch is due a line short of its size");
    kept.add(&line, 0);
    assert!(kept.due(1), "a batch of {BATCH_BYTES} bytes waits for the clock");
}

#[test]
fn a_restart_carries_on_the_file_it_finds() {
    let mut mem = Mem::default();
    let mut before = Kept::new();
    before.add("before the restart", 0);
    before.flush(&mut mem).unwrap();
    // the board came up again: a new ring over the same folder
    let mut after = Kept::new();
    after.add("after the restart", 0);
    after.flush(&mut mem).unwrap();
    assert_eq!(mem.files.len(), 1, "a restart began a file of its own: {:?}", mem.files.keys());
    assert_eq!(mem.read(), "before the restart\nafter the restart\n");
}

#[test]
fn the_ring_turns_over_and_never_holds_more_than_its_files() {
    let mut mem = Mem::default();
    let mut kept = Kept::new();
    let line = "y".repeat(1023);
    // enough for the ring to go round more than once
    for n in 0..(FILES as usize + 2) * FILE_BYTES / 1024 {
        kept.add(&line, n as u64);
        kept.flush(&mut mem).unwrap();
        assert!(mem.files.len() <= FILES as usize, "the ring holds {} files", mem.files.len());
        for (name, bytes) in &mem.files {
            assert!(bytes.len() <= FILE_BYTES, "{name} grew to {} bytes", bytes.len());
        }
    }
    assert_eq!(mem.files.len(), FILES as usize);
    // and a restart after it carries on the newest, not one of the old
    let newest = in_order(&mem.files.keys().cloned().collect::<Vec<_>>()).last().cloned().unwrap();
    let mut after = Kept::new();
    after.add("the newest", 0);
    after.flush(&mut mem).unwrap();
    assert!(mem.read().ends_with("the newest\n"), "the line after a restart is not the last one read");
    assert!(mem.files.contains_key(&newest));
}

#[test]
fn files_are_read_by_their_number_and_strangers_are_left_out() {
    let names: Vec<String> = ["10.log", "9.log", "notes.txt", "11.log"].iter().map(|s| s.to_string()).collect();
    assert_eq!(in_order(&names), vec!["9.log", "10.log", "11.log"]);
}

#[test]
fn lines_the_flash_refused_wait_for_the_next_batch() {
    let (mut kept, mut mem) = (Kept::new(), Mem { refuse: true, ..Default::default() });
    kept.add("refused once", 0);
    assert!(kept.flush(&mut mem).is_err(), "a refused write was reported as done");
    assert_eq!(kept.writes(), 0);
    mem.refuse = false;
    kept.flush(&mut mem).unwrap();
    assert_eq!(mem.read(), "refused once\n");
}

#[test]
fn a_flash_that_keeps_refusing_costs_the_oldest_lines_and_says_how_many() {
    let (mut kept, mut mem) = (Kept::new(), Mem { refuse: true, ..Default::default() });
    let line = "z".repeat(99);
    let lines = PENDING_MOST / 100 + 10;
    for _ in 0..lines {
        kept.add(&line, 0);
        let _ = kept.flush(&mut mem);
    }
    mem.refuse = false;
    kept.flush(&mut mem).unwrap();
    let read = mem.read();
    assert!(read.len() <= PENDING_MOST + 200, "RAM held {} bytes the flash refused", read.len());
    assert!(read.contains("lines lost"), "lines went and nobody was told: {}", &read[..80]);
}

// ---- a refusal at any step of a batch keeps its lines ----

/// A ring whose current file is full: the next batch begins a new file and takes the oldest off.
fn a_full_ring() -> (Kept, Mem) {
    let (mut kept, mut mem) = (Kept::new(), Mem::default());
    let line = "f".repeat(1023);
    for n in 0..FILES as usize * FILE_BYTES / 1024 {
        kept.add(&line, n as u64);
        kept.flush(&mut mem).unwrap();
    }
    assert_eq!(mem.files.len(), FILES as usize, "the ring is not full yet");
    (kept, mem)
}

/// Flushes LINE into MEM through KEPT once refused by REFUSE and once let through, and says what the flash then holds.
fn refused_then_let_through(mut kept: Kept, mut mem: Mem, refuse: fn(&mut Mem, bool), line: &str) -> (Kept, Mem) {
    kept.add(line, 0);
    refuse(&mut mem, true);
    let before = kept.refusals();
    assert!(kept.flush(&mut mem).is_err(), "a refused step was reported as a written batch");
    assert!(!mem.read().contains(line), "the line reached the flash through a refused step");
    refuse(&mut mem, false);
    kept.flush(&mut mem).unwrap();
    assert!(mem.read().ends_with(&format!("{line}\n")), "the line a refused step held back is gone from the flash");
    assert_eq!(kept.refusals(), before + 1, "a refused batch was not counted, or a written one was");
    (kept, mem)
}

#[test]
fn a_folder_that_cannot_be_listed_at_the_first_batch_keeps_its_lines() {
    refused_then_let_through(Kept::new(), Mem::default(), |m, on| m.refuse_names = on, "igniter on");
}

#[test]
fn a_file_that_cannot_be_sized_at_the_first_batch_keeps_its_lines() {
    let mut mem = Mem::default();
    mem.files.insert("1.log".into(), b"before the restart\n".to_vec());
    refused_then_let_through(Kept::new(), mem, |m, on| m.refuse_size = on, "igniter on");
}

#[test]
fn a_refused_write_while_the_ring_turns_leaves_the_oldest_file_and_keeps_the_batch() {
    // the batch goes down first and the oldest file goes after it: a refused write followed by a restart must not
    // cost both the oldest file and the batch in RAM
    let (mut kept, mut mem) = a_full_ring();
    let oldest = in_order(&mem.files.keys().cloned().collect::<Vec<_>>())[0].clone();
    let was = mem.files[&oldest].clone();
    let line = "i".repeat(1023);
    kept.add(&line, 0);
    mem.refuse = true;
    assert!(kept.flush(&mut mem).is_err(), "a refused write was reported as a written batch");
    assert_eq!(mem.files.get(&oldest), Some(&was), "the oldest file went before the batch that replaces it was down");
    assert!(!mem.read().contains(&line), "the refused batch reached the flash");
    mem.refuse = false;
    kept.flush(&mut mem).unwrap();
    assert!(mem.read().ends_with(&format!("{line}\n")), "the batch the refused write held back is gone");
    assert!(!mem.files.contains_key(&oldest), "the ring turned and the oldest file stayed");
    assert_eq!(mem.files.len(), FILES as usize);
}

/// A full ring turned with REFUSE on for the steps after the write: the batch is down once, the words say so, the
/// refusal is counted, and the file left over goes at the ring's next turn.
fn refused_after_the_batch_is_down(refuse: fn(&mut Mem, bool)) {
    let (mut kept, mut mem) = a_full_ring();
    let line = "h".repeat(1023);
    kept.add(&line, 0);
    refuse(&mut mem, true);
    let told = kept.flush(&mut mem).expect_err("an oldest file that stayed was not told");
    assert!(told.contains("written"), "the words do not say the batch is down: {told}");
    assert_eq!(kept.refusals(), 1, "a refused step after the write was not counted");
    assert!(mem.read().ends_with(&format!("{line}\n")), "the batch did not go down before the oldest file was taken");
    assert_eq!(mem.files.len(), FILES as usize + 1, "the ring should keep one file more until it can take it off");
    // the next batch finds nothing waiting from this one, and the ring's next turn takes the file it could not
    refuse(&mut mem, false);
    kept.add("next", 0);
    kept.flush(&mut mem).unwrap();
    assert_eq!(mem.read().matches(&line).count(), 1, "the batch went down twice");
    let filler = "j".repeat(1023);
    for _ in 0..FILE_BYTES / 1024 {
        kept.add(&filler, 0);
        kept.flush(&mut mem).unwrap();
    }
    assert_eq!(mem.files.len(), FILES as usize, "the file left over was never taken off");
}

#[test]
fn a_file_that_cannot_be_removed_after_the_batch_is_down_is_told_and_nothing_goes_down_twice() {
    refused_after_the_batch_is_down(|m, on| m.refuse_remove = on);
}

#[test]
fn a_folder_that_cannot_be_listed_after_the_batch_is_down_is_told_and_nothing_goes_down_twice() {
    refused_after_the_batch_is_down(|m, on| m.refuse_names = on);
}

#[test]
fn a_flash_that_takes_every_batch_counts_no_refusal() {
    let (mut kept, mut mem) = a_full_ring();
    kept.add("igniter off", 0);
    kept.flush(&mut mem).unwrap();
    assert_eq!(kept.refusals(), 0, "batches the flash took were counted as refused");
}

#[test]
fn a_line_is_read_before_it_reaches_the_flash_and_not_after() {
    let (mut kept, mut mem) = (Kept::new(), Mem::default());
    kept.add("igniter on", 0);
    assert_eq!(kept.waiting(), "igniter on\n", "a line not yet on the flash is not read");
    assert!(!kept.due(1), "a batch was due at once");
    assert_eq!(kept.waiting(), "igniter on\n");
    assert!(kept.due(FLUSH_AFTER_MS));
    kept.flush(&mut mem).unwrap();
    assert_eq!(mem.read(), "igniter on\n");
    assert_eq!(kept.writes(), 1);
    // the pair: once on the flash, it is not read twice
    assert_eq!(kept.waiting(), "", "a line on the flash is still read as waiting");
}

#[test]
fn lines_nobody_writes_down_stay_bounded() {
    let mut kept = Kept::new();
    for n in 0..PENDING_MOST {
        kept.add("0123456789", n as u64);
    }
    assert!(kept.waiting().len() <= PENDING_MOST, "{} bytes wait", kept.waiting().len());
}
