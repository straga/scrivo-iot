//! API of the energy counted from the meter's power: today, this month and in all, each the sum of both outputs in kilowatt-hours with L1 and L2 apart, the two settings of the history it keeps, and the
//! reset - the one command of this card, and one that cannot be taken back. What the module hands the stream for them.
//! NEVER HERE: how the energy is integrated or where it is kept (crates/energy, the device's count), a word for a person (page/menu.json).
//! CHECKS: tests/api.rs, tests/values.rs.

use page::dictionary::{joined, Entry, Kind, Value};
use settings::{Choice, Level, Setting};

/// The card the energy stands on.
pub const CARD: &str = "energy";

/// Minutes per row of the energy's history.
pub const HISTORY_EVERY: Setting = Setting {
    name: "rows",
    label: "History, a row every",
    default: "60",
    kind: settings::Kind::Choice(&[
        Choice { value: "15", words: "15 min", takes: None, checks: None, means: Some("the energy history takes a row every 15 min, 96 a day") },
        Choice { value: "30", words: "30 min", takes: None, checks: None, means: Some("the energy history takes a row every 30 min, 48 a day") },
        Choice { value: "60", words: "1 hour", takes: None, checks: None, means: Some("the energy history takes a row every hour, 24 a day") },
    ]),
    level: Level::Work,
    card: None,
};

/// Months of the energy's history kept.
pub const HISTORY_KEEP: Setting =
    Setting { name: "keep", label: "History, months kept", default: "12", kind: settings::Kind::Months { from: 1, to: 120 }, level: Level::Work, card: None };

/// Bytes a row of the CSV takes, a little over the longest seen: "2026-09-19 17:00+03:00,19.412,110.700,130.112\n".
const ROW_BYTES: u32 = 48;
const BLOCK: u32 = 4096;

/// How much room the history takes, in kilobytes: a month of it, the months kept, and the board's own vfs they are kept on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryRoom {
    pub month_kb: u32,
    pub all_kb: u32,
    pub fs_kb: u32,
}

/// How much room the history takes: a row every ROWS_MIN minutes, kept MONTHS months, on a vfs of FS_BYTES.
pub fn history_room(rows_min: u32, months: u32, fs_bytes: u32) -> HistoryRoom {
    let rows = 31 * 24 * 60 / rows_min.max(1);
    let month_kb = (rows * ROW_BYTES).div_ceil(BLOCK) * BLOCK / 1024;
    HistoryRoom { month_kb, all_kb: month_kb * months, fs_kb: fs_bytes / 1024 }
}

/// The settings of this module: the minutes per row of the history, and the months kept.
pub const SETTINGS: [&Setting; 2] = [&HISTORY_EVERY, &HISTORY_KEEP];

/// The sum a row says: kilowatt-hours with the digits a watt-hour is worth, or why there is none.
const fn sum(name: &'static str) -> Entry {
    Entry { unit: "kWh", decimals: Some(3), reasons: &["not_counting"], ..Entry::row(CARD, name, &[Kind::Number, Kind::Unknown]) }
}

/// An output's share of a sum.
const fn share(row: &'static str, name: &'static str) -> Entry {
    Entry { unit: "kWh", decimals: Some(3), reasons: &["not_counting"], ..Entry::part(CARD, row, name, &[Kind::Number, Kind::Unknown]) }
}

/// Why there is no number for today or the month: the count is not sure of its day.
pub const DAY_UNSURE: &str = "day_unsure";

/// The sum of a day or a month: no number either while the board is not sure of its day.
const fn sum_of_a_day(name: &'static str) -> Entry {
    Entry { reasons: &["not_counting", DAY_UNSURE], ..sum(name) }
}

/// An output's share of the sum of a day or a month.
const fn share_of_a_day(row: &'static str, name: &'static str) -> Entry {
    Entry { reasons: &["not_counting", DAY_UNSURE], ..share(row, name) }
}

const TODAY: [Entry; 3] = [sum_of_a_day("today"), share_of_a_day("today", "l1"), share_of_a_day("today", "l2")];
const MONTH: [Entry; 3] = [sum_of_a_day("month"), share_of_a_day("month", "l1"), share_of_a_day("month", "l2")];
const TOTAL: [Entry; 3] = [sum("total"), share("total", "l1"), share("total", "l2")];
const SUMS: [Entry; 6] = joined(&TODAY, &MONTH);
const COUNTED: [Entry; 9] = joined(&SUMS, &TOTAL);
/// The reset zeroes today, the month and the total: what was counted does not come back.
const RESET: Entry = Entry { can: &["press"], irreversible: true, ..Entry::row(CARD, "reset", &[Kind::Button]) };
const ROWS: [Entry; 10] = joined(&COUNTED, &[RESET]);

/// What the months kept take of the board, as parts of the setting's row: whether they fit, and the kilobytes a month takes, all the months kept take and the board has.
const fn room(name: &'static str, kinds: &'static [Kind]) -> Entry {
    Entry { unit: if matches!(kinds, [Kind::Word]) { "" } else { "KB" }, decimals: if matches!(kinds, [Kind::Word]) { None } else { Some(0) }, ..Entry::part(page::SETTINGS, HISTORY_KEEP.name, name, kinds) }
}

const HISTORY: [Entry; 4] = [
    Entry { words: &["fits", "over"], ..room("note", &[Kind::Word]) },
    room("month_kb", &[Kind::Number]),
    room("all_kb", &[Kind::Number]),
    room("fs_kb", &[Kind::Number]),
];
/// The settings as rows of the settings card: what each keeps, as it is kept; the months kept with what they take of the board.
const SETTING_ROWS: [Entry; 6] = joined(
    &[Entry::row(page::SETTINGS, HISTORY_EVERY.name, &[Kind::Text]), Entry::row(page::SETTINGS, HISTORY_KEEP.name, &[Kind::Text])],
    &HISTORY,
);

/// Everything this module adds to the table: its settings' rows, then its own.
pub const ENTRIES: [Entry; 16] = joined(&SETTING_ROWS, &ROWS);

/// The settings as a client needs them: name, default, what each takes. No card: where they stand is the page's menu.
pub fn settings() -> Vec<page::dictionary::Setting> {
    SETTINGS.iter().map(|s| dictionary::setting(s, &settings::NO_RULES)).collect()
}

/// The addresses of the energy's values in the device's table (`crate::ALL`).
pub mod at {
    use page::dictionary::{at, part_at};
    /// Today, the month, in all - the sum of both outputs.
    pub const SUM: [usize; 3] = [at(&crate::ALL, super::CARD, "today"), at(&crate::ALL, super::CARD, "month"), at(&crate::ALL, super::CARD, "total")];
    /// The same, per output: `[row][output]`.
    pub const SHARE: [[usize; 2]; 3] = [
        [part_at(&crate::ALL, super::CARD, "today", "l1"), part_at(&crate::ALL, super::CARD, "today", "l2")],
        [part_at(&crate::ALL, super::CARD, "month", "l1"), part_at(&crate::ALL, super::CARD, "month", "l2")],
        [part_at(&crate::ALL, super::CARD, "total", "l1"), part_at(&crate::ALL, super::CARD, "total", "l2")],
    ];
}

/// Where the room the months kept take is said in the table (`crate::ALL`).
pub mod room_at {
    use page::dictionary::part_at;
    pub const NOTE: usize = part_at(&crate::ALL, page::SETTINGS, "keep", "note");
    pub const MONTH_KB: usize = part_at(&crate::ALL, page::SETTINGS, "keep", "month_kb");
    pub const ALL_KB: usize = part_at(&crate::ALL, page::SETTINGS, "keep", "all_kb");
    pub const FS_KB: usize = part_at(&crate::ALL, page::SETTINGS, "keep", "fs_kb");
}

/// What the stream is told of what the months kept take of the board: whether they fit - exactly its size fits - and the kilobytes they are made of.
pub fn put_setting_values(room: &HistoryRoom, put: &mut dyn FnMut(usize, Value)) {
    put(room_at::NOTE, Value::Word(if room.all_kb > room.fs_kb { "over" } else { "fits" }));
    put(room_at::MONTH_KB, Value::Number(room.month_kb as f32));
    put(room_at::ALL_KB, Value::Number(room.all_kb as f32));
    put(room_at::FS_KB, Value::Number(room.fs_kb as f32));
}

/// What is counted: for today, this month and in all, each output's kilowatt-hours (L1, L2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sums {
    pub today: [f32; 2],
    pub month: [f32; 2],
    pub total: [f32; 2],
}

/// What the stream is told of the energy: each sum and each output's share, or - the count did not start - the reason there is no number.
/// DAY_SURE is whether the count is sure of its day (crates/tally): when it is not, today and the month are the reason there is no number, and the total is the number it is.
pub fn put_values(sums: Option<&Sums>, day_sure: bool, put: &mut dyn FnMut(usize, Value)) {
    match sums {
        Some(sums) => {
            for (n, apart) in [sums.today, sums.month, sums.total].into_iter().enumerate() {
                // row 0 is today, 1 the month, 2 the total
                let said = |number: f32| if day_sure || n == 2 { Value::Number(number) } else { Value::Unknown(DAY_UNSURE) };
                put(at::SUM[n], said(apart[0] + apart[1]));
                put(at::SHARE[n][0], said(apart[0]));
                put(at::SHARE[n][1], said(apart[1]));
            }
        }
        None => {
            for n in 0..3 {
                put(at::SUM[n], Value::Unknown("not_counting"));
                put(at::SHARE[n][0], Value::Unknown("not_counting"));
                put(at::SHARE[n][1], Value::Unknown("not_counting"));
            }
        }
    }
}
