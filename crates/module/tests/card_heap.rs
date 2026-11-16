//! A card is put together, published and read without the heap - counted by the allocator
//! itself. One test in this file on purpose: the count is the whole process's, and a test beside it on another thread
//! would add its own.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use module::card::{Card, Fill, Published, Reads};
use page::dictionary::Value;

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

static FAN: Published<8, 2> = Published::new();

#[test]
fn a_card_goes_from_its_owner_to_a_reader_without_the_heap_and_a_vec_of_strings_does_not() {
    // The host's critical section is a std mutex, and the first lock of one takes from the heap once (measured: 1, then
    // 0) - a fact of the host, not of a card; on the board it is IDF's. Taken once before the count.
    critical_section::with(|_| {});
    let mut read = 0;
    let heap = taken(|| {
        let mut card = Card::<8, 2>::new();
        card.put(0, Value::Number(31.5));
        card.put(1, Value::Word("hold"));
        card.text(2, "the comparator refused 5200 µs: ESP_ERR_INVALID_STATE");
        FAN.publish(card).unwrap();
        FAN.read(&mut |_, _| read += 1);
    });
    assert_eq!(heap, 0);
    assert_eq!(read, 3, "and it was read whole");
    // the pair: the count sees an allocation when there is one
    assert!(taken(|| drop(vec![String::from("a")])) > 0, "the counter counts");
}
