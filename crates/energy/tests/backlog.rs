//! A row of the CSV the flash refused is not lost: it goes to the file before the next one, in its order. Expected from what a person reading the history would want - no hole where the flash said no.

use energy::{Backlog, BACKLOG_MAX};

#[derive(Default)]
struct Files {
    refuse: u32,
    written: Vec<(String, String)>,
}

impl Files {
    fn put(&mut self, file: &str, line: &str) -> Result<(), String> {
        if self.refuse > 0 {
            self.refuse -= 1;
            return Err("NOT_ENOUGH_SPACE".into());
        }
        self.written.push((file.into(), line.into()));
        Ok(())
    }
}

#[test]
fn a_row_the_flash_took_is_written_and_nothing_waits() {
    let (mut backlog, mut files) = (Backlog::default(), Files::default());
    assert!(backlog.write("2026-10.csv", "a", |f, l| files.put(f, l)).is_ok());
    assert_eq!(files.written, [("2026-10.csv".to_string(), "a".to_string())]);
}

#[test]
fn a_row_the_flash_refused_waits_and_goes_before_the_next_in_its_order() {
    let (mut backlog, mut files) = (Backlog::default(), Files { refuse: 1, ..Files::default() });
    let said = backlog.write("2026-10.csv", "a", |f, l| files.put(f, l));
    assert_eq!(said.map_err(|(waiting, dropped, why)| (waiting, dropped, why)), Err((1, 0, "NOT_ENOUGH_SPACE".to_string())));
    assert!(files.written.is_empty());
    assert!(backlog.write("2026-10.csv", "b", |f, l| files.put(f, l)).is_ok());
    assert_eq!(files.written.iter().map(|(_, l)| l.as_str()).collect::<Vec<_>>(), ["a", "b"]);
}

#[test]
fn rows_of_two_months_keep_their_own_files_and_their_order() {
    let (mut backlog, mut files) = (Backlog::default(), Files { refuse: 1, ..Files::default() });
    let _ = backlog.write("2026-09.csv", "a", |f, l| files.put(f, l));
    backlog.write("2026-10.csv", "b", |f, l| files.put(f, l)).unwrap();
    assert_eq!(files.written, [("2026-09.csv".to_string(), "a".to_string()), ("2026-10.csv".to_string(), "b".to_string())]);
}

#[test]
fn a_flash_that_keeps_refusing_keeps_the_rows_up_to_the_bound_and_lets_the_oldest_go() {
    let (mut backlog, mut files) = (Backlog::default(), Files { refuse: u32::MAX, ..Files::default() });
    let mut last = None;
    for k in 0..BACKLOG_MAX + 5 {
        last = Some(backlog.write("f.csv", &k.to_string(), |f, l| files.put(f, l)));
    }
    assert_eq!(last.unwrap().map_err(|(waiting, dropped, _)| (waiting, dropped)), Err((BACKLOG_MAX, 1)));
    files.refuse = 0;
    backlog.write("f.csv", "z", |f, l| files.put(f, l)).unwrap();
    assert_eq!(files.written.len(), BACKLOG_MAX, "what waits never passes the bound");
    assert_eq!(files.written[0].1, "6", "the six that went - five over the bound, and one more for the row that came after - are the oldest");
    assert_eq!(files.written.last().unwrap().1, "z", "and the newest is the last");
}
