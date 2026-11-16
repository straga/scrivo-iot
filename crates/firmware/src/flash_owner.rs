//! The one task that writes the flash: the settings a person saves, the modules' memory, the files of
//! vfs and the journal on flash. The others ask at its desk and are answered; the settings as they stand are published
//! whole ([`SETTINGS`]) and read from there, so nobody reads nvs for a setting and nobody but this task writes nvs or
//! vfs. Writing the flash stops both cores for its length, and one writer is also the one place it happens from.

use std::sync::Arc;
use std::time::Duration;

use esp_idf_svc::sys::EspError;
use flash::{Answer, Ask, File, Owner};
use module::Role;
use settings::Values;

use crate::nvs_settings::Nvs;

/// Where the others ask the owner of the flash.
pub static DESK: flash::Desk = flash::Desk::new();

/// The settings as they stand, whole: published by the owner at start and after every save that moved them.
pub static SETTINGS: Role<Arc<Values>> = Role::new("settings");

/// How long an asker waits: the owner answers between its passes over the journal, and a batch of the journal is a
/// few writes of 2 KB; a second and a half of it is the owner at its slowest.
pub const WITHIN: Duration = Duration::from_secs(3);

/// How often the journal's lines that are due go to flash.
const JOURNAL_EVERY: Duration = Duration::from_secs(1);

/// What a device has the owner do at every pass of its loop, once a second: a count it keeps on flash (a module's own) places what it counted and writes what is due, without a task of its own - a task costs its stack
/// out of the internal heap, and this one is awake every second already and is the one that writes. **IT RUNS ON THE OWNER'S TASK: it writes files with `vfs` directly and never asks the desk** - the desk is answered by this
/// very task, and an ask from it waits for itself. Set once, before `firmware::run`.
static ON_PASS: std::sync::Mutex<Vec<fn()>> = std::sync::Mutex::new(Vec::new());

/// Has the owner call F at every pass. See [`ON_PASS`].
pub fn on_pass(f: fn()) {
    ON_PASS.lock().unwrap().push(f);
}

/// Starts the task `flash` with OWNER: once vfs is open, since the journal and the files are written there.
pub(crate) fn start(owner: Owner<Nvs>) -> Result<(), EspError> {
    crate::task::round("flash", plan::PLAN.flash_stack, crate::task::Core::Net, move || {
        let mut owner = owner;
        // the journal on flash, this task's alone: the lines come by their channel, /journal asks at the desk
        let mut kept = journal::kept::Kept::new();
        let mut refused: Option<String> = None;
        // how loud each module is, this task's alone: the lines name new modules by their channel, `log` asks the desk
        let loud = crate::log_tee::loud();
        let mut loudness = loud.map(loudness::Keeper::new);
        loop {
            let pass = between::clock::now();
            let _ = crate::log_tee::journal_round(&mut kept, &mut refused, false);
            // the load of the cores is counted in this task, once a pass: a task of its own would cost a stack out of the heap
            crate::cores::pass();
            if let (Some(loud), Some(keeper)) = (loud, loudness.as_mut()) {
                keeper.learn(loud);
            }
            let passes = ON_PASS.lock().unwrap().clone();
            for pass in passes {
                pass();
            }
            // A restart that was asked for goes only with the journal down on the flash and no relay of the auger or the plug closed; otherwise it waits for the next pass, and the journal says why once.
            let asked = crate::board::restart_asked();
            if crate::board::take_flush() || asked {
                let down = crate::log_tee::journal_round(&mut kept, &mut refused, true).is_ok();
                if asked {
                    match crate::board::settle_restart(down) {
                        daily::Settled::Go => {
                            crate::board::going();
                            std::thread::sleep(Duration::from_millis(200));
                            unsafe { esp_idf_svc::sys::esp_restart() };
                        }
                        daily::Settled::Waits { why: daily::Postponed::Journal, new: true } => {
                            log::warn!("restart: postponed - the journal did not go down to the flash ({}); asked again at the next pass", refused.as_deref().unwrap_or("no word"));
                        }
                        daily::Settled::Waits { why: daily::Postponed::Organ, new: true } => {
                            log::warn!("restart: postponed - a relay of the auger or the plug is closed; asked again at the next pass");
                        }
                        daily::Settled::Waits { new: false, .. } | daily::Settled::Idle => {}
                    }
                }
            }
            DESK.serve_for(JOURNAL_EVERY.saturating_sub(pass.elapsed()), |ask| match ask {
                Ask::Waiting => Answer::Waiting(crate::log_tee::keeps().then(|| kept.waiting().to_string())),
                Ask::Tail => Answer::Tail(crate::log_tee::keeps().then(|| {
                    let tail = kept.tail();
                    (tail.left_out, tail.text)
                })),
                Ask::Loud(asked) => Answer::Loud(match (loud, loudness.as_mut()) {
                    (Some(loud), Some(keeper)) => crate::log_tee::loudness(loud, keeper, asked),
                    _ => Err("this board has no log to be loud in".into()),
                }),
                ask => answer(&mut owner, ask),
            });
        }
    })
}

/// Carries ASK out on OWNER, and tells what depends on a setting that moved.
fn answer(owner: &mut Owner<Nvs>, ask: Ask) -> Answer {
    let answer = owner.answer(ask);
    // what the page was told is a code; the words of the store - why the flash refused - are for whoever reads the log
    if let Answer::Changed(Err(why)) = &answer {
        log::warn!("settings not saved - {why}");
    }
    if let Answer::Changed(Ok(changed)) = &answer {
        for line in &changed.said {
            log::info!("setting changed - {line}");
        }
        for name in &changed.names {
            crate::nvs_settings::changed(name);
        }
        // AN OPEN PAGE IS TOLD HERE, where every change passes - the page's save and the console's `set` alike; told
        // by the page's route alone, a `set` left an open page showing 51 while the board held 42 (4.4, 26.09)
        if !changed.names.is_empty() {
            web::announce_settings();
        }
    }
    answer
}

/// How the owner writes vfs.
pub(crate) fn files(file: &File) -> Result<(), String> {
    let done = match file {
        File::Write { path, bytes } => vfs::write(path, bytes),
        File::Append { path, bytes } => vfs::append(path, bytes),
        File::Remove(path) => vfs::remove(path),
    };
    done.map_err(|e| e.to_string())
}

fn file(file: File) -> Result<(), String> {
    match DESK.ask(Ask::File(file), WITHIN) {
        Ok(Answer::Filed(done)) => done,
        Ok(_) => Err("the flash answered a file as something else".into()),
        Err(refused) => Err(refusal(refused)),
    }
}

/// Writes BYTES as the file PATH of vfs, whole, by the owner of the flash.
pub fn write(path: &str, bytes: &[u8]) -> Result<(), String> {
    file(File::Write { path: path.into(), bytes: bytes.to_vec() })
}

/// Adds BYTES at the end of the file PATH of vfs, by the owner of the flash.
pub fn append(path: &str, bytes: &[u8]) -> Result<(), String> {
    file(File::Append { path: path.into(), bytes: bytes.to_vec() })
}

/// Takes the file PATH off vfs, by the owner of the flash.
pub fn remove(path: &str) -> Result<(), String> {
    file(File::Remove(path.into()))
}

/// The lines of the journal on flash not written yet, from its owner; None - the device keeps no journal.
pub fn waiting() -> Result<Option<String>, String> {
    match DESK.ask(Ask::Waiting, WITHIN) {
        Ok(Answer::Waiting(waiting)) => Ok(waiting),
        Ok(_) => Err("the flash answered the journal as something else".into()),
        Err(refused) => Err(refusal(refused)),
    }
}

/// The end of the journal as its owner holds it in memory; None - the device keeps no journal.
pub fn tail() -> Result<Option<journal::kept::Tail>, String> {
    match DESK.ask(Ask::Tail, WITHIN) {
        Ok(Answer::Tail(tail)) => Ok(tail.map(|(left_out, text)| journal::kept::Tail { left_out, text })),
        Ok(_) => Err("the flash answered the journal's end as something else".into()),
        Err(refused) => Err(refusal(refused)),
    }
}

/// How loud the modules are, from the owner of the flash: None - the listing; Some - MODULE set to LEVEL.
pub fn loud(asked: Option<(&str, &str)>) -> Result<Vec<String>, String> {
    let asked = asked.map(|(module, level)| (module.to_string(), level.to_string()));
    match DESK.ask(Ask::Loud(asked), WITHIN) {
        Ok(Answer::Loud(said)) => said,
        Ok(_) => Err("the flash answered the loudness as something else".into()),
        Err(refused) => Err(refusal(refused)),
    }
}

/// A desk's refusal of a save, as the code the page says in its words.
pub(crate) fn refusal_to_save(refused: between::Refused) -> settings::Refusal {
    match refused {
        between::Refused::Busy => settings::Refusal::Busy,
        between::Refused::Silent => settings::Refusal::Silent { within_s: WITHIN.as_secs() },
    }
}

/// A desk's refusal in words, for what only the log and the console read.
pub(crate) fn refusal(refused: between::Refused) -> String {
    match refused {
        between::Refused::Busy => "the flash has too much asked of it now - try again".into(),
        between::Refused::Silent => format!("the flash did not answer within {} s", WITHIN.as_secs()),
    }
}
