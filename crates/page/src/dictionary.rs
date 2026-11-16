//! What every address of a board IS, said once and for machines: the dictionary of the contract `CONTRACT` below names. A board
//! sends `[address, value]` and nothing else; the card, the short name, the kinds of value, the unit, the limits, the
//! commands and the codes a value is made of live here, built into the image and read by a client once.
//!
//! **NO PROSE, HERE OR IN THE STREAM.** Every phrase the board builds is a string in its heap,
//! for every open page, every second - what a device in service fell on. The words, the labels, the templates of a
//! phrase and the levels they are painted in are the client's: the page keeps its own table of texts in its kit,
//! Home Assistant its own translations. What is left here is what a client cannot guess: `flue_temp` is a number in
//! °C, `stage` is one of these codes.

use serde::Serialize;

/// The number of the contract the dictionary and the stream keep.
///
/// 3 since the dictionary says what the page used to know of the board by name: the state machine and its numbers (`machine`), the rows that count
/// down (`clock`), what a choice bears on (`shows`, `needs`), the rows of the card Now (`machine.now`) and what a setting's line is worked out
/// of (`effect`). A page that needs them and is given a dictionary of 2 says so and draws nothing (docs/CONTRACT_Device_Api.md).
///
/// 4 since the stream stops saying what a client can work out: a number goes out with the digits it needs (`decimals`), a count of seconds
/// goes out when it jumps and not every second (`counts`), and a reader asks how often it hears (`stream`). A client of 3 would read a count that
/// no longer arrives as a count that stands still.
///
/// 9 since the card `thermostat` has a third button, `light`, and the button `start` only switches on: it no longer lights the fire, the water does when it says `burn`, and `light` lights it at once.
/// A client of 8 would send `start` to light the fire.
///
/// 10 since a time of the auger is kept and said in tenths of a second: a setting of the kind `tenths` has its `from` and `to` in tenths, and its value is a text with one digit after the point at most.
/// A client of 9 would read "0.8" as a seconds value it cannot take.
pub const CONTRACT: u32 = 10;

/// How often a reader hears the board when it does not ask, in seconds: the stream is the board's and so is its pace.
pub const STREAM_EVERY_S: u32 = 5;
/// How long a reader that asked to hear every change (`every=0`, Live) keeps it, in seconds; then it hears at the pace of everyone.
pub const LIVE_S: u32 = 600;
/// The longest pace a reader may ask, in seconds.
pub const EVERY_MOST_S: u32 = 60;

/// What a value can be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    On,
    Number,
    AtLeast,
    Seconds,
    Word,
    Words,
    Unknown,
    Text,
    /// Only a command: no value, ever.
    Button,
}

/// A value as a board holds it and sends it.
///
/// **SHORT ON THE WIRE, BECAUSE THE KIND IS THE DICTIONARY'S.** A number is `61.5`, a switch `true`, a code `"kindling"`: the unit and the name of the kind stand in the dictionary once, not in
/// every record. Only what a reader could mistake for another kind carries its name: the top of a scale, the code of why there is no number.
///
/// **SMALL AND COPIED AS IT IS.** Its owner publishes it and every reader keeps a copy, so free text is not in
/// here: it would make every value the size of the longest text for the ten addresses that ever carry one. Text
/// travels as [`Said::Text`] from its own slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    /// Switched on or off.
    On(bool),
    /// A reading; its unit is the dictionary's.
    Number(f32),
    /// At least this: the top of a sensor's scale.
    AtLeast(f32),
    /// A count down in whole seconds.
    Seconds(u32),
    /// One code of the thing's list.
    Word(&'static str),
    /// Codes of the thing's list.
    Words(&'static [&'static str]),
    /// No number now, and the code of why, out of the thing's list.
    Unknown(&'static str),
    /// Nothing in this state.
    None,
}

/// What a board says at an address: a value, or words nobody could list beforehand - a refusal in the words of
/// whatever refused -, lent from the slot they are kept in. Sent only when it changes, like everything else.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Said<'a> {
    Value(Value),
    Text(&'a str),
}

impl From<Value> for Said<'_> {
    fn from(value: Value) -> Self {
        Said::Value(value)
    }
}

/// The longest free text an address carries: a setting's value, the longest the device has (`crate::MAX_VALUE`).
pub const LONGEST_TEXT: usize = crate::MAX_VALUE;

/// Free text in its slot, in place: a card its owner publishes is copied to every reader, and a copy that reached for
/// the heap would do it for every reader on every change.
///
/// Text longer than [`LONGEST_TEXT`] is cut where a character ends and closes with `…`, so what reaches the client
/// says it is not whole, and in the contract's own form, `{"text": "…"}`.
#[derive(Clone, Debug, PartialEq)]
pub struct Text(heapless::String<LONGEST_TEXT>);

/// What closes a cut text.
const CUT: char = '…';

impl Text {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Text {
    fn from(said: &str) -> Text {
        let mut text = heapless::String::new();
        if text.push_str(said).is_ok() {
            return Text(text);
        }
        let room = LONGEST_TEXT - CUT.len_utf8();
        let end = (0..=room).rev().find(|&at| said.is_char_boundary(at)).unwrap_or(0);
        // both fit by construction: END is at most ROOM, and ROOM leaves the mark its bytes
        let _ = text.push_str(&said[..end]);
        let _ = text.push(CUT);
        Text(text)
    }
}

/// A one-key object, `{"<name>": what}`.
fn named<S: serde::Serializer, T: Serialize + ?Sized>(out: S, name: &'static str, what: &T) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut map = out.serialize_map(Some(1))?;
    map.serialize_entry(name, what)?;
    map.end()
}

impl Serialize for Value {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::On(on) => out.serialize_bool(*on),
            Value::Number(n) => out.serialize_f32(*n),
            Value::Seconds(s) => out.serialize_u32(*s),
            Value::Word(w) => out.serialize_str(w),
            Value::Words(w) => w.serialize(out),
            Value::AtLeast(n) => named(out, "at_least", n),
            Value::Unknown(why) => named(out, "unknown", why),
            Value::None => out.serialize_unit(),
        }
    }
}

impl Serialize for Said<'_> {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        match self {
            Said::Value(value) => value.serialize(out),
            Said::Text(text) => named(out, "text", text),
        }
    }
}

/// One address of the dictionary: a row of a card, or a part of one - a number that belongs to a row and is no row
/// of its own (the spread of a pin, the address of a station).
///
/// Built with [`Entry::row`] and [`Entry::part`] and struct update, so a description writes only what it has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    pub card: &'static str,
    /// A short name, a code: `temp_in`, not "inlet temperature".
    pub name: &'static str,
    /// The row this is a part of, in the same card; None for a row.
    pub part_of: Option<&'static str>,
    /// The kinds its value can be.
    pub kinds: &'static [Kind],
    /// The unit of a number; empty for anything else.
    pub unit: &'static str,
    /// The codes a `word` or `words` value is made of.
    pub words: &'static [&'static str],
    /// The codes an `unknown` value can give.
    pub reasons: &'static [&'static str],
    /// The commands it takes.
    pub can: &'static [&'static str],
    /// The curves /curve draws of it, by name.
    pub curves: &'static [&'static str],
    /// The numbers a command may set it to; None - it takes none.
    pub range: Option<Range>,
    /// A number fixed in the image: the value is the dictionary's, and the board never sends it.
    pub fixed: Option<f32>,
    /// Sent when a reader opens and not after, however it moves: the reader counts on from it (the time a board has
    /// run).
    pub once: bool,
    /// It counts down, in seconds, to something that is about to happen: a client can say the nearest one that runs. A count of
    /// the time something has lasted is no clock.
    pub clock: bool,
    /// How many digits after the point a number is worth; None - as it is (a number the board does not round). The board rounds BEFORE it
    /// decides whether a value changed, so the stream sends a reading only when the rounded one moved.
    pub decimals: Option<u8>,
    /// A count of seconds that goes on by itself: up (the time a stage has lasted) or down (what is left to something). The board sends it when
    /// the reader opens and when it jumps - a new stage, a new target -, and the reader counts on from there.
    pub counts: Option<Count>,
    /// What its command does cannot be taken back (zeroing a count): a client asks the person before it sends the command. Said only by a row that takes one.
    pub irreversible: bool,
}

/// Which way a count of seconds goes on by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Count {
    Up,
    Down,
}

impl Entry {
    /// A row of CARD called NAME, its value one of KINDS, nothing else said yet.
    pub const fn row(card: &'static str, name: &'static str, kinds: &'static [Kind]) -> Entry {
        Entry { card, name, part_of: None, kinds, unit: "", words: &[], reasons: &[], can: &[], curves: &[], range: None, fixed: None, once: false, clock: false, decimals: None, counts: None, irreversible: false }
    }

    /// A part called NAME of the row ROW of CARD.
    pub const fn part(card: &'static str, row: &'static str, name: &'static str, kinds: &'static [Kind]) -> Entry {
        Entry { part_of: Some(row), ..Entry::row(card, name, kinds) }
    }

    fn who(&self) -> String {
        match self.part_of {
            Some(row) => format!("{}/{row}.{}", self.card, self.name),
            None => format!("{}/{}", self.card, self.name),
        }
    }
}

/// The numbers a command may set a thing to; where it stands now is the part AT of the same row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    pub min: u32,
    pub max: u32,
    pub step: u32,
    pub at: &'static str,
}

/// A dictionary as the image carries it: the JSON a client reads, its version, and the table the board's values are
/// addressed by - all three made of one table when the image was built.
#[derive(Clone, Copy)]
pub struct Served {
    pub json: &'static str,
    pub version: &'static str,
    pub table: &'static [Entry],
}

/// What a dictionary says besides its things: which device and image, its cards in their order, its settings, and - for a board that
/// has one - its state machine.
pub struct Meta<'a> {
    pub device: &'a str,
    pub board: &'a str,
    pub kit: &'a str,
    pub cards: &'a [&'a str],
    pub settings: &'a [Setting],
    pub machine: Option<&'a Machine>,
}

/// The state machine of a board as a client draws it: which of the board's cards is its, which of its rows say the stage it is in,
/// what it does and where a fault happened, what is pressed to run it, and which numbers it goes by in each stage. The words of all
/// this, the order of a stage's cells and what each stage means to a person are the client's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Machine {
    /// The card of its rows.
    pub card: &'static str,
    /// The card of the numbers the image fixes: they stand beside the settings in the table of what it goes by, and only a new image
    /// changes them.
    pub fixed: &'static str,
    /// The row that says the stage, the row that says what it does now - words of the stage's codes and the reasons for them - and the
    /// row that says the stage a fault happened in.
    pub stage: &'static str,
    pub says: &'static str,
    pub cause: &'static str,
    /// The rows pressed to run it, in the order they stand.
    pub press: &'static [&'static str],
    /// The rows of its card that are for something else than its list of rows: what it commands, for a drawing of the plant.
    pub aside: &'static [&'static str],
    /// The numbers it goes by, stage by stage.
    pub sections: &'static [Section],
    /// The rows of the card that sums up what it is doing, in the order they stand. Empty - the board has no such card.
    pub now: &'static [Shown],
}

/// One row of the card that sums up what the machine is doing: a row of a card, or - where which row it is depends on what the
/// board is set to, as a sensor that measures a liquid does - the row of a card whose part holds a code. The client's word for
/// it and how to show the number beside it are its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub card: &'static str,
    /// The row; empty where it is found by a part.
    pub row: &'static str,
    /// The part every row of the card has and the code it holds in the row that is meant; empty where the row is named.
    pub part: &'static str,
    pub is: &'static str,
    /// Another row of the card whose number is shown beside this one's word; empty - none.
    pub with: &'static str,
}

impl Shown {
    /// The row ROW of CARD.
    pub const fn row(card: &'static str, row: &'static str) -> Shown {
        Shown { card, row, part: "", is: "", with: "" }
    }

    /// The row of CARD whose part PART holds the code IS.
    pub const fn found_by(card: &'static str, part: &'static str, is: &'static str) -> Shown {
        Shown { card, row: "", part, is, with: "" }
    }

    /// The same, with the number of the row WITH shown beside its word.
    pub const fn beside(self, with: &'static str) -> Shown {
        Shown { with, ..self }
    }
}

/// What a setting's line - the words a client says of what turning it does - is worked out of, said by the board: the kind of
/// working out the client knows, and for each place in it the setting or the number of a row it takes. The words, and what the client chooses to
/// show (the size of the miss a line is said for), are the client's; a client that does not know the kind says no line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    pub kind: &'static str,
    pub uses: &'static [(&'static str, Use)],
}

/// What a place in an [`Effect`] takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Use {
    /// A setting of the board, by name.
    Setting(&'static str),
    /// A number, the part PART of the row ROW of the card CARD.
    Number(&'static str, &'static str, &'static str),
}

/// The numbers one stage of the state machine goes by: each item a setting, or a number the image fixes, by name - or several of them a
/// client says in one line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Section {
    pub name: &'static str,
    /// The setting whose choice decides which of this stage's numbers stand: the ones its choices list (`shows`) only while a choice that
    /// lists them is the one set. Empty - the stage's numbers stand whatever is chosen anywhere.
    pub by: &'static str,
    pub items: &'static [&'static [&'static str]],
}

/// A setting as a machine needs it: its name, the card it lies in (None - the board's own), what it is when nothing
/// is kept, and what it takes. Its label, the words of its choices and what they mean are the client's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setting {
    pub name: &'static str,
    pub card: Option<&'static str>,
    pub default: &'static str,
    pub shape: Shape,
    /// Folded away under the card's everyday settings (`crates/settings`, `Level::Tuning`); false - an everyday one, and then
    /// the dictionary says nothing of it, so a device with no folded settings serves the dictionary it served before.
    pub tuning: bool,
    /// What its line is worked out of; None - it has no line, and then the dictionary says nothing of it.
    pub effect: Option<Effect>,
}

/// One choice of a setting as a machine needs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variant {
    pub value: &'static str,
    /// It takes a word of its own, kept as `<value>:<word>`.
    pub takes: bool,
    /// The settings of the same card that matter while this is the choice set; empty - the choice says nothing of them, and then
    /// the dictionary says nothing either.
    pub shows: &'static [&'static str],
    /// The choice is offered only while one of the settings holds a value; None - it is always offered.
    pub needs: Option<Needs>,
}

impl Variant {
    /// A choice that bears on nothing.
    pub const fn plain(value: &'static str, takes: bool) -> Variant {
        Variant { value, takes, shows: &[], needs: None }
    }
}

/// What a choice needs to be offered: one of AMONG - settings of the board - holding HOLDS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Needs {
    pub among: &'static [&'static str],
    pub holds: &'static str,
}

/// What a setting takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// One of these values.
    Choice(&'static [Variant]),
    Months { from: u32, to: u32 },
    Seconds { from: u32, to: u32 },
    /// A number with a tenth, in UNIT (`s` for the times of the auger, `°C` for a target); FROM and TO are in tenths of UNIT.
    Tenths { from: u32, to: u32, unit: &'static str },
    Whole { from: u32, to: u32, unit: &'static str },
}

/// FIRST and THEN as one table: the rows every board has before a device's own. A table is a `const` and an address
/// too, so this runs when the image is built.
pub const fn joined<const N: usize>(first: &[Entry], then: &[Entry]) -> [Entry; N] {
    assert!(first.len() + then.len() == N, "the joined table is not as long as its two halves");
    let mut out = [Entry::row("", "", &[]); N];
    let mut i = 0;
    while i < first.len() {
        out[i] = first[i];
        i += 1;
    }
    let mut j = 0;
    while j < then.len() {
        out[i + j] = then[j];
        j += 1;
    }
    out
}

const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The address of the row NAME of CARD, worked out when the image is built: **A MODULE WRITES ITS VALUES BY THESE,
/// AND A ROW THE DESCRIPTION HAS NOT GOT STOPS THE BUILD** - the board cannot send a value nobody described.
pub const fn at(entries: &[Entry], card: &str, name: &str) -> usize {
    let mut i = 0;
    while i < entries.len() {
        if entries[i].part_of.is_none() && same(entries[i].card, card) && same(entries[i].name, name) {
            return i;
        }
        i += 1;
    }
    panic!("the description has no such row")
}

/// How many addresses CARD has - its rows and their parts -, worked out when the image is built: the size of the card
/// its module publishes.
pub const fn addresses(entries: &[Entry], card: &str) -> usize {
    let (mut i, mut n) = (0, 0);
    while i < entries.len() {
        if same(entries[i].card, card) {
            n += 1;
        }
        i += 1;
    }
    n
}

/// The address of the part NAME of the row ROW of CARD, the same way.
pub const fn part_at(entries: &[Entry], card: &str, row: &str, name: &str) -> usize {
    let mut i = 0;
    while i < entries.len() {
        if let Some(of) = entries[i].part_of {
            if same(of, row) && same(entries[i].card, card) && same(entries[i].name, name) {
                return i;
            }
        }
        i += 1;
    }
    panic!("the description has no such part")
}

/// The address of the row NAME of CARD; None when the description has no such row.
pub fn address(entries: &[Entry], card: &str, name: &str) -> Option<usize> {
    entries.iter().position(|e| e.part_of.is_none() && e.card == card && e.name == name)
}

/// The address of the part NAME of the row at ROW; None when the row has no such part.
pub fn part(entries: &[Entry], row: usize, name: &str) -> Option<usize> {
    let row = entries.get(row)?;
    entries.iter().position(|e| e.card == row.card && e.part_of == Some(row.name) && e.name == name)
}

/// A command a client sends to an address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Switch(bool),
    Toggle,
    Press,
    Hold(u32),
}

/// The command WHAT - `on`, `off`, `toggle`, `press`, `hold` - to the row at ADDRESS as a client sent them, BODY the
/// number a hold carries: the address and the command, or why not in words, before any module is asked. A part is no
/// thing a command reaches; a number outside a slider's range is refused here, as the page's slider never sends one.
pub fn command(entries: &[Entry], what: &str, address: &str, body: &str) -> Result<(usize, Command), String> {
    let at: usize = address.parse().map_err(|_| format!("'{address}' is no address of this board"))?;
    let entry = entries.get(at).filter(|e| e.part_of.is_none()).ok_or_else(|| format!("no thing of this board has the address {at}"))?;
    let (needed, command) = match what {
        "on" => ("switch", Command::Switch(true)),
        "off" => ("switch", Command::Switch(false)),
        "toggle" => ("toggle", Command::Toggle),
        "press" => ("press", Command::Press),
        "hold" => ("hold", Command::Hold(0)),
        other => return Err(format!("{other} is no command; the commands are on, off, toggle, press and hold")),
    };
    if !entry.can.contains(&needed) {
        let takes = if entry.can.is_empty() { "no command".to_string() } else { entry.can.join(", ") };
        return Err(format!("{} takes no {needed}: it takes {takes}", entry.name));
    }
    if command == Command::Hold(0) {
        let value: u32 = body.trim().parse().map_err(|_| format!("{} takes a whole number", entry.name))?;
        if let Some(range) = entry.range {
            if value < range.min || value > range.max {
                return Err(format!("{} takes {} to {}; {value} is not one of them", entry.name, range.min, range.max));
            }
        }
        return Ok((at, Command::Hold(value)));
    }
    Ok((at, command))
}

/// The FNV-1a print of BYTES: what tells two dictionaries apart when their images carry the same version.
pub fn print(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(*b)).wrapping_mul(0x0100_0193))
}

/// Whether WORD is a code: `[a-z][a-z0-9_]*`, what a machine reads and no person has to.
fn a_code(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Whether ENTRIES hold together: each row once in its card, each part under a row of its card, names and codes that
/// are codes, a unit for every number, a number that stands by a part of its row. Every refusal names the thing.
pub fn check(entries: &[Entry]) -> Result<(), String> {
    let mut refused = Vec::new();
    for (at, entry) in entries.iter().enumerate() {
        let who = entry.who();
        if entries[..at].iter().any(|e| e.card == entry.card && e.name == entry.name && e.part_of == entry.part_of) {
            refused.push(format!("{who} is described twice"));
        }
        if let Some(row) = entry.part_of {
            if address(entries, entry.card, row).is_none() {
                refused.push(format!("{who} is a part of {row}, and {} has no such row", entry.card));
            }
        }
        for code in [entry.card, entry.name].iter().chain(entry.words).chain(entry.reasons).chain(entry.can).chain(entry.curves) {
            if !a_code(code) {
                refused.push(format!("{who}: '{code}' is no code - a name or a code is [a-z][a-z0-9_]*, the words are the client's"));
            }
        }
        let counts = entry.kinds.iter().any(|k| matches!(k, Kind::Number | Kind::AtLeast));
        if counts && entry.unit.is_empty() {
            refused.push(format!("{who} holds a number and names no unit"));
        }
        if entry.decimals.is_some() && !counts {
            refused.push(format!("{who} says how many digits it keeps and holds no number"));
        }
        if entry.decimals.is_some_and(|n| n > 6) {
            refused.push(format!("{who} keeps more digits than an f32 has"));
        }
        if entry.counts.is_some() && entry.kinds != [Kind::Seconds] && entry.kinds != [Kind::Seconds, Kind::Unknown] {
            refused.push(format!("{who} counts and is no count of seconds"));
        }
        if entry.counts.is_some() && entry.once {
            refused.push(format!("{who} counts and is sent once: a count the reader goes on with is sent when it jumps, once is never again"));
        }
        if entry.fixed.is_some() && entry.kinds != [Kind::Number] {
            refused.push(format!("{who} is a number fixed in the image, and its value is said to be something else"));
        }
        if entry.irreversible && entry.can.is_empty() {
            refused.push(format!("{who} is irreversible and takes no command: there is nothing to ask before"));
        }
        if let Some(range) = entry.range {
            if part(entries, at, range.at).is_none() {
                refused.push(format!("{who} takes a number and stands by {}, and has no such part", range.at));
            }
        }
    }
    match refused.is_empty() {
        true => Ok(()),
        false => Err(refused.join("; ")),
    }
}

/// The dictionary as a client reads it: the contract, the version - the image and a print of everything else -, the
/// device, the cards in their order, and each address in its place. Made when the image is built, never on the board.
pub fn json(entries: &[Entry], meta: &Meta) -> String {
    use serde_json::{json, Map, Value as Json};
    let things: Vec<Json> = entries
        .iter()
        .enumerate()
        .map(|(at, e)| {
            let mut one = Map::new();
            one.insert("card".into(), json!(e.card));
            one.insert("name".into(), json!(e.name));
            if let Some(row) = e.part_of {
                one.insert("part_of".into(), json!(row));
            }
            one.insert("kinds".into(), json!(e.kinds));
            let mut put = |key: &str, value: Json, empty: bool| {
                if !empty {
                    one.insert(key.into(), value);
                }
            };
            put("unit", json!(e.unit), e.unit.is_empty());
            put("words", json!(e.words), e.words.is_empty());
            put("reasons", json!(e.reasons), e.reasons.is_empty());
            put("can", json!(e.can), e.can.is_empty());
            put("curves", json!(e.curves), e.curves.is_empty());
            if let Some(n) = e.fixed {
                put("fixed", json!(n), false);
            }
            put("once", json!(true), !e.once);
            put("clock", json!(true), !e.clock);
            if let Some(n) = e.decimals {
                put("decimals", json!(n), false);
            }
            if let Some(way) = e.counts {
                put("counts", json!(way), false);
            }
            put("irreversible", json!(true), !e.irreversible);
            if let Some(r) = e.range {
                put("range", json!({"min": r.min, "max": r.max, "step": r.step, "at": part(entries, at, r.at)}), false);
            }
            Json::Object(one)
        })
        .collect();
    let settings: Vec<Json> = meta
        .settings
        .iter()
        .map(|s| {
            let mut one = Map::new();
            one.insert("name".into(), json!(s.name));
            if let Some(card) = s.card {
                one.insert("card".into(), json!(card));
            }
            one.insert("default".into(), json!(s.default));
            let (kind, from, to, unit) = match s.shape {
                Shape::Choice(choices) => {
                    let choices: Vec<Json> = choices
                        .iter()
                        .map(|v| {
                            let mut one = Map::new();
                            one.insert("value".into(), json!(v.value));
                            if v.takes {
                                one.insert("takes".into(), json!(true));
                            }
                            if !v.shows.is_empty() {
                                one.insert("shows".into(), json!(v.shows));
                            }
                            if let Some(needs) = v.needs {
                                one.insert("needs".into(), json!({"among": needs.among, "holds": needs.holds}));
                            }
                            Json::Object(one)
                        })
                        .collect();
                    one.insert("choices".into(), json!(choices));
                    ("choice", None, None, "")
                }
                Shape::Months { from, to } => ("months", Some(from), Some(to), ""),
                Shape::Seconds { from, to } => ("seconds", Some(from), Some(to), ""),
                Shape::Tenths { from, to, unit } => ("tenths", Some(from), Some(to), unit),
                Shape::Whole { from, to, unit } => ("whole", Some(from), Some(to), unit),
            };
            one.insert("kind".into(), json!(kind));
            if s.tuning {
                one.insert("level".into(), json!("tuning"));
            }
            if let Some(effect) = s.effect {
                let uses: Map<String, Json> = effect
                    .uses
                    .iter()
                    .map(|(role, used)| {
                        let value = match *used {
                            Use::Setting(name) => json!(name),
                            Use::Number(card, row, part) => json!([card, row, part]),
                        };
                        (role.to_string(), value)
                    })
                    .collect();
                one.insert("effect".into(), json!({"kind": effect.kind, "uses": uses}));
            }
            if let (Some(from), Some(to)) = (from, to) {
                one.insert("from".into(), json!(from));
                one.insert("to".into(), json!(to));
            }
            if !unit.is_empty() {
                one.insert("unit".into(), json!(unit));
            }
            Json::Object(one)
        })
        .collect();
    let cards: Vec<&str> = [crate::SYSTEM, crate::SETTINGS, crate::OTHER].into_iter().chain(meta.cards.iter().copied()).collect();
    let mut body = json!({
        "contract": CONTRACT,
        "device": meta.device,
        "board": meta.board,
        "terminal": crate::CONSOLE,
        "cards": cards,
        "things": things,
        "settings": settings,
        "stream": {"every": STREAM_EVERY_S, "live_s": LIVE_S, "most": EVERY_MOST_S},
    });
    if let Some(m) = meta.machine {
        let sections: Vec<Json> = m.sections.iter().map(|s| {
            let mut one = json!({"name": s.name, "items": s.items});
            if !s.by.is_empty() {
                one["by"] = json!(s.by);
            }
            one
        }).collect();
        let now: Vec<Json> = m
            .now
            .iter()
            .map(|n| {
                let mut one = json!({"card": n.card});
                if n.row.is_empty() {
                    one["part"] = json!(n.part);
                    one["is"] = json!(n.is);
                } else {
                    one["row"] = json!(n.row);
                }
                if !n.with.is_empty() {
                    one["with"] = json!(n.with);
                }
                one
            })
            .collect();
        body["machine"] = json!({
            "card": m.card,
            "fixed": m.fixed,
            "stage": m.stage,
            "says": m.says,
            "cause": m.cause,
            "press": m.press,
            "aside": m.aside,
            "sections": sections,
        });
        if !now.is_empty() {
            body["machine"]["now"] = json!(now);
        }
    }
    let printed = format!("{:08x}", print(body.to_string().as_bytes()));
    body["version"] = json!(format!("{}+{printed}", meta.kit));
    body.to_string()
}
