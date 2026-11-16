//! HERE: the board's planned restart once a day (crates/daily): the hour from the settings, the moment it is due asked every second of the owner of the flash, the count of pellets written before it, the minute of the last planned restart kept in vfs and said on a card and in `status`, and the note a silent
//! draught's round leaves in the journal on flash.
//! NEVER HERE: when a restart is due and what it waits for (crates/daily), whether the burner stands (crates/automaton, `rest`), how the count is written (crates/pellets), a word of the page (page/menu.json), a code of the stream (api/src/restart.rs). What this file owns is the ORDER: ask the board's facts, judge, write what is owed, restart or leave a note.
//! **NO TASK OF ITS OWN**: the second comes from the pass of the owner of the flash (`firmware::flash_owner::on_pass`), the one task that writes and is awake already. **THE RESTART IS ASKED FOR HERE AND CARRIED OUT BY THE OWNER**, at the last moment and on facts, not on decisions: with the journal down on the flash, and with no relay of the auger or the plug closed (`organ_closed`), whoever closed it; the minute of the restart is written as the board goes (`going`).
//! CHECKS: crates/daily/tests/daily.rs, crates/automaton/tests/rest.rs, devices/burner/api/tests/restart_table.rs.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

use burner_api::restart as api;
use daily::{Clues, Hour, Moment, Verdict, Why};
use firmware::{clock, nvs_settings};
use module::card::{Fill, Published, Reads};
use module::Quiet;
use schedule::Wall;
use settings::Setting;

/// Where the minute of the last planned restart lies between restarts: in the board's /data, which a kit's upload never touches.
const LAST: &str = "/data/restart/last";

struct Kept {
    /// One case at a time for each: the draught's round, the flame's.
    clues: [Clues; 2],
    /// The minutes since the epoch at the last planned restart, None before there was one.
    last_min: Option<u64>,
    /// What the card last said, so that it is published when the number moves and not every second.
    said: Option<Result<u32, &'static str>>,
}

static KEPT: Mutex<Kept> = Mutex::new(Kept { clues: [Clues::default_const(), Clues::default_const()], last_min: None, said: None });

/// What the last second waited for, so that the journal says a wait once and not every second: 0 nothing, 1 the fire, 2 a fault, 3 the count.
static WAITED: AtomicU8 = AtomicU8::new(0);

/// The card of the restart as the pass last published it.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

/// The minutes since the epoch now, None while the board does not know the time.
fn now_min() -> Option<u64> {
    clock::now()?;
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(secs / 60)
}

fn hour() -> Hour {
    Hour::of(&nvs_settings::value(&burner_settings::RESTART_HOUR))
}

/// How long ago the last planned restart was, or why there is no number.
fn ago(last_min: Option<u64>) -> Result<u32, &'static str> {
    let Some(last) = last_min else { return Err(api::NONE_YET) };
    let now = now_min().ok_or(api::NO_CLOCK)?;
    Ok(now.saturating_sub(last).min(u64::from(u32::MAX)) as u32)
}

/// The words of `status` for the card's number.
fn words(said: Result<u32, &'static str>) -> String {
    match said {
        Ok(min) => format!("{min} min ago"),
        Err(why) if why == api::NONE_YET => "none since the record began".to_string(),
        Err(_) => "unknown: the board does not know the time".to_string(),
    }
}

/// Takes the minute of the last planned restart back from vfs. Call once, after vfs is open.
pub fn start() {
    let mut text = Vec::new();
    let read = vfs::read(LAST, |chunk: &[u8]| -> Result<(), ()> {
        text.extend_from_slice(chunk);
        Ok(())
    });
    let last_min = match read {
        Ok(Ok(_)) => String::from_utf8_lossy(&text).trim().parse::<u64>().ok(),
        Err(vfs::Error(-2)) => None,
        other => {
            log::error!("restart: {LAST} not read ({other:?}) - no planned restart on record");
            None
        }
    };
    KEPT.lock().unwrap().last_min = last_min;
    publish();
    firmware::board::set_restart_hooks(firmware::board::RestartHooks { organ_closed, going });
    firmware::flash_owner::on_pass(pass);
}

/// The pass of the owner of the flash, once a second: leaves the note a silent draught earns, says the minutes since the last planned restart, and judges whether a restart is due. Runs on the owner's task, so it writes with `vfs` itself.
fn pass() {
    note_the_silences();
    let (last_min, said) = {
        let kept = KEPT.lock().unwrap();
        (kept.last_min, kept.said)
    };
    let now = ago(last_min);
    if said != Some(now) {
        KEPT.lock().unwrap().said = Some(now);
        firmware::board::set_planned(words(now));
        publish();
        web::announce(api::CARD);
    }
    let rest = crate::modules::automaton::rest();
    let moment = Moment {
        wall: clock::now().map(|t| Wall::of_date(t.year, t.month, t.day, t.minute)),
        up_s: (firmware::board::uptime_ms() / 1000) as u64,
        standing: rest == Some(::automaton::Rest::Standing),
        fault: rest == Some(::automaton::Rest::Fault),
        owed: crate::modules::pellets::owed(),
        auger_closed: crate::modules::relays::auger_closed(),
        igniter_closed: crate::modules::relays::igniter_closed(),
    };
    let verdict = hour().judge(moment);
    if verdict != Verdict::Restart {
        firmware::board::cancel_restart();
    }
    match verdict {
        Verdict::Not => WAITED.store(0, Ordering::Relaxed),
        Verdict::Waits(why) => {
            let (code, said) = match why {
                Why::Fire => (1, "the burner is not standing with no fire"),
                Why::Fault => (2, "the burner is in a fault, which waits for a person"),
                Why::CountOwed => (3, "the count of pellets is being written"),
                Why::Organ => (5, "a relay of the auger or the plug is closed"),
            };
            if WAITED.swap(code, Ordering::Relaxed) != code {
                log::info!("restart: due, and waits: {said}");
            }
            if why == Why::CountOwed {
                crate::modules::pellets::write_now();
            }
        }
        Verdict::Restart => {
            if WAITED.swap(4, Ordering::Relaxed) != 4 {
                log::warn!("restart: the board is to restart by plan, up {} s, the burner at rest, the count written - the owner of the flash restarts it once the journal is down", moment.up_s);
            }
            firmware::board::restart_after_this_pass();
        }
    }
}

/// Whether a relay of the auger or the plug is closed now, whoever closed it: the owner of the flash asks it at the last moment, after the journal is down.
fn organ_closed() -> bool {
    crate::modules::relays::auger_closed() || crate::modules::relays::igniter_closed()
}

/// Written as the board goes: the minute of the planned restart, for the card and `status` of the board that comes up.
fn going() {
    let at = now_min();
    if let Err(e) = vfs::write(LAST, at.unwrap_or(0).to_string().as_bytes()) {
        log::error!("restart: {LAST} not written ({e}) - restarting all the same, the record will not show it");
    }
}

/// A round of the draught or of the flame silent past ten seconds leaves once for the case what `status` would have said: how long the board has been up, the timers, what each round that reads the shared analog unit held and waited for, and the tasks with the least free stack. Written to the journal on flash at once.
fn note_the_silences() {
    let silent = |quiet: Quiet| match quiet {
        Quiet::Speaks => None,
        Quiet::Silent(span) | Quiet::TooLong(span) => Some(span.as_secs().min(u64::from(u32::MAX)) as u32),
    };
    let now = between::clock::now();
    let causes = [
        ("the draught's round", silent(crate::roles::DRAUGHT.quiet_at(now))),
        ("the flame's round", silent(crate::roles::FLAME.quiet_at(now))),
    ];
    let mut noted = false;
    for (n, (cause, silent_s)) in causes.into_iter().enumerate() {
        if !KEPT.lock().unwrap().clues[n].see(silent_s) {
            continue;
        }
        log::warn!("restart: {cause} has been silent for {} s - what the board holds:", silent_s.unwrap_or(0));
        noted = true;
    }
    if !noted {
        return;
    }
    for line in firmware::board::clue_lines() {
        log::warn!("restart: {line}");
    }
    firmware::board::journal_soon();
}

/// Puts the card together and publishes it.
fn publish() {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(card));
}

fn values(card: &mut dyn Fill) {
    let last_min = KEPT.lock().unwrap().last_min;
    api::put_value(ago(last_min), &mut |at, value| card.put(at, value));
}

/// The planned restart by the modules' contract (crates/module).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn settings(&self) -> Vec<&'static Setting> {
        api::SETTINGS.to_vec()
    }
}
