//! Where a reading stands, by code: against the calibrated window, and against the converter's top.

use analog::{edge, standing, WINDOW_MV};

#[test]
fn a_reading_stands_under_inside_or_over_the_window_the_calibration_holds_in() {
    assert_eq!(standing(WINDOW_MV.0 - 1), "under");
    assert_eq!(standing(WINDOW_MV.0), "inside");
    assert_eq!(standing(1000), "inside");
    assert_eq!(standing(WINDOW_MV.1), "inside");
    assert_eq!(standing(WINDOW_MV.1 + 1), "over");
}

#[test]
fn the_top_of_the_scale_is_said_before_the_window_and_replaces_it() {
    assert_eq!(edge(true, 3153), "top");
    assert_eq!(edge(true, 1000), "top", "the top is the stronger fact, wherever the number stands");
    assert_eq!(edge(false, 3153), "over");
    assert_eq!(edge(false, 100), "under");
    assert_eq!(edge(false, 1000), "inside");
}
