//! HERE: the soot: how dirty the boiler is - the flue minus the water over the burns of the working stage (crates/soot) -, the pellets burned since the last cleaning, the day of it; the count kept in vfs beside the pellets' (`/data`, which a kit's upload never
//! touches), the card it publishes, its place in the module list, the button of the cleaning and the console word of a cleaning that was made before the count knew of it (`cleaned`).
//! NEVER HERE: when a burn begins or ends, the median, the grams (crates/soot), a percent or a word of the page - the boiler wants cleaning - (page/menu.json, page/scheme.js), a code of the stream (api/src/soot.rs). What this file owns is the ORDER:
//! take the second, place the cleaning in its day, write what is due, publish. **NO TASK OF ITS OWN**: the second comes from the automaton's round (`observe`, once a second after it has decided), and writing and publishing is the pass the
//! owner of the flash makes every second (`firmware::flash_owner::on_pass`) - the one task that writes, awake already; a task of this module's own would cost its 8 KB of stack out of the internal heap.
//! CHECKS: crates/soot/tests/soot.rs, crates/soot/tests/saving.rs, devices/burner/api/tests/soot_values.rs, devices/burner/api/tests/soot_table.rs.

use std::sync::Mutex;

use burner_api::soot as api;
use firmware::clock;
use module::card::{Fill, Published, Reads};
use settings::Setting;
use soot::{Date, Keeper, Saving, Soot};

/// Where the count lies between restarts: in the board's /data, which a kit's upload never touches.
const STATE: &str = "/data/soot/state";

struct Kept {
    soot: Soot,
    /// What the flash holds of the count, as the flash answered (crates/soot, `Saving`).
    saving: Saving,
    /// `soot.changes` as the card last said it.
    published_changes: u64,
}

static COUNT: Mutex<Option<Kept>> = Mutex::new(None);

/// Snapshots of COUNT are taken under its lock and written after it, in the order taken: the pass's snapshot taken before a cleaning never lands over the cleaning's.
static KEEPER: Keeper = Keeper::new();

/// The card of the soot as the round last published it.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

fn now_ms() -> u64 {
    between::clock::now().as_millis()
}

/// The date of today, when the board knows the time: the day a cleaning is marked with.
fn today() -> Option<Date> {
    clock::now().map(|t| Date { year: t.year, month: t.month, day: t.day })
}

/// Takes the count back from vfs, or starts it from nothing and says why. Call once, after vfs is open.
pub fn start() {
    let mut text = Vec::new();
    let soot = match vfs::read(STATE, |chunk: &[u8]| -> Result<(), ()> {
        text.extend_from_slice(chunk);
        Ok(())
    }) {
        Ok(Ok(_)) => match Soot::restore(&String::from_utf8_lossy(&text)) {
            Ok(s) => {
                log::info!("soot taken back from {STATE}: {} burns kept, {} g since the cleaning", s.burns_kept(), s.grams_since_clean());
                s
            }
            Err(why) => {
                log::error!("soot: {STATE} is not a count ({why}) - counting from nothing");
                Soot::default()
            }
        },
        Err(vfs::Error(-2)) => {
            log::info!("soot: no {STATE} yet - counting from nothing");
            Soot::default()
        }
        other => {
            log::error!("soot: {STATE} not read ({other:?}) - counting from nothing");
            Soot::default()
        }
    };
    let saving = Saving::new(&soot, now_ms());
    *COUNT.lock().unwrap() = Some(Kept { soot, saving, published_changes: u64::MAX });
    publish();
    firmware::flash_owner::on_pass(pass);
}

/// One second of the automaton's round, after it has decided: whether it stands in its working stage, and what the flue and the water read. It only takes the lock: the flash is the pass's.
pub fn observe(working: bool, on_big: bool, flue_c: Option<f32>, boiler_c: Option<f32>) {
    if let Some(kept) = COUNT.lock().unwrap().as_mut() {
        kept.soot.second(working, on_big, flue_c, boiler_c);
    }
}

/// The pass of the owner of the flash, once a second: looks at the total of the pellets, writes the count when it is due, tells an open page. Runs on the owner's task, so it writes with `vfs` itself.
fn pass() {
    let total = super::pellets::total_g();
    let due = {
        let mut count = COUNT.lock().unwrap();
        let Some(kept) = count.as_mut() else { return };
        if let Some(grams) = total {
            kept.soot.observe_total_g(grams);
        }
        kept.saving.due(&KEEPER, &kept.soot, now_ms())
    };
    if let Some(pending) = due {
        // a keeper that is busy - the cleaning is writing through the desk, which only this task answers - is not waited for: the count stays owed and is written at a later pass
        let done = match KEEPER.try_write(pending.snapshot.clone(), |text| vfs::write(STATE, text.as_bytes())) {
            Some(Ok(_)) => Ok(()),
            Some(Err(e)) => Err(format!("{STATE} not written: {e}")),
            None => Err("the cleaning is writing it".to_string()),
        };
        if let Err(why) = &done {
            // owed still: tried again after `soot::RETRY_MS`
            log::error!("soot: {why}");
        }
        finish(&pending, &done);
    }
    if moved() {
        publish();
        web::announce(api::CARD);
    }
}

/// Tells the count what the flash answered to PENDING: it is written only when the flash took it.
fn finish(pending: &soot::Pending, done: &Result<(), String>) {
    if let Some(kept) = COUNT.lock().unwrap().as_mut() {
        kept.saving.finish(pending, done.is_ok());
    }
}

/// Whether the count moved since the card last said it.
fn moved() -> bool {
    COUNT.lock().unwrap().as_ref().is_some_and(|k| k.soot.changes != k.published_changes)
}

/// Puts the card together and publishes it.
fn publish() {
    if let Some(kept) = COUNT.lock().unwrap().as_mut() {
        kept.published_changes = kept.soot.changes;
    }
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
}

fn values(card: &mut dyn Fill) {
    let count = COUNT.lock().unwrap();
    if let Some(kept) = count.as_ref() {
        let day = kept.soot.cleaned_on().map(|d| d.days());
        api::put_values(kept.soot.soot_c(), kept.soot.burns_counted(), kept.soot.grams_since_clean(), day, &mut |at, value| card.put(at, value));
    } else {
        api::put_values(None, 0, 0, None, &mut |at, value| card.put(at, value));
    }
}

/// Writes PENDING to the flash at once, through the desk of the flash, tells the count what the flash said and the open pages what the card says now.
fn write_at_once(pending: &soot::Pending) -> Result<(), String> {
    let done = KEEPER.write(pending.snapshot.clone(), |text| firmware::flash_owner::write(STATE, text.as_bytes())).map(|_| ()).map_err(|e| format!("{STATE} not written: {e}"));
    finish(pending, &done);
    publish();
    web::announce(api::CARD);
    done
}

/// The boiler was cleaned: the grams since begin from nought, the burns are forgotten, the day is marked - and the count is written to the flash AT ONCE, through the desk of the flash, and the person is answered by what the flash said: a restart
/// or a cut of power after "done" must not bring the grams and the burns back. A write the flash refused is answered with `NOT_WRITTEN`: the cleaning stands in memory and is owed, and the pass tries it again by itself.
fn clean() -> Result<(), String> {
    let pending = {
        let mut count = COUNT.lock().unwrap();
        let kept = count.as_mut().ok_or("the count did not start")?;
        kept.soot.cleaned(today());
        kept.saving.now(&KEEPER, &kept.soot, now_ms())
    };
    let done = write_at_once(&pending);
    match done {
        Ok(()) => {
            log::info!("soot: the boiler cleaned and written, the grams since the cleaning begin from nought");
            Ok(())
        }
        Err(why) => {
            log::error!("soot: the boiler cleaned, but {why}; it is tried again");
            Err(api::NOT_WRITTEN.to_string())
        }
    }
}

/// `cleaned <day> <kg>`: a cleaning made before the count knew of it, the day of it and the total of the pellets then, given once to a board that knows no cleaning. The count takes it as `clean` does - at once to the flash - but keeps the burns, which are the boiler's since that cleaning.
fn cleaned_before(words: &[&str]) -> Result<String, String> {
    let refuse = |why: soot::past::Refused| format!("refused, {}: {}", why.code(), why.words());
    let (day, total_g) = soot::past::parse(words).map_err(refuse)?;
    let pending = {
        let mut count = COUNT.lock().unwrap();
        let kept = count.as_mut().ok_or("the count did not start")?;
        kept.soot.cleaned_before(day, today(), total_g).map_err(refuse)?;
        kept.saving.now(&KEEPER, &kept.soot, now_ms())
    };
    let grams = COUNT.lock().unwrap().as_ref().map_or(0, |k| k.soot.grams_since_clean());
    match write_at_once(&pending) {
        Ok(()) => {
            log::info!("soot: a cleaning of {} marked, {grams} g since it", day.words());
            Ok(format!("cleaning of {} marked, {:.1} kg since it", day.words(), grams as f64 / 1000.0))
        }
        Err(why) => {
            log::error!("soot: a cleaning of {} marked, but {why}; it is tried again", day.words());
            Err(api::NOT_WRITTEN.to_string())
        }
    }
}

fn cleaned_word(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    Some(cleaned_before(words).map(|done| line(&done)))
}

static VERBS: [module::Verb; 1] = [module::Verb {
    name: "cleaned",
    help: "Mark a cleaning the board was not there for, once: cleaned <day> <kg of pellets burned by then>, as cleaned 2026-11-08 82.4",
    run: cleaned_word,
}];

/// The soot by the modules' contract (crates/module).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn takes(&self) -> &'static [&'static str] {
        &["press"]
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

    fn order(&self, name: &str, order: module::Order) -> Option<Result<(), String>> {
        (name == "clean").then(|| match order {
            module::Order::Press => clean(),
            other => Err(format!("clean is pressed, not {other:?}")),
        })
    }
}
