//! The level the thermostat orders by how far the held water is from the setpoint, on rows of readings: the
//! biggest level far from it, the middle one nearer, the small one nearest, the fire down to smouldering at the setpoint - as Plum's
//! MAX, MED and MIN.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: the big level while the water is under the setpoint by MORE than H2, the middle
//! one from H2 to H1, the small one under H1; a level goes down the moment the water is nearer than its line and comes back up only once
//! the water has gone farther than the line by the hysteresis, so that water standing on a line does not move the fire between two
//! levels; the middle is skipped when its row is the big one's or the small one's; with none chosen the three are one level and the
//! order is what it was before.
//!
//! Numbers of the rows: setpoint 60, H2 8, H1 3, the hysteresis 5. The three are told apart on purpose: the margin of the way up is
//! the hysteresis (5), so a row that held it at H1 or at H2 would stand on the wrong side of every line.

use automaton::{Feed, Heard, Step};
use thermostat::{Fire, Hold, Lockout, Rung, Temperatures, Thermostat, Tuning};

const WANT: f32 = 60.0;
const H2: f32 = 8.0;
const H1: f32 = 3.0;
const HYSTERESIS: f32 = 5.0;

fn big() -> Step {
    Step { pa: 120.0, feed: Feed { on_s: 4.6, off_s: 15.0 } }
}

fn mid() -> Step {
    Step { pa: 80.0, feed: Feed { on_s: 3.4, off_s: 17.0 } }
}

fn small() -> Step {
    Step { pa: 40.0, feed: Feed { on_s: 1.0, off_s: 21.0 } }
}

fn power() -> Step {
    Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } }
}

fn tuning(hold: Hold, mid: Step, small: Step) -> Tuning {
    Tuning { hold, want_c: WANT, hysteresis_c: HYSTERESIS, step: big(), mid, small, mid_c: H2, small_c: H1, power: power(), lockout: Lockout::default() }
}

/// Three levels of their own, held by the boiler.
fn three() -> thermostat::Sound {
    tuning(Hold::Boiler, mid(), small()).sound(88.0).expect("the numbers are sound")
}

fn at(boiler_c: f32) -> Temperatures {
    Temperatures { boiler_c: Some(boiler_c), return_c: Some(30.0), return_heard: Heard::Speaks, asked: None, room_c: None, room_target_c: None, fire: Fire::default() }
}

/// The level of the order at each of WATERS in turn, one thermostat across them: the memory of the way up is part of the answer.
fn walk(sound: &thermostat::Sound, waters: &[f32]) -> Vec<Step> {
    let mut thermostat = Thermostat::new();
    waters.iter().map(|w| thermostat.order(&at(*w), sound).step).collect()
}

/// The levels as words, for a row that is read as a path.
fn rungs(sound: &thermostat::Sound, waters: &[f32]) -> Vec<Option<Rung>> {
    let mut thermostat = Thermostat::new();
    waters.iter().map(|w| {
        thermostat.order(&at(*w), sound);
        thermostat.rung()
    }).collect()
}

fn one(sound: &thermostat::Sound, water: f32) -> Step {
    walk(sound, &[water])[0]
}

// ---------------------------------------------------------------- the way down

#[test]
fn far_from_the_setpoint_the_fire_burns_on_the_big_level() {
    assert_eq!(one(&three(), 30.0), big());
    assert_eq!(one(&three(), WANT - H2 - 0.1), big(), "51.9 is 8.1 under: more than H2");
}

#[test]
fn from_h2_to_h1_it_burns_on_the_middle_level_and_both_lines_belong_to_it() {
    assert_eq!(one(&three(), WANT - H2), mid(), "52 is 8 under: H2 itself is the middle's");
    assert_eq!(one(&three(), 55.0), mid());
    assert_eq!(one(&three(), WANT - H1), mid(), "57 is 3 under: H1 itself is the middle's");
}

#[test]
fn nearer_than_h1_it_burns_on_the_small_level_up_to_the_setpoint_and_over() {
    assert_eq!(one(&three(), WANT - H1 + 0.1), small(), "57.1 is 2.9 under");
    assert_eq!(one(&three(), 59.0), small());
    assert_eq!(one(&three(), WANT), small(), "at the setpoint the water says Enough and the fire smoulders; the level it would carry is the small one");
    assert_eq!(one(&three(), 70.0), small());
}

#[test]
fn a_level_goes_down_at_once_and_may_leap_over_the_middle_one() {
    assert_eq!(walk(&three(), &[40.0, 58.0]), [big(), small()], "from far to near in one second: no stop on the middle level");
    assert_eq!(walk(&three(), &[40.0, 54.0, 58.0]), [big(), mid(), small()]);
}

// ---------------------------------------------------------------- the way up

#[test]
fn a_level_comes_back_up_only_once_the_water_is_farther_than_its_line_by_the_hysteresis() {
    // the small level, then the water falls: H1 + hysteresis is 8 under the setpoint
    assert_eq!(walk(&three(), &[59.0, 57.0, 53.0]), [small(), small(), small()], "3 under and 7 under are inside the margin: the small level holds");
    assert_eq!(walk(&three(), &[59.0, 51.0]), [small(), mid()], "9 under is past H1 and the margin");
    // the middle level, then it falls farther: H2 + hysteresis is 13 under
    assert_eq!(walk(&three(), &[55.0, 49.0, 47.5]), [mid(), mid(), mid()], "11 under and 12.5 under hold the middle level");
    assert_eq!(walk(&three(), &[55.0, 46.5]), [mid(), big()], "13.5 under is past H2 and the margin");
    assert_eq!(walk(&three(), &[59.0, 40.0]), [small(), big()], "the way up may leap the middle one too");
}

#[test]
fn water_standing_on_a_line_does_not_move_the_fire_between_two_levels() {
    // 52 is H2 itself; the readings jitter a tenth of a degree either side of it
    let wobble = [52.1, 51.9, 52.1, 51.9, 52.1, 51.9];
    assert_eq!(walk(&three(), &wobble), [mid(); 6], "down at the line, and not back up for a tenth");
    // and the same on H1
    let wobble = [57.1, 56.9, 57.1, 56.9];
    assert_eq!(walk(&three(), &wobble), [small(); 4]);
}

// ---------------------------------------------------------------- a middle that is skipped

#[test]
fn a_middle_that_is_the_big_row_is_skipped_and_the_big_level_holds_down_to_h1() {
    let sound = tuning(Hold::Boiler, big(), small()).sound(88.0).expect("sound");
    assert_eq!(walk(&sound, &[40.0, 54.0, 58.0]), [big(), big(), small()]);
    assert_eq!(rungs(&sound, &[40.0, 54.0, 58.0]), [Some(Rung::Big), Some(Rung::Big), Some(Rung::Small)], "the middle is never the level it says");
    assert_eq!(walk(&sound, &[59.0, 51.0]), [small(), big()], "the way up from the small level lands on the big one: the middle is the same row");
}

#[test]
fn a_middle_that_is_the_small_row_is_skipped_and_the_small_level_begins_at_h2() {
    let sound = tuning(Hold::Boiler, small(), small()).sound(88.0).expect("sound");
    assert_eq!(walk(&sound, &[40.0, 54.0, 58.0]), [big(), small(), small()]);
    assert_eq!(rungs(&sound, &[40.0, 54.0, 58.0]), [Some(Rung::Big), Some(Rung::Small), Some(Rung::Small)]);
    assert_eq!(walk(&sound, &[58.0, 51.0, 46.5]), [small(), small(), big()]);
}

#[test]
fn with_no_level_chosen_the_three_are_the_one_and_the_order_is_what_it_was() {
    let sound = tuning(Hold::Boiler, big(), big()).sound(88.0).expect("sound");
    let waters = [20.0, 40.0, 52.0, 54.0, 57.0, 58.0, 59.5, 60.0, 61.0, 58.0, 50.0, 30.0];
    assert_eq!(walk(&sound, &waters), vec![big(); waters.len()], "same level in every second, whatever the water does");
}

// ---------------------------------------------------------------- what the held water is

#[test]
fn the_level_goes_by_the_sensor_the_burner_holds_and_not_by_the_boiler() {
    let sound = tuning(Hold::Return, mid(), small()).sound(88.0).expect("sound");
    let near_return = Temperatures { return_c: Some(58.0), ..at(10.0) };
    assert_eq!(Thermostat::new().order(&near_return, &sound).step, small(), "the boiler is far under and the mode does not look at it");
    let far_return = Temperatures { return_c: Some(30.0), ..at(59.0) };
    assert_eq!(Thermostat::new().order(&far_return, &sound).step, big());
}

#[test]
fn a_reading_that_is_not_there_leaves_the_level_where_it_was() {
    let sound = three();
    let blind = Temperatures { boiler_c: None, ..at(0.0) };
    let mut thermostat = Thermostat::new();
    assert_eq!(thermostat.order(&at(58.0), &sound).step, small());
    assert_eq!(thermostat.order(&blind, &sound).step, small(), "no reading is not a cold water and not a hot one: the level it had");
    assert_eq!(thermostat.rung(), Some(Rung::Small));
    assert_eq!(thermostat.left_c(), None, "and it says it has no distance this second");
}

#[test]
fn in_the_mode_of_power_the_level_is_the_one_chosen_and_there_are_no_levels() {
    let sound = tuning(Hold::Power, mid(), small()).sound(88.0).expect("sound");
    let mut thermostat = Thermostat::new();
    assert_eq!(thermostat.order(&at(58.0), &sound).step, power());
    assert_eq!(thermostat.order(&at(20.0), &sound).step, power());
    assert_eq!(thermostat.rung(), None);
    assert_eq!(thermostat.left_c(), None);
}

// ---------------------------------------------------------------- what the card says

#[test]
fn the_card_is_told_the_level_and_how_far_the_water_is() {
    let mut thermostat = Thermostat::new();
    assert_eq!((thermostat.rung(), thermostat.left_c()), (None, None), "before the first reading");
    thermostat.order(&at(54.0), &three());
    assert_eq!((thermostat.rung(), thermostat.left_c()), (Some(Rung::Mid), Some(6.0)));
    thermostat.order(&at(40.0), &three());
    assert_eq!((thermostat.rung(), thermostat.left_c()), (Some(Rung::Big), Some(20.0)));
    thermostat.order(&at(61.0), &three());
    assert_eq!((thermostat.rung(), thermostat.left_c()), (Some(Rung::Small), Some(-1.0)), "over the setpoint: the distance is below nought");
}

#[test]
fn the_levels_have_the_names_the_page_reads() {
    assert_eq!(thermostat::RUNGS, ["big", "mid", "small"]);
    assert_eq!([Rung::Big, Rung::Mid, Rung::Small].map(thermostat::rung_word), ["big", "mid", "small"]);
}

// ---------------------------------------------------------------- the door

fn refused(t: Tuning) -> (String, &'static str, Option<&'static str>) {
    let refusal = t.sound(88.0).expect_err("the door took numbers that make no stairs");
    (refusal.field, refusal.code, refusal.than)
}

#[test]
fn a_distance_that_is_nought_or_not_a_number_is_refused_by_its_name() {
    let base = tuning(Hold::Boiler, mid(), small());
    assert_eq!(refused(Tuning { mid_c: 0.0, ..base }), ("mid_c".into(), "above_nought", None));
    assert_eq!(refused(Tuning { small_c: -1.0, ..base }), ("small_c".into(), "above_nought", None));
    assert_eq!(refused(Tuning { small_c: f32::NAN, ..base }).1, "not_a_number");
}

#[test]
fn the_small_distance_has_to_be_under_the_middle_one() {
    let base = tuning(Hold::Boiler, mid(), small());
    assert_eq!(refused(Tuning { small_c: H2, ..base }), ("small_c".into(), "below", Some("mid_c")), "equal lines leave the middle no water");
    assert_eq!(refused(Tuning { small_c: 9.0, ..base }), ("small_c".into(), "below", Some("mid_c")));
    assert!(Tuning { small_c: 7.9, ..base }.sound(88.0).is_ok());
}

#[test]
fn a_level_nobody_can_burn_on_is_refused_by_the_field_it_stands_in() {
    let base = tuning(Hold::Boiler, mid(), small());
    let none = Step { pa: f32::NAN, feed: Feed { on_s: f32::NAN, off_s: f32::NAN } };
    assert_eq!(refused(Tuning { mid: none, ..base }).0, "mid.pa");
    assert_eq!(refused(Tuning { small: none, ..base }).0, "small.pa");
}
