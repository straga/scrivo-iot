//! The log's way costs the heap nothing once it is made: the feed from the tasks that log and the ring of
//! the consoles, put through ten thousand lines. Alone in its binary, one test, so no other test's allocations are
//! counted with it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use journal::{Feed, Line, Ring};

struct Counted;
static ASKED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ASKED.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTED: Counted = Counted;

#[test]
fn the_feed_and_the_ring_ask_the_heap_for_nothing_once_they_are_made() {
    let feed = Feed::<16>::new();
    let text = "W (1234) burner::fan: a line of the log as long as they come";
    let mut ring: Ring<64> = Ring::new();
    let mut cursor = ring.now();
    feed.put(text);
    feed.take();
    let before = ASKED.load(Ordering::Relaxed);
    for n in 0..10_000u32 {
        feed.put(text);
        if n % 3 == 0 {
            while let Some(line) = feed.take() {
                ring.push(line);
            }
        }
        if n % 7 == 0 {
            while ring.read(&mut cursor).is_some() {}
        }
    }
    let mut line = Line::new();
    let _ = core::fmt::Write::write_fmt(&mut line, format_args!("line {}", 1));
    ring.push(line);
    let asked = ASKED.load(Ordering::Relaxed) - before;
    assert_eq!(asked, 0, "the feed and the ring asked the heap {asked} times");
    assert!(ring.read(&mut cursor).is_some(), "nothing reached the ring - the count above measured nothing");
}
