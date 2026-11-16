//! What a window of samples comes to. No hardware here: the ADC is the device's half, and what the millivolts
//! mean belongs to the sensor's own crate.

#[test]
fn a_window_keeps_tenths_of_a_millivolt_rather_than_whole_ones() {
    // MEASURED ON THE BOARD, and the reason this exists. A step of this ADC is about 0.76 mV, so the mean of
    // twenty samples lands between whole millivolts - and rounded to a whole one, a pin sitting quietly at
    // 161.4 mV was drawn as square teeth jumping 160-161-163. That is our own arithmetic, not the sensor.
    let window = analog::window(&[161, 161, 161, 162]).expect("four samples are a window");
    assert_eq!(window.tenths, 1_613);
    // and the whole millivolts are still there for a person, rounded to the nearest
    assert_eq!(window.mv(), 161);
}

#[test]
fn a_window_of_one_sample_is_that_sample() {
    let window = analog::window(&[161]).expect("a sample is a window");
    assert_eq!(window.tenths, 1_610);
    assert_eq!(window.mv(), 161);
    assert_eq!(window.low, 161);
    assert_eq!(window.high, 161);
    assert_eq!(window.spread(), 0);
}

#[test]
fn a_window_is_the_mean_of_its_samples_and_carries_what_they_ranged_over() {
    // THE SPREAD TRAVELS WITH THE NUMBER. A mean on its own cannot say whether the pin was steady or swinging,
    // and that is the one thing a person setting the window up needs to see.
    let window = analog::window(&[158, 160, 161, 161]).expect("four samples are a window");
    assert_eq!(window.tenths, 1_600);
    assert_eq!(window.mv(), 160);
    assert_eq!(window.low, 158);
    assert_eq!(window.high, 161);
    assert_eq!(window.spread(), 3);
}

#[test]
fn the_mean_is_rounded_to_the_nearest_and_not_cut() {
    // 161 + 162 = 323 over two is 161.5, and cutting it would lean every reading of the board one way
    // in tenths there is nothing to round away: 161 and 162 average to exactly 161.5
    assert_eq!(analog::window(&[161, 162]).map(|w| w.tenths), Some(1_615));
    assert_eq!(analog::window(&[161, 161, 162]).map(|w| w.tenths), Some(1_613));
    // the whole millivolts still round to the nearest, and 161.5 goes up
    assert_eq!(analog::window(&[161, 162]).map(|w| w.mv()), Some(162));
}

#[test]
fn a_window_of_no_samples_is_nothing_and_not_a_zero() {
    // the same rule as sensor_mv: the absence of an answer is not the number zero. A round where every read
    // refused has no millivolts, and calling that 0 mV would show it as a sensor lying on the floor.
    assert_eq!(analog::window(&[]).is_none(), true);
}

#[test]
fn a_full_window_of_the_biggest_readings_does_not_wrap() {
    // the sum of a long window of u16 does not fit a u16, and a mean that wrapped would come back as an
    // ordinary small pressure - the same door the u32 of sensor_mv was caught in
    let brim = [u16::MAX; 64];
    // tenths of the biggest reading do not fit a u16, so the tenths are held as u32; the whole ones still fit
    assert_eq!(analog::window(&brim).map(|w| w.mv()), Some(u16::MAX));
    assert_eq!(analog::window(&brim).map(|w| w.tenths), Some(655_350));
}

#[test]
fn a_window_longer_than_a_sum_can_hold_is_nothing_and_not_a_wrapped_number() {
    // the same class in the other function of this crate, found by listing every narrowing cast rather than by
    // waiting for it to be reported: 65 538 samples of u16 overflow a u32 sum, and the mean of the overflowed
    // sum is an ordinary quiet number in the middle of the scale
    let many = vec![u16::MAX; 70_000];
    assert_eq!(analog::window(&many).map(|w| w.mv()), Some(u16::MAX));
}

// ---- the window the chip's calibration holds in ----

#[test]
fn a_number_inside_the_calibrated_window_is_quoted_without_a_word() {
    assert_eq!(analog::outside(150), "");
    assert_eq!(analog::outside(1_600), "");
    assert_eq!(analog::outside(2_450), "");
}

#[test]
fn a_number_outside_it_says_which_side_it_fell_off() {
    // BOTH SIDES, and they are different facts: under the window is a pin nothing drives, over it is where the
    // flame's darkness lives - 3153 mV of it - and the board quotes such a number only with this beside it.
    assert!(analog::outside(149).contains("150"));
    assert!(analog::outside(2_451).contains("2450"));
    assert_ne!(analog::outside(149), analog::outside(2_451));
}

#[test]
fn the_word_fits_the_row_it_is_added_to() {
    // The row already carries "3153 mV · spread 0 mV of 20" - about 28 characters - and the whole of it has to
    // read on a phone. Measured: the first version ran to 43 characters and took the row to three wrapped lines
    // (runs/4.1/flame_card_375.png).
    for mv in [0u16, 149, 2_451, 3_300] {
        assert!(analog::outside(mv).len() <= 32, "{mv}: {} characters", analog::outside(mv).len());
    }
}

// ---- the ceiling of the converter, which is a different fact from the calibrated window ----

#[test]
fn a_reading_at_the_ceiling_says_so_instead_of_saying_it_is_uncalibrated() {
    // THE STRONGER FACT REPLACES THE WEAKER ONE. Outside the calibrated window a number is still about the pin,
    // only less exactly; at the ceiling it is not about the pin at all - every voltage above that point comes
    // back as this one. Said together they would read as one hedge.
    assert_eq!(analog::ceiling_words(true, 3_153), " · at the top of the scale");
    assert_eq!(analog::ceiling_words(false, 3_153), " · over the calibrated 2450 mV");
    assert_eq!(analog::ceiling_words(false, 1_600), "");
}

#[test]
fn the_ceiling_word_fits_the_row_too() {
    for (ceiling, mv) in [(true, 3_153u16), (false, 149), (false, 2_451), (false, 1_600)] {
        let words = analog::ceiling_words(ceiling, mv);
        assert!(words.len() <= 32, "{ceiling}/{mv}: {} characters", words.len());
    }
}
