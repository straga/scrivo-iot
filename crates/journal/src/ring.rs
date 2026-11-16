//! The last lines of the log as the network's loop keeps them for its consoles: a ring of N, made once, and a cursor a
//! console. A console that reads slower than lines come loses the oldest, is told how many, and costs no memory.

use crate::feed::Line;

/// What a console reads next.
#[derive(Debug, PartialEq)]
pub enum Read<'a> {
    Line(&'a str),
    /// This many lines went before the console read them.
    Lost(u64),
}

/// The last N lines.
pub struct Ring<const N: usize> {
    lines: Vec<Line>,
    /// How many lines have been pushed since the start: the number the next one gets.
    next: u64,
}

impl<const N: usize> Ring<N> {
    /// A ring of N empty slots, made here and never again.
    pub fn new() -> Self {
        let mut lines = Vec::with_capacity(N);
        lines.resize_with(N, Line::new);
        Self { lines, next: 0 }
    }

    /// Puts LINE in place of the oldest.
    pub fn push(&mut self, line: Line) {
        let at = (self.next % N as u64) as usize;
        self.lines[at] = line;
        self.next += 1;
    }

    /// Where a console that opens now starts: the lines from now on.
    pub fn now(&self) -> u64 {
        self.next
    }

    /// What the console at CURSOR reads next, the cursor moved past it; None when it has read everything.
    pub fn read(&self, cursor: &mut u64) -> Option<Read<'_>> {
        if *cursor >= self.next {
            return None;
        }
        let oldest = self.next.saturating_sub(N as u64);
        if *cursor < oldest {
            let lost = oldest - *cursor;
            *cursor = oldest;
            return Some(Read::Lost(lost));
        }
        let at = (*cursor % N as u64) as usize;
        *cursor += 1;
        Some(Read::Line(self.lines[at].as_str()))
    }
}

impl<const N: usize> Default for Ring<N> {
    fn default() -> Self {
        Self::new()
    }
}
