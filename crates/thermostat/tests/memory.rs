//! What the thermostat writes down of the switch and what it reads back, and no board anywhere: the last press of
//! Start or Stop survives a restart, a board nobody pressed anything on reads back as one nobody asked, and the flash is written
//! once for a press and never for a second.
//!
//! **THE EXPECTATIONS ARE THE BOARD'S, NOT THE CODE'S**: what a person means by "Start survives" - the next boot is `On` and sends
//! no press of its own, because a press lights at once whatever the water says and a restart is not the person.

use automaton::{Heard, Presses, Switch};
use thermostat::{kept_switch, switch_kept, Fire, Hold, Lockout, Press, Settled, Temperatures, Thermostat, Tuning, SWITCH_CARD};

fn step() -> automaton::Step {
    automaton::Step { pa: 120.0, feed: automaton::Feed { on_s: 4.6, off_s: 15.0 } }
}

fn sound() -> thermostat::Sound {
    Tuning { hold: Hold::Boiler, want_c: 60.0, hysteresis_c: 4.0, step: step(), mid: step(), small: step(), mid_c: 8.0, small_c: 3.0, power: step(), lockout: Lockout::default() }
        .sound(88.0)
        .expect("the numbers are sound")
}

fn at(boiler_c: f32) -> Temperatures {
    Temperatures { boiler_c: Some(boiler_c), return_c: Some(30.0), return_heard: Heard::Speaks, asked: None, room_c: None, room_target_c: None, fire: Fire::default() }
}

// ---------------------------------------------------------------- the words

#[test]
fn a_press_of_start_or_stop_is_kept_as_its_word_and_read_back_as_the_same_switch() {
    assert_eq!(switch_kept(Switch::On), Some("on"));
    assert_eq!(switch_kept(Switch::Off), Some("off"));
    assert_eq!(kept_switch(Some("on")), Switch::On);
    assert_eq!(kept_switch(Some("off")), Switch::Off);
}

#[test]
fn a_board_nobody_pressed_anything_on_keeps_nothing_and_reads_back_unasked() {
    assert_eq!(switch_kept(Switch::Unasked), None);
    assert_eq!(kept_switch(None), Switch::Unasked);
}

#[test]
fn a_word_nobody_wrote_is_not_a_press_it_is_unasked_the_side_where_nothing_lights_itself() {
    for torn in ["", "o", "onn", "ON", "unasked", "on ", "start", "off:1"] {
        assert_eq!(kept_switch(Some(torn)), Switch::Unasked, "{torn:?}");
    }
}

#[test]
fn the_key_the_switch_is_kept_under_fits_the_fifteen_bytes_nvs_takes() {
    assert!(format!("m.{SWITCH_CARD}").len() <= 15, "settings::memory_key makes m.<card>");
}

// ---------------------------------------------------------------- coming back

#[test]
fn a_board_that_found_on_in_its_memory_comes_up_on_and_sends_no_press() {
    let mut heat = Thermostat::restored(Switch::On);
    assert_eq!(heat.switch(), Switch::On);
    let order = heat.order(&at(55.0), &sound());
    assert_eq!(order.switch, Switch::On);
    assert_eq!(order.presses, Presses::default(), "a restart is not a person: a press would light whatever the water says");
}

#[test]
fn a_board_that_found_off_in_its_memory_comes_up_off_and_sends_no_press() {
    let mut heat = Thermostat::restored(Switch::Off);
    assert_eq!(heat.switch(), Switch::Off);
    assert_eq!(heat.order(&at(55.0), &sound()).presses, Presses::default(), "and no Stop either: nothing is burnt off by a restart");
}

#[test]
fn a_board_that_found_nothing_comes_up_unasked() {
    let heat = Thermostat::restored(Switch::Unasked);
    assert_eq!(heat.switch(), Switch::Unasked);
    assert!(!heat.from_memory(), "there is nothing the switch stands on but the board's coming up");
}

#[test]
fn a_switch_that_stands_as_the_memory_gave_it_says_so_until_a_person_presses() {
    let mut heat = Thermostat::restored(Switch::On);
    assert!(heat.from_memory());
    heat.press(Press::Stop);
    assert!(!heat.from_memory(), "a person's press is the switch now");
    let mut heat = Thermostat::restored(Switch::Off);
    heat.press(Press::Start);
    assert!(!heat.from_memory());
}

#[test]
fn a_board_nobody_pressed_anything_on_does_not_say_its_switch_is_from_memory() {
    assert!(!Thermostat::new().from_memory());
}

// ---------------------------------------------------------------- the writes

/// The flash as the board has it: the word it holds, and how many of the next writes it refuses (an nvs that is full, busy or failing). ATTEMPTS counts every try, taken or refused.
#[derive(Default)]
struct Flash {
    held: Option<String>,
    refuse: u32,
    attempts: u32,
}

impl Flash {
    fn put(&mut self, word: &str) -> Result<(), String> {
        self.attempts += 1;
        if self.refuse > 0 {
            self.refuse -= 1;
            return Err("nvs: no room".into());
        }
        self.held = Some(word.to_string());
        Ok(())
    }
}

/// A press written down at once, as the board does it.
fn pressed(heat: &mut Thermostat, flash: &mut Flash, press: Press) -> Settled {
    heat.press(press);
    heat.write_now(&mut |word| flash.put(word))
}

/// The board comes up again on what the flash holds.
fn restart(flash: &Flash) -> Thermostat {
    Thermostat::restored(kept_switch(flash.held.as_deref()))
}

#[test]
fn a_press_is_one_write_of_its_word() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash::default());
    assert_eq!(heat.write_now(&mut |word| flash.put(word)), Settled::Nothing, "nothing is pressed, nothing is written");
    assert_eq!(pressed(&mut heat, &mut flash, Press::Start), Settled::Written { after_refusals: 0 });
    assert_eq!(flash.held.as_deref(), Some("on"));
    assert_eq!(heat.retry(1.0, &mut |word| flash.put(word)), Settled::Nothing, "the word is written once");
    pressed(&mut heat, &mut flash, Press::Stop);
    assert_eq!(flash.held.as_deref(), Some("off"));
    assert_eq!(flash.attempts, 2);
}

#[test]
fn seconds_write_nothing_whatever_the_water_does() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash::default());
    pressed(&mut heat, &mut flash, Press::Start);
    for second in 0..3600 {
        heat.order(&at(40.0 + (second % 30) as f32), &sound());
        assert_eq!(heat.retry(1.0, &mut |word| flash.put(word)), Settled::Nothing, "second {second}");
    }
    assert_eq!(flash.attempts, 1, "one press, one try");
}

#[test]
fn a_press_that_changes_nothing_in_the_memory_is_not_written_again() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash::default());
    pressed(&mut heat, &mut flash, Press::Start);
    assert_eq!(pressed(&mut heat, &mut flash, Press::Start), Settled::Nothing, "the flash has `on` already, and a Start in the same breath is the same word");
    assert_eq!(flash.attempts, 1);
}

#[test]
fn a_press_that_agrees_with_what_the_board_came_up_with_is_not_written() {
    let (mut heat, mut flash) = (Thermostat::restored(Switch::On), Flash { held: Some("on".into()), ..Flash::default() });
    assert_eq!(heat.write_now(&mut |word| flash.put(word)), Settled::Nothing, "what was found is what the flash holds");
    assert_eq!(pressed(&mut heat, &mut flash, Press::Start), Settled::Nothing);
    assert_eq!(pressed(&mut heat, &mut flash, Press::Stop), Settled::Written { after_refusals: 0 });
    assert_eq!(flash.held.as_deref(), Some("off"));
}

// ---------------------------------------------------------------- a flash that refuses (6.16, the review)

#[test]
fn a_stop_whose_write_was_refused_is_written_on_a_later_second_and_the_restart_finds_the_stop() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash::default());
    pressed(&mut heat, &mut flash, Press::Start);
    assert_eq!(flash.held.as_deref(), Some("on"), "the Start is saved");
    flash.refuse = 1;
    assert!(matches!(pressed(&mut heat, &mut flash, Press::Stop), Settled::Refused { .. }), "the flash refuses the Stop");
    assert_eq!(flash.held.as_deref(), Some("on"), "and still holds the Start");
    assert_eq!(heat.retry(1.0, &mut |word| flash.put(word)), Settled::Written { after_refusals: 1 }, "the next second tries again, and it goes through");
    assert_eq!(restart(&flash).switch(), Switch::Off, "a person pressed Stop last: the restart finds the Stop and not the Start before it");
}

#[test]
fn a_refused_word_is_not_counted_written_until_the_flash_takes_it() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash { refuse: 2, ..Flash::default() });
    assert!(matches!(pressed(&mut heat, &mut flash, Press::Start), Settled::Refused { .. }));
    assert!(matches!(heat.retry(1.0, &mut |word| flash.put(word)), Settled::Refused { .. }), "still refused: still pending");
    assert_eq!(heat.retry(1.0, &mut |word| flash.put(word)), Settled::Written { after_refusals: 2 });
    assert_eq!(flash.held.as_deref(), Some("on"));
    assert_eq!(heat.retry(1.0, &mut |word| flash.put(word)), Settled::Nothing, "and written is written");
    assert_eq!(flash.attempts, 3);
}

#[test]
fn a_retry_waits_its_second_and_does_not_try_more_than_once_a_second() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash { refuse: u32::MAX, ..Flash::default() });
    pressed(&mut heat, &mut flash, Press::Start);
    for _ in 0..100 {
        heat.retry(0.1, &mut |word| flash.put(word));
    }
    // ten seconds of tenths: the press itself and a try for each full second after it
    assert!(flash.attempts <= 11, "{} tries in ten seconds", flash.attempts);
    assert!(flash.attempts >= 10, "a refused word is tried again, and not forgotten: {} tries in ten seconds", flash.attempts);
}

#[test]
fn a_series_of_refusals_is_told_once_and_the_recovery_once() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash { refuse: 6, ..Flash::default() });
    let mut said = vec![pressed(&mut heat, &mut flash, Press::Start)];
    for _ in 0..8 {
        said.push(heat.retry(1.0, &mut |word| flash.put(word)));
    }
    let firsts = said.iter().filter(|s| matches!(s, Settled::Refused { first: true, .. })).count();
    let laters = said.iter().filter(|s| matches!(s, Settled::Refused { first: false, .. })).count();
    let recovered = said.iter().filter(|s| matches!(s, Settled::Written { after_refusals } if *after_refusals > 0)).count();
    assert_eq!((firsts, laters, recovered), (1, 5, 1), "six refusals are one series: the first says it, the rest are silent, the recovery says how many");
    // and the next series is a series of its own
    flash.refuse = 1;
    assert!(matches!(pressed(&mut heat, &mut flash, Press::Stop), Settled::Refused { first: true, .. }));
}

#[test]
fn a_press_while_a_write_is_refused_replaces_the_word_waiting_and_the_old_one_is_never_written() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash::default());
    pressed(&mut heat, &mut flash, Press::Stop);
    flash.refuse = 1;
    assert!(matches!(pressed(&mut heat, &mut flash, Press::Start), Settled::Refused { .. }));
    // the person changes their mind before the flash has taken the Start
    assert_eq!(pressed(&mut heat, &mut flash, Press::Stop), Settled::Nothing, "the flash holds `off` and the switch is `off` again: nothing waits");
    assert_eq!(flash.held.as_deref(), Some("off"));
    assert_eq!(heat.retry(1.0, &mut |word| flash.put(word)), Settled::Nothing);
    assert_eq!(restart(&flash).switch(), Switch::Off);
}

#[test]
fn a_board_that_never_got_a_word_into_the_flash_comes_up_asking_nothing() {
    let (mut heat, mut flash) = (Thermostat::new(), Flash { refuse: u32::MAX, ..Flash::default() });
    pressed(&mut heat, &mut flash, Press::Start);
    assert_eq!(restart(&flash).switch(), Switch::Unasked, "nothing was kept: the side where nothing lights itself");
}
