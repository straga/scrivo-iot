//! What goes to the console and what waits until a person asks for it. No hardware here: the levels are words
//! and a table, and the firmware's logger asks this before it writes a line.
//!
//! The table has one keeper: a line of the log, from any task, reads the table as
//! its keeper last published it and names a module it has not seen; the keeper learns those names on its pass, sets
//! levels, lists, and publishes the table whole again.

use loudness::{Keeper, Level, Loudness};

/// A table whose keeper has learned MODULES, each having spoken once.
fn spoken(modules: &[&str]) -> (Loudness, Keeper) {
    let loud = Loudness::new(Level::Info);
    let mut keeper = Keeper::new(&loud);
    for module in modules {
        loud.speaks(module, Level::Info);
    }
    keeper.learn(&loud);
    (loud, keeper)
}

#[test]
fn a_module_nobody_touched_speaks_at_the_level_the_board_starts_with() {
    // The default is what the console looks like when nobody has asked for anything: events, not wellbeing.
    let loud = Loudness::new(Level::Info);
    assert!(loud.speaks("burner::fan", Level::Warn));
    assert!(loud.speaks("burner::fan", Level::Info));
    assert!(!loud.speaks("burner::fan", Level::Debug));
}

#[test]
fn details_asked_for_by_name_come_from_that_module_alone() {
    // THE WHOLE POINT. A person watching the fan turns the fan up, and the rest of the board stays as quiet as
    // it was - otherwise turning anything on brings back the flood the quiet was for.
    let (loud, mut keeper) = spoken(&["burner::fan", "burner::flue"]);
    keeper.set(&loud, "burner::fan", Level::Debug).expect("a module that has spoken takes a level");
    assert!(loud.speaks("burner::fan", Level::Debug));
    assert!(!loud.speaks("burner::flue", Level::Debug));
}

#[test]
fn a_module_turned_back_down_goes_quiet_again() {
    let (loud, mut keeper) = spoken(&["burner::fan"]);
    keeper.set(&loud, "burner::fan", Level::Debug).unwrap();
    assert!(loud.speaks("burner::fan", Level::Debug));
    keeper.set(&loud, "burner::fan", Level::Info).unwrap();
    assert!(!loud.speaks("burner::fan", Level::Debug));
}

#[test]
fn a_failure_is_seen_whatever_the_level_says_short_of_off() {
    // What must never be turned off by tidying: a board told to be quiet still says when something broke.
    let (loud, mut keeper) = spoken(&["burner::bus"]);
    keeper.set(&loud, "burner::bus", Level::Error).unwrap();
    assert!(loud.speaks("burner::bus", Level::Error));
    assert!(!loud.speaks("burner::bus", Level::Warn));
    // and off means off, which is a thing a person asks for on purpose
    keeper.set(&loud, "burner::bus", Level::Off).unwrap();
    assert!(!loud.speaks("burner::bus", Level::Error));
}

#[test]
fn a_name_the_board_does_not_know_is_refused_with_the_names_it_does() {
    // The pair on bad data, and the reason the command is worth having: a name typed wrong must not read as
    // "done, the details are on" - a person would then sit watching a console that says nothing.
    let (loud, mut keeper) = spoken(&["burner::fan", "burner::flue"]);
    let why = keeper.set(&loud, "burner::fna", Level::Debug).expect_err("an unknown module is refused");
    assert!(why.contains("burner::fna"), "{why:?}");
    assert!(why.contains("burner::fan"), "{why:?}");
    assert!(why.contains("burner::flue"), "{why:?}");
    // and nothing was changed by the attempt
    assert!(!loud.speaks("burner::fan", Level::Debug));
}

#[test]
fn the_listing_names_every_module_that_has_spoken_and_the_level_it_stands_at() {
    // So that a name is never read out of the code: what the board lists is what the board has.
    let (loud, mut keeper) = spoken(&["burner::fan", "burner::flue"]);
    keeper.set(&loud, "burner::fan", Level::Debug).unwrap();
    loud.speaks("burner::fan", Level::Debug);
    let said = keeper.lines(&loud).join("\n");
    assert!(said.contains("burner::fan"), "{said:?}");
    assert!(said.contains("burner::flue"), "{said:?}");
    assert!(said.contains("debug"), "{said:?}");
    assert!(said.contains("info"), "{said:?}");
    // a module listed once, however many lines it wrote
    assert_eq!(keeper.lines(&loud).len(), 2, "{said:?}");
}

#[test]
fn a_module_that_spoke_since_the_keeper_s_last_pass_is_listed_and_takes_a_level() {
    // The table is the keeper's; a line only names a module it has not seen. Until the keeper's pass that module
    // speaks at the default, never refused and never silent.
    let (loud, mut keeper) = spoken(&["burner::fan"]);
    assert!(loud.speaks("burner::draught", Level::Info), "a module the table has not seen yet is silent");
    let said = keeper.lines(&loud).join("\n");
    assert!(said.contains("burner::draught"), "the listing does not name a module that has spoken: {said:?}");
    keeper.set(&loud, "burner::draught", Level::Debug).expect("a module learned on the pass takes a level");
    assert!(loud.speaks("burner::draught", Level::Debug));
}

#[test]
fn a_level_the_keeper_sets_reaches_a_line_written_in_another_task() {
    let (loud, mut keeper) = spoken(&["burner::fan", "burner::flue"]);
    let loud: &'static Loudness = Box::leak(Box::new(loud));
    keeper.set(loud, "burner::fan", Level::Trace).unwrap();
    let heard = std::thread::spawn(move || loud.speaks("burner::fan", Level::Trace)).join().unwrap();
    assert!(heard, "a line of another task did not see the level the keeper set");
    // the pair: a module the keeper did not touch stays where the board starts, in that task too
    let quiet = std::thread::spawn(move || loud.speaks("burner::flue", Level::Trace)).join().unwrap();
    assert!(!quiet, "a module nobody touched spoke trace in another task");
}

#[test]
fn modules_that_speak_faster_than_the_keeper_takes_their_names_are_all_learned_as_they_speak_again() {
    // Between two passes more new modules may speak than their channel holds; its oldest names go, and a module the
    // table has not seen names itself again with its next line.
    let (loud, mut keeper) = spoken(&[]);
    let many: Vec<String> = (0..loudness::HEARD * 2).map(|i| format!("module::{i}")).collect();
    for _ in 0..3 {
        for module in &many {
            loud.speaks(module, Level::Info);
        }
        keeper.learn(&loud);
    }
    let said = keeper.lines(&loud).join("\n");
    assert_eq!(keeper.lines(&loud).len(), many.len(), "{said:?}");
}

#[test]
fn the_words_of_a_level_go_both_ways_and_nothing_else_does() {
    for level in [Level::Off, Level::Error, Level::Warn, Level::Info, Level::Debug, Level::Trace] {
        assert_eq!(Level::of_word(level.word()), Some(level), "{:?}", level.word());
        assert!(Level::WORDS.contains(level.word()), "{} is not in WORDS", level.word());
    }
    // the pair: a word that is not a level is not guessed at
    assert_eq!(Level::of_word("loud"), None);
    assert_eq!(Level::of_word(""), None);
    assert_eq!(Level::of_word("INFO"), Some(Level::Info), "the case a person types in is not a refusal");
}

#[test]
fn a_board_that_has_more_modules_than_it_remembers_says_so_instead_of_dropping_them_quietly() {
    // The pair on the table's own limit. Silence here would read as "that module never spoke", and a person
    // would go looking for it in the code - which is the one thing this command exists to spare them.
    let (loud, mut keeper) = spoken(&[]);
    for i in 0..loudness::MODULES + 3 {
        loud.speaks(&format!("module::{i}"), Level::Info);
        keeper.learn(&loud);
    }
    let said = keeper.lines(&loud).join("\n");
    assert_eq!(keeper.lines(&loud).len(), loudness::MODULES + 1, "{said:?}");
    assert!(said.contains("3 more"), "{said:?}");
    // and a module the table had no room for still speaks at the default rather than going silent
    assert!(loud.speaks("module::34", Level::Info));
    assert!(!loud.speaks("module::34", Level::Debug));
}
