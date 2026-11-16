use switch::*;

#[test]
fn a_press_of_a_pushbutton_is_one_toggle() {
    let s = Switch::new(Mode::Pushbutton);
    s.start(false);
    assert_eq!(s.settled(true), Movement::Toggle);
    assert!(s.closed());
    assert_eq!((s.closed(), s.toggles()), (true, 1));
}

#[test]
fn the_release_of_a_pushbutton_moves_and_toggles_nothing() {
    let s = Switch::new(Mode::Pushbutton);
    s.start(false);
    s.settled(true);
    assert_eq!(s.settled(false), Movement::Moved);
    assert_eq!((s.closed(), s.toggles()), (false, 1));
}

#[test]
fn a_bounce_that_came_back_is_no_movement() {
    let s = Switch::new(Mode::Pushbutton);
    s.start(false);
    assert_eq!(s.settled(false), Movement::None);
    s.settled(true);
    assert_eq!(s.settled(true), Movement::None);
    assert_eq!((s.closed(), s.toggles()), (true, 1));
}

#[test]
fn a_key_held_at_start_is_no_press() {
    let s = Switch::new(Mode::Pushbutton);
    s.start(true);
    assert_eq!(s.settled(true), Movement::None);
    assert_eq!(s.settled(false), Movement::Moved);
    assert_eq!((s.closed(), s.toggles()), (false, 0));
}

#[test]
fn a_toggle_switch_toggles_on_both_movements() {
    let s = Switch::new(Mode::Toggle);
    s.start(false);
    assert_eq!(s.settled(true), Movement::Toggle);
    assert_eq!(s.settled(false), Movement::Toggle);
    assert_eq!((s.closed(), s.toggles()), (false, 2));
}
