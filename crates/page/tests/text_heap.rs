//! Free text in its slot is made and copied without the heap - counted by the allocator itself. One
//! test in this file on purpose: the count is the whole process's, and a test beside it on another thread would add
//! its own.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use page::dictionary::Text;

struct Counting;

static TAKEN: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TAKEN.fetch_add(1, Ordering::SeqCst);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// How many times F took from the heap.
fn taken(f: impl FnOnce()) -> usize {
    let before = TAKEN.load(Ordering::SeqCst);
    f();
    TAKEN.load(Ordering::SeqCst) - before
}

#[test]
fn a_text_is_made_and_copied_without_the_heap_and_a_string_is_not() {
    let said = "the comparator refused 5200 µs: ESP_ERR_INVALID_STATE";
    let mut text = Text::from("");
    assert_eq!(taken(|| text = Text::from(said)), 0, "making it");
    let mut copy = Text::from("");
    assert_eq!(taken(|| copy = text.clone()), 0, "copying it");
    assert_eq!(copy.as_str(), said);
    // the pair: the count sees an allocation when there is one - a String copied is one
    let string = said.to_string();
    let mut other = String::new();
    assert!(taken(|| other = string.clone()) > 0, "the counter counts");
    assert_eq!(other, string);
}
