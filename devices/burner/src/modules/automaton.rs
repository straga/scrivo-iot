//! HERE: the burner's cycle: the readings of the modules by role, one step of crates/automaton a second on them, its commands to the relays and the fan by role, a person's command to an organ judged by the state it is in, the stage kept
//! on flash and taken back after a restart, the card it publishes by the entities of api/src/automaton.rs, its place in the module list.
//! NEVER HERE: a pin, a threshold or a number of the fire (crates/settings, `fire` and the card's settings), a state being decided (crates/automaton), a word of the page (page/menu.json), a code of the stream (api/src/automaton.rs). What this
//! file owns is the ORDER: read the roles, step, hand the commands over, keep, publish.
//! CHECKS: devices/burner/api/tests/automaton_values.rs, crates/settings/tests/automaton_tuning.rs, crates/automaton/tests; on the stand tools/module_compare.py --modules automaton against the first burner.
//!
//! **IT SWITCHES THE ORGANS FOR REAL, ALWAYS**, on the card's numbers and on the flame its sensor sees: there is no quick run and no flame put before it from the console.
//!
//! **THE LIGHTING POURS ONE DOSE, RISING FEEDS NOTHING, THE WORK FEEDS BY ITS OWN PAIR**: the dose fills the pot above the plug, the fire eats it through rising, and the work turns the auger 2 s of
//! every 22. Lighting and work each hold a draught of their own - 51 Pa both, as the factory sets them, apart so lighting can be turned down on its own. Those, how long rising lasts, how long a lost flame is waited for, the window
//! of the flue, the purge, the igniter, the wait and the attempts are settings of this card; every other number the automaton needs is fixed in the image (crates/settings, `fire`).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;

use ::automaton::{hand_is_free, Asked, Automaton, Burning, Clock, Doing, Driving, Feeding, Hands, Order, Seen, Silence, Sound, State, Tuning};
use ::flame::Flame;
use ::thermostat::Press;
use burner_api::automaton::{self as api, Now, Silent, FEED, SEMI};
use between::clock::Moment;
use between::Desk;
use firmware::nvs_settings;
use module::card::{Fill, Published, Reads};
use settings::Setting;

use crate::modules::demand::{self, Demanding};
use crate::modules::thermostat::{self, heard, Heat};
use crate::roles;

/// The automaton and its hands on the organs: **THE ONE PLACE A COMMAND IS JUDGED AND APPLIED**. It lives in the automaton's own task and nowhere else: its seconds, a press of its
/// buttons and a person's hand on an organ each come to that task in turn - the hand by the desk ([`DESK`]) -, so each is judged by the state the automaton is in when it is applied, and nothing comes between the verdict and the organ.
struct Core {
    /// The automaton with its hands on the organs: where it takes them back from a person is its own rule (`automaton::takes_back`), in Auto and in Semi.
    driving: Driving<Organs>,
    /// What was seen last, and the numbers of the last second: what a press is applied on.
    seen: Seen,
    sound: Sound,
    /// The thermostat, whose order the automaton carries out (`roles::HEAT`): what a person pressed and the numbers it decides by.
    heat: Heat,
    /// The players of the demand, which the thermostat reads by its role (`roles::DEMAND`) and does not know them by.
    demanding: Demanding,
    /// The order of the last second or the last press: what a ban of the feed is applied on, a second of no length.
    order: Order,
    /// When a person last started the burner, for the journal's clock.
    started: Option<Moment>,
}

impl Core {
    /// A press, judged and applied now; SHOWN says at once what it did.
    fn press(&mut self, press: Press, shown: &mut Option<Shown>) -> Result<(), String> {
        let was = self.driving.automaton.state();
        // a manual cycle does not go on over a fault, which waits for a person: told before the press is judged
        self.heat.faulted(matches!(was, State::Fault(_)));
        // the thermostat judges the press, counts it and publishes its order of this moment; the automaton takes it in the same breath. **A LIGHT THE NOMINAL REFUSES IS NOT COUNTED AND THE FIRE IS NOT ASKED**: the refusal goes back to the person as data (`Refusal::wire`) and the page says it in words
        self.order = match self.heat.press(press, between::clock::now(), was == State::Stop, self.seen.flame == Flame::Steady) {
            Ok(order) => order,
            Err(refusal) => return Err(refusal.wire()),
        };
        let doing = self.driving.heat(&self.order, self.seen, &self.sound);
        if press != Press::Stop && hand_is_free(was) && !hand_is_free(doing.state) {
            self.started = Some(between::clock::now());
        }
        // THE CARD SAYS WHAT THE PRESS DID AT ONCE: the event the press announces must not carry the state before it
        if let Some(shown) = shown.as_mut() {
            if shown.doing.map(|d| d.state) != Some(doing.state) {
                shown.since = between::clock::now();
            }
            shown.doing = Some(doing);
            shown.clocks = self.driving.automaton.clocks(&self.sound);
        }
        Ok(())
    }

    /// The feed allowed or banned, applied now - a ban in the middle of a dose stops the auger in this call.
    fn feed(&mut self, allowed: bool, shown: &mut Option<Shown>) {
        // an auger a person turned on is off with the ban, not only the automaton's (`Driving::feed`)
        let doing = self.driving.feed(allowed, &self.order, self.seen, &self.sound);
        FEED_ALLOWED.store(self.driving.automaton.feed_allowed(), Ordering::Relaxed);
        if let Some(shown) = shown.as_mut() {
            shown.doing = Some(doing);
            shown.feed_allowed = self.driving.automaton.feed_allowed();
        }
    }

    /// Auto or Semi, switched now and written down, so the board comes up in it after a restart.
    fn semi(&mut self, semi: bool, shown: &mut Option<Shown>) {
        self.driving.set_semi(semi);
        SEMI_ON.store(semi, Ordering::Relaxed);
        if let Err(why) = nvs_settings::remember(::automaton::MODE_CARD, ::automaton::mode_word(semi)) {
            log::error!("automaton: the mode was not written down: {why}");
        }
        if let Some(shown) = shown.as_mut() {
            shown.semi = semi;
        }
    }
}

/// How often it steps: the automaton's clock is seconds, and so are the sensors' means it reads.
const EVERY: Duration = Duration::from_secs(1);
/// How long a board that has just come up waits for the flame and the flue to speak before it decides anything: `Burning::resumed` judges by them, and a sensor that has not read yet is not a dark pot.
const FIRST_READINGS: Duration = Duration::from_secs(15);

/// Every number of the fire as it stands now: the card's settings, all from one set, and the ones the image fixes (crates/settings, `automaton_tuning`).
fn tuning() -> Tuning {
    let now = nvs_settings::now();
    // a value the card could not have taken reads as none, and the automaton's door refuses it by name
    burner_settings::automaton_tuning(&|s: &Setting| now.value(s))
}

/// What the board sees this second, in the automaton's terms, BY ROLE (Concept_Burner_Modules): the automaton asks for the flame, the water of the boiler, the flue gas and the draught, and which module gives them is the modules' business.
/// **NONE IS NEVER A ZERO**: a flue that said nothing is not a flue at 0 °C, and a draught under its own zero is no number - the regulator reads it the same way.
fn seen(eyes: &mut Eyes) -> Seen {
    // one moment for the whole second: a reading and its silence judged at two moments could disagree at the line
    let at = between::clock::now();
    // read once: the number and the word "under my zero" are the two halves of one reading
    let draught = eyes.draught.now_at(at);
    Seen {
        // before its first window, and if its round never started: dark, which is the side a burner that cannot see its flame fails to - it stops feeding (crates/automaton, `Seen::flame`)
        flame: eyes.flame.now_at(at).unwrap_or(Flame::Out),
        boiler_c: eyes.boiler.now_at(at),
        flue_c: eyes.flue.now_at(at),
        draught_pa: match draught {
            Some(::draught::Reading::Is(pa)) => Some(pa),
            Some(::draught::Reading::Over) => Some(::draught::FULL_PA as f32),
            _ => None,
        },
        // **A SENSOR THAT SAYS "UNDER MY ZERO" IS THERE** (crates/automaton, `Seen::draught_low`): it is counted for `low_pressure_s` and not as a sensor that gives no number
        draught_low: matches!(draught, Some(::draught::Reading::Under)),
        gate: crate::modules::fan::gate_percent(),
        // why a reading above is None, when its round has stopped publishing
        silence: silence_of(&quiets(at)),
    }
}

/// The automaton's own places on the roles it sees by, taken once when its task starts.
struct Eyes {
    flame: module::Given<'static, Flame>,
    boiler: module::Given<'static, f32>,
    flue: module::Given<'static, f32>,
    draught: module::Given<'static, ::draught::Reading>,
    /// What the thermostat orders (`roles::HEAT`): the automaton holds the role, and not the module that plays it.
    heat: module::Given<'static, Order>,
}

impl Eyes {
    fn open() -> Eyes {
        let place = "a place on a role for the automaton";
        Eyes {
            flame: roles::FLAME.receiver().expect(place),
            boiler: roles::BOILER.receiver().expect(place),
            flue: roles::FLUE_GAS.receiver().expect(place),
            draught: roles::DRAUGHT.receiver().expect(place),
            heat: roles::HEAT.receiver().expect(place),
        }
    }
}

/// What the card shows: the last second, and how it came about.
struct Shown {
    doing: Option<Doing>,
    /// When the state was entered, by the board's clock.
    since: Moment,
    /// Why the numbers on the card could not be taken, as the door says it; None - they were.
    refused: Option<::automaton::Refused>,
    /// What ends in this state and in how many seconds (`Burning::clocks`).
    clocks: Vec<Clock>,
    /// The stage it was in when it failed, while it burns off after the failure and stands in the fault; None otherwise, and after a restart, which does not know it.
    broke_at: Option<&'static str>,
    /// Whether the auger may turn, as the card shows it.
    feed_allowed: bool,
    /// Whether the automaton runs in Semi, as the card shows it.
    semi: bool,
}

/// Whether the auger may turn, for the console's answer after a command (`is_on`). Written by the automaton's task alone.
static FEED_ALLOWED: AtomicBool = AtomicBool::new(true);

/// Whether the automaton runs in Semi, for the console's answer after a command (`is_on`). Written by the automaton's task alone.
static SEMI_ON: AtomicBool = AtomicBool::new(false);

/// What the automaton's task is asked by the others.
enum Request {
    /// Start, Stop or Light pressed.
    Press(Press),
    /// The feed allowed, banned, or switched to the other side of what it is (None).
    Feed(Option<bool>),
    /// Semi on, off, or switched to the other side of what it is (None).
    Semi(Option<bool>),
    /// A person's command to an organ the automaton keeps (`through`): judged, and carried out over the modules.
    Through(&'static module::Modules, String, module::Order),
}

/// Where the others ask the automaton (between::Desk): a person, from the page or a console, one at a time besides.
static DESK: Desk<Request, Result<(), String>, 4, 3> = Desk::new();

/// How long a person waits for the automaton: three seconds, which is not how long a step of the automaton can take. In a turn of its loop it can wait one after another for the relays' task (1 s each, `relay::board::WITHIN`: the auger, the igniter, the pump, the igniter's guard), for the fan (2 s, `modules/fan.rs`) and for the flash (3 s, `flash_owner::WITHIN`) - nine seconds at the worst, which is what `relay::HORIZON_MS` is taken from.
const WITHIN: Duration = Duration::from_secs(3);

/// A press of Start, Stop or Light, asked of the automaton's task and answered by it: the thermostat counts the press, and the fire takes the order that carries it in the same breath (the card of the thermostat has the buttons).
pub fn press(press: Press) -> Result<(), String> {
    ask(Request::Press(press))
}

/// What the automaton's task holds: the automaton once it has come up, what its card shows, and a press that came before it did.
// the automaton's own state, a variable of its task: it stays small enough for the task's stack
const _: () = assert!(core::mem::size_of::<Mind>() <= firmware::task::ON_STACK_AT_MOST);

struct Mind {
    core: Option<Core>,
    shown: Option<Shown>,
    /// Start, stop or light pressed before the automaton came up, applied by its first second.
    early: Option<Press>,
}

impl Mind {
    /// What the others asked, carried out in the automaton's task.
    fn answer(&mut self, asked: Request) -> Result<(), String> {
        match asked {
            Request::Press(press) => {
                match self.core.as_mut() {
                    Some(core) => core.press(press, &mut self.shown)?,
                    // before its first second: start and stop wait for it
                    None => self.early = Some(press),
                }
                publish(&self.shown);
                log::info!("automaton: a person pressed {press:?}");
                Ok(())
            }
            Request::Feed(asked) => {
                let core = self.core.as_mut().ok_or("the automaton has not taken its first second yet - wait a few seconds")?;
                let allowed = asked.unwrap_or(!core.driving.automaton.feed_allowed());
                core.feed(allowed, &mut self.shown);
                publish(&self.shown);
                log::info!("automaton: a person {} the feed", if allowed { "allowed" } else { "banned" });
                Ok(())
            }
            Request::Semi(asked) => {
                let core = self.core.as_mut().ok_or("the automaton has not taken its first second yet - wait a few seconds")?;
                let semi = asked.unwrap_or(!core.driving.automaton.semi());
                core.semi(semi, &mut self.shown);
                publish(&self.shown);
                log::info!("automaton: a person switched to {}", if semi { "Semi" } else { "Auto" });
                Ok(())
            }
            Request::Through(modules, name, order) => {
                if let Some(core) = self.core.as_ref() {
                    core.driving.automaton.by_hand(&name)?;
                }
                // THE PLUG ONLY WITH AIR, by hand too: on 26.09 a plug switched on with the fan standing burnt through. A board that has not seen its first second has no draught to show.
                if name == "igniter" {
                    let closing = match order {
                        module::Order::Switch(on) => on,
                        module::Order::Toggle => !crate::modules::relays::igniter_closed(),
                        module::Order::Press | module::Order::Hold(_) => false,
                    };
                    let draught = self.core.as_ref().and_then(|core| core.seen.draught_pa);
                    ::automaton::plug_by_hand(closing, crate::modules::fan::gate_percent(), draught)?;
                }
                module::carry(modules, &name, order)
            }
        }
    }
}

/// Asks the automaton's task, and waits for it.
fn ask(asked: Request) -> Result<(), String> {
    DESK.ask(asked, WITHIN).unwrap_or_else(|refused| Err(refused.to_string()))
}

/// The organs as the automaton reaches them: BY ROLE (roles.rs), which is its own road and not a person's. What it passes on only when it changes is `OnChange`'s, around this.
struct Organs;

impl Organs {
    fn relay(role: &module::Role<bool>, on: bool) {
        if let Err(why) = firmware::site::command(role, on) {
            log::error!("automaton: the {} was not switched {}: {why}", role.name, if on { "on" } else { "off" });
        }
    }
}

/// Seconds as the relays' owner counts: milliseconds, never none.
fn millis(seconds: f32) -> u32 {
    ((seconds * 1000.0).round() as u32).max(1)
}

/// The automaton's words in seconds as the relays' owner counts them: milliseconds, never none.
fn told_of(feeding: Feeding) -> relay::Feeding {
    match feeding {
        Feeding::Stop => relay::Feeding::Stop,
        Feeding::Dose { left_s } => relay::Feeding::Dose { left_ms: millis(left_s) },
        Feeding::Rhythm { into_s, on_s, off_s } => relay::Feeding::Rhythm { into_ms: (into_s * 1000.0).round() as u32, on_ms: millis(on_s), off_ms: millis(off_s) },
    }
}

impl Organs {
    fn tell(told: relay::Told) {
        if let Err(why) = firmware::site::command(&roles::AUGER, told) {
            log::error!("automaton: the auger was not told {told:?}: {why}");
        }
    }
}

impl Hands for Organs {
    fn auger(&mut self, feeding: Feeding) {
        Organs::tell(relay::Told::Say(told_of(feeding)));
    }
    fn auger_still(&mut self, feeding: Feeding) {
        Organs::tell(relay::Told::Still(told_of(feeding)));
    }
    fn igniter(&mut self, on: bool) {
        Organs::relay(&roles::IGNITER, on);
    }
    fn pump(&mut self, on: bool) {
        Organs::relay(&roles::PUMP, on);
    }
    fn fan(&mut self, asked: Asked) {
        if let Err(why) = firmware::site::command(&roles::FAN, asked) {
            log::error!("automaton: the fan was not asked {asked:?}: {why}");
        }
    }
}

/// THE AUTOMATON'S STACK IS ITS OWN, NOT THE ROUNDS' (6.10, after the flash of 26.10.03.3): the thermostat runs in this task and not in a task of its own - the order is made on the readings of the second it is taken in, a press is carried out in the breath it comes -, so the
/// stack holds the fire, the thermostat's door and order, the schedule and the card, and the least free of it on 26.10.03.3 was 1032 bytes of the 8192 `round::STACK` gives, on a burner standing still. Every other round has enough of the common one; this one is given 12288, a multiple of 4096.
const STACK: usize = 12288;

/// Starts the round. Call once, after the flame, the flue, the draught and the fan have started.
pub fn start() -> Result<(), esp_idf_svc::sys::EspError> {
    // the card of an automaton not up yet, so a reader before its first second gets the card and not nothing
    publish_card(&None);
    thermostat::publish();
    demand::publish();
    firmware::task::round("automaton", STACK, firmware::task::Core::Hardware, run)
}

/// NUMBERS through the automaton's door, or the door's refusal.
fn sound_of(numbers: Tuning) -> Result<Sound, ::automaton::Refused> {
    numbers.sound()
}

/// The numbers of the card through both doors, the fire's and the thermostat's, the first refusal of either: **ONE CARD, ONE REFUSAL** - a person fixes a number on the card and the card says which one, whichever of the two it belongs to.
fn doors() -> Result<(Sound, ::thermostat::Sound), ::automaton::Refused> {
    Ok((sound_of(tuning())?, thermostat::door()?))
}

fn run() {
    // the witnesses first: `resumed` decides by them, and one that has not spoken is not darkness
    let began = between::clock::now();
    let mut eyes = Eyes::open();
    let mut mind = Mind { core: None, shown: None, early: None };
    // the desk is served from the first moment: a press waits for the first second, a hand on an organ goes through
    while (eyes.flame.now().is_none() || eyes.flue.now().is_none()) && began.elapsed() < FIRST_READINGS {
        DESK.serve_for(Duration::from_millis(200), |asked| mind.answer(asked));
    }
    let mut refused = None;
    let (sound, heat_sound) = loop {
        match doors() {
            Ok(sounds) => break sounds,
            Err(why) => {
                if refused.as_ref() != Some(&why) {
                    log::error!("automaton: not started, a number of the card is refused: {}", why.words());
                    mind.shown = Some(Shown { doing: None, since: between::clock::now(), refused: Some(why.clone()), clocks: Vec::new(), broke_at: None, feed_allowed: true, semi: false });
                    publish(&mind.shown);
                    refused = Some(why);
                }
                DESK.serve_for(EVERY, |asked| mind.answer(asked));
            }
        }
    };
    let kept = nvs_settings::recall(api::CARD);
    let was = kept.as_deref().and_then(::automaton::recalled);
    let now = seen(&mut eyes);
    let burning = Burning::resumed(was.unwrap_or(State::Stop), &now, &sound);
    log::info!(
        "automaton: came up remembering {} ({}), sees flame {:?}, flue {:?} - goes to {}",
        was.map_or("nothing".to_string(), ::automaton::state_words),
        kept.as_deref().unwrap_or("no key"),
        now.flame,
        now.flue_c,
        ::automaton::state_words(burning.state())
    );
    // THE FAULT FOR WANT OF FRONTS IS LET GO OF BY THE BOARD when the pin has been back sixty seconds in a row, three times a local day at most (`automaton::Relief`); the count of the day is read back from the flash
    let mut letting = crate::modules::relief::open();
    let mut last: Option<Doing> = None;
    let mut since = between::clock::now();
    let mut step_at = between::clock::now();
    let mut broke_at: Option<&'static str> = None;
    // THE MODE COMES BACK AS IT WAS SWITCHED; nothing kept, or a word of something else, is Auto
    let semi = ::automaton::semi_of(nvs_settings::recall(::automaton::MODE_CARD).as_deref());
    // WHERE THE FIRE STANDS as the board comes up: the thermostat is told it with every look, and finds the end of burning in the look that says Stop after one that said anything else
    let out_at_start = burning.state() == State::Stop;
    let mut driving = Driving::new(Automaton::new(burning), Organs);
    driving.set_semi(semi);
    SEMI_ON.store(semi, Ordering::Relaxed);
    log::info!("automaton: comes up in {}", if semi { "Semi" } else { "Auto" });
    // THE SWITCH THE PERSON LEFT comes back: heat asked for before the power went is asked for still, and a Stop stays a Stop; the fire's own memory (`resumed`) carries the stage, and a fault waits for a person's Stop whatever the switch
    let mut heat = Heat::new(heat_sound, thermostat::recalled());
    // THE HOUSE SPEAKS BEFORE THE THERMOSTAT ORDERS: the word of the first second is on its role when the first order is made
    let mut demanding = Demanding::new();
    demanding.second(between::clock::now());
    // the order of the first second, published before the automaton holds its place on the role: a press that came before it did is carried out on it
    let order = heat.second(between::clock::now(), out_at_start, now.flame == Flame::Steady);
    let mut came = Core { driving, seen: now, sound, heat, demanding, order, started: None };
    if let Some(press) = mind.early.take() {
        // a press that came before the first second is judged on it, and a Light the nominal refuses is said in the journal: there is nobody to answer
        if let Err(refused) = came.press(press, &mut mind.shown) {
            log::info!("automaton: the press that came before the first second was refused: {refused}");
        }
    }
    FEED_ALLOWED.store(came.driving.automaton.feed_allowed(), Ordering::Relaxed);
    mind.core = Some(came);
    loop {
        let round = between::clock::now();
        let secs = step_at.elapsed().as_secs_f32();
        step_at = between::clock::now();
        let (doing, clocks, started, feed_allowed, semi, lit_by, went_out_by, draught_said) = {
            let core = mind.core.as_mut().expect("the automaton came up above");
            // numbers changed on the card are taken at the next second, and the automaton itself takes them at the next boundary of a dose; a refused set leaves the last sound one working and says why on the card
            match doors() {
                Ok((fresh, heat_fresh)) => {
                    core.sound = fresh;
                    core.heat.take(heat_fresh);
                    core.demanding.take();
                    if refused.take().is_some() {
                        log::info!("automaton: the card's numbers are taken again");
                    }
                }
                Err(why) => {
                    if refused.as_ref() != Some(&why) {
                        log::warn!("automaton: a number of the card is refused, the last good ones stay: {}", why.words());
                        refused = Some(why);
                    }
                }
            }
            core.seen = seen(&mut eyes);
            // the fault for want of fronts goes before the second is taken: a stop, so the second below is a stop's, and the burner lights by the room as after any stop
            if letting.second(core.driving.automaton.state(), crate::modules::fan::mains_alive_s()).is_some() && !core.driving.clear_fault(::automaton::Why::NoFronts) {
                log::error!("automaton: the fault for want of fronts was to be let go of and was not a fault any more");
            }
            // the house says cold or warm on the readings of this second, and the thermostat orders on the word; the automaton takes the order from the role and carries it out
            core.demanding.second(between::clock::now());
            core.heat.faulted(matches!(core.driving.automaton.state(), State::Fault(_)));
            core.heat.second(between::clock::now(), core.driving.automaton.state() == State::Stop, core.seen.flame == Flame::Steady);
            core.order = eyes.heat.now().unwrap_or(core.order);
            let doing = core.driving.second(&core.order, core.seen, secs, &core.sound);
            // a plug a person switched on goes off the second the air goes: the automaton's own already does
            let closed = crate::modules::relays::igniter_closed();
            if closed && !::automaton::plug_stays(closed, crate::modules::fan::gate_percent(), core.seen.draught_pa) {
                log::warn!("automaton: the igniter is switched off - the fan makes no air under it");
                Organs::relay(&roles::IGNITER, false);
            }
            let automaton = &core.driving.automaton;
            // a stop that goes to a purge is a light: the journal names who asked for it - the button, the house, the water - as the order of this second stood
            let lit_by = (last.is_some_and(|d| d.state == State::Stop) && doing.state == State::Purge).then(|| why_lit(&core.order));
            // a smouldering fire that goes to burning off with the flame dark went out with no heat asked: the journal says it was not lit again, and what the water said
            let went_out_by = (last.is_some_and(|d| d.state == State::Smouldering) && doing.state == State::BurningOff(None) && core.seen.flame == Flame::Out).then(|| ::automaton::water_word(core.order.water));
            // a burning off by the draught names what lay in `seen` at the second it was decided: the card shows the readings, and the decision's own second is what the record could not tell (29.10 15:44:33)
            let draught_said = match (last.map(|d| d.state), doing.state) {
                (was, State::BurningOff(Some(why))) if was != Some(doing.state) => ::automaton::voice::draught_line(why, &core.seen, automaton.draught_counts(), crate::modules::draught::last_mv()),
                _ => None,
            };
            FLAME_SEEN.store(core.seen.flame != Flame::Out, Ordering::Release);
            (doing, automaton.clocks(&core.sound), core.started, automaton.feed_allowed(), automaton.semi(), lit_by, went_out_by, draught_said)
        };
        let clock = started.map_or(String::new(), |t| format!("+{} s ", t.elapsed().as_secs()));
        // A CHANGE OF STATE IS WHAT THE JOURNAL IS FOR, AND THE ORGANS INSIDE ONE STATE ARE A DETAIL: the auger on and off at every dose was 44 lines in ten minutes of fire, and the card shows it every second.
        match ::automaton::voice::voice(last.map(|d| d.state), doing.state, last.is_some_and(|was| said(&was) != said(&doing))) {
            ::automaton::voice::Voice::Detail => {
                log::debug!("automaton: {clock}{}: {}", ::automaton::state_words(doing.state), said(&doing));
            }
            ::automaton::voice::Voice::Silent => {}
            ::automaton::voice::Voice::Event => {
                broke_at = ::automaton::broken_at(broke_at, last.map(|d| d.state), doing.state);
                log::info!("automaton: {clock}{} -> {}: {}", last.map_or("start".to_string(), |d| ::automaton::state_words(d.state)), ::automaton::state_words(doing.state), said(&doing));
                if let Some(by) = lit_by {
                    log::info!("automaton: lit from a stop - {by}");
                }
                if let Some(said) = draught_said {
                    log::warn!("automaton: {said}");
                }
                if let Some(water) = went_out_by {
                    log::info!("automaton: the flame went out in smouldering, water {water}: no heat asked, burning off and not lit again");
                }
                since = between::clock::now();
                if let Err(why) = nvs_settings::remember(api::CARD, &::automaton::remembered(doing.state)) {
                    log::error!("automaton: the state was not written down: {why}");
                }
            }
        }
        if doing.alarm && !last.is_some_and(|d| d.alarm) {
            log::error!("automaton: ALARM - the water is past {} °C", nvs_settings::value(&burner_settings::AUTO_ALARM_C));
        }
        crate::modules::soot::observe(doing.state == State::Working, mind.core.as_ref().is_some_and(|c| c.heat.rung() == Some(::thermostat::Rung::Big)), mind.core.as_ref().and_then(|c| c.seen.flue_c), mind.core.as_ref().and_then(|c| c.seen.boiler_c));
        last = Some(doing);
        mind.shown = Some(Shown { doing: Some(doing), since, refused: refused.clone(), clocks, broke_at, feed_allowed, semi });
        publish(&mind.shown);
        // the rest of the second is the desk's: a press or a hand on an organ is answered as it comes
        DESK.serve_for(EVERY.saturating_sub(round.elapsed()), |asked| mind.answer(asked));
    }
}

/// How the burner stands as the last second left it, for the planned restart: 0 not known yet, 1 at rest, 2 fire, 3 fault.
static REST: AtomicU8 = AtomicU8::new(0);
/// Whether the flame sensor saw a flame in the last second.
static FLAME_SEEN: AtomicBool = AtomicBool::new(false);

/// How the burner stands now; None before its first second.
pub fn rest() -> Option<::automaton::Rest> {
    match REST.load(Ordering::Acquire) {
        1 => Some(::automaton::Rest::Standing),
        2 => Some(::automaton::Rest::Fire),
        3 => Some(::automaton::Rest::Fault),
        _ => None,
    }
}

/// Publishes SHOWN and tells an open page.
fn publish(shown: &Option<Shown>) {
    REST.store(
        match shown.as_ref().and_then(|s| s.doing) {
            None => 0,
            Some(doing) => match ::automaton::rest(&doing, FLAME_SEEN.load(Ordering::Acquire)) {
                ::automaton::Rest::Standing => 1,
                ::automaton::Rest::Fire => 2,
                ::automaton::Rest::Fault => 3,
            },
        },
        Ordering::Release,
    );
    publish_card(shown);
    web::announce(api::CARD);
    // the thermostat's and the demand's cards move with the automaton's second and with a press: they have no task of their own
    thermostat::publish();
    demand::publish();
}

/// Why a stop is lit, for the journal: the switch, what the house says and why, and the word of the water - the three things a light is decided by.
fn why_lit(order: &Order) -> String {
    let house = roles::DEMAND.latest_at(between::clock::now()).map_or("nobody asks".to_string(), |asked| format!("{} ({})", ::demand::demand_word(asked.demand), ::demand::why_word(asked.why)));
    format!("switch {}, house {house}, water {}", ::automaton::switch_word(order.switch), ::automaton::water_word(order.water))
}

/// On or off, for the journal.
fn on(yes: bool) -> &'static str {
    if yes { "on" } else { "off" }
}

/// What the fan is asked for, for the journal.
fn fan_words(asked: Asked) -> String {
    match asked {
        Asked::Stop => "off".into(),
        Asked::Purge => "full, no setpoint".into(),
        Asked::Hold(pa) => format!("{pa:.0} Pa"),
        Asked::ByHand => "by hand".into(),
    }
}

/// What the organs are told this second, in one line of the journal: which of them it does and which it only would.
fn said(doing: &Doing) -> String {
    format!("auger {}, igniter {}, pump {}, fan {}", on(doing.auger), on(doing.igniter), on(doing.pump), fan_words(doing.fan))
}

/// How each of the four sensors the automaton sees by is heard at AT: the draught, the flame, the flue and the water, in this order. **ONE LIST FOR WHAT THE AUTOMATON IS TOLD OF THEM AND WHAT THE
/// CARD SAYS OF THEM** ([`silence_of`], [`silent_of`]): a fifth sensor is one line here, and not an edit in two files that agree by hand. The return and the air are the thermostat's: it holds by them
/// and says their silence on its own card (`modules::thermostat`).
fn quiets(at: Moment) -> [module::Quiet; 4] {
    [roles::DRAUGHT.quiet_at(at), roles::FLAME.quiet_at(at), roles::FLUE_GAS.quiet_at(at), roles::BOILER.quiet_at(at)]
}

/// What the automaton is told of the four: whether each speaks, and if not whether for long enough to stop a burner.
fn silence_of(quiets: &[module::Quiet; 4]) -> Silence {
    let [draught, flame, flue, water] = quiets.map(heard);
    Silence { draught, flame, flue, water, fronts_s: crate::modules::fan::mains_silent_s() }
}

/// What the card says of the four: the seconds each has been silent - nothing while it speaks. The card of a sensor whose round has stopped is published by nobody, so the automaton says it.
fn silent_of(quiets: &[module::Quiet; 4]) -> Silent {
    let [draught, flame, flue, water] = quiets.map(crate::modules::thermostat::silent_for);
    Silent { draught, flame, flue, water, fronts: crate::modules::fan::mains_silent_s() }
}

/// How long each sensor the automaton sees by has been silent now, for its card.
fn silent_now() -> Silent {
    silent_of(&quiets(between::clock::now()))
}

/// The card by address: what the API names, from what the automaton did last second.
fn values(shown: &Option<Shown>, put: &mut dyn Fill) {
    let silent = silent_now();
    // the water the alarm sounds past, as the owner has set it
    let alarm_c = nvs_settings::value(&burner_settings::AUTO_ALARM_C).parse::<f32>().unwrap_or(f32::NAN);
    let now = match shown.as_ref() {
        Some(shown) => Now {
            doing: shown.doing,
            lasted_s: shown.since.elapsed().as_secs() as u32,
            refused: shown.refused.as_ref(),
            clocks: &shown.clocks,
            broke_at: shown.broke_at,
            feed_allowed: shown.feed_allowed,
            semi: shown.semi,
            silent,
            relief: crate::modules::relief::now(),
            alarm_c,
        },
        None => Now { doing: None, lasted_s: 0, refused: None, clocks: &[], broke_at: None, feed_allowed: true, semi: SEMI_ON.load(Ordering::Relaxed), silent, relief: crate::modules::relief::now(), alarm_c },
    };
    api::put_values(&now, &mut |at, said| put.said(at, said));
}

/// The automaton's card as it last published it; two text slots for a refusal of its numbers: the field and the field it was held against.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 2> = Published::new();

/// Puts the card together and publishes it: every second of the automaton and every press or switch of the feed.
fn publish_card(shown: &Option<Shown>) {
    firmware::publish(&PUBLISHED, api::CARD, |card| values(shown, card));
}

/// The automaton by the modules' contract (crates/module): its card, its settings, its two switches.
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn takes(&self) -> &'static [&'static str] {
        &["toggle", "on", "off"]
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    /// The two switches of the card: the feed allowed or banned, and Semi on or off.
    fn is_on(&self, name: &str) -> Option<bool> {
        match name {
            FEED => Some(FEED_ALLOWED.load(Ordering::Relaxed)),
            SEMI => Some(SEMI_ON.load(Ordering::Relaxed)),
            _ => None,
        }
    }

    fn settings(&self) -> Vec<&'static Setting> {
        api::kept()
    }

    /// Its two switches: the feed allowed or banned from the main screen, and Auto or Semi from the scheme.
    fn order(&self, name: &str, order: module::Order) -> Option<Result<(), String>> {
        match (name, order) {
            (FEED, module::Order::Switch(on)) => Some(ask(Request::Feed(Some(on)))),
            (FEED, module::Order::Toggle) => Some(ask(Request::Feed(None))),
            (SEMI, module::Order::Switch(on)) => Some(ask(Request::Semi(Some(on)))),
            (SEMI, module::Order::Toggle) => Some(ask(Request::Semi(None))),
            (FEED | SEMI, other) => Some(Err(format!("{name} does not take {other:?}"))),
            _ => None,
        }
    }

    /// **EVERY PERSON'S COMMAND TO A RELAY OR THE FAN PASSES HERE** (crates/module, `person`): judged by the state the automaton is in now and carried out in its task, so it cannot take a second between the two. In Auto, while it runs the
    /// burner, the command is refused in words that say what to switch; in a stop, in a fault and in Semi it is carried out, except the auger's while the feed is banned (`automaton::by_hand`).
    fn through(&self, modules: &'static module::Modules, name: &str, order: module::Order) -> Option<Result<(), String>> {
        let kept = burner_api::relays::NAMES.contains(&name) || burner_api::fan::BY_HAND.contains(&name);
        kept.then(|| ask(Request::Through(modules, name.to_string(), order)))
    }
}
