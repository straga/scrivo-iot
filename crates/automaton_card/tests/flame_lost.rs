//! A flame that blinks does not end the fire, and a flue that warms slowly does not accuse the feed.
//! The flame sensor flickers and pellets settle, so a flame gone in rising or at work leaves the stage as it was - the
//! same feed, the same air - for `auto_no_flame_s`; back within it, nothing happened; not back, the fire is lit again,
//! and burnt off only when the attempts have not brought it back.
//! The flue that decides whether fuel is arriving is given `auto_no_fuel_s` of work, because a fire has inertia. A
//! person's Stop waits for none of this. The numbers are the rows' own, written below and handed through the mapping
//! the board and the stand share (`automaton_card::tuning`): what a board holds is a person's to set, and the rows are
//! about what the automaton does with such numbers.

mod common;

use automaton::{Asked, Doing, State, Why};
use common::pair::{Automaton, Burning, Press, Seen, Silence, Sound};
use common::sound;
use flame::Flame;

/// The automaton from a Start at 2 s and a Stop at STOP, if any: FLAME and FLUE give each second - a flue of no number
/// is one that says nothing -, the water stays cool.
fn run(seconds: u32, sound: &Sound, flame: impl Fn(u32) -> Flame, flue: impl Fn(u32) -> Option<f32>, stop: Option<u32>) -> Vec<Doing> {
    let mut a = Automaton::new(Burning::new());
    (0..seconds)
        .map(|at| {
            let seen = Seen { flame: flame(at), boiler_c: Some(40.0), return_c: None, flue_c: flue(at), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD };
            match at {
                2 => a.press(Press::Start, seen, sound),
                at if Some(at) == stop => a.press(Press::Stop, seen, sound),
                _ => a.second(seen, 1.0, sound),
            }
        })
        .collect()
}

/// Lit at 90 s; the flame holds the card's 5 s in the lighting and rising takes the card's 60 s, so the work begins at 154 s.
const WORK: usize = 154;

fn lit_from_90(at: u32) -> Flame {
    if at >= 90 { Flame::Steady } else { Flame::Out }
}

/// The flame of a fire that is dimming: weak from 90 s, the light the flue is asked under. The work begins at the same second as under a steady one.
fn dim_from_90(at: u32) -> Flame {
    if at >= 90 { Flame::Weak } else { Flame::Out }
}

/// Dark over DARK, lit from 90 s otherwise.
fn dark_over(dark: std::ops::Range<u32>) -> impl Fn(u32) -> Flame {
    move |at| if dark.contains(&at) { Flame::Out } else { lit_from_90(at) }
}

/// A flue that climbs well under what a working burner holds (100 °C), so that it never decides anything here: 12 °C a
/// minute, 94 °C at 320 s.
fn cool_climbing(at: u32) -> Option<f32> {
    Some(30.0 + 0.2 * at as f32)
}

/// The same flue, silent from GONE on: a flame gone for good over a flue that says nothing, so the pot counts as cold
/// and every attempt at lighting it again has its plug.
fn silent_from(gone: u32) -> impl Fn(u32) -> Option<f32> {
    move |at| if at >= gone { None } else { cool_climbing(at) }
}

/// The first second the automaton is in WANTED, or None.
fn first(seconds: &[Doing], wanted: State) -> Option<usize> {
    seconds.iter().position(|d| d.state == wanted)
}

#[test]
fn the_work_begins_where_these_rows_expect_it() {
    let seconds = run(200, &sound(&[]), lit_from_90, cool_climbing, None);
    assert_eq!(first(&seconds, State::Working), Some(WORK));
}

#[test]
fn a_flame_gone_twenty_seconds_at_work_changes_nothing_the_same_feed_and_the_same_air() {
    // a blink is not a lost fire: the work goes on as it was, the auger on its pair and the fan on its step
    let steady = run(400, &sound(&[]), lit_from_90, cool_climbing, None);
    // the blink stands at the same seconds of the work as before the flame was held in the lighting: 50 to 70 s into it, with the work at 154 s
    let blinked = run(400, &sound(&[]), dark_over(204..224), cool_climbing, None);
    let states: Vec<State> = blinked[WORK..].iter().map(|d| d.state).collect();
    assert!(states.iter().all(|s| *s == State::Working), "the work was left: {:?}", states.iter().find(|s| **s != State::Working));
    assert!(steady[204..224].iter().any(|d| d.auger), "the rows expect a dose inside the blink, or they prove nothing");
    let organs = |run: &[Doing]| run[WORK..].iter().map(|d| (d.auger, d.fan)).collect::<Vec<_>>();
    assert_eq!(organs(&blinked), organs(&steady), "the feed or the air of a work whose flame blinked is not a steady one's");
}

#[test]
fn a_flame_gone_for_good_in_rising_waits_the_same_and_feeds_nothing() {
    // the work's pair: a flame lost in rising is the rising's own, not the work's
    let seconds = run(400, &sound(&[]), dark_over(110..400), silent_from(110), None);
    assert_eq!(seconds[229].state, State::Rising, "120 seconds without a flame are still rising");
    assert_eq!(seconds[230].state, State::Kindling(1), "the 121st second without a flame");
    assert_eq!(seconds[90..230].iter().filter(|d| d.auger).count(), 0, "the auger turned over a pot with the dose in it");
}

#[test]
fn a_wait_set_on_the_card_is_the_wait() {
    let seconds = run(400, &sound(&[("auto_no_flame_s", 60.0)]), dark_over(200..400), silent_from(200), None);
    assert_eq!(seconds[259].state, State::Working);
    assert_eq!(seconds[260].state, State::Kindling(1), "the 61st second without a flame");
}

#[test]
fn a_person_s_stop_burns_off_at_once_whatever_the_flame_is_doing() {
    // the pair of the waiting: the waiting is for the fire, and a person is not a fire
    let seconds = run(400, &sound(&[]), dark_over(200..400), silent_from(200), Some(210));
    assert_eq!(seconds[210].state, State::BurningOff(None), "Stop waited for the flame's clock");
}

#[test]
fn a_flue_climbing_through_the_cold_mark_at_a_degree_a_minute_keeps_the_work_going() {
    // cold is under 70 °C for the whole window of 600 s, and this flue stands at 66 °C at the start of work and crosses the mark in four minutes
    let slow = |at: u32| Some(if at < WORK as u32 { 20.0 + at as f32 * 0.25 } else { 66.0 + (at - WORK as u32) as f32 / 60.0 });
    let seconds = run(WORK as u32 + 600, &sound(&[]), lit_from_90, slow, None);
    let left = seconds[WORK..].iter().position(|d| d.state != State::Working);
    assert_eq!(left, None, "the work was left at its {left:?} second: {:?}", left.map(|s| seconds[WORK + s].state));
}

#[test]
fn a_flue_that_stays_cold_burns_off_after_the_window_of_work() {
    // under 70 °C for all of the card's 600 s of window, counted from the first second of work; under a flame that is not steady
    let flat = |at: u32| Some(if at < WORK as u32 { 20.0 + at as f32 * 0.25 } else { 60.0 });
    let seconds = run(WORK as u32 + 700, &sound(&[]), dim_from_90, flat, None);
    let gone = seconds.iter().position(|d| d.state == State::BurningOff(Some(Why::NoFuel))).expect("a flue that stayed cold was never accused");
    assert!((WORK + 599..=WORK + 601).contains(&gone), "the fuel was given up on {} s into the work, and the window is 600 s", gone - WORK);
}

#[test]
fn a_flue_that_does_not_climb_does_not_cut_short_the_wait_for_a_lost_flame() {
    // the flue stands at 60 °C through the work, and the flame goes at 400 s: the window of the flue closes at about
    // 450 s, inside the wait - and the wait is the flame's, so the fire is lit again on the flame's 121st second
    // and silent once the flame has gone for good, so the wait is the flame's alone
    let flat = |at: u32| if at >= 400 { None } else { Some(if at < WORK as u32 { 20.0 + at as f32 * 0.25 } else { 60.0 }) };
    let seconds = run(600, &sound(&[("auto_flue_win_s", 300.0)]), dark_over(400..600), flat, None);
    let left = seconds[WORK..].iter().position(|d| d.state != State::Working).map(|s| s + WORK);
    assert_eq!(left, Some(520), "the work was left at {left:?}: {:?}", left.map(|s| seconds[s].state));
    assert_eq!(seconds[520].state, State::Kindling(1));
}

#[test]
fn a_fire_given_up_burns_out_on_the_step_s_draught_for_a_minute_and_only_then_is_blown_clear() {
    // gone at 200 s, three attempts at lighting it again from 320 s, given up on at about 1349 s: the pot is dark
    // already, and still it burns out a minute before the full blow
    let seconds = run(1500, &sound(&[]), dark_over(200..1500), silent_from(200), None);
    let off: Vec<&Doing> = seconds.iter().filter(|d| matches!(d.state, State::BurningOff(_))).collect();
    let held = off.iter().take_while(|d| d.fan == Asked::Hold(51.0)).count();
    let blown = off.iter().skip(held).take_while(|d| d.fan == Asked::Purge).count();
    assert_eq!((held, blown, off.len()), (60, 20, 80), "burning off: {held} s on the step's draught, then {blown} s at full, of {} s", off.len());
}

// a flame not back within its wait is lit again, not burnt off - three
// attempts, pouring the card's doses in turn (`auto_relight_1/2/3`) and nothing between them; over a hot flue from the
// embers with no plug, and a flame back on any of them is work again; over a cold one with the plug, and a flame back rises
// and works; either way the count starts again; not back after the last, the fire is burnt off and the fault says so.

/// The states SECONDS went through from FROM on, each once in the order they came.
fn went_through(seconds: &[Doing], from: usize) -> Vec<State> {
    let mut states: Vec<State> = seconds[from..].iter().map(|d| d.state).collect();
    states.dedup();
    states
}

/// The seconds from FROM on spent in WANTED without a break, from its first.
fn stretch(seconds: &[Doing], from: usize, wanted: State) -> &[Doing] {
    let start = from + seconds[from..].iter().position(|d| d.state == wanted).unwrap_or_else(|| panic!("never in {wanted:?}"));
    let len = seconds[start..].iter().take_while(|d| d.state == wanted).count();
    &seconds[start..start + len]
}

#[test]
fn a_flame_gone_for_good_at_work_is_lit_again_on_its_hundred_and_twentieth_second_and_the_wait_feeds_as_the_work_did() {
    // the wait is the work's: the pair runs on through it, and the first attempt pours the card's first dose
    let seconds = run(800, &sound(&[]), dark_over(200..800), silent_from(200), None);
    let steady = run(800, &sound(&[]), lit_from_90, cool_climbing, None);
    assert_eq!(seconds[319].state, State::Working, "120 seconds without a flame are still the work");
    let fed = |run: &[Doing]| run[200..320].iter().map(|d| d.auger).collect::<Vec<_>>();
    assert_eq!(fed(&seconds), fed(&steady), "the wait for the flame did not feed as the work did");
    assert_eq!(seconds[320].state, State::Kindling(1), "the 121st second without a flame");
    let again = stretch(&seconds, 320, State::Kindling(1));
    assert_eq!(again.iter().filter(|d| d.auger).count(), 20, "the attempt did not pour the card's first dose");
    assert!(again.iter().any(|d| d.igniter && matches!(d.fan, Asked::Hold(pa) if pa > 0.0)), "the plug never burnt with air");
}

#[test]
fn a_flame_gone_for_good_in_rising_is_lit_again_with_the_card_s_first_dose() {
    // the pair: rising feeds nothing, so the auger stood through the wait and the attempt pours the first dose
    let seconds = run(800, &sound(&[]), dark_over(110..800), silent_from(110), None);
    assert_eq!(seconds[230].state, State::Kindling(1), "the 121st second without a flame");
    let again = stretch(&seconds, 230, State::Kindling(1));
    assert_eq!(again.iter().filter(|d| d.auger).count(), 20, "the attempt after an auger that stood poured other than 20 s");
}

#[test]
fn a_first_dose_set_on_the_card_is_the_dose() {
    let seconds = run(800, &sound(&[("auto_relight_1", 25.0)]), dark_over(110..800), silent_from(110), None);
    let again = stretch(&seconds, 230, State::Kindling(1));
    assert_eq!(again.iter().filter(|d| d.auger).count(), 25, "the attempt did not pour the card's 25 s");
}

#[test]
fn a_hot_flue_lights_again_from_the_embers_with_no_plug() {
    // the flue at 150 °C while the fire burns and cooling 1.5 °C a minute once it has gone - hot the
    // second the flame is taken for lost, so the attempts pour their doses with no plug
    let hot = |at: u32| Some(if at < 200 { 150.0 } else { 150.0 - 1.5 * (at - 200) as f32 / 60.0 });
    let seconds = run(1200, &sound(&[]), dark_over(200..1200), hot, None);
    let lost = first(&seconds[200..], State::Kindling(1)).expect("the flame was never taken for lost") + 200;
    for n in 1..=3 {
        let again = stretch(&seconds, lost, State::Kindling(n));
        assert!(again.iter().all(|d| !d.igniter), "attempt {n} over a hot flue had the plug");
    }
}

#[test]
fn a_cool_flue_lights_again_with_the_plug_and_never_in_still_air() {
    // the pair: over a cool flue the plug burns - and only with air, as every plug does
    let seconds = run(800, &sound(&[]), dark_over(200..800), silent_from(200), None);
    let again = stretch(&seconds, 320, State::Kindling(1));
    assert!(again.iter().any(|d| d.igniter), "the plug never came on over a cool flue");
    let still = again.iter().filter(|d| d.igniter && d.fan == Asked::Stop).count();
    assert_eq!(still, 0, "the plug burned {still} s in still air over a cool flue");
}

#[test]
fn the_first_lighting_still_pours_the_whole_dose() {
    // the pair of the tenth: the dose is cut for a pot that already holds fuel, and the first lighting's pot does not
    let seconds = run(800, &sound(&[]), dark_over(200..800), silent_from(200), None);
    let first = stretch(&seconds, 0, State::Kindling(1));
    assert_eq!(first.iter().filter(|d| d.auger).count(), 32, "the first lighting poured other than its 32 s");
}

#[test]
fn three_attempts_that_do_not_bring_the_flame_back_end_in_a_fault_that_says_so() {
    let seconds = run(1700, &sound(&[]), dark_over(200..1700), silent_from(200), None);
    let why = Why::FlameNotBack(3);
    assert_eq!(
        went_through(&seconds, WORK),
        vec![State::Working, State::Kindling(1), State::Kindling(2), State::Kindling(3), State::BurningOff(Some(why)), State::Fault(why)],
    );
    // the three attempts pour the card's three doses
    let poured = seconds[320..].iter().filter(|d| d.auger).count();
    assert_eq!(poured, 40, "three attempts at lighting again poured {poured} s, and 20, 10 and 10 is 40");
    let said = automaton::words(why);
    assert!(said.contains("3 attempts") && said.contains("pot"), "the fault does not say how many attempts and where to look: {said}");
}

#[test]
fn a_flame_back_on_the_second_attempt_rises_and_works_and_the_count_starts_again() {
    // where the second attempt begins over a flame that never comes back
    let dark = run(1200, &sound(&[]), dark_over(200..1200), silent_from(200), None);
    let second = first(&dark, State::Kindling(2)).expect("no second attempt") as u32;
    let back = second + 20;
    // back 20 s into the second attempt, and gone for good once the work has held four minutes
    let gone = back + 60 + 240;
    let flame = move |at: u32| if (200..back).contains(&at) || at >= gone { Flame::Out } else { lit_from_90(at) };
    // the wait, three attempts of about 343 s each, and a burning off of 80 s
    let flue = move |at: u32| if at >= 200 && flame(at) == Flame::Out { None } else { cool_climbing(at) };
    let seconds = run(gone + 120 + 1300, &sound(&[]), flame, flue, None);
    let why = Why::FlameNotBack(3);
    assert_eq!(
        went_through(&seconds, WORK),
        vec![
            State::Working, State::Kindling(1), State::Kindling(2), State::Rising, State::Working,
            State::Kindling(1), State::Kindling(2), State::Kindling(3), State::BurningOff(Some(why)), State::Fault(why),
        ],
        "the flame back on the second attempt did not start the count again",
    );
}

#[test]
fn a_person_s_stop_in_an_attempt_at_lighting_again_burns_off_and_the_next_start_pours_the_whole_dose() {
    // the count belongs to one lost flame: a stop ends it, and the next lighting is a first one
    let flame = |at: u32| if at < 200 { lit_from_90(at) } else if at >= 1100 { Flame::Steady } else { Flame::Out };
    let t = sound(&[]);
    let mut a = Automaton::new(Burning::new());
    let seconds: Vec<Doing> = (0..1200u32)
        .map(|at| {
            let flue = if at >= 200 && flame(at) == Flame::Out { None } else { cool_climbing(at) };
            let seen = Seen { flame: flame(at), boiler_c: Some(40.0), return_c: None, flue_c: flue, draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD };
            match at {
                2 | 700 => a.press(Press::Start, seen, &t),
                340 => a.press(Press::Stop, seen, &t),
                _ => a.second(seen, 1.0, &t),
            }
        })
        .collect();
    assert_eq!(seconds[339].state, State::Kindling(1), "not in an attempt at lighting again when the stop came");
    assert_eq!(seconds[340].state, State::BurningOff(None));
    let next = stretch(&seconds, 700, State::Kindling(1));
    assert_eq!(next.iter().filter(|d| d.auger).count(), 32, "the lighting after a stop poured a relighting's tenth");
}

#[test]
fn an_attempt_that_pours_nothing_shows_its_dose_done_from_its_first_second() {
    // the page reads the clocks: a dose of no seconds is no dose, and a clock that read "1 s" promised one. The attempt
    // that pours nothing is the plug after the embers: the flue hot, cooling once the flame went
    let t = sound(&[]);
    let mut a = Automaton::new(Burning::new());
    let mut read = None;
    for at in 0..1500u32 {
        let flue = if at < 200 { 150.0 } else { 150.0 - 1.5 * (at - 200) as f32 / 60.0 };
        let seen = Seen { flame: dark_over(200..1500)(at), boiler_c: Some(40.0), return_c: None, flue_c: Some(flue), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD };
        let did = if at == 2 { a.press(Press::Start, seen, &t) } else { a.second(seen, 1.0, &t) };
        if did.state == State::Kindling(4) {
            read = a.clocks(&t).iter().find(|c| c.what == "dose ends in").and_then(|c| c.left_s);
            break;
        }
    }
    assert_eq!(read, Some(0.0), "the first second of an attempt that pours nothing");
}

#[test]
fn an_attempt_that_pours_shows_its_dose_counting_down() {
    // the pair: after a rising, the dose up to the plug is counted
    let t = sound(&[]);
    let mut a = Automaton::new(Burning::new());
    let mut read = None;
    for at in 0..400u32 {
        let seen = Seen { flame: dark_over(110..400)(at), boiler_c: Some(40.0), return_c: None, flue_c: silent_from(110)(at), draught_pa: Some(45.0), draught_low: false, gate: 30, silence: Silence::HEARD };
        let did = if at == 2 { a.press(Press::Start, seen, &t) } else { a.second(seen, 1.0, &t) };
        if did.state == State::Kindling(1) && at > 200 {
            read = a.clocks(&t).iter().find(|c| c.what == "dose ends in").and_then(|c| c.left_s);
            break;
        }
    }
    assert!(read.is_some_and(|left| left > 0.0), "the dose up to the plug read {read:?} in its first second");
}

// ---- the light alone decides that the flame is lost, through the card ----

#[test]
fn a_dark_sensor_over_a_flue_that_holds_is_a_lost_flame_lit_again_from_the_embers() {
    // the flue climbs through the fire and holds at 76 °C once the sensor goes dark: it does not stretch the wait, and it
    // says the pot is hot - the first attempt is from the embers, with no plug
    let holds = |at: u32| Some(if at < 200 { 20.0 + 0.28 * at as f32 } else { 76.0 });
    let seconds = run(700, &sound(&[]), dark_over(200..700), holds, None);
    assert_eq!(seconds[320].state, State::Kindling(1), "the 121st second without a flame over a flue that holds");
    let again = stretch(&seconds, 320, State::Kindling(1));
    assert!(again.iter().all(|d| !d.igniter), "the attempt over a flue of 76 °C had the plug");
}
