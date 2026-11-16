//! One log and where its lines go: to the console on the wire at once, to the network's loop by a channel of fixed size
//! (its consoles read them from its ring), and - for the lines a device keeps - to the owner of the flash by a channel
//! of its own. Two sources feed it - the `log` macros of our Rust code and the C log of IDF (Wi-Fi, netif, the OTA
//! calls) - so both are caught here.
//!
//! **THE WIRE IS WRITTEN AT ONCE, AND IT IS THE ONE PLACE THAT IS.** The lines just before a fall are the ones a fall is
//! read by, and a line waiting in a channel falls with the board; the others are readers that may lose the oldest.

use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use esp_idf_svc::sys;
use journal::kept::{Folder, Kept, FOLDER};
use journal::{Feed, Fold, Verdict, BLACK_BOX_PER_MINUTE};
use loudness::{Level, Loudness};

/// How loud each module of this board is. **`Info` by default: events, not wellbeing** - the board used to
/// write a line a second about being well, and the one line that mattered went past in the flood exactly when
/// somebody needed it. Details are turned on by module from the shell's `log`, which asks the owner of the flash: the
/// table is its task's ([`loudness::Keeper`]), and every line reads it as that task last published it.
static LOUD: OnceLock<Loudness> = OnceLock::new();

fn as_loudness(level: log::Level) -> Level {
    match level {
        log::Level::Error => Level::Error,
        log::Level::Warn => Level::Warn,
        log::Level::Info => Level::Info,
        log::Level::Debug => Level::Debug,
        log::Level::Trace => Level::Trace,
    }
}

/// Every line of the log, to the network's loop: its consoles read them from its ring (web::serve).
///
/// **THE CHANNELS' LINES ARE IN THE HEAP, TAKEN ONCE IN [`start`]** - a block of kilobytes, which the heap puts in the
/// external RAM on a board with PSRAM; as statics they were 6.5 KB of the internal memory every task's stack comes
/// from. What stays here is the head of each.
static TO_NET: OnceLock<web::LogFeed> = OnceLock::new();

/// The lines kept on flash (journal::kept), each with its local time, to the owner of the flash: the modules a device
/// names in [`keep`], and every warning and error of any module. The owner takes them once a second, and before it
/// starts the board's coming up waits here: the size of that burst, which the plan sets by the board (32 lines, 8 without external RAM).
static TO_FLASH: OnceLock<Feed<{ plan::PLAN.to_flash_lines }>> = OnceLock::new();

/// What the flash has been told already: the lines that repeat are held here and told as one with their count, and the lines of the black box are written whole under a limit a minute (journal::folded).
/// The console and the network's consoles get every line either way; only the flash is spared.
static FOLD: Mutex<Fold> = Mutex::new(Fold::new());

/// The leaps of the tick's counter as they stood at the last look of the flash's owner (`journal_round`).
static TICKS: Mutex<between::tick::Watch> = Mutex::new(between::tick::Watch::new());

/// Seconds of the board's steady clock, which only goes forward: what the fold counts its periods by.
fn now_s() -> u64 {
    between::clock::now().as_micros() / 1_000_000
}

/// Batches of the journal written and refused since the start: written by the owner of the flash alone, read by
/// `status`.
pub static WRITES: AtomicU32 = AtomicU32::new(0);
pub static REFUSALS: AtomicU32 = AtomicU32::new(0);
/// The modules whose lines go to the flash, by their log target; None - this device keeps no journal.
static KEEPS: OnceLock<&'static [&'static str]> = OnceLock::new();

struct Tee;

static TEE: Tee = Tee;

impl log::Log for Tee {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        LOUD.get().is_some_and(|loud| loud.speaks(metadata.target(), as_loudness(metadata.level())))
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let level = match record.level() {
            log::Level::Error => 'E',
            log::Level::Warn => 'W',
            log::Level::Info => 'I',
            log::Level::Debug => 'D',
            log::Level::Trace => 'V',
        };
        // The same shape as an IDF line, so the two sources read as one log.
        let text = record.args().to_string();
        let line = format!("{level} ({}) {}: {text}", crate::board::uptime_ms(), record.target());
        let _ = writeln!(std::io::stdout(), "{line}");
        if let Some(net) = TO_NET.get() {
            net.put(&line);
        }
        if let (Some(keeps), Some(flash)) = (KEEPS.get(), TO_FLASH.get()) {
            if record.level() <= log::Level::Warn || keeps.contains(&record.target()) {
                let stamp = stamp();
                // read BEFORE the lock: a reading of the board's clock may say a leap of it, which is a line of this log and takes the lock again
                let at_s = now_s();
                let verdict = FOLD.lock().unwrap_or_else(|held| held.into_inner()).put(record.target(), &text, &stamp, at_s);
                match verdict {
                    Verdict::Write => flash.put(&format!("{stamp} {line}")),
                    Verdict::Repeated { times } => flash.put(&format!("{stamp} x{times} {line}")),
                    Verdict::WriteAfterSkipped { skipped } => {
                        flash.put(&format!("{stamp} ... {}: {skipped} lines skipped over {BLACK_BOX_PER_MINUTE} a minute", record.target()));
                        flash.put(&format!("{stamp} {line}"));
                    }
                    Verdict::Hold => {}
                }
            }
        }
    }

    fn flush(&self) {}
}

extern "C" {
    // newlib's; the generated bindings leave out the va_list functions.
    fn vsnprintf(
        buf: *mut core::ffi::c_char,
        size: usize,
        format: *const core::ffi::c_char,
        args: sys::va_list,
    ) -> core::ffi::c_int;
}

/// The one buffer every line of the C log is formatted in, taken under its lock.
///
/// **NOT ON THE STACK OF THE TASK THAT LOGS** (6.4, the fall in Journal::subscribe): a line of IDF comes from whichever
/// task wrote it - the TCP/IP task, the server's, Wi-Fi's - and their stacks are sized for IDF's own vprintf, not for
/// 256 more bytes of ours on top. The lock is held for the formatting and the copy only; the console and the journals
/// are written after it is let go.
static IDF_LINE: Mutex<[u8; 256]> = Mutex::new([0; 256]);

// The C log hands over a format and its arguments. They can be consumed once, so the line is formatted
// here and written to the console by us instead of by the vprintf IDF had.
unsafe extern "C" fn idf_line(format: *const core::ffi::c_char, args: sys::va_list) -> core::ffi::c_int {
    let (n, text) = {
        // a task that died holding the lock left a buffer that is only bytes: the next line writes over it
        let mut buf = IDF_LINE.lock().unwrap_or_else(|held| held.into_inner());
        let n = vsnprintf(buf.as_mut_ptr().cast(), buf.len() as _, format, args);
        if n < 0 {
            return n;
        }
        (n, buf[..(n as usize).min(buf.len() - 1)].to_vec())
    };
    let _ = std::io::stdout().write_all(&text);
    let line = String::from_utf8_lossy(&text);
    let line = line.trim_end_matches(['\r', '\n']);
    if let (false, Some(net)) = (line.is_empty(), TO_NET.get()) {
        net.put(line);
    }
    n
}

pub fn start() {
    // the channels and the table first: a line logged before them would have nowhere to go
    let _ = LOUD.set(Loudness::new(Level::Info));
    let _ = TO_NET.set(web::LogFeed::new());
    let _ = TO_FLASH.set(Feed::new());
    let _ = log::set_logger(&TEE);
    // EVERY LEVEL REACHES THE LOGGER, AND THE TABLE DECIDES. Capped here at Info instead, no command could
    // ever bring a debug line back: the log crate drops a record before the logger is asked, and the module
    // would not even be in the table to be named.
    log::set_max_level(log::LevelFilter::Trace);
    unsafe { sys::esp_log_set_vprintf(Some(idf_line)) };
}

/// The log's lines for the network's loop; None before [`start`].
pub fn to_net() -> Option<&'static web::LogFeed> {
    TO_NET.get()
}

/// How loud the modules are; None before [`start`]. Its keeper is the owner of the flash.
pub(crate) fn loud() -> Option<&'static Loudness> {
    LOUD.get()
}

/// The shell's `log`, answered by KEEPER in the task that holds it: ASKED None - the listing; Some - a module and a
/// level as a person typed them.
pub(crate) fn loudness(
    loud: &Loudness,
    keeper: &mut loudness::Keeper,
    asked: Option<(String, String)>,
) -> Result<Vec<String>, String> {
    match asked {
        None => Ok(keeper.lines(loud)),
        Some((module, level)) => {
            let level =
                Level::of_word(&level).ok_or_else(|| format!("'{level}' is not a level; they are: {}", Level::WORDS))?;
            keeper.set(loud, &module, level).map(|said| vec![said])
        }
    }
}

/// The local time of a line on flash, read days later: the uptime in the line says nothing across a restart.
fn stamp() -> String {
    match crate::clock::now() {
        Some(t) => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            t.year,
            t.month,
            t.day,
            t.minute / 60,
            t.minute % 60,
            t.second
        ),
        None => "no-clock".into(),
    }
}

/// The ring's folder in vfs.
struct Vfs;

impl Vfs {
    fn path(name: &str) -> String {
        format!("{FOLDER}/{name}")
    }
}

impl Folder for Vfs {
    fn names(&mut self) -> Result<Vec<String>, String> {
        vfs::files(FOLDER).map_err(|e| e.to_string())
    }
    fn size(&mut self, name: &str) -> Result<usize, String> {
        vfs::stat(&Self::path(name)).map(|stat| stat.size).map_err(|e| e.to_string())
    }
    fn append(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        vfs::append(&Self::path(name), bytes).map_err(|e| e.to_string())
    }
    fn remove(&mut self, name: &str) -> Result<(), String> {
        vfs::remove(&Self::path(name)).map_err(|e| e.to_string())
    }
    fn read_at(&mut self, name: &str, at: usize, buf: &mut [u8]) -> Result<usize, String> {
        vfs::read_at(&Self::path(name), at, buf).map_err(|e| e.to_string())
    }
}

/// Keeps the lines of the modules TARGETS - their log targets, as `device::module` - and every warning and error for the
/// journal on flash, FROM THIS LINE ON: they wait in their channel until the owner of the flash has vfs open and takes
/// them (`journal_round`). Call it first thing in main, after `start`, so the board's coming up is among them.
pub fn keep(targets: &'static [&'static str]) {
    let _ = KEEPS.set(targets);
}

/// Whether this device keeps a journal on flash.
pub(crate) fn keeps() -> bool {
    KEEPS.get().is_some()
}

/// One pass of the journal on flash, in the task of its owner: the lines that came into KEPT, and the batch that is due
/// to /data/journal, when the device asked for a journal (`keep`). The owner calls it once a second (`flash_owner`);
/// REFUSED is what the flash said last time, so a flash that keeps refusing is said once and not a line a second. Answers whether the batch went down: a refusal leaves its lines waiting in memory, and a caller about to restart the board waits for the flash.
pub(crate) fn journal_round(kept: &mut Kept, refused: &mut Option<String>, now_please: bool) -> Result<(), String> {
    // THE LEAPS OF THE TICK are counted in the interrupt, where no line can be written; the line is written here, at the owner's next look, with how many came and the last one's size and way.
    let leap = TICKS.lock().unwrap_or_else(|held| held.into_inner()).look(crate::board::tick_leaps());
    if let Some(leap) = leap {
        let way = if leap.last_was_back { "back" } else { "forward" };
        log::warn!(target: journal::BOX_TICK, "tick leap {way} {} ms, {} since the last look", leap.last_ticks, leap.new);
    }
    if !keeps() {
        return Ok(());
    }
    let now = crate::board::uptime_ms() as u64;
    let Some(flash) = TO_FLASH.get() else { return Ok(()) };
    // the end of the ring into memory, once a start: /journal answers it and not the flash
    if let Err(why) = kept.recall(&mut Vfs) {
        log::warn!("journal on {FOLDER}: its end not taken into memory ({why}) - /journal?all reads the flash");
    }
    while let Some(line) = flash.take() {
        kept.add(&line, now);
    }
    // the runs that went quiet, and the lines of the black box skipped over the limit that no line followed to tell: one line each
    let at_s = now_s();
    let quiet = FOLD.lock().unwrap_or_else(|held| held.into_inner()).quiet(at_s);
    for (target, summary) in quiet {
        kept.add(&format!("{} {}", stamp(), summary.line(&target)), now);
    }
    let lost = flash.lost();
    if lost > 0 {
        kept.add(&format!("... {lost} lines lost: they came faster than the flash's owner took them"), now);
    }
    let done = match now_please || kept.due(now) {
        true => kept.flush(&mut Vfs),
        false => Ok(()),
    };
    WRITES.store(kept.writes(), Ordering::Release);
    REFUSALS.store(kept.refusals(), Ordering::Release);
    match &done {
        Ok(()) => {
            if refused.take().is_some() {
                log::info!("journal: the flash takes the batches again");
            }
        }
        Err(why) => {
            if refused.as_ref() != Some(why) {
                log::warn!("journal on {FOLDER}: {why}");
                *refused = Some(why.clone());
            }
        }
    }
    done
}
