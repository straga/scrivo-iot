//! WHICH PROFILE STANDS NOW: a profile is a name, a window says which profile stands in it, and the board's wall clock says which window is open. What a profile stands for is not here: for a room it is a target, for the schedule as
//! the source of the demand it is whether the house is asked to burn (`crates/demand`) - as Plum's time spans and ESPHome's presets name a setting and leave the holding to others. Nothing here lights or puts out anything.
//!
//! **A WINDOW BELONGS TO THE DAY IT STARTS ON.** One that crosses midnight on a Friday is still open on the Saturday at three, and a
//! Saturday that is no day of it opens none. Where two windows are open at once the lower number stands: the windows are a list a person
//! reads from the top, and the one nearer the top is the one that was meant.
//!
//! **A WINDOW IS WRITTEN AS TEXT** - `mon-fri 22:00-06:00 eco`, `daily 07:00+3h/20m comfort` - and [`Window::parse`] is the one door it comes
//! in by. The card keeps the text, the page shows and takes it, and this crate is the only place that knows what it means.
//!
//! NEVER HERE: a target, a hysteresis, the water, the demand, where the time comes from (the board's clock), what a profile is called on a page (the page's table).

/// How many windows the card keeps.
pub const WINDOWS: usize = 6;

/// A name a person has given to what the house is asked for: the number or the word it stands for belongs to whoever reads the profile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    /// The house as it is wanted by day.
    Comfort,
    /// The hours the house can do with less.
    Eco,
    /// The house is not asked for heat.
    Off,
}

/// Every profile by the name a client reads it by.
pub const PROFILES: [&str; 3] = ["comfort", "eco", "off"];

/// The name of a profile, from [`PROFILES`].
pub fn profile_word(profile: Profile) -> &'static str {
    match profile {
        Profile::Comfort => PROFILES[0],
        Profile::Eco => PROFILES[1],
        Profile::Off => PROFILES[2],
    }
}

/// The days by the name a window is written with, Monday first.
const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

const MINUTES_A_DAY: u16 = 24 * 60;

/// The local time of the board: the day of the week, counted from Monday as nought, and the minute of the day.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Wall {
    pub day: u8,
    pub minute: u16,
}

impl Wall {
    /// The wall of the local date YEAR-MONTH-DAY at MINUTE of it. The clock of the board says a date and a minute and no day of the week.
    pub fn of_date(year: u16, month: u8, day: u8, minute: u16) -> Wall {
        // the days from 1970-01-01 to the date in the civil calendar, the proleptic Gregorian one (H. Hinnant's days_from_civil)
        let (y, m, d) = (i64::from(year) - i64::from(month <= 2), i64::from(month), i64::from(day));
        let era = y.div_euclid(400);
        let year_of_era = y - era * 400;
        let day_of_year = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        let days = era * 146_097 + day_of_era - 719_468;
        // 1970-01-01 was a Thursday, the fourth day counted from Monday as nought
        Wall { day: (days + 3).rem_euclid(7) as u8, minute }
    }
}

/// When in the day a window is open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Span {
    /// From a minute of the day to another, the second the first minute outside it. A TO that is not after FROM runs over midnight into the next day.
    Between { from: u16, to: u16 },
    /// From a minute of the day, every EVERY_H hours, for FOR_MIN minutes each time: the boiler lifted every two or three hours. The count starts again every day at FROM (see `Window::active`).
    Every { from: u16, every_h: u8, for_min: u16 },
}

/// A window of the schedule: the days it starts on, when in the day, and the profile that stands in it. A window with no profile is written
/// and off: it is passed over whole.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Window {
    /// The days it starts on, Monday the lowest bit.
    pub days: u8,
    pub span: Span,
    pub profile: Option<Profile>,
}

impl Window {
    /// The window a text writes, or why it is none. The text is `<days> <from>-<to> <profile>` or `<days> <from>+<N>h/<M>m <profile>`.
    pub fn parse(text: &str) -> Result<Window, &'static str> {
        let text = text.to_ascii_lowercase();
        let mut words = text.split_whitespace();
        let (Some(days), Some(span), Some(profile), None) = (words.next(), words.next(), words.next(), words.next()) else {
            return Err("a window is days, times and a profile");
        };
        Ok(Window { days: days_of(days)?, span: span_of(span)?, profile: profile_of(profile)? })
    }

    /// Whether the window is open at AT: the minute of the day it ends at, none when it is not open - and none always for a window with no profile.
    pub fn active(&self, at: Wall) -> Option<u16> {
        self.profile?;
        let on = |day: u8| self.days >> (day % 7) & 1 == 1;
        let (today, yesterday) = (on(at.day), on(at.day + 6));
        let m = at.minute;
        match self.span {
            Span::Between { from, to } => {
                if from == to || from >= MINUTES_A_DAY || to >= MINUTES_A_DAY {
                    return None;
                }
                let open = if from < to { today && (from..to).contains(&m) } else { (today && m >= from) || (yesterday && m < to) };
                open.then_some(to)
            }
            Span::Every { from, every_h, for_min } => {
                let gap = u16::from(every_h) * 60;
                if gap == 0 || from >= MINUTES_A_DAY {
                    return None;
                }
                // THE COUNT STARTS AGAIN EVERY DAY AT ITS OWN TIME: the starts of a day are FROM and every GAP minutes after it, up to the next day's FROM - through the night, and not
                // on through the days. A start belongs to the day the count began on, so the one at one o'clock belongs to the day before. A count that went on through the days would move the times
                // by the hours that do not divide the day; this one says the same times every day, and where the hours divide the day the two are one.
                for back in 0..=2u16 {
                    let day = at.day % 7 + 7 - (back % 7) as u8;
                    if !on(day) {
                        continue;
                    }
                    // the minute now is, counted from midnight of the day the count began on
                    let now = m + back * MINUTES_A_DAY;
                    if now < from {
                        continue;
                    }
                    let behind = (now - from) / gap * gap;
                    if behind >= MINUTES_A_DAY {
                        continue;
                    }
                    let end = from + behind + for_min;
                    if now < end {
                        return Some(end % MINUTES_A_DAY);
                    }
                }
                None
            }
        }
    }
}

/// The bits of the days a text names: `daily`, or names, ranges and lists of them - `mon,wed`, `mon-fri`, `sat-mon` (a range that runs past Sunday comes round).
fn days_of(text: &str) -> Result<u8, &'static str> {
    if text == "daily" {
        return Ok(0b111_1111);
    }
    let day = |name: &str| DAYS.iter().position(|d| *d == name).ok_or("no such day: the days are mon tue wed thu fri sat sun, or daily");
    let mut bits = 0u8;
    for part in text.split(',') {
        match part.split_once('-') {
            Some((from, to)) => {
                let (from, to) = (day(from)?, day(to)?);
                let mut at = from;
                loop {
                    bits |= 1 << at;
                    if at == to {
                        break;
                    }
                    at = (at + 1) % 7;
                }
            }
            None => bits |= 1 << day(part)?,
        }
    }
    Ok(bits)
}

/// A minute of the day written `HH:MM`, and no other way: a time a person reads back as it was typed.
fn time_of(text: &str) -> Result<u16, &'static str> {
    let bytes = text.as_bytes();
    let digits = |from: usize| bytes[from..from + 2].iter().all(u8::is_ascii_digit);
    if bytes.len() != 5 || bytes[2] != b':' || !digits(0) || !digits(3) {
        return Err("a time is written HH:MM");
    }
    let (h, m) = (text[..2].parse::<u16>().unwrap_or(99), text[3..].parse::<u16>().unwrap_or(99));
    if h > 23 || m > 59 {
        return Err("a time is from 00:00 to 23:59");
    }
    Ok(h * 60 + m)
}

fn span_of(text: &str) -> Result<Span, &'static str> {
    if let Some((from, rest)) = text.split_once('+') {
        let from = time_of(from)?;
        let (hours, minutes) = rest.split_once('/').ok_or("every is written +<N>h/<M>m")?;
        let every_h = hours.strip_suffix('h').and_then(|n| n.parse::<u8>().ok()).ok_or("every is written +<N>h/<M>m")?;
        let for_min = minutes.strip_suffix('m').and_then(|n| n.parse::<u16>().ok()).ok_or("every is written +<N>h/<M>m")?;
        if !(1..=24).contains(&every_h) {
            return Err("every is from 1 to 24 hours");
        }
        if for_min == 0 || for_min > u16::from(every_h) * 60 {
            return Err("the minutes are from 1 to the hours between starts");
        }
        return Ok(Span::Every { from, every_h, for_min });
    }
    let (from, to) = text.split_once('-').ok_or("the times are <from>-<to> or <from>+<N>h/<M>m")?;
    let (from, to) = (time_of(from)?, time_of(to)?);
    if from == to {
        return Err("a window is not empty and not the whole day: from is not to");
    }
    Ok(Span::Between { from, to })
}

fn profile_of(text: &str) -> Result<Option<Profile>, &'static str> {
    match text {
        "comfort" => Ok(Some(Profile::Comfort)),
        "eco" => Ok(Some(Profile::Eco)),
        "off" => Ok(Some(Profile::Off)),
        "none" => Ok(None),
        _ => Err("the profile is comfort, eco, off or none"),
    }
}

/// The windows that choose between the profiles, and the profile that stands outside them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Schedule {
    /// The profile that stands outside every window.
    pub main: Profile,
    /// The windows in the order the card lists them; a place with none is a window nobody wrote.
    pub windows: [Option<Window>; WINDOWS],
}

impl Default for Schedule {
    /// No window and the comfort profile: the house as it was before the schedule.
    fn default() -> Self {
        Schedule { main: Profile::Comfort, windows: [None; WINDOWS] }
    }
}

/// What chose the profile that stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// The main profile: no window is open, or none is written.
    Main,
    /// The window of this number, from 1.
    Window(u8),
    /// The main profile, because windows are written and the board does not know the time.
    NoClock,
}

/// Every kind of source by the name a client reads it by: a window is `window` whatever its number, which a row of its own carries.
pub const SOURCES: [&str; 3] = ["main", "window", "no_clock"];

/// The name of the kind of a source, from [`SOURCES`].
pub fn source_word(source: Source) -> &'static str {
    match source {
        Source::Main => SOURCES[0],
        Source::Window(_) => SOURCES[1],
        Source::NoClock => SOURCES[2],
    }
}

impl Schedule {
    /// Whether a profile is one the schedule can stand on: the main one, or the one of a window in use.
    pub fn uses(&self, profile: Profile) -> bool {
        self.main == profile || self.windows.iter().flatten().any(|w| w.profile == Some(profile))
    }

    /// The profile that stands at WALL, what chose it, and the minute of the day the window in force ends at.
    pub fn pick(&self, wall: Option<Wall>) -> (Profile, Source, Option<u16>) {
        let in_use = self.windows.iter().flatten().any(|w| w.profile.is_some());
        let Some(wall) = wall else {
            return (self.main, if in_use { Source::NoClock } else { Source::Main }, None);
        };
        for (at, window) in self.windows.iter().enumerate() {
            if let Some((window, until)) = window.and_then(|w| w.active(wall).map(|until| (w, until))) {
                if let Some(profile) = window.profile {
                    return (profile, Source::Window(at as u8 + 1), Some(until));
                }
            }
        }
        (self.main, Source::Main, None)
    }
}
