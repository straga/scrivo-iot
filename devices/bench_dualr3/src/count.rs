//! Electricity of the box counted from the meter's power (crates/energy): today, this month and in all per output,
//! kept in vfs so a power cut loses at most a minute, and a row of CSV per stretch of the setting rows in
//! /data/energy/<year>-<month>.csv - kept for the months of the setting keep, the oldest going sooner when vfs runs short.
//!
//! The meter's thread hands in every asking (meter.rs); the count is placed in its day with the clock's local time
//! (firmware::clock), and without it waits in the count until the time comes.

use std::sync::Mutex;

use bench_dualr3_api::energy as api;
use energy::{file_of, reset_row, Backlog, Energy, Keeper, Now, Snapshot, Watch, CSV_HEADER};
use firmware::{clock, nvs_settings};
use module::card::{Fill, Published, Reads};
use page::CSV_FOLDER;


/// Where the count lies between power cuts: beside the CSVs, in the board's /data.
const STATE: &str = "/data/energy/state";
/// How often the count goes to vfs: what a power cut can lose.
const SNAPSHOT_EVERY_MS: u64 = 60_000;
/// Below this many free blocks of vfs the oldest month's CSV goes before its time.
const MIN_FREE_BLOCKS: u32 = 16;

struct Count {
    energy: Energy,
    /// Whether the card last said today and the month as numbers: a count that becomes sure of its day, or stops being, is news the page is owed.
    sure_said: bool,
    /// Whether the time of day is to be believed (crates/tally, `Watch`): a leap of the clock must not become a day of the count.
    watch: Watch,
    saved_at_ms: u64,
}

static COUNT: Mutex<Option<Count>> = Mutex::new(None);
/// The card of the energy as its count last published it: each sum and each output's share, or why there is no number.
static PUBLISHED: Published<{ page::dictionary::addresses(&bench_dualr3_api::TABLE, api::CARD) }, 0> = Published::new();
/// Snapshots of COUNT are taken under its lock and written after it, in the order taken: the meter's snapshot taken
/// before a reset never lands over the reset's.
static KEEPER: Keeper = Keeper::new();

/// Writes SNAPSHOT to STATE unless a newer one is there; says in the log when it could not.
fn save(snapshot: Snapshot) -> Result<(), String> {
    match KEEPER.write(snapshot, |text| firmware::flash_owner::write(STATE, text.as_bytes())) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("{STATE} not written: {e}")),
    }
}

fn csv_path(name: &str) -> String {
    format!("{CSV_FOLDER}/{name}")
}

/// Takes the count back from vfs, or starts it from zero and says why.
pub fn start() {
    let mut text = Vec::new();
    let energy = match vfs::read(STATE, |chunk: &[u8]| -> Result<(), ()> {
        text.extend_from_slice(chunk);
        Ok(())
    }) {
        Ok(Ok(_)) => match Energy::restore(&String::from_utf8_lossy(&text)) {
            Ok(e) => {
                log::info!("energy taken back from {STATE}: {}", e.words().map(|(n, w)| format!("{n} {w}")).join(", "));
                e
            }
            Err(why) => {
                log::error!("energy: {STATE} is not a count ({why}) - counting from zero");
                Energy::default()
            }
        },
        Err(vfs::Error(-2)) => {
            log::info!("energy: no {STATE} yet - counting from zero");
            Energy::default()
        }
        other => {
            log::error!("energy: {STATE} not read ({other:?}) - counting from zero");
            Energy::default()
        }
    };
    match vfs::usage() {
        Ok((used, all)) => log::info!("vfs: {used} of {all} blocks in use"),
        Err(e) => log::warn!("vfs: usage not read: {e}"),
    }
    *COUNT.lock().unwrap() = Some(Count { energy, sure_said: false, watch: Watch::new(), saved_at_ms: between::clock::now().as_millis() });
    publish();
}

/// What is counted now as `bench_dualr3_api` takes it; None before the count started.
fn sums() -> Option<api::Sums> {
    let count = COUNT.lock().unwrap();
    let [today, month, total] = count.as_ref()?.energy.sums_by_output_kwh().map(|(_, apart)| apart);
    Some(api::Sums { today, month, total })
}

/// Whether the count is sure of its day, so that today and the month are numbers the card may say; not before the count started.
fn day_sure() -> bool {
    COUNT.lock().unwrap().as_ref().is_some_and(|c| c.energy.day_sure())
}

/// Puts the card of the energy together and publishes it.
fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| api::put_values(sums().as_ref(), day_sure(), &mut |at, value| card.put(at, value)));
}

fn now() -> Option<Now> {
    clock::now().map(|t| Now {
        date: energy::Date { year: t.year, month: t.month, day: t.day },
        minute: t.minute,
        offset_min: t.offset_min,
    })
}

/// The time of day when it is to be believed, else None (crates/tally, `Watch`). The steady clock is read first - that reading is what tells a leap of the chip's counter - then the answer of SNTP, then the time of day.
fn believed(watch: &mut Watch) -> Option<Now> {
    let steady = between::clock::now().as_millis();
    let answered = clock::answered_since_leap();
    let answers = clock::answers();
    watch.believe(now(), steady, answered, answers)
}

fn rows_min() -> u16 {
    nvs_settings::value(&bench_dualr3_api::energy::HISTORY_EVERY).parse().unwrap_or(60)
}

fn keep_months() -> u32 {
    nvs_settings::value(&bench_dualr3_api::energy::HISTORY_KEEP).parse().unwrap_or(12)
}

/// What the months kept take of this board's vfs now: the kilobytes a month, all the months and the board, and whether they fit.
fn room() -> Option<bench_dualr3_api::energy::HistoryRoom> {
    let (_, blocks) = vfs::usage().ok()?;
    Some(bench_dualr3_api::energy::history_room(u32::from(rows_min()), keep_months(), blocks * 4096))
}

/// One asking of the meter at AT_MS: the power of L1 and L2 when it answered, None when not.
pub fn asked(at_ms: u64, watts: Option<[f32; 2]>) {
    let (row, snapshot, sure_changed) = {
        let mut count = COUNT.lock().unwrap();
        let Some(c) = count.as_mut() else { return };
        c.energy.poll(at_ms, watts);
        let row = c.energy.at(believed(&mut c.watch), rows_min());
        let sure_changed = c.energy.day_sure() != c.sure_said;
        c.sure_said = c.energy.day_sure();
        let snapshot = (at_ms.saturating_sub(c.saved_at_ms) >= SNAPSHOT_EVERY_MS || row.is_some()).then(|| {
            c.saved_at_ms = at_ms;
            KEEPER.take(c.energy.snapshot())
        });
        (row, snapshot, sure_changed)
    };
    if let Some(row) = row {
        for (n, ms) in row.negative_ms.iter().enumerate() {
            if *ms > 0 {
                log::warn!("energy: L{} read negative power for {} s in the stretch from {}", n + 1, ms / 1000, row.csv());
            }
        }
        write_row(&file_of(row.slot.date), &row.csv());
    }
    if let Some(snapshot) = snapshot {
        if let Err(why) = save(snapshot) {
            log::error!("energy: {why}");
        }
    }
    publish();
    if watts.is_some() || sure_changed {
        web::announce(api::CARD);
    }
}

/// The rows of the CSV the flash has not taken yet.
static BACKLOG: Mutex<Backlog> = Mutex::new(Backlog::new());

/// Adds LINE to the CSV NAME behind the rows the flash refused before it, with the header first in a new file; then makes room. A row the flash refuses waits and goes first on the next one: a row is written when the flash took it.
fn write_row(name: &str, line: &str) {
    let mut backlog = BACKLOG.lock().unwrap();
    match backlog.write(name, line, append_row) {
        Ok(()) => prune(name),
        Err((waiting, dropped, why)) => {
            log::error!("energy: {} not written: {why} - {waiting} rows wait for the flash", csv_path(name));
            if dropped > 0 {
                log::error!("energy: {dropped} rows of the CSV let go: the flash has refused for too long");
            }
        }
    }
}

/// One row to its file, with the header first when the file is new.
fn append_row(name: &str, line: &str) -> Result<(), String> {
    let new = !vfs::files(CSV_FOLDER).is_ok_and(|names| names.iter().any(|n| n == name));
    let text = if new { format!("{CSV_HEADER}\n{line}\n") } else { format!("{line}\n") };
    firmware::flash_owner::append(&csv_path(name), text.as_bytes())
}

/// Removes the CSVs older than the months kept, and the oldest one more when vfs has fewer than MIN_FREE_BLOCKS
/// free; never KEEP, the one just written.
fn prune(keep: &str) {
    let Ok(names) = vfs::files(CSV_FOLDER) else { return };
    let mut months: Vec<String> = names.into_iter().filter(|n| page::csv_file(n).is_ok() && n != keep).collect();
    months.sort();
    let kept = keep_months() as usize;
    // the one written is one of the months kept
    let over = (months.len() + 1).saturating_sub(kept);
    for name in months.drain(..over.min(months.len())) {
        match firmware::flash_owner::remove(&csv_path(&name)) {
            Ok(()) => log::info!("energy: {name} removed - {kept} months are kept"),
            Err(e) => log::error!("energy: {name} not removed: {e}"),
        }
    }
    if let (Ok((used, all)), Some(oldest)) = (vfs::usage(), months.first()) {
        if all.saturating_sub(used) < MIN_FREE_BLOCKS {
            match firmware::flash_owner::remove(&csv_path(oldest)) {
                Ok(()) => log::warn!("energy: {oldest} removed before its time - vfs had {} blocks free", all - used),
                Err(e) => log::error!("energy: {oldest} not removed: {e}"),
            }
        }
    }
}

/// The rows of the card "meter" and of `meter` for the count: today, month, total in kWh; a dash for today and the month while the count is not sure of its day.
pub fn rows() -> Vec<(&'static str, String)> {
    let count = COUNT.lock().unwrap();
    match count.as_ref() {
        Some(c) => c.energy.words_shown().to_vec(),
        None => vec![("total", "- (the count did not start)".into())],
    }
}

/// Zeroes the count: today, month and total of L1, L2 and the sum. Kept at once, and marked in the log and in the
/// CSV; the rows before it stay.
fn reset() -> Result<String, String> {
    let snapshot = {
        let mut count = COUNT.lock().unwrap();
        let c = count.as_mut().ok_or("the count did not start")?;
        c.energy.reset();
        KEEPER.take(c.energy.snapshot())
    };
    save(snapshot).map_err(|why| format!("zeroed, but {why}"))?;
    log::warn!("energy reset: today, month and total of L1, L2 and the sum zeroed");
    let at = COUNT.lock().unwrap().as_mut().and_then(|c| believed(&mut c.watch));
    match at {
        Some(t) => write_row(&file_of(t.date), &reset_row(t)),
        None => log::warn!("energy reset without exact time: no row in the CSV marks it"),
    }
    publish();
    web::announce(api::CARD);
    Ok("energy zeroed: today, month and total of L1, L2 and the sum; the CSV keeps its rows".into())
}

/// What `reset energy` answers before it does anything.
const RESET_ASKS: &str = "reset energy zeroes today, month and total of L1, L2 and the sum; the CSV keeps its rows. To do it: reset energy yes";

/// The console word `reset energy`, then `reset energy yes`. Any other word after `reset` is not the energy's.
fn reset_word(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    match words {
        [] => Some(Err("reset takes what to zero: reset energy".into())),
        ["energy", rest @ ..] => Some(module::say::confirmed(rest).and_then(|yes| match yes {
            true => reset().map(|done| line(&done)),
            false => Ok(line(RESET_ASKS)),
        })),
        _ => None,
    }
}

static VERBS: [module::Verb; 1] =
    [module::Verb { name: "reset", help: "Zero the energy counted: reset energy, then reset energy yes", run: reset_word }];

/// The energy by the modules' contract (crates/module): its card, the two settings of its history and what the months kept take of the board, and the reset - a press nobody takes back.
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn verbs(&self) -> &'static [module::Verb] {
        &VERBS
    }

    fn takes(&self) -> &'static [&'static str] {
        &["press"]
    }

    fn settings(&self) -> Vec<&'static settings::Setting> {
        api::SETTINGS.to_vec()
    }

    fn setting_values(&self, each: &mut dyn FnMut(usize, &page::dictionary::Value)) {
        if let Some(room) = room() {
            api::put_setting_values(&room, &mut |at, value| each(at, &value));
        }
    }

    fn order(&self, name: &str, order: module::Order) -> Option<Result<(), String>> {
        (name == "reset").then(|| match order {
            module::Order::Press => reset().map(|_| ()),
            other => Err(format!("reset is pressed, not {other:?}")),
        })
    }
}
