//! What the house says to the boiler, on rows of a room's readings and the board's clock, and no board anywhere.
//!
//! **THE EXPECTATIONS COME FROM THE REQUIREMENTS, NOT FROM THE CODE**: a room above its target says warm, under the target less its own hysteresis says cold, and between the two
//! says what it said before; a window of the schedule moves the target of the room; the schedule as the source says cold in a window that burns and warm outside it;
//! a source that cannot say is cold, because a house without heat is worse than heat too much.

use demand::{Asked, By, Demand, Player, Tuning, Why};
use schedule::{Profile, Schedule, Wall, Window};

const COMFORT: f32 = 21.0;
const ECO: f32 = 17.0;
const DROP: f32 = 0.5;

fn window(text: &str) -> Option<Window> {
    Some(Window::parse(text).expect("the row's window is written right"))
}

fn tuning(by: By, schedule: Schedule) -> Tuning {
    Tuning { by, room_c: COMFORT, room_eco_c: ECO, room_drop_c: DROP, schedule }
}

/// Monday at HH:MM.
fn monday(hour: u16, minute: u16) -> Option<Wall> {
    Some(Wall { day: 0, minute: hour * 60 + minute })
}

fn cold(why: Why) -> Option<Asked> {
    Some(Asked { demand: Demand::Cold, why })
}

fn warm(why: Why) -> Option<Asked> {
    Some(Asked { demand: Demand::Warm, why })
}

fn room_says(player: &mut Player, t: &Tuning, room_c: f32) -> Option<Asked> {
    player.ask(t, Some(room_c), None).asked
}

// ---------------------------------------------------------------- the room against its target

#[test]
fn a_room_at_or_over_its_target_says_warm() {
    let t = tuning(By::Room, Schedule::default());
    assert_eq!(room_says(&mut Player::new(), &t, COMFORT), warm(Why::TargetReached), "21 is the line itself: the room is warm on it");
    assert_eq!(room_says(&mut Player::new(), &t, 24.0), warm(Why::TargetReached));
}

#[test]
fn a_room_at_its_target_less_the_hysteresis_or_under_it_says_cold() {
    let t = tuning(By::Room, Schedule::default());
    assert_eq!(room_says(&mut Player::new(), &t, COMFORT - DROP), cold(Why::BelowTarget), "20.5 is the line itself: the room is cold on it");
    assert_eq!(room_says(&mut Player::new(), &t, 12.0), cold(Why::BelowTarget));
}

#[test]
fn between_the_two_lines_the_room_says_what_it_said_before() {
    let t = tuning(By::Room, Schedule::default());
    let mut player = Player::new();
    assert_eq!(room_says(&mut player, &t, 18.0), cold(Why::BelowTarget));
    assert_eq!(room_says(&mut player, &t, 20.8), cold(Why::BelowTarget), "20.8 is inside the band, and the room came up from the cold: still cold");
    assert_eq!(room_says(&mut player, &t, 21.2), warm(Why::TargetReached));
    assert_eq!(room_says(&mut player, &t, 20.8), warm(Why::TargetReached), "20.8 is inside the band, and the room came down from the warm: still warm");
    assert_eq!(room_says(&mut player, &t, 20.4), cold(Why::BelowTarget));
}

#[test]
fn a_board_that_comes_up_with_the_room_inside_the_band_says_warm() {
    let t = tuning(By::Room, Schedule::default());
    assert_eq!(room_says(&mut Player::new(), &t, 20.8), warm(Why::TargetReached), "the first reading is inside the band and the house is not known to be cold: a restart does not light the burner");
}

#[test]
fn a_board_that_comes_up_with_the_room_under_the_band_says_cold() {
    let t = tuning(By::Room, Schedule::default());
    assert_eq!(room_says(&mut Player::new(), &t, 20.0), cold(Why::BelowTarget), "under the band the house is cold, restart or not");
}

#[test]
fn a_room_not_read_once_since_the_board_came_up_asks_for_no_heat() {
    let t = tuning(By::Room, Schedule::default());
    let mut player = Player::new();
    assert_eq!(player.ask(&t, None, None).asked, warm(Why::NotRead), "nothing is known of the house yet: the boiler is not asked");
    assert_eq!(player.ask(&t, None, None).asked, warm(Why::NotRead), "and it stays so while the room says nothing");
    assert_eq!(room_says(&mut player, &t, 20.8), warm(Why::TargetReached), "the first reading inside the band ends the wait as the warm house");
}

#[test]
fn a_room_first_read_under_the_band_after_the_wait_says_cold() {
    let t = tuning(By::Room, Schedule::default());
    let mut player = Player::new();
    assert_eq!(player.ask(&t, None, None).asked, warm(Why::NotRead));
    assert_eq!(room_says(&mut player, &t, 19.0), cold(Why::BelowTarget));
    assert_eq!(room_says(&mut player, &t, 20.8), cold(Why::BelowTarget), "inside the band the room that came up from the cold is still cold");
}

#[test]
fn a_room_that_spoke_and_fell_silent_is_cold_and_stays_cold_inside_the_band_when_it_returns() {
    let t = tuning(By::Room, Schedule::default());
    let mut player = Player::new();
    assert_eq!(room_says(&mut player, &t, 20.8), warm(Why::TargetReached));
    assert_eq!(player.ask(&t, None, None).asked, cold(Why::SourceSilent), "a sensor that spoke and went silent is a protection and it stays");
    assert_eq!(room_says(&mut player, &t, 20.8), cold(Why::BelowTarget), "silence set the memory cold: back inside the band it is cold until the target");
}

#[test]
fn the_hysteresis_of_the_room_is_its_own_number() {
    let wide = Tuning { room_drop_c: 2.0, ..tuning(By::Room, Schedule::default()) };
    let mut player = Player::new();
    assert_eq!(room_says(&mut player, &wide, 21.5), warm(Why::TargetReached));
    assert_eq!(room_says(&mut player, &wide, 19.5), warm(Why::TargetReached), "19.5 is inside a band of 2 under 21: the narrow band would have said cold");
    assert_eq!(room_says(&mut player, &wide, 19.0), cold(Why::BelowTarget));
}

#[test]
fn a_room_that_cannot_say_is_cold_whatever_it_said_before() {
    let t = tuning(By::Room, Schedule::default());
    let mut player = Player::new();
    assert_eq!(room_says(&mut player, &t, 23.0), warm(Why::TargetReached));
    assert_eq!(player.ask(&t, None, None).asked, cold(Why::SourceSilent), "a warm room that went silent is a source that is silent");
}

// ---------------------------------------------------------------- the windows move the target

#[test]
fn the_window_that_is_open_names_the_target_of_the_room() {
    let schedule = Schedule { windows: [window("daily 22:00-06:00 eco"), None, None, None, None, None], ..Schedule::default() };
    let t = tuning(By::Room, schedule);
    // 19 °C: under the comfort target by more than the band, over the eco target
    let by_day = Player::new().ask(&t, Some(19.0), monday(12, 0));
    assert_eq!((by_day.asked, by_day.target_c, by_day.profile), (cold(Why::BelowTarget), Some(COMFORT), Profile::Comfort));
    let by_night = Player::new().ask(&t, Some(19.0), monday(23, 0));
    assert_eq!((by_night.asked, by_night.target_c, by_night.profile), (warm(Why::TargetReached), Some(ECO), Profile::Eco));
}

#[test]
fn a_profile_that_is_off_has_no_target_and_the_room_is_warm_however_cold_it_is() {
    let schedule = Schedule { windows: [window("daily 22:00-06:00 off"), None, None, None, None, None], ..Schedule::default() };
    let t = tuning(By::Room, schedule);
    let at_night = Player::new().ask(&t, Some(10.0), monday(23, 0));
    assert_eq!((at_night.asked, at_night.target_c), (warm(Why::ProfileOff), None));
    let silent_at_night = Player::new().ask(&t, None, monday(23, 0));
    assert_eq!(silent_at_night.asked, warm(Why::ProfileOff), "nothing is asked of a room under a profile that holds none, and its silence asks for nothing");
}

#[test]
fn a_board_that_does_not_know_the_time_holds_the_room_by_the_main_profile() {
    let schedule = Schedule { main: Profile::Eco, windows: [window("daily 22:00-06:00 comfort"), None, None, None, None, None], ..Schedule::default() };
    let t = tuning(By::Room, schedule);
    let verdict = Player::new().ask(&t, Some(18.0), None);
    assert_eq!((verdict.profile, verdict.target_c, verdict.asked), (Profile::Eco, Some(ECO), warm(Why::TargetReached)));
}

// ---------------------------------------------------------------- the schedule as the source

#[test]
fn the_schedule_says_cold_in_a_window_that_burns_and_warm_outside_it() {
    let schedule = Schedule { main: Profile::Off, windows: [window("daily 07:00-09:00 comfort"), None, None, None, None, None], ..Schedule::default() };
    let t = tuning(By::Schedule, schedule);
    assert_eq!(Player::new().ask(&t, None, monday(8, 0)).asked, cold(Why::ScheduledOn));
    assert_eq!(Player::new().ask(&t, None, monday(12, 0)).asked, warm(Why::ScheduledOff));
    assert_eq!(Player::new().ask(&t, None, monday(9, 0)).asked, warm(Why::ScheduledOff), "the window is open up to its end and not at it");
}

#[test]
fn the_schedule_as_the_source_does_not_look_at_the_room() {
    let schedule = Schedule { main: Profile::Off, windows: [window("daily 07:00-09:00 comfort"), None, None, None, None, None], ..Schedule::default() };
    let t = tuning(By::Schedule, schedule);
    assert_eq!(Player::new().ask(&t, Some(30.0), monday(8, 0)).asked, cold(Why::ScheduledOn), "a hot room does not close the window");
    assert_eq!(Player::new().ask(&t, Some(5.0), monday(12, 0)).asked, warm(Why::ScheduledOff), "a cold room does not open it");
    assert_eq!(Player::new().ask(&t, None, monday(8, 0)).asked, cold(Why::ScheduledOn), "and a room that says nothing is no silence of this source");
}

#[test]
fn a_board_that_does_not_know_the_time_says_what_the_main_profile_says() {
    let burning = Schedule { main: Profile::Comfort, windows: [window("daily 07:00-09:00 off"), None, None, None, None, None], ..Schedule::default() };
    assert_eq!(Player::new().ask(&tuning(By::Schedule, burning), None, None).asked, cold(Why::ScheduledOn));
    let standing = Schedule { main: Profile::Off, windows: [window("daily 07:00-09:00 comfort"), None, None, None, None, None], ..Schedule::default() };
    assert_eq!(Player::new().ask(&tuning(By::Schedule, standing), None, None).asked, warm(Why::ScheduledOff));
}

#[test]
fn the_eco_profile_burns_as_the_source_and_only_the_off_profile_stands() {
    let schedule = Schedule { main: Profile::Off, windows: [window("daily 07:00-09:00 eco"), None, None, None, None, None], ..Schedule::default() };
    assert_eq!(Player::new().ask(&tuning(By::Schedule, schedule), None, monday(8, 0)).asked, cold(Why::ScheduledOn));
}

// ---------------------------------------------------------------- nobody

#[test]
fn nobody_playing_the_role_says_nothing_whatever_the_room_and_the_clock_say() {
    let schedule = Schedule { main: Profile::Off, windows: [window("daily 07:00-09:00 comfort"), None, None, None, None, None], ..Schedule::default() };
    let t = tuning(By::Nobody, schedule);
    for (room_c, wall) in [(Some(5.0), monday(8, 0)), (Some(30.0), monday(12, 0)), (None, None)] {
        assert_eq!(Player::new().ask(&t, room_c, wall).asked, None, "{room_c:?} at {wall:?}");
    }
}

// ---------------------------------------------------------------- the two words as one byte for a card

#[test]
fn every_word_with_every_reason_is_a_code_that_reads_back_as_itself_and_nought_is_nothing() {
    let reasons = [Why::BelowTarget, Why::TargetReached, Why::ProfileOff, Why::ScheduledOn, Why::ScheduledOff, Why::SourceSilent];
    let mut seen = std::collections::BTreeSet::new();
    for demand in [Demand::Cold, Demand::Warm] {
        for why in reasons {
            let asked = Asked { demand, why };
            assert!(asked.code() >= 1, "{asked:?}: nought is nothing asked");
            assert_eq!(Asked::of_code(asked.code()), Some(asked));
            assert!(seen.insert(asked.code()), "{asked:?} shares a code");
        }
    }
    assert_eq!(Asked::of_code(0), None);
    assert_eq!(Asked::of_code(200), None, "a number no word makes is no word");
}

// ---------------------------------------------------------------- the first round of the sensors after the board came up

#[test]
fn before_the_first_round_of_the_sensors_the_house_asks_for_nothing_whatever_the_room_says() {
    // the room may be unread only because its turn has not come, and a reading among the first may be the first of many: nothing that lights is said yet
    let t = tuning(By::Room, Schedule::default());
    let mut player = Player::after_boot();
    assert_eq!(player.ask(&t, None, None).asked, warm(Why::Starting), "the room is unread, and its turn may not have come");
    assert_eq!(room_says(&mut player, &t, 12.0), warm(Why::Starting), "12 is far under the band, and the round has not gone through: it does not light");
    assert_eq!(room_says(&mut player, &t, 20.8), warm(Why::Starting));
    assert_eq!(room_says(&mut player, &t, 23.0), warm(Why::Starting));
}

#[test]
fn after_the_first_round_the_room_that_answered_decides() {
    let t = tuning(By::Room, Schedule::default());
    let (mut inside, mut under) = (Player::after_boot(), Player::after_boot());
    inside.first_round_done();
    under.first_round_done();
    assert_eq!(room_says(&mut inside, &t, 20.8), warm(Why::TargetReached), "inside the band, and the house is not known to be cold");
    assert_eq!(room_says(&mut under, &t, 19.0), cold(Why::BelowTarget));
}

#[test]
fn after_the_first_round_a_room_that_never_answered_is_a_dead_sensor_and_says_not_read() {
    let t = tuning(By::Room, Schedule::default());
    let mut player = Player::after_boot();
    assert_eq!(player.ask(&t, None, None).asked, warm(Why::Starting));
    player.first_round_done();
    assert_eq!(player.ask(&t, None, None).asked, warm(Why::NotRead), "the round went through and the room said nothing: the floor of the water is all that lights");
    player.first_round_done();
    assert_eq!(player.ask(&t, None, None).asked, warm(Why::NotRead), "the round is done once, and saying so again changes nothing");
}

#[test]
fn a_profile_that_is_off_is_off_before_the_first_round_too() {
    let schedule = Schedule { windows: [window("daily 22:00-06:00 off"), None, None, None, None, None], ..Schedule::default() };
    let t = tuning(By::Room, schedule);
    assert_eq!(Player::after_boot().ask(&t, Some(12.0), monday(23, 0)).asked, warm(Why::ProfileOff), "the person said off, and that is not a guess");
}
