//! HERE: the pellets: the seconds the auger turns, counted by the rate the owner weighed (crates/pellets) into kilograms of today, yesterday, this month and in all; the count kept in vfs, beside the settings' flash, so a restart and a new kit
//! leave it; the card it publishes; its place in the module list; the shell's `reset pellets`.
//! NEVER HERE: a pin, when the auger turns (the relays' owner tells this module at every change and every second of a limit), what a day or a month is (crates/tally), a word of the page - that the count is approximate, in
//! page/menu.json - or a code of the stream (api/src/pellets.rs). What this file owns is the ORDER: take the asking, place it in its day, write what is due, publish. **NO TASK OF ITS OWN**: the askings come from the relays' owner (it is the one that closes the auger and calls `observe`
//! at every change and every second of a limit), and placing, writing and publishing is the pass the owner of the flash makes every second (`firmware::flash_owner::on_pass`) - the one task that writes, awake already; a task of this module's own
//! cost its 8 KB of stack out of the internal heap.
//! CHECKS: crates/pellets/tests/pellets.rs, crates/pellets/tests/saving.rs, devices/burner/api/tests/pellets_values.rs.

use std::sync::Mutex;

use burner_api::pellets as api;
use firmware::{clock, nvs_settings};
use module::card::{Fill, Published, Reads};
use pellets::{Keeper, Now, Pellets, Pending, Saving, Watch};
use settings::Setting;

/// Where the count lies between restarts: in the board's /data, which a kit's upload never touches.
const STATE: &str = "/data/pellets/state";

/// Where yesterday lies: a file of its own, written when a day turns. Not a line of STATE: an image that reads only the old six lines refuses a count with a line more and counts from nought.
const YESTERDAY: &str = "/data/pellets/yesterday";

struct Kept {
    pellets: Pellets,
    /// Whether the time of day is to be believed (crates/tally, `Watch`): a leap of the clock must not become a day of the count.
    watch: Watch,
    /// What the flash holds of the count, as the flash answered (crates/pellets, `Saving`).
    saving: Saving,
    /// `pellets.changes` as the card last said it.
    published_changes: u64,
    /// Whether the card last said today and the month as numbers: a count that becomes sure of its day, or stops being, is news the card owes.
    published_sure: bool,
}

static COUNT: Mutex<Option<Kept>> = Mutex::new(None);

/// Snapshots of COUNT are taken under its lock and written after it, in the order taken: the round's snapshot taken before a reset never lands over the reset's.
static KEEPER: Keeper = Keeper::new();

/// The card of the pellets as the round last published it.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

fn now_ms() -> u64 {
    between::clock::now().as_millis()
}

fn now() -> Option<Now> {
    clock::now().map(|t| Now { date: pellets::Date { year: t.year, month: t.month, day: t.day }, minute: t.minute, offset_min: t.offset_min })
}

/// The time of day when it is to be believed, else None (crates/tally, `Watch`). The steady clock is read first - that reading is what tells a leap of the chip's counter - then the answer of SNTP, then the time of day.
pub(crate) fn believed(watch: &mut Watch) -> Option<Now> {
    let steady = now_ms();
    let answered = clock::answered_since_leap();
    let answers = clock::answers();
    watch.believe(now(), steady, answered, answers)
}

/// The rate in force, in grams a minute: taken afresh at every asking, so a number typed in works from the next second.
fn rate() -> u32 {
    nvs_settings::value(&burner_settings::PELLET_RATE).parse().unwrap_or_else(|_| burner_settings::PELLET_RATE.default.parse().unwrap_or(0))
}

/// Tells the count what the flash answered to PENDING: it is written only when the flash took it.
fn finish(pending: &Pending, done: &Result<(), String>) {
    if let Some(kept) = COUNT.lock().unwrap().as_mut() {
        kept.saving.finish(pending, done.is_ok());
    }
}

/// Writes PENDING to STATE through the desk of the flash, for a caller that is not its owner (the shell's reset), unless a newer snapshot is there.
fn save_by_desk(pending: &Pending) -> Result<(), String> {
    let done = KEEPER.write(pending.snapshot.clone(), |text| firmware::flash_owner::write(STATE, text.as_bytes())).map(|_| ()).map_err(|e| format!("{STATE} not written: {e}"));
    finish(pending, &done);
    done
}

/// Takes the count back from vfs, or starts it from zero and says why. Call once, after vfs is open.
pub fn start() {
    let mut text = Vec::new();
    let pellets = match vfs::read(STATE, |chunk: &[u8]| -> Result<(), ()> {
        text.extend_from_slice(chunk);
        Ok(())
    }) {
        Ok(Ok(_)) => match Pellets::restore(&String::from_utf8_lossy(&text)) {
            Ok(p) => {
                log::info!("pellets taken back from {STATE}: {} g in all", p.grams()[2].1);
                p
            }
            Err(why) => {
                log::error!("pellets: {STATE} is not a count ({why}) - counting from zero");
                Pellets::default()
            }
        },
        Err(vfs::Error(-2)) => {
            log::info!("pellets: no {STATE} yet - counting from zero");
            Pellets::default()
        }
        other => {
            log::error!("pellets: {STATE} not read ({other:?}) - counting from zero");
            Pellets::default()
        }
    };
    let mut pellets = pellets;
    restore_yesterday(&mut pellets);
    let saving = Saving::new(&pellets, now_ms());
    *COUNT.lock().unwrap() = Some(Kept { pellets, watch: Watch::new(), saving, published_changes: u64::MAX, published_sure: false });
    publish();
    firmware::flash_owner::on_pass(pass);
}

/// Takes yesterday back from its file into PELLETS. A file that is not there, or is not a day, leaves yesterday unknown and the count as it is.
fn restore_yesterday(pellets: &mut Pellets) {
    let mut text = Vec::new();
    match vfs::read(YESTERDAY, |chunk: &[u8]| -> Result<(), ()> {
        text.extend_from_slice(chunk);
        Ok(())
    }) {
        Ok(Ok(_)) => match pellets.restore_yesterday(&String::from_utf8_lossy(&text)) {
            Ok(()) => log::info!("pellets: yesterday taken back from {YESTERDAY}"),
            Err(why) => log::error!("pellets: {YESTERDAY} is not a day ({why}) - yesterday unknown"),
        },
        Err(vfs::Error(-2)) => log::info!("pellets: no {YESTERDAY} yet - yesterday unknown"),
        other => log::error!("pellets: {YESTERDAY} not read ({other:?}) - yesterday unknown"),
    }
}

/// Writes yesterday to its file, or takes the file off, when that is due (crates/pellets, `Saving::yesterday_due`): once when a day has turned, once when a reset forgot it. Runs on the owner's task, which writes with `vfs` itself.
fn write_yesterday() {
    let due = COUNT.lock().unwrap().as_mut().and_then(|kept| kept.saving.yesterday_due(&kept.pellets, now_ms()));
    let Some(pending) = due else { return };
    let done = match &pending.text {
        Some(text) => vfs::write(YESTERDAY, text.as_bytes()),
        // a file that is not there is as good as taken off
        None => match vfs::remove(YESTERDAY) {
            Err(vfs::Error(-2)) => Ok(()),
            other => other,
        },
    }
    .map_err(|e| format!("{YESTERDAY} not written: {e}"));
    if let Err(why) = &done {
        // owed still: tried again after `pellets::RETRY_MS`
        log::error!("pellets: {why}");
    }
    if let Some(kept) = COUNT.lock().unwrap().as_mut() {
        kept.saving.yesterday_finish(&pending, done.is_ok());
    }
}

/// One asking of the auger's relay: whether it is closed now. The relays' owner calls it after every change and every second a limit runs - and a limit always runs on the auger (`AUGER_LIMIT` has a floor) -, so the
/// seconds of turning are counted as they pass, and the edges exactly. It only takes the lock: the flash is the round's.
pub fn observe(auger_closed: bool) {
    let rate = rate();
    if let Some(kept) = COUNT.lock().unwrap().as_mut() {
        kept.pellets.observe(now_ms(), auger_closed, rate);
    }
}

/// The pass of the owner of the flash, once a second: places the count in its day, writes it when it is due, tells an open page. Runs on the owner's task, so it writes with `vfs` itself; a keeper that is busy - the reset is
/// writing through the desk, which only this task answers - is not waited for: the count stays owed and is written at a later pass.
fn pass() {
    if let Some(pending) = place_and_take() {
        let done = match KEEPER.try_write(pending.snapshot.clone(), |text| vfs::write(STATE, text.as_bytes())) {
            Some(Ok(_)) => Ok(()),
            Some(Err(e)) => Err(format!("{STATE} not written: {e}")),
            None => Err("the reset is writing it".to_string()),
        };
        if let Err(why) = &done {
            // owed still: tried again after `pellets::RETRY_MS`
            log::error!("pellets: {why}");
        }
        finish(&pending, &done);
    }
    write_yesterday();
    if moved() {
        publish();
        web::announce(api::CARD);
    }
}

/// Places what was counted in its day and takes the snapshot of the count when it is due for the flash (crates/pellets, `Saving::due`).
fn place_and_take() -> Option<Pending> {
    let mut count = COUNT.lock().unwrap();
    let kept = count.as_mut()?;
    kept.pellets.at(believed(&mut kept.watch));
    kept.saving.due(&KEEPER, &kept.pellets, now_ms())
}

/// The total of the pellets burned, in whole grams; None while the count did not start. What the soot counts the grams since the cleaning from.
pub(crate) fn total_g() -> Option<u64> {
    COUNT.lock().unwrap().as_ref().map(|k| k.pellets.grams()[2].1)
}

/// Whether the flash lacks something of the count: a restart now would lose it.
pub fn owed() -> bool {
    COUNT.lock().unwrap().as_ref().is_some_and(|k| k.saving.owed(&k.pellets))
}

/// Writes the count now, whatever its interval: for the planned restart, which waits until `owed` is false. Only from the pass of the owner of the flash, which writes with `vfs` itself.
pub fn write_now() {
    let pending = {
        let mut count = COUNT.lock().unwrap();
        let Some(kept) = count.as_mut() else { return };
        kept.pellets.at(believed(&mut kept.watch));
        kept.saving.now(&KEEPER, &kept.pellets, now_ms())
    };
    let done = match KEEPER.try_write(pending.snapshot.clone(), |text| vfs::write(STATE, text.as_bytes())) {
        Some(Ok(_)) => Ok(()),
        Some(Err(e)) => Err(format!("{STATE} not written: {e}")),
        None => Err("the reset is writing it".to_string()),
    };
    if let Err(why) = &done {
        log::error!("pellets: {why}");
    }
    finish(&pending, &done);
    write_yesterday();
}

/// Whether the count moved since the card last said it, or became sure of its day or stopped being.
fn moved() -> bool {
    COUNT.lock().unwrap().as_ref().is_some_and(|k| k.pellets.changes != k.published_changes || k.pellets.day_sure() != k.published_sure)
}

/// Whether the count is sure of its day, so that today and the month are numbers the card may say; not before the count started.
fn day_sure() -> bool {
    COUNT.lock().unwrap().as_ref().is_some_and(|k| k.pellets.day_sure())
}

/// The three counts in kilograms - today, this month, in all - as the card says them; nought for a count that did not start.
fn kilograms() -> [f32; 3] {
    match COUNT.lock().unwrap().as_ref() {
        Some(kept) => kept.pellets.kilograms().map(|(_, kg)| kg),
        None => [0.0; 3],
    }
}

/// Kilograms of yesterday as the card says them; None while the count does not have the day before today.
fn yesterday() -> Option<f32> {
    COUNT.lock().unwrap().as_ref().and_then(|kept| kept.pellets.yesterday_kg())
}

/// Puts the card together and publishes it.
fn publish() {
    if let Some(kept) = COUNT.lock().unwrap().as_mut() {
        kept.published_changes = kept.pellets.changes;
        kept.published_sure = kept.pellets.day_sure();
    }
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
}

fn values(card: &mut dyn Fill) {
    api::put_values(&kilograms(), yesterday(), day_sure(), &mut |at, value| card.put(at, value));
}

/// Zeroes today, yesterday, this month and in all, and writes it at once, so a restart does not bring the count back. A write the flash refuses is told to the one who asked and tried again by the round: the zero is owed until the
/// flash has taken it.
fn reset() -> Result<String, String> {
    let pending = {
        let mut count = COUNT.lock().unwrap();
        let kept = count.as_mut().ok_or("the count did not start")?;
        kept.pellets.reset();
        kept.saving.now(&KEEPER, &kept.pellets, now_ms())
    };
    let written = save_by_desk(&pending);
    publish();
    web::announce(api::CARD);
    written.map_err(|why| format!("zeroed, but {why}; it is tried again"))?;
    log::warn!("pellets reset: today, yesterday, month and total zeroed");
    Ok("pellets zeroed: today, yesterday, month and total".into())
}

/// What `reset pellets` answers before it does anything.
const RESET_ASKS: &str = "reset pellets zeroes today, yesterday, month and total of the pellets burned. To do it: reset pellets yes";

/// The console word `reset pellets`, then `reset pellets yes`. Any other word after `reset` is not the pellets'.
fn reset_word(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    match words {
        [] => Some(Err("reset takes what to zero: reset pellets".into())),
        ["pellets", rest @ ..] => Some(module::say::confirmed(rest).and_then(|yes| match yes {
            true => reset().map(|done| line(&done)),
            false => Ok(line(RESET_ASKS)),
        })),
        _ => None,
    }
}

static VERBS: [module::Verb; 1] =
    [module::Verb { name: "reset", help: "Zero the pellets counted: reset pellets, then reset pellets yes", run: reset_word }];

/// The pellets by the modules' contract (crates/module).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn verbs(&self) -> &'static [module::Verb] {
        &VERBS
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn settings(&self) -> Vec<&'static Setting> {
        api::SETTINGS.to_vec()
    }
}
