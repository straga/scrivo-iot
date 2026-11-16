//! A ring built on the heap without passing through the stack of the task that builds it (the
//! regulator's six rings, 5.4 KB, overflowed the 8 KB stack of `holding` on the way to `Box::new`).
//!
//! Its own test binary: a ring that does pass through the stack overflows it, and an overflow aborts the process.

use curve::Ring;

/// Many times the stack the thread is given, so a ring that is built on the stack cannot fit there.
const BIG: usize = 1 << 20;
const STACK: usize = 64 * 1024;

#[test]
fn a_boxed_ring_bigger_than_the_stack_is_built_on_a_small_stack() {
    let len = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(|| {
            let mut ring: Box<Ring<BIG, f32>> = Ring::boxed(f32::NAN);
            ring.push(1.5);
            ring.len()
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(len, 1);
}

#[test]
fn a_boxed_ring_reads_out_as_the_ring_built_in_place() {
    let mut boxed: Box<Ring<4>> = Ring::boxed(0);
    let mut placed: Ring<4> = Ring::new();
    assert_eq!(boxed.len(), 0);
    assert_eq!(boxed.points(), Vec::<u16>::new());
    for v in [10, 20, 30, 40, 50, 60] {
        boxed.push(v);
        placed.push(v);
    }
    assert_eq!(boxed.points(), placed.points());
    assert_eq!(boxed.points(), vec![30, 40, 50, 60]);
}

#[test]
fn a_boxed_ring_of_nothing_holds_nothing() {
    let mut ring: Box<Ring<0>> = Ring::boxed(7);
    ring.push(1);
    assert_eq!(ring.len(), 0);
    assert_eq!(ring.points(), Vec::<u16>::new());
}
