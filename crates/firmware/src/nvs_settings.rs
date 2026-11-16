//! The settings a person changes in place (crates/settings), kept in nvs, namespace `rust_app`, a key a card
//! (settings::key_of) and each card's set in one write so a card is never half written: seeded once from the device's
//! description when empty, the one set of an older image split into the cards at the first start, changed
//! from the page and the console by the owner of the flash (`flash_owner`), read from the set it publishes - so a
//! change works without a restart, and a new image leaves what was set in place.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use flash::{Answer, Ask, Owner};
use settings::{Keeper, Moved, Refusal, Setting, Store, Writes};

use crate::flash_owner::{refusal, refusal_to_save, DESK, SETTINGS, WITHIN};

const NAMESPACE: &str = "rust_app";

/// The longest set read back from nvs: one card's settings, name and value each, within page::MAX_VALUE - or, once,
/// the one set an older image kept for the whole board.
const MAX_READ: usize = 1024;

/// How many times this image has touched the flash through this store since it came up, the settings and the modules'
/// memory apart. Written by one task at a time - main while it seeds, then the owner of the flash - and read by
/// `status`: atomic counters with one writer, not a lock two parts take.
///
/// **IT IS A MEASURING INSTRUMENT, AND IT LIVES IN RAM FOR THAT REASON.** What a write costs is not the wear
/// alone: writing nvs shuts the cache off for the length of it, and that window is where the unexplained
/// restart of 22.09 was laid. A counter kept in flash would open the very window it exists to count, and its
/// numbers would be about the instrument. Dying with the board is what such an instrument may cost.
static SETTINGS_WRITTEN: AtomicU32 = AtomicU32::new(0);
static MEMORY_WRITTEN: AtomicU32 = AtomicU32::new(0);

/// How many writes to flash this store has made since the board came up, the settings apart from the modules'
/// memory (`remember`): a module writes its state down on each change of its own, and a start that did no more
/// than that must not read as a start that changed a setting. Nought settings across a whole run of a probe is
/// what says the probe left no setting behind; any other number says how many it left.
///
/// **IT COUNTS THIS WRITER AND NOT THE FLASH.** There are at least four: this store; the count of short
/// starts, which `ways_down` keeps in nvs through its own handle in C and writes on every boot; the
/// confirmation of the image, which `net::confirm_image` writes to otadata once the network is up; and the
/// Wi-Fi setting itself, which the driver keeps in its own nvs namespace on every `set_configuration`
/// (`net::join`, and `CONFIG_ESP_WIFI_NVS_ENABLED=y` in our build). A run that reads nought here left no
/// settings behind - it says nothing about the other three.
pub fn written() -> Writes {
    Writes { settings: SETTINGS_WRITTEN.load(Ordering::Relaxed), memory: MEMORY_WRITTEN.load(Ordering::Relaxed) }
}

/// Counts one write to KEY, as [`Writes::count`] tells a setting's key from a module's memory.
fn count(key: &str) {
    let mut one = Writes::default();
    one.count(key);
    SETTINGS_WRITTEN.fetch_add(one.settings, Ordering::Relaxed);
    MEMORY_WRITTEN.fetch_add(one.memory, Ordering::Relaxed);
}

/// nvs, namespace `rust_app`, as the owner of the flash keeps it.
pub(crate) struct Nvs(EspNvs<NvsDefault>);

impl Store for Nvs {
    fn get(&self, name: &str) -> Option<String> {
        let mut buf = [0u8; MAX_READ];
        self.0.get_str(name, &mut buf).ok().flatten().map(str::to_string)
    }

    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        // COUNTED BEFORE THE CALL AND NOT AFTER A SUCCESS. A write that came back an error had reached the
        // flash all the same and had already shut the cache off; counting the Ok ones would answer about our
        // bookkeeping rather than about the flash.
        count(name);
        self.0.set_str(name, value).map_err(|e| format!("nvs: {e}"))
    }

    fn remove(&mut self, name: &str) -> Result<bool, String> {
        count(name);
        self.0.remove(name).map_err(|e| format!("nvs: {e}"))
    }
}

/// The board's settings, in the order the tab shows them; set once at start.
static LIST: OnceLock<Vec<&'static Setting>> = OnceLock::new();
/// What is done when a setting changes, by its name: the clock takes a new server or zone at once.
static ON_CHANGE: OnceLock<fn(&str)> = OnceLock::new();
/// What a setting means now, by its name, beside it on the tab: where the time came from, how much room the history
/// takes.
/// Opens the namespace, carries what changed its meaning and seeds what is empty in it from DESCRIPTION, else from the
/// defaults - in the calling task, before any other writes the flash; answers the owner of the flash that keeps them
/// from then on (`flash_owner::start`). ON_CHANGE is told the name of every setting changed later. Without nvs every setting reads its default and none can change; the files are written all the same.
pub fn start(
    part: EspDefaultNvsPartition,
    list: Vec<&'static Setting>,
    rules: &'static settings::Rules,
    description: &[(&str, &str)],
    on_change: fn(&str),
) -> Owner<Nvs> {
    let _ = LIST.set(list.clone());
    let _ = ON_CHANGE.set(on_change);
    let mut store = match EspNvs::new(part, NAMESPACE, true) {
        Ok(nvs) => Nvs(nvs),
        Err(e) => {
            log::error!("settings: nvs namespace '{NAMESPACE}' not opened: {e} - every setting reads its default");
            return Owner::new(None, &SETTINGS, crate::flash_owner::files);
        }
    };
    // BEFORE THE SEEDING, because that is what tells a new board from one tuned under an older meaning: a new
    // board has no set at all, and seeding creates one. A setting whose unit changed between images is carried
    // here, at the loudest level the log has for a thing nobody asked for - it is a change to what a device
    // does, made without a person touching anything.
    for line in settings::carry_over(&list, rules, &mut store) {
        log::warn!("{line}");
    }
    for line in settings::seed(&list, rules, &mut store, description) {
        log::info!("{line}");
    }
    Owner::new(Some(Keeper::new(list, rules, store)), &SETTINGS, crate::flash_owner::files)
}

/// Tells what depends on the setting NAME that it moved; the owner of the flash calls it after a save.
pub(crate) fn changed(name: &str) {
    if let Some(on_change) = ON_CHANGE.get() {
        on_change(name);
    }
}

/// The settings as they stand at one moment, whole. **A DECISION THAT READS MORE THAN ONE SETTING TAKES ONE OF THESE
/// AND READS THEM ALL FROM IT**: read one by one, a save landing between two reads would hand it half of the set before
/// and half after - two thresholds out of their order, two sockets with the same role.
pub struct Now(Option<std::sync::Arc<settings::Values>>);

impl Now {
    /// The value of SETTING in this set; its default when the board keeps no settings.
    pub fn value(&self, setting: &Setting) -> String {
        self.0.as_ref().map_or_else(|| setting.default.to_string(), |set| set.get(setting))
    }

    /// The value of the setting called NAME in this set; None for a name the board has not.
    pub fn kept(&self, name: &str) -> Option<String> {
        let setting = *LIST.get()?.iter().find(|x| x.name == name)?;
        Some(self.value(setting))
    }
}

/// The settings as they stand now, whole.
pub fn now() -> Now {
    Now(SETTINGS.latest())
}

/// The value of SETTING now - for a decision that reads this one alone; one that reads more takes [`now`].
pub fn value(setting: &Setting) -> String {
    now().value(setting)
}

/// The value of the setting called NAME now, as it is kept; None for a name the board has not.
pub fn kept(name: &str) -> Option<String> {
    now().kept(name)
}

/// Changes NAME to TEXT: what it is now in words, or why not - the old value stays. What depends on it is told at
/// once.
pub fn change(name: &str, text: &str) -> Result<String, String> {
    let list = LIST.get().map(|list| list.as_slice()).unwrap_or(&[]);
    change_all(&[(name.to_string(), text.to_string())])
        .map(|moved| moved.iter().map(|m| m.said(list)).collect::<Vec<_>>().join("; "))
        .map(|s| if s.is_empty() { "nothing changed".to_string() } else { s })
        .map_err(|why| why.to_string())
}

/// The tab's one save: all PAIRS or none, and what changed - each name with the value it is kept as, for the page to
/// say in its words - by the owner of the flash, and what depends on a changed setting is told there at once. A
/// refusal is a code with its fields (`settings::Refusal`).
pub fn change_all(pairs: &[(String, String)]) -> Result<Vec<Moved>, Refusal> {
    match DESK.ask(Ask::Change(pairs.to_vec()), WITHIN) {
        Ok(Answer::Changed(said)) => said.map(|changed| changed.moved),
        Ok(_) => Err(Refusal::NotWritten { detail: "the flash answered a save as something else".into() }),
        Err(refused) => Err(refusal_to_save(refused)),
    }
}

/// What the module of CARD wrote down for itself (`remember`); None when nothing is, or nvs did not open. Not a
/// setting: a person never sees or types it.
pub fn recall(card: &str) -> Option<String> {
    match DESK.ask(Ask::Recall(card.into()), WITHIN) {
        Ok(Answer::Recalled(kept)) => kept,
        _ => None,
    }
}

/// Writes TEXT down for the module of CARD, under its own key (`settings::memory_key`) and never under a setting's;
/// counted apart from the settings ([`written`]).
pub fn remember(card: &str, text: &str) -> Result<(), String> {
    match DESK.ask(Ask::Remember { card: card.into(), text: text.into() }, WITHIN) {
        Ok(Answer::Remembered(done)) => done,
        Ok(_) => Err("the flash answered the memory as something else".into()),
        Err(refused) => Err(refusal(refused)),
    }
}

/// The shell's `settings`: a line each - the label and the value in words.
pub fn lines(line: &mut dyn FnMut(&str)) -> Result<(), String> {
    let (Some(list), Some(set)) = (LIST.get(), SETTINGS.latest()) else { return Err(settings::NOT_KEPT.into()) };
    let kept: Vec<(&'static Setting, String)> = list.iter().map(|x| (*x, set.get(x))).collect();
    for (x, value) in kept {
        line(&format!("{}: {}", x.label, settings::words(x, &value)));
    }
    Ok(())
}
