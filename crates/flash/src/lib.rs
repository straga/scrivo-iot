//! The owner of the board's flash: one task writes the settings and the modules' memory, the others
//! ask it at its desk and are answered, and the set of settings as it stands is published whole - every reader takes
//! it from there, and nobody reads the flash for a setting.

use std::sync::Arc;

use module::Role;
use settings::{Changed, Keeper, Refusal, Store, Values, NOT_KEPT};

/// What the others ask the owner of the flash.
pub enum Ask {
    /// The tab's one save, or one setting from the console: all pairs or none.
    Change(Vec<(String, String)>),
    /// A module writes TEXT down for itself under its CARD.
    Remember { card: String, text: String },
    /// A module reads back what it wrote down.
    Recall(String),
    /// A file of vfs written, added to or taken off.
    File(File),
    /// The lines of the journal not on the flash yet: the journal is the task's that serves this desk, not the
    /// owner's, so the owner answers None and the task answers it before the owner is asked (firmware::flash_owner).
    Waiting,
    /// The end of the journal as the task holds it in memory, answered the same way as `Waiting`.
    Tail,
    /// How loud the modules are: None - the listing; Some - a module and a level, as a person typed them. The table is
    /// the same task's, like the journal, and answered the same way.
    Loud(Option<(String, String)>),
}

/// What is done to a file of vfs.
pub enum File {
    Write { path: String, bytes: Vec<u8> },
    Append { path: String, bytes: Vec<u8> },
    Remove(String),
}

/// What the owner answers.
pub enum Answer {
    Changed(Result<Changed, Refusal>),
    Remembered(Result<(), String>),
    Recalled(Option<String>),
    Filed(Result<(), String>),
    /// The lines of the journal not on the flash yet; None - no journal is kept.
    Waiting(Option<String>),
    /// The end of the journal - whether older lines are left out, and the lines -; None - no journal is kept.
    Tail(Option<(bool, String)>),
    /// The listing of the modules, or what stands after a level was set; Err says why not in words.
    Loud(Result<Vec<String>, String>),
}

/// What the owner answers to `Ask::Loud`: the table is not its own.
const NOT_LOUD: &str = "the loudness of the log is kept by the task of the flash, not by the keeper of the settings";

/// Where the others ask: four waiting at most, three answers at a time.
pub type Desk = between::Desk<Ask, Answer, 4, 3>;

/// The keeper of the settings and where the set it holds is published, and what writes vfs.
pub struct Owner<S: Store> {
    /// None when nvs did not open: the settings are refused in words, the files are written all the same.
    keeper: Option<Keeper<S>>,
    published: &'static Role<Arc<Values>>,
    files: fn(&File) -> Result<(), String>,
}

impl<S: Store> Owner<S> {
    /// Takes KEEPER and publishes the set it holds into PUBLISHED; FILES is how a file of vfs is written.
    pub fn new(keeper: Option<Keeper<S>>, published: &'static Role<Arc<Values>>, files: fn(&File) -> Result<(), String>) -> Self {
        if let Some(keeper) = &keeper {
            published.publish(Some(keeper.now()));
        }
        Owner { keeper, published, files }
    }

    /// Carries ASK out and answers it.
    pub fn answer(&mut self, ask: Ask) -> Answer {
        let files = self.files;
        let Some(keeper) = self.keeper.as_mut() else {
            return match ask {
                Ask::Change(_) => Answer::Changed(Err(Refusal::NotKept)),
                Ask::Remember { .. } => Answer::Remembered(Err(NOT_KEPT.into())),
                Ask::Recall(_) => Answer::Recalled(None),
                Ask::File(file) => Answer::Filed(files(&file)),
                Ask::Waiting => Answer::Waiting(None),
                Ask::Tail => Answer::Tail(None),
                Ask::Loud(_) => Answer::Loud(Err(NOT_LOUD.into())),
            };
        };
        match ask {
            Ask::Change(pairs) => {
                let before = keeper.now();
                let pairs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
                let said = keeper.change_all(&pairs);
                // published whenever the set moved - after a refusal too, when the flash took part of a save
                if !Arc::ptr_eq(&before, &keeper.now()) {
                    self.published.publish(Some(keeper.now()));
                }
                Answer::Changed(said)
            }
            Ask::Remember { card, text } => Answer::Remembered(keeper.remember(&card, &text)),
            Ask::Recall(card) => Answer::Recalled(keeper.recall(&card)),
            Ask::File(file) => Answer::Filed(files(&file)),
            Ask::Waiting => Answer::Waiting(None),
            Ask::Tail => Answer::Tail(None),
            Ask::Loud(_) => Answer::Loud(Err(NOT_LOUD.into())),
        }
    }

    pub fn keeper(&self) -> Option<&Keeper<S>> {
        self.keeper.as_ref()
    }
}
