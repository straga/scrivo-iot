//! The board's network in one loop: the page on port 80 - its files from vfs, the dictionary,
//! the state, the commands, the cards that changed as server-sent events, the console over a WebSocket - and the
//! console on TCP port 23, all futures of one task, `net`, woken by the one reactor thread of `async-io-mini`.
//!
//! **NOTHING HERE WAITS ON ANOTHER TASK.** A command to a module, a saved setting, a curve, a typed line - whatever
//! waits for the task that owns it - is asked of the task `orders` and awaited ([`orders`]); the loop reads values
//! the modules published and nothing else. Every write into a socket has a limit ([`Limits::write`]): a client that
//! takes nothing for that long is let go, and the others are not held behind it.
//!
//! **THE MEMORY IS SET AT START.** [`HANDLERS`] handlers, each serving one connection at a time with its buffer for
//! the head of a request; a seventh connection waits in the socket's queue for a free one. The consoles on port 23
//! have [`TELNET`] places. No task, no stack and no thread is made for a connection.
//!
//! The loop is generic over the socket stack, so the tests run it on the host over `edge-nal-std` exactly as the
//! board does; what is of the board - the eventfd, the task, the reactor's name - is in `esp`.

use core::cell::{Cell, RefCell};
use core::fmt::{Debug, Display};
use std::sync::OnceLock;
use std::time::Duration;

use edge_http::io::server::{handle_connection, Connection, Handler};
use edge_http::Method;
use edge_nal::{TcpAccept, TcpSplit};
use core::future::Future;
use core::pin::Pin;

use embassy_futures::select::{select, select3, select_slice, Either, Either3};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use embedded_io_async::{Read, Write};

pub mod orders;
pub mod outside;
#[cfg(target_os = "espidf")]
pub mod esp;

use orders::{time, Answer, Order, Orders, BROWSER_CONSOLES, CONSOLES, PORT_CONSOLES};

/// Where the page files lie in vfs; the kit put there by the recovery image mirrors it.
pub const FOLDER: &str = "/page";
pub use page::CONSOLE;

/// The page's port, and the console's.
pub const HTTP_PORT: u16 = 80;
pub const TELNET_PORT: u16 = 23;

/// Handlers of the page, by the board's plan: six on a board with external RAM (two streams, two consoles in browsers, two for the short requests), three without.
pub const HANDLERS: usize = plan::PLAN.handlers;
/// A handler's buffer for the head of a request.
pub const BUFFER: usize = plan::PLAN.head_buffer;
/// Consoles on port 23 at once; one acceptor more, to say "busy" to the next and let it go.
pub const TELNET: usize = PORT_CONSOLES;
/// Lines of the log between the tasks that write them and the loop, which takes them as they come (`log_pump` awaits the
/// channel): what it holds is only a burst between two polls of the loop, and a burst past it costs the consoles their
/// oldest lines - the wire and the journal on flash take the log by ways of their own. The slots are taken from the heap
/// once, at start (journal::Feed).
pub const LOG_FEED: usize = 8;
/// The log's way to the loop: the firmware's logger puts every line in (firmware::log_tee).
pub type LogFeed = journal::Feed<LOG_FEED>;
/// The last lines of the log the loop keeps for its consoles; a console further behind loses the oldest.
const LOG_RING: usize = plan::PLAN.log_ring;

/// Which console places are held, a bit a place: written by the loop alone, read by `status` and the page's card.
static HELD: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// What the page's handlers hold, for `status` (on a device in service: /state went unanswered for 15 s while the
/// console answered). Written by the loop alone: a bit a handler holding a connection, how many handlers are inside an
/// answer, and how many of those answers are streams.
static CONNECTED: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
static ANSWERING: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
static STREAMING: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// The page's handlers now: of [`HANDLERS`], how many hold a connection, and what holds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handlers {
    pub held: usize,
    pub streams: usize,
    pub browser_consoles: usize,
    /// Inside an answer that is neither a stream nor a console: a page file, /state, a command waiting for `orders`.
    pub answering: usize,
    /// A connection kept alive between its requests, until it is quiet for KEEP_ALIVE_MS.
    pub kept: usize,
}

pub fn handlers() -> Handlers {
    use core::sync::atomic::Ordering::Acquire;
    let held = CONNECTED.load(Acquire).count_ones() as usize;
    let inside = ANSWERING.load(Acquire) as usize;
    let streams = STREAMING.load(Acquire) as usize;
    let browser_consoles = consoles_open().0;
    Handlers {
        held,
        streams,
        browser_consoles,
        answering: inside.saturating_sub(streams + browser_consoles),
        kept: held.saturating_sub(inside),
    }
}

/// One more in COUNT while it lives.
struct Counted(&'static core::sync::atomic::AtomicU32);

impl Counted {
    fn new(count: &'static core::sync::atomic::AtomicU32) -> Self {
        count.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
        Self(count)
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_sub(1, core::sync::atomic::Ordering::AcqRel);
    }
}

/// Consoles open now: in browsers, and on port 23.
pub fn consoles_open() -> (usize, usize) {
    let held = HELD.load(core::sync::atomic::Ordering::Acquire);
    let count = |from: usize, to: usize| (from..to).filter(|p| held & (1 << p) != 0).count();
    (count(0, BROWSER_CONSOLES), count(BROWSER_CONSOLES, BROWSER_CONSOLES + PORT_CONSOLES))
}

/// A piece of a file read and sent at once. Each piece opens the file anew (`vfs::read_at`), since a file held open
/// across a write into a socket would hold app_fs's lock against the task `flash` all that while.
const PIECE: usize = 4096;
/// How long the loop sleeps after every piece of a file it read off flash: its task falls asleep, and the idle task of
/// core 0 and the task `flash` get the core. A socket that takes the piece at once does not put the task to sleep, and
/// without this a /journal of 230 KB ran the loop 4-30 s without a sleep until the task watchdog reset the board (the
/// device in service on 26.09). A millisecond is 64 ms over a file of 256 KB.
const LET_GO: Duration = Duration::from_millis(1);
/// Headers of a request, at most: a browser sends about eighteen.
pub const HEADERS: usize = 32;

/// The longest form of the tab Settings: every setting's name and value, each value within page::MAX_VALUE.
const FORM: usize = 1024;
/// A connection quiet between its requests gives its handler back after this. One browser keeps up to six connections
/// to a board; kept 10 s each, a page left a second viewer - a phone - refused (on a device in service). A page
/// loads its files in a burst under a second (`app.js` 362 ms on `.30`), and a page without a stream of its own
/// asks /state every 5 s: a second holds the burst on one connection and the polling page holds a handler one second
/// in five. A stream and a console leave HTTP behind (`unbind`), and this does not touch them.
const KEEP_ALIVE_MS: u32 = 1_000;
/// How long a command waits for its owner - a deciding module waits up to a second on each organ, and may be in the
/// middle of its own second.
const COMMAND: Duration = Duration::from_secs(5);
/// How long a console's line waits: `bus scan` walks 247 addresses at 80 ms.
const TYPED: Duration = Duration::from_secs(35);
/// How often a console drains the log it was subscribed to (until it gets a channel of its own).
const LOG_EVERY: Duration = Duration::from_millis(100);

/// On every answer a client reads: the same page served on a computer, pointed at this board, reads them.
/// Who may command the board at all is a question of signing in, not of this header.
const ANY_ORIGIN: (&str, &str) = ("Access-Control-Allow-Origin", "*");

/// The limits the loop keeps; the tests shorten them.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// A write into a socket that does not end within this lets the client go.
    pub write: Duration,
    /// A quiet stream sends a beat this often (page::EVERY_MS).
    pub beat: Duration,
    /// An answered /restart or /down waits this long before it acts (page::GOING_MS).
    pub going: Duration,
    /// How long a connection that finds every handler held waits for one to let go before it is refused.
    pub wait_for_a_handler: Duration,
    /// What one second of the pace a reader asks for (`/events?every=5`) lasts: a second on the board, shorter in the tests.
    pub second: Duration,
    /// How long a reader that asked to hear every change (`every=0`, Live) keeps it before it hears at the pace of everyone (page::dictionary::LIVE_S).
    pub live: Duration,
    /// How many handlers streams and consoles of browsers may hold at once, the rest being left to the short requests (`plan::Plan::long_holders`).
    pub long_holders: usize,
}

impl Limits {
    pub const BOARD: Limits = Limits {
        write: Duration::from_secs(5),
        beat: Duration::from_millis(page::EVERY_MS),
        going: Duration::from_millis(page::GOING_MS),
        wait_for_a_handler: Duration::from_secs(3),
        second: Duration::from_secs(1),
        live: Duration::from_secs(page::dictionary::LIVE_S as u64),
        long_holders: plan::PLAN.long_holders(),
    };
}

/// What the loop reads of the board. Nothing here may wait on another task: what does is an [`Order`].
pub trait Board: page::Values + Send + Sync + 'static {
    /// The board's dictionary as the image carries it (`page::dictionary`).
    fn dictionary(&self) -> page::dictionary::Served;
    /// The board's name, by which a person asks it to go down (`/down/<name>`).
    fn board(&self) -> String;
    fn restart(&self);
    fn down(&self);
    /// The size of the file at PATH in vfs and a tag that changes when the file is written again, or None when it is
    /// not there. Neither is got by reading the file through: the loop holds its core while it asks.
    fn file(&self, path: &str) -> Option<(usize, u64)>;
    /// Reads from the file at PATH at AT into BUF; the count read, 0 at its end.
    fn read_at(&self, path: &str, at: usize, buf: &mut [u8]) -> Result<usize, String>;
    /// The names in the folder DIR of vfs.
    fn files(&self, dir: &str) -> Result<Vec<String>, String>;
}

/// Where the board announces which of its cards has changed: every stream is marked with that card and sends it at
/// once instead of at its next tick. More places than there are streams: one beside them for a subscriber that is
/// not a page (`page::SUBSCRIBERS`).
pub static CHANGES: events::Changes<{ page::SUBSCRIBERS }> = events::Changes::new();

/// The steps of the task the loop runs in: how long it ran without falling asleep, which the idle task of its core
/// waits for (`between::block_on_stepped`). `status` says the longest.
pub static STEPS: between::Steps = between::Steps::new();
/// The longest the loop runs before it falls asleep for a tick though it has work ready: under load its sockets wake it
/// faster than it runs, and without this it ran 1-8 s awake, till the task watchdog reset the board (`.30`, 26.09).
/// A step of the page's work is a few milliseconds - a piece of a file, a /state -, so 20 ms cuts none of them in two
/// and leaves the watchdog's 5 s far off.
pub const STEP_MOST: Duration = Duration::from_millis(20);

/// The cards of this board, set when the loop starts; a change announced before it marks everything.
static CARDS: OnceLock<&'static [&'static str]> = OnceLock::new();

/// The card CARD of this board has changed. What a module calls; the card is the name it already writes into
/// the dictionary. A card the device has not declared marks everything - the board then sends the whole of itself,
/// which costs bytes and cannot be wrong about what it says.
pub fn announce(card: &str) {
    let marks = match CARDS.get() {
        Some(cards) => page::mark_of(cards, card),
        None => page::Marks::ALL,
    };
    CHANGES.announce(marks);
}

/// The settings have changed: every card is marked, because a setting is read by whichever module cares about it and the
/// board keeps no map of which. What the owner of the flash calls after every change that moved a setting - the page's
/// save, the console's `set`, anyone's - so an open page is told whoever changed it.
pub fn announce_settings() {
    CHANGES.announce(page::Marks::ALL);
}

/// The board as the page asks it, with the one thing the board does not know: where the page's own console is.
struct Here<'a, B>(&'a B);

impl<B: Board> page::Values for Here<'_, B> {
    fn version(&self) -> &str {
        page::Values::version(self.0)
    }
    fn cards(&self) -> &'static [&'static str] {
        page::Values::cards(self.0)
    }
    // THE DIGITS A NUMBER IS WORTH ARE THE BOARD'S, said once here: every way a value leaves - /state, the first event, the changes, the answer of a command - passes this, so a reader never has more digits
    // than the dictionary says, and the stream's "did it change" is asked of what is sent and not of what was read.
    fn values(&self, wanted: &page::Wanted, each: &mut dyn FnMut(usize, page::dictionary::Said<'_>)) {
        let table = self.0.dictionary().table;
        page::Values::values(self.0, wanted, &mut |at, said| match table.get(at) {
            Some(entry) => each(at, page::rounded(entry, said)),
            None => each(at, said),
        })
    }
}

/// What the page's handlers share - all of it the loop's own, on its one task.
struct Loop<'a, B> {
    board: &'a B,
    orders: &'a Orders,
    limits: Limits,
    streams: Cell<usize>,
    /// A reader of /journal is being answered: the next is refused at once (`journal`).
    journal_read: Cell<bool>,
    /// Which console places are held: the browsers' first, then the port's.
    consoles: [Cell<bool>; CONSOLES],
    /// What an answered /restart or /down does once its answer has left.
    going: Signal<NoopRawMutex, fn(&B)>,
    /// A handler took a connection or let one go: what the refuser of the page waits on.
    handlers_moved: Signal<NoopRawMutex, ()>,
    /// The last lines of the log, for the consoles.
    ring: RefCell<journal::Ring<LOG_RING>>,
    /// Bodies are made here before they go out, and every other buffer a connection needs while it waits - a piece
    /// of a file, a console's output, a frame - is taken here and given back: a buffer inside a future would be
    /// carried by every handler's future at once, and the loop's whole future is born on its task's stack
    /// (`round::ON_STACK_AT_MOST`). Kept for the next one, so the heap is asked only while the pool grows.
    bodies: RefCell<Vec<Vec<u8>>>,
}

/// One of the loop's tasks, in a box of its own.
type Task<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

/// TASK in its box, made once at start. It is made where this is called and passes through the stack of `net` on its
/// way, so on the board the build refuses one heavier than `round::BOXED_ON_START` (`between::fits`); on the host the
/// sizes are not the board's, and nothing is refused.
fn boxed<'a>(task: impl Future<Output = ()> + 'a) -> Task<'a> {
    #[cfg(target_os = "espidf")]
    let task = between::fits::<_, { round::BOXED_ON_START }>(task);
    Box::pin(task)
}

/// Serves the page on HTTP and the console on TELNET, for good. ORDERS is served by its own task.
///
/// **EVERY TASK OF THE LOOP IS BORN ALONE, IN ITS OWN BOX.** A handler with its connection and a console is a few
/// kilobytes of future; all of them in one future were 86 KB on the board, and a future is made where
/// it is called - on the stack of the task `net`, 12 KB, before any box takes it. The one that did overran the stack
/// and broke a mutex of another task at start. One handler at a time passes through the stack instead, and the
/// future of the whole loop keeps only the boxes.
pub async fn serve<B: Board, A: TcpAccept, T: TcpAccept>(
    board: &B,
    orders: &Orders,
    log: &LogFeed,
    http: A,
    telnet: T,
    limits: Limits,
) {
    let _ = CARDS.set(page::Values::cards(board));
    let at = Loop {
        board,
        orders,
        limits,
        streams: Cell::new(0),
        journal_read: Cell::new(false),
        consoles: [const { Cell::new(false) }; CONSOLES],
        going: Signal::new(),
        handlers_moved: Signal::new(),
        ring: RefCell::new(journal::Ring::new()),
        bodies: RefCell::new(Vec::new()),
    };
    // the heads of the requests: one lump in the heap, a piece a handler
    let mut heads = vec![0u8; HANDLERS * BUFFER];
    let errands = outside::taken();
    let mut tasks: Vec<Task<'_>> = Vec::with_capacity(HANDLERS + TELNET + 3 + errands.len());
    for (n, head) in heads.chunks_mut(BUFFER).enumerate() {
        tasks.push(boxed(page_handler(&at, &http, n, head)));
    }
    for _ in 0..=TELNET {
        tasks.push(boxed(telnet_console(&at, &telnet)));
    }
    tasks.push(boxed(refuser(&at, &http)));
    tasks.push(boxed(going(&at)));
    tasks.push(boxed(log_pump(&at, log)));
    // what a device left to run beside the page (`outside`): each born alone, as the loop's own are, by the device's `outside::boxed`
    for start in errands {
        tasks.push(Box::pin(outside::for_good(start())));
    }
    let sizes: usize = HANDLERS * core::mem::size_of_val(&page_handler(&at, &http, 0, &mut []));
    log::info!("the loop's tasks: {} in boxes, the page's handlers {sizes} bytes", tasks.len());
    select_slice(Pin::new(&mut tasks[..])).await;
    log::error!("a task of the loop ended");
}

/// One of the page's handlers: a connection at a time, its requests kept alive until it goes quiet.
async fn page_handler<B: Board, A: TcpAccept>(at: &Loop<'_, B>, acceptor: &A, n: usize, head: &mut [u8]) {
    loop {
        match acceptor.accept().await {
            Ok((_, io)) => {
                CONNECTED.fetch_or(1 << n, core::sync::atomic::Ordering::AcqRel);
                at.handlers_moved.signal(());
                handle_connection::<_, _, HEADERS>(io, head, Some(KEEP_ALIVE_MS), n, at).await;
                CONNECTED.fetch_and(!(1 << n), core::sync::atomic::Ordering::AcqRel);
                at.handlers_moved.signal(());
            }
            Err(e) => {
                log::warn!("page: a connection not accepted: {e:?}");
                Timer::after(time(Duration::from_millis(100))).await;
            }
        }
    }
}

/// What a connection is told when every handler holds one.
const EVERY_HANDLER_HELD: &str = "busy: every connection of the board is taken - try again in a few seconds";

/// The page's seventh acceptor: a connection that finds all six handlers held waits in the TCP queue for one to let
/// go, and is answered 503 in words only when none does within `wait_for_a_handler`. Without it the connection waited
/// for as long as the six kept asking - a page and a browser's six kept connections held /state unanswered for 15 s on
/// a device in service. **AND IT WAITS BEFORE IT REFUSES, BECAUSE A PAGE LOADS IN A BURST** (25.09): a browser
/// opens a connection a file at once beside the page's stream, and a refusal at once left the page blank - its app.js
/// answered 503. A kept-alive connection lets go within KEEP_ALIVE_MS, so the burst is served in a second or two. It
/// accepts only while every handler has been held the whole wait, so a connection that finds a handler free is that
/// handler's. **THE WAIT IS COUNTED FROM THE LAST HANDLER THAT MOVED**, not from the connection's coming: a connection in
/// the TCP queue is not seen until it is taken, and one taken cannot be handed to a handler - so into six held still
/// longer than the wait, a new connection is refused at once.
async fn refuser<B: Board, A: TcpAccept>(at: &Loop<'_, B>, acceptor: &A) {
    loop {
        if handlers().held < HANDLERS {
            at.handlers_moved.wait().await;
            continue;
        }
        // every handler held: one that lets go takes the next connection itself, so the wait starts again
        if let Either::Second(()) = select(Timer::after(time(at.limits.wait_for_a_handler)), at.handlers_moved.wait()).await {
            continue;
        }
        let (peer, mut socket) = match select(acceptor.accept(), at.handlers_moved.wait()).await {
            Either::First(Ok(accepted)) => accepted,
            Either::First(Err(e)) => {
                log::warn!("page: a connection not accepted: {e:?}");
                Timer::after(time(Duration::from_millis(100))).await;
                continue;
            }
            Either::Second(()) => continue,
        };
        let (mut from, mut to) = socket.split();
        // the request's head is read first: a socket closed with it unread is reset, and the refusal lost with it
        let mut head = [0u8; 256];
        let _ = embassy_time::with_timeout(time(Duration::from_millis(500)), from.read(&mut head)).await;
        let answer = format!(
            "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\
             Content-Length: {}\r\n\r\n{EVERY_HANDLER_HELD}",
            EVERY_HANDLER_HELD.len()
        );
        let _ = at.write(&mut to, answer.as_bytes()).await;
        log::warn!("page: connection from {peer} refused: all {HANDLERS} handlers hold a connection");
    }
}

/// The log's lines into the ring, as they come; lines lost on the way are said there in their place.
async fn log_pump<B: Board>(at: &Loop<'_, B>, feed: &LogFeed) {
    loop {
        let line = feed.next().await;
        let lost = feed.lost();
        let mut ring = at.ring.borrow_mut();
        if lost > 0 {
            ring.push(journal::line_of(&format!("... {lost} lines dropped: the log came faster than the loop took it")));
        }
        ring.push(line);
    }
}

/// What an answered /restart or /down does, once its answer has left.
async fn going<B: Board>(at: &Loop<'_, B>) {
    let action = at.going.wait().await;
    Timer::after(time(at.limits.going)).await;
    action(at.board);
}

// ---- answers ----

type Io<T> = edge_http::io::Error<T>;

impl<B: Board> Loop<'_, B> {
    fn body(&self) -> Vec<u8> {
        let mut body = self.bodies.borrow_mut().pop().unwrap_or_default();
        body.clear();
        body
    }

    fn give_back(&self, body: Vec<u8>) {
        self.bodies.borrow_mut().push(body);
    }

    /// Writes ALL within the write limit.
    async fn write<W: Write>(&self, to: &mut W, all: &[u8]) -> Result<(), ()> {
        match embassy_time::with_timeout(time(self.limits.write), to.write_all(all)).await {
            Ok(Ok(())) => Ok(()),
            _ => Err(()),
        }
    }

    async fn answer<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        status: u16,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<(), Io<T::Error>> {
        let length = body.len().to_string();
        let mut all: heapless::Vec<(&str, &str), 8> = heapless::Vec::new();
        let _ = all.extend_from_slice(headers);
        let _ = all.push(("Content-Length", &length));
        conn.initiate_response(status, None, &all).await?;
        self.write(conn, body).await.map_err(|_| edge_http::io::Error::ConnectionClosed)
    }

    async fn json<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        status: u16,
        made: impl FnOnce(&mut Vec<u8>) -> bool,
    ) -> Result<(), Io<T::Error>> {
        let mut body = self.body();
        if !made(&mut body) {
            log::error!("an answer could not be made whole");
        }
        let out = self.answer(conn, status, &[("Content-Type", "application/json"), ANY_ORIGIN], &body).await;
        self.give_back(body);
        out
    }

    async fn value<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        value: &serde_json::Value,
    ) -> Result<(), Io<T::Error>> {
        self.json(conn, 200, |body| serde_json::to_writer(body, value).is_ok()).await
    }

    async fn words<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        status: u16,
        text: &str,
    ) -> Result<(), Io<T::Error>> {
        self.answer(conn, status, &[("Content-Type", "text/plain; charset=utf-8"), ANY_ORIGIN], text.as_bytes()).await
    }

    /// Says why the owner of commands did not answer, the way every refusal of a command is said: in words, with
    /// status 200 (docs/CONTRACT_Device_Api.md).
    async fn refused<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        why: between::Refused,
    ) -> Result<(), Io<T::Error>> {
        let words = match why {
            between::Refused::Busy => "the board is busy with a long command - try again when it ends",
            between::Refused::Silent => "the board did not answer in time - try again",
        };
        self.value(conn, &serde_json::json!({ "error": words })).await
    }

    /// The body of a short request, when it is at most LIMIT bytes; None for a longer one, refused whole rather than
    /// cut to something that reads as another value.
    async fn read_body<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        limit: usize,
    ) -> Result<Option<String>, Io<T::Error>> {
        let mut buf = vec![0u8; limit + 1];
        let mut len = 0;
        while len < buf.len() {
            let n = match embassy_time::with_timeout(time(self.limits.write), conn.read(&mut buf[len..])).await {
                Ok(n) => n?,
                Err(_) => return Err(edge_http::io::Error::ConnectionClosed),
            };
            if n == 0 {
                break;
            }
            len += n;
        }
        Ok((len <= limit).then(|| String::from_utf8_lossy(&buf[..len]).into_owned()))
    }

    /// A page file from vfs, gzipped when the browser takes it, with a tag of its bytes and `no-cache`: the browser
    /// asks before it uses what it kept, and a reflashed page is never yesterday's.
    async fn file<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        name: &str,
        content_type: &str,
    ) -> Result<(), Io<T::Error>> {
        let headers = conn.headers()?;
        let accept = headers.headers.get("Accept-Encoding").unwrap_or("").to_string();
        let asked = headers.headers.get("If-None-Match").unwrap_or("").to_string();
        let (path, gzipped) = page::served(FOLDER, name, &accept);
        let Some((size, hash)) = self.board.file(&path) else {
            let text = format!("{path} is not on the board: the page files are put in vfs by the recovery image");
            return self.words(conn, 404, &text).await;
        };
        let tag = format!("\"{hash:016x}\"");
        if asked.split(',').any(|one| one.trim() == tag) {
            let headers = [("ETag", tag.as_str()), ("Cache-Control", "no-cache")];
            return self.answer(conn, 304, &headers, &[]).await;
        }
        let length = size.to_string();
        let mut headers: heapless::Vec<(&str, &str), 5> = heapless::Vec::new();
        let _ = headers.push(("Content-Type", content_type));
        let _ = headers.push(("Content-Length", length.as_str()));
        let _ = headers.push(("ETag", tag.as_str()));
        let _ = headers.push(("Cache-Control", "no-cache"));
        if gzipped {
            let _ = headers.push(("Content-Encoding", "gzip"));
        }
        conn.initiate_response(200, None, &headers).await?;
        self.send_file(conn, &path, size).await.map_err(|_| edge_http::io::Error::ConnectionClosed)
    }

    /// Sends SIZE bytes of the file at PATH a piece at a time; a file that ends short ends the connection, since
    /// its length has been said.
    async fn send_file<W: Write>(&self, to: &mut W, path: &str, size: usize) -> Result<(), ()> {
        let mut buf = self.body();
        buf.resize(PIECE, 0);
        let sent = async {
            let mut at = 0;
            while at < size {
                let n = match self.board.read_at(path, at, &mut buf) {
                    Ok(0) | Err(_) => {
                        log::warn!("{path} could not be read while being sent");
                        return Err(());
                    }
                    Ok(n) => n.min(size - at),
                };
                self.write(to, &buf[..n]).await?;
                at += n;
                Timer::after(time(LET_GO)).await;
            }
            Ok(())
        }
        .await;
        self.give_back(buf);
        sent
    }

    /// Asks the owner of commands, and answers what it said.
    async fn ordered<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        order: Order,
    ) -> Result<Option<Answer>, Io<T::Error>> {
        match self.orders.ask(order, COMMAND).await {
            Ok(answer) => Ok(Some(answer)),
            Err(why) => self.refused(conn, why).await.map(|_| None),
        }
    }
}

// ---- the page's routes ----

fn path_of(uri: &str) -> &str {
    uri.split('?').next().unwrap_or(uri)
}

/// The route of ORDER that answers PATH and WAY: the first, as IDF's server matched them.
fn route_of(path: &str, way: page::Way) -> Option<page::Route> {
    page::ORDER.into_iter().find(|r| r.way() == way && page::wildcard_match(r.uri(), path))
}

impl<B: Board> Handler for Loop<'_, B> {
    type Error<E>
        = Io<E>
    where
        E: Debug;

    async fn handle<T, const N: usize>(
        &self,
        _task_id: impl Display + Copy,
        conn: &mut Connection<'_, T, N>,
    ) -> Result<(), Self::Error<T::Error>>
    where
        T: Read + Write + TcpSplit,
    {
        let _answering = Counted::new(&ANSWERING);
        let headers = conn.headers()?;
        let way = match headers.method {
            Method::Get => page::Way::Get,
            Method::Post => page::Way::Post,
            _ => return self.words(conn, 405, "only GET and POST").await,
        };
        let uri = headers.path.to_string();
        let Some(route) = route_of(path_of(&uri), way) else {
            return self.words(conn, 404, "no such file on this page").await;
        };
        self.route(conn, route, &uri).await
    }
}

impl<B: Board> Loop<'_, B> {
    async fn route<T: Read + Write + TcpSplit, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        route: page::Route,
        uri: &str,
    ) -> Result<(), Io<T::Error>> {
        use page::Route;
        match route {
            Route::Index => self.file(conn, "index.html", "text/html; charset=utf-8").await,
            Route::Icon | Route::Favicon => self.file(conn, "icon.svg", "image/svg+xml").await,
            Route::Rest => {
                let name = page::name_in(uri, "/");
                match page::mime(&name) {
                    Some(kind) => self.file(conn, &name, kind).await,
                    None => self.words(conn, 404, "no such file on this page").await,
                }
            }
            Route::State => {
                let here = Here(self.board);
                self.json(conn, 200, |body| page::write_values(body, &here).is_ok()).await
            }
            Route::Dictionary => {
                let headers = [("Content-Type", "application/json"), ("Cache-Control", "no-cache"), ANY_ORIGIN];
                self.answer(conn, 200, &headers, self.board.dictionary().json.as_bytes()).await
            }
            Route::Restart => {
                self.value(conn, &page::restart()).await?;
                self.going.signal(|b| b.restart());
                Ok(())
            }
            Route::Down => {
                let said = page::name_in(uri, "/down/");
                match page::down(&self.board.board(), &said) {
                    Ok(answer) => {
                        self.value(conn, &answer).await?;
                        self.going.signal(|b| b.down());
                        Ok(())
                    }
                    Err(refusal) => self.value(conn, &refusal).await,
                }
            }
            Route::Toggle | Route::On | Route::Off | Route::Press | Route::Hold => {
                let (what, prefix) = match route {
                    Route::Toggle => ("toggle", "/toggle/"),
                    Route::On => ("on", "/on/"),
                    Route::Off => ("off", "/off/"),
                    Route::Press => ("press", "/press/"),
                    _ => ("hold", "/hold/"),
                };
                let said = match route {
                    Route::Hold => self.read_body(conn, page::MAX_VALUE).await?.unwrap_or_default(),
                    _ => String::new(),
                };
                self.by_address(conn, uri, what, prefix, &said).await
            }
            Route::Settings => {
                let Some(form) = self.read_body(conn, FORM).await? else {
                    let refusal = page::refusal::form_too_long(FORM);
                    return self.value(conn, &refusal).await;
                };
                // the save is announced by the owner of the flash that carried it out (announce_settings), as every
                // change of a setting is, whoever made it
                match self.ordered(conn, Order::Save(form)).await? {
                    Some(Answer::Json(said)) => self.value(conn, &said).await,
                    Some(other) => self.said(conn, other).await,
                    None => Ok(()),
                }
            }
            Route::Curve => {
                let name = page::name_in(uri, "/curve/");
                match self.ordered(conn, Order::Curve(name.clone())).await? {
                    Some(Answer::Curve(Some(drawn))) => {
                        self.json(conn, 200, |body| serde_json::to_writer(body, &drawn).is_ok()).await
                    }
                    Some(Answer::Curve(None)) => {
                        let said = serde_json::json!({ "said": format!("no curve called {name}") });
                        self.json(conn, 404, |body| serde_json::to_writer(body, &said).is_ok()).await
                    }
                    Some(other) => self.said(conn, other).await,
                    None => Ok(()),
                }
            }
            Route::Energy => {
                let name = page::name_in(uri, "/energy/");
                let path = match page::csv_file(&name) {
                    Ok(path) => path,
                    Err(why) => return self.words(conn, 404, &why).await,
                };
                let Some((size, _)) = self.board.file(&path) else {
                    let text = format!("{name} is not on the board: the board keeps a CSV for each month it counted");
                    return self.words(conn, 404, &text).await;
                };
                let length = size.to_string();
                let disposition = format!("attachment; filename=\"{name}\"");
                let headers = [
                    ("Content-Type", "text/csv; charset=utf-8"),
                    ("Content-Length", length.as_str()),
                    ("Content-Disposition", disposition.as_str()),
                    ANY_ORIGIN,
                ];
                conn.initiate_response(200, None, &headers).await?;
                self.send_file(conn, &path, size).await.map_err(|_| edge_http::io::Error::ConnectionClosed)
            }
            Route::Journal => self.journal(conn, uri).await,
            Route::Events => self.stream(conn, uri).await,
            Route::Console => self.browser_console(conn).await,
        }
    }

    /// Answers what the owner of commands said to a command.
    async fn said<T: Read + Write, const N: usize>(&self, conn: &mut Connection<'_, T, N>, answer: Answer) -> Result<(), Io<T::Error>> {
        match answer {
            Answer::Json(value) => self.value(conn, &value).await,
            Answer::Refused(why) => self.value(conn, &serde_json::json!({ "error": why })).await,
            Answer::Done => self.value(conn, &serde_json::json!({})).await,
            Answer::Curve(_) | Answer::Waiting(_) | Answer::Tail(_) => {
                self.value(conn, &serde_json::json!({ "error": "an answer of another question where a command was asked" })).await
            }
        }
    }

    /// A command of contract 3: WHAT to the address after PREFIX, looked up in the board's dictionary and refused in
    /// words when the thing has no such address or takes no such command; carried out, it answers the values it left
    /// - the row and its parts.
    async fn by_address<T: Read + Write, const N: usize>(
        &self,
        conn: &mut Connection<'_, T, N>,
        uri: &str,
        what: &str,
        prefix: &str,
        said: &str,
    ) -> Result<(), Io<T::Error>> {
        let served = self.board.dictionary();
        let address = page::name_in(uri, prefix);
        let (at, command) = match page::dictionary::command(served.table, what, &address, said) {
            Ok(found) => found,
            Err(why) => return self.value(conn, &serde_json::json!({"error": why})).await,
        };
        let entry = served.table[at];
        match self.ordered(conn, Order::Act { name: entry.name, command }).await? {
            Some(Answer::Done) => {}
            Some(other) => return self.said(conn, other).await,
            None => return Ok(()),
        }
        let of_row =
            |a: usize| a == at || served.table.get(a).is_some_and(|e| e.card == entry.card && e.part_of == Some(entry.name));
        let cards = [entry.card];
        let here = Here(self.board);
        let wanted = page::Wanted::of(page::Values::cards(&here), &cards);
        self.json(conn, 200, |body| page::write_values_of(body, &here, &wanted, &of_row).is_ok()).await
    }

    /// /journal: the journal as text, oldest first, to its last line - the lines still waiting for their batch too.
    /// Its end the board holds in memory (`journal::kept::TAIL_BYTES`), and that is what it answers without reading
    /// the flash; `?all` answers the ring's files on the flash in order, then the lines waiting. The whole ring read
    /// every time was what a device's readers did on 26.09.
    ///
    /// **ONE READER AT A TIME.** A journal goes out as fast as the socket takes it, and every socket that sends at that
    /// speed holds its send buffer in the internal heap: two readers of it by turns took the floor of the heap from
    /// ~72 KB to 34 KB on `.30` (26.09). The next reader is refused at once and told to come back.
    async fn journal<T: Read + Write, const N: usize>(&self, conn: &mut Connection<'_, T, N>, uri: &str) -> Result<(), Io<T::Error>> {
        if self.journal_read.replace(true) {
            return self.words(conn, 503, "busy: the journal is being read by another reader - try again in a few seconds").await;
        }
        // let go however the answer ends: done, failed, or its future dropped
        struct Reading<'c>(&'c Cell<bool>);
        impl Drop for Reading<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reading = Reading(&self.journal_read);
        self.journal_once(conn, uri).await
    }

    async fn journal_once<T: Read + Write, const N: usize>(&self, conn: &mut Connection<'_, T, N>, uri: &str) -> Result<(), Io<T::Error>> {
        let all = uri.split_once('?').is_some_and(|(_, query)| query.split('&').any(|p| p == "all" || p.starts_with("all=")));
        if !all {
            return self.journal_tail(conn).await;
        }
        let waiting = match self.ordered(conn, Order::Waiting).await? {
            Some(Answer::Waiting(Some(waiting))) => waiting,
            Some(Answer::Waiting(None)) => return self.words(conn, 404, "this board keeps no journal on flash").await,
            Some(other) => return self.said(conn, other).await,
            None => return Ok(()),
        };
        let folder = journal::kept::FOLDER;
        let headers = [("Content-Type", "text/plain; charset=utf-8"), ANY_ORIGIN];
        conn.initiate_response(200, None, &headers).await?;
        let closed = |_| edge_http::io::Error::ConnectionClosed;
        let names = match self.board.files(folder) {
            Ok(names) => journal::kept::in_order(&names),
            Err(e) => {
                let text = format!("... {folder} could not be listed: {e} - its files are not read\n");
                self.write(conn, text.as_bytes()).await.map_err(closed)?;
                Vec::new()
            }
        };
        for name in names {
            let path = format!("{folder}/{name}");
            match self.board.file(&path) {
                Some((size, _)) => self.send_file(conn, &path, size).await.map_err(closed)?,
                None => self.write(conn, format!("... {path} could not be read\n").as_bytes()).await.map_err(closed)?,
            }
        }
        self.write(conn, waiting.as_bytes()).await.map_err(closed)
    }

    /// The end of the journal from memory, a piece at a time with a sleep between the pieces, as a file is sent.
    async fn journal_tail<T: Read + Write, const N: usize>(&self, conn: &mut Connection<'_, T, N>) -> Result<(), Io<T::Error>> {
        let tail = match self.ordered(conn, Order::Tail).await? {
            Some(Answer::Tail(Some(tail))) => tail,
            Some(Answer::Tail(None)) => return self.words(conn, 404, "this board keeps no journal on flash").await,
            Some(other) => return self.said(conn, other).await,
            None => return Ok(()),
        };
        let headers = [("Content-Type", "text/plain; charset=utf-8"), ANY_ORIGIN];
        conn.initiate_response(200, None, &headers).await?;
        let closed = |_| edge_http::io::Error::ConnectionClosed;
        if tail.left_out {
            let said = "... older lines are on the flash: /journal?all answers the whole journal\n";
            self.write(conn, said.as_bytes()).await.map_err(closed)?;
        }
        for piece in tail.text.as_bytes().chunks(PIECE) {
            self.write(conn, piece).await.map_err(closed)?;
            Timer::after(time(LET_GO)).await;
        }
        Ok(())
    }
}

// ---- /events ----

impl<B: Board> Loop<'_, B> {
    /// The cards that changed, as server-sent events, until the reader goes or a write takes longer than the limit.
    ///
    /// The first event is the whole board; after it, each carries the values this reader has not got yet - a value that
    /// changed twice since the last event goes once, as it is now. **NOT MORE OFTEN THAN THE PACE THE READER ASKED** (`?every=<s>`, the pace of everyone when it asks
    /// nothing): a change that comes before the pace allows is held, with the others, and goes with them at the end of it. Live (`every=0`) is every change at once,
    /// and ends by itself after `limits.live`: a reader that was forgotten does not keep the board at a full run. A quiet board sends a beat on the tick, and a
    /// reader that closed its socket is seen at once: the stream waits on the socket as well as on the changes.
    async fn stream<T: Read + Write + TcpSplit, const N: usize>(&self, conn: &mut Connection<'_, T, N>, uri: &str) -> Result<(), Io<T::Error>> {
        if self.streams.get() >= page::MAX_STREAMS {
            let headers = [("Content-Type", "text/plain; charset=utf-8"), ANY_ORIGIN];
            return self.answer(conn, 503, &headers, page::BUSY.as_bytes()).await;
        }
        if self.long_held() >= self.limits.long_holders {
            let headers = [("Content-Type", "text/plain; charset=utf-8"), ANY_ORIGIN];
            return self.answer(conn, 503, &headers, LONG_HELD.as_bytes()).await;
        }
        let Some(mut listener) = CHANGES.listen() else {
            let headers = [("Content-Type", "text/plain; charset=utf-8"), ANY_ORIGIN];
            return self.answer(conn, 503, &headers, page::BUSY.as_bytes()).await;
        };
        self.streams.set(self.streams.get() + 1);
        let _held = Held(&self.streams);
        let _streaming = Counted::new(&STREAMING);
        // Connection: Close makes the body the raw rest of the socket - no chunks -, so the socket can be read for
        // the reader's going while it is written
        let headers = [
            ("Content-Type", "text/event-stream"),
            ("Cache-Control", "no-cache"),
            ("Connection", "Close"),
            ANY_ORIGIN,
        ];
        conn.initiate_response(200, None, &headers).await?;
        let socket = conn.unbind()?;
        let (mut from, mut to) = socket.split();
        let mut sent = page::Sent::of(self.board.dictionary().table);
        // the pace of this reader: what it asked, and for Live, none until `limits.live` is over - then the pace of everyone
        let every = page::every_asked(uri);
        let (began, live_for, second) = (embassy_time::Instant::now(), time(self.limits.live), time(self.limits.second));
        let pace_at = |now: embassy_time::Instant| match every {
            0 if now.duration_since(began) < live_for => embassy_time::Duration::from_ticks(0),
            0 => second * page::dictionary::STREAM_EVERY_S,
            n => second * n,
        };
        let mut next_ok = embassy_time::Instant::now();
        let here = Here(self.board);
        let mut byte = [0u8; 16];
        loop {
            let marks = listener.take();
            let mut body = self.body();
            sent.at(embassy_time::Instant::now().as_millis());
            if marks.any() {
                let wanted = page::Wanted::new(page::Values::cards(&here), marks);
                if page::write_changed(&mut body, &here, &wanted, &mut sent).is_err() {
                    log::error!("an event could not be made whole");
                }
            } else {
                body.extend_from_slice(page::BEAT);
            }
            let wrote = if body.is_empty() { Ok(()) } else { self.write(&mut to, &body).await };
            let size = body.len();
            // an event sent starts the pace; a beat, and an event that came to nothing, do not
            if marks.any() && size > 0 {
                let now = embassy_time::Instant::now();
                next_ok = now + pace_at(now);
            }
            self.give_back(body);
            if wrote.is_err() {
                log::warn!("a stream let go: {size} bytes not taken within {:?}", self.limits.write);
                return Ok(());
            }
            match select3(listener.changed(), Timer::after(time(self.limits.beat)), from.read(&mut byte)).await {
                // a change before the pace allows is held, with whatever comes meanwhile, until it does - the reader's going is seen meanwhile as ever
                Either3::First(()) => {
                    if embassy_time::Instant::now() < next_ok {
                        match select(Timer::at(next_ok), from.read(&mut byte)).await {
                            Either::First(()) => {}
                            Either::Second(Ok(n)) if n > 0 => {}
                            Either::Second(end) => {
                                log::info!("a stream ended: its reader {}", if end.is_ok() { "closed" } else { "failed" });
                                return Ok(());
                            }
                        }
                    }
                }
                Either3::Second(()) => {}
                Either3::Third(Ok(n)) if n > 0 => {}
                Either3::Third(end) => {
                    log::info!("a stream ended: its reader {}", if end.is_ok() { "closed" } else { "failed" });
                    return Ok(());
                }
            }
        }
    }
}

/// A count held for as long as this lives.
struct Held<'a>(&'a Cell<usize>);

impl Drop for Held<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

// ---- consoles ----

/// How a console's bytes travel: in WebSocket frames, or as they are on port 23.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Framing {
    Ws,
    Raw,
}

/// What a stream is told when the handlers that may be held by long connections are all held.
const LONG_HELD: &str = "the board is busy: every long connection it can hold is open - close a page or a console and reload this one";
/// The same to a console of a browser, in the frame its console speaks in.
const LONG_HELD_CONSOLE: &[u8] = b"busy: every long connection the board can hold is open - close a page or a console and reload\r\n";

impl<B: Board> Loop<'_, B> {
    /// The connections that last and hold a handler of the page now: its streams and the consoles of browsers.
    fn long_held(&self) -> usize {
        self.streams.get() + self.consoles[..BROWSER_CONSOLES].iter().filter(|c| c.get()).count()
    }

    /// Takes a console place among FROM..TO, or None when all are held.
    fn take_console(&self, from: usize, to: usize) -> Option<usize> {
        let place = (from..to).find(|&p| !self.consoles[p].get())?;
        self.consoles[place].set(true);
        HELD.fetch_or(1 << place, core::sync::atomic::Ordering::AcqRel);
        Some(place)
    }

    fn give_console(&self, place: usize) {
        self.consoles[place].set(false);
        HELD.fetch_and(!(1 << place), core::sync::atomic::Ordering::AcqRel);
    }

    async fn browser_console<T: Read + Write + TcpSplit, const N: usize>(&self, conn: &mut Connection<'_, T, N>) -> Result<(), Io<T::Error>> {
        if !conn.is_ws_upgrade_request()? {
            return self.words(conn, 400, "the console is a WebSocket").await;
        }
        let mut key = [0u8; edge_http::ws::MAX_BASE64_KEY_RESPONSE_LEN];
        conn.initiate_ws_upgrade_response(&mut key).await?;
        conn.complete().await?;
        let socket = conn.unbind()?;
        let (from, mut to) = socket.split();
        if self.long_held() >= self.limits.long_holders {
            let _ = self.frame(&mut to, LONG_HELD_CONSOLE).await;
            log::warn!("browser console refused: the handlers kept for the short requests are all that is left");
            return Ok(());
        }
        let Some(place) = self.take_console(0, BROWSER_CONSOLES) else {
            let words = b"busy: both browser consoles are open - close one and reload\r\n";
            let _ = self.frame(&mut to, words).await;
            log::warn!("browser console refused: both are open");
            return Ok(());
        };
        log::info!("browser console {place} opened");
        self.console(place, from, to, Framing::Ws).await;
        log::info!("browser console {place} closed");
        Ok(())
    }

    /// Sends TEXT as one text frame within the write limit.
    async fn frame<W: Write>(&self, to: &mut W, text: &[u8]) -> Result<(), ()> {
        let header = edge_ws::FrameHeader { frame_type: edge_ws::FrameType::Text(false), payload_len: text.len() as u64, mask_key: None };
        let mut head = [0u8; edge_ws::FrameHeader::MAX_LEN];
        let len = header.serialize(&mut head).map_err(|_| ())?;
        self.write(to, &head[..len]).await?;
        self.write(to, text).await
    }

    /// Sends BYTES the way the console's FRAMING carries them; a frame carries whole characters only, and what is
    /// left of one cut by the pipe waits in CARRY for the next bytes.
    async fn say<W: Write>(&self, to: &mut W, framing: Framing, carry: &mut heapless::Vec<u8, 4>, bytes: &[u8]) -> Result<(), ()> {
        if framing == Framing::Raw {
            return self.write(to, bytes).await;
        }
        let mut whole = self.body();
        whole.extend_from_slice(carry);
        whole.extend_from_slice(bytes);
        carry.clear();
        let end = match core::str::from_utf8(&whole) {
            Ok(_) => whole.len(),
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            Err(_) => whole.len(),
        };
        let _ = carry.extend_from_slice(&whole[end..]);
        let sent = if end == 0 { Ok(()) } else { self.frame(to, &whole[..end]).await };
        self.give_back(whole);
        sent
    }

    /// Serves a console at PLACE until its reader goes: what it types goes to its shell in the owner's task, what the
    /// shell writes and what the log says come back.
    async fn console<R: Read, W: Write>(&self, place: usize, mut from: R, mut to: W, framing: Framing) {
        self.orders.opened(place);
        let reading = async {
            let mut opened = false;
            loop {
                if !opened {
                    match self.orders.ask(Order::Open(place), TYPED).await {
                        Ok(_) => opened = true,
                        Err(_) => {
                            self.orders.tell(place, b"busy: a command of another console is running - the prompt comes when it ends\r\n").await;
                            Timer::after(time(Duration::from_secs(1))).await;
                            continue;
                        }
                    }
                }
                let typed = match self.typed(&mut from, framing).await {
                    Typed::Gone => return,
                    Typed::TooLong => {
                        let words = format!("line longer than {} bytes refused\r\n", shell::MAX_LINE);
                        self.orders.tell(place, words.as_bytes()).await;
                        continue;
                    }
                    Typed::Line(typed) if typed.is_empty() => continue,
                    Typed::Line(typed) => typed,
                };
                if let Err(why) = self.orders.ask(Order::Feed(place, typed), TYPED).await {
                    let words: &[u8] = match why {
                        between::Refused::Busy => b"busy: a command of another console is still running - type it again when it ends\r\n",
                        between::Refused::Silent => b"the board did not answer in time\r\n",
                    };
                    self.orders.tell(place, words).await;
                }
            }
        };
        let mut cursor = self.ring.borrow().now();
        let writing = async {
            let mut buf = self.body();
            buf.resize(orders::PIPE, 0);
            let mut carry: heapless::Vec<u8, 4> = heapless::Vec::new();
            let gone = async {
                loop {
                    if let Either::First(n) = select(self.orders.heard(place, &mut buf), Timer::after(time(LOG_EVERY))).await {
                        if self.say(&mut to, framing, &mut carry, &buf[..n]).await.is_err() {
                            return;
                        }
                    }
                    loop {
                        let mut out = self.body();
                        let read = match self.ring.borrow().read(&mut cursor) {
                            Some(journal::Read::Line(line)) => {
                                out.extend_from_slice(line.as_bytes());
                                true
                            }
                            Some(journal::Read::Lost(n)) => {
                                out.extend_from_slice(format!("... {n} lines dropped: the reader was slower than the log").as_bytes());
                                true
                            }
                            None => false,
                        };
                        if read {
                            out.extend_from_slice(b"\r\n");
                        }
                        let sent = if read { self.say(&mut to, framing, &mut carry, &out).await } else { Ok(()) };
                        self.give_back(out);
                        if sent.is_err() {
                            return;
                        }
                        if !read {
                            break;
                        }
                    }
                }
            };
            gone.await;
            self.give_back(buf);
        };
        select(reading, writing).await;
        self.orders.closed(place);
        self.give_console(place);
    }

    /// What the reader typed next: a frame's text and a return, or the bytes as they came.
    async fn typed<R: Read>(&self, from: &mut R, framing: Framing) -> Typed {
        let mut out = heapless::Vec::new();
        match framing {
            Framing::Raw => {
                let mut buf = [0u8; shell::MAX_LINE];
                let n = match from.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => return Typed::Gone,
                };
                let _ = out.extend_from_slice(&buf[..n]);
            }
            Framing::Ws => {
                let Ok(header) = edge_ws::FrameHeader::recv(&mut *from).await else { return Typed::Gone };
                match header.frame_type {
                    edge_ws::FrameType::Close => return Typed::Gone,
                    edge_ws::FrameType::Text(_) | edge_ws::FrameType::Binary(_) => {}
                    _ => {
                        // a control frame carries at most 125 bytes
                        let mut skip = [0u8; 125];
                        return match header.recv_payload(&mut *from, &mut skip).await {
                            Ok(_) => Typed::Line(out),
                            Err(_) => Typed::Gone,
                        };
                    }
                }
                // a frame beyond LONGEST_FRAME is no line anyone typed, and it is not read at all: the console closes
                let mut buf = self.body();
                buf.resize(LONGEST_FRAME, 0);
                let typed = match header.recv_payload(&mut *from, &mut buf).await {
                    Err(_) => Typed::Gone,
                    Ok(payload) if payload.len() > shell::MAX_LINE => Typed::TooLong,
                    Ok(payload) => {
                        let _ = out.extend_from_slice(payload);
                        let _ = out.push(b'\r');
                        Typed::Line(out)
                    }
                };
                self.give_back(buf);
                return typed;
            }
        }
        Typed::Line(out)
    }
}

/// What a console's reader sent.
enum Typed {
    Line(heapless::Vec<u8, { orders::LINE }>),
    /// A frame longer than a line: refused in words, the console goes on.
    TooLong,
    Gone,
}

/// A frame the console reads to refuse it, at most.
const LONGEST_FRAME: usize = 1024;

/// A console on port 23: [`TELNET`] places, and a task more than places, which says "busy" to the next and lets it go.
async fn telnet_console<B: Board, T: TcpAccept>(at: &Loop<'_, B>, acceptor: &T) {
    loop {
        let Ok((peer, mut socket)) = acceptor.accept().await else {
            Timer::after(time(Duration::from_millis(100))).await;
            continue;
        };
        let (from, mut to) = socket.split();
        match at.take_console(BROWSER_CONSOLES, BROWSER_CONSOLES + PORT_CONSOLES) {
            Some(place) => {
                log::info!("console connection from {peer}");
                at.console(place, from, to, Framing::Raw).await;
                log::info!("console connection from {peer} ended");
            }
            None => {
                let _ = at.write(&mut to, b"busy: both console places are taken, try later\r\n").await;
                log::warn!("console connection from {peer} refused: no free place");
            }
        }
    }
}
