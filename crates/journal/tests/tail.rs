//! The end of the journal in memory (6.6, the burner on 26.09): /journal answers it without reading the flash. It holds
//! the lines written and the lines waiting, the last TAIL_BYTES of them from the start of a line, and after a restart
//! it is taken once off the ring on the flash.

use std::collections::BTreeMap;

use journal::kept::{Folder, Kept, Tail, FILE_BYTES, TAIL_BYTES};

#[derive(Default)]
struct Mem {
    files: BTreeMap<String, Vec<u8>>,
    refuse: bool,
    reads: usize,
}

impl Folder for Mem {
    fn names(&mut self) -> Result<Vec<String>, String> {
        Ok(self.files.keys().cloned().collect())
    }
    fn size(&mut self, name: &str) -> Result<usize, String> {
        self.files.get(name).map(Vec::len).ok_or_else(|| format!("no {name}"))
    }
    fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if self.refuse {
            return Err("the flash refused".into());
        }
        self.files.entry(name.into()).or_default().extend_from_slice(bytes);
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<(), String> {
        self.files.remove(name).map(|_| ()).ok_or_else(|| format!("no {name}"))
    }
    fn read_at(&mut self, name: &str, at: usize, buf: &mut [u8]) -> Result<usize, String> {
        self.reads += 1;
        let bytes = self.files.get(name).ok_or_else(|| format!("no {name}"))?;
        let rest = bytes.get(at..).unwrap_or(&[]);
        let n = buf.len().min(rest.len());
        buf[..n].copy_from_slice(&rest[..n]);
        Ok(n)
    }
}

/// Lines added and written at a batch each FLUSH_AFTER_MS, as the flash's owner does; the time it ended at.
fn run(kept: &mut Kept, mem: &mut Mem, lines: impl IntoIterator<Item = String>) -> u64 {
    let mut now = 0;
    for line in lines {
        kept.add(&line, now);
        if kept.due(now) {
            let _ = kept.flush(mem);
        }
        now += 100;
    }
    now
}

fn numbered(from: usize, to: usize) -> impl Iterator<Item = String> {
    (from..to).map(|n| format!("automaton: line {n:06}"))
}

#[test]
fn the_tail_holds_the_lines_written_and_the_lines_waiting_in_their_order() {
    let (mut kept, mut mem) = (Kept::new(), Mem::default());
    kept.add("igniter on", 0);
    kept.flush(&mut mem).unwrap();
    kept.add("flame seen", 1_000);
    assert_eq!(kept.waiting(), "flame seen\n");
    assert_eq!(kept.tail(), Tail { left_out: false, text: "igniter on\nflame seen\n".into() });
}

#[test]
fn a_journal_longer_than_the_tail_leaves_its_start_out_from_a_whole_line() {
    let (mut kept, mut mem) = (Kept::new(), Mem::default());
    run(&mut kept, &mut mem, numbered(0, 20_000));
    let tail = kept.tail();
    assert!(tail.left_out, "the start of a journal of {} lines was left out unsaid", 20_000);
    assert!(tail.text.len() <= TAIL_BYTES, "the tail holds {} bytes", tail.text.len());
    assert!(tail.text.len() > TAIL_BYTES - 64, "the tail holds only {} bytes", tail.text.len());
    assert!(tail.text.starts_with("automaton: line "), "the first line is cut: {:?}", &tail.text[..30]);
    assert!(tail.text.ends_with("automaton: line 019999\n"));
}

#[test]
fn lines_the_flash_refused_are_in_the_tail_all_the_same() {
    let (mut kept, mut mem) = (Kept::new(), Mem::default());
    mem.refuse = true;
    kept.add("igniter on", 0);
    assert!(kept.flush(&mut mem).is_err());
    assert_eq!(kept.tail().text, "igniter on\n");
}

#[test]
fn after_a_restart_the_tail_is_taken_off_the_flash_once() {
    let (mut kept, mut mem) = (Kept::new(), Mem::default());
    let now = run(&mut kept, &mut mem, numbered(0, 20_000));
    kept.flush(&mut mem).unwrap();
    let before = kept.tail();
    assert!(mem.files.values().map(Vec::len).sum::<usize>() > 2 * FILE_BYTES);

    let mut again = Kept::new();
    again.recall(&mut mem).unwrap();
    assert_eq!(again.tail(), before, "the tail after a restart is not the one before it");
    let reads = mem.reads;
    again.add("up again", now);
    again.recall(&mut mem).unwrap();
    assert_eq!(mem.reads, reads, "the flash was read again after the first time");
    assert!(again.tail().text.ends_with("automaton: line 019999\nup again\n"));
}

#[test]
fn a_short_journal_on_the_flash_comes_back_whole_and_nothing_is_said_left_out() {
    let mut mem = Mem::default();
    mem.files.insert("1.log".into(), b"igniter on\nflame seen\n".to_vec());
    let mut kept = Kept::new();
    kept.recall(&mut mem).unwrap();
    assert_eq!(kept.tail(), Tail { left_out: false, text: "igniter on\nflame seen\n".into() });
}
