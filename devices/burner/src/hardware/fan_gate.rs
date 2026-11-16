//! HERE: the zero-crossing interrupt counted and the MCPWM gate fired from it: the crossings and the gate's own pulses counted a second at a time, the mains read off a window of them, the gate built once the detector is
//! alive, and an angle in - two moments of a half cycle - pulses out.
//! NEVER HERE: percent of air, pascals, the regulator (crates/regulator), which angle a percent is (crates/dimmer's `order`, asked by the module), a word for a person. THE TWO HANDLERS IN IRAM ONLY WRITE DOWN - a count, a moment, a gap - and call
//! nothing of ours: nothing in them may reach flash, which Wi-Fi keeps busy. What the numbers mean - a half cycle off a count, the crossings a gap swallowed - is worked out by the round that counts the seconds, with crates/dimmer's pure arithmetic.
//! CHECKS: crates/dimmer/tests/dimmer.rs for that arithmetic, the ring of edges and what is made of them included (`Edge`, `EdgeReader`, `unread`); for the handlers, `tools/panic_path_in_iram.py ELF --also <edge> <fired> <gate_stop>` on the built image - each lies in IRAM and every call it makes lands in IRAM (names: `xtensa-esp32s3-elf-nm ELF | grep fan_gate`); the gate
//! itself only by the image on the board.
//!
//! The mains never reaches these pins. An optocoupler stands on each side - 4N25 on the way in, MOC3021 on the way out - and the owner is the one who puts 220 V on the module.

use std::ptr::{addr_of_mut, read_volatile, write_volatile};
use std::sync::Mutex;
use std::time::Duration;

use between::Lock;
use embassy_sync::signal::Signal;

use esp_idf_svc::sys::{self, EspError};
use module::Role;

use super::pin;

/// Edges counted since the board started, one counter per direction.
///
/// PLAIN WORDS, READ AND WRITTEN BY HAND, AND NOT ATOMICS. An `AtomicU32::fetch_add` on Xtensa is not an
/// instruction but a call into a library routine, and that routine lives in flash - which is exactly what an
/// interrupt in IRAM must never touch. Measured on this board, 20.09: with atomics here the counting ran three
/// or four seconds at a time and then `Guru Meditation (Cache error)`, over and over.
///
/// A u32 is written and read in one instruction on a 32-bit core, so the task never sees half a number. A u64
/// would be two, and that is the mistake our own triac.c made.
static mut RISING: u32 = 0;
static mut FALLING: u32 = 0;

/// Pulses the gate has actually been given, counted the same way and for the same reason: the stage is proved by
/// a hundred of these against a hundred crossings, and nothing else can say whether a half cycle was skipped.
///
/// The pin counts ITSELF. MCPWM's generator is told `io_loop_back`, which leaves the pad's input path on while
/// it drives the output (esp_driver_mcpwm/src/mcpwm_gen.c:86-88), so the pulse the peripheral makes comes back
/// in through the GPIO interrupt. No wire, no second pin, and what is counted is the pad - not our intent.
static mut FIRED: u32 = 0;

/// The MCPWM timers' own counts, one load each and no call: `MCPWM_TIMER0_STATUS_REG` is
/// `DR_REG_PWM0_BASE + 0x10` and timer 1's is at `+ 0x20`; the count is the low sixteen bits of either
/// (esp32s3/register/soc/mcpwm_reg.h:124,236).
///
/// TIMER 0 IS SYNCHRONISED from the crossing pin, so its count is microseconds since the last rising edge took
/// it to zero - that is what makes the gate fire where it should, and what gives the pulse width for free.
///
/// TIMER 1 IS SYNCHRONISED BY NOTHING and just runs, and it is the clock this handler never had. It wraps every
/// 65 536 µs, which is six half cycles, so a wrapping difference between two crossings is always the true gap.
const MCPWM_TIMER0_STATUS: u32 = sys::DR_REG_PWM0_BASE + 0x10;
const MCPWM_TIMER1_STATUS: u32 = sys::DR_REG_PWM0_BASE + 0x20;

/// THE EDGES, WRITTEN DOWN RAW AND NOT JUDGED: the handler puts each edge - which way it went, the free-running timer's count, the count of the timer the crossing resets - as one word at the next place of a ring and counts the places it
/// has written. The round that counts the seconds reads them and makes of them what is made (`dimmer::EdgeReader`): which edge is no transition, how many crossings the gaps between the rises swallowed, how wide the pulses are. A handler that
/// judged an edge would keep state and call code of ours, and code of ours may lie in flash, which Wi-Fi keeps busy. A power of two of places, so the place of a count is a mask and not a call; two hundred edges a second against places for
/// two and a half seconds of them, read every second.
const EDGE_PLACES: usize = 512;
static mut EDGES: [u32; EDGE_PLACES] = [0; EDGE_PLACES];
static mut EDGES_WRITTEN: u32 = 0;


/// The interrupt: one edge, one count. Which direction it was is read off the pin itself, so both are counted at
/// once and the choice of edge is made from numbers rather than from a guess.
///
/// IT LIVES IN IRAM, AND THAT IS NOT A PRECAUTION. Left in flash, it is unreachable whenever the flash is busy -
/// and Wi-Fi makes it busy constantly. Measured on this board, 20.09: the first image counted crossings from a
/// handler in flash, and the panic came three lines after `wifi: Set ps type`, a Double exception with the stack
/// already gone. `gpio_get_level` is safe to call from here: IDF puts it in IRAM too (esp_driver_gpio/linker.lf,
/// `gpio: gpio_get_level (noflash)`).
#[link_section = ".iram1.fan_edge"]
unsafe extern "C" fn edge(pin: *mut core::ffi::c_void) {
    // NOT ONE CALL LEAVES THIS FUNCTION, and that is the whole requirement. The level is read out of the GPIO
    // input register by hand - one load - instead of asking `gpio_get_level`, which IDF keeps in IRAM only when
    // `CONFIG_GPIO_CTRL_FUNC_IN_IRAM` is on, and in this build it is not. Disassembly of the image that failed
    // showed exactly that: the handler sat in IRAM at 0x40375438 and reached `gpio_get_level` at 0x4207338c,
    // in flash, through a `callx8`.
    //
    // The register is read rather than the build flag set, because the rule outlives the flag: an interrupt
    // handler must call nothing that could live in flash, and a flag someone turns off next year says nothing
    // about this function.
    let pin = pin as u32;
    let level = if pin < 32 {
        read_volatile(sys::GPIO_IN_REG as *const u32) >> pin & 1
    } else {
        read_volatile(sys::GPIO_IN1_REG as *const u32) >> (pin - 32) & 1
    };
    let counter = if level == 1 { addr_of_mut!(RISING) } else { addr_of_mut!(FALLING) };
    write_volatile(counter, read_volatile(counter).wrapping_add(1));

    // THE EDGE IS WRITTEN DOWN AS IT CAME AND NOTHING IS JUDGED HERE: whether it is a transition at all (a rise straight after a rise is the same one seen twice - the open collector of the 4N25 is pulled down by its transistor and
    // released upwards through a resistor, so the rising edge is the slow one, and a slow edge on a pin with no hysteresis is where the extra edges come from), how far it is from the crossing before it, how wide the pulse is - all of that
    // is the round's, from the two counts written here: the free-running timer's, which gives the gap between two rises, and the timer the crossing resets, which at a falling edge IS the pulse's width. Two loads of a register, a word
    // written, a count moved on - the word first and the count after, so the round that reads a count finds every place it counts written.
    let free = read_volatile(MCPWM_TIMER1_STATUS as *const u32) as u16;
    let pulse = read_volatile(MCPWM_TIMER0_STATUS as *const u32) as u16;
    let word = dimmer::Edge { rising: level == 1, free_us: free, pulse_us: pulse }.pack();
    let written = read_volatile(addr_of_mut!(EDGES_WRITTEN));
    write_volatile((addr_of_mut!(EDGES) as *mut u32).add(written as usize & (EDGE_PLACES - 1)), word);
    write_volatile(addr_of_mut!(EDGES_WRITTEN), written.wrapping_add(1));
}

/// The gate's pulse, counted where it leaves the board. Rising edges only, so one count is one pulse.
///
/// IN IRAM AND CALLING NOTHING, for the reason written over `edge`: this one fires a hundred times a second too,
/// and a handler in flash is unreachable whenever the flash is busy. It needs no level: the interrupt is set to
/// the rising edge alone, so being here IS the pulse.
#[link_section = ".iram1.fan_fired"]
unsafe extern "C" fn fired(_unused: *mut core::ffi::c_void) {
    let counter = addr_of_mut!(FIRED);
    write_volatile(counter, read_volatile(counter).wrapping_add(1));
}

/// THE GATE STOPS ITSELF WHEN THE FRONTS STOP. The gate's timer is reset by every front; with none it reaches its period (`PERIOD_TICKS`, `dimmer::STOP_AFTER_US`) and counts again, and the comparators, which sit inside the period, fire a
/// pulse on every lap with nothing to say where in the wave it is (24.10, 23:47:00: 49-52 pulses a second). The callback at the timer's PEAK is called only then - a front resets the count before the peak - and moves both comparators out of reach; while the
/// fronts are away it is called again at every lap, and a write of the regulator that landed beside it (`Gate::move_to` racing the interrupt) cannot leave the comparators in reach, because of the order of its writes (`dimmer::write_gate`).
///
/// IN IRAM, FOR THE SAME REASON AS `edge`, and with ONE call out: the IDF's `mcpwm_comparator_set_compare_value`, which lies in IRAM when `CONFIG_MCPWM_CTRL_FUNC_IN_IRAM=y` (components/esp_driver_mcpwm/linker.lf:4-6, set in sdkconfig.defaults). The
/// driver checks at registration that the callback is in IRAM (mcpwm_timer.c:193-206, `CONFIG_MCPWM_ISR_CACHE_SAFE=y`), and `tools/panic_path_in_iram.py` checks the built image. `mcpwm_generator_set_force_level` would be the stronger stop, but it is
/// not in IRAM (mcpwm_gen.c:124) and the registers are not written by hand here.
static mut STOPPED: u32 = 0;
/// Peaks of the timer since the start: how many times the callback has run. A plain word, read by hand, for the reason of `RISING`.
static mut STOPS: u32 = 0;
/// Whether the callback was registered: 1 when the gate can stop itself, 0 before the gate is built and when the driver refused.
static mut STOP_ARMED: u32 = 0;
/// The two comparators of the gate, for the callback, which is handed no context (the driver wants a context in internal RAM, and a static is).
static mut COMPARATORS: [sys::mcpwm_cmpr_handle_t; 2] = [core::ptr::null_mut(); 2];
/// What the regulator last asked of the comparators, kept while the gate is stopped so that it fires again at that and not at what the stop left (`Gate::move_to`, `Gate::resume`).
static mut WANTED_OPEN: u32 = DOWN_AT_US;
static mut WANTED_CLOSE: u32 = DOWN_AT_US;

#[link_section = ".iram1.fan_gate_stop"]
unsafe extern "C" fn gate_stop(_timer: sys::mcpwm_timer_handle_t, _event: *const sys::mcpwm_timer_event_data_t, _context: *mut core::ffi::c_void) -> bool {
    write_volatile(addr_of_mut!(STOPPED), 1);
    write_volatile(addr_of_mut!(STOPS), read_volatile(addr_of_mut!(STOPS)).wrapping_add(1));
    let comparators = read_volatile(addr_of_mut!(COMPARATORS));
    sys::mcpwm_comparator_set_compare_value(comparators[0], DOWN_AT_US);
    sys::mcpwm_comparator_set_compare_value(comparators[1], DOWN_AT_US);
    false
}

/// Whether the gate is stopped for want of fronts, and how many times it has been stopped (callbacks of the peak) since the start.
pub fn gate_stopped() -> bool {
    unsafe { read_volatile(addr_of_mut!(STOPPED)) != 0 }
}

pub fn gate_stops() -> u32 {
    unsafe { read_volatile(addr_of_mut!(STOPS)) }
}

pub fn gate_stop_armed() -> bool {
    unsafe { read_volatile(addr_of_mut!(STOP_ARMED)) != 0 }
}

/// The pad's own glitch filter, declared here because the generated bindings leave it out - the same reason
/// crates/firmware/src/log_tee.rs declares `vsnprintf` by hand.
///
/// COPIED FROM IDF v5.5.2, `components/esp_driver_gpio/include/driver/gpio_filter.h`:
///
/// ```c
/// typedef struct {
///     glitch_filter_clock_source_t clk_src; /*!< Clock source for the glitch filter */
///     gpio_num_t gpio_num;                  /*!< GPIO number */
/// } gpio_pin_glitch_filter_config_t;
/// ```
///
/// A HAND-WRITTEN COPY OF SOMEBODY ELSE'S STRUCT GOES STALE WITHOUT SAYING SO, and it does not go stale as a
/// compile error: the fields shift and the call quietly configures something else. This one already did - the
/// first version left `clk_src` out, so the pin number went where the clock belongs, the filter was never
/// switched on, and the silence was read as "this chip cannot filter it". Which is why the check below reads
/// the PAD, not the return code.
#[repr(C)]
struct PinGlitchFilter {
    clk_src: sys::soc_module_clk_t,
    gpio_num: i32,
}

const _: () = assert!(
    core::mem::size_of::<PinGlitchFilter>() == 8,
    "gpio_pin_glitch_filter_config_t of IDF v5.5.2 is two 32-bit fields: clk_src then gpio_num"
);

/// Bit 15 of `IO_MUX_GPIO<n>_REG` is the pad's filter (esp32s3/register/soc/io_mux_reg.h:67), and the registers
/// stand a word apart by pin number - which is how IDF's own `gpio_ll_pin_filter_enable` reaches them.
///
/// GPIO0's register is the IO_MUX base plus four (`PERIPHS_IO_MUX_GPIO0_U`, io_mux_reg.h:200), so pin `n` is at
/// `base + 4 + 4n`. Checked against the far end of the range rather than assumed: GPIO41 works out at
/// `base + 0xa8`, which is what `PERIPHS_IO_MUX_MTDI_U` is (io_mux_reg.h:395).
const IO_MUX_FILTER_EN: u32 = 1 << 15;

extern "C" {
    fn gpio_new_pin_glitch_filter(config: *const PinGlitchFilter, out: *mut *mut core::ffi::c_void) -> sys::esp_err_t;
    fn gpio_glitch_filter_enable(filter: *mut core::ffi::c_void) -> sys::esp_err_t;
}

/// Whether the pad of GPIO `zc` actually has its filter switched on - read off the pad itself.
fn filter_is_on(zc: i32) -> bool {
    let io_mux = sys::REG_IO_MUX_BASE + 4 + zc as u32 * 4;
    unsafe { read_volatile(io_mux as *const u32) & IO_MUX_FILTER_EN != 0 }
}

/// THE ROUND'S STACK, the zero-crossing round of `fan zc` (from the board of 04.10): the least free of the 8192 it had was 4624 bytes over five and a half hours of work, the page and the recording open; 6144 leaves 2576 - not under two kilobytes. The number is this round's own: the common one
/// (round::STACK) is not touched.
const STACK: usize = 6144;

/// Starts counting the crossings. Call once.
pub fn start() -> Result<(), EspError> {
    let zc = pin(env!("STRAGA_PIN_ZC"));
    let config = sys::gpio_config_t {
        pin_bit_mask: 1u64 << zc,
        mode: sys::gpio_mode_t_GPIO_MODE_INPUT,
        // The detector is an open collector and idles high, so nothing is pulled here: a pull of
        // our own would fight it and round off the very edges being measured.
        pull_up_en: sys::gpio_pullup_t_GPIO_PULLUP_DISABLE,
        pull_down_en: sys::gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
        // Both edges: which one the detector actually gives is found by measuring.
        intr_type: sys::gpio_int_type_t_GPIO_INTR_ANYEDGE,
        ..Default::default()
    };
    sys::esp!(unsafe { sys::gpio_config(&config) })?;

    // THE ONE FILTER THAT STANDS BEFORE THE PERIPHERAL. Rejecting a bounce in this file is too late: MCPWM takes
    // the pad straight from the matrix, so a spurious edge resets its timer before any code of ours hears about
    // it, and the gate fires at the wrong phase while our own counter shows a clean minute - it threw the
    // duplicate away. The pad's filter is upstream of both, so what it drops reaches neither.
    //
    // It is the FIXED one, because this chip has nothing else: the S3 carries SOC_GPIO_SUPPORT_PIN_GLITCH_FILTER
    // and neither the flexible filter (C5, C6, H2 have it; there is no gpio_glitch_filter_ll.h for esp32s3) nor
    // the pin hysteresis. Its window is two IO-MUX clocks, about 25 ns at 80 MHz, and whether that is wide
    // enough for THIS detector is not an argument to have - the doubled-edge counter answers it.
    let mut filter: *mut core::ffi::c_void = core::ptr::null_mut();
    let filter_config =
        PinGlitchFilter { clk_src: sys::soc_module_clk_t_SOC_MOD_CLK_APB, gpio_num: zc };
    if let Err(why) = sys::esp!(unsafe { gpio_new_pin_glitch_filter(&filter_config, &mut filter) })
        .and_then(|()| sys::esp!(unsafe { gpio_glitch_filter_enable(filter) }))
    {
        log::warn!("fan: the pad filter of GPIO{zc} refused: {why}");
    }
    // AND THE ANSWER IS TAKEN FROM THE PAD, not from what the call returned. A return code says the driver was
    // happy; the bit says the filter is on, and on THIS pin. The previous round reported a filter that was never
    // switched on, and then read the bounce that followed as a fact about the chip.
    log::info!("fan: pad filter of GPIO{zc} is {}", if filter_is_on(zc) { "ON" } else { "OFF" });
    // THE INTERRUPTS ARE INSTALLED FROM `fan zc`, ON CORE 1: GPIO's service is served on the core of the task that
    // installs it, and every GPIO handler added later - the gate's own pulses among them - shares it.
    // Installed from `main` it sat on core 0, beside Wi-Fi and lwip.
    firmware::task::round("fan zc", STACK, firmware::task::Core::Hardware, move || {
        match crossings_counted(zc) {
            Ok(()) => watch(zc),
            Err(why) => log::error!("fan: the zero crossings of GPIO{zc} are not counted: {why} - no gate either"),
        }
    })?;
    Ok(())
}

/// Installs GPIO's interrupt service on the core of the calling task and counts the crossings of ZC on it.
fn crossings_counted(zc: i32) -> Result<(), EspError> {
    // ESP_INTR_FLAG_IRAM says the handler may run while the flash is busy, and it is the flag the panic above was
    // missing. LEVEL1 is the lowest priority that still preempts the scheduler, which is all the counting needs;
    // the firing does not ride on an interrupt at all - it is MCPWM.
    let flags = (sys::ESP_INTR_FLAG_IRAM | sys::ESP_INTR_FLAG_LEVEL1) as i32;
    let installed = unsafe { sys::gpio_install_isr_service(flags) };
    match installed {
        sys::ESP_OK => log::info!("fan: GPIO interrupts served on core {}", unsafe { sys::xTaskGetCoreID(core::ptr::null_mut()) }),
        // already installed is not an error, but its core is whoever installed it first, not necessarily this one
        sys::ESP_ERR_INVALID_STATE => log::warn!("fan: GPIO interrupts were installed before - on another task's core"),
        _ => sys::esp!(installed)?,
    }
    sys::esp!(unsafe { sys::gpio_isr_handler_add(zc, Some(edge), zc as *mut core::ffi::c_void) })
}

/// Says every second how many edges of each direction came, and every minute whether any went missing.
///
/// A count that is not a hundred a second is the whole answer about this detector: too few and edges are being
/// lost, twice as many and both directions are being counted as crossings. Either way the firing that comes
/// later would sit at the wrong phase, and this is where it is caught - before a fan is on the other end.
fn watch(zc: i32) {
    log::info!("fan: counting zero crossings on GPIO{zc}, both edges");
    let dim = pin(env!("STRAGA_PIN_DIM"));
    let mut gate_built = false;
    // A copy of the gate, to let it fire again after it stopped for want of fronts; the regulator holds the gate itself.
    let mut gate: Option<Gate> = None;
    // The stops the gate had at the last second, and whether this round has said the current one: a stop is a line of the black box once, not once a lap of the timer.
    let (mut was_stops, mut told_stopped) = (0u32, false);
    let (mut was_rising, mut was_falling, mut was_pulses) = (0u32, 0u32, 0u32);
    let (mut minute_rising, mut minute_falling, mut minute_fired, mut seconds) = (0u32, 0u32, 0u32, 0u32);
    let (mut worst_rising, mut worst_falling) = (u32::MAX, u32::MAX);
    // The worst second of the gate is kept as a DIFFERENCE from the crossings of that same second, not as a count
    // of its own: a hundred pulses in a second that saw a hundred and one crossings is a half cycle skipped, and
    // by itself the number 100 says the opposite.
    let mut worst_missed = 0u32;
    // The ring of edges the handler writes: how many places the round has read, what it makes of them, and how many it lost to the ring being written over.
    let mut edges_read = 0u32;
    let mut edges = dimmer::EdgeReader::new();
    // Whether the pin gives a mains, second by second: the loss of its fronts and their return are lines of the black box, told once each.
    let mut fronts = dimmer::Fronts::new();
    let mut minute_edges_lost = 0u32;
    // A second measured to a deadline, not a second of sleeping plus whatever the loop costs. The drift of the
    // second shape is small and it lands squarely on the number this stage is judged by: it turned a hundred
    // crossings a second into "101", and a hundred and one is not a thing a 50 Hz mains can do.
    let mut due = between::clock::now() + Duration::from_secs(1);
    // The mains as it was last said out loud, so that saying it again is only for a number that moved far
    // enough to matter.
    let mut told_mains: Option<u32> = None;
    // The half cycle last measured, and Err until a whole window has gone by (empty words: not measured yet).
    let mut mains: Result<u32, String> = Err(String::new());
    // The window the mains is read from: when it opened, and how many crossings had been counted by then.
    let (mut window_opened, mut window_from) = (between::clock::now(), 0u32);
    // The minute's own clock, and what the extra-edge counters stood at when it opened.
    let mut window = between::clock::now();
    let (mut was_doubled_up, mut was_doubled_down) = (0u32, 0u32);
    loop {
        between::nap(due.since(between::clock::now()));
        due = due + Duration::from_secs(1);
        let (rising, falling) = counted();
        let so_far = fired_count();
        let (up, down) = (rising.wrapping_sub(was_rising), falling.wrapping_sub(was_falling));
        let pulses = so_far.wrapping_sub(was_pulses);
        was_rising = rising;
        was_falling = falling;
        was_pulses = so_far;
        // THE FRONTS LOST AND BACK, FOR THE JOURNAL: on 24.10 the pin gave none from 23:47:00 to 01:13:37 and the board kept no time of either. Two lines of the black box - the event, and what the chip holds for the pin
        // at that second, the word `zc` (zc_dump) - so that nobody has to be at the console in the night to read the level of the pin.
        if let Some(change) = fronts.second(up) {
            let what = match change {
                dimmer::Change::Lost { last_alive } => format!("fronts lost: {up}/s after {last_alive}/s"),
                dimmer::Change::Returned { silent_s } => format!("fronts back after {silent_s} s: {up}/s"),
            };
            log::warn!(target: firmware::BOX_FRONTS, "{what}");
            log::warn!(target: firmware::BOX_FRONTS, "{}", super::zc_dump::registers().line(rising, falling));
        }
        // THE GATE STOPPED FOR WANT OF FRONTS (`gate_stop`): said once, and let go once a second has gone by with no new stop and the fronts of a mains in it (`dimmer::may_resume`). Not by `Fronts`: a hole in the fronts shorter than the second the automaton judges
        // never makes a loss, and the gate must not stand because of it. From the stop to the firing again is one to two seconds: the second it came in is not counted, the next one is.
        let stops = gate_stops();
        let new_stops = stops.wrapping_sub(was_stops);
        was_stops = stops;
        let stopped = gate_stopped();
        if stopped && !told_stopped {
            told_stopped = true;
            log::warn!(target: firmware::BOX_FRONTS, "gate stopped: no front for {} µs; stop {stops}, {up}/s this second", dimmer::STOP_AFTER_US);
        }
        if dimmer::may_resume(stopped, new_stops, up) {
            if let Some(gate) = gate {
                match gate.resume() {
                    Ok(()) => {
                        told_stopped = false;
                        log::warn!(target: firmware::BOX_FRONTS, "gate fires again: {up}/s, {stops} stops so far");
                    }
                    Err(why) => log::error!("fan: the gate would not fire again: {why}"),
                }
            }
        }
        minute_rising += up;
        minute_falling += down;
        minute_fired += pulses;
        worst_rising = worst_rising.min(up);
        worst_falling = worst_falling.min(down);
        // The edges the handler wrote down this second, judged here: the nominal half cycle and not the measured one, the measured one being counted off this same pin, and a measure fed by what it judges can agree with anything.
        let (written, lost) = edges_unread(&mut edges_read);
        if lost > 0 {
            edges.lost();
            minute_edges_lost += lost;
        }
        written.for_each(|word| edges.feed(dimmer::Edge::unpack(word)));
        let asked = ASKED_NOW.latest();
        // A second the gate was never asked to fire in is not a second it missed anything: the difference is only
        // a fault where a percent was being held.
        if asked.is_some_and(|(percent, _)| percent > 0) {
            worst_missed = worst_missed.max(up.saturating_sub(pulses));
        }
        seconds += 1;
        // Wellbeing, once a second: a person turns it on with `log burner::fan debug` while watching the fan
        // and off again after. At info it was the flood the quiet is for.
        log::debug!("fan zc: {up} rising, {down} falling, {pulses} gate pulses this second");

        // The mains, once a whole window of counting has gone by, and again every window after: the grid drifts,
        // and a percent is kept as a share of the half cycle precisely so that it follows.
        let counting_for = window_opened.elapsed();
        if counting_for >= WINDOW {
            let read = dimmer::half_cycle_of(rising.wrapping_sub(window_from), counting_for.as_millis() as u32);
            match &read {
                // The mains becoming known, and then DRIFTING, is an event; measuring it again is not. The
                // measurement itself wanders by a few µs from window to window - 9991, 9992, 10001 on this
                // grid - so "it changed" fired nearly every window and read as news. What is news is a drift
                // the gate would be built differently for.
                Ok(half) if told_mains.is_none_or(|was| was.abs_diff(*half) >= TELLING_US) => {
                    log::info!("fan: mains {half} µs a half cycle, over {} s of counting", counting_for.as_secs());
                    told_mains = Some(*half);
                }
                Ok(half) => log::debug!("fan: mains {half} µs a half cycle, unchanged"),
                Err(why) => log::warn!("fan: {why}"),
            }
            mains = read;
            window_opened = between::clock::now();
            window_from = rising;
        }
        // THE SECOND GOES TO THE REGULATOR, which owns the fan's card and the gate: it reads the last one when it steps
        // and when it turns a percent into an angle (`half_to_fire`)
        SECOND.publish(Some(Second { crossings: up, pulses, ended: between::clock::now(), mains: mains.clone(), silent_s: fronts.silent_s(), alive_s: fronts.alive_s() }));

        // THE GATE IS BUILT FROM A MEASURED MAINS, NOT AT START. Until the board knows what its mains is there
        // is nothing to fire against, and MCPWM built over a detector that is silent would sit there looking
        // correct. It also keeps the failure readable from the boiler: a gate that will not build says so in the
        // log of a board that is up and on the network, instead of taking the start down where no wire is left.
        //
        // AND IT IS BUILT IN THE FIRST SECOND THE DETECTOR IS ALIVE, not after the ten of the window:
        // a board that came up over a fire blows at once, on the nominal half cycle until the window has read the
        // real one (dimmer::half_to_fire). The gate is synced from the crossing either way; what it is built against
        // is only the number in this line of the log.
        if !gate_built {
            let half = half_cycle_of(&mains).ok().or_else(|| dimmer::LIVE_CROSSINGS.contains(&up).then_some(dimmer::NOMINAL_HALF_US));
            if let Some(half) = half {
                gate_built = true;
                gate = raise_gate(zc, dim, half);
            }
        }

        if seconds == 60 {
            // THREE NUMBERS, AND THE THIRD IS THE ONLY INDEPENDENT ONE. Crossings and gate pulses both grow out
            // of the same pin: lose a crossing and neither sees it, so they agree while a half cycle goes
            // unfired. What the CLOCK owes is the one reference the detector does not feed.
            let took = window.elapsed();
            let owed = dimmer::owed_by_the_clock(took.as_millis() as u32);
            let (doubled_up, doubled_down) = edges.doubled();
            let pulse_seen = edges.pulses();
            log::debug!(
                "fan zc: over {} ms - {minute_rising} rising, {minute_falling} falling, \
                 {minute_fired} gate pulses, {owed} owed by the clock; \
                 extra edges {} up and {} down; pulse {} to {} µs",
                took.as_millis(),
                doubled_up.wrapping_sub(was_doubled_up),
                doubled_down.wrapping_sub(was_doubled_down),
                // NOTHING MEASURED AND MEASURED ZERO ARE NOT THE SAME READING, and printing both as "0" is how
                // a zero that meant "no falling edge was timed this minute" got read as "a pulse of no width",
                // and from there as bounce.
                pulse_seen.map_or("none".to_string(), |(narrowest, _)| narrowest.to_string()),
                pulse_seen.map_or(0, |(_, widest)| widest)
            );
            // What the two sides of this cannot show, said out loud so nobody reads the line as more than it is:
            // the edges of ONE pulse must come in pairs, so a difference between the two counts is ours and not
            // the grid's; a difference from what the clock owes is the grid's up to about ±12 a minute, which is
            // the band a real mains wanders in.
            // AND THE ONE LINE THAT CAN SHOW A SINGLE MISS. The sums above cannot: one lost crossing in six
            // thousand sits inside the band a real grid wanders in. A gap does not average - where 10 ms belongs
            // a lost half cycle puts 20, and no mains does that.
            let (missed, widest_gap) = edges.gaps();
            // The filter's state goes in EVERY minute, because the line that said it once at start was printed
            // before any console could be listening - and a fact nobody reads is a fact nobody has.
            // THE FAULT COMES OUT OF THE SUMMARY AND GOES OUT ON ITS OWN. The whole minute's accounting is
            // wellbeing and waits to be asked for, but a half cycle actually lost is a thing to go and fix,
            // and it has to be seen by somebody who never turned anything on.
            // ONE LINE A MINUTE, AND ONLY FOR A HALF CYCLE ACTUALLY LOST: a crossing missed by the gaps, or the ring of edges written over. A second one gate pulse short with nothing lost is the difference of two counts and
            // stood at `warn` every ~75 s - eight lines in ten minutes of fire - over the line that would mean something; it rides on this one when there is one, and waits in the debug summary below when there is not.
            let minute = dimmer::Minute { missed, worst_missed, edges_lost: minute_edges_lost };
            if minute.worth_a_line() {
                log::warn!("{}", minute.words());
            }
            log::debug!(
                "fan zc: pad filter {}; {missed} crossings MISSED by the gaps, widest gap {widest_gap} µs; \
                 edges of one pulse differ by {}, and the clock by {}; worst second {worst_rising} rising, \
                 short by {worst_missed} pulses, percent asked {asked:?}",
                if filter_is_on(zc) { "ON" } else { "OFF" },
                minute_rising.abs_diff(minute_falling),
                minute_rising.abs_diff(owed)
            );
            was_doubled_up = doubled_up;
            was_doubled_down = doubled_down;
            window = between::clock::now();
            seconds = 0;
            minute_rising = 0;
            minute_falling = 0;
            minute_fired = 0;
            worst_rising = u32::MAX;
            worst_falling = u32::MAX;
            worst_missed = 0;
            minute_edges_lost = 0;
        }
    }
}

/// Puts the gate into service: MCPWM built over the crossing pin, and the pad set to count the pulses it makes.
///
/// Nothing here is returned to the caller, because there is nothing a counting loop could do about it. What a
/// refusal is for is the log of a board that is up and on the network - the only way anything is read once this
/// board stands at the boiler.
fn raise_gate(zc: i32, dim: i32, half_cycle_us: u32) -> Option<Gate> {
    // The sync is on the RISING edge, and that is not a choice made here: the angles crates/dimmer carries were
    // measured from the rising edge of a detector like this one (the owner's own module,
    // `Pin.IRQ_RISING`). The true zero sits half a detector pulse away from that edge, and the angles carry that
    // offset inside them already, because they were taken the same way. They are checked on the live fan.
    log::info!("fan: building the gate against a {half_cycle_us} µs half cycle, synced on the rising edge");
    let gate = match build_gate(zc, dim) {
        Ok(gate) => gate,
        Err(why) => {
            log::error!("fan: no gate - MCPWM refused: {why}. The crossings are still counted, and `dim` will refuse.");
            return None;
        }
    };
    // the gate is the regulator's from here on: it alone moves the two compare values (`Fan::hold`); this round keeps a copy for one thing only, to let a gate that stopped for want of fronts fire again (`Gate::resume`)
    GATE_BUILT.signal(gate);

    // The pulse counts ITSELF: io_loop_back left the pad's input path on, so this interrupt sees what the
    // peripheral put out. The edge is set first and the handler after, because adding a handler enables the
    // interrupt on its way out (esp_driver_gpio/src/gpio.c, gpio_isr_handler_add).
    let counting = sys::esp!(unsafe { sys::gpio_set_intr_type(dim, sys::gpio_int_type_t_GPIO_INTR_POSEDGE) })
        .and_then(|()| sys::esp!(unsafe { sys::gpio_isr_handler_add(dim, Some(fired), core::ptr::null_mut()) }));
    match counting {
        Ok(()) => log::info!(
            "fan: gate on GPIO{dim}, timer synced from GPIO{zc}, counting its own pulses, down until a percent \
             is asked for"
        ),
        Err(why) => log::error!("fan: the gate is up but its pulses will not be counted: {why}"),
    }
    Some(gate)
}

/// How far the mains has to drift for the log to say so out loud, in microseconds of a half cycle. Half a
/// percent of a 10 ms half cycle: under that the number is the window's own jitter, and the gate is synced
/// from the crossing every time anyway.
const TELLING_US: u32 = 50;

/// How long the board counts crossings before it says what the mains is. Ten seconds, because that is what makes
/// the answer sharper than the thing it feeds: a thousand crossings, and one miscounted moves the half cycle by
/// 10 µs - a fifth of the gate pulse. A single second would move it by a hundred.
const WINDOW: Duration = Duration::from_secs(10);

/// A second as `fan zc` counted it: crossings and gate pulses, when it ended, and the mains as it stood then (Err with
/// no words - not measured yet).
///
/// **THE MOMENT IS PART OF THE READING**: a second that ended before the percent was last changed counted a different
/// percent, and answering with it says the gate is silent when it has only not been listened to yet.
#[derive(Clone)]
pub struct Second {
    pub crossings: u32,
    pub pulses: u32,
    pub ended: between::clock::Moment,
    pub mains: Result<u32, String>,
    /// How long the pin has given no fronts, in seconds, while it gives none (`dimmer::Fronts`, the one definition of a loss); None while it gives them.
    pub silent_s: Option<u32>,
    /// How many seconds in a row the pin has given a mains, while it does (`dimmer::Fronts::alive_s`); None while it gives none.
    pub alive_s: Option<u32>,
}

/// The last second `fan zc` counted: it alone writes it, the fan's module reads it.
static SECOND: Role<Second> = Role::new("fan_second");

/// The last second the gate counted; None before the first.
pub fn second() -> Option<Second> {
    SECOND.latest()
}

/// The percent the gate was last set to, and when: the fan's module alone writes it ([`asked`]), `fan zc` reads it to
/// judge a second short of pulses.
static ASKED_NOW: Role<(u32, between::clock::Moment)> = Role::new("fan_asked");

/// Says the percent the gate has just been set to, for `fan zc` to judge the seconds that follow by.
pub fn asked(percent: u32) -> between::clock::Moment {
    let at = between::clock::now();
    ASKED_NOW.publish(Some((percent, at)));
    at
}

/// The gate, built by `fan zc` once the detector is alive and handed over once: [`take_gate`].
static GATE_BUILT: Signal<Lock, Gate> = Signal::new();

/// The half cycle READ says, for whoever is about to turn a percent into an angle. Err while the first window is still
/// filling, and Err again whenever what was counted is not a mains - an angle taken from a number like that would sit
/// anywhere in the wave.
fn half_cycle_of(read: &Result<u32, String>) -> Result<u32, String> {
    match read {
        Ok(half) => Ok(*half),
        Err(why) if why.is_empty() => {
            Err(format!("the mains has not been measured yet: the board counts for {} s first", WINDOW.as_secs()))
        }
        Err(why) => Err(why.clone()),
    }
}

// ---- the gate: the pulse is made by the peripheral, not by an interrupt ----

/// The tick of the MCPWM timer: one microsecond, which is the unit every angle in crates/dimmer is written in.
const TICK_HZ: u32 = 1_000_000;

/// How far the timer is allowed to count before it wraps on its own: four nominal half cycles (`dimmer::STOP_AFTER_US`), longer than a front missing and than two in a row, because the zero crossing is what resets it. A wrap of its own means the crossings
/// stopped arriving, and then the gate stops (`gate_stop`): a period of 20 000 µs is exactly one front missing, and the lap that followed would fire a pulse at no phase.
const PERIOD_TICKS: u32 = dimmer::STOP_AFTER_US;

/// The gate, once it is built. Only the two compare values are ever changed afterwards.
#[derive(Clone, Copy)]
pub struct Gate {
    open_at: sys::mcpwm_cmpr_handle_t,
    close_at: sys::mcpwm_cmpr_handle_t,
    /// Kept for one reason: the gate is forced down while the two compare values are moved. See [`Gate::move_to`].
    generator: sys::mcpwm_gen_handle_t,
}

unsafe impl Send for Gate {}

/// Where both compare values stand while the gate is held down: the last tick the timer counts (`dimmer::DOWN_AT_US`, which says why that tick and not the one past it). The tick is reached; what keeps the output low there is the generator's priority of the event of
/// comparator B (clear) over comparator A (set) when both fall on one tick, ESP32-S3 TRM v1.8 table 36.3-3 (p. 1341), `open_at` being the first comparator built and so A (mcpwm_cmpr.c:20-35 gives the first free place). The board says the same: 342 514 seconds of
/// 21-25.10 with the gate off for four seconds running, that is both comparators here, counted no pulse on the pad.
pub const DOWN_AT_US: u32 = dimmer::DOWN_AT_US;
const _: () = assert!(DOWN_AT_US == PERIOD_TICKS - 1, "the stopped value is the last tick of the timer's period");

/// One writer of the hardware among the tasks: the pair of comparators is written whole or not at all. The interrupt does not take this lock and is not kept out by it - that is `dimmer::write_gate`'s order; this keeps TWO TASKS (the regulator's `holding`, through
/// `Fan::hold`, and `fan zc`, through `Gate::resume`, both on the hardware core and able to preempt each other) from leaving one comparator of each.
static GATE_WRITE: Mutex<()> = Mutex::new(());

/// The gate's hardware for `dimmer::write_gate`: each method one call of the driver, or one load.
struct Wires<'a>(&'a Gate);

impl dimmer::GateHardware for Wires<'_> {
    // THE FORCED LEVEL IS CALLED FROM A TASK ONLY: `mcpwm_generator_set_force_level` is not in IRAM (mcpwm_gen.c:124, linker.lf lists two functions), so the interrupt does not hold the output - it moves the comparators.
    fn hold_low(&mut self) -> Result<(), String> {
        sys::esp!(unsafe { sys::mcpwm_generator_set_force_level(self.0.generator, 0, true) }).map_err(|why| format!("the gate would not be forced down: {why}"))
    }
    fn set_open(&mut self, at_us: u32) -> Result<(), String> {
        sys::esp!(unsafe { sys::mcpwm_comparator_set_compare_value(self.0.open_at, at_us) }).map_err(|why| format!("the comparator refused {at_us} µs: {why}"))
    }
    fn set_close(&mut self, at_us: u32) -> Result<(), String> {
        sys::esp!(unsafe { sys::mcpwm_comparator_set_compare_value(self.0.close_at, at_us) }).map_err(|why| format!("the comparator refused {at_us} µs: {why}"))
    }
    fn stopped(&self) -> bool {
        gate_stopped()
    }
    fn release(&mut self) -> Result<(), String> {
        sys::esp!(unsafe { sys::mcpwm_generator_set_force_level(self.0.generator, -1, true) }).map_err(|why| format!("the gate would not be forced free: {why}"))
    }
}

impl Gate {
    /// Moves the moment the gate rises at and the moment it falls, in microseconds after the crossing.
    ///
    /// THE GATE IS HELD DOWN WHILE THE TWO NUMBERS MOVE, and this is not tidiness. The counter does not stop for us: caught between the old raise and the old lower, a gate whose new lower has already gone by stays
    /// up through the crossing and opens the triac on the next half cycle at full power - the defect of our own triac.c, arrived at by a change of percent instead of by a bad angle. The cost is one half cycle without
    /// a pulse, once, at the moment a person asks for a different percent. What is asked is kept; a gate stopped for want of fronts is not moved, and `resume` brings it to what was asked last.
    pub fn move_to(&self, open_at_us: u32, close_at_us: u32) -> Result<(), String> {
        let _writing = GATE_WRITE.lock().unwrap_or_else(|held| held.into_inner());
        unsafe {
            write_volatile(addr_of_mut!(WANTED_OPEN), open_at_us);
            write_volatile(addr_of_mut!(WANTED_CLOSE), close_at_us);
        }
        dimmer::write_gate(&mut Wires(self), open_at_us, close_at_us)
    }

    /// Lets a stopped gate fire again, at what was last asked of it. The stop is lifted FIRST and the pair written after, in the order of `dimmer::write_gate`: a peak of the timer that comes at any point of it (it comes at every lap while the fronts are away) puts
    /// the comparators out of reach again and leaves the output held low, so what the race costs is a gate that stays stopped for one more second of `watch`, never a gate that fires at no phase.
    pub fn resume(&self) -> Result<(), String> {
        let _writing = GATE_WRITE.lock().unwrap_or_else(|held| held.into_inner());
        unsafe { write_volatile(addr_of_mut!(STOPPED), 0) };
        let (open_at, close_at) = unsafe { (read_volatile(addr_of_mut!(WANTED_OPEN)), read_volatile(addr_of_mut!(WANTED_CLOSE))) };
        dimmer::write_gate(&mut Wires(self), open_at, close_at)
    }
}

/// The gate, once `fan zc` has built it: taken once, by the module that moves it.
pub fn take_gate() -> Option<Gate> {
    GATE_BUILT.try_take()
}

/// Builds the gate: a timer reset by the zero-crossing pin, and two comparators that raise and lower the triac's
/// gate by themselves.
///
/// NOTHING HERE RUNS AT FIRING TIME. The processor may be anywhere - in Wi-Fi, in flash, in another task - and
/// the pulse still lands where the counter says, because the counter raises the pin. That is the whole reason
/// this is MCPWM and not a timer interrupt (Espressif's own solution; ESPHome's jitters 4.7-20 µs with Wi-Fi on).
fn build_gate(zc: i32, dim: i32) -> Result<Gate, EspError> {
    let timer_config = sys::mcpwm_timer_config_t {
        group_id: 0,
        clk_src: sys::soc_periph_mcpwm_timer_clk_src_t_MCPWM_TIMER_CLK_SRC_DEFAULT,
        resolution_hz: TICK_HZ,
        count_mode: sys::mcpwm_timer_count_mode_t_MCPWM_TIMER_COUNT_MODE_UP,
        period_ticks: PERIOD_TICKS,
        ..Default::default()
    };
    let mut timer: sys::mcpwm_timer_handle_t = core::ptr::null_mut();
    sys::esp!(unsafe { sys::mcpwm_new_timer(&timer_config, &mut timer) })?;

    // AND A SECOND TIMER THAT NOTHING SYNCHRONISES - the clock the interrupt handler never had. It just runs at
    // a microsecond a tick and wraps at 65 536, which is six half cycles, so the difference between two readings
    // is always the true gap between two crossings. One load of a register to read, which is all a handler may
    // do; every other clock on this chip is a call, and a call from here may land in flash.
    let free_config = sys::mcpwm_timer_config_t { period_ticks: 0xFFFF, ..timer_config };
    let mut free: sys::mcpwm_timer_handle_t = core::ptr::null_mut();
    sys::esp!(unsafe { sys::mcpwm_new_timer(&free_config, &mut free) })?;
    sys::esp!(unsafe { sys::mcpwm_timer_enable(free) })?;
    sys::esp!(unsafe {
        sys::mcpwm_timer_start_stop(free, sys::mcpwm_timer_start_stop_cmd_t_MCPWM_TIMER_START_NO_STOP)
    })?;

    // The zero crossing drives the peripheral directly. Which edge is the one we count is settled by measuring; the
    // same edge resets the timer here, so the count of one is the phase of the other.
    let mut sync_config = sys::mcpwm_gpio_sync_src_config_t { group_id: 0, gpio_num: zc, ..Default::default() };
    sync_config.flags.set_active_neg(0); // a rising edge triggers the sync, as in the mcpwm_sync example
    sync_config.flags.set_io_loop_back(0);
    let mut sync: sys::mcpwm_sync_handle_t = core::ptr::null_mut();
    sys::esp!(unsafe { sys::mcpwm_new_gpio_sync_src(&sync_config, &mut sync) })?;
    let phase = sys::mcpwm_timer_sync_phase_config_t {
        sync_src: sync,
        count_value: 0,
        direction: sys::mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_UP,
    };
    sys::esp!(unsafe { sys::mcpwm_timer_set_phase_on_sync(timer, &phase) })?;

    let operator_config = sys::mcpwm_operator_config_t { group_id: 0, ..Default::default() };
    let mut operator_: sys::mcpwm_oper_handle_t = core::ptr::null_mut();
    sys::esp!(unsafe { sys::mcpwm_new_operator(&operator_config, &mut operator_) })?;
    sys::esp!(unsafe { sys::mcpwm_operator_connect_timer(operator_, timer) })?;

    let comparator_config = sys::mcpwm_comparator_config_t::default();
    let mut open_at: sys::mcpwm_cmpr_handle_t = core::ptr::null_mut();
    let mut close_at: sys::mcpwm_cmpr_handle_t = core::ptr::null_mut();
    sys::esp!(unsafe { sys::mcpwm_new_comparator(operator_, &comparator_config, &mut open_at) })?;
    sys::esp!(unsafe { sys::mcpwm_new_comparator(operator_, &comparator_config, &mut close_at) })?;

    let mut generator_config = sys::mcpwm_generator_config_t { gen_gpio_num: dim, ..Default::default() };
    // io_loop_back leaves the pad's input path on while the peripheral drives its output
    // (esp_driver_mcpwm/src/mcpwm_gen.c:86-88, `gpio_ll_input_enable`). That is what lets the board count its own
    // gate pulses through a GPIO interrupt - the proof this stage is closed by - with no wire and no second pin.
    // IDF marks the flag deprecated for v6.0; when that comes the same thing is done by enabling the input by hand.
    generator_config.flags.set_io_loop_back(1);
    let mut generator: sys::mcpwm_gen_handle_t = core::ptr::null_mut();
    sys::esp!(unsafe { sys::mcpwm_new_generator(operator_, &generator_config, &mut generator) })?;

    // One comparator raises the gate, the other lowers it. Both are compared against the same counter, so the
    // pulse cannot outlive the half cycle: crates/dimmer keeps its end inside it by arithmetic.
    let raise = sys::mcpwm_gen_compare_event_action_t {
        direction: sys::mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_UP,
        comparator: open_at,
        action: sys::mcpwm_generator_action_t_MCPWM_GEN_ACTION_HIGH,
    };
    let lower = sys::mcpwm_gen_compare_event_action_t {
        direction: sys::mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_UP,
        comparator: close_at,
        action: sys::mcpwm_generator_action_t_MCPWM_GEN_ACTION_LOW,
    };
    sys::esp!(unsafe { sys::mcpwm_generator_set_action_on_compare_event(generator, raise) })?;
    sys::esp!(unsafe { sys::mcpwm_generator_set_action_on_compare_event(generator, lower) })?;

    // AND THE GATE GOES DOWN AT EVERY COUNT OF ZERO, whatever else has been asked. It is the one action that
    // cannot be reasoned away: whether the counter reached zero by wrapping or was put there by the crossing, the
    // pin is low from that tick on, so no pulse can be carried across a crossing into the next half cycle. That
    // carry is exactly the defect of our own triac.c, and the stage forbids it by name.
    let down_at_zero = sys::mcpwm_gen_timer_event_action_t {
        direction: sys::mcpwm_timer_direction_t_MCPWM_TIMER_DIRECTION_UP,
        event: sys::mcpwm_timer_event_t_MCPWM_TIMER_EVENT_EMPTY,
        action: sys::mcpwm_generator_action_t_MCPWM_GEN_ACTION_LOW,
    };
    sys::esp!(unsafe { sys::mcpwm_generator_set_action_on_timer_event(generator, down_at_zero) })?;

    // Off until a percent is asked for: both comparators past anything the timer will reach before the next
    // crossing resets it, so the gate is never raised.
    sys::esp!(unsafe { sys::mcpwm_comparator_set_compare_value(open_at, PERIOD_TICKS - 1) })?;
    sys::esp!(unsafe { sys::mcpwm_comparator_set_compare_value(close_at, PERIOD_TICKS - 1) })?;

    // THE STOP: the callback at the timer's peak, registered BEFORE the timer is enabled (mcpwm_timer.h:133). The comparators are handed over through a static, which the driver's check of the context (internal RAM) is satisfied by.
    unsafe { write_volatile(addr_of_mut!(COMPARATORS), [open_at, close_at]) };
    let callbacks = sys::mcpwm_timer_event_callbacks_t { on_full: Some(gate_stop), on_empty: None, on_stop: None };
    // A REFUSAL HERE DOES NOT TAKE THE FAN DOWN: the gate without the stop still fires in step with the fronts, and a fan that does not turn is worse than a fan that fires blind when the fronts stop. It is said at `error`, and the word `zc` says
    // `armed=0` until someone has seen it.
    match sys::esp!(unsafe { sys::mcpwm_timer_register_event_callbacks(timer, &callbacks, core::ptr::null_mut()) }) {
        Ok(()) => unsafe { write_volatile(addr_of_mut!(STOP_ARMED), 1) },
        Err(why) => log::error!("fan: the gate cannot stop itself when the fronts stop: {why}. It fires as it did before, blind to a pin that goes silent."),
    }

    sys::esp!(unsafe { sys::mcpwm_timer_enable(timer) })?;
    sys::esp!(unsafe { sys::mcpwm_timer_start_stop(timer, sys::mcpwm_timer_start_stop_cmd_t_MCPWM_TIMER_START_NO_STOP) })?;
    Ok(Gate { open_at, close_at, generator })
}

/// The edges counted so far: rising, falling. Read the same way the interrupt writes
/// them, one word at a time.
fn counted() -> (u32, u32) {
    unsafe { (read_volatile(addr_of_mut!(RISING)), read_volatile(addr_of_mut!(FALLING))) }
}

/// The gate pulses counted so far, read the same way.
fn fired_count() -> u32 {
    unsafe { read_volatile(addr_of_mut!(FIRED)) }
}

/// The edges the handler has written since TAIL was last read up to, oldest first, and how many were written over before they were read; TAIL moved up to what was read.
fn edges_unread(tail: &mut u32) -> (impl Iterator<Item = u32>, u32) {
    let head = unsafe { read_volatile(addr_of_mut!(EDGES_WRITTEN)) };
    let found = dimmer::unread(head, *tail, EDGE_PLACES);
    *tail = head;
    let places = (0..found.count).map(move |k| {
        let place = found.first.wrapping_add(k as u32) as usize & (EDGE_PLACES - 1);
        unsafe { read_volatile((addr_of_mut!(EDGES) as *const u32).add(place)) }
    });
    (places, found.lost)
}

/// The rises and falls counted since the start, for the console word `zc`: the same two counts `watch` takes every second, read once.
pub fn edges_counted() -> (u32, u32) {
    counted()
}
