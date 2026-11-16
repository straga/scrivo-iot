//! The lines of the log worth reading after the fact, kept on the board's flash: a ring of numbered files in one
//! folder, written in batches, and read back in the order they were written.
//!
//! **A DEBUGGING TOOL, AND SIMPLE ON PURPOSE**: a line an event, a ring in /data, read over HTTP. What
//! it answers is the question the first live lighting could not - "did the igniter come on, and when" - once the
//! stream of the consoles ([`crate::Ring`]) had long gone past.
//!
//! **IN BATCHES, BECAUSE FLASH WEARS**: a line an event written at once is a write a second
//! through a kindling. Lines wait in RAM until [`BATCH_BYTES`] of them have gathered or the oldest has waited
//! [`FLUSH_AFTER_MS`], and a power cut loses at most that. Nothing here touches the flash itself: the board hands
//! in a [`Folder`], and the computer hands in one kept in memory.

use std::collections::VecDeque;

/// What a batch is written by: the board's vfs, or a folder in memory on the computer.
pub trait Folder {
    /// The names of the files in the folder.
    fn names(&mut self) -> Result<Vec<String>, String>;
    /// The size of the file NAME in bytes.
    fn size(&mut self, name: &str) -> Result<usize, String>;
    /// Adds BYTES to the end of the file NAME, making it when it is not there.
    fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String>;
    /// Removes the file NAME.
    fn remove(&mut self, name: &str) -> Result<(), String>;
    /// Reads from the file NAME at AT into BUF; the count read, 0 at its end.
    fn read_at(&mut self, name: &str, at: usize, buf: &mut [u8]) -> Result<usize, String>;
}

/// The end of the journal as the board holds it in memory: what /journal answers without reading the flash.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tail {
    /// Older lines are on the flash and not here.
    pub left_out: bool,
    /// The lines, oldest first, each ended by a newline; the first is whole.
    pub text: String,
}

/// The folder in vfs the ring lies in: in /data, what the board makes itself, which a kit's upload neither writes nor
/// takes off.
pub const FOLDER: &str = "/data/journal";
/// How many bytes of lines make a batch that is written without waiting for the clock.
pub const BATCH_BYTES: usize = 2048;
/// How long the oldest line waits before its batch is written: what a power cut can lose.
pub const FLUSH_AFTER_MS: u64 = 30_000;
/// How large one file of the ring grows before the next one is begun.
pub const FILE_BYTES: usize = 64 * 1024;
/// How many files the ring keeps; the oldest goes when one more is begun.
pub const FILES: u32 = 4;
/// How many bytes may wait in RAM while the flash refuses them; past it the oldest lines go, and are counted.
pub const PENDING_MOST: usize = 16 * 1024;
/// How much of the journal's end the board holds in memory for /journal: one file of the ring.
pub const TAIL_BYTES: usize = 64 * 1024;

/// The ring, as the board holds it between batches.
pub struct Kept {
    /// The lines waiting for their batch, each ended by a newline.
    pending: String,
    /// When the oldest of them was added; None - nothing waits.
    since_ms: Option<u64>,
    /// Lines let go while the flash refused them, told in the next batch that gets through.
    lost: u32,
    /// The number of the file being written and its size; None until the folder has been looked at.
    file: Option<(u32, usize)>,
    /// Batches written since the start.
    writes: u32,
    /// Batches the flash refused since the start, at any step of their writing.
    refusals: u32,
    /// The end of what is on the flash, from the start of a line, up to [`TAIL_BYTES`]: with `pending` after it, what
    /// /journal answers. Asked of the heap once, at its full size - the external RAM on a board with PSRAM.
    down: VecDeque<u8>,
    /// Older lines are on the flash and not in `down`.
    left_out: bool,
    /// `down` has been taken off the flash, or needs not be: once a start.
    recalled: bool,
}

impl Kept {
    /// A ring that has not looked at its folder yet: it does so at its first batch.
    pub const fn new() -> Self {
        Self {
            pending: String::new(),
            since_ms: None,
            lost: 0,
            file: None,
            writes: 0,
            refusals: 0,
            down: VecDeque::new(),
            left_out: false,
            recalled: false,
        }
    }

    /// Puts BYTES, whole lines, after the end of `down`, and lets its oldest lines go past [`TAIL_BYTES`].
    fn keep_down(&mut self, bytes: &[u8]) {
        if self.down.capacity() == 0 {
            self.down.reserve_exact(TAIL_BYTES + BATCH_BYTES);
        }
        self.down.extend(bytes);
        if self.down.len() > TAIL_BYTES {
            self.down.drain(..self.down.len() - TAIL_BYTES);
            let whole = self.down.iter().position(|b| *b == b'\n').map_or(0, |end| end + 1);
            self.down.drain(..whole);
            self.left_out = true;
        }
    }

    /// Adds LINE, taken at NOW_MS by the board's clock since its start.
    ///
    /// **RAM IS NOT A SECOND FLASH.** While the flash refuses, lines wait up to [`PENDING_MOST`] bytes; past it the
    /// oldest go and are counted, so a board whose vfs broke keeps its memory rather than its whole history.
    pub fn add(&mut self, line: &str, now_ms: u64) {
        self.pending.push_str(line);
        self.pending.push('\n');
        self.since_ms.get_or_insert(now_ms);
        while self.pending.len() > PENDING_MOST {
            let Some(end) = self.pending.find('\n') else { break };
            self.pending.drain(..=end);
            self.lost = self.lost.saturating_add(1);
        }
    }

    /// Whether a batch is to be written at NOW_MS: a full one at once, any other once its oldest line has waited.
    pub fn due(&self, now_ms: u64) -> bool {
        match self.since_ms {
            None => false,
            Some(since) => self.pending.len() >= BATCH_BYTES || now_ms.saturating_sub(since) >= FLUSH_AFTER_MS,
        }
    }

    /// Writes what is waiting into FOLDER; on a refusal the lines stay waiting for the next batch, and it is counted.
    ///
    /// **THE LINES LEAVE RAM ONLY ONCE THEY ARE ON THE FLASH.** Every step before the write - finding the newest
    /// file, listing the folder - can be refused too, and a refusal at any of them leaves the waiting lines as they
    /// were. An Err says in its words whether the lines wait or are already down.
    pub fn flush(&mut self, folder: &mut dyn Folder) -> Result<(), String> {
        if self.pending.is_empty() && self.lost == 0 {
            return Ok(());
        }
        let written = self.write(folder);
        if let Err(why) = written {
            self.refusals = self.refusals.saturating_add(1);
            return Err(match self.since_ms.is_some() {
                true => format!("a batch was not written to the flash: {why} - its lines wait for the next"),
                false => why,
            });
        }
        Ok(())
    }

    /// One batch into FOLDER; the waiting lines are let go only after the write went through.
    ///
    /// **THE BATCH GOES DOWN FIRST, THE OLDEST FILE AFTER IT.** Taken off before, a refused write and a restart would
    /// cost both the oldest file and the batch in RAM; taken off after, a refused removal costs a file more in the
    /// ring until its next turn.
    fn write(&mut self, folder: &mut dyn Folder) -> Result<(), String> {
        let told = match self.lost {
            0 => String::new(),
            n => format!("... {n} lines lost: the flash refused them\n"),
        };
        let bytes = told.len() + self.pending.len();
        let (mut number, mut size) = match self.file {
            Some(file) => file,
            None => newest(folder)?,
        };
        let turns = size > 0 && size + bytes > FILE_BYTES;
        if turns {
            number += 1;
            size = 0;
        }
        let batch = match told.is_empty() {
            true => std::mem::take(&mut self.pending),
            false => told + &self.pending,
        };
        if let Err(why) = folder.append(&name_of(number), batch.as_bytes()) {
            if self.pending.is_empty() {
                self.pending = batch;
            }
            return Err(why);
        }
        // what is on the flash now is the end of `down`; a recall after this would read it twice
        self.recalled = true;
        self.keep_down(batch.as_bytes());
        self.file = Some((number, size + bytes));
        self.pending.clear();
        self.since_ms = None;
        self.lost = 0;
        self.writes = self.writes.saturating_add(1);
        if turns {
            // the ring keeps FILES, the one just begun among them
            let left = |why: String| format!("the batch is written, but {why} - one file more till the ring turns");
            for name in in_order(&folder.names().map_err(left)?) {
                if number_of(&name).is_some_and(|n| n + FILES <= number) {
                    folder.remove(&name).map_err(left)?;
                }
            }
        }
        Ok(())
    }

    /// How many batches have been written to the flash since the start.
    pub fn writes(&self) -> u32 {
        self.writes
    }

    /// How many batches the flash has refused since the start.
    pub fn refusals(&self) -> u32 {
        self.refusals
    }

    /// The lines not yet on the flash, oldest first, each ended by a newline: what a reader gets after the files.
    pub fn waiting(&self) -> &str {
        &self.pending
    }

    /// Takes the end of the ring on FOLDER into memory, once a start and before the first batch: after a restart,
    /// /journal answers what came before it without reading the flash. What cannot be read is left out, and said so.
    pub fn recall(&mut self, folder: &mut dyn Folder) -> Result<(), String> {
        if std::mem::replace(&mut self.recalled, true) {
            return Ok(());
        }
        let recalled = self.read_end(folder);
        if recalled.is_err() {
            self.down.clear();
            self.left_out = true;
        }
        recalled
    }

    fn read_end(&mut self, folder: &mut dyn Folder) -> Result<(), String> {
        let mut ring = Vec::new();
        for name in in_order(&folder.names()?) {
            let size = folder.size(&name)?;
            ring.push((name, size));
        }
        let total: usize = ring.iter().map(|(_, size)| size).sum();
        let mut skip = total.saturating_sub(TAIL_BYTES);
        let cut = skip > 0;
        let mut buf = vec![0u8; 4096];
        for (name, size) in ring {
            if skip >= size {
                skip -= size;
                continue;
            }
            let mut at = std::mem::take(&mut skip);
            while at < size {
                let n = folder.read_at(&name, at, &mut buf)?.min(size - at);
                if n == 0 {
                    return Err(format!("{name} ended before its size"));
                }
                self.keep_down(&buf[..n]);
                at += n;
            }
        }
        if cut {
            let whole = self.down.iter().position(|b| *b == b'\n').map_or(0, |end| end + 1);
            self.down.drain(..whole);
            self.left_out = true;
        }
        Ok(())
    }

    /// The end of the journal, written or not: the last [`TAIL_BYTES`] of it from the start of a line.
    pub fn tail(&self) -> Tail {
        let mut bytes: Vec<u8> = Vec::with_capacity(self.down.len() + self.pending.len());
        bytes.extend(&self.down);
        bytes.extend_from_slice(self.pending.as_bytes());
        let mut left_out = self.left_out;
        if bytes.len() > TAIL_BYTES {
            let over = bytes.len() - TAIL_BYTES;
            let whole = bytes[over..].iter().position(|b| *b == b'\n').map_or(over, |end| over + end + 1);
            bytes.drain(..whole);
            left_out = true;
        }
        Tail { left_out, text: String::from_utf8_lossy(&bytes).into_owned() }
    }
}

impl Default for Kept {
    fn default() -> Self {
        Self::new()
    }
}

/// The name of the file numbered NUMBER.
fn name_of(number: u32) -> String {
    format!("{number}.log")
}

/// The number of the ring's file NAME; None - a file that is not the ring's.
fn number_of(name: &str) -> Option<u32> {
    let digits = name.strip_suffix(".log")?;
    match !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        true => digits.parse().ok(),
        false => None,
    }
}

/// The newest file in FOLDER and its size: where a board that has just come up carries on. An empty folder begins
/// at file one.
fn newest(folder: &mut dyn Folder) -> Result<(u32, usize), String> {
    match in_order(&folder.names()?).last() {
        Some(name) => Ok((number_of(name).unwrap_or(1), folder.size(name)?)),
        None => Ok((1, 0)),
    }
}

/// The files of the ring among NAMES, oldest first: the order they are read in.
pub fn in_order(names: &[String]) -> Vec<String> {
    let mut ring: Vec<(u32, &String)> = names.iter().filter_map(|n| number_of(n).map(|k| (k, n))).collect();
    ring.sort();
    ring.into_iter().map(|(_, n)| n.clone()).collect()
}
