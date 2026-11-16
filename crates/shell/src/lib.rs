//! The board's command shell. One table of commands, one instance per byte stream: the USB console and
//! every TCP connection each feed their bytes to their own `Shell`, and all of them act on the board
//! through the same `Board`.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use embedded_cli::arguments::Arg;
use embedded_cli::cli::{Cli, CliBuilder, CliHandle};
use embedded_cli::codes;
use embedded_cli::command::RawCommand;
use embedded_cli::service::{CommandProcessor, FromRaw, ParseError, ProcessError};
use embedded_cli::Command;

/// What a board's heap has left, in bytes: all of it, and the biggest piece it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    pub free: usize,
    pub largest: usize,
}

/// The biggest piece of heap `status` needs to be run: twice the working set of the report (the tasks and their
/// stacks, counted from the code, about 4 KB). Below it the shell does not ask the board for the report.
pub const STATUS_NEEDS: usize = 8192;

/// What the commands do on the board. The firmware implements it; tests implement it with a record.
pub trait Board {
    /// What the heap has left now; None for a board that cannot say, which is asked for every report.
    fn room(&self) -> Option<Room> {
        None
    }
    /// The facts `status` prints, one line each, handed to `line`.
    fn status(&mut self, line: &mut dyn FnMut(&str));
    /// Restarts the board.
    fn restart(&mut self);
    /// Makes the recovery image in factory the boot partition and restarts; Err says why it did not.
    fn down(&mut self) -> Result<(), String>;
    /// Switches the thing NAME over; Ok says what was done, Err why not. A board with nothing to switch refuses.
    fn toggle(&mut self, name: &str) -> Result<String, String> {
        Err(format!("this board switches nothing, and no {name}"))
    }
    /// Presses the button NAME; Ok says what was done, Err why not. A board with no buttons refuses.
    fn press(&mut self, name: &str) -> Result<String, String> {
        Err(format!("this board has no buttons, and no {name}"))
    }
    /// Turns the thing NAME on or off; Ok says what it is now, Err why not. A board with nothing to switch refuses.
    fn switch(&mut self, name: &str, _on: bool) -> Result<String, String> {
        Err(format!("this board switches nothing, and no {name}"))
    }
    /// What the board recorded about the last time it fell, one line each, handed to `line`; Err says why
    /// there is nothing to say. A board that has not fallen since its power came on has nothing, and that is
    /// not a fault of anything.
    fn fell(&mut self, _line: &mut dyn FnMut(&str)) -> Result<(), String> {
        Err("this board keeps no note of a fall".into())
    }
    /// Makes the board fall on purpose, so that the note can be read afterwards and the whole way proved
    /// without waiting for a fall nobody asked for. HARD takes the way a hardware fault takes; the other way
    /// is our code giving up. Does not return on a board that can do it.
    fn fall_now(&mut self, _hard: bool) {}
    /// The board's modules and how loud each one is, one line each, handed to `line`.
    fn loudness(&mut self, _line: &mut dyn FnMut(&str)) -> Result<(), String> {
        Err("this board keeps no levels of its own".into())
    }
    /// Turns MODULE up or down to LEVEL; Ok says what stands now, Err why not - which level words there are,
    /// or which modules the board has. Both refusals name the choices: a name typed wrong that answered "done"
    /// would leave a person watching a console that stays silent.
    fn set_loudness(&mut self, _module: &str, _level: &str) -> Result<String, String> {
        Err("this board keeps no levels of its own".into())
    }
    /// The board's settings with their values, one line each, handed to `line`; Err says why there are none.
    fn settings(&mut self, _line: &mut dyn FnMut(&str)) -> Result<(), String> {
        Err("this board has no settings".into())
    }
    /// Changes the setting NAME to VALUE; Ok says the value as stored, Err why not - the old value stays.
    fn set(&mut self, _name: &str, _value: &str) -> Result<String, String> {
        Err("this board has no settings".into())
    }
    /// The words this board adds to the shell's own, each with its line of help: a module's, brought
    /// with the module. The shell knows none of them; a board that has no such module does not list them, and the word is unknown on it. The
    /// shell asks once for each stream it serves, at its first byte: a board's words are fixed while it runs.
    fn words(&self) -> Vec<Word> {
        Vec::new()
    }
    /// Carries out the board's own word NAME, WORDS being what followed it; its lines go to `line`. None - NAME is not a word
    /// of this board, or nothing here takes WORDS after it; Some(Err) says why it was refused.
    fn word(&mut self, _name: &str, _words: &[&str], _line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
        None
    }
    /// Whether the board takes the shell's word NAME that not every board has - `toggle`, `press`, `on`, `off`: a board with
    /// nothing that turns over, is pressed or goes on and off does not list them, and they are unknown on it.
    fn takes(&self, _name: &str) -> bool {
        true
    }
}

/// A word a board adds to the shell's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub name: &'static str,
    /// One line for `help`.
    pub help: &'static str,
}

/// The shell's words that not every board has, asked of `Board::takes`.
pub const OPTIONAL: [&str; 4] = ["toggle", "press", "on", "off"];

// Lines end in "\n" alone: the cli's writer turns every "\n" into "\r\n" on the wire, so a "\r\n" written here goes out
// as "\r\r\n".

/// Longest line the shell takes, in bytes; a longer one is refused whole.
pub const MAX_LINE: usize = 64;
const HISTORY: usize = 128;
const PROMPT: &str = "rust> ";

// Parsing is ours (skip_from_raw): the derived one takes a variant without fields by its name alone and
// drops whatever follows, so "down now" would go down.
#[derive(Debug, Command)]
#[command(skip_from_raw)]
enum BoardCommand<'a> {
    /// Version, running partition, uptime, memory and network
    Status,
    /// Restart the board
    Restart,
    /// Boot the recovery image in factory
    Down,
    /// Switch a thing over: toggle <name>
    Toggle { name: &'a str },
    /// Press a button: press <name>
    Press { name: &'a str },
    /// Turn a thing on: on <name>
    On { name: &'a str },
    /// Turn a thing off: off <name>
    Off { name: &'a str },
    /// What the board recorded about the last time it fell: fell, or fell now to make it fall on purpose
    Fell { how: [Option<&'a str>; FELL_WORDS] },
    /// The modules and how loud each is; log <module> <level> turns one up
    Log { words: [Option<&'a str>; LOG_WORDS] },
    /// The settings and their values
    Settings,
    /// Change a setting: set time auto|internet|<server>, set zone kyiv, set <name> <value>
    Set { name: &'a str, value: String },
}

/// The most words a board's own word takes after its name: what to do and three numbers.
pub const WORD_WORDS: usize = 4;

/// The most words `fell` takes after its own name: `now hard`.
pub const FELL_WORDS: usize = 2;

/// The most words `log` takes after its own name: a module and a level.
pub const LOG_WORDS: usize = 2;

/// What `log` answers to anything but nothing or a module and a level. Half a command is not a command: a
/// module named without a level could mean "show me this one" as easily as "turn it up", and guessing between
/// them changes what the board does.
const LOG_TAKES: &str = "log takes nothing, or a module and a level: log alone lists the modules and how loud \
    each is; log <module> <level> turns one of them up";

/// What `fell` answers to a word it does not take. It names every form rather than the one word that was
/// wrong: a board is not to be made to fall by a typo, and `fell` alone is a question, not an order.
const FELL_TAKES: &str = "fell takes nothing, or now, or now hard. fell says what the board recorded about \
    its last fall; fell now makes it fall on purpose, our code giving up; fell now hard takes the way a \
    hardware fault takes";

/// The name of a thing the next argument gives; none, or an option, is refused.
fn thing<'a>(arg: Option<Result<Arg<'a>, embedded_cli::arguments::ArgError>>) -> Result<&'a str, ParseError<'a>> {
    match arg {
        Some(Ok(Arg::Value(name))) => Ok(name),
        None => Err(ParseError::MissingRequiredArgument { name: "name" }),
        Some(Ok(Arg::LongOption(name))) => Err(ParseError::UnexpectedLongOption { name }),
        Some(Ok(Arg::ShortOption(name))) => Err(ParseError::UnexpectedShortOption { name }),
        Some(Ok(_)) => Err(ParseError::UnexpectedArgument { value: "--" }),
        Some(Err(_)) => Err(ParseError::NonAsciiShortOption),
    }
}

/// The next argument as a plain value, None when there is none; an option or anything else is refused.
fn value<'a>(arg: Option<Result<Arg<'a>, embedded_cli::arguments::ArgError>>) -> Result<Option<&'a str>, ParseError<'a>> {
    match arg {
        None => Ok(None),
        Some(Ok(Arg::Value(v))) => Ok(Some(v)),
        Some(Ok(Arg::LongOption(name))) => Err(ParseError::UnexpectedLongOption { name }),
        Some(Ok(Arg::ShortOption(name))) => Err(ParseError::UnexpectedShortOption { name }),
        Some(Ok(_)) => Err(ParseError::UnexpectedArgument { value: "--" }),
        Some(Err(_)) => Err(ParseError::NonAsciiShortOption),
    }
}

impl<'a> FromRaw<'a> for BoardCommand<'a> {
    fn parse(raw: RawCommand<'a>) -> Result<Self, ParseError<'a>> {
        let mut args = raw.args().args();
        let command = match raw.name() {
            "status" => BoardCommand::Status,
            "restart" => BoardCommand::Restart,
            "down" => BoardCommand::Down,
            "log" => {
                let mut words = [None; LOG_WORDS];
                for word in words.iter_mut() {
                    *word = value(args.next())?;
                }
                BoardCommand::Log { words }
            }
            "fell" => {
                let mut how = [None; FELL_WORDS];
                for word in how.iter_mut() {
                    *word = value(args.next())?;
                }
                BoardCommand::Fell { how }
            }
            "settings" => BoardCommand::Settings,
            "set" => {
                let name = value(args.next())?.ok_or(ParseError::MissingRequiredArgument { name: "name" })?;
                // Everything after the name is the value, words and all: a setting may take a name a person
                // writes, and "own:buffer tank" is one value. Refusing the second word would leave such a setting
                // reachable from the page alone.
                let mut said = String::new();
                while let Some(word) = value(args.next())? {
                    if !said.is_empty() {
                        said.push(' ');
                    }
                    said.push_str(word);
                }
                if said.is_empty() {
                    return Err(ParseError::MissingRequiredArgument { name: "value" });
                }
                BoardCommand::Set { name, value: said }
            }
            "toggle" => BoardCommand::Toggle { name: thing(args.next())? },
            "press" => BoardCommand::Press { name: thing(args.next())? },
            "on" => BoardCommand::On { name: thing(args.next())? },
            "off" => BoardCommand::Off { name: thing(args.next())? },
            _ => return Err(ParseError::UnknownCommand),
        };
        match args.next() {
            None => Ok(command),
            Some(Ok(Arg::Value(value))) => Err(ParseError::UnexpectedArgument { value }),
            Some(Ok(Arg::LongOption(name))) => Err(ParseError::UnexpectedLongOption { name }),
            Some(Ok(Arg::ShortOption(name))) => Err(ParseError::UnexpectedShortOption { name }),
            Some(Ok(_)) => Err(ParseError::UnexpectedArgument { value: "--" }),
            Some(Err(_)) => Err(ParseError::NonAsciiShortOption),
        }
    }
}

/// The writer the cli owns, with a switch that swallows its output while the shell clears a refused line.
struct Out<W> {
    inner: W,
    mute: Arc<AtomicBool>,
}

impl<W: embedded_io::Write> embedded_io::ErrorType for Out<W> {
    type Error = W::Error;
}

impl<W: embedded_io::Write> embedded_io::Write for Out<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, W::Error> {
        if self.mute.load(Ordering::Relaxed) {
            return Ok(buf.len());
        }
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<(), W::Error> {
        self.inner.flush()
    }
}

pub struct Shell<W: embedded_io::Write> {
    cli: Cli<Out<W>, W::Error, [u8; MAX_LINE], [u8; HISTORY]>,
    mute: Arc<AtomicBool>,
    // Bytes typed into the current line: the cli's editor drops what does not fit without a word, and a
    // line cut to the buffer can read as another command ("down" and sixty spaces reads as "down").
    typed: usize,
    last: u8,
    // What the board adds and lacks, asked of it once: a board's words do not change while it runs, and asking on every byte
    // would build the list again for each character of a line. Set at the first byte, because only `feed` has the board.
    known: Option<Rc<Known>>,
}

impl<W: embedded_io::Write> Shell<W> {
    /// Returns None when the writer fails on the first prompt.
    pub fn new(writer: W) -> Option<Self> {
        let mute = Arc::new(AtomicBool::new(false));
        let cli = CliBuilder::default()
            .writer(Out { inner: writer, mute: mute.clone() })
            .command_buffer([0u8; MAX_LINE])
            .history_buffer([0u8; HISTORY])
            .prompt(PROMPT)
            .build()
            .ok()?;
        Some(Self { cli, mute, typed: 0, last: 0, known: None })
    }

    /// Takes one byte from the stream; a completed line runs its command on board. Err is the writer's.
    pub fn feed(&mut self, byte: u8, board: &mut dyn Board) -> Result<(), W::Error> {
        let enter = match byte {
            codes::CARRIAGE_RETURN => self.last != codes::LINE_FEED,
            codes::LINE_FEED => self.last != codes::CARRIAGE_RETURN,
            _ => false,
        };
        self.last = byte;
        if enter && self.typed > MAX_LINE {
            self.typed = 0;
            return self.refuse_line();
        }
        if enter {
            self.typed = 0;
        } else if byte == codes::BACKSPACE || byte == 0x7f {
            self.typed = self.typed.saturating_sub(1);
        } else if byte >= 0x20 {
            self.typed = self.typed.saturating_add(1);
        }
        let known = self.known.get_or_insert_with(|| Rc::new(Known::of(board))).clone();
        KNOWN.with(|slot| *slot.borrow_mut() = Some(known));
        self.cli.process_byte::<Listed, _>(byte, &mut Processor { board })
    }

    /// Empties the cli's line without showing it and says why: moves to the end, then erases back.
    fn refuse_line(&mut self) -> Result<(), W::Error> {
        self.mute.store(true, Ordering::Relaxed);
        let mut result = Ok(());
        for _ in 0..MAX_LINE {
            for &b in codes::CURSOR_FORWARD {
                result = result.and(self.cli.process_byte::<Listed, _>(b, &mut Ignore));
            }
        }
        for _ in 0..MAX_LINE {
            result = result.and(self.cli.process_byte::<Listed, _>(codes::BACKSPACE, &mut Ignore));
        }
        self.mute.store(false, Ordering::Relaxed);
        result?;
        self.cli.write(|w| {
            ufmt::uwrite!(w, "\nline longer than {} bytes refused\n{}", MAX_LINE, PROMPT)
        })
    }
}

/// A processor that does nothing: the bytes that clear a refused line are run through the cli's editor and no command comes of them.
struct Ignore;

impl<W: embedded_io::Write<Error = E>, E: embedded_io::Error> CommandProcessor<W, E> for Ignore {
    fn process<'a>(&mut self, _cli: &mut CliHandle<'_, W, E>, _raw: RawCommand<'a>) -> Result<(), ProcessError<'a, E>> {
        Ok(())
    }
}

/// Runs a line on the board: the shell's own words as parsed above, a word the board adds by the board.
struct Processor<'b> {
    board: &'b mut dyn Board,
}

impl<W: embedded_io::Write<Error = E>, E: embedded_io::Error> CommandProcessor<W, E> for Processor<'_> {
    fn process<'a>(&mut self, cli: &mut CliHandle<'_, W, E>, raw: RawCommand<'a>) -> Result<(), ProcessError<'a, E>> {
        match BoardCommand::parse(raw.clone()) {
            Ok(command) => run(cli, command, self.board).map_err(ProcessError::WriteError),
            // a word that is none of the shell's own is the board's, or nobody's: the cli says what it says of every word it does not know
            Err(ParseError::UnknownCommand) => self.board_word(cli, raw),
            // an optional word the board lacks is unknown with any arguments, or none: its missing or extra argument is not what to answer
            Err(_) if OPTIONAL.contains(&raw.name()) && !self.board.takes(raw.name()) => {
                unknown(cli.writer()).map_err(ProcessError::WriteError)
            }
            Err(other) => Err(ProcessError::ParseError(other)),
        }
    }
}

impl Processor<'_> {
    /// The board's own word on a line, its lines written as they come.
    fn board_word<'a, W: embedded_io::Write<Error = E>, E: embedded_io::Error>(
        &mut self,
        cli: &mut CliHandle<'_, W, E>,
        raw: RawCommand<'a>,
    ) -> Result<(), ProcessError<'a, E>> {
        let name = raw.name();
        if !KNOWN.with(|slot| slot.borrow().as_deref().is_some_and(|k| k.words.iter().any(|word| word.name == name))) {
            return unknown(cli.writer()).map_err(ProcessError::WriteError);
        }
        let mut words: Vec<&str> = Vec::new();
        for arg in raw.args().args() {
            match arg {
                Ok(Arg::Value(word)) if words.len() < WORD_WORDS => words.push(word),
                Ok(Arg::Value(word)) => return Err(ProcessError::ParseError(ParseError::UnexpectedArgument { value: word })),
                Ok(Arg::LongOption(option)) => return Err(ProcessError::ParseError(ParseError::UnexpectedLongOption { name: option })),
                Ok(Arg::ShortOption(option)) => return Err(ProcessError::ParseError(ParseError::UnexpectedShortOption { name: option })),
                Ok(_) => return Err(ProcessError::ParseError(ParseError::UnexpectedArgument { value: "--" })),
                Err(_) => return Err(ProcessError::ParseError(ParseError::NonAsciiShortOption)),
            }
        }
        let w = cli.writer();
        let mut failed = None;
        let said = self.board.word(name, &words, &mut |line| {
            if failed.is_none() {
                failed = w.write_str(line).and_then(|_| w.write_str("\n")).err();
            }
        });
        if let Some(e) = failed {
            return Err(ProcessError::WriteError(e));
        }
        let refused = match said {
            Some(Ok(())) => return Ok(()),
            Some(Err(why)) => why,
            None => format!("nothing on this board takes {}", words.first().copied().unwrap_or("it")),
        };
        let written = w.write_str(name).and_then(|_| w.write_str(" refused: ")).and_then(|_| w.write_str(&refused)).and_then(|_| w.write_str("\n"));
        written.map_err(ProcessError::WriteError)
    }
}

/// The shell's optional word a command is, if it is one.
fn optional_word(command: &BoardCommand<'_>) -> Option<&'static str> {
    match command {
        BoardCommand::Toggle { .. } => Some("toggle"),
        BoardCommand::Press { .. } => Some("press"),
        BoardCommand::On { .. } => Some("on"),
        BoardCommand::Off { .. } => Some("off"),
        _ => None,
    }
}

/// What the board a shell runs on adds and lacks: its words, and the optional words of the shell it does not take.
struct Known {
    words: Vec<Word>,
    lacks: Vec<&'static str>,
}

impl Known {
    fn of(board: &dyn Board) -> Self {
        Self { words: board.words(), lacks: OPTIONAL.into_iter().filter(|name| !board.takes(name)).collect() }
    }
}

thread_local! {
    /// The `Known` of the shell whose byte is being processed. Set by `feed` before every byte (a clone of a counted pointer, no
    /// list is built), because the cli asks the help and the completion without the board.
    static KNOWN: std::cell::RefCell<Option<Rc<Known>>> = const { std::cell::RefCell::new(None) };
}

/// The shell's own words with the one-line help each has, in the order `help` lists them (the doc lines of `BoardCommand`).
const OWN: [(&str, &str); 11] = [
    ("status", "Version, running partition, uptime, memory and network"),
    ("restart", "Restart the board"),
    ("down", "Boot the recovery image in factory"),
    ("toggle", "Switch a thing over: toggle <name>"),
    ("press", "Press a button: press <name>"),
    ("on", "Turn a thing on: on <name>"),
    ("off", "Turn a thing off: off <name>"),
    ("fell", "What the board recorded about the last time it fell: fell, or fell now to make it fall on purpose"),
    ("log", "The modules and how loud each is; log <module> <level> turns one up"),
    ("settings", "The settings and their values"),
    ("set", "Change a setting: set time auto|internet|<server>, set zone kyiv, set <name> <value>"),
];

/// The answer to a word the board does not have: it says so and names the words it has.
fn unknown<W: embedded_io::Write<Error = E>, E: embedded_io::Error>(w: &mut embedded_cli::writer::Writer<'_, W, E>) -> Result<(), E> {
    w.write_str("error: unknown command; the words of this board: ")?;
    for (at, (name, _)) in listed().iter().enumerate() {
        if at > 0 {
            w.write_str(", ")?;
        }
        w.write_str(name)?;
    }
    w.write_str("\n")
}

/// The words `help` lists and Tab completes: the shell's own that the board takes, then the board's.
fn listed() -> Vec<(String, String)> {
    KNOWN.with(|slot| {
        let slot = slot.borrow();
        let known = slot.as_deref();
        let (words, lacks) = known.map_or((&[][..], &[][..]), |k| (&k.words[..], &k.lacks[..]));
        OWN.iter()
            .filter(|(name, _)| !lacks.contains(name))
            .map(|(name, help)| (name.to_string(), help.to_string()))
            .chain(words.iter().map(|word| (word.name.to_string(), word.help.to_string())))
            .collect()
    })
}

/// What the cli asks `help` and Tab of: the shell's words and the board's.
struct Listed;

impl embedded_cli::service::Autocomplete for Listed {
    fn autocomplete(request: embedded_cli::autocomplete::Request<'_>, autocompletion: &mut embedded_cli::autocomplete::Autocompletion<'_>) {
        let embedded_cli::autocomplete::Request::CommandName(name) = request else { return };
        for (listed, _) in listed() {
            if let Some(rest) = listed.strip_prefix(name) {
                autocompletion.merge_autocompletion(rest);
            }
        }
    }
}

impl embedded_cli::service::Help for Listed {
    fn command_count() -> usize {
        listed().len()
    }

    fn list_commands<W: embedded_io::Write<Error = E>, E: embedded_io::Error>(
        writer: &mut embedded_cli::writer::Writer<'_, W, E>,
    ) -> Result<(), E> {
        let all = listed();
        let longest = all.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
        writer.write_title("Commands:")?;
        writer.writeln_str("")?;
        for (name, help) in &all {
            writer.write_list_element(name, help, longest)?;
        }
        Ok(())
    }

    fn command_help<
        W: embedded_io::Write<Error = E>,
        E: embedded_io::Error,
        F: FnMut(&mut embedded_cli::writer::Writer<'_, W, E>) -> Result<(), E>,
    >(
        parent: &mut F,
        command: RawCommand<'_>,
        writer: &mut embedded_cli::writer::Writer<'_, W, E>,
    ) -> Result<(), embedded_cli::service::HelpError<E>> {
        let name = command.name();
        if KNOWN.with(|slot| slot.borrow().as_deref().is_some_and(|k| k.lacks.contains(&name))) {
            return Err(embedded_cli::service::HelpError::UnknownCommand);
        }
        if let Some(word) = KNOWN.with(|slot| slot.borrow().as_deref().and_then(|k| k.words.iter().find(|word| word.name == name).cloned())) {
            writer.writeln_str(word.help)?;
            writer.writeln_str("")?;
            writer.write_str("Usage: ")?;
            writer.write_str(word.name)?;
            writer.writeln_str(" [WORDS]...")?;
            return Ok(());
        }
        <BoardCommand as embedded_cli::service::Help>::command_help(parent, command, writer)
    }
}

/// The board's answer to COMMAND on a line: what was done, or why it was refused.
fn answer<W: embedded_io::Write<Error = E>, E: embedded_io::Error>(
    w: &mut embedded_cli::writer::Writer<'_, W, E>,
    command: &str,
    said: Result<String, String>,
) -> Result<(), E> {
    match said {
        Ok(done) => w.write_str(&done)?,
        Err(why) => {
            w.write_str(command)?;
            w.write_str(" refused: ")?;
            w.write_str(&why)?;
        }
    }
    w.write_str("\n")
}

fn run<W: embedded_io::Write<Error = E>, E: embedded_io::Error>(
    cli: &mut CliHandle<'_, W, E>,
    command: BoardCommand<'_>,
    board: &mut dyn Board,
) -> Result<(), E> {
    let w = cli.writer();
    if let Some(word) = optional_word(&command) {
        if !board.takes(word) {
            return unknown(w);
        }
    }
    match command {
        BoardCommand::Status => {
            // The report allocates, and an allocation that fails takes the whole board down (rust_oom): on a heap with no
            // piece to spare the shell says what the heap has left - written without allocating - and asks no more.
            if let Some(room) = board.room().filter(|room| room.largest < STATUS_NEEDS) {
                return ufmt::uwrite!(
                    w,
                    "status not run: the heap is too tight to print it - free {} bytes, biggest piece {}, the report needs a piece of {}\n",
                    room.free as u32,
                    room.largest as u32,
                    STATUS_NEEDS as u32
                );
            }
            let mut failed = None;
            board.status(&mut |line| {
                if failed.is_none() {
                    failed = w.write_str(line).and_then(|_| w.write_str("\n")).err();
                }
            });
            failed.map_or(Ok(()), Err)
        }
        BoardCommand::Restart => {
            w.write_str("restarting\n")?;
            board.restart();
            Ok(())
        }
        BoardCommand::Down => {
            w.write_str("going down to the recovery image\n")?;
            match board.down() {
                Ok(()) => Ok(()),
                Err(why) => {
                    w.write_str("down refused: ")?;
                    w.write_str(&why)?;
                    w.write_str("\n")
                }
            }
        }
        BoardCommand::Log { words: [None, None] } => {
            let mut failed = None;
            let listed = board.loudness(&mut |line| {
                if failed.is_none() {
                    failed = w.write_str(line).and_then(|_| w.write_str("\n")).err();
                }
            });
            if let Some(e) = failed {
                return Err(e);
            }
            match listed {
                Ok(()) => Ok(()),
                Err(why) => {
                    w.write_str("log refused: ")?;
                    w.write_str(&why)?;
                    w.write_str("\n")
                }
            }
        }
        BoardCommand::Log { words: [Some(module), Some(level)] } => {
            answer(w, "log", board.set_loudness(module, level))
        }
        BoardCommand::Log { .. } => {
            w.write_str(LOG_TAKES)?;
            w.write_str("\n")
        }
        BoardCommand::Fell { how: [None, None] } => {
            let mut failed = None;
            let kept = board.fell(&mut |line| {
                if failed.is_none() {
                    failed = w.write_str(line).and_then(|_| w.write_str("\n")).err();
                }
            });
            if let Some(e) = failed {
                return Err(e);
            }
            match kept {
                Ok(()) => Ok(()),
                Err(why) => {
                    w.write_str("fell refused: ")?;
                    w.write_str(&why)?;
                    w.write_str("\n")
                }
            }
        }
        BoardCommand::Fell { how: [Some("now"), hard] } if hard.is_none() || hard == Some("hard") => {
            w.write_str("falling on purpose - the board restarts and says where it fell\n")?;
            board.fall_now(hard == Some("hard"));
            // Only a board that cannot be made to fall comes back here.
            w.write_str("fell now refused: this board cannot be made to fall\n")
        }
        BoardCommand::Fell { .. } => {
            w.write_str(FELL_TAKES)?;
            w.write_str("\n")
        }
        BoardCommand::Settings => {
            let mut failed = None;
            let read = board.settings(&mut |line| {
                if failed.is_none() {
                    failed = w.write_str(line).and_then(|_| w.write_str("\n")).err();
                }
            });
            if let Some(e) = failed {
                return Err(e);
            }
            match read {
                Ok(()) => Ok(()),
                Err(why) => {
                    w.write_str("settings refused: ")?;
                    w.write_str(&why)?;
                    w.write_str("\n")
                }
            }
        }
        BoardCommand::Set { name, value } => {
            match board.set(name, &value) {
                Ok(done) => w.write_str(&done)?,
                Err(why) => {
                    w.write_str("set refused: ")?;
                    w.write_str(&why)?;
                }
            }
            w.write_str("\n")
        }
        BoardCommand::Toggle { name } => answer(w, "toggle", board.toggle(name)),
        BoardCommand::Press { name } => answer(w, "press", board.press(name)),
        BoardCommand::On { name } => answer(w, "on", board.switch(name, true)),
        BoardCommand::Off { name } => answer(w, "off", board.switch(name, false)),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::convert::Infallible;
    use std::rc::Rc;

    use super::*;

    #[derive(Clone, Default)]
    struct Sink(Rc<RefCell<Vec<u8>>>);

    impl embedded_io::ErrorType for Sink {
        type Error = Infallible;
    }

    impl embedded_io::Write for Sink {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Infallible> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> Result<(), Infallible> {
            Ok(())
        }
    }

    struct Quiet;

    impl Board for Quiet {
        fn status(&mut self, _line: &mut dyn FnMut(&str)) {}
        fn restart(&mut self) {}
        fn down(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn the_count_of_a_line_stops_at_its_top_instead_of_starting_over() {
        // 2^32 bytes without a line end are out of a test's reach: the count is set next to its top instead
        let sink = Sink::default();
        let mut shell = Shell::new(sink.clone()).unwrap();
        shell.typed = usize::MAX - 1;
        for &b in b"xyz" {
            shell.feed(b, &mut Quiet).unwrap();
        }
        assert_eq!(shell.typed, usize::MAX);
        // and the line is still the long one it is: refused, not run
        shell.feed(b'\n', &mut Quiet).unwrap();
        assert_eq!(shell.typed, 0);
        let out = String::from_utf8_lossy(&sink.0.borrow()).into_owned();
        assert!(out.contains("refused"), "{out:?}");
    }
}
