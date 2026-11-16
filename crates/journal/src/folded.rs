//! What a line is worth on the flash when it has been said before: a line that repeats is written once and counted, and the lines of the black box are written whole.
//!
//! **A LINE THAT REPEATS IS ONE LINE AND A COUNT.** The night of 24-25.10 pushed one warning of the analog unit through the flash every 180 ms, and by 04:26 the journal held nothing from before 04:21. A
//! line is the same line when its target and its words are the same and only its numbers differ (`cut after 5 of 20` and `cut after 6 of 20`); the first three are written at once, the rest are counted, and a line
//! comes again when a period has run since the last - with how many were held, and the last text whole, with the numbers of the last. A run that goes quiet is told once, as one line (`Fold::quiet`). The
//! cadence is [`crate::Repeats`]', the one the network's failures already keep; what is added is the key of a line and the last text a run keeps.
//!
//! **THE BLACK BOX IS NOT FOLDED.** A leap of the clock, a leap of the tick, the fronts lost and back are the lines the order of events is read by after a fall: each carries its own time and its own number, and
//! counting two of them as one would throw away the one thing they are for. They are told apart by their target (`black_box:` and a kind) and held by a limit a minute for each kind instead: past it
//! the lines are skipped and counted, and the first line of the next minute, or the quiet after, says how many.
//!
//! No clock here, and no flash: the seconds are the caller's, a counter that only goes forward, and the lines are the caller's too.

use crate::feed::Line;
use crate::repeats::Repeats;

/// What a target of the black box begins with. A line of any other target is folded.
pub const BLACK_BOX: &str = "black_box:";
/// The kinds of the black box: a leap of the board's clock, a leap of the tick, the fronts of the zero-crossing pin lost and back.
pub const BOX_CLOCK: &str = "black_box:clock";
pub const BOX_TICK: &str = "black_box:tick";
pub const BOX_FRONTS: &str = "black_box:fronts";
/// How many lines of one kind of the black box are written in a minute; the rest are skipped and counted.
pub const BLACK_BOX_PER_MINUTE: u32 = 12;
/// How many times a line is written before it begins to be counted. Three, so that a line which flips between two states inside a period (A, B, A) is written whole: the order of events is what the journal is read for.
pub const FOLD_BURST: u32 = 3;
/// How long a line that repeats is held before it is said again, and how long a run is quiet before it is told as it ended.
pub const FOLD_PERIOD_S: u64 = 60;
/// How many kinds of folded line are held at once; one more takes the place of the oldest, whose run is forgotten and whose lines were all written.
const RUNS: usize = 8;
/// How many kinds of the black box are held at once.
const KINDS: usize = 4;
/// The longest local time a run keeps: "2026-10-25 04:21:32".
const STAMP: usize = 24;

/// What to do with a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Write it.
    Write,
    /// Write it for the nth time of its run: this many since the last line that was written, itself included.
    Repeated { times: u32 },
    /// Write it, and first say that this many lines of its kind were skipped over the limit.
    WriteAfterSkipped { skipped: u32 },
    /// Do not write it: it is counted.
    Hold,
}

/// A run that went quiet, or lines of the black box that were skipped and no line followed to say so: one line for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// How many lines the run held; for skipped lines, how many were skipped.
    pub times: u32,
    /// The last text of the run, whole; empty for skipped lines.
    pub last: String,
    /// The local time of the last line.
    pub last_at: String,
    /// The lines were skipped over the limit of the black box.
    pub skipped: bool,
}

impl Summary {
    /// The line for the journal, from the target of the run.
    pub fn line(&self, target: &str) -> String {
        match self.skipped {
            true => format!("... {target}: {} lines skipped over {BLACK_BOX_PER_MINUTE} a minute", self.times),
            false => format!("... {target}: x{}, the last at {}: {}", self.times, self.last_at, self.last),
        }
    }
}

struct Run {
    key: u64,
    target: heapless::String<48>,
    repeats: Repeats,
    /// How many times the line has been written in its burst.
    seen: u32,
    last: Line,
    last_stamp: heapless::String<STAMP>,
    last_at_s: u64,
}

struct Kind {
    key: u64,
    target: heapless::String<48>,
    window_s: u64,
    written: u32,
    skipped: u32,
}

/// The lines of the journal on flash that repeat, and the kinds of the black box, as they stood.
pub struct Fold {
    runs: heapless::Vec<Run, RUNS>,
    kinds: heapless::Vec<Kind, KINDS>,
}

/// FNV-1a over a target and its words, each run of digits being one mark: what makes `5 of 20` and `6 of 20` the same line.
fn key_of(target: &str, text: &str) -> u64 {
    const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = BASIS;
    let mut eat = |byte: u8| hash = (hash ^ u64::from(byte)).wrapping_mul(PRIME);
    target.bytes().for_each(&mut eat);
    eat(0xff);
    let mut in_digits = false;
    for byte in text.bytes() {
        let digit = byte.is_ascii_digit();
        if digit && in_digits {
            continue;
        }
        in_digits = digit;
        eat(if digit { b'#' } else { byte });
    }
    hash
}

fn small<const N: usize>(text: &str) -> heapless::String<N> {
    let mut out = heapless::String::new();
    for ch in text.chars() {
        if out.push(ch).is_err() {
            break;
        }
    }
    out
}

impl Fold {
    pub const fn new() -> Self {
        Self { runs: heapless::Vec::new(), kinds: heapless::Vec::new() }
    }

    /// A line of TARGET with its words TEXT, said at the local time STAMP, AT_S seconds of a clock that only goes forward. -> what to do with it.
    pub fn put(&mut self, target: &str, text: &str, stamp: &str, at_s: u64) -> Verdict {
        match target.starts_with(BLACK_BOX) {
            true => self.boxed(target, at_s),
            false => self.folded(target, text, stamp, at_s),
        }
    }

    fn folded(&mut self, target: &str, text: &str, stamp: &str, at_s: u64) -> Verdict {
        let key = key_of(target, text);
        let place = match self.runs.iter().position(|run| run.key == key) {
            Some(place) => place,
            None => {
                if self.runs.is_full() {
                    self.runs.remove(0);
                }
                let run = Run { key, target: small(target), repeats: Repeats::new(FOLD_PERIOD_S as u32), seen: 0, last: Line::new(), last_stamp: heapless::String::new(), last_at_s: at_s };
                let _ = self.runs.push(run);
                self.runs.len() - 1
            }
        };
        let run = &mut self.runs[place];
        run.last = crate::feed::line_of(text);
        run.last_stamp = small(stamp);
        run.last_at_s = at_s;
        // THE FIRST FEW TIMES ARE WRITTEN WHOLE: a line that flips between two states (A, B, A) is the order of events, and only a line that keeps coming is a storm. The period of the count begins with the last of them.
        if run.seen < FOLD_BURST {
            run.seen += 1;
            if run.seen == FOLD_BURST {
                let _ = run.repeats.fail(at_s);
            }
            return Verdict::Write;
        }
        match run.repeats.fail(at_s) {
            None => Verdict::Hold,
            Some(said) if said.unsaid == 0 => Verdict::Write,
            Some(said) => Verdict::Repeated { times: said.unsaid + 1 },
        }
    }

    fn boxed(&mut self, target: &str, at_s: u64) -> Verdict {
        let key = key_of(target, "");
        let place = match self.kinds.iter().position(|kind| kind.key == key) {
            Some(place) => place,
            None => {
                if self.kinds.is_full() {
                    self.kinds.remove(0);
                }
                let _ = self.kinds.push(Kind { key, target: small(target), window_s: at_s, written: 0, skipped: 0 });
                self.kinds.len() - 1
            }
        };
        let kind = &mut self.kinds[place];
        if at_s >= kind.window_s + 60 {
            let skipped = kind.skipped;
            kind.window_s = at_s;
            kind.written = 1;
            kind.skipped = 0;
            return if skipped > 0 { Verdict::WriteAfterSkipped { skipped } } else { Verdict::Write };
        }
        if kind.written < BLACK_BOX_PER_MINUTE {
            kind.written += 1;
            return Verdict::Write;
        }
        kind.skipped += 1;
        Verdict::Hold
    }

    /// The runs that have gone quiet for a period at AT_S, and the skipped lines of the black box that no line followed to tell: one summary for each, each told once.
    /// Called by whoever owns the flash, once a second.
    pub fn quiet(&mut self, at_s: u64) -> Vec<(String, Summary)> {
        let mut said = Vec::new();
        self.runs.retain(|run| {
            let due = run.repeats.unsaid() > 0 && at_s >= run.last_at_s + FOLD_PERIOD_S;
            if due {
                said.push((run.target.to_string(), Summary { times: run.repeats.unsaid(), last: run.last.to_string(), last_at: run.last_stamp.to_string(), skipped: false }));
            }
            !due
        });
        for kind in self.kinds.iter_mut() {
            if kind.skipped > 0 && at_s >= kind.window_s + 60 {
                said.push((kind.target.to_string(), Summary { times: kind.skipped, last: String::new(), last_at: String::new(), skipped: true }));
                kind.skipped = 0;
            }
        }
        said
    }
}

impl Default for Fold {
    fn default() -> Self {
        Self::new()
    }
}
