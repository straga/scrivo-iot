//! The board's clock: the time from SNTP and the local zone, both settings (crates/settings TIME, ZONE) - the servers
//! made from the way a person chose, the POSIX rule from the city. SNTP starts once the station has an address -
//! `gateway` in the list is the router that gave it - and starts anew with the new list when the setting changes. Until a server has answered the board says it has no exact
//! time, rather than counting days from 1970.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use between::Lock;
use embassy_sync::signal::Signal;

use esp_idf_svc::sys;

use crate::nvs_settings;

/// The local time as the board knows it.
#[derive(Clone, Copy, Debug)]
pub struct Local {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub minute: u16,
    pub second: u8,
    /// Minutes east of UTC.
    pub offset_min: i16,
}

/// A server has answered since the start: the clock is right.
static SYNCED: AtomicBool = AtomicBool::new(false);
/// When a server last answered, seconds since 1970 - in u32 until 2106, the ESP32 has no 64-bit atomics; 0 before any
/// did.
static LAST_ANSWER: AtomicU32 = AtomicU32::new(0);
/// How many answers of a server there have been since the start; a count that places itself by the time of day tells a fresh answer by it changing (crates/tally, `Watch`).
static ANSWERS: AtomicU32 = AtomicU32::new(0);

/// The chip's counter leapt and no server has answered since: the time of day is a leap's leftover until one does. Set by
/// `leapt`, cleared by `synced`; read by `answered_since_leap` and by nothing else, so it changes nothing of the time.
static LEAP_UNANSWERED: AtomicBool = AtomicBool::new(false);

/// The servers SNTP asks, as the setting names them, and the router's address: what the owner of SNTP - the task `main`,
/// which joins the network (`net::run`) - last started it with. Published for every reader; nobody else writes it.
#[derive(Clone, Default)]
pub struct Servers {
    said: Vec<String>,
    gateway: Option<String>,
}

pub static SERVERS: module::Role<std::sync::Arc<Servers>> = module::Role::new("sntp servers");

/// The setting of the time changed, or the chip's counter leapt: the owner of SNTP starts it anew on its next look
/// (`Sntp::wait`).
static ASKED: Signal<Lock, ()> = Signal::new();
/// The asking is a leap's: its line is a warning, which the journal on flash keeps (it keeps no `info` of this module).
static LEAPT: AtomicBool = AtomicBool::new(false);

fn servers() -> std::sync::Arc<Servers> {
    SERVERS.latest().unwrap_or_default()
}

/// Sets the zone from the setting: the city's rule as TZ, and tzset, as newlib reads it. Called at start and when it
/// changes.
pub fn set_zone() {
    let tz = settings::posix(&nvs_settings::value(&settings::ZONE)).to_string();
    let (Ok(key), Ok(value)) = (CString::new("TZ"), CString::new(tz.as_str())) else { return };
    unsafe {
        sys::setenv(key.as_ptr(), value.as_ptr(), 1);
        sys::tzset();
    }
    log::info!("time zone {tz}");
}

// Which server answered is not asked here: lwIP calls this before it marks the answer in the server's reachability
// register (sntp.c, sntp_recv: sntp_process, then reachability |= 1), so here it still reads as unanswered.
unsafe extern "C" fn synced(tv: *mut sys::timeval) {
    if let Some(tv) = unsafe { tv.as_ref() } {
        LAST_ANSWER.store(u32::try_from(tv.tv_sec).unwrap_or(0), Ordering::Relaxed);
    }
    SYNCED.store(true, Ordering::Relaxed);
    LEAP_UNANSWERED.store(false, Ordering::Release);
    // last: whoever reads the count after it changed reads a time that is the answer's already
    ANSWERS.fetch_add(1, Ordering::Release);
}

/// SNTP as its owner holds it: the names handed to lwIP, which keeps the pointers, and the router's address.
#[derive(Default)]
pub struct Sntp {
    given: Vec<CString>,
    gateway: Option<String>,
}

impl Sntp {
    /// Starts SNTP on the servers of the setting, anew if it ran; GATEWAY is the router's address when the station has
    /// one, and None keeps the one known.
    pub fn start(&mut self, gateway: Option<String>) {
        self.begin(gateway, log::Level::Info);
    }

    fn begin(&mut self, gateway: Option<String>, said_at: log::Level) {
        if gateway.is_some() {
            self.gateway = gateway;
        }
        let said = settings::servers(&nvs_settings::value(&settings::TIME));
        let given: Vec<CString> = said
            .iter()
            .filter_map(|name| match name.as_str() {
                "gateway" => self.gateway.clone(),
                other => Some(other.to_string()),
            })
            .filter_map(|name| CString::new(name).ok())
            .collect();
        unsafe {
            if sys::esp_sntp_enabled() {
                sys::esp_sntp_stop();
            }
            sys::esp_sntp_setoperatingmode(sys::esp_sntp_operatingmode_t_ESP_SNTP_OPMODE_POLL);
            for i in 0..settings::MAX_SERVERS {
                let name = given.get(i).map_or(core::ptr::null(), |c| c.as_ptr());
                sys::esp_sntp_setservername(i as u8, name);
            }
            sys::sntp_set_time_sync_notification_cb(Some(synced));
            sys::esp_sntp_init();
        }
        log::log!(said_at, "sntp on {}", given.iter().map(|c| c.to_string_lossy()).collect::<Vec<_>>().join(", "));
        // the old names are dropped only now: lwIP was handed the new ones above
        self.given = given;
        SERVERS.publish(Some(std::sync::Arc::new(Servers { said, gateway: self.gateway.clone() })));
    }

    /// Waits up to FOR for the setting of the time to change, and starts SNTP anew when it did - what the owner does
    /// instead of sleeping between its looks at the link.
    pub fn wait(&mut self, r#for: std::time::Duration) {
        let within = embassy_time::Duration::from_micros(r#for.as_micros() as u64);
        if between::block_on(embassy_time::with_timeout(within, ASKED.wait())).is_ok() {
            // a leap's asking is told in the journal on flash, where the line says that the time of day was asked for again
            let level = if LEAPT.swap(false, Ordering::AcqRel) { log::Level::Warn } else { log::Level::Info };
            self.begin(None, level);
        }
    }
}

/// The chip's counter leapt: the time of day is the counter's plus an offset (IDF, esp_time_impl.c), so it leapt with it
/// and stays wrong until SNTP asks again - an hour later at its pace (CONFIG_LWIP_SNTP_UPDATE_DELAY), and the timetable
/// of a module that goes by the time of day goes by it. The owner of SNTP starts it anew on its next look, which asks at once.
pub fn leapt() {
    LEAP_UNANSWERED.store(true, Ordering::Release);
    LEAPT.store(true, Ordering::Release);
    ASKED.signal(());
}

/// Whether a server has answered since the chip's counter last leapt - or it has not leapt since the start: the time of
/// day is no leap's leftover. Read only: it says what happened to the time and changes nothing of it. A count that
/// places itself by the time of day believes the time only when this is true (crates/tally, `Watch`).
pub fn answered_since_leap() -> bool {
    !LEAP_UNANSWERED.load(Ordering::Acquire)
}

/// How many answers of a server there have been since the start. A reader that wants the time an answer set reads this BEFORE the time: the count moves after the time is set.
pub fn answers() -> u32 {
    ANSWERS.load(Ordering::Acquire)
}

/// What is done when a setting of the clock changes.
pub fn changed(name: &str) {
    if name == settings::TIME.name {
        ASKED.signal(());
    } else if name == settings::ZONE.name {
        set_zone();
    }
}

/// What the clock knows, as data: the time, where it came from, who is silent. What a person reads of it is the
/// client's to make; `words` below is the console's.
pub struct Heard {
    /// The local time, `2026-09-24 18:40`, once a server has answered.
    pub now: Option<String>,
    pub zone: String,
    /// The server that answers, as the setting names it: `gateway` for the router, else its host.
    pub from: Option<String>,
    /// The router's address, when it is known.
    pub gateway: Option<String>,
    /// The servers asked, as the setting names them.
    pub asked: Vec<String>,
    /// When a server last answered, `18:40`.
    pub last_at: Option<String>,
    /// Whether any server has answered since the start.
    pub synced: bool,
}

pub fn heard() -> Heard {
    let from = answered_by().map(|(said, _)| said);
    let (asked, gateway) = {
        let s = servers();
        (s.said.clone(), s.gateway.clone())
    };
    let last = LAST_ANSWER.load(Ordering::Relaxed);
    let last_at = (last > 0).then(|| {
        let at = sys::time_t::from(last);
        let mut local: sys::tm = unsafe { core::mem::zeroed() };
        unsafe { sys::localtime_r(&at, &mut local) };
        format!("{:02}:{:02}", local.tm_hour, local.tm_min)
    });
    Heard {
        now: now().map(|t| format!("{:04}-{:02}-{:02} {:02}:{:02}", t.year, t.month, t.day, t.minute / 60, t.minute % 60)),
        zone: zone_name(),
        from,
        gateway,
        asked,
        last_at,
        synced: SYNCED.load(Ordering::Relaxed),
    }
}

/// The local time, None until a server has answered.
pub fn now() -> Option<Local> {
    if !SYNCED.load(Ordering::Relaxed) {
        return None;
    }
    let t = unsafe { sys::time(core::ptr::null_mut()) };
    let (mut local, mut utc): (sys::tm, sys::tm) = unsafe { (core::mem::zeroed(), core::mem::zeroed()) };
    unsafe {
        sys::localtime_r(&t, &mut local);
        sys::gmtime_r(&t, &mut utc);
    }
    // newlib's tm has no gmtoff: the offset is local minus UTC, across a day's turn either way
    let day_turn = match (local.tm_year, local.tm_yday) {
        l if l == (utc.tm_year, utc.tm_yday) => 0,
        l if l > (utc.tm_year, utc.tm_yday) => 1,
        _ => -1,
    };
    let offset = day_turn * 1440 + (local.tm_hour * 60 + local.tm_min) - (utc.tm_hour * 60 + utc.tm_min);
    Some(Local {
        year: (local.tm_year + 1900) as u16,
        month: (local.tm_mon + 1) as u8,
        day: local.tm_mday as u8,
        minute: (local.tm_hour * 60 + local.tm_min) as u16,
        second: local.tm_sec as u8,
        offset_min: offset as i16,
    })
}

/// The first server that answered any of its last 8 polls (its reachability register, RFC 5905), as a person reads
/// it - "the router (192.0.2.1)", "pool.ntp.org".
fn answered() -> Option<String> {
    answered_by().map(|(said, address)| match said.as_str() {
        "gateway" => format!("the router ({})", address.unwrap_or_default()),
        _ => said,
    })
}

/// The same server by the name the setting gives it, and the address it was asked at.
fn answered_by() -> Option<(String, Option<String>)> {
    let i = (0..settings::MAX_SERVERS as u8).find(|&i| unsafe { sys::esp_sntp_getreachability(i) } != 0)? as usize;
    let s = servers();
    let mut given = 0;
    for said in &s.said {
        let address = if said == "gateway" { s.gateway.clone() } else { Some(said.clone()) };
        if address.is_none() {
            continue;
        }
        if given == i {
            return Some((said.clone(), address));
        }
        given += 1;
    }
    None
}

/// The line `time` of the page: the local time and the server it came from, or that there is none and which servers
/// do not answer.
pub fn words() -> String {
    match now() {
        Some(t) => {
            let zone = zone_name();
            let from = answered().map_or(String::new(), |s| format!(" \u{b7} from {s}"));
            format!("{:04}-{:02}-{:02} {:02}:{:02} {zone}{from}", t.year, t.month, t.day, t.minute / 60, t.minute % 60)
        }
        None => {
            let s = servers();
            if s.said.is_empty() {
                return "no exact time: not in the network yet".into();
            }
            let named: Vec<String> = s
                .said
                .iter()
                .map(|n| match (n.as_str(), &s.gateway) {
                    ("gateway", Some(g)) => format!("the router ({g})"),
                    _ => n.clone(),
                })
                .collect();
            format!("no exact time: {} do not answer - choose where the time comes from on the tab Settings", named.join(", "))
        }
    }
}

/// The zone's name now - EEST in summer, EET in winter - as tzset read it.
fn zone_name() -> String {
    let t = unsafe { sys::time(core::ptr::null_mut()) };
    let mut local: sys::tm = unsafe { core::mem::zeroed() };
    let mut buf = [0u8; 16];
    let n = unsafe {
        sys::localtime_r(&t, &mut local);
        sys::strftime(buf.as_mut_ptr().cast(), buf.len(), c"%Z".as_ptr(), &local)
    };
    String::from_utf8_lossy(&buf[..n]).into_owned()
}
