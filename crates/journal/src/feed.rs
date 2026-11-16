//! A line of the log on its way from whichever task wrote it to the one that reads it: a channel of fixed size, a line
//! a slot, nothing made per reader. The writer never waits: when the reader has fallen behind, the OLDEST line waiting
//! goes, and the count of lines lost is kept for the reader to say.

use core::cell::RefCell;
use core::future::poll_fn;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::Poll;

use between::Lock;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::waitqueue::WakerRegistration;

/// The longest line a slot holds, in bytes; a longer one is cut at a whole character and ends in "…".
pub const LINE: usize = 160;

/// One line of the log, as a slot holds it.
pub type Line = heapless::String<LINE>;

/// LINE as a slot holds it: the whole of it, or its head cut at a whole character with "…" at the end.
pub fn line_of(text: &str) -> Line {
    let mut line = Line::new();
    if line.push_str(text).is_ok() {
        return line;
    }
    const MORE: &str = "…";
    let mut end = LINE - MORE.len();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let _ = line.push_str(&text[..end]);
    let _ = line.push_str(MORE);
    line
}

/// A channel of N lines from any task to one reader. Its lines are in the heap, asked for once in [`Feed::new`]: where
/// the feed is declared holds only its head, and the slots go where the heap puts a block of their size - the external
/// RAM on a board with PSRAM, off the internal memory every task's stack comes from.
pub struct Feed<const N: usize> {
    queue: Mutex<Lock, RefCell<Queue>>,
    lost: AtomicU32,
}

/// The lines waiting, oldest at FIRST, and the reader to wake when one comes.
struct Queue {
    lines: Vec<Line>,
    first: usize,
    len: usize,
    reader: WakerRegistration,
}

impl<const N: usize> Feed<N> {
    /// A feed of N empty slots, made here and never again: a slot at a time, so no more than a line passes through
    /// the stack of the task that makes it.
    pub fn new() -> Self {
        let mut lines = Vec::with_capacity(N);
        lines.resize_with(N, Line::new);
        let queue = Queue { lines, first: 0, len: 0, reader: WakerRegistration::new() };
        Self { queue: Mutex::new(RefCell::new(queue)), lost: AtomicU32::new(0) }
    }

    /// Puts TEXT in, never waiting: a full feed lets its oldest line go and counts it.
    pub fn put(&self, text: &str) {
        let line = line_of(text);
        self.queue.lock(|queue| {
            let mut q = queue.borrow_mut();
            if q.len == N {
                q.first = (q.first + 1) % N;
                q.len -= 1;
                self.lost.fetch_add(1, Ordering::AcqRel);
            }
            let at = (q.first + q.len) % N;
            q.lines[at] = line;
            q.len += 1;
            q.reader.wake();
        });
    }

    /// The next line, when one waits.
    pub fn take(&self) -> Option<Line> {
        self.queue.lock(|queue| Self::first(&mut queue.borrow_mut()))
    }

    /// The next line, awaited.
    pub async fn next(&self) -> Line {
        poll_fn(|cx| {
            self.queue.lock(|queue| {
                let mut q = queue.borrow_mut();
                match Self::first(&mut q) {
                    Some(line) => Poll::Ready(line),
                    None => {
                        q.reader.register(cx.waker());
                        Poll::Pending
                    }
                }
            })
        })
        .await
    }

    /// The lines lost since the last asking; asking clears the count.
    pub fn lost(&self) -> u32 {
        self.lost.swap(0, Ordering::AcqRel)
    }

    fn first(q: &mut Queue) -> Option<Line> {
        if q.len == 0 {
            return None;
        }
        let at = q.first;
        q.first = (q.first + 1) % N;
        q.len -= 1;
        Some(core::mem::take(&mut q.lines[at]))
    }
}

impl<const N: usize> Default for Feed<N> {
    fn default() -> Self {
        Self::new()
    }
}
