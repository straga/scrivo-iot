//! A cleaning the count was not there for: the day of it and the total of the pellets when it was made (`cleaned 2026-11-08 82.4`).
//! Here the words of the console are read and the refusals are named; what the count does with them is `Soot::cleaned_before`.

use tally::Date;

/// Why a past cleaning is not taken: a code for a client, the words for a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Not a day and a total.
    Usage,
    /// The first word is no day.
    BadDay,
    /// The second word is no number of kilograms.
    BadKg,
    /// The board does not know today's date, so a day cannot be told from the future.
    TimeUnknown,
    /// The day is later than today.
    FutureDay,
    /// The board has not looked at the total of the pellets yet, so the grams since cannot be told.
    TotalNotSeen,
    /// The total at the cleaning is more than the total now: the pellets were zeroed since, or the number is wrong.
    TotalAboveNow,
    /// The board knows a cleaning already: a past one is given once, to a board that was born after it, and the button marks the rest.
    AlreadyKnown,
}

impl Refused {
    /// The code a client reads the refusal by.
    pub fn code(self) -> &'static str {
        match self {
            Refused::Usage => "usage",
            Refused::BadDay => "bad_day",
            Refused::BadKg => "bad_kg",
            Refused::TimeUnknown => "time_unknown",
            Refused::FutureDay => "future_day",
            Refused::TotalNotSeen => "total_not_seen",
            Refused::TotalAboveNow => "total_above_now",
            Refused::AlreadyKnown => "already_known",
        }
    }

    /// What a person is told.
    pub fn words(self) -> &'static str {
        match self {
            Refused::Usage => "cleaned takes a day and the kilograms of pellets burned by then: cleaned 2026-11-08 82.4",
            Refused::BadDay => "the first word is no day; write it as 2026-11-08",
            Refused::BadKg => "the second word is no number of kilograms; write it as 82.4, up to three digits after the point",
            Refused::TimeUnknown => "the board does not know today's date yet, so the day cannot be checked",
            Refused::FutureDay => "that day has not come yet",
            Refused::TotalNotSeen => "the board has not looked at the pellets yet; try again in a few seconds",
            Refused::TotalAboveNow => "the pellets counted are fewer than that: they were zeroed since, or the number is wrong",
            Refused::AlreadyKnown => "the board knows a cleaning already; a cleaning that was before it is given once, and the button Cleaned marks the next",
        }
    }
}

/// The day and the total in grams from the words after `cleaned`.
pub fn parse(words: &[&str]) -> Result<(Date, u64), Refused> {
    let [day, kg] = words else { return Err(Refused::Usage) };
    let day = Date::parse(day).map_err(|_| Refused::BadDay)?;
    Ok((day, grams_of(kg)?))
}

/// Kilograms as written, `82.4`, in grams: digits, and after a point one to three more. No float: a total is counted in whole grams.
fn grams_of(kg: &str) -> Result<u64, Refused> {
    let (whole, fraction) = match kg.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (kg, None),
    };
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    // a total of seven digits of kilograms is no pellets: a typing error, and no overflow below
    if !digits(whole) || whole.len() > 7 || fraction.is_some_and(|f| !digits(f) || f.len() > 3) {
        return Err(Refused::BadKg);
    }
    let kilograms: u64 = whole.parse().map_err(|_| Refused::BadKg)?;
    let grams: u64 = fraction.map_or(Ok(0), |f| format!("{f:0<3}").parse().map_err(|_| Refused::BadKg))?;
    Ok(kilograms * 1000 + grams)
}
