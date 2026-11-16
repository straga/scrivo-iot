//! How loud each module of the board is: what goes to the console by itself, and what waits until a person
//! asks for it by name.
//!
//! **A CONSOLE THAT SAYS EVERYTHING SAYS NOTHING.** The board used to write a line a second about being
//! well - crossings counted, a temperature with its raw word, rounds with no refusals - and the one line that
//! mattered, at a fall or a sensor dropping off, went past in the flood exactly when somebody needed it. So by
//! default a module speaks of events, and the details it can give are turned on where they are wanted and
//! turned off after.
//!
//! No hardware here: this is a table of words, and the firmware's logger asks it before it writes a line.

use std::sync::Arc;

use between::Lock;
use embassy_sync::watch::Watch;
use journal::Feed;

/// How many modules the board remembers with a level of their own. The burner's firmware has about twenty.
pub const MODULES: usize = 32;

/// How much a module is allowed to say.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    /// Every level a person can name, for a refusal to list.
    pub const WORDS: &'static str = "off, error, warn, info, debug, trace";

    pub fn word(self) -> &'static str {
        match self {
            Level::Off => "off",
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        }
    }

    /// The level a person typed, whatever case they typed it in; None for a word that is not one.
    pub fn of_word(word: &str) -> Option<Level> {
        match word.to_ascii_lowercase().as_str() {
            "off" => Some(Level::Off),
            "error" => Some(Level::Error),
            "warn" => Some(Level::Warn),
            "info" => Some(Level::Info),
            "debug" => Some(Level::Debug),
            "trace" => Some(Level::Trace),
            _ => None,
        }
    }
}

/// How many names of modules the table has not seen wait for its keeper between two of its passes. More lose their
/// oldest, and a module the table has not seen names itself again with its next line.
pub const HEARD: usize = 16;

/// The modules that have spoken and the level each was asked to stand at, as the keeper holds and publishes it.
#[derive(Clone, Default)]
struct Table {
    /// The modules that have spoken, and the level each was asked to stand at; None - the board's own.
    seen: Vec<(String, Option<Level>)>,
    /// Names the table had no room for. Kept so that the listing can say they exist: a module missing from it
    /// reads as one that never spoke, and then a person goes looking for the name in the code.
    over: Vec<String>,
}

impl Table {
    /// Some - the table knows MODULE, with the level asked for it; None - it has not seen it.
    fn asked(&self, module: &str) -> Option<Option<Level>> {
        match self.seen.iter().find(|(name, _)| name == module) {
            Some((_, asked)) => Some(*asked),
            None => self.over.iter().any(|name| name == module).then_some(None),
        }
    }

    /// Takes MODULE in; true when the table grew.
    fn add(&mut self, module: &str) -> bool {
        if self.asked(module).is_some() {
            return false;
        }
        if self.seen.len() < MODULES {
            self.seen.push((module.to_string(), None));
        } else if self.over.len() < MODULES {
            self.over.push(module.to_string());
        } else {
            return false;
        }
        true
    }
}

/// What every line of the log asks, from whichever task wrote it: the table as its [`Keeper`] last published it, whole.
/// A line reads it and changes nothing in it - a module the table has not seen is named to the keeper by a channel of
/// fixed size and speaks at the default meanwhile.
///
/// **NO LOCK OVER THE TABLE.** One mutex taken by every line of every task, and written by a module speaking first,
/// would put the whole log behind one lock: the keeper alone writes the table and publishes a copy, and a line takes the
/// copy as it stands. The table is read on every line and changed seldom - the way the settings are read.
pub struct Loudness {
    by_default: Level,
    table: Watch<Lock, Arc<Table>, 1>,
    heard: Feed<HEARD>,
}

impl Loudness {
    /// The table empty, every module at BY_DEFAULT; the channel of names takes its slots from the heap here, once.
    pub fn new(by_default: Level) -> Self {
        Self { by_default, table: Watch::new(), heard: Feed::new() }
    }

    /// Whether a line of LEVEL from MODULE goes out; a module the table has not seen is named to its keeper.
    ///
    /// Remembering what spoke rather than a list written by hand: a list would say what somebody believed the modules
    /// were, and this says which ones there are.
    pub fn speaks(&self, module: &str, level: Level) -> bool {
        let asked = match self.table.try_get().and_then(|table| table.asked(module)) {
            Some(asked) => asked,
            None => {
                self.heard.put(module);
                None
            }
        };
        let at = asked.unwrap_or(self.by_default);
        at != Level::Off && level <= at
    }
}

/// The one writer of the table: learns the modules that have spoken, sets their levels, lists them, and publishes the
/// table whole after every change. Held by one task (firmware: the owner of the flash, which passes over it once a
/// second); the others ask that task.
pub struct Keeper {
    table: Table,
    by_default: Level,
}

impl Keeper {
    pub fn new(loud: &Loudness) -> Self {
        let keeper = Self { table: Table::default(), by_default: loud.by_default };
        keeper.publish(loud);
        keeper
    }

    /// Takes in the modules named since the last pass, and publishes the table when it grew.
    pub fn learn(&mut self, loud: &Loudness) {
        let mut grew = false;
        while let Some(name) = loud.heard.take() {
            grew |= self.table.add(&name);
        }
        // lost names come again with their modules' next lines
        let _ = loud.heard.lost();
        if grew {
            self.publish(loud);
        }
    }

    /// Sets MODULE to LEVEL; Ok says what stands now, Err names the modules this board knows.
    ///
    /// A name it does not know is refused rather than taken: a name typed wrong that answered "done" would
    /// leave a person watching a console that says nothing, sure the details are on.
    pub fn set(&mut self, loud: &Loudness, module: &str, level: Level) -> Result<String, String> {
        self.learn(loud);
        match self.table.seen.iter_mut().find(|(name, _)| name == module) {
            Some((_, asked)) => {
                *asked = Some(level);
                self.publish(loud);
                Ok(format!("{module} {}", level.word()))
            }
            None => {
                let known: Vec<&str> = self.table.seen.iter().map(|(name, _)| name.as_str()).collect();
                Err(format!("this board has no module {module}; it has: {}", known.join(", ")))
            }
        }
    }

    /// The modules and the level each stands at, a line each, in the order they first spoke.
    pub fn lines(&mut self, loud: &Loudness) -> Vec<String> {
        self.learn(loud);
        let table = &self.table;
        let mut said: Vec<String> = table
            .seen
            .iter()
            .map(|(name, asked)| match asked {
                Some(level) => format!("{name} {} - asked for", level.word()),
                None => format!("{name} {} - as the board starts", self.by_default.word()),
            })
            .collect();
        if !table.over.is_empty() {
            said.push(format!(
                "{} more modules have spoken than this board remembers, so they stand at {} and cannot be \
                 turned up: {}",
                table.over.len(),
                self.by_default.word(),
                table.over.join(", ")
            ));
        }
        said
    }

    fn publish(&self, loud: &Loudness) {
        loud.table.sender().send(Arc::new(self.table.clone()));
    }
}
