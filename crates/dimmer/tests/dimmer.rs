//! A percent into a moment to fire, judged by values alone. The fan is not needed to say that a gate pulse must
//! not reach the next zero crossing - and that is the defect this crate exists to make impossible.

use dimmer::{order, Order, FULL_US, GATE_US, GUARD_US, LOW_US};

/// The half cycle of a 50 Hz mains, as the board measures it between two zero crossings.
const HALF: u32 = 10_000;

fn fired(percent: u32, half: u32) -> (u32, u32) {
    match order(percent, half).expect("a percent within bounds") {
        Order::Fire { delay_us, gate_us } => (delay_us, gate_us),
        Order::Off => panic!("{percent}% was refused as off"),
    }
}

#[test]
fn the_gate_pulse_never_reaches_the_next_zero_crossing() {
    // THE POINT OF THE WHOLE CRATE. Our own triac.c held the gate through the crossing, and the triac then fired
    // at the start of the next half cycle - full power at any setting.
    // Whatever the percent and whatever the mains does to the period, the pulse ends before the half cycle does.
    for half in [9_000, 9_800, HALF, 10_200, 12_000] {
        for percent in 1..=100 {
            let (delay, gate) = fired(percent, half);
            assert!(delay + gate + GUARD_US <= half,
                    "{percent}% of a {half} µs half cycle fires at {delay} for {gate} - that reaches the crossing");
        }
    }
}

#[test]
fn the_ends_of_the_scale_are_the_angles_the_owners_fan_was_set_to() {
    // 50 and 74 ticks of a 100 µs timer in the MicroPython burner of the garage (
    // power_max=50 full, power_min=74 lowest) - measured on THIS fan, and the starting point here.
    let (full, _) = fired(100, HALF);
    let (lowest, _) = fired(1, HALF);
    assert_eq!(full, FULL_US);
    assert_eq!(lowest, LOW_US);
    assert_eq!((FULL_US, LOW_US), (5_000, 7_400));
}

#[test]
fn a_higher_percent_is_more_air_and_never_fires_later() {
    // The old scale was upside down - a bigger number was a longer delay and less power (power_min=74 was the
    // LOWEST speed). Here a percent means what a person means by it, and the arithmetic does the turning.
    let mut previous = u32::MAX;
    for percent in 1..=100 {
        let (delay, _) = fired(percent, HALF);
        assert!(delay <= previous, "{percent}% fires at {delay}, later than the percent below it ({previous})");
        previous = delay;
    }
}

#[test]
fn zero_is_off_and_not_a_very_late_firing() {
    // A very late firing is not silence: the triac still conducts a sliver of the half cycle, and the motor still
    // hums. Off means the gate is never raised at all.
    assert_eq!(order(0, HALF), Ok(Order::Off));
}

#[test]
fn a_percent_above_a_hundred_is_refused_in_words() {
    let refusal = order(101, HALF).expect_err("101% taken");
    assert!(refusal.contains("101"), "{refusal:?} does not say what was asked");
    assert!(refusal.contains("100"), "{refusal:?} does not say what is allowed");
}

#[test]
fn a_half_cycle_no_mains_could_give_is_refused_in_words() {
    // The period comes from counting zero crossings, so a wild one means the crossings are wrong - fire on it and
    // the phase is anywhere. 50 Hz is 10 ms and 60 Hz is 8.3; a hundred microseconds or a whole second is neither.
    for absurd in [0, 100, 1_000, 100_000] {
        let refusal = order(50, absurd).expect_err("an absurd half cycle was taken");
        assert!(refusal.contains("half cycle"), "{refusal:?} does not name what was wrong");
    }
}

#[test]
fn the_gate_pulse_is_long_enough_for_the_triac_to_latch() {
    // 50 µs is what ESPHome sends (ac_dimmer.cpp:28) and what Espressif's own solution uses; a BT136 needs 2 µs.
    // The garage burner latched on 40: its RMT pulse was 400 ticks of 100 ns, whatever the comment beside it said.
    // So 50 is the floor here, and the live fan decides whether it wants more.
    for percent in [1, 50, 100] {
        let (_, gate) = fired(percent, HALF);
        assert!(gate >= GATE_US, "{percent}% sends a {gate} µs pulse, shorter than the {GATE_US} µs a triac takes");
    }
}

#[test]
fn the_same_percent_is_the_same_share_of_the_half_cycle() {
    // The angles were measured on a 50 Hz mains, and a percent has to mean the same SHARE of the half cycle when
    // the period drifts - otherwise the same setting is a different angle on a different day.
    let share = |half: u32| {
        let (delay, _) = fired(50, half);
        delay as f64 / half as f64
    };
    let (fifty, sixty) = (share(HALF), share(8_333));
    assert!((fifty - sixty).abs() < 0.01, "50% is {fifty} of a 50 Hz half cycle and {sixty} of a 60 Hz one");
}

#[test]
fn a_shorter_mains_period_moves_every_firing_earlier() {
    let (on_fifty, _) = fired(50, HALF);
    let (on_sixty, _) = fired(50, 8_333);
    assert!(on_sixty < on_fifty, "50% fires at {on_sixty} on 60 Hz and {on_fifty} on 50 Hz");
}

#[test]
fn every_percent_fires_after_the_crossing_and_before_the_middle_of_what_is_left() {
    // The pair to the pulse test: a firing must not land on the crossing itself either, where there is no current
    // for the triac to latch onto, and it must stay inside the angles the fan was measured with.
    for percent in 1..=100 {
        let (delay, _) = fired(percent, HALF);
        assert!(delay >= FULL_US, "{percent}% fires at {delay}, earlier than the fan's full-speed angle");
        assert!(delay <= LOW_US, "{percent}% fires at {delay}, later than the fan's lowest-speed angle");
    }
}

// ---- the half cycle, read off how many crossings came in a measured window ----

#[test]
fn a_thousand_crossings_in_ten_seconds_are_a_fifty_hertz_mains() {
    assert_eq!(dimmer::half_cycle_of(1_000, 10_000).expect("a mains"), 10_000);
}

#[test]
fn a_single_second_is_read_the_same_way_as_ten() {
    assert_eq!(dimmer::half_cycle_of(100, 1_000).expect("a mains"), 10_000);
}

#[test]
fn sixty_hertz_is_a_mains_too() {
    assert_eq!(dimmer::half_cycle_of(1_200, 10_000).expect("a mains"), 8_333);
}

#[test]
fn no_crossings_at_all_is_refused_and_says_the_detector_is_silent() {
    let why = dimmer::half_cycle_of(0, 10_000).expect_err("nothing came");
    assert!(why.contains("no crossings"), "{why:?}");
}

#[test]
fn too_few_crossings_for_a_mains_are_refused() {
    // the detector lost half of them mid-window: 25 Hz is not a mains, and an angle from it lands anywhere
    let why = dimmer::half_cycle_of(500, 10_000).expect_err("no mains");
    assert!(why.contains("no mains"), "{why:?}");
}

#[test]
fn too_many_crossings_for_a_mains_are_refused() {
    // edges bouncing, counted as crossings
    let why = dimmer::half_cycle_of(3_000, 10_000).expect_err("no mains");
    assert!(why.contains("no mains"), "{why:?}");
}

#[test]
fn a_window_of_no_time_is_refused_rather_than_divided_by() {
    let why = dimmer::half_cycle_of(1_000, 0).expect_err("no window");
    assert!(why.contains("no crossings") || why.contains("no mains"), "{why:?}");
}

#[test]
fn what_is_measured_feeds_the_order_it_was_measured_for() {
    // the two halves meet here: what the board counted is what the angle is taken as a share of
    let half = dimmer::half_cycle_of(1_200, 10_000).expect("a mains");
    assert_eq!(dimmer::order(100, half).expect("full"), dimmer::Order::Fire { delay_us: 4_166, gate_us: dimmer::GATE_US });
}

// ---- the two ends of what THIS fan does, measured on it ----

#[test]
fn the_fan_needs_more_to_start_than_to_keep_turning() {
    // The whole point of there being two numbers: one is what breaks it away from rest, the other what it will
    // still turn on once it is going. A start percent at or below the holding one means somebody wrote one number
    // into two names.
    assert!(dimmer::STARTS_AT > dimmer::HOLDS_AT, "{} is not above {}", dimmer::STARTS_AT, dimmer::HOLDS_AT);
}

#[test]
fn both_ends_are_percents_this_crate_will_take() {
    for percent in [dimmer::HOLDS_AT, dimmer::STARTS_AT] {
        let order = dimmer::order(percent, dimmer::NOMINAL_HALF_US).expect("a percent");
        assert!(matches!(order, dimmer::Order::Fire { .. }), "{percent}%: {order:?}");
    }
}

#[test]
fn starting_fires_earlier_in_the_half_cycle_than_holding() {
    // more air is a shorter delay: the start angle has to let more of the half cycle through than the holding one
    let start = dimmer::order(dimmer::STARTS_AT, dimmer::NOMINAL_HALF_US).expect("a percent");
    let hold = dimmer::order(dimmer::HOLDS_AT, dimmer::NOMINAL_HALF_US).expect("a percent");
    let delay = |o| match o {
        dimmer::Order::Fire { delay_us, .. } => delay_us,
        dimmer::Order::Off => unreachable!("neither end is off"),
    };
    assert!(delay(start) < delay(hold), "{:?} is not earlier than {:?}", start, hold);
}

// ---- what the CLOCK says a minute owes, which is the only reference the detector does not feed ----

#[test]
fn a_minute_of_fifty_hertz_owes_six_thousand_crossings() {
    assert_eq!(dimmer::owed_by_the_clock(60_000), 6_000);
}

#[test]
fn a_second_owes_a_hundred() {
    assert_eq!(dimmer::owed_by_the_clock(1_000), 100);
}

#[test]
fn a_window_that_is_not_a_whole_second_still_owes_what_it_owes() {
    // the window is measured, not assumed: a minute that took 60 300 ms owes the crossings of 60 300 ms
    assert_eq!(dimmer::owed_by_the_clock(60_300), 6_030);
}

#[test]
fn what_the_clock_owes_is_not_taken_from_the_detector() {
    // THE POINT OF THIS NUMBER. Counting crossings and counting gate pulses both grow out of the same pin, so
    // they agree even when a crossing is missed - a measure that cannot show a disagreement proves nothing. The
    // clock is the one reference the detector does not feed.
    for crossings in [0, 5_000, 6_000, 9_999] {
        let _ = crossings;
        assert_eq!(dimmer::owed_by_the_clock(60_000), 6_000);
    }
}

// ---- a gap between crossings: the one measure that can show a SINGLE miss ----

#[test]
fn a_gap_of_one_half_cycle_missed_nothing() {
    assert_eq!(dimmer::missed_in(10_000, 10_000), 0);
}

#[test]
fn a_gap_of_two_half_cycles_missed_one_crossing() {
    // THE POINT OF MEASURING GAPS AT ALL. A sum over a minute averages a single miss away - 5999 against 6000
    // sits inside what the grid itself wanders - while a gap says it at once: 20 ms where 10 belongs.
    assert_eq!(dimmer::missed_in(20_000, 10_000), 1);
}

#[test]
fn three_half_cycles_missed_two() {
    assert_eq!(dimmer::missed_in(30_000, 10_000), 2);
}

#[test]
fn the_grid_drifting_is_not_a_miss() {
    // the grid moves by fractions of a percent; a miss doubles. Nothing in between is a real mains.
    for gap in [9_900, 10_100, 11_000, 12_500, 14_000] {
        assert_eq!(dimmer::missed_in(gap, 10_000), 0, "gap {gap}");
    }
}

#[test]
fn sixty_hertz_counts_its_own_misses() {
    assert_eq!(dimmer::missed_in(8_333, 8_333), 0);
    assert_eq!(dimmer::missed_in(16_666, 8_333), 1);
}

#[test]
fn a_gap_shorter_than_a_half_cycle_missed_nothing_either() {
    // bounce, or a crossing counted twice: it is not a MISS, and this measure does not pretend to see it
    for gap in [0, 5, 400, 5_000] {
        assert_eq!(dimmer::missed_in(gap, 10_000), 0, "gap {gap}");
    }
}

#[test]
fn a_half_cycle_of_nothing_is_refused_rather_than_divided_by() {
    assert_eq!(dimmer::missed_in(20_000, 0), 0);
}

// ---- the first seconds after a start, before the mains is measured ----

#[test]
fn before_the_mains_is_measured_full_air_fires_on_the_nominal_half_cycle() {
    // a board that came up over a fire blows at once: ten seconds of counting are ten seconds of fire without air
    let half = dimmer::half_to_fire(None, 100).expect("a live detector and no measurement yet refused full air");
    assert_eq!(half, dimmer::NOMINAL_HALF_US);
    assert_eq!(dimmer::order(100, half), Ok(dimmer::Order::Fire { delay_us: dimmer::FULL_US, gate_us: dimmer::GATE_US }));
}

#[test]
fn before_the_mains_is_measured_a_kick_fires_too() {
    // a fire carried on or burnt out on low air is held by the regulator, which breaks the fan away with a kick
    let half = dimmer::half_to_fire(None, 99).expect("a kick was refused before the mains was measured");
    assert!(matches!(dimmer::order(33, half), Ok(dimmer::Order::Fire { .. })));
}

#[test]
fn a_silent_detector_fires_nothing_before_the_mains_is_measured() {
    // the pair: with no crossings the gate would fire at no phase at all
    let why = dimmer::half_to_fire(None, 0).expect_err("a silent detector was fired against");
    assert!(why.contains("crossing"), "{why}");
}

#[test]
fn a_detector_counting_both_edges_fires_nothing_before_the_mains_is_measured() {
    assert!(dimmer::half_to_fire(None, 200).is_err(), "twice the crossings a mains gives was taken for one");
}

#[test]
fn a_measured_mains_is_fired_against_as_measured() {
    assert_eq!(dimmer::half_to_fire(Some(Ok(9_980)), 100), Ok(9_980));
}

#[test]
fn a_window_that_counted_no_mains_is_not_replaced_by_the_nominal() {
    // the board counted and found something that is not a mains: the nominal would fire at a phase it has no reason for
    let refused = dimmer::half_to_fire(Some(Err("no mains".into())), 100);
    assert_eq!(refused, Err("no mains".into()));
}

// ---- what the task that reads the handler's ring does: the handler writes each edge down and calls nothing, the task reads ----

#[test]
fn a_ring_read_in_time_gives_the_places_written_since_the_last_read() {
    assert_eq!(dimmer::unread(5, 3, 8), dimmer::Unread { first: 3, count: 2, lost: 0 });
    assert_eq!(dimmer::unread(3, 3, 8), dimmer::Unread { first: 3, count: 0, lost: 0 });
    // a full ring is still all there
    assert_eq!(dimmer::unread(11, 3, 8), dimmer::Unread { first: 3, count: 8, lost: 0 });
}

#[test]
fn a_ring_the_handler_wrote_over_gives_the_last_places_and_says_how_many_were_lost() {
    // twelve written since the last read, eight places: the oldest four are gone and the count says so
    assert_eq!(dimmer::unread(15, 3, 8), dimmer::Unread { first: 7, count: 8, lost: 4 });
}

#[test]
fn the_counts_wrap_like_the_numbers_the_handler_adds_to() {
    assert_eq!(dimmer::unread(2, u32::MAX - 1, 8), dimmer::Unread { first: u32::MAX - 1, count: 4, lost: 0 });
}

// ---- what the round that counts the seconds makes of the edges the handler wrote down, raw ----

use dimmer::{Edge, EdgeReader};

fn rise(free_us: u16) -> Edge {
    Edge { rising: true, free_us, pulse_us: 0 }
}

fn fall(pulse_us: u16) -> Edge {
    Edge { rising: false, free_us: 0, pulse_us }
}

fn read(edges: &[Edge]) -> EdgeReader {
    let mut reader = EdgeReader::new();
    edges.iter().for_each(|e| reader.feed(*e));
    reader
}

#[test]
fn an_edge_written_as_a_word_comes_back_as_it_was() {
    for edge in [rise(0), rise(65_535), fall(693), Edge { rising: true, free_us: 4_321, pulse_us: 19_999 }, Edge { rising: false, free_us: 65_535, pulse_us: 32_767 }] {
        assert_eq!(Edge::unpack(edge.pack()), edge);
    }
    assert_ne!(rise(10).pack(), fall(10).pack(), "the way is in the word");
}

#[test]
fn the_gaps_between_the_rises_are_judged_and_the_falls_between_them_are_not_a_gap() {
    // a mains followed, a half cycle lost (20 ms), a mains followed, two lost (30 ms)
    let mut reader = read(&[rise(0), fall(400), rise(10_000), fall(400), rise(30_000), fall(400), rise(60_000), fall(400)]);
    assert_eq!(reader.gaps(), (3, 30_000));
}

#[test]
fn a_rise_straight_after_a_rise_is_a_doubled_edge_and_neither_a_gap_nor_a_move_of_the_one_it_doubles() {
    // rise at 0, the same rise seen again at 3 ms, the fall, the next rise at 10 ms: the gap is from the first rise
    let mut reader = read(&[rise(0), rise(3_000), fall(400), rise(10_000)]);
    assert_eq!(reader.doubled(), (1, 0));
    assert_eq!(reader.gaps(), (0, 10_000));
    // and a fall straight after a fall is doubled too, and its pulse is not a pulse
    let mut reader = read(&[rise(0), fall(400), fall(50), rise(10_000)]);
    assert_eq!(reader.doubled(), (0, 1));
    assert_eq!(reader.pulses(), Some((400, 400)));
}

#[test]
fn the_free_timer_wraps_and_the_gap_is_still_the_true_one() {
    let mut reader = read(&[rise(60_000), fall(400), rise(4_464)]);
    assert_eq!(reader.gaps(), (0, 10_000), "60000 to 4464 across the wrap is 10 ms");
}

#[test]
fn the_pulses_are_the_narrowest_and_widest_of_the_falls_and_none_when_there_was_none() {
    let mut reader = read(&[rise(0), fall(400), rise(10_000), fall(693), rise(20_000), fall(100)]);
    assert_eq!(reader.pulses(), Some((100, 693)));
    assert_eq!(read(&[rise(0), rise(10_000)]).pulses(), None, "nothing was seen to fall");
}

#[test]
fn what_is_asked_for_starts_again_from_nothing_and_the_doubled_edges_keep_counting() {
    let mut reader = read(&[rise(0), rise(1_000), fall(400), rise(20_000), fall(500)]);
    assert_eq!(reader.gaps(), (1, 20_000));
    assert_eq!(reader.pulses(), Some((400, 500)));
    assert_eq!((reader.gaps(), reader.pulses()), ((0, 0), None));
    assert_eq!(reader.doubled(), (1, 0), "the total of the extra edges is not what a minute asks for");
}

#[test]
fn a_ring_written_over_breaks_the_chain_so_no_gap_is_made_across_what_was_lost() {
    let mut reader = EdgeReader::new();
    reader.feed(rise(0));
    reader.feed(fall(400));
    reader.lost();
    // the next rise is the first of a new chain: not a gap of 50 ms to a rise nobody can vouch for, and not a doubled rise either
    reader.feed(rise(50_000));
    assert_eq!((reader.gaps(), reader.doubled()), ((0, 0), (0, 0)));
    reader.feed(fall(300));
    reader.feed(rise(60_000));
    assert_eq!(reader.gaps(), (0, 10_000));
}

// ---- what a minute of the gate's accounting is worth in the journal ----
// 8 warnings in ten minutes of fire, one every ~75 s, said "0 crossings lost ... a second short by 1 gate pulses": the line of wellbeing stood at `warn`, a half cycle was never lost, and it drowned the one that would be.

use dimmer::Minute;

#[test]
fn a_minute_with_no_crossing_lost_and_no_edge_written_over_says_nothing_even_if_one_second_was_short_by_a_pulse() {
    let minute = Minute { missed: 0, worst_missed: 1, edges_lost: 0 };
    assert!(!minute.worth_a_line());
}

#[test]
fn a_minute_with_a_crossing_lost_by_the_gaps_is_worth_a_line() {
    assert!(Minute { missed: 1, worst_missed: 1, edges_lost: 0 }.worth_a_line());
}

#[test]
fn a_minute_in_which_the_ring_of_edges_was_written_over_is_worth_a_line_too() {
    assert!(Minute { missed: 0, worst_missed: 0, edges_lost: 3 }.worth_a_line());
}

#[test]
fn both_in_one_minute_are_one_line_with_both_numbers() {
    let words = Minute { missed: 2, worst_missed: 4, edges_lost: 3 }.words();
    assert!(words.contains("2 crossings") && words.contains("3 times") && words.contains("4 gate pulses"), "{words}");
}
