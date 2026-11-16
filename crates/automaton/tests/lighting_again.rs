//! Lighting again without the plug, and the plug only with air: the rows of
//! each requirement, each with its pair.
//!
//! **THE PLUG BURNS OUT IN STILL AIR** (26.09: three attempts of ten seconds of heat in still air, and a plug
//! switched on by hand with the fan standing, burnt the plug through). **A DARK SENSOR OVER A FLUE THAT HOLDS IS A
//! DIM FIRE, NOT A LOST ONE** (26.09 17:30: the pin read 2454-2614 over a threshold of 2400 while the flue held
//! 76-79 °C). **A LOST FLAME IN A HOT POT IS LIT FROM ITS EMBERS**: a little fuel, then the igniter, with the air
//! only ten seconds later, is the wrong order.

mod common;

use automaton::{Asked, Doing, Feed, Lighting, State, Step, Told};
use common::{Hold, Burning, Seen, Silence, Tuning};
use flame::Flame;

fn tuning() -> Tuning {
    Tuning {
        purge_before_s: 30.0,
        purge_after_s: 20.0,
        igniter_s: 180.0,
        wait_for_flame_s: 90.0,
        tries: 3,
        dose_after_first: 0.1,
        rise_s: 60.0,
        flame_lost_s: 20.0,
        smoulder_s: 300.0,
        burn_off_s: 600.0,
        burn_out_min_s: 60.0,
        pump_on_c: 35.0,
        // under the flue of these rows' fire (76 °C), so the sign that the fuel stopped is not what they meet: it is
        // judged by rows of its own, and 6.5 does not touch it
        flue_holds_c: 70.0,
        fire_numbers: common::FireNumbers::CARD,
        want_c: 60.0,
        hysteresis_c: 4.0,
        hold: Hold::Boiler,
        power: Step { pa: 51.0, feed: Feed { on_s: 32.0, off_s: 15.0 } },
        pump_off_below_c: 4.0,
        too_hot_c: 88.0,
        alarm_c: 95.0,
        sure_s: 10.0,
        relight_s: [20.0, 10.0, 10.0],
        relight_hot_c: 45.0,
        spin_up_s: 10.0,
        low_pressure_s: 20.0,
        kindling: Lighting { pa: 45.0, dose_s: 32.0 },
        step: Step { pa: 42.0, feed: Feed { on_s: 2.0, off_s: 42.0 } },
        smoulder: Step { pa: 0.0, feed: Feed { on_s: 1.0, off_s: 50.0 } },
    }
}

fn sound() -> common::Sound {
    tuning().sound().expect("the numbers of 26.09 are sound")
}

/// A cold burner, no flame, and the fan making whatever DRAUGHT says.
fn dark(draught_pa: Option<f32>) -> Seen {
    // the gate stands open while the fan is asked to blow, as the board's regulator sets it
    Seen { flame: Flame::Out, boiler_c: Some(30.0), return_c: None, flue_c: Some(20.0), draught_pa, draught_low: false, gate: if draught_pa.is_some_and(|pa| pa >= 5.0) { 30 } else { 0 }, silence: Silence::HEARD }
}

/// A second of the burner and what it decided.
fn second(burning: &mut Burning, seen: Seen) -> Doing {
    burning.step(Told::Nothing, seen, 1.0, &sound())
}

/// A burner started and purged: the first second of its first attempt at lighting.
fn lighting() -> Burning {
    let mut burning = Burning::new();
    burning.step(Told::Start, dark(Some(0.0)), 1.0, &sound());
    for _ in 0..30 {
        second(&mut burning, dark(Some(200.0)));
    }
    assert_eq!(burning.state(), State::Kindling(1), "the scaffolding did not reach the first attempt");
    burning
}

// ---------------------------------------------------------------- 1. the plug only with air

#[test]
fn the_plug_never_burns_while_the_fan_makes_no_air() {
    // the whole first attempt with the fan standing: the dose, the still air after it, the air that never comes
    let mut burning = lighting();
    for s in 0..400 {
        let did = second(&mut burning, dark(Some(0.0)));
        assert!(!did.igniter, "the plug burnt with 0 Pa at second {s} of the attempt ({:?})", did.state);
    }
}

#[test]
fn the_plug_burns_once_the_fan_makes_air() {
    // the pair: the dose, then the fan asked, and a draught that comes - the plug burns
    let mut burning = lighting();
    let mut burnt = None;
    for s in 0..80 {
        let did = second(&mut burning, dark(Some(if s < 32 { 0.0 } else { 45.0 })));
        if did.igniter && burnt.is_none() {
            burnt = Some(s);
        }
        if did.fan == Asked::Stop {
            assert!(!did.igniter, "the plug burnt at second {s} while the fan was asked to stand");
        }
    }
    assert!(burnt.is_some_and(|s| s <= 34), "the plug did not burn with 45 Pa after the dose (first at {burnt:?})");
}

#[test]
fn the_plug_goes_off_the_second_the_air_goes() {
    // burning with air, and then the draught sensor gives nothing - a stall, a tube off, a sensor gone
    for (what, draught) in [("no number", None), ("a stall, 2 Pa", Some(2.0))] {
        let mut burning = lighting();
        for s in 0..40 {
            second(&mut burning, dark(Some(if s < 32 { 0.0 } else { 45.0 })));
        }
        assert!(second(&mut burning, dark(Some(45.0))).igniter, "the plug was not burning with air");
        let did = second(&mut burning, dark(draught));
        assert!(!did.igniter, "the plug stayed on with {what}");
    }
}

#[test]
fn a_person_is_refused_the_plug_while_the_fan_makes_no_air_and_told_what_to_switch() {
    for draught in [None, Some(0.0), Some(3.0)] {
        let refused = automaton::plug_by_hand(true, 45, draught).expect_err("the plug was given to a hand with no air");
        assert!(refused.contains("Draught") && refused.contains("Burner"),
                "the refusal does not name the key and the tab: {refused}");
    }
}

#[test]
fn a_person_gets_the_plug_with_air_and_may_always_switch_it_off() {
    // the pair: with air the plug is the hand's, and switching it off is never refused
    assert_eq!(automaton::plug_by_hand(true, 45, Some(45.0)), Ok(()));
    for draught in [None, Some(0.0), Some(45.0)] {
        assert_eq!(automaton::plug_by_hand(false, 0, draught), Ok(()), "switching the plug off was refused at {draught:?}");
    }
}

#[test]
fn a_floating_draught_pin_over_a_closed_gate_gives_the_automaton_no_plug_either() {
    // review 6.5 R1, F1: the same floating pin under the automaton's own attempt - 171 Pa read, the gate at 0 % because
    // the fan never took the angle. The plug is not lit on a draught nobody is blowing.
    let mut burning = lighting();
    let floating = Seen { draught_pa: Some(171.0), draught_low: false, gate: 0, ..dark(Some(0.0)) };
    for s in 0..200 {
        let did = second(&mut burning, floating);
        assert!(!did.igniter, "the plug burnt over a closed gate at second {s} of the attempt ({:?})", did.state);
    }
}

#[test]
fn an_open_gate_and_a_draught_give_the_automaton_its_plug() {
    // the pair: the gate open and the draught there - the plug burns after the dose
    let mut burning = lighting();
    let blowing = Seen { draught_pa: Some(45.0), draught_low: false, gate: 30, ..dark(Some(0.0)) };
    let burnt = (0..60).any(|_| second(&mut burning, blowing).igniter);
    assert!(burnt, "the plug did not burn over an open gate and 45 Pa");
}

#[test]
fn a_sensor_that_reads_a_draught_over_a_fan_asked_to_stand_is_no_air() {
    // the bench board .30, 27.09: its draught pin floats and reads 171 Pa with the gate at 0 % - and the plug was given
    // to a hand. A tube off, a drift, a chimney pulling on its own read the same; air is a fan asked to blow AND a
    // draught that says it does.
    let refused = automaton::plug_by_hand(true, 0, Some(171.0)).expect_err("a plug was given over a gate at 0 %");
    assert!(refused.contains("Draught") && refused.contains("Burner"), "the refusal does not name the key and the tab: {refused}");
    assert!(!automaton::plug_stays(true, 0, Some(171.0)), "a hand's plug stayed on over a gate at 0 %");
}

#[test]
fn a_plug_a_person_switched_on_goes_off_when_the_air_goes() {
    assert!(!automaton::plug_stays(true, 45, None), "a hand's plug stayed on with no draught");
    assert!(!automaton::plug_stays(true, 45, Some(2.0)), "a hand's plug stayed on with 2 Pa");
    // the pair: with air it stays, and an open plug is never closed by this
    assert!(automaton::plug_stays(true, 45, Some(45.0)), "a hand's plug was taken with 45 Pa");
    assert!(!automaton::plug_stays(false, 45, Some(45.0)), "an open plug was closed");
}

// ---------------------------------------------------------------- 2. a blink, and a lost flame lit from its embers

/// The fire these rows carry: a steady flame, the flue at FLUE, the fan on the step's draught.
fn lit(flue: f32) -> Seen {
    Seen { flame: Flame::Steady, boiler_c: Some(40.0), return_c: None, flue_c: Some(flue), draught_pa: Some(42.0), draught_low: false, gate: 30, silence: Silence::HEARD }
}

/// The same fire with the sensor dark.
fn out(flue: f32) -> Seen {
    Seen { flame: Flame::Out, ..lit(flue) }
}

/// A burner lit and carried into work over a flue at FLUE, then SECONDS more of a steady fire.
fn at_work(flue: f32, seconds: usize) -> Burning {
    let mut burning = lighting();
    for _ in 0..110 {
        second(&mut burning, lit(flue));
    }
    assert_eq!(burning.state(), State::Working, "the scaffolding did not reach work");
    for _ in 0..seconds {
        second(&mut burning, lit(flue));
    }
    burning
}

/// A fire at work over a flue that HOLDS at FLUE, the sensor dark from the first of SECONDS on: what the burner did.
fn dark_at_work(flue: f32, seconds: usize) -> Vec<Doing> {
    let mut burning = at_work(flue, 150);
    (0..seconds).map(|_| second(&mut burning, out(flue))).collect()
}

/// Every attempt at lighting again in DID, in order: its number, the seconds of auger it poured, whether the plug burnt.
fn attempts(did: &[Doing]) -> Vec<(u32, usize, bool)> {
    let mut out: Vec<(u32, usize, bool)> = Vec::new();
    for d in did {
        if let State::Kindling(n) = d.state {
            if out.last().map(|a| a.0) != Some(n) {
                out.push((n, 0, false));
            }
            let a = out.last_mut().unwrap();
            a.1 += d.auger as usize;
            a.2 |= d.igniter;
        }
    }
    out
}

#[test]
fn a_flame_dark_fifteen_seconds_at_work_changes_nothing_the_same_feed_and_the_same_air() {
    // owner, 27.09: dark up to 20 s is a blink - the same feed, the same air. The same fire twice, one of them with its
    // sensor dark 15 s: every second the organs are told the same.
    // the blink covers a feed of the steady fire, or the row proves nothing
    let mut probe = at_work(76.0, 150);
    let feeds = (0..120).position(|s| second(&mut probe, lit(76.0)).auger && s >= 10).expect("no feed in two minutes of work");
    let blink = feeds - 5..feeds + 10;
    let (mut steady, mut blinking) = (at_work(76.0, 150), at_work(76.0, 150));
    for s in 0..120 {
        let seen = if blink.contains(&s) { out(76.0) } else { lit(76.0) };
        let (a, b) = (second(&mut steady, lit(76.0)), second(&mut blinking, seen));
        assert_eq!(b.state, State::Working, "a blink of 15 s left the work at second {s}");
        assert_eq!((b.auger, b.fan), (a.auger, a.fan), "second {s} of a blink of 15 s: the feed or the air changed");
    }
}

#[test]
fn a_flame_dark_twenty_five_seconds_is_lost_and_lit_from_the_embers_with_a_first_portion_of_twenty_seconds() {
    // the flue holds 76 °C the whole time - it does not hold the verdict back, it only says the pot is hot
    let did = dark_at_work(76.0, 25);
    let lost = did.iter().position(|d| matches!(d.state, State::Kindling(_)));
    assert!(lost.is_some_and(|s| s < 25), "a flame dark 25 s over a flue of 76 °C was not taken for lost: {lost:?}");
    let did = dark_at_work(76.0, 25 + 60);
    let first = attempts(&did);
    assert_eq!(first.first().copied(), Some((1, 20, false)), "the first attempt: {first:?}");
}

#[test]
fn between_the_portions_the_auger_stands_and_the_way_pours_no_more_than_its_portions() {
    // lost past 20 s, three portions from the embers, the plug once with nothing poured, then the fire burnt off: after the
    // flame is taken for lost the auger turns only inside a portion, and 20 + 10 + 10 s is all it turns
    let did = dark_at_work(76.0, 20 + 3 * 200 + 600);
    let lost = did.iter().position(|d| matches!(d.state, State::Kindling(_))).expect("the flame was never taken for lost");
    let after = &did[lost..];
    let tried = attempts(after);
    assert_eq!(tried, [(1, 20, false), (2, 10, false), (3, 10, false), (4, 0, true)], "the attempts after the embers");
    let outside = after.iter().filter(|d| d.auger && !matches!(d.state, State::Kindling(_))).count();
    assert_eq!(outside, 0, "the auger turned {outside} s between the portions");
    let poured = after.iter().filter(|d| d.auger).count();
    assert_eq!(poured, 40, "the way from a lost flame poured {poured} s of auger");
    assert!(after.iter().any(|d| matches!(d.state, State::BurningOff(Some(automaton::Why::FlameNotBack(_))))),
            "the fire that did not come back was not burnt off");
}

#[test]
fn a_flame_back_in_the_wait_after_a_portion_is_work_again() {
    // the pair: the light back in the wait of the first attempt from the embers, and in its portion - the fire is worked
    for (when, what) in [(20 + 40, "the wait"), (20 + 5, "the portion")] {
        let mut burning = at_work(76.0, 150);
        for _ in 0..when {
            second(&mut burning, out(76.0));
        }
        assert_eq!(burning.state(), State::Kindling(1), "the scaffolding did not reach the first attempt");
        // the flame is work again when it has held five seconds, and not before
        for held in 1..=4 {
            assert_eq!(second(&mut burning, lit(76.0)).state, State::Kindling(1), "second {held} of a flame back in {what} was taken for a fire");
        }
        let did = second(&mut burning, lit(76.0));
        assert_eq!(did.state, State::Working, "a flame back in {what} was not work again");
    }
}

#[test]
fn a_flame_lost_over_a_cold_flue_is_lit_with_the_plug_and_the_plug_burns_with_air() {
    // the pair of the embers: a flue under relight_hot_c in the second the flame is taken for lost - a cold pot, the plug
    // at every attempt, and burning only with the fan blowing
    let did = dark_at_work(40.0, 20 + 3 * 300);
    let tried = attempts(&did);
    assert_eq!(tried.iter().map(|a| (a.0, a.1)).collect::<Vec<_>>(), [(1, 20), (2, 10), (3, 10)], "{tried:?}");
    assert!(tried.iter().all(|a| a.2), "an attempt over a cold flue had no plug: {tried:?}");
    for d in &did {
        assert!(!(d.igniter && d.fan == Asked::Stop), "the plug burnt with the fan asked to stand ({:?})", d.state);
    }
}

#[test]
fn a_flue_that_gives_no_number_is_a_cold_pot() {
    // an open thermocouple in the second the flame is taken for lost says nothing about embers: the plug
    let mut burning = at_work(76.0, 150);
    let did: Vec<Doing> = (0..20 + 300).map(|_| second(&mut burning, Seen { flue_c: None, ..out(76.0) })).collect();
    let tried = attempts(&did);
    assert!(tried.first().is_some_and(|a| a.2), "a lost flame over a flue with no number had no plug: {tried:?}");
}

#[test]
fn a_floating_draught_pin_gives_a_cold_pot_no_plug() {
    // a cold pot lit again over the bench board's floating pin: 171 Pa read, the gate at 0 % - no plug on the whole way
    let mut burning = at_work(40.0, 150);
    for s in 0..20 + 600 {
        let did = second(&mut burning, Seen { draught_pa: Some(171.0), draught_low: false, gate: 0, ..out(40.0) });
        assert!(!did.igniter, "the plug burnt over a closed gate at second {s} of the dark ({:?})", did.state);
    }
}

#[test]
fn a_flame_lost_in_rising_is_lit_from_the_embers_the_same_way() {
    // rising feeds nothing, and the flame is given the same 20 s
    let mut burning = lighting();
    for _ in 0..5 {
        second(&mut burning, lit(76.0));
    }
    assert_eq!(burning.state(), State::Rising, "the scaffolding did not reach rising");
    let did: Vec<Doing> = (0..20 + 3 * 130).map(|_| second(&mut burning, out(76.0))).collect();
    assert_eq!(attempts(&did)[..3], [(1, 20, false), (2, 10, false), (3, 10, false)], "the attempts over a hot flue");
}

#[test]
fn a_flame_dark_twenty_seconds_is_still_a_blink_and_the_twenty_first_is_lost() {
    // owner, 27.09: dark up to 20 s is a blink, dark longer is lost - the boundary itself is a blink
    let did = dark_at_work(76.0, 20);
    assert!(did.iter().all(|d| d.state == State::Working), "20 s of dark left the work: {:?}", did.last().map(|d| d.state));
    let did = dark_at_work(76.0, 21);
    assert_eq!(did.last().map(|d| d.state), Some(State::Kindling(1)), "the 21st second of dark was not a lost flame");
}

// ---------------------------------------------------------------- 3. the same rule in smouldering

/// A fire carried into work over a flue at FLUE and then into smouldering by warm water: the boiler at 65 °C, over
/// want_c (60) and above the line that sends it back to work (56).
fn smouldering(flue: f32) -> Burning {
    let mut burning = at_work(flue, 150);
    let warm = Seen { boiler_c: Some(65.0), ..lit(flue) };
    second(&mut burning, warm);
    assert_eq!(burning.state(), State::Smouldering, "the scaffolding did not reach smouldering");
    burning
}

#[test]
fn a_flame_dark_past_its_wait_in_smouldering_with_the_water_asking_for_burning_is_lost_and_lit_from_the_embers() {
    // smouldering feeds on its own pair, and a dark pot fed until smoulder_s ran out: the flame's wait holds here too; the water fell under the line (56) while the flame was dark, so heat is asked and the lost flame is lit again
    let mut burning = smouldering(76.0);
    let dark = Seen { boiler_c: Some(50.0), ..out(76.0) };
    let did: Vec<Doing> = (0..21 + 60).map(|_| second(&mut burning, dark)).collect();
    // the water burns from the first dark second, so smouldering gives way to work at once and the wait runs on in work: the light follows within a second of the wait
    assert!(did[..23].iter().any(|d| d.state == State::Kindling(1)), "no light by the 23rd second of dark: {:?}", did[22].state);
    assert!(did.iter().all(|d| !matches!(d.state, State::BurningOff(_))), "heat was asked and the lost flame was burnt off");
    assert_eq!(attempts(&did).first().map(|a| (a.0, a.2)), Some((1, false)), "the first attempt from smouldering");
}

#[test]
fn a_flame_dark_past_its_wait_in_smouldering_with_the_water_over_its_line_goes_to_burning_off_and_is_not_lit_again() {
    // the pair: the water stays at 65, over the setpoint (60) - nobody asks for heat, and the fire that went out is not blown up but burnt off, with no fault
    let mut burning = smouldering(76.0);
    let dark = Seen { boiler_c: Some(65.0), ..out(76.0) };
    let did: Vec<Doing> = (0..21 + 60).map(|_| second(&mut burning, dark)).collect();
    assert_eq!(did[20].state, State::BurningOff(None), "the 21st second of dark in smouldering: {:?}", did[20].state);
    assert!(attempts(&did).is_empty(), "a fire nobody asked for was lit again: {:?}", attempts(&did));
}

#[test]
fn a_flame_dark_fifteen_seconds_in_smouldering_changes_nothing() {
    // the pair: a blink in smouldering - the same feed on its pair, the same air, the same state
    let warm = |flame| Seen { flame, boiler_c: Some(65.0), ..lit(76.0) };
    let mut probe = smouldering(76.0);
    let feeds = (0..120).position(|s| second(&mut probe, warm(Flame::Steady)).auger && s >= 10).expect("no feed in two minutes of smouldering");
    let blink = feeds - 5..feeds + 10;
    let (mut steady, mut blinking) = (smouldering(76.0), smouldering(76.0));
    for s in 0..120 {
        let seen = warm(if blink.contains(&s) { Flame::Out } else { Flame::Steady });
        let (a, b) = (second(&mut steady, warm(Flame::Steady)), second(&mut blinking, seen));
        assert_eq!(b.state, State::Smouldering, "a blink of 15 s left smouldering at second {s}");
        assert_eq!((b.auger, b.fan), (a.auger, a.fan), "second {s} of a blink of 15 s in smouldering: the feed or the air changed");
    }
}
