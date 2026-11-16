//! The board's settings a person changes in place, from the page's tab Settings or the console, and nothing needs
//! rebuilding: what each is called for a person, the choices it offers in words, the value it keeps. No hardware here -
//! the firmware keeps the values in nvs through `Store`, so all of it is tested on the computer.
//!
//! A person never reads a key or a format: the time zone is a city, and the POSIX rule behind it stays inside; the
//! servers of time are a way ("automatically", "only from the internet", "from a server") and the list SNTP gets is
//! made from it. A new setting is one more `Setting` in ALL: the page and the console draw and take every setting the
//! same way, and neither knows any by name.


/// Where the values live: nvs on the board, a map in tests.
pub trait Store {
    /// The stored value of NAME; None when nothing is stored.
    fn get(&self, name: &str) -> Option<String>;
    /// Stores VALUE as NAME, or says why not.
    fn put(&mut self, name: &str, value: &str) -> Result<(), String>;
    /// Removes NAME: whether it was there, or why not.
    fn remove(&mut self, name: &str) -> Result<bool, String>;
}

/// Every setting of a list as it was kept at one moment: what the owner of the flash hands every reader whole, so a
/// reader sees one set - the one before a save or the one after it, never a card of each.
#[derive(PartialEq)]
pub struct Values(Vec<(&'static str, String)>);

impl Values {
    /// The value of every setting of LIST in STORE now.
    pub fn of(list: &[&'static Setting], store: &dyn Store) -> Values {
        Values(list.iter().map(|s| (s.name, value(s, store))).collect())
    }

    /// The value of SETTING in this set; its default when the set has none.
    pub fn get(&self, setting: &Setting) -> String {
        self.0.iter().find(|(n, _)| *n == setting.name).map_or_else(|| setting.default.to_string(), |(_, v)| v.clone())
    }
}

/// What a save changed: the settings moved with the value each is kept as, the lines of the log, and the names of the
/// settings whose values moved.
pub struct Changed {
    pub moved: Vec<Moved>,
    pub said: Vec<String>,
    pub names: Vec<&'static str>,
}

/// One setting a save moved, and the value it is kept as. **THE BOARD SAYS THE NAME AND THE VALUE, NOT THE LABEL OR
/// THE WORDS**: the label of a setting and the words of a choice are the page's own table, in the language the page
/// speaks, and a phrase built here is a string the board makes on the heap for every save and a person reads in the
/// wrong language ("Hold until" on a Russian page).
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
    pub name: &'static str,
    pub value: String,
}

impl Moved {
    /// The answer of a save for this one: `{"name", "value"}`.
    pub fn wire(&self) -> serde_json::Value {
        serde_json::json!({"name": self.name, "value": self.value})
    }

    /// The line of the log and of the console: what a person reads of it, in the board's English.
    pub fn said(&self, list: &[&Setting]) -> String {
        match find(list, self.name) {
            Some(s) => format!("{}: {}", s.label, words(s, &self.value)),
            None => format!("{}: {}", self.name, self.value),
        }
    }
}

/// Why a save was not taken: a code and the numbers or names it stands on. **A CODE AND NOT A SENTENCE**, for the
/// reason [`Moved`] says; [`Refusal::wire`] is what the page gets and [`std::fmt::Display`] is the line of the log and
/// of the console, which a person reads off a terminal.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// The board has no setting of this name.
    NoSetting { name: String },
    /// TEXT is no number of months from FROM to TO.
    NotMonths { name: &'static str, text: String, from: u32, to: u32 },
    /// TEXT is no number of seconds from FROM to TO.
    NotSeconds { name: &'static str, text: String, from: u32, to: u32 },
    /// FROM and TO in tenths of UNIT - of a second for the times of the auger, of a degree for a target.
    NotTenths { name: &'static str, text: String, from: u32, to: u32, unit: &'static str },
    /// TEXT is no whole number from FROM to TO.
    NotWhole { name: &'static str, text: String, from: u32, to: u32 },
    /// TEXT is none of CHOICES, by their values. OWN is the value of the choice that takes a word of its own, when the
    /// setting has one: what it takes is in the page's table.
    NoneOf { name: &'static str, text: String, choices: Vec<&'static str>, own: Option<&'static str> },
    /// A mode held by a sensor is asked for while no socket plays ROLE; SAID is the role in the words of a person, which the device's rule knows.
    NoSocket { name: &'static str, role: &'static str, said: String },
    /// The board keeps no settings: nvs did not open.
    NotKept,
    /// The store refused the write, for DETAIL - the store's own reason, kept for the log and the console and never sent to the page.
    NotWritten { detail: String },
    /// The owner of the flash has too much asked of it now.
    Busy,
    /// The owner of the flash did not answer within WITHIN_S seconds.
    Silent { within_s: u64 },
}

impl Refusal {
    /// The code the page looks its phrase up by.
    pub fn code(&self) -> &'static str {
        match self {
            Refusal::NoSetting { .. } => "no_setting",
            Refusal::NotMonths { .. } => "not_months",
            Refusal::NotSeconds { .. } => "not_seconds",
            Refusal::NotTenths { .. } => "not_tenths",
            Refusal::NotWhole { .. } => "not_whole",
            Refusal::NoneOf { .. } => "none_of",
            Refusal::NoSocket { .. } => "no_socket",
            Refusal::NotKept => "not_kept",
            Refusal::NotWritten { .. } => "not_written",
            Refusal::Busy => "owner_busy",
            Refusal::Silent { .. } => "owner_silent",
        }
    }

    /// The refusal as the page gets it: the code and the fields it stands on.
    pub fn wire(&self) -> serde_json::Value {
        use serde_json::json;
        let code = self.code();
        match self {
            Refusal::NoSetting { name } => json!({"code": code, "name": name}),
            Refusal::NotMonths { name, text, from, to }
            | Refusal::NotSeconds { name, text, from, to }
            | Refusal::NotWhole { name, text, from, to } => json!({"code": code, "name": name, "text": text, "from": from, "to": to}),
            // the span is said in the unit, as a person types it, and not in the tenths it is kept in
            Refusal::NotTenths { name, text, from, to, unit } => json!({"code": code, "name": name, "text": text, "from": tenths_text(*from), "to": tenths_text(*to), "unit": unit}),
            Refusal::NoneOf { name, text, choices, own: None } => json!({"code": code, "name": name, "text": text, "choices": choices}),
            Refusal::NoneOf { name, text, choices, own: Some(own) } => json!({"code": code, "name": name, "text": text, "choices": choices, "own": own}),
            Refusal::NoSocket { name, role, .. } => json!({"code": code, "name": name, "role": role}),
            // the store's own words are for the log (`Display`): a text a driver wrote is no word of the page's and is not sent
            Refusal::NotKept | Refusal::Busy | Refusal::NotWritten { .. } => json!({"code": code}),
            Refusal::Silent { within_s } => json!({"code": code, "within_s": within_s}),
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NoSetting { name } => write!(f, "there is no setting '{name}'"),
            Refusal::NotMonths { name, text, from, to } => write!(f, "{name}: '{text}' is not a number of months from {from} to {to}"),
            Refusal::NotSeconds { name, text, from, to } => write!(f, "{name}: '{text}' is not a number of seconds from {from} to {to}"),
            Refusal::NotTenths { name, text, from, to, unit } => write!(f, "{name}: '{text}' is not a number from {} to {} {unit} with one digit after the point at most", tenths_text(*from), tenths_text(*to)),
            Refusal::NotWhole { name, text, from, to } => write!(f, "{name}: '{text}' is not a whole number from {from} to {to}"),
            Refusal::NoneOf { name, text, choices, own: None } => write!(f, "{name}: '{text}' is none of {}", choices.join(", ")),
            Refusal::NoneOf { name, text, choices, own: Some(own) } => {
                write!(f, "{name}: '{text}' is none of {} - for anything else write {own}:<a word of its own>", choices.join(", "))
            }
            Refusal::NoSocket { name, said, .. } => write!(f, "{name}: no socket measures {said}; give one that role first"),
            Refusal::NotKept => f.write_str(NOT_KEPT),
            Refusal::NotWritten { detail } => write!(f, "not written: {detail}"),
            Refusal::Busy => f.write_str("the flash has too much asked of it now - try again"),
            Refusal::Silent { within_s } => write!(f, "the flash did not answer within {within_s} s"),
        }
    }
}

/// Why the settings cannot be asked for: the board keeps none.
pub const NOT_KEPT: &str = "the settings are not kept: nvs was not opened";

/// The settings of one board and the store they are kept in, held by the one task that writes the flash; the set as
/// it stands now is [`Keeper::now`].
pub struct Keeper<S: Store> {
    list: Vec<&'static Setting>,
    rules: &'static Rules,
    store: S,
    now: std::sync::Arc<Values>,
}

impl<S: Store> Keeper<S> {
    pub fn new(list: Vec<&'static Setting>, rules: &'static Rules, store: S) -> Self {
        let now = std::sync::Arc::new(Values::of(&list, &store));
        Keeper { list, rules, store, now }
    }

    /// The set as it stands now, whole.
    pub fn now(&self) -> std::sync::Arc<Values> {
        self.now.clone()
    }

    /// The settings of the board, in the order of the list.
    pub fn list(&self) -> &[&'static Setting] {
        &self.list
    }

    /// The tab's one save: all PAIRS or none; the set as it stands now moves only when the store took them.
    ///
    /// **AFTER A REFUSAL THE SET IS READ BACK FROM THE STORE, AND NOT KEPT AS IT WAS PROMISED.** A save across two
    /// cards is two writes ([`change_all`]); the flash refusing the second leaves the first written. What the board
    /// holds then is what the flash holds, so the set moves to that - and stays the same set, the same `Arc`, when the
    /// flash took nothing.
    pub fn change_all(&mut self, pairs: &[(&str, &str)]) -> Result<Changed, Refusal> {
        let moved = change_all(&self.list, self.rules, &mut self.store, pairs);
        let now = Values::of(&self.list, &self.store);
        let names: Vec<&'static str> =
            self.list.iter().filter(|s| now.get(s) != self.now.get(s)).map(|s| s.name).collect();
        if now != *self.now {
            self.now = std::sync::Arc::new(now);
        }
        moved.map(|moved| Changed { said: moved.iter().map(|m| m.said(&self.list)).collect(), moved, names })
    }

    /// Writes TEXT down for the module of CARD, under its own key and never under a setting's.
    pub fn remember(&mut self, card: &str, text: &str) -> Result<(), String> {
        let key = memory_key_checked(card)?;
        self.store.put(&key, text)
    }

    /// What the module of CARD wrote down for itself; None when nothing is.
    pub fn recall(&self, card: &str) -> Option<String> {
        self.store.get(&memory_key_checked(card).ok()?)
    }

    pub fn store(&self) -> &S {
        &self.store
    }
}

/// The one key the whole set of settings was kept under before a key went to each card; `split` takes it apart.
pub const KEY: &str = "settings";

/// The key of nvs the settings of CARD are kept under - `s` for the board's own (the tab Settings), `s.<card>` for
/// a module's.
///
/// **A KEY A CARD, AND NOT ONE FOR ALL.** Under one key every module's save rewrote every other module's values,
/// and the set grew with every module into one entry read through one buffer (355 bytes on 23.09, about 520 with
/// one more module's, against a buffer of 1024). A card's settings still go down in one write, so a card is never half
/// new.
pub fn key_of(card: Option<&str>) -> String {
    match card {
        None => "s".to_string(),
        Some(card) => format!("s.{card}"),
    }
}

/// The keys of LIST, each once, in the order of the list.
fn keys(list: &[&Setting]) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for key in list.iter().map(|s| key_of(s.card)) {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

/// Where the names the list no longer has are kept when the one set is split - a module taken off, a setting
/// renamed, a field of an older description. Their values are a person's, and a new image has no business erasing
/// what it does not know.
///
/// **A KEY OF ITS OWN, AND NOT THE ONE SET LEFT STANDING.** While the one set stands, every setting is read from it
/// (`value`) - that is what makes a cut in the middle of the split harmless - so a set kept for the sake of a few
/// names nobody reads would go on shadowing every card's key for good. Written before the one set is removed, it is
/// one more write of the same run: a cut before the removal starts the split again from the beginning.
pub const LEFT: &str = "s-left";

/// The key a module's own memory is kept under - what it writes down for itself and reads back after a restart,
/// a deciding module's state for one.
///
/// `m.<card>`, beside the settings' `s` and `s.<card>` and never one of them: a module writing its memory must not
/// replace a person's settings with a line of its own.
pub fn memory_key(card: &str) -> String {
    format!("m.{card}")
}

/// How many writes the store has taken since the board came up, told apart by whose they are: a person's
/// settings, or a module's memory (`memory_key`).
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Writes {
    pub settings: u32,
    pub memory: u32,
}

impl Writes {
    /// Counts one write to KEY - a put or a removal.
    pub fn count(&mut self, key: &str) {
        match key.starts_with(&memory_key("")) {
            true => self.memory += 1,
            false => self.settings += 1,
        }
    }

    /// The lines of `status`, one a writer.
    pub fn lines(&self) -> [String; 2] {
        [
            format!("settings written to flash: {} since start", self.settings),
            format!("modules' memory written to flash: {} since start", self.memory),
        ]
    }
}

/// The same, refused in words for a card whose key nvs would not take - at most 15 bytes.
pub fn memory_key_checked(card: &str) -> Result<String, String> {
    let key = memory_key(card);
    match key.len() <= 15 {
        true => Ok(key),
        false => Err(format!("the card '{card}' makes the key '{key}', longer than the 15 bytes nvs takes")),
    }
}

/// Splits the one set a board kept under KEY into a key per card; the lines of what it did, for the log.
///
/// **THE ONE SET IS REMOVED LAST, AND ITS REMOVAL IS THE MOMENT THE SPLIT HAPPENS.** Every card's key is written
/// first, each from the one set; only then is the one set taken away, in one write. A cut anywhere before that
/// leaves the one set standing - and while it stands it is what every setting reads (`value`) - so the next start
/// splits it again from the beginning and ends in the same place. A cut cannot leave a card read from a half.
///
/// Names the list no longer has go under LEFT with their values, and are said.
pub fn split(list: &[&Setting], store: &mut dyn Store) -> Vec<String> {
    let Some(text) = store.get(KEY) else { return Vec::new() };
    let whole = parse(&text);
    let meaning = whole.iter().find(|(n, _)| n == MEANING_IN_SET).cloned();
    for key in keys(list) {
        let mut set: Vec<(String, String)> = whole
            .iter()
            .filter(|(n, _)| find(list, n).is_some_and(|s| key_of(s.card) == key))
            .cloned()
            .collect();
        if set.is_empty() {
            continue;
        }
        set.extend(meaning.clone());
        if let Err(why) = store.put(&key, &unparse(&set)) {
            return vec![format!("settings not split: {why} - they stay in the one set, and the next start splits them")];
        }
    }
    let left: Vec<(String, String)> =
        whole.iter().filter(|(n, _)| n != MEANING_IN_SET && find(list, n).is_none()).cloned().collect();
    if !left.is_empty() {
        // what an earlier split kept stays, and a name kept again takes the one set's value
        let mut kept: Vec<(String, String)> =
            parse(&store.get(LEFT).unwrap_or_default()).into_iter().filter(|(n, _)| !left.iter().any(|(l, _)| l == n)).collect();
        kept.extend(left.iter().cloned());
        if let Err(why) = store.put(LEFT, &unparse(&kept)) {
            return vec![format!("settings not split: {why} - they stay in the one set, and the next start splits them")];
        }
    }
    if let Err(why) = store.remove(KEY) {
        return vec![format!("settings split, and the one set not removed: {why} - the next start splits it again")];
    }
    let mut lines = vec![format!("settings split out of the one set into a key a card: {}", keys(list).join(", "))];
    if !left.is_empty() {
        let names: Vec<&str> = left.iter().map(|(n, _)| n.as_str()).collect();
        lines.push(format!("settings this image no longer has kept with their values under {LEFT}: {}", names.join(", ")));
    }
    lines
}

fn parse(text: &str) -> Vec<(String, String)> {
    text.lines().filter_map(|l| l.split_once('=')).map(|(n, v)| (n.to_string(), v.to_string())).collect()
}

fn unparse(set: &[(String, String)]) -> String {
    set.iter().map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join("\n")
}

/// One choice of a setting: the value kept, the words a person reads, and - for a choice that takes a word of its
/// own, like a server's name - what that word is.
pub struct Choice {
    pub value: &'static str,
    pub words: &'static str,
    pub takes: Option<&'static str>,
    /// What the written-in word has to be, for a choice that takes one; None - any word a person can read back.
    /// The rule belongs to the choice, not to the code that checks settings: a server's name is a host, and a
    /// sensor's name is whatever the person calls it.
    pub checks: Option<fn(&str) -> bool>,
    /// What choosing it means now, in one line - said under a switch for the choice that is set; None for a choice of
    /// a list, whose words already say it.
    pub means: Option<&'static str>,
}

/// What a setting takes.
pub enum Kind {
    Choice(&'static [Choice]),
    /// A whole number of months.
    Months { from: u32, to: u32 },
    /// A span of time in whole seconds, read by a person in minutes and seconds: "5 min", "1 min 30 s".
    Seconds { from: u32, to: u32 },
    /// A span of time in seconds with one digit after the point at most - a dose of the auger, 0.8 s - kept as a person
    /// typed it ("2", "0.8"); FROM and TO are in tenths of UNIT: of a second for the times of the auger ("s"), of a degree for a target of the room ("°C").
    Tenths { from: u32, to: u32, unit: &'static str },
    /// A whole number that is a number and nothing else - a module's address on a fieldbus, say. UNIT is what
    /// stands beside it for a person; "" when the number speaks for itself.
    Whole { from: u32, to: u32, unit: &'static str },
}

/// Where a setting stands on its card: a property of the setting, and the page only draws it. Most settings
/// are `Work`; `Tuning` is folded away under them on the card that has both.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    /// What a person turns from day to day.
    Work,
    /// What is set once, when the device is fitted out.
    Tuning,
}

/// One setting.
pub struct Setting {
    /// Its key in nvs and its word in the console: lowercase letters, digits and _, at most 15 bytes. Never shown.
    pub name: &'static str,
    /// What it is, for a person.
    pub label: &'static str,
    /// Its value when nothing is stored and the device's description names none.
    pub default: &'static str,
    pub kind: Kind,
    /// Where it stands on its card: what a person turns from day to day, or what is turned when the device is fitted out.
    pub level: Level,
    /// The card of the module whose setting it is, on the device's own page; None - the board's, on the tab Settings.
    pub card: Option<&'static str>,
}

/// Where the time comes from: automatically - the router, and the internet when the router is silent -, only from the
/// internet, or from one server a person names.
pub const TIME: Setting = Setting {
    name: "time",
    label: "Time comes from",
    default: "auto",
    kind: Kind::Choice(&[
        Choice { value: "auto", words: "auto", takes: None, checks: None, means: Some("the router, or the internet when it is silent") },
        Choice { value: "internet", words: "internet", takes: None, checks: None, means: Some("the internet only (pool.ntp.org)") },
        Choice {
            value: "server",
            words: "server",
            takes: Some("the server's name or address"),
            checks: Some(host),
            means: Some("the server written below"),
        },
    ]),
    level: Level::Work,
    card: None,
};

// ---- a station that cannot join ----

/// How many joins in a row may fail at one access point before the station leaves it for the next strongest of the network.
pub const NET_POINT_TRIES: Setting = Setting {
    name: "net_point_tries",
    label: "Failed joins in a row before the next access point",
    default: "2",
    kind: Kind::Whole { from: 1, to: 20, unit: "" },
    level: Level::Work,
    card: None,
};

/// How many laps over the network's access points may end without a join before the radio is reset.
pub const NET_RESET_LAPS: Setting = Setting {
    name: "net_reset_laps",
    label: "Laps without a network before the radio is reset",
    default: "2",
    kind: Kind::Whole { from: 1, to: 20, unit: "" },
    level: Level::Work,
    card: None,
};

/// How many laps more after the radio's reset before the board restarts.
pub const NET_REBOOT_LAPS: Setting = Setting {
    name: "net_reboot_laps",
    label: "Laps after the reset before the board restarts",
    default: "2",
    kind: Kind::Whole { from: 1, to: 20, unit: "" },
    level: Level::Work,
    card: None,
};

/// The time zone, named by its standard (winter) time as it is written everywhere: "GMT+2" is Eastern European time, which moves to +3 in summer by itself, "GMT+3" does not move, "UTC" does not either.
/// The POSIX rule behind each is in RULES. The first stands where the owner lives, and a board that never chose comes up on it.
pub const ZONE: Setting =
    Setting { name: "zone", label: "Time zone", default: "gmt+2", kind: Kind::Choice(&ZONES), level: Level::Work, card: None };

const ZONES: [Choice; 5] = [
    Choice { value: "gmt+2", words: "GMT+2", takes: None, checks: None, means: None },
    Choice { value: "gmt+1", words: "GMT+1", takes: None, checks: None, means: None },
    Choice { value: "gmt", words: "GMT", takes: None, checks: None, means: None },
    Choice { value: "gmt+3", words: "GMT+3", takes: None, checks: None, means: None },
    Choice { value: "utc", words: "UTC", takes: None, checks: None, means: None },
];

/// A value a setting once offered and offers no more: a store that kept it is still understood, and it means what it was renamed to. NAME is the setting's, OLD what was kept, NEW what a person is shown now.
/// The zones were cities once; the city stands for the zone it was in.
const RENAMED: [(&str, &str, &str); 5] = [
    ("zone", "kyiv", "gmt+2"),
    ("zone", "riga", "gmt+2"),
    ("zone", "warsaw", "gmt+1"),
    ("zone", "london", "gmt"),
    ("zone", "istanbul", "gmt+3"),
];

/// VALUE of the setting called NAME as it is meant today: a value that was renamed is its new name, anything else as it is.
fn renamed_in<'a>(name: &str, value: &'a str) -> &'a str {
    RENAMED.iter().find(|(setting, old, _)| *setting == name && *old == value).map_or(value, |(_, _, new)| new)
}

/// VALUE as S means it today.
fn renamed<'a>(s: &Setting, value: &'a str) -> &'a str {
    renamed_in(s.name, value)
}

/// The POSIX rule of each zone of ZONES, as newlib's tzset reads it: the offset in POSIX is the one to ADD to local time to get UTC, so "EET-2" is UTC+2, and a second name (EEST) is the summer time that moves it an hour.
const RULES: [(&str, &str); 5] = [
    ("gmt+2", "EET-2EEST,M3.5.0/3,M10.5.0/4"),
    ("gmt+1", "CET-1CEST,M3.5.0,M10.5.0/3"),
    ("gmt", "GMT0BST,M3.5.0/1,M10.5.0"),
    ("gmt+3", "<+03>-3"),
    ("utc", "UTC0"),
];

/// The settings every board has - the clock's and the station's - in the order the page draws them.
pub const ENGINE: [&Setting; 5] = [&TIME, &ZONE, &NET_POINT_TRIES, &NET_RESET_LAPS, &NET_REBOOT_LAPS];

/// Most servers of time: CONFIG_LWIP_SNTP_MAX_SERVERS of sdkconfig.defaults.
pub const MAX_SERVERS: usize = 3;

/// Where the internet's time is asked.
const POOL: &str = "pool.ntp.org";
/// How a server named by a person is kept.
const SERVER: &str = "server:";

/// The value TEXT means for S, as it is kept - a choice by its value or its word, a server's name as "server:<name>",
/// a number of months -, or why the text is none, in words.
pub fn check(s: &Setting, text: &str) -> Result<String, Refusal> {
    let text = text.trim();
    match &s.kind {
        Kind::Months { from, to } => match text.parse::<u32>() {
            Ok(n) if (*from..=*to).contains(&n) && n.to_string() == text => Ok(text.to_string()),
            _ => Err(Refusal::NotMonths { name: s.name, text: text.to_string(), from: *from, to: *to }),
        },
        Kind::Seconds { from, to } => match text.parse::<u32>() {
            Ok(n) if (*from..=*to).contains(&n) && n.to_string() == text => Ok(text.to_string()),
            _ => Err(Refusal::NotSeconds { name: s.name, text: text.to_string(), from: *from, to: *to }),
        },
        Kind::Tenths { from, to, unit } => match tenths_of(text) {
            Some(n) if (*from..=*to).contains(&n) => Ok(text.to_string()),
            _ => Err(Refusal::NotTenths { name: s.name, text: text.to_string(), from: *from, to: *to, unit }),
        },
        // the text has to BE the number: "2 " and "0x02" name the same value and are not the same setting, and a
        // value that comes back written otherwise than it went in is a value nobody can compare
        Kind::Whole { from, to, .. } => match text.parse::<u32>() {
            Ok(n) if (*from..=*to).contains(&n) && n.to_string() == text => Ok(text.to_string()),
            _ => Err(Refusal::NotWhole { name: s.name, text: text.to_string(), from: *from, to: *to }),
        },
        Kind::Choice(choices) => {
            let lower = text.to_ascii_lowercase();
            let lower = renamed(s, &lower);
            if let Some(c) = choices.iter().find(|c| c.value == lower && c.takes.is_none()) {
                return Ok(c.value.to_string());
            }
            match choices.iter().find(|c| c.takes.is_some()) {
                // The choice that takes a word of its own. Its value is the prefix the word is kept behind
                // ("server:time.google.com", "own:buffer tank"), and what the word has to be is the choice's own
                // rule - a host for a server, anything readable for a name.
                Some(c) => {
                    let prefix = format!("{}:", c.value);
                    // A choice with a rule of its own is recognised without the prefix - "time.google.com" is a
                    // server's address and nothing else. A choice WITHOUT one has to be written behind it, or a
                    // misspelt role would be taken for a name a person meant: "tank" is not a sensor called
                    // tank, it is "hopper" typed wrong.
                    let typed = match (text.strip_prefix(&prefix), c.checks) {
                        (Some(behind), _) => behind.trim(),
                        (None, Some(_)) => text.trim(),
                        (None, None) => return Err(none_of(s, text, choices, Some(c.value))),
                    };
                    let good = match c.checks {
                        Some(check) => check(&typed.to_ascii_lowercase()) && typed.to_ascii_lowercase() != "gateway",
                        // no rule of its own: any word a person can read back, and nothing that would break the
                        // prefix it is kept behind
                        None => !typed.is_empty() && typed.len() <= 32 && !typed.contains(':') && typed.chars().all(|ch| !ch.is_control()),
                    };
                    if !good {
                        return Err(none_of(s, text, choices, Some(c.value)));
                    }
                    let kept = match c.checks {
                        Some(_) => typed.to_ascii_lowercase(),
                        None => typed.to_string(),
                    };
                    Ok(format!("{prefix}{kept}"))
                }
                None => Err(none_of(s, text, choices, None)),
            }
        }
    }
}

/// The refusal of TEXT for the choice S: none of the values CHOICES carry, OWN naming the one that takes a word of its own.
fn none_of(s: &Setting, text: &str, choices: &[Choice], own: Option<&'static str>) -> Refusal {
    Refusal::NoneOf { name: s.name, text: text.to_string(), choices: choices.iter().map(|c| c.value).collect(), own }
}

/// The most choices a setting may offer and still be drawn as a switch: one press sets it, no Save.
pub const SWITCH_UP_TO: usize = 3;

/// Why S, a choice of up to SWITCH_UP_TO, cannot stand as a switch, in words naming it; None when it can or is no such
/// choice.
pub fn switch_refusal(s: &Setting) -> Option<String> {
    let Kind::Choice(choices) = &s.kind else { return None };
    if choices.len() > SWITCH_UP_TO {
        return None;
    }
    let long: Vec<&str> = choices.iter().filter(|c| c.words.split_whitespace().count() > 2).map(|c| c.value).collect();
    let silent: Vec<&str> = choices.iter().filter(|c| c.means.map_or(true, str::is_empty)).map(|c| c.value).collect();
    let mut why = Vec::new();
    if !long.is_empty() {
        why.push(format!("more than two words on {}", long.join(", ")));
    }
    if !silent.is_empty() {
        why.push(format!("nothing said of what {} means", silent.join(", ")));
    }
    (!why.is_empty()).then(|| format!("{}: a switch of {} choices with {}", s.label, choices.len(), why.join("; ")))
}

/// What a person reads of the kept value STORED of S.
pub fn words(s: &Setting, stored: &str) -> String {
    match &s.kind {
        Kind::Months { .. } => format!("{stored} months"),
        Kind::Seconds { .. } => span_words(stored.parse().unwrap_or(0)),
        Kind::Tenths { unit, .. } => format!("{stored} {unit}"),
        Kind::Whole { unit, .. } => format!("{stored} {unit}").trim_end().to_string(),
        Kind::Choice(choices) => match stored.split_once(':') {
            // a choice that took a word of its own shows that word, not the key it is kept behind: a person reads
            // back what they wrote
            Some((behind, written)) if choices.iter().any(|c| c.value == behind && c.takes.is_some()) => {
                match behind {
                    "server" => format!("from the server {written}"),
                    _ => written.to_string(),
                }
            }
            _ => {
                let stored = renamed(s, stored);
                choices.iter().find(|c| c.value == stored).map_or(stored.to_string(), |c| c.words.to_string())
            }
        },
    }
}

/// The tenths of a second TEXT says: "2" is 20, "0.8" is 8, "30.1" is 301; None for what is no such number - a sign, a
/// second point, no digit before or after the point, more than one digit after it, a comma, an exponent. A value that
/// comes back written otherwise than it went in is a value nobody can compare, so the text has to BE the number.
pub fn tenths_of(text: &str) -> Option<u32> {
    let (whole, tenth) = match text.split_once('.') {
        Some((whole, tenth)) if tenth.len() == 1 => (whole, tenth),
        Some(_) => return None,
        None => (text, "0"),
    };
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || !digits(tenth) || (whole.len() > 1 && whole.starts_with('0')) {
        return None;
    }
    whole.parse::<u32>().ok()?.checked_mul(10)?.checked_add(tenth.parse().ok()?)
}

/// TENTHS of a second as the text a person types: "2" for 20, "0.8" for 8.
pub fn tenths_text(tenths: u32) -> String {
    if tenths % 10 == 0 {
        (tenths / 10).to_string()
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

/// SECONDS as a person reads a span: "5 min", "1 min 30 s", "45 s".
pub fn span_words(seconds: u32) -> String {
    match (seconds / 60, seconds % 60) {
        (0, s) => format!("{s} s"),
        (m, 0) => format!("{m} min"),
        (m, s) => format!("{m} min {s} s"),
    }
}

/// The servers SNTP asks for the kept time TIME, in order; `gateway` stands for the router the address came from.
pub fn servers(time: &str) -> Vec<String> {
    match time.strip_prefix(SERVER) {
        Some(name) => vec![name.to_string()],
        None if time == "internet" => vec![POOL.to_string()],
        None => vec!["gateway".to_string(), POOL.to_string()],
    }
}

/// The POSIX rule of the kept zone ZONE, a zone kept under a city's name being the offset that city had; UTC for a zone the list has not.
pub fn posix(zone: &str) -> &'static str {
    let zone = renamed_in(ZONE.name, zone);
    RULES.iter().find(|(offset, _)| *offset == zone).map_or("UTC0", |(_, rule)| rule)
}

/// Whether newlib reads RULE as it is meant: POSIX TZ with the rules of summer time when there is one.
pub fn posix_ok(rule: &str) -> bool {
    posix_tz(rule).is_ok()
}


/// The setting called NAME in LIST.
pub fn find<'a>(list: &'a [&'a Setting], name: &str) -> Option<&'a Setting> {
    list.iter().copied().find(|s| s.name == name)
}

/// The keys settings were kept under one by one before the set went under KEY: erased when the set is first seeded,
/// so nothing reads them as settings again.
const BEFORE: [&str; 8] = ["time", "zone", "rows", "keep", "ntp", "tz", "csv_min", "csv_months"];

/// The set kept under KEY, name and value a pair each; empty when nothing is.
/// The set kept under KEY: name and value, in the order kept; none when nothing is.
pub fn kept(store: &dyn Store, key: &str) -> Vec<(String, String)> {
    store.get(key).map(|text| parse(&text)).unwrap_or_default()
}

/// Puts a card's whole set under its KEY in one write: nvs writes one entry whole or not at all, so a set is never
/// half new.
///
/// **THE MEANING OF THE SET IS CARRIED ACROSS EVERY WRITE THAT DOES NOT CARRY IT ITSELF.** [`change_all`]
/// rebuilds the set out of the list of settings, so a person saving anything at all would otherwise drop the
/// record's own field - and the next start would read a set with no meaning as one written before any of this
/// existed, and carry every value a second time. Only [`carry_over`] puts a meaning in the set it hands over;
/// everyone else gets the stored one put back.
///
/// **AND SO IS EVERY NAME THE KEY HOLDS THAT THE LIST NO LONGER HAS.** `change_all` builds a card's set out of the
/// settings of the list, so a setting a newer image took off the card - a module removed, a name changed - would be
/// dropped by the first save of that card, with its value, and nobody told. What the key holds and the new set does
/// not name is written back as it was.
fn keep_all(store: &mut dyn Store, key: &str, set: &[(String, String)]) -> Result<(), String> {
    let mut text: Vec<String> = set.iter().map(|(n, v)| format!("{n}={v}")).collect();
    for (name, was) in kept(store, key) {
        if !set.iter().any(|(n, _)| *n == name) {
            text.push(format!("{name}={was}"));
        }
    }
    store.put(key, &text.join("\n"))
}

/// The meaning SET was written under, if it says.
fn meaning_of(set: &[(String, String)]) -> Option<u32> {
    set.iter().find(|(n, _)| n == MEANING_IN_SET).and_then(|(_, v)| v.trim().parse().ok())
}

/// Seeds every setting of LIST that has nothing kept: with its value in DESCRIPTION when the device's description
/// names one, else with its default. A kept value is never touched - what a person set in place outlives a new image.
/// When the set is seeded the first time, the keys of the settings before it are erased. The lines of what was
/// seeded and erased, for the log.
pub fn seed(list: &[&Setting], rules: &Rules, store: &mut dyn Store, description: &[(&str, &str)]) -> Vec<String> {
    let mut lines = split(list, store);
    lines.extend(move_gone(list, store));
    lines.extend(bring_back(list, store));
    lines.extend(put_aside(list, store));
    let first = keys(list).iter().all(|key| store.get(key).is_none());
    for key in keys(list) {
        let new = store.get(&key).is_none();
        let mut set = kept(store, &key);
        let mut seeded = false;
        for s in list.iter().filter(|s| key_of(s.card) == key) {
            if set.iter().any(|(n, _)| n == s.name) {
                continue;
            }
            let (seed, from) = match description.iter().find(|(name, _)| *name == s.name) {
                Some((_, v)) => (*v, "the description"),
                None => (s.default, "the default"),
            };
            set.push((s.name.to_string(), seed.to_string()));
            lines.push(format!("setting {} seeded from {from}: {seed}", s.name));
            seeded = true;
        }
        // A SET BEING CREATED CARRIES THE PRESENT MEANING FROM ITS FIRST WRITE. Without it a new board would come
        // up with a set that says nothing about what it means, and the next start would read that as a board
        // written before any meaning existed - and carry defaults that are already in the present one.
        if new {
            set.push((MEANING_IN_SET.to_string(), rules.meaning.to_string()));
        }
        if seeded {
            if let Err(why) = keep_all(store, &key, &set) {
                lines.push(format!("settings of {key} not seeded: {why} - they read their defaults"));
            }
        }
    }
    if first {
        let erased: Vec<&str> = BEFORE.iter().copied().filter(|k| store.remove(k).unwrap_or(false)).collect();
        if !erased.is_empty() {
            lines.push(format!("settings kept one by one before erased: {}", erased.join(", ")));
        }
    }
    lines
}

/// Moves every name a card's key holds that LIST no longer has there - a setting a newer image took off its card, or
/// keeps on another card now - under LEFT with its value, as the split does for the one set; the lines of what it
/// moved, for the log. A setting moved to another card comes back out of LEFT under its new key ([`bring_back`]).
///
/// **LEFT IS WRITTEN FIRST, AND THE CARD'S KEY ONLY AFTER.** A cut between the two leaves the name in both, and the
/// next start moves it again and ends where one whole start ends; the other order could lose the value. A card's
/// key kept for a card the list has no setting of at all is not reached here: nvs is asked by key, and such a key
/// is named nowhere in this image.
fn move_gone(list: &[&Setting], store: &mut dyn Store) -> Vec<String> {
    let mut lines = Vec::new();
    for key in keys(list) {
        let set = kept(store, &key);
        let (gone, stays): (Vec<_>, Vec<_>) =
            set.into_iter().partition(|(n, _)| n != MEANING_IN_SET && find(list, n).is_none_or(|s| key_of(s.card) != key));
        if gone.is_empty() {
            continue;
        }
        let mut left: Vec<(String, String)> =
            kept(store, LEFT).into_iter().filter(|(n, _)| !gone.iter().any(|(g, _)| g == n)).collect();
        left.extend(gone.iter().cloned());
        if let Err(why) = store.put(LEFT, &unparse(&left)).and_then(|_| store.put(&key, &unparse(&stays))) {
            lines.push(format!("settings {key} holds that this image no longer has not moved: {why} - the next start moves them"));
            continue;
        }
        let names: Vec<&str> = gone.iter().map(|(n, _)| n.as_str()).collect();
        lines.push(format!("settings this image no longer has moved with their values from {key} to {LEFT}: {}", names.join(", ")));
    }
    lines
}

/// Whether WAS is a value of S as this image takes it - the test [`value`] reads a kept value by.
pub fn sound(s: &Setting, was: &str) -> bool {
    check(s, was).is_ok_and(|c| c == was)
}

/// Puts under LEFT every value a card's key holds for a setting of LIST that this image does not take - a bound
/// narrowed since it was kept -, so that the seeding after it writes the default in its place; the lines of what it put
/// aside, for the log. [`value`] already reads the default over such a value; kept in the card it would come back the
/// day the bound is widened, and a person reading the key would see a number the board does not burn by.
///
/// **LEFT IS WRITTEN FIRST, AND THE CARD'S KEY ONLY AFTER** - the order of [`move_gone`], for its reason.
fn put_aside(list: &[&Setting], store: &mut dyn Store) -> Vec<String> {
    let mut lines = Vec::new();
    for key in keys(list) {
        let set = kept(store, &key);
        let (aside, stays): (Vec<_>, Vec<_>) = set
            .into_iter()
            .partition(|(n, v)| find(list, n).is_some_and(|s| key_of(s.card) == key && !sound(s, v)));
        if aside.is_empty() {
            continue;
        }
        let mut left: Vec<(String, String)> =
            kept(store, LEFT).into_iter().filter(|(n, _)| !aside.iter().any(|(a, _)| a == n)).collect();
        left.extend(aside.iter().cloned());
        if let Err(why) = store.put(LEFT, &unparse(&left)).and_then(|_| store.put(&key, &unparse(&stays))) {
            lines.push(format!("settings of {key} this image does not take not put aside: {why} - they read their defaults"));
            continue;
        }
        let named: Vec<String> = aside.iter().map(|(n, v)| format!("{n}={v}")).collect();
        lines.push(format!("settings this image does not take put aside from {key} to {LEFT}, they read their defaults: {}", named.join(", ")));
    }
    lines
}

/// Brings back out of LEFT every setting of LIST that its card's key does not hold: a value a person set, put aside by
/// an image that did not know the setting, goes back to the first image that knows it again - what was changed in
/// place, an image does not touch (6.6, 25.09: an image without the roles of the sockets put them aside, and the next
/// one read them as free). A setting its key already holds keeps that value, and its older copy in LEFT is dropped.
/// The lines of what came back, for the log.
///
/// **THE CARDS' KEYS ARE WRITTEN FIRST, AND LEFT ONLY AFTER.** A cut between them leaves a value in both, and the next
/// start finds it in its key and drops the copy; the other order could lose it.
fn bring_back(list: &[&Setting], store: &mut dyn Store) -> Vec<String> {
    let left = kept(store, LEFT);
    if left.is_empty() {
        return Vec::new();
    }
    let mut taken: Vec<String> = Vec::new();
    let mut back: Vec<&str> = Vec::new();
    for key in keys(list) {
        let mut set = kept(store, &key);
        let mut brought = false;
        for s in list.iter().filter(|s| key_of(s.card) == key) {
            let Some((_, was)) = left.iter().find(|(n, _)| n == s.name) else { continue };
            // a value this image does not take stays where [`put_aside`] left it: brought back, it would be put aside
            // again at every start, and dropped, nobody could see what the board held
            if !sound(s, was) {
                continue;
            }
            taken.push(s.name.to_string());
            if set.iter().any(|(n, _)| n == s.name) {
                continue;
            }
            set.push((s.name.to_string(), was.clone()));
            back.push(s.name);
            brought = true;
        }
        if brought {
            if let Err(why) = store.put(&key, &unparse(&set)) {
                return vec![format!("settings not brought back from {LEFT} to {key}: {why} - the next start brings them")];
            }
        }
    }
    if taken.is_empty() {
        return Vec::new();
    }
    let stays: Vec<(String, String)> = left.into_iter().filter(|(n, _)| !taken.contains(n)).collect();
    let written = match stays.is_empty() {
        true => store.remove(LEFT).map(|_| ()),
        false => store.put(LEFT, &unparse(&stays)),
    };
    let mut lines = Vec::new();
    if !back.is_empty() {
        lines.push(format!("settings this image knows again brought back from {LEFT} with their values: {}", back.join(", ")));
    }
    if let Err(why) = written {
        lines.push(format!("{LEFT} not cleared of what came back: {why} - the next start clears it"));
    }
    lines
}

/// The meaning the settings of the engine itself carry: TIME, ZONE and NET_* have meant the same since the first set that
/// stamped one. A device whose settings have a history of their own - a value that changed its unit - says so in its [`Rules`].
pub const ENGINE_MEANING: u32 = 4;

/// Where the meaning is kept: INSIDE the set, under a name no setting can have.
///
/// **THE SET AND ITS MEANING ARE ONE WRITE, BECAUSE TWO WRITES CANNOT BOTH BE CERTAIN TO HAPPEN.** Kept
/// beside the set, the meaning went down in a second write - and a device is switched off by its power, which
/// is an ordinary way to stop it rather than an accident. A cut landing between the two left the carried
/// numbers in nvs under the OLD meaning, and the next start carried them again: 30 to 150 to 750, into the
/// top of the span. A regulator five times too gentle became one five times too fierce.
///
/// nvs writes one entry whole or not at all, so with the meaning inside the entry a cut leaves the board
/// entirely in the old state or entirely in the new one, and the carry can only ever happen once.
///
/// The name begins with '@', which no setting's name may contain (lowercase letters, digits and _), so it
/// can never collide with one. What made this look impossible before is that [`change_all`] rebuilds the set
/// out of the list of settings and would drop it - and that is now a rule of [`keep_all`] instead: what
/// rewrites a person's settings does not touch the record's own field.
const MEANING_IN_SET: &str = "@meaning";

/// One setting whose stored value has to be carried across a change of meaning.
pub struct Carry {
    /// Its name, as it is kept.
    pub name: &'static str,
    /// The meaning this carry brings a value UP TO: it runs when what is stored is older than this.
    pub to: u32,
    /// The old value in, the new one out; None when it cannot be carried and the default has to be taken.
    pub carry: fn(&Setting, &str) -> Option<String>,
}

/// What a device knows about its own settings that the mechanism cannot: the history of what they meant, and which
/// choices need something else in the set. **THE MECHANISM KEEPS, CHECKS AND CARRIES; THE DEVICE SAYS WHAT** - a gain that
/// changed its unit, a mode that needs a sensor on a socket are a device's, and a board without that device has none of
/// them. Every call that carries or changes a set is handed one.
pub struct Rules {
    /// What the device's set means now, as [`ENGINE_MEANING`] says of the engine's: a number that goes up by one whenever
    /// a value stops meaning what it meant. A set is stamped with it in its first write.
    pub meaning: u32,
    /// The values carried one by one when the meaning they were kept under is older than the carry's.
    pub carries: &'static [Carry],
    /// The cards whose key holds a set to carry although no carried value is on them.
    pub carried_cards: &'static [Option<&'static str>],
    /// What the carries above cannot say - values of the set that move together - for a set kept under meaning STORED, and
    /// the lines of what it did. Reads STORE for what an older image put aside ([`LEFT`]).
    pub carry_sets: fn(stored: u32, set: &mut Vec<(String, String)>, store: &dyn Store) -> Vec<String>,
    /// The settings whose choice is asked of [`Rules::refuse`] when a save changes it.
    pub modes: &'static [&'static str],
    /// Why the mode NAME may not be MODE, given the SET as the save leaves it; None when it may. Only a save that CHANGES a
    /// mode is asked: the tab sends every field, and a mode left as it stood is not a choice made now.
    pub refuse: fn(name: &'static str, mode: &str, set: &[(String, String)]) -> Option<Refusal>,
    /// What CHOICE of SETTING bears on, for the dictionary a client reads; None when it bears on nothing.
    pub bearing: fn(setting: &str, choice: &str) -> Option<&'static Bearing>,
    /// The settings that hold the role a choice needs (`Bearing::role`): the names of the sockets.
    pub among: &'static [&'static str],
}

/// What a choice of a setting bears on: what the device knows of its own choices, said once, for the page's dictionary and for the device's own refusal. A choice that has no entry bears on nothing.
#[derive(Clone, Copy, Debug)]
pub struct Bearing {
    /// The setting, and the choice of it.
    pub setting: &'static str,
    pub choice: &'static str,
    /// The settings of the same card that matter while this is the choice set. A setting that no choice of the setting names is in force
    /// whatever is chosen.
    pub shows: &'static [&'static str],
    /// The role a socket has to play for the choice to be taken: taken only while a socket measures that sensor ([`Rules::refuse`]). None - the choice needs no sensor.
    pub role: Option<&'static str>,
}

fn no_bearing(_setting: &str, _choice: &str) -> Option<&'static Bearing> {
    None
}

fn no_sets(_stored: u32, _set: &mut Vec<(String, String)>, _store: &dyn Store) -> Vec<String> {
    Vec::new()
}

fn no_refusal(_name: &'static str, _mode: &str, _set: &[(String, String)]) -> Option<Refusal> {
    None
}

/// The rules of a device whose settings are the engine's alone: nothing carried, nothing refused.
pub static NO_RULES: Rules = Rules { meaning: ENGINE_MEANING, carries: &[], carried_cards: &[], carry_sets: no_sets, modes: &[], refuse: no_refusal, bearing: no_bearing, among: &[] };

/// Brings values written under an older meaning up to the present one, and says what it did.
///
/// **CALLED BEFORE [`seed`], AND THE ORDER IS THE WHOLE OF HOW A NEW BOARD IS TOLD APART FROM AN OLD ONE.**
/// A board that has never had settings has no set at all: its defaults are written in the present meaning and
/// carrying them would multiply the seed by five. A board that HAS a set and no meaning beside it was tuned
/// before any of this existed, and every value in it is old. After `seed` the two look alike, because seeding
/// creates the set.
pub fn carry_over(list: &[&Setting], rules: &Rules, store: &mut dyn Store) -> Vec<String> {
    // THE ONE SET IS CARRIED WHERE IT LIES, before it is split - its meaning is one for all of it. After the split
    // every card's key carries its own.
    let holders: Vec<String> = match store.get(KEY) {
        Some(_) => vec![KEY.to_string()],
        None => {
            let mut keys: Vec<String> = Vec::new();
            for key in rules.carries.iter().filter_map(|c| find(list, c.name)).map(|s| key_of(s.card)).chain(rules.carried_cards.iter().map(|card| key_of(*card))) {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
            keys
        }
    };
    holders.iter().flat_map(|key| carry_over_at(list, rules, store, key)).collect()
}

fn carry_over_at(list: &[&Setting], rules: &Rules, store: &mut dyn Store, key: &str) -> Vec<String> {
    let mut set = kept(store, key);
    // nothing kept at all: a new board, whose defaults are already in the present meaning. `seed` stamps it
    // when it writes the set, in the same write.
    if set.is_empty() {
        return Vec::new();
    }
    let stored = meaning_of(&set).unwrap_or(0);
    if stored >= rules.meaning {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for c in rules.carries.iter().filter(|c| c.to > stored) {
        let Some(s) = find(list, c.name) else { continue };
        let Some(slot) = set.iter_mut().find(|(n, _)| n == c.name) else { continue };
        let was = slot.1.clone();
        match (c.carry)(s, &was) {
            // a value the carry leaves as it was is not news, and is not said
            Some(new) if new == was => {}
            Some(new) => {
                slot.1 = new.clone();
                lines.push(format!("setting {} carried to meaning {}: {was} is now {new}", c.name, c.to));
            }
            None => {
                slot.1 = s.default.to_string();
                lines.push(format!(
                    "setting {} could not be carried from '{was}' to meaning {}: it reads its default {}",
                    c.name, c.to, s.default
                ));
            }
        }
    }
    lines.extend((rules.carry_sets)(stored, &mut set, &*store));
    // THE VALUES AND THE MEANING GO DOWN TOGETHER. Written apart, a cut between them leaves carried numbers
    // under the old meaning, and the next start carries them again.
    set.retain(|(n, _)| n != MEANING_IN_SET);
    set.push((MEANING_IN_SET.to_string(), rules.meaning.to_string()));
    if let Err(why) = keep_all(store, key, &set) {
        return vec![format!("settings not carried: {why} - they still mean what they used to")];
    }
    lines
}

/// The kept value of S now: the stored one, or its default when nothing sound is stored. While the one set a board
/// kept before is still there, it is what is read: `split` has not finished, and it alone is whole.
pub fn value(s: &Setting, store: &dyn Store) -> String {
    let key = match store.get(KEY) {
        Some(_) => KEY.to_string(),
        None => key_of(s.card),
    };
    kept(store, &key)
        .into_iter()
        .find(|(n, _)| n == s.name)
        .and_then(|(_, v)| check(s, &v).ok().filter(|c| *c == v || renamed(s, &v) == c))
        .unwrap_or_else(|| s.default.to_string())
}

fn named<'a>(list: &'a [&'a Setting], name: &str) -> Result<&'a Setting, Refusal> {
    find(list, name).ok_or_else(|| Refusal::NoSetting { name: name.to_string() })
}

/// Changes NAME of LIST to TEXT: stored, and what it is now in words - "Time comes from: only from the internet
/// (pool.ntp.org)"; or why not, and the old value stays.
pub fn change(list: &[&Setting], rules: &Rules, store: &mut dyn Store, name: &str, text: &str) -> Result<String, String> {
    let moved = change_all(list, rules, store, &[(name, text)]).map_err(|why| why.to_string())?;
    let s = named(list, name).map_err(|why| why.to_string())?;
    Ok(moved.first().map(|m| m.said(list)).unwrap_or_else(|| format!("{}: {}", s.label, words(s, &value(s, store)))))
}

/// The tab's one save: every pair checked first, and only when all are good the whole set written in one go - a
/// person reads what changed, a line each; one bad value, or a write the store refuses, changes nothing and says why.
pub fn change_all(list: &[&Setting], rules: &Rules, store: &mut dyn Store, pairs: &[(&str, &str)]) -> Result<Vec<Moved>, Refusal> {
    let mut set: Vec<(String, String)> = list.iter().map(|s| (s.name.to_string(), value(s, store))).collect();
    let mut said = Vec::new();
    let mut touched: Vec<String> = Vec::new();
    let mut modes_now: Vec<(&'static str, String)> = Vec::new();
    for (name, text) in pairs {
        let s = named(list, name)?;
        let new = check(s, text)?;
        let slot = set.iter_mut().find(|(n, _)| n == s.name).expect("every setting of the list is in the set");
        if slot.1 != new {
            if rules.modes.contains(&s.name) {
                modes_now.push((s.name, new.clone()));
            }
            slot.1 = new.clone();
            said.push(Moved { name: s.name, value: new.clone() });
            let key = key_of(s.card);
            if !touched.contains(&key) {
                touched.push(key);
            }
        }
    }
    if said.is_empty() {
        return Ok(said);
    }
    // A MODE THE DEVICE'S RULES REFUSE IS REFUSED BEFORE ANYTHING IS WRITTEN, and its rules read the set as this save leaves it, so a
    // save that gives a mode what it needs and picks the mode together is taken.
    for (name, mode) in modes_now {
        if let Some(why) = (rules.refuse)(name, &mode, &set) {
            return Err(why);
        }
    }
    // a board that still keeps the one set has it split before anything is written beside it
    split(list, store);
    // ONLY THE CARDS A CHANGE TOUCHED ARE WRITTEN, each whole: a save of one card's setpoint does not rewrite the
    // values of another. A save across two cards is two writes, and a cut between them leaves each card
    // whole - one new, one as it was.
    for key in touched {
        let card: Vec<(String, String)> = set
            .iter()
            .filter(|(n, _)| find(list, n).is_some_and(|s| key_of(s.card) == key))
            .cloned()
            .collect();
        keep_all(store, &key, &card).map_err(|detail| Refusal::NotWritten { detail })?;
    }
    Ok(said)
}

/// Checks the device's DESCRIPTION against LIST before it is built in: every name a setting, every value one it
/// takes as it is kept. The build refuses a description that fails, with the reason.
pub fn check_description(list: &[&Setting], description: &[(&str, &str)]) -> Result<(), String> {
    for (name, text) in description {
        let s = find(list, name).ok_or_else(|| {
            let names: Vec<&str> = list.iter().map(|s| s.name).collect();
            format!("the description names a setting '{name}' the firmware has not; it has: {}", names.join(", "))
        })?;
        let kept = check(s, text).map_err(|why| why.to_string())?;
        if kept != *text {
            return Err(format!("the description writes {name} as '{text}'; write it as it is kept: '{kept}'"));
        }
    }
    Ok(())
}

/// A host name or an IPv4 address: labels of letters, digits and -, none empty or longer than 63, none starting or
/// ending with -.
fn host(name: &str) -> bool {
    name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

/// POSIX TZ: std offset [dst [offset] ,rule,rule]. Daylight saving without its rules is refused: newlib would take
/// the rules of the USA for it, and the clock would turn on the wrong days without a word.
fn posix_tz(text: &str) -> Result<(), String> {
    let mut rest = text.as_bytes();
    zone_name(&mut rest).ok_or("has no zone name of 3 letters or <...>")?;
    offset(&mut rest, 24).ok_or("has no offset from UTC, hours 0-24")?;
    if rest.is_empty() {
        return Ok(());
    }
    zone_name(&mut rest).ok_or("has something after the offset that is not a zone name")?;
    if !rest.starts_with(b",") {
        offset(&mut rest, 24).ok_or("has no rules of the daylight saving: ,start,end")?;
    }
    for which in ["start", "end"] {
        rest = rest.strip_prefix(b",").ok_or("has no rules of the daylight saving: ,start,end")?;
        rule(&mut rest).ok_or_else(|| format!("has a {which} of the daylight saving that is not Mm.w.d, Jn or n"))?;
        if let Some(r) = rest.strip_prefix(b"/") {
            rest = r;
            offset(&mut rest, 167).ok_or_else(|| format!("has an hour of the {which} that is not one"))?;
        }
    }
    if !rest.is_empty() {
        return Err(format!("has '{}' after its end", String::from_utf8_lossy(rest)));
    }
    Ok(())
}

fn zone_name(rest: &mut &[u8]) -> Option<()> {
    let (len, skip) = if rest.first() == Some(&b'<') {
        let end = rest.iter().position(|&b| b == b'>')?;
        if !rest[1..end].iter().all(|b| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'-') {
            return None;
        }
        (end - 1, end + 1)
    } else {
        let len = rest.iter().take_while(|b| b.is_ascii_alphabetic()).count();
        (len, len)
    };
    (len >= 3).then(|| *rest = &rest[skip..])
}

/// [+-]hh[:mm[:ss]] with hours up to MAX_HOURS.
fn offset(rest: &mut &[u8], max_hours: u32) -> Option<()> {
    if let Some(r) = rest.strip_prefix(b"+").or_else(|| rest.strip_prefix(b"-")) {
        *rest = r;
    }
    let hours = number(rest, 1, 3)?;
    if hours > max_hours {
        return None;
    }
    for _ in 0..2 {
        let Some(r) = rest.strip_prefix(b":") else { break };
        *rest = r;
        if number(rest, 2, 2)? > 59 {
            return None;
        }
    }
    Some(())
}

/// Mm.w.d, Jn (1-365) or n (0-365).
fn rule(rest: &mut &[u8]) -> Option<()> {
    if let Some(r) = rest.strip_prefix(b"M") {
        *rest = r;
        let month = number(rest, 1, 2)?;
        *rest = rest.strip_prefix(b".")?;
        let week = number(rest, 1, 1)?;
        *rest = rest.strip_prefix(b".")?;
        let day = number(rest, 1, 1)?;
        ((1..=12).contains(&month) && (1..=5).contains(&week) && day <= 6).then_some(())
    } else if let Some(r) = rest.strip_prefix(b"J") {
        *rest = r;
        (1..=365).contains(&number(rest, 1, 3)?).then_some(())
    } else {
        (number(rest, 1, 3)? <= 365).then_some(())
    }
}

/// MIN to MAX digits, as a number.
fn number(rest: &mut &[u8], min: usize, max: usize) -> Option<u32> {
    let len = rest.iter().take(max).take_while(|b| b.is_ascii_digit()).count();
    if len < min {
        return None;
    }
    let n = core::str::from_utf8(&rest[..len]).ok()?.parse().ok()?;
    *rest = &rest[len..];
    Some(n)
}
