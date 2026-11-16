//! A feed's lines are in the heap, asked for once when it is made: where the feed
//! is declared - a static of the board - holds only its head, and the slots go where the heap puts a block that
//! size (the external RAM on a board with PSRAM). The pair is tests/feed_heap.rs: once made, it asks nothing. Alone in
//! its binary, so no other test's allocations are counted with it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use journal::{Feed, LINE};

struct Counted;
static BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTED: Counted = Counted;

#[test]
fn a_feed_takes_its_lines_from_the_heap_when_it_is_made() {
    let before = BYTES.load(Ordering::Relaxed);
    let feed = Feed::<32>::new();
    let asked = BYTES.load(Ordering::Relaxed) - before;
    assert!(asked >= 32 * LINE, "a feed of 32 lines asked the heap for {asked} bytes when it was made");
    feed.put("a line");
    assert_eq!(feed.take().as_deref(), Some("a line"), "the feed made in the heap does not carry a line");
}
