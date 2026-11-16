//! What the shell's commands do on this board. Stateless: every shell - USB and each TCP connection -
//! gets its own `Esp` and asks IDF for the facts at the moment of the command.

use core::ffi::CStr;

use esp_idf_svc::ota::EspOta;
use esp_idf_svc::sys;

pub struct Esp;

static FLUSH: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
static RESTART: std::sync::Mutex<daily::Request> = std::sync::Mutex::new(daily::Request::new());
static PLANNED: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
static RESTART_HOOKS: std::sync::OnceLock<RestartHooks> = std::sync::OnceLock::new();

/// What the module that plans restarts lends the owner of the flash: whether a relay of the auger or the plug is closed now (read at the last moment, after the journal is down), and what is written down as the board goes.
#[derive(Clone, Copy)]
pub struct RestartHooks {
    pub organ_closed: fn() -> bool,
    pub going: fn(),
}

/// Lends the hooks of the planned restart; once, at start.
pub fn set_restart_hooks(hooks: RestartHooks) {
    let _ = RESTART_HOOKS.set(hooks);
}

/// Has the owner of the flash write the journal's waiting lines at the end of this pass, not when their batch is due: the lines a module has just logged are down when the call returns to the owner's loop.
pub fn journal_soon() {
    FLUSH.store(true, core::sync::atomic::Ordering::Release);
}

/// Asks the owner of the flash to restart the board at the end of this pass, if the journal went down to the flash and no relay of the auger or the plug is closed: it asks again every pass until it goes, or [`cancel_restart`].
pub fn restart_after_this_pass() {
    RESTART.lock().unwrap().ask();
}

/// The reason to restart is gone: nothing is asked for.
pub fn cancel_restart() {
    RESTART.lock().unwrap().cancel();
}

pub(crate) fn take_flush() -> bool {
    FLUSH.swap(false, core::sync::atomic::Ordering::AcqRel)
}

pub(crate) fn restart_asked() -> bool {
    RESTART.lock().unwrap().asked()
}

/// What the owner does with the restart that was asked for, once it tried to write the journal (JOURNAL_DOWN says it went).
pub(crate) fn settle_restart(journal_down: bool) -> daily::Settled {
    let organ_closed = RESTART_HOOKS.get().is_some_and(|hooks| (hooks.organ_closed)());
    RESTART.lock().unwrap().settle(journal_down, organ_closed)
}

/// Lets the module that plans restarts write down what it keeps, as the board goes.
pub(crate) fn going() {
    if let Some(hooks) = RESTART_HOOKS.get() {
        (hooks.going)();
    }
}

/// What `status` says of the last planned restart: the module that plans them sets it at start, and again when it plans one.
pub fn set_planned(words: String) {
    *PLANNED.lock().unwrap() = words;
}

/// The leaps of the tick's counter as the interrupt of the tick keeps them (systick.c), for `between::tick::Watch`: numbers, and a line is the caller's.
pub fn tick_leaps() -> between::tick::TickLeaps {
    let leaps = unsafe { esp_idf_svc::sys::systick::systick_leaps() };
    between::tick::TickLeaps {
        count: leaps.count,
        last_ticks: leaps.last_ticks,
        last_was_back: leaps.last_was_back != 0,
        largest_back: leaps.largest_back,
        largest_forward: leaps.largest_forward,
        rearms: leaps.rearms,
    }
}

/// The time since start by the board's steady clock: a leap of the chip's counter is not in it. Every stamp of
/// the journal and every `up` the board says is taken here.
pub fn uptime_ms() -> i64 {
    between::clock::uptime_ms() as i64
}

fn c_str(chars: &[core::ffi::c_char]) -> String {
    let bytes: Vec<u8> = chars.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn running() -> (String, &'static str) {
    unsafe {
        let part = sys::esp_ota_get_running_partition();
        if part.is_null() {
            return ("?".into(), "unknown");
        }
        let label = CStr::from_ptr((*part).label.as_ptr()).to_string_lossy().into_owned();
        let mut state = 0;
        let state = if sys::esp_ota_get_state_partition(part, &mut state) != sys::ESP_OK {
            "no ota state"
        } else {
            match state {
                sys::esp_ota_img_states_t_ESP_OTA_IMG_VALID => "confirmed",
                sys::esp_ota_img_states_t_ESP_OTA_IMG_PENDING_VERIFY => "not confirmed yet",
                sys::esp_ota_img_states_t_ESP_OTA_IMG_NEW => "new",
                _ => "invalid",
            }
        };
        (label, state)
    }
}

/// The station's link: the network, the address from the router, how well it is heard, and which access point of the
/// network it is - two of them share one name; None out of the network.
pub struct Link {
    pub ssid: String,
    pub ip: Option<String>,
    pub rssi: i8,
    pub bssid: [u8; 6],
    pub channel: u8,
}

pub fn link() -> Option<Link> {
    unsafe {
        let mut ap: sys::wifi_ap_record_t = core::mem::zeroed();
        if sys::esp_wifi_sta_get_ap_info(&mut ap) != sys::ESP_OK {
            return None;
        }
        let ssid: Vec<u8> = ap.ssid.iter().copied().take_while(|&b| b != 0).collect();
        let netif = sys::esp_netif_get_handle_from_ifkey(c"WIFI_STA_DEF".as_ptr());
        let mut ip: sys::esp_netif_ip_info_t = core::mem::zeroed();
        let ip = (!netif.is_null() && sys::esp_netif_get_ip_info(netif, &mut ip) == sys::ESP_OK && ip.ip.addr != 0)
            .then(|| {
                let a = ip.ip.addr.to_le_bytes();
                format!("{}.{}.{}.{}", a[0], a[1], a[2], a[3])
            });
        Some(Link { ssid: String::from_utf8_lossy(&ssid).into_owned(), ip, rssi: ap.rssi, bssid: ap.bssid, channel: ap.primary })
    }
}

/// A MAC address as the router and `airport -s` print it: dc:2c:6e:7f:39:4f.
pub fn mac(bytes: &[u8; 6]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(":")
}

fn network() -> String {
    match link() {
        None => "no network".into(),
        Some(l) => format!(
            "'{}' {} rssi {} bssid {} channel {}",
            l.ssid,
            l.ip.as_deref().unwrap_or("no address"),
            l.rssi,
            mac(&l.bssid),
            l.channel
        ),
    }
}

/// Why the chip came up this time, straight from it: the code and what it means in words.
///
/// It is asked at any time and always answers about THIS start - the chip keeps it until the next reset - so
/// the line can be logged at boot and shown in `status` afterwards, which matters: a restart takes the log with
/// it, and the person who finds the board up may never have seen the line.
pub fn why_up() -> (u32, String) {
    let code = unsafe { sys::esp_reset_reason() } as u32;
    (code, reset_why::words(code))
}

/// Free and used internal heap, in bytes.
pub fn heap() -> (usize, usize) {
    let internal = sys::MALLOC_CAP_INTERNAL;
    unsafe {
        let free = sys::heap_caps_get_free_size(internal);
        (free, sys::heap_caps_get_total_size(internal).saturating_sub(free))
    }
}

/// The least a task's stack may keep free under load, in bytes (6.6: any task below it gets a bigger stack, with
/// the measurement beside the number).
pub const ROOMY: usize = 1536;

/// How much stack each task has NEVER used, smallest first - the tasks closest to falling over.
///
/// **A STACK OVERFLOW IS NOT DEBUGGED FROM THE TRACE.** FreeRTOS catches it at a context switch, so what the
/// trace names is the scheduler and not the code that ate the stack; the board fell exactly that way on 21.09
/// (`A stack overflow in task pthread has been detected`). What picks the repair is this number, and it has to
/// be readable BEFORE the fall, on a board that is running.
///
/// The number is a high-water mark in bytes: the least free the task's stack has ever been since it started.
/// Printed: every task with less than [`ROOMY`] free, and never fewer than the tightest few - a board runs
/// twenty-odd tasks and the roomy ones say nothing useful, but a tight one that is sixth is still tight.
pub fn timers_line() -> String {
    // the tasks waiting by embassy-time: the places of the time driver's queue taken now, at the most, the wakers that found
    // none, the times esp_timer refused its alarm, and whether the beat by the tick runs (between::steady_time)
    let waiting = between::steady_time::waiters();
    format!(
        "timers: {} waiting, {} at most, {} crowded out, {} alarms refused, beat {}, {} timers before it",
        waiting.held,
        waiting.high,
        waiting.crowded_out,
        waiting.arm_failed,
        if waiting.beating { "on" } else { "off" },
        waiting.early
    )
}

/// What a silent draught leaves for the person who finds the board later: how long it has been up, the timers as `status` says them, and the tasks with the least free stack.
pub fn clue_lines() -> Vec<String> {
    let mut lines = vec![format!("up {} s", uptime_ms() / 1000), timers_line()];
    // what each round that reads the shared analog unit holds, waits for and takes as a step - the lock the silences of the draught and the flame took turns at
    lines.extend(analog::board::lane_lines());
    lines.push(load_line());
    lines.extend(stacks());
    lines
}

/// Every task the kernel has now, with what it knows of each.
fn tasks_now() -> Vec<sys::TaskStatus_t> {
    let count = unsafe { sys::uxTaskGetNumberOfTasks() } as usize;
    let mut tasks: Vec<sys::TaskStatus_t> = vec![unsafe { core::mem::zeroed() }; count];
    let filled =
        unsafe { sys::uxTaskGetSystemState(tasks.as_mut_ptr(), count as u32, core::ptr::null_mut()) } as usize;
    tasks.truncate(filled.min(count));
    tasks
}

/// The name of a task as the kernel keeps it.
fn task_name(t: &sys::TaskStatus_t) -> String {
    // the name is a NUL-terminated C string the kernel owns; c_str takes it as the bytes it is
    unsafe { core::ffi::CStr::from_ptr(t.pcTaskName) }.to_string_lossy().into_owned()
}

/// Which core each task is pinned to: the network's side on 0, the hardware's on 1, and whatever is left to the
/// scheduler said as such - a task of ours there was born without a core.
fn pinned_line(tasks: &[sys::TaskStatus_t]) -> String {
    let mut cores: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for t in tasks {
        let at = match unsafe { sys::xTaskGetCoreID(t.xHandle) } {
            0 => 0,
            1 => 1,
            _ => 2,
        };
        cores[at].push(task_name(t));
    }
    cores.iter_mut().for_each(|names| names.sort_unstable());
    format!("cores: 0 - {}; 1 - {}; either - {}", cores[0].join(", "), cores[1].join(", "), cores[2].join(", "))
}

/// How busy each core was in the last window (`cores`), one line: the numbers a person reads `status` for.
fn load_line() -> String {
    let said = |share: Option<u8>| share.map_or_else(|| "not measured yet".to_string(), |per_cent| format!("{per_cent} %"));
    let [core_0, core_1] = crate::cores::percents();
    format!("core load, last {} s: core 0 {}, core 1 {}", core_load::WINDOW_US / 1_000_000, said(core_0), said(core_1))
}

fn stacks() -> Vec<String> {
    const SHOWN: usize = 5;
    let tasks = tasks_now();
    let mut seen: Vec<(u32, String)> = tasks.iter().map(|t| (t.usStackHighWaterMark as u32, task_name(t))).collect();
    // Every task by name with its least free stack, in one line: what tells a new task from one that should not be
    // there (6.6: the network is one loop and one reactor), and what a stack is set by - used under load plus ROOMY -
    // for a task the tightest few leave out.
    let mut named = seen.iter().map(|(free, name)| (name.as_str(), *free)).collect::<Vec<_>>();
    named.sort_unstable();
    let listed = named.iter().map(|(name, free)| format!("{name} {free}")).collect::<Vec<_>>();
    let all = format!("tasks {}: {}", listed.len(), listed.join(", "));
    let pinned = pinned_line(&tasks);
    seen.sort_by_key(|(free, _)| *free);
    let tight = seen.iter().take_while(|(free, _)| (*free as usize) < ROOMY).count();
    seen.truncate(tight.max(SHOWN));
    let said = seen.iter().map(|(free, name)| format!("{name} {free}")).collect::<Vec<_>>().join(", ");
    vec![format!("least stack ever free, bytes: {said}"), all, pinned]
}

impl shell::Board for Esp {
    fn room(&self) -> Option<shell::Room> {
        let internal = sys::MALLOC_CAP_INTERNAL;
        unsafe {
            Some(shell::Room {
                free: sys::heap_caps_get_free_size(internal),
                largest: sys::heap_caps_get_largest_free_block(internal),
            })
        }
    }

    fn status(&mut self, line: &mut dyn FnMut(&str)) {
        let desc = unsafe { &*sys::esp_app_get_description() };
        let (partition, state) = running();
        line(&format!("version {} ({}), running {partition}, {state}", c_str(&desc.version), c_str(&desc.project_name)));
        line(&format!("up {} s", uptime_ms() / 1000));
        let (code, why) = why_up();
        line(&format!("came up: {why} [code {code}]"));
        // Where it fell, when it fell: the boot lines scroll away, and this is the fact that picks the repair.
        for said in crate::fell::lines() {
            line(&said);
        }
        let internal = sys::MALLOC_CAP_INTERNAL;
        unsafe {
            line(&format!(
                "internal heap: free {}, largest block {}, lowest free since start {}",
                sys::heap_caps_get_free_size(internal),
                sys::heap_caps_get_largest_free_block(internal),
                sys::heap_caps_get_minimum_free_size(internal),
            ));
        }
        // **THE EXTERNAL MEMORY, AND ONLY WHEN THERE IS ANY.** A board without PSRAM, or one built without it
        // switched on, says nothing here rather than a row of zeroes: a number that is always nought teaches
        // the reader to skip the line, and then the day it matters they skip it too.
        unsafe {
            let spiram = sys::MALLOC_CAP_SPIRAM;
            let total = sys::heap_caps_get_total_size(spiram);
            if total > 0 {
                line(&format!(
                    "external heap: free {} of {}, largest block {}, lowest free since start {}",
                    sys::heap_caps_get_free_size(spiram),
                    total,
                    sys::heap_caps_get_largest_free_block(spiram),
                    sys::heap_caps_get_minimum_free_size(spiram),
                ));
            }
        }
        // **WHAT THIS IMAGE HAS WRITTEN TO THE SETTINGS, so that "it left no trace" is a number and not a
        // reading of the code.** A write shuts the cache off while it runs, so the line answers two questions
        // at once: how much the settings have worn the flash, and how many such windows were opened since the
        // board came up. It counts this writer alone - the short-start count, the image confirmation and the
        // Wi-Fi setting write elsewhere. The modules' memory - the state a module keeps for itself - has a line of its own: a
        // start that only wrote that down changed no setting, and one line for both said it had.
        for said in crate::nvs_settings::written().lines() {
            line(&said);
        }
        line(&format!("wifi {}", network()));
        line(&format!("time {}", crate::clock::words()));
        line(&format!("last planned restart: {}", PLANNED.lock().unwrap().as_str()));
        let (_, on_port) = web::consoles_open();
        line(&format!("consoles on tcp {on_port} of {}", web::orders::PORT_CONSOLES));
        let h = web::handlers();
        line(&format!(
            "page handlers: {} of {} hold a connection - streams {}, browser consoles {}, other answers {}, kept between \
             requests {}",
            h.held,
            web::HANDLERS,
            h.streams,
            h.browser_consoles,
            h.answering,
            h.kept
        ));
        // the longest the loop held its core without falling asleep, in CPU: the idle task of core 0 feeds the task
        // watchdog only while the loop sleeps, and five seconds of it reset the board (seen on a device with a busy loop, 26.09)
        let (us, at_ms) = web::STEPS.longest();
        line(&format!(
            "net loop: longest on its core without a sleep {} ms, ended {} s after start",
            us / 1000,
            at_ms / 1000
        ));
        // how busy each core is, and which tasks sit on which: a core held by a task that spins is what a mean of the two hides
        line(&load_line());
        line(&pinned_line(&tasks_now()));
        // the leaps the chip's clock took, which the board's steady clock did not take after it (between::clock)
        line(&between::clock::jumps().to_string());
        // the tasks waiting by embassy-time: the places of the time driver's queue taken now, at the most, the wakers that found
        // none, the times esp_timer refused its alarm, and whether the beat by the tick runs (between::steady_time)
        line(&timers_line());
        // each round that reads the shared analog unit: how long it held the lock, waited for it and took as a step (analog::gauge)
        for lane in analog::board::lane_lines() {
            line(&lane);
        }
        // the leaps of the systimer's counter the tick of FreeRTOS resynced to instead of catching up with (systick.c)
        let leaps = unsafe { esp_idf_svc::sys::systick::systick_leaps() };
        line(&format!(
            "tick leaps: {}, largest back {} ms, forward {} ms, alarms set again by the watcher {}",
            leaps.count, leaps.largest_back, leaps.largest_forward, leaps.rearms
        ));
        // the journal on flash writes in batches, and this is how many: the wear it costs, as a number, and how many
        // the flash refused - their lines still wait in RAM
        line(&format!(
            "journal on flash: {} batches written since start, {} refused, in {}",
            crate::log_tee::WRITES.load(std::sync::atomic::Ordering::Acquire),
            crate::log_tee::REFUSALS.load(std::sync::atomic::Ordering::Acquire),
            journal::kept::FOLDER
        ));
        for said in stacks() {
            line(&said);
        }
    }

    fn restart(&mut self) {
        log::info!("restart asked from the shell");
        std::thread::sleep(std::time::Duration::from_millis(200));
        unsafe { sys::esp_restart() };
    }

    fn down(&mut self) -> Result<(), String> {
        let mut ota = EspOta::new().map_err(|e| format!("ota: {e}"))?;
        ota.factory_reset().map_err(|e| format!("boot partition not set to factory: {e}"))?;
        log::info!("boot set to factory - restarting into the recovery image");
        std::thread::sleep(std::time::Duration::from_millis(200));
        unsafe { sys::esp_restart() };
        #[allow(unreachable_code)]
        Ok(())
    }

    fn toggle(&mut self, name: &str) -> Result<String, String> {
        crate::site::toggle(name)?;
        Ok(crate::site::said(name))
    }

    fn press(&mut self, name: &str) -> Result<String, String> {
        crate::site::press(name)?;
        Ok(format!("{name} pressed"))
    }

    fn switch(&mut self, name: &str, on: bool) -> Result<String, String> {
        crate::site::switch(name, on)?;
        Ok(crate::site::said(name))
    }

    fn fell(&mut self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        let said = crate::fell::lines();
        if said.is_empty() {
            return Err("nothing has fallen since the power came on".into());
        }
        said.iter().for_each(|one| line(one));
        Ok(())
    }

    fn fall_now(&mut self, hard: bool) {
        crate::fell::on_purpose(hard);
    }

    fn loudness(&mut self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        crate::flash_owner::loud(None)?.iter().for_each(|one| line(one));
        Ok(())
    }

    fn set_loudness(&mut self, module: &str, level: &str) -> Result<String, String> {
        crate::flash_owner::loud(Some((module, level))).map(|said| said.join("; "))
    }

    fn words(&self) -> Vec<shell::Word> {
        crate::site::words()
    }

    fn word(&mut self, name: &str, words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
        crate::site::word(name, words, line)
    }

    fn takes(&self, name: &str) -> bool {
        crate::site::takes(name)
    }

    fn settings(&mut self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        crate::nvs_settings::lines(line)
    }

    fn set(&mut self, name: &str, value: &str) -> Result<String, String> {
        crate::nvs_settings::change(name, value)
    }
}
