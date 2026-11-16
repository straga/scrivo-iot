//! The board's page contract, the one `docs/CONTRACT_Device_Api.md` names: which file answers for / and /icon.svg, what /state and /events say - every value by the address its
//! dictionary gives it (`dictionary`) -, and the answers of /restart, /down/<name>, /on/<address>, /off/<address>, /press/<address>, /toggle/<address> and /hold/<address>.
//! Nothing here touches the board; the firmware serves the answers and the modules hand their own values over.
//!
//! **A PART OF THE STATE IS ADDRESSED BY THE CARD IT LIES IN.** /state is the whole of it; an event on /events carries the cards that changed, so a page puts them into what it holds
//! instead of replacing everything (`Marks`, `Wanted`). Which is the point of the whole arrangement: a board that sent its whole self for one number off one sensor cost 7448 bytes ten times a
//! second to every open tab.

pub mod dictionary;

use serde::Serialize;
use serde_json::{json, Value};

/// Streams the page holds at once: a phone and a computer.
pub const MAX_STREAMS: usize = 2;
/// Subscribers the board keeps marks for at once: the open pages, and one place beside them for a subscriber
/// that is NOT a page - the sending outward.
///
/// Two lists on purpose. A stream is a socket with a worker task behind it and there is room for two; a
/// subscriber is a set of marks and costs four bytes, so the one that sends outward has no business competing
/// with a person's phone for a place.
pub const SUBSCRIBERS: usize = MAX_STREAMS + 1;

/// The number of the contract every answer of /state and /events keeps: what each thing is, the kind of its value, its
/// unit or its words, and the commands it answers to - `docs/CONTRACT_Device_Api.md`. Raised whenever a client reading
/// the old contract would read the new answer wrong; a client that reads another number says so instead of drawing.
pub const CONTRACT: u32 = 1;

/// The card of the things every board carries - the console, the station - and of what the board is: its name,
/// its image, its clock.
pub const SYSTEM: &str = "system";
/// The part that is the settings of the tab, all of them. Not a card of things: settings change when a person
/// presses save and at no other time, and they are two thirds of what a full state weighs.
pub const SETTINGS: &str = "settings";
/// The card a thing that names none of its own goes in - the page draws it under this very word.
pub const OTHER: &str = "other";

/// The parts every board has, before the device's own cards. Their order is their place in a set of marks, so
/// it is fixed: a card's bit must mean the same card on the board and on the wire.
const RESERVED: [&str; 3] = [SYSTEM, SETTINGS, OTHER];

/// Which parts of the state have changed: one bit each, and nothing allocated ever.
///
/// **A COUNTER CANNOT CARRY THIS, AND THAT IS WHY THERE IS NONE.** A change as one number bumped would mean:
/// every waiting stream wakes with no way of knowing what had moved, so the only thing left to send is the
/// whole board - 7448 bytes ten times a second while a motor spins, for one number off one sensor.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Marks(u32);

impl Marks {
    /// Nothing changed. What a subscriber takes on a tick when the board has been quiet.
    pub const NONE: Marks = Marks(0);
    /// Every part at once: what a subscriber starts with, so its first send is the whole state BY CONSTRUCTION
    /// rather than by a branch somewhere asking whether this is the first one.
    pub const ALL: Marks = Marks(u32::MAX);
    /// How many parts a set can tell apart.
    pub const PARTS: usize = u32::BITS as usize;

    /// The set of the one part at AT, or None for a place the set has not got. None rather than a wrap: a part
    /// folded onto a bit already spoken for would send one module's rows under another module's name.
    pub const fn part_at(at: usize) -> Option<Marks> {
        match at < Self::PARTS {
            true => Some(Marks(1 << at)),
            false => None,
        }
    }

    /// The set of the one part at AT. For a place known to be one: a literal in a test, an index already checked.
    pub const fn part(at: usize) -> Marks {
        match Self::part_at(at) {
            Some(one) => one,
            None => panic!("a part beyond the thirty-two a set of marks has"),
        }
    }

    /// Both sets. What two announcements before one send come to.
    pub const fn with(self, other: Marks) -> Marks {
        Marks(self.0 | other.0)
    }

    /// Whether anything at all is marked.
    pub const fn any(self) -> bool {
        self.0 != 0
    }

    /// Whether any part of ONE is in this set.
    pub const fn holds(self, one: Marks) -> bool {
        self.0 & one.0 != 0
    }

    /// Whether this is every part there is - the whole of the state rather than a piece of it.
    pub const fn everything(self) -> bool {
        self.0 == Self::ALL.0
    }

    /// This set without the parts of OTHER.
    pub const fn without(self, other: Marks) -> Marks {
        Marks(self.0 & !other.0)
    }

    /// The set as one word, for a keeper that has to hold it in an atomic (`events::Changes`).
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// The set a word stands for. Paired with [`Marks::bits`] and for nothing else: a bit here is a PLACE in a
    /// board's list of parts, and a number written by hand would mean whichever part happens to sit there.
    pub const fn of_bits(bits: u32) -> Marks {
        Marks(bits)
    }
}

/// What a reader is being sent: the board's parts by name, and which of them this answer is about.
///
/// **THE ADDRESS OF A PART IS THE CARD'S NAME, and it was there before this.** A thing already says which card
/// it lies in (`Thing::group`), the page already draws a card per group, and a device already names its cards
/// in its modules. So nothing had to be invented for a module to say WHICH of its own changed - the names are
/// unique where the names of things are not: `pin` belongs to two sensors both.
pub struct Wanted<'a> {
    /// The device's own cards, in the order its page draws them; the parts of RESERVED come before them.
    cards: &'a [&'static str],
    marks: Marks,
}

impl<'a> Wanted<'a> {
    /// Every part there is - what /state answers, and what a subscriber that has just opened is sent.
    pub const ALL: Wanted<'static> = Wanted { cards: &[], marks: Marks::ALL };

    pub const fn new(cards: &'a [&'static str], marks: Marks) -> Self {
        Self { cards, marks }
    }

    /// The parts NAMED, out of the board's CARDS. A name the board does not carry gives every part rather than
    /// none: a module announcing a card nobody declared must not quietly mean some other module's card, and
    /// sending too much is the only answer that cannot be wrong about what it says.
    pub fn of(cards: &'a [&'static str], named: &[&str]) -> Self {
        let marks = named.iter().fold(Marks::NONE, |marks, name| marks.with(mark_of(cards, name)));
        Self { cards, marks }
    }

    /// Whether this answer carries the part called NAME.
    pub fn has(&self, name: &str) -> bool {
        self.marks.holds(mark_of(self.cards, name))
    }

    /// Whether this is the whole of the state. A page merges a part into what it holds and replaces the whole,
    /// so the two must be told apart on the wire.
    pub fn everything(&self) -> bool {
        self.marks.everything()
    }

}

/// The parts of a board with these CARDS, in the order of their places in a set of marks.
fn parts<'a>(cards: &'a [&'static str]) -> impl Iterator<Item = &'static str> + 'a {
    RESERVED.into_iter().chain(cards.iter().copied())
}

/// The one part called NAME out of a board with these CARDS - or every part, for a name the board has not got
/// and for a board with more cards than a set of marks has places.
pub fn mark_of(cards: &[&'static str], name: &str) -> Marks {
    parts(cards)
        .position(|part| part == name)
        .and_then(Marks::part_at)
        .unwrap_or(Marks::ALL)
}
/// Below this the page warns against more tabs (__init__.py LOW_MEMORY).
pub const LOW_MEMORY: usize = 16384;
/// How often a subscriber is looked in on when nothing has changed, in ms. What goes out then is [`BEAT`] and
/// not the state: a board that repeated itself every five seconds to every open tab is the cost this avoids,
/// and what the tick is still FOR is finding a socket whose browser has gone.
pub const EVERY_MS: u64 = 5000;
/// How long the answer has to leave before the board restarts or goes down, in ms.
pub const GOING_MS: u64 = 500;
/// How long the page is without the board after a restart, as the page tells a person.
pub const BACK_IN_S: u32 = 20;
/// What a third stream gets with its 503.
pub const BUSY: &str = "the board is busy: two pages are open on it already - close one and reload this one";

/// How many bytes of an answer stand in memory at once while it goes out.
///
/// Small on purpose. What this buys is not bytes on the wire - those are the same - but the size of the piece
/// the HEAP has to give in one lump, and a heap with a page open has tens of kilobytes whole, not hundreds.
pub const PIECE: usize = 512;

/// Hands an answer out in pieces of at most [`PIECE`] bytes instead of building it whole.
///
/// **A BOARD FELL ON 21.09 BECAUSE AN ANSWER WAS BUILT WHOLE.** `/curve` went through `serde_json::to_value`
/// and then `to_string`: six hundred points became six hundred `Value`s in one `Vec`, then the text of them in
/// one growing `String`, and the lump the heap was asked for was bigger than the biggest lump it had - with
/// a hundred kilobytes still free, in pieces. Written through this, the cost of an answer is this buffer and
/// nothing else, whatever the answer's size.
///
/// The sink says whether it took the piece. `false` stops the answer there: writing on into a browser that
/// has gone away would leave the request looking answered, and every further piece is a copy for nothing.
pub struct Pieces<F> {
    send: F,
    buf: [u8; PIECE],
    filled: usize,
    refused: bool,
}

impl<F: FnMut(&[u8]) -> bool> Pieces<F> {
    pub fn new(send: F) -> Self {
        Self { send, buf: [0; PIECE], filled: 0, refused: false }
    }

    /// Hands over what is left and says whether every piece was taken.
    pub fn done(&mut self) -> bool {
        !self.refused && self.hand_over().is_ok()
    }

    fn hand_over(&mut self) -> std::io::Result<()> {
        if self.filled == 0 {
            return Ok(());
        }
        let taken = (self.send)(&self.buf[..self.filled]);
        self.filled = 0;
        if !taken {
            self.refused = true;
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "the piece was not taken"));
        }
        Ok(())
    }
}

impl<F: FnMut(&[u8]) -> bool> std::io::Write for Pieces<F> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.refused {
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "an earlier piece was not taken"));
        }
        let take = (PIECE - self.filled).min(data.len());
        self.buf[self.filled..self.filled + take].copy_from_slice(&data[..take]);
        self.filled += take;
        if self.filled == PIECE {
            self.hand_over()?;
        }
        Ok(take)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.hand_over()
    }
}

/// A curve as /curve answers it: how far apart the points are IN FACT, and the LINES of the picture.
///
/// THE STEP IS MEASURED AND NOT DECLARED. A module asks for a round every 100 ms and gets one every 109 - the
/// tick of the RTOS rounds a sleep up - and a curve drawn on the asked-for step would stretch a minute into
/// sixty-five seconds without a word.
///
/// **A LIST OF LINES AND NOT A FIXED PAIR.** It was `raw` and `value` - what the pin held and what that was
/// taken to mean - and two was all the shape allowed. A regulator needs three on one field: what was asked,
/// what came out, and the angle it is held by; without the third, "the regulator is twitching" and "the air is
/// breathing" look the same. The first line is what the picture is ABOUT,
/// and lines that share a UNIT share a scale - which is why there is no flag saying so.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Curve {
    pub step_ms: u32,
    pub lines: Vec<Line>,
}

/// One line of a curve: what it is for a person, its unit, the points oldest first, and the span the VALUE
/// ITSELF can take.
///
/// THE SPAN IS THE BOARD'S TO SAY, not the page's. Drawn to fit its own data, a curve looks the same whatever
/// it holds: eight pascals of noise fill the picture exactly as a motor spinning up from 160 to 1338 mV does, and
/// a person reading the shape - which is what a person reads - sees a storm either way and goes to fix what is
/// not broken. What the value CAN be is physics: the sensor's scale, the converter's ceiling, the range a gas
/// ever reaches. None for a line with no such limit, and then only the fitted view is offered.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Line {
    /// What this line is, in a word or two: "what the pin held", "asked", "by angle".
    pub words: String,
    pub unit: String,
    pub points: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<(f32, f32)>,
}

/// A board of contract 3 as its values: the version of its dictionary, its cards, and each value of the cards WANTED
/// handed over by address, one at a time, as it is made.
pub trait Values {
    fn version(&self) -> &str;
    fn cards(&self) -> &'static [&'static str];
    fn values(&self, wanted: &Wanted, each: &mut dyn FnMut(usize, dictionary::Said<'_>));
}

/// The values of the cards wanted as `[[address, value], ...]`, written straight from whatever hands them over.
struct ValuesOf<'a>(&'a dyn Values, &'a Wanted<'a>);

impl Serialize for ValuesOf<'_> {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = out.serialize_seq(None)?;
        let mut failed = None;
        self.0.values(self.1, &mut |at, value| {
            if failed.is_none() {
                if let Err(why) = seq.serialize_element(&(at, value)) {
                    failed = Some(why);
                }
            }
        });
        match failed {
            Some(why) => Err(why),
            None => seq.end(),
        }
    }
}

/// Writes /state of contract 3 straight into OUT: the contract, the version of the dictionary a reader checks before
/// it reads a single value, and every value by address. No name, no card, no word goes out with them.
pub fn write_values<W: std::io::Write>(out: W, board: &dyn Values) -> serde_json::Result<()> {
    use serde::ser::SerializeMap;
    struct State<'a>(&'a dyn Values);
    impl Serialize for State<'_> {
        fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
            let mut map = out.serialize_map(Some(3))?;
            map.serialize_entry("contract", &dictionary::CONTRACT)?;
            map.serialize_entry("dictionary", self.0.version())?;
            map.serialize_entry("values", &ValuesOf(self.0, &Wanted::ALL))?;
            map.end()
        }
    }
    serde_json::to_writer(out, &State(board))
}

/// Writes the answer to a command of contract 3: `{"values": [[address, value], ...]}` of the cards WANTED, those KEEP
/// says are of the row the command reached - the row and its parts.
pub fn write_values_of<W: std::io::Write>(out: W, board: &dyn Values, wanted: &Wanted, keep: &dyn Fn(usize) -> bool) -> serde_json::Result<()> {
    use serde::ser::{SerializeMap, SerializeSeq};
    struct Kept<'a>(&'a dyn Values, &'a Wanted<'a>, &'a dyn Fn(usize) -> bool);
    impl Serialize for Kept<'_> {
        fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
            let mut seq = out.serialize_seq(None)?;
            let mut failed = None;
            self.0.values(self.1, &mut |at, value| {
                if failed.is_none() && (self.2)(at) {
                    if let Err(why) = seq.serialize_element(&(at, value)) {
                        failed = Some(why);
                    }
                }
            });
            match failed {
                Some(why) => Err(why),
                None => seq.end(),
            }
        }
    }
    struct Answer<'a>(Kept<'a>);
    impl Serialize for Answer<'_> {
        fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
            let mut map = out.serialize_map(Some(1))?;
            map.serialize_entry("values", &self.0)?;
            map.end()
        }
    }
    serde_json::to_writer(out, &Answer(Kept(board, wanted, keep)))
}

/// What one reader of the stream has been sent: a print of each value by address, 0 for one never sent.
///
/// **ONE PER READER, MADE WHEN IT OPENS.** A board sends the cards a module marked, and a card of thirty rows is marked
/// every second while one clock in it counts down; what goes out is only the values whose print this reader has not
/// got. A reader that opens has none, so it gets everything - by construction, not by a branch asking whether it is the
/// first.
pub struct Sent {
    prints: Vec<u32>,
    /// Sent when the reader opens and never after, however it moves: the reader counts on from it.
    once: Vec<bool>,
    /// The counts of seconds that go on by themselves, and what the reader was last told of each.
    counters: Vec<Counter>,
    /// The board's clock at the send in hand, in milliseconds.
    now_ms: u64,
}

/// A count of seconds at an address, and the value the reader was last given with the time it was given.
struct Counter {
    at: usize,
    way: dictionary::Count,
    sent: Option<(u32, u64)>,
}

impl Counter {
    /// What the reader counts to NOW_MS from what it was told: up adds the seconds gone by, down takes them off down to nothing.
    fn expected(&self, now_ms: u64) -> Option<u32> {
        let (told, at) = self.sent?;
        let gone = u32::try_from(now_ms.saturating_sub(at).saturating_add(500) / 1000).unwrap_or(u32::MAX);
        Some(match self.way {
            dictionary::Count::Up => told.saturating_add(gone),
            dictionary::Count::Down => told.saturating_sub(gone),
        })
    }
}

impl Sent {
    pub fn new(addresses: usize) -> Self {
        Sent { prints: vec![0; addresses], once: vec![false; addresses], counters: Vec::new(), now_ms: 0 }
    }

    /// A reader of the table TABLE: what the table says is sent once, and what it says counts on by itself, is known from the start.
    pub fn of(table: &[dictionary::Entry]) -> Self {
        let mut sent = Sent::new(table.len());
        table.iter().enumerate().filter(|(_, e)| e.once).for_each(|(at, _)| sent.once(at));
        sent.counters = table.iter().enumerate().filter_map(|(at, e)| e.counts.map(|way| Counter { at, way, sent: None })).collect();
        sent
    }

    /// The board's clock, in milliseconds, at the send that follows: a count is judged against how long ago it was sent.
    pub fn at(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// The value at AT goes out once, when the reader opens.
    pub fn once(&mut self, at: usize) {
        if let Some(once) = self.once.get_mut(at) {
            *once = true;
        }
    }
}

/// The value as the entry says it is worth being sent: a number rounded to the digits the entry keeps, anything else as it is.
pub fn rounded<'a>(entry: &dictionary::Entry, said: dictionary::Said<'a>) -> dictionary::Said<'a> {
    use dictionary::{Said, Value};
    let Some(digits) = entry.decimals else { return said };
    match said {
        Said::Value(Value::Number(n)) => Said::Value(Value::Number(round_to(n, digits))),
        Said::Value(Value::AtLeast(n)) => Said::Value(Value::AtLeast(round_to(n, digits))),
        other => other,
    }
}

/// N to DIGITS places after the point; what rounds to nothing is 0 and never minus 0.
fn round_to(n: f32, digits: u8) -> f32 {
    let step = 10f32.powi(i32::from(digits));
    let r = (n * step).round() / step;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

/// What `?every=` of the request URI asks for the stream, in seconds between two sends: 0 - at every change (Live). Nothing asked, or
/// something that is no whole number of seconds up to [`dictionary::EVERY_MOST_S`], is the pace of everyone.
pub fn every_asked(uri: &str) -> u32 {
    let asked = uri.split_once('?').and_then(|(_, query)| query.split('&').find_map(|pair| pair.strip_prefix("every=")));
    match asked {
        Some(n) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => n.parse().ok().filter(|n| *n <= dictionary::EVERY_MOST_S),
        _ => None,
    }
    .unwrap_or(dictionary::STREAM_EVERY_S)
}

/// FNV-1a over what is written into it: a value's print, made without a byte of the heap.
struct Printer(u32);

impl std::io::Write for Printer {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.0 = data.iter().fold(self.0, |h, b| (h ^ u32::from(*b)).wrapping_mul(0x0100_0193));
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn print_of(value: &dictionary::Said<'_>) -> u32 {
    let mut printer = Printer(0x811c_9dc5);
    // a value that cannot be written has no print worth keeping: it is sent every time, and the send says why
    match serde_json::to_writer(&mut printer, value) {
        Ok(()) => printer.0.max(1),
        Err(_) => 0,
    }
}

/// Writes one server-sent event of contract 3 with the values of the cards WANTED that this reader - SENT - has not got
/// as they are now, and answers how many; a board where nothing changed writes nothing at all.
pub fn write_changed<W: std::io::Write>(mut out: W, board: &dyn Values, wanted: &Wanted, sent: &mut Sent) -> std::io::Result<usize> {
    let (mut written, mut failed) = (0, None);
    board.values(wanted, &mut |at, value| {
        if failed.is_some() {
            return;
        }
        let print = print_of(&value);
        let had = sent.prints.get(at).copied().unwrap_or(0);
        let once = sent.once.get(at).copied().unwrap_or(false);
        let counter = sent.counters.iter().position(|c| c.at == at);
        // a count the reader goes on with by itself is said when it is not where the reader has it - and said whenever the reader has none
        let by_count = match (counter, &value) {
            (Some(c), dictionary::Said::Value(dictionary::Value::Seconds(n))) => Some((c, *n)),
            _ => None,
        };
        let due = match by_count {
            Some((c, n)) => sent.counters[c].expected(sent.now_ms).is_none_or(|expected| n.abs_diff(expected) > 1),
            None => !(print != 0 && (had == print || (once && had != 0))),
        };
        if !due {
            return;
        }
        let head: &[u8] = if written == 0 { b"data: [" } else { b"," };
        let wrote = out
            .write_all(head)
            .and_then(|()| serde_json::to_writer(&mut out, &(at, value)).map_err(std::io::Error::other));
        match wrote {
            Ok(()) => {
                if let Some(slot) = sent.prints.get_mut(at) {
                    *slot = print;
                }
                if let Some(c) = counter {
                    let now = sent.now_ms;
                    sent.counters[c].sent = by_count.map(|(_, n)| (n, now));
                }
                written += 1;
            }
            Err(why) => failed = Some(why),
        }
    });
    if let Some(why) = failed {
        return Err(why);
    }
    if written > 0 {
        out.write_all(b"]\n\n")?;
    }
    Ok(written)
}

/// What a subscriber is sent on a tick when nothing has changed: an SSE comment, which every reader ignores.
///
/// Three bytes, and they are not decoration. A stream learns that a browser has gone by a write that fails,
/// and there are two places for streams: with nothing at all going out, a tab closed on a phone would hold one
/// of them for as long as the board runs.
pub const BEAT: &[u8] = b":\n\n";

/// The longest value a setting is sent, in bytes.
pub const MAX_VALUE: usize = 128;

/// The refusals of a save that the page's contract gives before the board is asked, each as `{"error": {"code": ..., ...}}`: a code with the fields it stands on, the words being the page's own.
pub mod refusal {
    use serde_json::{json, Value};

    /// The form names a setting this board has not.
    pub fn no_setting(name: &str) -> Value {
        json!({"error": {"code": "no_setting", "name": name}})
    }

    /// A value is longer than [`crate::MAX_VALUE`], GOT bytes.
    pub fn value_too_long(name: &str, got: usize) -> Value {
        json!({"error": {"code": "value_too_long", "name": name, "max": crate::MAX_VALUE, "got": got}})
    }

    /// The form carries no pair at all.
    pub fn nothing_to_save() -> Value {
        json!({"error": {"code": "nothing_to_save"}})
    }

    /// The form is longer than MAX bytes and was not read.
    pub fn form_too_long(max: usize) -> Value {
        json!({"error": {"code": "form_too_long", "max": max}})
    }
}

/// The tab's one save: BODY is the form as a browser sends it, name=value&..., and APPLY gets every pair of it - only
/// for settings TABLE has, with values no longer than MAX_VALUE - and answers what changed, or why not. **BOTH ARE
/// DATA, NOT WORDS**: what changed is each setting's name and the value kept, and a refusal is a code with the fields
/// it stands on (`{"error": {"code": "not_whole", "name": ..., "from": ..., "to": ...}}`); the label of a setting and
/// the phrase of a refusal are the page's own table, in the language the page speaks.
pub fn save(table: &[dictionary::Entry], body: &str, apply: impl FnOnce(&[(String, String)]) -> Result<Vec<Value>, Value>) -> Value {
    let mut pairs = Vec::new();
    for part in body.split('&').filter(|p| !p.is_empty()) {
        let (name, value) = part.split_once('=').unwrap_or((part, ""));
        let (name, value) = (form_decoded(name), form_decoded(value));
        if !has_setting(table, &name) {
            return refusal::no_setting(&name);
        }
        if value.len() > MAX_VALUE {
            return refusal::value_too_long(&name, value.len());
        }
        pairs.push((name, value));
    }
    if pairs.is_empty() {
        return refusal::nothing_to_save();
    }
    match apply(&pairs) {
        Ok(changed) => json!({"changed": changed}),
        Err(why) => json!({"error": why}),
    }
}

/// Whether TABLE, the device's dictionary, has a setting called NAME: a row of the settings card.
fn has_setting(table: &[dictionary::Entry], name: &str) -> bool {
    dictionary::address(table, SETTINGS, name).is_some()
}

/// A part of a form as a browser wrote it: + is a space, %XX a byte.
fn form_decoded(part: &str) -> String {
    name_in(&format!("/{}", part.replace('+', " ")), "/")
}

/// The answer of /state and of every event.
/// Which file answers for NAME and whether it is gzipped: a browser that takes gzip gets NAME.gz, a client that
/// says nothing of it, names something else or weighs gzip at q=0 gets the plain file - never a refusal.
pub fn served(folder: &str, name: &str, accept_encoding: &str) -> (String, bool) {
    for part in accept_encoding.split(',') {
        let (encoding, weight) = part.trim().split_once(';').unwrap_or((part.trim(), ""));
        if encoding.trim().eq_ignore_ascii_case("gzip") {
            let weight = weight.trim();
            if let Some(q) = weight.strip_prefix("q=") {
                if q.parse::<f32>().unwrap_or(0.0) == 0.0 {
                    break;
                }
            }
            return (format!("{folder}/{name}.gz"), true);
        }
    }
    (format!("{folder}/{name}"), false)
}

/// Down to the recovery image only when the request names this board exactly: Ok is the answer before going.
pub fn down(board: &str, said: &str) -> Result<Value, Value> {
    if said != board {
        return Err(json!({"error": format!(
            "the board goes down only when asked by its name, {board}; the request said '{said}'")}));
    }
    Ok(json!({"going": "recovery"}))
}

/// The answer before a restart.
pub fn restart() -> Value {
    json!({"going": "restart", "back_in_s": BACK_IN_S})
}

/// Where a counter's CSV files lie in vfs, one a month: in /data, what the board makes itself, which a kit's upload
/// neither writes nor takes off (tools/upload_kit.py).
pub const CSV_FOLDER: &str = "/data/energy";

/// The path in vfs of the CSV NAME asks for - "2026-09.csv", a month's file and nothing else; or why not.
pub fn csv_file(name: &str) -> Result<String, String> {
    let refused = || format!("no CSV is called '{name}': the files are named by month, 2026-09.csv");
    let b = name.as_bytes();
    let digits = |r: core::ops::Range<usize>| b.get(r).is_some_and(|d| d.iter().all(u8::is_ascii_digit));
    if b.len() != 11 || !digits(0..4) || b[4] != b'-' || !digits(5..7) || &b[7..] != b".csv" {
        return Err(refused());
    }
    let month: u8 = name[5..7].parse().map_err(|_| refused())?;
    if !(1..=12).contains(&month) {
        return Err(refused());
    }
    Ok(format!("{CSV_FOLDER}/{name}"))
}

/// The name after PREFIX in URI, without a query, percent-decoded as encodeURIComponent wrote it. An escape that
/// is not one stays as written; what the name then is gets judged by down() or toggle(). Another prefix - empty.
pub fn name_in(uri: &str, prefix: &str) -> String {
    let Some(rest) = uri.strip_prefix(prefix) else { return String::new() };
    let raw = rest.split('?').next().unwrap_or("").as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let escaped = (raw[i] == b'%' && i + 3 <= raw.len())
            .then(|| core::str::from_utf8(&raw[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()))
            .flatten();
        match escaped {
            Some(b) => {
                out.push(b);
                i += 3;
            }
            None => {
                out.push(raw[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What a file of the page IS, by its name - and None when the name is not a file of a page at all.
///
/// A browser refuses a stylesheet served as text/plain and will not run a script served as anything else, so the
/// type is not decoration. The None is the other half: the page folder is all a browser may ask for, and a name
/// with a slash or a dot-dot in it is not asking for a file of the page, it is asking to leave it.
pub fn mime(name: &str) -> Option<&'static str> {
    if name.is_empty() || name.contains('/') || name.contains("..") {
        return None;
    }
    // a gzipped file is the same file: the encoding travels as a header, not as a type
    let name = name.strip_suffix(".gz").unwrap_or(name);
    match name.rsplit_once('.')?.1 {
        "js" => Some("text/javascript; charset=utf-8"),
        "css" => Some("text/css; charset=utf-8"),
        "html" => Some("text/html; charset=utf-8"),
        "svg" => Some("image/svg+xml"),
        // the page's own table of texts (contract 3): the words it reads the board's codes by
        "json" => Some("application/json; charset=utf-8"),
        _ => None,
    }
}

// ---- the paths the board answers, and the order they are registered in ----

/// The method a path is registered with. A WebSocket is a GET: its handshake is one, and esp-idf-svc registers it
/// as such (esp-idf-svc/src/http/server.rs, ws_handler -> Method::Get).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Way {
    Get,
    Post,
}

/// One path the board's page server answers. The enum is the list: a path the network's loop (crates/web) serves
/// without a variant here is invisible to ORDER, and the compiler's match is what keeps the two together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Route {
    Index,
    Icon,
    Favicon,
    State,
    Restart,
    Down,
    Toggle,
    On,
    Off,
    Press,
    Hold,
    Settings,
    Energy,
    Curve,
    /// The log kept on flash (journal::kept), as text: what the device did in the last run, read after the fact.
    Journal,
    /// What every address of a board of contract 3 is (`dictionary`), served from the image.
    Dictionary,
    Console,
    Events,
    Rest,
}

impl Route {
    /// Every route there is, in the order they are written above - which says nothing about registration.
    pub const ALL: [Route; 19] = [
        Route::Index,
        Route::Icon,
        Route::Favicon,
        Route::State,
        Route::Restart,
        Route::Down,
        Route::Toggle,
        Route::On,
        Route::Off,
        Route::Press,
        Route::Hold,
        Route::Settings,
        Route::Energy,
        Route::Curve,
        Route::Journal,
        Route::Dictionary,
        Route::Console,
        Route::Events,
        Route::Rest,
    ];

    /// The template the route is registered under - a trailing `*` standing for any tail.
    pub fn uri(self) -> &'static str {
        match self {
            Route::Index => "/",
            Route::Icon => "/icon.svg",
            Route::Favicon => "/favicon.ico",
            Route::State => "/state",
            Route::Restart => "/restart",
            Route::Down => "/down/*",
            Route::Toggle => "/toggle/*",
            Route::On => "/on/*",
            Route::Off => "/off/*",
            Route::Press => "/press/*",
            Route::Hold => "/hold/*",
            Route::Settings => "/settings",
            Route::Energy => "/energy/*",
            Route::Curve => "/curve/*",
            Route::Journal => "/journal",
            Route::Dictionary => "/dictionary",
            Route::Console => CONSOLE,
            Route::Events => "/events",
            Route::Rest => "/*",
        }
    }

    pub fn way(self) -> Way {
        match self {
            Route::Restart
            | Route::Down
            | Route::Toggle
            | Route::On
            | Route::Off
            | Route::Press
            | Route::Hold
            | Route::Settings => Way::Post,
            _ => Way::Get,
        }
    }
}

/// Where the console's WebSocket is; /state names it as `terminal`, and the page shows its terminal by it.
pub const CONSOLE: &str = "/console";

/// THE ORDER crates/web LOOKS THEM UP IN, AND THE ORDER IS THE POINT: a request goes to the first route that answers
/// for its path and method. A wildcard put above a named path answers for it, and the named one is never reached -
/// the page without its console, the stream or the state. refused() is what says so before an image carries it to a
/// board.
pub const ORDER: [Route; 19] = [
    Route::Index,
    Route::Icon,
    Route::Favicon,
    Route::State,
    Route::Restart,
    Route::Down,
    Route::Toggle,
    Route::On,
    Route::Off,
    Route::Press,
    Route::Hold,
    Route::Settings,
    Route::Energy,
    Route::Curve,
    Route::Journal,
    Route::Dictionary,
    Route::Console,
    Route::Events,
    Route::Rest,
];

/// Whether TEMPLATE answers for URI: a trailing `*` stands for any tail, and a template without one answers for its
/// own name and nothing else. `?` is no wildcard here, and no route uses it (the tests hold that).
pub fn wildcard_match(template: &str, uri: &str) -> bool {
    match template.strip_suffix('*') {
        Some(head) => uri.starts_with(head),
        None => template == uri,
    }
}

/// Every route of ORDER that is never reached, each with the earlier route that swallows it - empty when every route
/// is: an earlier route whose template answers for the later one's uri with the same method takes all its requests.
pub fn refused(order: &[Route]) -> Vec<(Route, Route)> {
    order
        .iter()
        .enumerate()
        .filter_map(|(i, later)| {
            let swallowed = |earlier: &&Route| earlier.way() == later.way() && wildcard_match(earlier.uri(), later.uri());
            order[..i].iter().find(swallowed).map(|earlier| (*later, *earlier))
        })
        .collect()
}
