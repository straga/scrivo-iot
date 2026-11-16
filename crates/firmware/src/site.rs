//! What a device tells its page: the dictionary of its entities and, by their addresses, the values of what every device carries - the board, its memory and time, the console and the
//! station - and of the device's own modules (crates/module).

use page::dictionary::{Said, Served, Value};

use crate::board::{self, Esp};

pub struct Site {
    pub device: &'static str,
    pub board: &'static str,
    /// The image's version (APP_VERSION of the device).
    pub kit: &'static str,
    /// The device's modules, in the order its page draws their cards. Everything the page and the console ask of a
    /// module - its rows, its switches and sliders, its lines for the console, its settings and what they mean now, its
    /// curves - is answered by walking this list (crates/module), so a module is named here and nowhere else.
    pub modules: &'static module::Modules,
    /// What the device knows about its own settings that the mechanism cannot: what an older image kept and what it means now,
    /// which mode needs what (`settings::Rules`). `settings::NO_RULES` for a device whose settings are the engine's alone.
    pub rules: &'static settings::Rules,
    /// What the device's description seeds its settings with when nvs has nothing: name and value.
    pub description: &'static [(&'static str, &'static str)],
    /// The device's dictionary, made when its image was built (`page::dictionary`, `dictionary::image`).
    pub dictionary: Served,
}

/// The device's modules. Set once by firmware::run.
static MODULES: std::sync::OnceLock<&'static module::Modules> = std::sync::OnceLock::new();

/// The device's own cards, worked out of its modules once. Set once by firmware::run.
static CARDS: std::sync::OnceLock<&'static [&'static str]> = std::sync::OnceLock::new();

fn modules() -> &'static module::Modules {
    MODULES.get().copied().unwrap_or(&[])
}

/// Keeps the device's modules and works out its cards. Called once by firmware::run, before anything asks.
pub(crate) fn keep(site: &Site) {
    let _ = MODULES.set(site.modules);
    let _ = CARDS.set(Box::leak(module::cards(site.modules).into_boxed_slice()));
}

/// Hands WHAT to whichever module carries out ROLE on this device (crates/module, `command`).
pub fn command<T: Clone + 'static>(role: &module::Role<T>, what: T) -> Result<(), String> {
    module::command(modules(), role, what)
}

/// The settings of the device's modules, in the order of the modules.
pub(crate) fn settings() -> Vec<&'static settings::Setting> {
    module::settings(modules())
}

/// The curve called NAME, for the web server's /curve.
pub fn curve(name: &str) -> Option<page::Curve> {
    module::curve(modules(), name)
}

/// Switches a thing over by name, for the shell's `toggle` and the shared page: on top of `switch`.
pub(crate) fn toggle(name: &str) -> Result<(), String> {
    module::toggle(modules(), name)
}

/// Presses a button by name, for the shell's `press`.
pub(crate) fn press(name: &str) -> Result<(), String> {
    module::press(modules(), name)
}

/// Turns a thing on or off by name, for the shell's `on` and `off`: a person's command (crates/module, `by_person`).
pub(crate) fn switch(name: &str, on: bool) -> Result<(), String> {
    module::switch(modules(), name, on)
}

/// Sets a thing to a number by name - a person's, from the console (crates/module, `by_person`).
pub fn hold(name: &str, value: u32) -> Result<(), String> {
    module::hold(modules(), name, value)
}

/// What NAME is after a command, for the console: on or off where the board keeps a switch of that name.
pub(crate) fn said(name: &str) -> String {
    match module::is_on(modules(), name) {
        Some(on) => format!("{name} {}", if on { "on" } else { "off" }),
        None => format!("{name} done"),
    }
}

/// The console words the device's modules add (crates/module, `Verb`), for the shell's `help` and its lines.
pub(crate) fn words() -> Vec<shell::Word> {
    module::verbs(modules()).into_iter().map(|v| shell::Word { name: v.name, help: v.help }).collect()
}

/// The console word NAME with WORDS after it, carried out by the module that has it.
pub(crate) fn word(name: &str, words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    module::verb(modules(), name, words, line)
}

/// Whether some module of the device carries out the shell's word NAME that not every board has.
pub(crate) fn takes(name: &str) -> bool {
    module::takes(modules(), name)
}

impl web::Board for Site {
    fn dictionary(&self) -> Served {
        self.dictionary
    }

    fn board(&self) -> String {
        self.board.into()
    }

    fn restart(&self) {
        shell::Board::restart(&mut Esp);
    }

    fn down(&self) {
        if let Err(why) = shell::Board::down(&mut Esp) {
            log::error!("down from the page refused: {why}");
        }
    }

    /// The size and a tag - FNV-1a over the size and the block the file's last piece lies in -, from the file's
    /// directory entry and not from its bytes: a read of every byte ran the network's loop for seconds on a /journal
    /// (a device with a long one, 26.09). A file written again lies in new blocks, so its tag changes; the tag is not of the image's
    /// version, since the page is uploaded by the recovery image and the firmware by another step. A file small enough
    /// to lie inside its directory has no block of its own, and its bytes - 128 at most - make its tag.
    fn file(&self, path: &str) -> Option<(usize, u64)> {
        let fnv = |hash: u64, bytes: &[u8]| bytes.iter().fold(hash, |h, b| (h ^ *b as u64).wrapping_mul(0x100_0000_01b3));
        let stat = vfs::stat(path).ok()?;
        let hash = fnv(0xcbf2_9ce4_8422_2325, &(stat.size as u64).to_le_bytes());
        let hash = match stat.head {
            Some(block) => fnv(hash, &block.to_le_bytes()),
            None => {
                let mut bytes = [0u8; 128];
                let n = vfs::read_at(path, 0, &mut bytes).ok()?;
                fnv(hash, &bytes[..n])
            }
        };
        Some((stat.size, hash))
    }

    fn read_at(&self, path: &str, at: usize, buf: &mut [u8]) -> Result<usize, String> {
        vfs::read_at(path, at, buf).map_err(|e| e.to_string())
    }

    fn files(&self, dir: &str) -> Result<Vec<String>, String> {
        vfs::files(dir).map_err(|e| e.to_string())
    }
}

/// A command of contract 3 to the thing NAME, in the task that may wait on its owner.
pub(crate) fn act(site: &Site, name: &str, command: page::dictionary::Command) -> Result<(), String> {
    use page::dictionary::Command;
    match command {
        Command::Switch(on) => module::switch(site.modules, name, on),
        Command::Toggle => module::toggle(site.modules, name),
        Command::Press => module::press(site.modules, name),
        Command::Hold(value) => module::hold(site.modules, name, value),
    }
}

/// The curve called NAME of the site's modules.
pub(crate) fn curve_of(site: &Site, name: &str) -> Option<page::Curve> {
    module::curve(site.modules, name)
}

/// The board as its values (contract 3): the card every board carries, then the modules', then the settings - each by
/// the address its device's dictionary gives it, handed over as it is made.
impl page::Values for Site {
    fn version(&self) -> &str {
        self.dictionary.version
    }

    fn cards(&self) -> &'static [&'static str] {
        CARDS.get().copied().unwrap_or(&[])
    }

    fn values(&self, wanted: &page::Wanted, each: &mut dyn FnMut(usize, Said<'_>)) {
        let served = self.dictionary;
        if wanted.has(page::SYSTEM) {
            system_values(served.table, each);
        }
        // what each module's owner last published; the module's own code does not run here
        module::card::read(self.modules, served.table, wanted, each);
        if wanted.has(page::SETTINGS) {
            setting_values(served, self.modules, each);
        }
    }
}

/// Text at an address, or nothing there.
fn text(t: Option<&str>) -> Said<'_> {
    t.map_or(Said::Value(Value::None), Said::Text)
}

/// The values of the card every board carries (`dictionary::system`).
fn system_values(table: &[page::dictionary::Entry], each: &mut dyn FnMut(usize, Said<'_>)) {
    use dictionary::system as at;
    let consoles = {
        let (browsers, port) = web::consoles_open();
        browsers + port
    };
    each(at::CONSOLE, Value::Number(consoles as f32).into());
    let link = board::link();
    let joined = link.as_ref().filter(|l| l.ip.is_some());
    each(at::WIFI, Value::Word(if joined.is_some() { "joined" } else { "not_joined" }).into());
    each(at::WIFI_IP, text(joined.and_then(|l| l.ip.as_deref())));
    let ssid = joined.map(|l| l.ssid.to_string());
    each(at::WIFI_SSID, text(ssid.as_deref()));
    each(at::WIFI_SIGNAL, joined.map_or(Value::None, |l| Value::Number(f32::from(l.rssi))).into());
    let (free, used) = board::heap();
    let kilobytes = |bytes: usize| Value::Number((bytes / 1024 * 1024) as f32);
    each(at::FREE, kilobytes(free).into());
    each(at::USED, kilobytes(used).into());
    each(at::LOW_MEMORY, Value::On(free < page::LOW_MEMORY).into());
    each(at::UPTIME, Value::Number(board::uptime_ms() as f32).into());
    let heard = crate::clock::heard();
    each(
        at::TIME,
        match (&heard.now, heard.asked.is_empty()) {
            (Some(now), _) => Said::Text(now),
            (None, true) => Value::Unknown("not_joined").into(),
            (None, false) => Value::Unknown("silent").into(),
        },
    );
    each(at::TIME_ZONE, Said::Text(&heard.zone));
    each(at::TIME_FROM, server(&heard.from));
    each(at::TIME_GATEWAY, text(heard.gateway.as_deref()));
    // who does not answer, when nobody does: the router apart, for the client to name it by its address
    let silent: Vec<&String> = heard.asked.iter().filter(|s| heard.now.is_none() && s.as_str() != "gateway").collect();
    let names = silent.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
    each(at::TIME_SILENT, text((!names.is_empty()).then_some(names.as_str())));
    each(at::TIME_ROUTER_SILENT, Value::On(heard.now.is_none() && heard.asked.iter().any(|s| s == "gateway")).into());
    // how busy each core is, where the device's table has the rows (`dictionary::system::CORES`): none before the first window of five seconds is over
    for (name, share) in ["core_0", "core_1"].into_iter().zip(crate::cores::percents()) {
        if let Some(row) = page::dictionary::address(table, page::SYSTEM, name) {
            each(row, share.map_or(Value::None, |per_cent| Value::Number(f32::from(per_cent))).into());
        }
    }
}

/// A server of the time: the router by its code, any other by the name the setting gives it.
fn server(named: &Option<String>) -> Said<'_> {
    match named.as_deref() {
        None => Value::None.into(),
        Some("gateway") => Value::Word("gateway").into(),
        Some(host) => Said::Text(host),
    }
}

/// The settings of the device as kept, each by its row in the card of settings, and what the ones that say
/// something now say: the clock's note here, the modules' by their own addresses.
fn setting_values(served: Served, modules: &module::Modules, each: &mut dyn FnMut(usize, Said<'_>)) {
    let table = served.table;
    // one set for the whole answer: a save landing in the middle of it would otherwise show half of each
    let now = crate::nvs_settings::now();
    for (at, entry) in table.iter().enumerate().filter(|(_, e)| e.card == page::SETTINGS && e.part_of.is_none()) {
        let kept = now.kept(entry.name);
        each(at, text(kept.as_deref()));
    }
    if let Some(time) = page::dictionary::address(table, page::SETTINGS, settings::TIME.name) {
        let heard = crate::clock::heard();
        let part = |name| page::dictionary::part(table, time, name);
        let note = match (&heard.from, &heard.last_at) {
            (Some(_), Some(_)) => "came_from_at",
            (Some(_), None) => "came_from",
            (None, _) if heard.synced => "silent_time_kept",
            (None, _) => "no_answer",
        };
        let said = [
            ("note", Value::Word(note).into()),
            ("from", server(&heard.from)),
            ("gateway", text(heard.gateway.as_deref())),
            ("at", text(heard.last_at.as_deref())),
        ];
        for (name, said) in said {
            if let Some(at) = part(name) {
                each(at, said);
            }
        }
    }
    module::setting_values(modules, &mut |at, value| each(at, (*value).into()));
}
