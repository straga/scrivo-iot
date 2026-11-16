//! The network's loop run on the host over std's sockets, as the board runs it over lwIP's: a made-up board of
//! values the test sets, a made-up owner of commands in its own thread, and clients that are plain sockets.
//!
//! Every test file is its own process: the loop's `CHANGES` is one static per program, as on the board.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use edge_nal::TcpBind;
use page::dictionary::{Entry, Kind, Said, Served, Value};
use page::Wanted;
use web::orders::{Answer, Hands, Order, Orders};
use web::{Board, Limits};

/// Numbers on the card `flame`, texts after them on the same card, and a pump on the card `relays`.
pub const NUMBERS: usize = 4;
pub const TEXTS: usize = 40;
pub const PUMP: usize = NUMBERS + TEXTS;

pub struct Made {
    pub values: Mutex<Vec<f32>>,
    pub texts: Mutex<Vec<String>>,
    pub pump: Mutex<bool>,
    pub restarts: AtomicU64,
    pub downs: AtomicU64,
    /// What every read of a piece of a file costs, in microseconds of CPU: the board's flash holds its core as long.
    pub piece_us: AtomicU64,
    /// The files of the journal kept on flash, by name.
    pub journal: Mutex<Vec<(String, Vec<u8>)>>,
    /// How many times a file of the journal was read.
    pub journal_reads: AtomicU64,
    /// The end of the journal the owner of the flash holds in memory.
    pub tail: Mutex<journal::kept::Tail>,
}


static CARDS: &[&str] = &["flame", "relays"];

fn table() -> &'static [Entry] {
    let mut t = Vec::new();
    // the first number is worth a whole one, the second a tenth, the rest are sent as they are
    for n in 0..NUMBERS {
        let digits = match n {
            0 => Some(0),
            1 => Some(1),
            _ => None,
        };
        t.push(Entry { unit: "u", decimals: digits, ..Entry::row("flame", Box::leak(format!("n{n}").into_boxed_str()), &[Kind::Number]) });
    }
    for n in 0..TEXTS {
        t.push(Entry::row("flame", Box::leak(format!("t{n}").into_boxed_str()), &[Kind::Text]));
    }
    t.push(Entry { can: &["on", "off", "toggle"], ..Entry::row("relays", "pump", &[Kind::On]) });
    Box::leak(t.into_boxed_slice())
}

impl Made {
    pub fn new() -> &'static Made {
        Box::leak(Box::new(Made {
            values: Mutex::new(vec![0.0; NUMBERS]),
            texts: Mutex::new(vec![String::new(); TEXTS]),
            pump: Mutex::new(false),
            restarts: AtomicU64::new(0),
            downs: AtomicU64::new(0),
            piece_us: AtomicU64::new(0),
            journal: Mutex::new(Vec::new()),
            journal_reads: AtomicU64::new(0),
            tail: Mutex::new(journal::kept::Tail::default()),
        }))
    }

    /// Sets the number at AT and says the card `flame` changed.
    pub fn set(&self, at: usize, value: f32) {
        self.values.lock().unwrap()[at] = value;
    }

    /// Sets every text to TEXT.
    pub fn texts(&self, text: &str) {
        self.texts.lock().unwrap().iter_mut().for_each(|t| *t = text.to_string());
    }
}

impl page::Values for Made {
    fn version(&self) -> &str {
        "v+00000000"
    }
    fn cards(&self) -> &'static [&'static str] {
        CARDS
    }
    fn values(&self, wanted: &Wanted, each: &mut dyn FnMut(usize, Said<'_>)) {
        if wanted.has("flame") {
            for (at, v) in self.values.lock().unwrap().iter().enumerate() {
                each(at, Value::Number(*v).into());
            }
            for (at, t) in self.texts.lock().unwrap().iter().enumerate() {
                each(NUMBERS + at, Said::Text(t));
            }
        }
        if wanted.has("relays") {
            each(PUMP, Value::On(*self.pump.lock().unwrap()).into());
        }
    }
}

static SERVED: std::sync::OnceLock<Served> = std::sync::OnceLock::new();

impl Board for Made {
    fn dictionary(&self) -> Served {
        *SERVED.get_or_init(|| Served { json: "{}", version: "v+00000000", table: table() })
    }
    fn board(&self) -> String {
        "HOST".into()
    }
    fn restart(&self) {
        self.restarts.fetch_add(1, Ordering::Relaxed);
    }
    fn down(&self) {
        self.downs.fetch_add(1, Ordering::Relaxed);
    }
    /// Every file of the page folder but a gzipped one: PAGE_FILE bytes of `x`, the size of the burner's app.js, and
    /// BIG_FILE of them for `big.js`; the journal's files as they were put.
    fn file(&self, path: &str) -> Option<(usize, u64)> {
        if let Some(name) = path.strip_prefix(journal::kept::FOLDER).and_then(|n| n.strip_prefix('/')) {
            let files = self.journal.lock().unwrap();
            return files.iter().find(|(n, _)| n == name).map(|(_, bytes)| (bytes.len(), 1));
        }
        let size = if path.ends_with("/big.js") { BIG_FILE } else { PAGE_FILE };
        (path.starts_with("/page/") && !path.ends_with(".gz")).then_some((size, 1))
    }
    fn read_at(&self, path: &str, at: usize, buf: &mut [u8]) -> Result<usize, String> {
        // a read of the board's flash holds its core: busy, not asleep
        let until = Instant::now() + Duration::from_micros(self.piece_us.load(Ordering::Relaxed));
        while Instant::now() < until {
            std::hint::spin_loop();
        }
        if let Some(name) = path.strip_prefix(journal::kept::FOLDER).and_then(|n| n.strip_prefix('/')) {
            self.journal_reads.fetch_add(1, Ordering::Relaxed);
            let files = self.journal.lock().unwrap();
            let (_, bytes) = files.iter().find(|(n, _)| n == name).ok_or("no such file")?;
            let rest = bytes.get(at..).unwrap_or(&[]);
            let n = buf.len().min(rest.len());
            buf[..n].copy_from_slice(&rest[..n]);
            return Ok(n);
        }
        let (size, _) = self.file(path).ok_or("no such file")?;
        let n = buf.len().min(size.saturating_sub(at));
        buf[..n].fill(b'x');
        Ok(n)
    }
    fn files(&self, dir: &str) -> Result<Vec<String>, String> {
        match dir == journal::kept::FOLDER {
            true => Ok(self.journal.lock().unwrap().iter().map(|(n, _)| n.clone()).collect()),
            false => Ok(Vec::new()),
        }
    }
}

/// The owner of commands: a switch of the pump, and a console whose line `long` takes LONG and `short` takes SHORT.
pub struct MadeHands {
    pub board: &'static Made,
    pub shells: [bool; web::orders::CONSOLES],
}

pub const LONG: Duration = Duration::from_secs(3);
pub const SHORT: Duration = Duration::from_millis(100);

impl Hands for MadeHands {
    fn carry(&mut self, order: Order, out: &Orders) -> Answer {
        match order {
            Order::Act { name: "pump", command } => {
                let mut pump = self.board.pump.lock().unwrap();
                *pump = match command {
                    page::dictionary::Command::Switch(on) => on,
                    page::dictionary::Command::Toggle => !*pump,
                    _ => return Answer::Refused("pump takes switch, toggle".into()),
                };
                Answer::Done
            }
            Order::Open(place) => {
                self.shells[place] = true;
                out.say(place, b"rust> ");
                Answer::Done
            }
            Order::Feed(place, bytes) => {
                let line = String::from_utf8_lossy(&bytes).trim().to_string();
                match line.as_str() {
                    "long" => std::thread::sleep(LONG),
                    "short" => std::thread::sleep(SHORT),
                    _ => {}
                }
                if !line.is_empty() {
                    out.say(place, format!("{line} done\r\nrust> ").as_bytes());
                }
                Answer::Done
            }
            Order::Waiting => Answer::Waiting(Some("a line not written yet\n".into())),
            Order::Tail => Answer::Tail(Some(self.board.tail.lock().unwrap().clone())),
            other => Answer::Refused(format!("not made: {other:?}")),
        }
    }
}

/// The size of every page file the made board holds.
pub const PAGE_FILE: usize = 20_885;
/// The size of `big.js`: as long as the burner's /journal, 64 pieces.
pub const BIG_FILE: usize = 256 * 1024;

pub struct Running {
    pub board: &'static Made,
    /// The log's lines into this loop, as the firmware's logger puts them: a feed a loop, since every loop takes them.
    pub log: &'static web::LogFeed,
    pub http: SocketAddr,
    pub telnet: SocketAddr,
}

fn free_port() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap()
}

/// The loop's log on stderr, with the time since the test began: what a failed run is read by.
struct Printed(Instant);

impl log::Log for Printed {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        eprintln!("{:>8.3} {} {}", self.0.elapsed().as_secs_f32(), record.level(), record.args());
    }
    fn flush(&self) {}
}

/// Starts the loop with LIMITS and its owner of commands, each in a thread of its own.
pub fn start(limits: Limits) -> Running {
    if log::set_boxed_logger(Box::new(Printed(Instant::now()))).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
    let board = Made::new();
    let orders: &'static Orders = Box::leak(Box::new(Orders::new()));
    std::thread::spawn(move || {
        let mut hands = MadeHands { board, shells: [false; web::orders::CONSOLES] };
        orders.serve(&mut hands)
    });
    let (http, telnet) = (free_port(), free_port());
    let log: &'static web::LogFeed = Box::leak(Box::new(web::LogFeed::new()));
    std::thread::spawn(move || {
        let serving = async {
            let stack = edge_nal_std::Stack::new();
            let page = stack.bind(http).await.unwrap();
            let port = stack.bind(telnet).await.unwrap();
            web::serve(board, orders, log, page, port, limits).await
        };
        between::block_on_stepped(serving, &web::STEPS, web::STEP_MOST)
    });
    // the loop is up when it answers
    let until = Instant::now() + Duration::from_secs(5);
    while TcpStream::connect(http).is_err() {
        assert!(Instant::now() < until, "the loop did not come up");
        std::thread::sleep(Duration::from_millis(20));
    }
    Running { board, log, http, telnet }
}

/// The limits of a test; WEB_TEST_WRITE_MS, when set, stands for the write limit - how a run shows that a test sees
/// the limit at all.
pub fn limits(write: Duration) -> Limits {
    let write = std::env::var("WEB_TEST_WRITE_MS").ok().and_then(|ms| ms.parse().ok()).map(Duration::from_millis).unwrap_or(write);
    // a second of the pace a reader asks for is a tenth of one here, and Live is long enough for a test to be over before it is
    Limits {
        write,
        beat: Duration::from_secs(5),
        going: Duration::from_millis(100),
        wait_for_a_handler: Duration::from_secs(3),
        second: Duration::from_millis(100),
        live: Duration::from_secs(60),
        long_holders: plan::LARGE.long_holders(),
    }
}

/// Opens a stream that hears every change (`every=0`, Live): the socket, after its answer's head was read.
pub fn stream(at: SocketAddr) -> TcpStream {
    stream_asking(at, "?every=0")
}

/// Opens a stream asking for QUERY (`?every=2`, or nothing: the pace of everyone).
pub fn stream_asking(at: SocketAddr, query: &str) -> TcpStream {
    let mut s = TcpStream::connect(at).unwrap();
    s.write_all(format!("GET /events{query} HTTP/1.1\r\nHost: board\r\n\r\n").as_bytes()).unwrap();
    s
}

/// Reads from S for up to WITHIN; what came, as text.
pub fn read_for(s: &mut TcpStream, within: Duration) -> String {
    let mut out = Vec::new();
    let until = Instant::now() + within;
    let mut buf = [0u8; 4096];
    while Instant::now() < until {
        s.set_read_timeout(Some((until - Instant::now()).max(Duration::from_millis(1)))).unwrap();
        match s.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// One request and its whole answer, on a connection of its own; None when nothing came within WITHIN.
pub fn ask(at: SocketAddr, request: &str, within: Duration) -> Option<String> {
    let mut s = TcpStream::connect(at).ok()?;
    s.write_all(request.as_bytes()).ok()?;
    s.set_read_timeout(Some(within)).ok()?;
    answer_of(&mut s)
}

/// Reads one answer with a Content-Length off S.
pub fn answer_of(s: &mut TcpStream) -> Option<String> {
    let mut got = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let text = String::from_utf8_lossy(&got).into_owned();
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length: ").map(|n| n.trim().parse::<usize>().unwrap()))
                .unwrap_or(0);
            if got.len() >= end + 4 + length {
                return Some(text[..end + 4 + length].to_string());
            }
        }
        match s.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => got.extend_from_slice(&buf[..n]),
        }
    }
}

pub fn post(path: &str) -> String {
    format!("POST {path} HTTP/1.1\r\nHost: board\r\nContent-Length: 0\r\n\r\n")
}

// ---- a console in a browser ----

/// A browser console: the upgrade answered; what came after the answer's head, as frames' text.
pub fn browser_said(run: &Running) -> (TcpStream, String) {
    let mut c = TcpStream::connect(run.http).unwrap();
    c.write_all(
        b"GET /console HTTP/1.1\r\nHost: board\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
          Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
    )
    .unwrap();
    let raw = read_raw(&mut c, Duration::from_millis(500));
    assert!(raw.starts_with(b"HTTP/1.1 101"), "the console was not upgraded: {:?}", String::from_utf8_lossy(&raw));
    let head = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    (c, text_of(&raw[head..]))
}

pub fn browser(run: &Running) -> TcpStream {
    browser_said(run).0
}

pub fn read_raw(c: &mut TcpStream, within: Duration) -> Vec<u8> {
    let mut out = Vec::new();
    let until = Instant::now() + within;
    let mut buf = [0u8; 4096];
    while Instant::now() < until {
        c.set_read_timeout(Some((until - Instant::now()).max(Duration::from_millis(1)))).unwrap();
        match c.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
        }
    }
    out
}

/// A text frame from a browser: masked, as a browser sends it.
pub fn typed(c: &mut TcpStream, text: &str) {
    let mask = [1u8, 2, 3, 4];
    let mut frame = vec![0x81];
    match text.len() {
        n if n < 126 => frame.push(0x80 | n as u8),
        n => {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(n as u16).to_be_bytes());
        }
    }
    frame.extend_from_slice(&mask);
    frame.extend(text.bytes().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    c.write_all(&frame).unwrap();
}

/// The text of the frames that came within WITHIN.
pub fn frames(c: &mut TcpStream, within: Duration) -> String {
    text_of(&read_raw(c, within))
}

/// The text of the server's frames in RAW.
pub fn text_of(raw: &[u8]) -> String {
    let (mut at, mut out) = (0, String::new());
    while at + 2 <= raw.len() {
        let (len, head) = match raw[at + 1] & 0x7f {
            126 => (u16::from_be_bytes([raw[at + 2], raw[at + 3]]) as usize, 4),
            n => (n as usize, 2),
        };
        let end = (at + head + len).min(raw.len());
        out.push_str(&String::from_utf8_lossy(&raw[at + head..end]));
        at = end;
    }
    out
}
