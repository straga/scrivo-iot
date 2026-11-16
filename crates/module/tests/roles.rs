//! A quantity by its role: the module that plays it publishes it, and a consumer holds the role and
//! not the module - the concept's "a value comes by its role", without a consumer running the module's code.

use module::Role;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Level {
    Out,
    Steady,
}

#[test]
fn a_consumer_reads_the_role_as_its_player_last_published_it() {
    static WATER: Role<f32> = Role::new("water");
    let mut automaton = WATER.receiver().expect("a place");
    WATER.publish(Some(51.0));
    WATER.publish(Some(64.5));
    assert_eq!(automaton.now(), Some(64.5), "the last, not the first");
    static FLAME: Role<Level> = Role::new("flame");
    let mut eye = FLAME.receiver().expect("a place");
    FLAME.publish(Some(Level::Steady));
    assert_eq!(eye.now(), Some(Level::Steady));
    FLAME.publish(Some(Level::Out));
    assert_eq!(eye.now(), Some(Level::Out), "a state is given as it is, whichever it is");
}

#[test]
fn a_role_nobody_has_published_and_one_whose_player_cannot_say_are_none_and_never_a_zero() {
    static SILENT: Role<f32> = Role::new("flue_gas");
    let mut consumer = SILENT.receiver().expect("a place");
    assert_eq!(consumer.now(), None, "nobody has published");
    SILENT.publish(Some(180.0));
    SILENT.publish(None);
    assert_eq!(consumer.now(), None, "the player said it cannot say now, and the last number is not kept for it");
}

#[test]
fn a_role_has_three_places_and_a_fourth_consumer_is_refused() {
    static DRAUGHT: Role<f32> = Role::new("draught");
    let taken: Vec<_> = (0..3).map(|_| DRAUGHT.receiver().expect("one of three")).collect();
    assert!(DRAUGHT.receiver().is_none(), "a fourth place");
    drop(taken);
}

#[test]
fn a_reader_without_a_place_gets_the_last_published_and_none_before_any() {
    static BOILER: Role<f32> = Role::new("boiler");
    assert_eq!(BOILER.latest(), None);
    BOILER.publish(Some(64.5));
    assert_eq!(BOILER.latest(), Some(64.5));
    let places: Vec<_> = (0..3).map(|_| BOILER.receiver().expect("a place")).collect();
    assert_eq!(BOILER.latest(), Some(64.5), "and it takes none of the three places");
    drop(places);
}

// A sensor's role grows old: past three periods of its round, at least 3 s, it gives nothing out;
// past ten periods, at least 30 s, it is silent too long. The moments are the board's steady clock, set by the row.

use between::clock::Moment;
use core::time::Duration;
use module::{quiet_after, Quiet};

fn second() -> Duration {
    Duration::from_secs(1)
}

fn tenth() -> Duration {
    Duration::from_millis(100)
}

fn two_seconds() -> Duration {
    Duration::from_secs(2)
}

fn five_seconds() -> Duration {
    Duration::from_secs(5)
}

fn at(secs: u64) -> Moment {
    Moment::default() + Duration::from_secs(secs)
}

fn at_ms(ms: u64) -> Moment {
    Moment::default() + Duration::from_millis(ms)
}

#[test]
fn a_sensor_published_four_seconds_ago_gives_nothing_and_one_published_a_second_ago_gives_its_number() {
    static DRAUGHT: Role<f32> = Role::sensor("draught", second);
    let mut regulator = DRAUGHT.receiver().expect("a place");
    DRAUGHT.publish_at(Some(25.05), at(100));
    assert_eq!(regulator.now_at(at(101)), Some(25.05), "a second old: a reading");
    assert_eq!(regulator.now_at(at(104)), None, "four seconds old: the round has stopped, and its last number is not given out");
    assert_eq!(DRAUGHT.latest_at(at(104)), None, "the reader without a place is told the same");
    DRAUGHT.publish_at(Some(44.0), at(105));
    assert_eq!(regulator.now_at(at(105)), Some(44.0), "the round is back: its number is a reading again");
}

#[test]
fn the_silence_is_named_in_seconds_and_past_thirty_it_is_too_long() {
    static FLAME: Role<Level> = Role::sensor("flame", tenth);
    let mut automaton = FLAME.receiver().expect("a place");
    FLAME.publish_at(Some(Level::Steady), at(100));
    assert_eq!(automaton.quiet_at(at(102)), Quiet::Speaks);
    assert_eq!(automaton.quiet_at(at(104)), Quiet::Silent(Duration::from_secs(4)));
    assert_eq!(automaton.quiet_at(at(129)), Quiet::Silent(Duration::from_secs(29)), "29 s: not yet");
    assert_eq!(automaton.quiet_at(at(131)), Quiet::TooLong(Duration::from_secs(31)));
}

#[test]
fn a_round_of_a_tenth_of_a_second_is_given_three_seconds_and_not_three_tenths() {
    static FLAME: Role<Level> = Role::sensor("flame_fast", tenth);
    FLAME.publish_at(Some(Level::Out), at_ms(10_000));
    assert_eq!(FLAME.latest_at(at_ms(12_900)), Some(Level::Out), "2.9 s: within the floor of 3 s");
    assert_eq!(FLAME.latest_at(at_ms(13_100)), None, "3.1 s: silent");
}

#[test]
fn a_slow_round_is_given_three_of_its_periods_and_ten_before_it_is_too_long() {
    // the water: read every `poll_every`, 2 s by default and up to 60
    static WATER: Role<f32> = Role::sensor("water", two_seconds);
    WATER.publish_at(Some(64.5), at(100));
    assert_eq!(WATER.latest_at(at(105)), Some(64.5), "5 s is under three periods of 2 s");
    assert_eq!(WATER.latest_at(at(107)), None, "7 s is past them");
    static SLOW: Role<f32> = Role::sensor("water_slow", five_seconds);
    SLOW.publish_at(Some(64.5), at(100));
    assert_eq!(SLOW.quiet_at(at(114)), Quiet::Speaks, "14 s: under three periods of 5 s");
    assert_eq!(SLOW.quiet_at(at(149)), Quiet::Silent(Duration::from_secs(49)), "49 s: under ten periods");
    assert_eq!(SLOW.quiet_at(at(151)), Quiet::TooLong(Duration::from_secs(51)));
}

#[test]
fn a_sensor_that_never_published_is_silent_since_the_board_started() {
    static FLUE: Role<f32> = Role::sensor("flue_never", second);
    assert_eq!(FLUE.quiet_at(at(2)), Quiet::Speaks, "the round is still starting");
    assert_eq!(FLUE.quiet_at(at(40)), Quiet::TooLong(Duration::from_secs(40)), "its round never started");
    assert_eq!(FLUE.latest_at(at(40)), None);
}

#[test]
fn a_role_played_by_decisions_holds_its_last_for_as_long_as_nobody_publishes() {
    static FAN: Role<f32> = Role::new("fan_hand");
    let mut regulator = FAN.receiver().expect("a place");
    FAN.publish_at(Some(45.0), at(100));
    assert_eq!(regulator.now_at(at(3_700)), Some(45.0), "an hour later: the automaton simply decided nothing new");
    assert_eq!(regulator.quiet_at(at(3_700)), Quiet::Speaks);
}

// A round that goes on and publishes "no number" is as silent as one that stopped: the silence counts
// from the last NUMBER, not from the last publishing. A value the sensor says in its own words - the draught under its
// own zero - is a number of its kind, and is never silence.

#[derive(Clone, Copy, PartialEq, Debug)]
enum Draught {
    Is(f32),
    Under,
}

#[test]
fn a_flame_whose_round_publishes_nothing_every_second_falls_silent_as_one_that_stopped() {
    static FLAME: Role<Level> = Role::sensor("flame_none", tenth);
    let mut automaton = FLAME.receiver().expect("a place");
    FLAME.publish_at(Some(Level::Steady), at(100));
    for second in 101..=104 {
        FLAME.publish_at(None, at(second));
    }
    assert_eq!(automaton.quiet_at(at(104)), Quiet::Silent(Duration::from_secs(4)), "four seconds without a reading");
    assert_eq!(automaton.now_at(at(104)), None);
    static PAIR: Role<Level> = Role::sensor("flame_numbers", tenth);
    let mut pair = PAIR.receiver().expect("a place");
    for second in 100..=104 {
        PAIR.publish_at(Some(Level::Out), at(second));
    }
    assert_eq!(pair.quiet_at(at(104)), Quiet::Speaks, "a reading every second");
    assert_eq!(pair.now_at(at(104)), Some(Level::Out));
}

#[test]
fn water_read_as_nothing_for_thirty_one_seconds_is_silent_too_long() {
    static WATER: Role<f32> = Role::sensor("water_none", second);
    WATER.publish_at(Some(64.5), at(100));
    for second in 101..=131 {
        WATER.publish_at(None, at(second));
    }
    assert_eq!(WATER.quiet_at(at(131)), Quiet::TooLong(Duration::from_secs(31)));
}

#[test]
fn a_draught_under_its_own_zero_for_a_minute_is_a_reading_and_not_silence() {
    static DRAUGHT: Role<Draught> = Role::sensor("draught_under", tenth);
    let mut regulator = DRAUGHT.receiver().expect("a place");
    for second in 100..=160 {
        DRAUGHT.publish_at(Some(Draught::Under), at(second));
    }
    assert_eq!(regulator.quiet_at(at(160)), Quiet::Speaks, "the fan stands: no draught is what it says");
    assert_eq!(regulator.now_at(at(160)), Some(Draught::Under));
    DRAUGHT.publish_at(Some(Draught::Is(44.0)), at(161));
    assert_eq!(regulator.now_at(at(161)), Some(Draught::Is(44.0)));
}

#[test]
fn a_round_that_never_read_anything_is_silent_since_the_board_started() {
    static FLUE: Role<f32> = Role::sensor("flue_never_read", second);
    for second in 1..=40 {
        FLUE.publish_at(None, at(second));
    }
    assert_eq!(FLUE.quiet_at(at(40)), Quiet::TooLong(Duration::from_secs(40)));
}

#[test]
fn a_leap_at_the_end_of_a_period_does_not_make_a_sensor_silent() {
    // a sensor read every second, a second and a fifth since its reading, and the leap of 2 s the clock took in it
    assert_eq!(quiet_after(Duration::from_secs(1), Duration::from_millis(3200), Duration::from_secs(2)), Quiet::Speaks);
}

#[test]
fn a_sensor_silent_past_its_limit_across_a_leap_is_silent_by_what_it_really_was() {
    assert_eq!(
        quiet_after(Duration::from_secs(1), Duration::from_millis(5500), Duration::from_secs(2)),
        Quiet::Silent(Duration::from_millis(3500))
    );
}

#[test]
fn a_leap_makes_a_long_silence_no_longer_than_it_was() {
    assert_eq!(
        quiet_after(Duration::from_secs(1), Duration::from_secs(31), Duration::from_secs(2)),
        Quiet::Silent(Duration::from_secs(29)),
        "31 s by the clock with a leap in it is 29 s: not yet too long"
    );
}

#[test]
fn without_a_leap_the_silence_is_what_the_clock_says() {
    assert_eq!(quiet_after(Duration::from_secs(1), Duration::from_secs(4), Duration::ZERO), Quiet::Silent(Duration::from_secs(4)));
    assert_eq!(quiet_after(Duration::from_secs(1), Duration::from_secs(31), Duration::ZERO), Quiet::TooLong(Duration::from_secs(31)));
    assert_eq!(quiet_after(Duration::from_secs(1), Duration::from_secs(3), Duration::ZERO), Quiet::Speaks, "3 s: not yet");
}

#[test]
fn a_leap_longer_than_the_silence_is_no_silence_at_all() {
    assert_eq!(quiet_after(Duration::from_secs(1), Duration::from_secs(1), Duration::from_secs(2)), Quiet::Speaks);
}
