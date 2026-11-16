//! The owner of the flash: asked at its desk from other threads, as the page, the console and the
//! automaton ask it on the board; the set it publishes is read the way every reader reads it.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use flash::{Answer, Ask, Desk, File, Owner};
use module::Role;
use settings::{seed, Keeper, Setting, Store, Values, ENGINE as ALL, NO_RULES};

/// nvs as a map, the writes in the order they came, and a flash that can be told to refuse every write from now on.
#[derive(Default)]
struct Nvs {
    kept: BTreeMap<String, String>,
    writes: Arc<Mutex<Vec<String>>>,
    refuses: bool,
}

impl Store for Nvs {
    fn get(&self, name: &str) -> Option<String> {
        self.kept.get(name).cloned()
    }

    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        if self.refuses {
            return Err("nvs: ESP_ERR_NVS_NOT_ENOUGH_SPACE".into());
        }
        self.writes.lock().unwrap().push(format!("{name}: {}", value.replace('\n', " ")));
        self.kept.insert(name.into(), value.into());
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<bool, String> {
        if self.refuses {
            return Err("nvs: ESP_ERR_NVS_NOT_ENOUGH_SPACE".into());
        }
        Ok(self.kept.remove(name).is_some())
    }
}

/// vfs as a list of what was done to it, in order; a path under /full is refused as a flash with no room refuses it.
static FILES: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn files(file: &File) -> Result<(), String> {
    let (what, path) = match file {
        File::Write { path, bytes } => (format!("write {} bytes", bytes.len()), path),
        File::Append { path, bytes } => (format!("append {} bytes", bytes.len()), path),
        File::Remove(path) => ("remove".to_string(), path),
    };
    if path.starts_with("/full") {
        return Err("vfs: -28".into());
    }
    FILES.lock().unwrap().push(format!("{what} {path}"));
    Ok(())
}

fn setting(name: &str) -> &'static Setting {
    ALL.iter().copied().find(|s| s.name == name).unwrap()
}

/// A board seeded as a new one is, its owner serving DESK on a thread of its own until STOP; the store's writes log.
fn owner_serving(desk: &'static Desk, role: &'static Role<Arc<Values>>, refuses: bool) -> (Arc<AtomicBool>, Arc<Mutex<Vec<String>>>) {
    let mut nvs = Nvs::default();
    seed(&ALL, &NO_RULES, &mut nvs, &[]);
    nvs.refuses = refuses;
    let writes = nvs.writes.clone();
    writes.lock().unwrap().clear();
    let stop = Arc::new(AtomicBool::new(false));
    let until = stop.clone();
    let mut owner = Owner::new(Some(Keeper::new(ALL.to_vec(), &NO_RULES, nvs)), role, files);
    std::thread::spawn(move || {
        while !until.load(Ordering::Relaxed) {
            desk.serve_for(Duration::from_millis(5), |ask| owner.answer(ask));
        }
    });
    (stop, writes)
}

fn change(desk: &Desk, pairs: &[(&str, &str)]) -> Result<settings::Changed, settings::Refusal> {
    let pairs = pairs.iter().map(|(n, v)| (n.to_string(), v.to_string())).collect();
    match desk.ask(Ask::Change(pairs), Duration::from_secs(2)) {
        Ok(Answer::Changed(said)) => said,
        Ok(_) => panic!("a save was answered as something else"),
        Err(refused) => panic!("the owner did not answer: {refused:?}"),
    }
}

/// draught_pa and poll_every as a reader sees them now - two cards, two keys of nvs.
fn pair(role: &Role<Arc<Values>>) -> Option<(String, String)> {
    let set = role.latest()?;
    Some((set.get(setting("net_point_tries")), set.get(setting("net_reset_laps"))))
}

#[test]
fn a_change_that_comes_while_a_reader_reads_reaches_it_whole_the_old_set_or_the_new() {
    static DESK: Desk = Desk::new();
    static ROLE: Role<Arc<Values>> = Role::new("settings");
    let (stop, _) = owner_serving(&DESK, &ROLE, false);
    let first = pair(&ROLE);
    let reading = Arc::new(AtomicBool::new(true));
    let still = reading.clone();
    let reader = std::thread::spawn(move || {
        let mut seen = Vec::new();
        while still.load(Ordering::Relaxed) {
            if let Some(p) = pair(&ROLE) {
                if !seen.contains(&p) {
                    seen.push(p);
                }
            }
        }
        seen
    });
    for round in 0..40 {
        let (pa, every) = if round % 2 == 0 { ("4", "5") } else { ("6", "7") };
        change(&DESK, &[("net_point_tries", pa), ("net_reset_laps", every)]).expect("a good save refused");
    }
    reading.store(false, Ordering::Relaxed);
    let seen = reader.join().unwrap();
    stop.store(true, Ordering::Relaxed);
    let whole = [("4", "5"), ("6", "7")].map(|(a, b)| (a.to_string(), b.to_string()));
    assert!(seen.iter().any(|p| whole.contains(p)), "the reader never saw a save at all: {seen:?}, before {first:?}");
    let mixed: Vec<_> = seen.iter().filter(|p| !whole.contains(p) && Some(*p) != first.as_ref()).collect();
    assert!(mixed.is_empty(), "a reader saw half of one save and half of another: {mixed:?}");
}

#[test]
fn two_saves_in_a_row_land_on_the_flash_in_the_order_they_came() {
    static DESK: Desk = Desk::new();
    static ROLE: Role<Arc<Values>> = Role::new("settings");
    let (stop, writes) = owner_serving(&DESK, &ROLE, false);
    let first = std::thread::spawn(|| change(&DESK, &[("net_point_tries", "8")]));
    std::thread::sleep(Duration::from_millis(2));
    let second = std::thread::spawn(|| change(&DESK, &[("net_point_tries", "9")]));
    assert!(first.join().unwrap().is_ok(), "the first save was not answered with its lines");
    assert!(second.join().unwrap().is_ok(), "the second save was not answered with its lines");
    stop.store(true, Ordering::Relaxed);
    let writes = writes.lock().unwrap().clone();
    let at = |v: &str| writes.iter().position(|w| w.contains(&format!("net_point_tries={v}")));
    assert!(at("8").is_some() && at("9").is_some(), "both saves did not reach the flash: {writes:?}");
    assert!(at("8") < at("9"), "the saves reached the flash out of order: {writes:?}");
    assert_eq!(pair(&ROLE).map(|p| p.0), Some("9".to_string()), "the set published is not the last save");
}

#[test]
fn a_save_the_flash_refuses_is_refused_to_its_sender_and_the_published_set_stays() {
    static DESK: Desk = Desk::new();
    static ROLE: Role<Arc<Values>> = Role::new("settings");
    let (stop, _) = owner_serving(&DESK, &ROLE, true);
    let before = ROLE.latest().expect("the owner published no set at start");
    let said = change(&DESK, &[("net_point_tries", "4")]);
    stop.store(true, Ordering::Relaxed);
    let why = said.err().expect("a save the flash refused was answered as done");
    assert_eq!(why.code(), "not_written", "{why}");
    assert!(why.to_string().contains("NOT_ENOUGH_SPACE"), "the refusal does not say why: {why}");
    let after = ROLE.latest().unwrap();
    assert!(Arc::ptr_eq(&before, &after), "the set published moved on a refused save");
    assert_eq!(after.get(setting("net_point_tries")), before.get(setting("net_point_tries")));
}

#[test]
fn what_a_module_writes_down_through_the_owner_it_reads_back() {
    static DESK: Desk = Desk::new();
    static ROLE: Role<Arc<Values>> = Role::new("settings");
    let (stop, _) = owner_serving(&DESK, &ROLE, false);
    let wrote = DESK.ask(Ask::Remember { card: "automaton".into(), text: "stop".into() }, Duration::from_secs(2));
    assert!(matches!(wrote, Ok(Answer::Remembered(Ok(())))), "the memory was not written");
    let read = DESK.ask(Ask::Recall("automaton".into()), Duration::from_secs(2));
    stop.store(true, Ordering::Relaxed);
    assert!(matches!(read, Ok(Answer::Recalled(Some(ref t))) if t == "stop"), "the memory did not read back");
}

#[test]
fn files_are_written_by_the_owner_in_the_order_asked_and_a_refusal_comes_back_to_the_sender() {
    static DESK: Desk = Desk::new();
    static ROLE: Role<Arc<Values>> = Role::new("settings");
    let (stop, _) = owner_serving(&DESK, &ROLE, false);
    let file = |f: File| match DESK.ask(Ask::File(f), Duration::from_secs(2)) {
        Ok(Answer::Filed(done)) => done,
        _ => panic!("a file was not answered as a file"),
    };
    assert_eq!(file(File::Write { path: "/data/count/state".into(), bytes: vec![1; 12] }), Ok(()));
    assert_eq!(file(File::Append { path: "/data/count/2026-09.csv".into(), bytes: vec![2; 30] }), Ok(()));
    assert_eq!(file(File::Remove("/data/count/2026-08.csv".into())), Ok(()));
    let refused = file(File::Write { path: "/full/state".into(), bytes: vec![3] });
    stop.store(true, Ordering::Relaxed);
    assert_eq!(refused, Err("vfs: -28".to_string()), "a write vfs refused was not refused to its sender");
    let done: Vec<String> = FILES.lock().unwrap().iter().filter(|l| l.contains("/data/count/")).cloned().collect();
    assert_eq!(
        done,
        ["write 12 bytes /data/count/state", "append 30 bytes /data/count/2026-09.csv", "remove /data/count/2026-08.csv"],
        "the files were not done by the owner, or not in the order asked"
    );
}
