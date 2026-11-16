//! HERE: the fan: the draught of the last second (src/roles.rs) held by crates/regulator through the gate of src/hardware/fan_gate.rs, a person's commands to its two sliders and its purge, the card it publishes by api/src/fan.rs, the
//! curves it keeps, and the numbers under the regulator's knobs; its place in the module list.
//! NEVER HERE: an interrupt or a pin (the hardware, burner.yaml), what angle holds the draught (crates/regulator), which moments of a half cycle a percent is (crates/dimmer), a word of the page (page/menu.json), a code of the stream
//! (api/src/fan.rs). What this file owns is the ORDER, once a second: read the draught, decide, carry it to the gate, keep, publish.
//! CHECKS: devices/burner/api/tests/fan_table.rs, devices/burner/api/tests/fan_values.rs, crates/regulator/tests/holding.rs, crates/regulator/tests/knobs.rs, crates/dimmer/tests/dimmer.rs.
//!
//! THE REGULATOR LIVES IN ONE TASK, `holding`, and nowhere else: what it is asked for, the regulator, what it said, the gate, the percent set and the curves. The page and the console ask at its desk - the automaton will, too; the gate's
//! counting round hands it the gate once and a second every second.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use between::Desk;
use burner_api::fan::{self as api, Gate as Outcome, Now, Refusal, BY_HAND, CURVES, DRAUGHT_ROW, GATE_ROW, PURGE_ROW};
use esp_idf_svc::sys::EspError;
use firmware::nvs_settings;
use module::card::{Fill, Published, Reads};
use page::dictionary::Value;
use regulator::knobs;
use settings::Setting;

use crate::hardware::fan_gate::{self, Gate, Second};

/// How often the regulator steps.
///
/// ONCE A SECOND, AND IT IS THE TRACT THAT SAYS SO. The pressure creeps for ten to fifteen seconds at small angles; a regulator stepping ten times a second would be answering the breathing of the air, not the boiler. The sensor's own
/// second of readings is averaged for exactly this step (`modules::draught`).
const STEP: Duration = Duration::from_secs(1);

/// How long the convergence curve holds: five minutes at a point a second. Five, because that is what a step of the setpoint takes to be read off a picture - arrival, overshoot and whether it settles - and three rings of it cost 1824 bytes
/// on the board, 300 values of 2 bytes and two counters of 4 each, which is fixed and known.
const HOLD_POINTS: usize = 300;

/// How long the others wait for the regulator: it answers between its steps, and a step is a second.
const WITHIN: Duration = Duration::from_secs(2);

/// The card of the fan as its regulator last published it.
static PUBLISHED: Published<{ page::dictionary::addresses(&burner_api::TABLE, api::CARD) }, 0> = Published::new();

/// Whether the fan purges now, for the console's answer after a command (`is_on`). Written by the regulator alone.
static PURGING: AtomicBool = AtomicBool::new(false);

/// The point the knobs are answered for - pascals and percent, as the bits of two f32 - as the regulator last knew it: read where the settings are asked about, which is not the regulator's task. Two words and not one of 64 bits, which this chip
/// has no atomic for; the two may be read from two different seconds, and the numbers under a knob then say a fan that is between them - the words move with the knobs and not with the breathing either way.
static POINT_PA: AtomicU32 = AtomicU32::new(0);

/// The percent the gate is asked to now; 0 before the regulator has set it. Written by the regulator's task alone, read by whoever judges a plug beside the draught (`automaton::plug_by_hand`): what the automaton sees of the air.
static GATE_PERCENT: AtomicU32 = AtomicU32::new(0);

/// How long the zero-crossing pin has given no fronts, in seconds, while it gives none: what the automaton and the card are told of the pin. None while it gives them, and before its first second.
pub fn mains_silent_s() -> Option<u32> {
    fan_gate::second().and_then(|second| second.silent_s)
}

/// How many seconds in a row the zero-crossing pin has given a mains, while it does: what the fault for want of fronts is let go by (`automaton::Relief`). None while it gives none, and before its first second.
pub fn mains_alive_s() -> Option<u32> {
    fan_gate::second().and_then(|second| second.alive_s)
}

/// The percent the gate is asked to now; 0 before the regulator has set it.
pub fn gate_percent() -> u32 {
    GATE_PERCENT.load(Ordering::Relaxed)
}
static POINT_ANGLE: AtomicU32 = AtomicU32::new(0);

fn point() -> (f32, f32) {
    match (POINT_PA.load(Ordering::Relaxed), POINT_ANGLE.load(Ordering::Relaxed)) {
        (0, _) | (_, 0) => regulator::A_FAN_LIKE_THIS,
        (pa, angle) => (f32::from_bits(pa), f32::from_bits(angle)),
    }
}

/// Why the gate refused, as the module knows it where it refuses: the code for the stream, the words for the log.
struct Refused {
    why: Refusal,
    words: String,
}

/// What the fan's regulator is asked by the others.
enum Request {
    /// A person's command to one of its rows (crates/module, `Order`).
    Order(String, module::Order),
    /// The automaton's hand on the fan (role `fan`).
    Automaton(regulator::Asked),
    /// A curve by the name the card offers it under.
    Curve(String),
}

/// What it answers.
enum Answer {
    Done(Result<(), String>),
    Curve(Option<page::Curve>),
}

/// Where the others ask the fan's regulator: a person from the page or a console.
static DESK: Desk<Request, Answer, 4, 3> = Desk::new();

/// The fan as its regulator owns it: what it is asked for, the regulator, what it said, the gate, the percent set and the curves - all in the task `holding`.
struct Fan {
    /// What the fan is asked to do. The person sets it from its card, and it is the same three words whoever asks (`regulator::Asked`).
    ///
    /// IT LIVES IN MEMORY AND NOT IN nvs, and the board comes up with the fan STOPPED. The setting seeds the setpoint a person starts from; the value they then slide to is this. Same rule as the relays: every organ off whatever it was
    /// before the restart, and moved from then on only by a command.
    asked_to: regulator::Asked,
    /// The regulator itself. None until the first round builds it from the angle the fan is ALREADY held at.
    holding: Option<regulator::Holding>,
    /// What the last round decided AND what became of that decision at the triac. Two facts and not one: the angle is an intention, and until 21.09 the round published it as held even when the gate had refused it.
    said: Option<(regulator::Held, Outcome)>,
    /// The gate, once the counting round has built it and handed it over.
    gate: Option<Gate>,
    /// The percent last set at the gate, and the moment it was set.
    asked: Option<(u32, between::clock::Moment)>,
    /// The last second the gate counted.
    second: Option<Second>,
    /// What the card showed when it was last announced: a second that counted the same is not news.
    shown: Option<(u32, u32, Option<u32>)>,
    /// The three lines of the convergence curve: what was asked, what came out, and the angle it is held by.
    ///
    /// **THE THIRD LINE IS NOT DECORATION.** Without it "the regulator is twitching" and "the air is breathing" look the same on a picture. The angle also answers a question nothing else does: the same setpoint held at a rising angle
    /// means something has got harder - the flue is furring up, the impeller is sooty, a bearing is going - and the regulator HIDES that by holding the pressure. On this curve it is visible weeks before it stops being able to.
    ///
    /// Each ring in a box of its own, built there by `Ring::boxed`: the six of them are 5.4 KB, and a `Fan` that carried them inline went through the 8 KB stack of `holding` on its way to the heap and overflowed it at start.
    wanted: Box<curve::Ring<HOLD_POINTS>>,
    came_out: Box<curve::Ring<HOLD_POINTS>>,
    by_angle: Box<curve::Ring<HOLD_POINTS>>,
    /// The three lines of the regulator's own curve: the miss, and the P and I halves of the move it made of it, a point a second and NaN where it regulated nothing. **SO THE GAINS ARE TUNED BY SIGHT**: which half moved the angle, and
    /// which set did it, is read here.
    miss: Box<curve::Ring<HOLD_POINTS, f32>>,
    p_half: Box<curve::Ring<HOLD_POINTS, f32>>,
    i_half: Box<curve::Ring<HOLD_POINTS, f32>>,
}

// the regulator is born on the stack of `holding` and lives there: its curves are in the heap, and it stays small
const _: () = assert!(core::mem::size_of::<Fan>() <= firmware::task::ON_STACK_AT_MOST);

impl Fan {
    fn new() -> Fan {
        Fan {
            asked_to: regulator::Asked::Stop,
            holding: None,
            said: None,
            gate: None,
            asked: None,
            second: None,
            shown: None,
            wanted: curve::Ring::boxed(0),
            came_out: curve::Ring::boxed(0),
            by_angle: curve::Ring::boxed(0),
            miss: curve::Ring::boxed(f32::NAN),
            p_half: curve::Ring::boxed(f32::NAN),
            i_half: curve::Ring::boxed(f32::NAN),
        }
    }

    /// The percent set at the gate, now; the counting round hears of it.
    fn set_asked(&mut self, percent: u32) {
        GATE_PERCENT.store(percent, Ordering::Relaxed);
        self.asked = Some((percent, fan_gate::asked(percent)));
    }

    /// What the fan is, for the API to give each of it its code.
    fn now(&self) -> Now {
        // Whether the last full second is even about the percent being held now. Straight after the slider moves it is not, and a count from before the move would paint the row red for a second on every move.
        let counted_this = match (&self.second, self.asked) {
            (Some(second), Some((_, at))) => second.ended >= at,
            (Some(_), None) => true,
            (None, _) => false,
        };
        Now {
            asked_to: self.asked_to,
            said: self.said,
            draught: crate::roles::DRAUGHT.latest(),
            second: self.second.as_ref().map(|s| api::Second { crossings: s.crossings, pulses: s.pulses }),
            counted_this,
            percent: self.asked.map(|(percent, _)| percent),
            seeded_pa: seeded_pa(),
        }
    }

    /// What the fan says of itself by address: TWO NUMBERS, NOT A WORD, for the gate - the stage is closed by a hundred pulses against a hundred crossings, and from the boiler there is no wire to read a log on - so what proves the
    /// fan is a thing the page carries, readable by one request at any hour.
    fn values(&self, card: &mut dyn Fill) {
        api::put_values(&self.now(), &mut |at, value| card.put(at, value));
    }

    /// Puts the card together and publishes it: every step of the regulator, and wherever the fan's state is moved.
    fn publish(&self) {
        firmware::publish(&PUBLISHED, api::CARD, |card| self.values(card));
    }

    /// Publishes the card, and tells an open page when what it shows has moved: the crossings, the pulses and the percent are read live - a person turns the slider and watches the pulses follow the crossings.
    fn publish_and_tell(&mut self) {
        self.publish();
        let second = self.second.as_ref();
        let shown = Some((second.map_or(0, |s| s.crossings), second.map_or(0, |s| s.pulses), self.asked.map(|(percent, _)| percent)));
        if shown != self.shown {
            self.shown = shown;
            web::announce(api::CARD);
        }
    }

    /// The half cycle a percent is fired against now, by the last second: the measured one, or before the first window the nominal one over a live detector.
    fn half_to_fire(&self) -> Result<u32, Refused> {
        let second = self.second.as_ref();
        let measured = second.and_then(|second| match &second.mains {
            Err(why) if why.is_empty() => None,
            read => Some(read.clone()),
        });
        // the crossings of the last whole second, and none if that second is old: a detector that stopped is not alive
        let crossings = second.filter(|second| second.ended.elapsed() < Duration::from_secs(3)).map_or(0, |second| second.crossings);
        let why = api::mains_refusal(second.map(|second| &second.mains));
        dimmer::half_to_fire(measured, crossings).map_err(|words| Refused { why, words })
    }

    /// Sets the angle for the percent asked, using the half cycle the board has just measured. Off puts both comparators out of reach, so the gate stays down.
    fn hold(&mut self, percent: u32) -> Result<String, Refused> {
        // OFF IS NEVER REFUSED, and that is the whole shape of this function. A person pressing Stop on a running motor is not asking the board to check anything first, and every reason this could refuse - the mains not measured yet, the
        // gate not built - is a reason the fan is ALREADY not turning. Answering "no" to that is a board arguing with somebody who wants it to stop.
        let order = match percent {
            0 => dimmer::Order::Off,
            percent => {
                let half = self.half_to_fire()?;
                dimmer::order(percent, half).map_err(|words| Refused { why: if percent > 100 { Refusal::OverScale } else { Refusal::MainsWrong }, words })?
            }
        };
        let Some(gate) = self.gate else {
            if percent == 0 {
                self.set_asked(0);
                return Ok("0%: there is no gate yet, and nothing is firing".to_string());
            }
            return Err(Refused { why: Refusal::NoGate, words: "the gate was never built: see the log at start".into() });
        };
        let (open_at, close_at) = match order {
            dimmer::Order::Fire { delay_us, gate_us } => (delay_us, delay_us + gate_us),
            dimmer::Order::Off => (fan_gate::DOWN_AT_US, fan_gate::DOWN_AT_US),
        };
        gate.move_to(open_at, close_at).map_err(|words| Refused { why: Refusal::GateRefused, words })?;
        self.set_asked(percent);
        // the shell's `dim` moves the gate too, and an open page has no other way of hearing about it
        self.publish();
        web::announce(api::CARD);
        Ok(match order {
            dimmer::Order::Fire { delay_us, gate_us } => format!("{percent}% of air: gate at {delay_us} µs for {gate_us} µs"),
            dimmer::Order::Off => format!("{percent}%: the gate stays down"),
        })
    }

    /// A slider on the fan's card moved.
    ///
    /// TWO OF THEM, AND THEY ARE DIFFERENT THINGS. The draught is what a person asks for, in pascals, and the regulator holds it; the gate is the angle by hand, in percent, and it is for probing - a percent carries no physical meaning
    /// at all, which is the measurement this whole stage rests on (30 % read as 89 Pa and, within the hour, as 47).
    fn held(&mut self, name: &str, value: u32) -> Result<(), String> {
        if name == DRAUGHT_ROW {
            return self.hold_pa(value);
        }
        if name != GATE_ROW {
            return Err(format!("this board holds nothing at a number, and no {name}"));
        }
        // BY HAND MEANS THE REGULATOR LETS GO. Two writers of one angle is the fault this avoids: the person would move the slider and the next second would take it back, and the board would look broken.
        self.asked_to = regulator::Asked::ByHand;
        self.hold(value).map_err(|refused| refused.words)?;
        Ok(())
    }

    /// The draught the card's slider asks for, in pascals.
    fn hold_pa(&mut self, pa: u32) -> Result<(), String> {
        // Zero pascals is STOP and not a pressure to chase: chasing it would leave the motor humming at the floor of the angle. Any other number is the setpoint from now on.
        //
        // **COMMANDING IS NOT SAVING, AND THIS ROUTE ONLY COMMANDS.** It used to keep the number in nvs as well, which made a measurement into a write: a probe that walks the setpoint wore the flash and - worse - opened a window with the
        // cache off at a moment nobody chose. What keeps the setpoint is the card's Save (`POST /settings`), which is a person deciding, and that is the only thing that should reach the flash.
        self.asked_to = match pa {
            0 => regulator::Asked::Stop,
            pa => regulator::Asked::Hold(pa as f32),
        };
        Ok(())
    }

    /// The purge turned on or off - or to the other side of what it is (None): full air with no feedback, or the fan stopped.
    fn purge(&mut self, on: Option<bool>) {
        self.asked_to = match on.unwrap_or(self.asked_to != regulator::Asked::Purge) {
            true => regulator::Asked::Purge,
            false => regulator::Asked::Stop,
        };
    }

    /// A person's ORDER to the row NAME.
    fn order(&mut self, name: &str, order: module::Order) -> Result<(), String> {
        match (name, order) {
            (PURGE_ROW, module::Order::Switch(on)) => self.purge(Some(on)),
            (PURGE_ROW, module::Order::Toggle) => self.purge(None),
            (GATE_ROW | DRAUGHT_ROW, module::Order::Hold(value)) => self.held(name, value)?,
            (_, other) => return Err(format!("{name} does not take {other:?}")),
        }
        Ok(())
    }

    /// What the others asked, carried out in the regulator's task.
    fn answer(&mut self, request: Request) -> Answer {
        let done = match request {
            Request::Curve(name) => return Answer::Curve(self.curve(&name)),
            Request::Order(name, order) => self.order(&name, order),
            // WRITTEN WHERE THE CARD'S SLIDER AND PURGE BUTTON WRITE, so the round takes it as it takes a person's - one fan, one thing it is asked for, and the card shows whoever asked last
            Request::Automaton(asked) => {
                self.asked_to = asked;
                Ok(())
            }
        };
        PURGING.store(self.asked_to == regulator::Asked::Purge, Ordering::Relaxed);
        if done.is_ok() {
            self.publish();
            web::announce(api::CARD);
        }
        Answer::Done(done)
    }

    /// The regulator's own curve: the miss the picture is about - its sign names the set of gains that worked - and the P and I halves of each move on one scale of their own, so the bigger half reads as bigger. What the lines are called
    /// is the menu's.
    fn regulator_curve(&self) -> page::Curve {
        let half = |ring: &curve::Ring<HOLD_POINTS, f32>| page::Line { words: String::new(), unit: "% a second".into(), points: ring.points(), span: None };
        page::Curve {
            step_ms: STEP.as_millis() as u32,
            lines: vec![
                page::Line { words: String::new(), unit: "Pa".into(), points: self.miss.points(), span: None },
                half(&self.p_half),
                half(&self.i_half),
            ],
        }
    }

    /// What the regulator on the settings of now does with a jump of [`knobs::JUMP_PA`] on the fan it knows, second by second - asked for by the page and drawn by it, never kept.
    fn response_curve(&self) -> page::Curve {
        let (pa, angle) = knobs::known(self.holding.as_ref());
        let (mut came_out, mut by_angle) = (Vec::with_capacity(regulator::ANSWER_S), Vec::with_capacity(regulator::ANSWER_S));
        regulator::answer_to_a_jump(&tuning(), pa, angle, knobs::JUMP_PA, &mut |p, a| {
            came_out.push(p);
            by_angle.push(a);
        });
        page::Curve {
            step_ms: STEP.as_millis() as u32,
            lines: vec![
                page::Line { words: String::new(), unit: "Pa".into(), points: vec![pa; came_out.len()], span: None },
                page::Line { words: String::new(), unit: "Pa".into(), points: came_out, span: None },
                page::Line { words: String::new(), unit: "%".into(), points: by_angle, span: None },
            ],
        }
    }

    /// The convergence curve, the regulator's and its answer to a jump, by the names the card offers them under.
    fn curve(&self, name: &str) -> Option<page::Curve> {
        if name == CURVES[1] {
            return Some(self.regulator_curve());
        }
        if name == CURVES[2] {
            return Some(self.response_curve());
        }
        if name != CURVES[0] {
            return None;
        }
        let of = |ring: &curve::Ring<HOLD_POINTS>| ring.points().iter().map(|p| *p as f32).collect::<Vec<f32>>();
        Some(page::Curve {
            step_ms: STEP.as_millis() as u32,
            lines: vec![
                page::Line { words: String::new(), unit: "Pa".into(), points: of(&self.wanted), span: None },
                page::Line { words: String::new(), unit: "Pa".into(), points: of(&self.came_out), span: None },
                // its own unit, so it is drawn against its own scale: one picture cannot hold pascals and percent on one axis, and the shape is what the angle is read for
                page::Line { words: String::new(), unit: "%".into(), points: of(&self.by_angle), span: None },
            ],
        })
    }
}

/// What the settings of the card say now.
///
/// TAKEN AFRESH EVERY ROUND, like the draught's divider and for the same reason: a person turns these while the boiler runs, and a regulator that read them once at start would hold by numbers nobody has any more.
fn tuning() -> regulator::Tuning {
    let now = nvs_settings::now();
    burner_settings::fan_tuning(&|s| now.value(s))
}

/// The setpoint the card's setting holds, in pascals: the one the board COMES UP AT.
///
/// **IT IS WHERE THE SLIDER STANDS, NOT WHAT THE FAN IS BEING HELD AT.** The slider commands and the card's Save keeps, so the two numbers part company the moment a person moves the slider without saving - and the row says so rather than
/// leaving them to find out at the next restart (`comes_up`).
fn seeded_pa() -> f32 {
    nvs_settings::value(&burner_settings::DRAUGHT_PA).parse::<u32>().unwrap_or(0) as f32
}

/// THE ROUND'S STACK, the round that holds the draught (from the board of 04.10): the least free of the 8192 it had was 3740 bytes over five and a half hours of work, the page and the recording open; 7168 leaves 2716 - and 6144 would have left 1692, under the two kilobytes a round wants, so it keeps seven. The number is this round's own: the common one
/// (round::STACK) is not touched.
const HOLDING_STACK: usize = 7168;

/// Starts the counting of the crossings and the gate, and the round that holds the draught. Call once, after the draught's own round.
///
/// A gate that will not build is written in the log and nothing more: a fan must not keep this board off the network, and the fan then moves only by hand.
pub fn start() -> Result<(), EspError> {
    fan_gate::start()?;
    firmware::task::round("holding", HOLDING_STACK, firmware::task::Core::Hardware, || {
        log::info!("holding: the draught is held once a second, by the settings of the fan's card");
        holding_round()
    })?;
    Ok(())
}

/// What the holding is DOING now, with no numbers in it: the thing a person reads a log line for. Two rounds that agree here are the same news, however far the pascals have moved between them.
#[derive(Clone, Copy, PartialEq)]
enum State {
    ByHand,
    Off,
    Purging,
    Holding,
    Blind(regulator::Blind),
    GateStopped,
    GateLost,
}

fn state_now(held: regulator::Held, carried: &regulator::Carried, asked: regulator::Asked) -> State {
    match (carried, held, asked) {
        (regulator::Carried::Stopped { .. }, ..) => State::GateStopped,
        (regulator::Carried::Lost { .. }, ..) => State::GateLost,
        (_, regulator::Held::Blind(why), _) => State::Blind(why),
        (_, _, regulator::Asked::ByHand) | (_, regulator::Held::Alone, _) => State::ByHand,
        (_, _, regulator::Asked::Stop) => State::Off,
        (_, _, regulator::Asked::Purge) => State::Purging,
        (_, _, regulator::Asked::Hold(_)) => State::Holding,
    }
}

fn holding_round() -> ! {
    let mut told: Option<State> = None;
    // the regulator reads the draught by its role, not by the sensor
    let mut draught = crate::roles::DRAUGHT.receiver().expect("a place on the draught for the regulator");
    // the curves are on the heap already (`Fan::new`), what is left is small enough for the stack
    let mut fan = Fan::new();
    loop {
        let round = between::clock::now();
        if let Some(gate) = fan_gate::take_gate() {
            fan.gate = Some(gate);
        }
        fan.second = fan_gate::second();
        fan.step(&mut draught, &mut told);
        fan.publish_and_tell();
        // the rest of the second is the desk's: a command or a question is answered as it comes
        DESK.serve_for(STEP.saturating_sub(round.elapsed()), |request| fan.answer(request));
    }
}

impl Fan {
    /// One step of the regulator on the draught the sensor said over the last second.
    fn step(&mut self, draught: &mut module::Given<'static, ::draught::Reading>, told: &mut Option<State>) {
        let asked = self.asked_to;
        // THE FIRST ROUND STARTS THE REGULATOR AT THE ANGLE THE FAN IS ALREADY HELD AT. Started at zero it would drop the fan for a moment, and on a live fire that catches or smothers the flame.
        let at = self.asked.map_or(0, |(percent, _)| percent);
        let mut holding = self.holding.take().unwrap_or_else(|| regulator::Holding::from(at as f32));
        // No second of readings yet - the sensor has been up for less than a second. Nothing is decided on a reading that does not exist; the fan stays where it is.
        let Some(said) = draught.now() else {
            self.holding = Some(holding);
            return;
        };
        let knew = holding.knows();
        // WHAT THE GATE DID WITH THE DECISION IS PART OF THE ROUND, not a line in the log: a refused angle is followed by a stop, the regulator is told its decision did not get through, and the card says which of the two happened - a fan
        // standing at zero, or a triac nobody can account for. The round is crates/regulator's, the same the stand runs; the gate is handed in, and its refusals are written down in the order it gave them, for the codes.
        let mut refusals: Vec<Refusal> = Vec::new();
        let regulator::Round { held, commanded, carried } = regulator::round(&mut holding, asked, said, STEP.as_secs_f32(), &tuning(), &mut |percent| {
            self.hold(percent).map_err(|refused| {
                refusals.push(refused.why);
                refused.words
            })
        });
        let [miss, p, i] = regulator::Parts::points(holding.parts());
        self.miss.push(miss);
        self.p_half.push(p);
        self.i_half.push(i);
        // **WHAT THE BOARD LEARNT ABOUT ITS OWN FAN, SAID OUT LOUD ONCE.** Until this line appears, the angle a start comes down to is reasoned from somebody else's fan (`regulator::A_FAN_LIKE_THIS`, measured on the owner's burner
        // 21.09) - which is a legitimate way to begin and a bad thing to pass off as knowledge. After it, the board is working from what THIS fan did on THIS chimney.
        if knew.is_none() {
            if let Some((pa, angle)) = holding.knows() {
                log::info!("the fan held {pa:.0} Pa at {angle:.0} % - from here a start comes down to what this fan does, not to what a fan like it did");
            }
        }
        let (pa, at_angle) = knobs::known(Some(&holding));
        POINT_PA.store(pa.to_bits(), Ordering::Relaxed);
        POINT_ANGLE.store(at_angle.to_bits(), Ordering::Relaxed);
        self.holding = Some(holding);
        self.said = Some((held, api::gate_of(&carried, &refusals)));
        // the three lines, one point each a second, in the units a person reads them in
        let wanted = match asked {
            regulator::Asked::Hold(pa) => pa,
            _ => 0.0,
        };
        let came_out = match said {
            ::draught::Reading::Is(pa) => pa,
            ::draught::Reading::Over => ::draught::FULL_PA as f32,
            _ => 0.0,
        };
        self.wanted.push(wanted.round() as u16);
        self.came_out.push(came_out.round().max(0.0) as u16);
        // WHAT THE FAN IS ACTUALLY HELD AT, and while a person holds it by hand that is the percent THEY set - the curve is about the fan, not about the regulator's opinion of it. AND THE SAME WHEN THE GATE REFUSED IT: the angle the
        // round decided on never reached the triac, so drawing it would draw the regulator's opinion again. `hold` sets the percent only when it succeeds, so what stands there is the last angle that was really set.
        let really = self.asked.map_or(0, |(p, _)| p) as f32;
        let by_angle = match carried {
            regulator::Carried::Out => commanded.map_or(really, |percent| percent as f32),
            _ => really,
        };
        self.by_angle.push(by_angle.round() as u16);
        // said once, when it changes: a line a second about being well is a log whose one important line goes past unseen
        let now = row_words(held, &carried, asked);
        // **THE LOG SAYS WHAT CHANGED, AND WHAT CHANGED IS THE STATE - NOT THE NUMBERS IN IT.** The words carry the pascals and the angle, and a working regulator moves both every second, so a line printed whenever the WORDS differ is a
        // line every second. The state is one of a handful of things, and that is what a person reads a log for; the numbers are on the card, and here they wait for somebody to turn the module up.
        log::debug!("holding: {now}");
        let state = state_now(held, &carried, asked);
        if *told != Some(state) {
            // A COMMAND THAT DID NOT GET THROUGH IS AN ERROR AND NOT NEWS. It is the one line in this log a person is looking for after a fire went wrong, and `info` is where it would be scrolled past.
            match matches!(carried, regulator::Carried::Out) {
                true => log::info!("holding: {now}"),
                false => log::error!("holding: {now}"),
            }
            *told = Some(state);
            // the holding row is a row of the fan's card, and its state just changed
            self.publish();
            web::announce(api::CARD);
        }
    }
}

/// What the holding says in the log: the state now and what to do about it, never what it used to do. For a person at a console - the card says it by code.
fn row_words(held: regulator::Held, carried: &regulator::Carried, asked: regulator::Asked) -> String {
    // WHAT THE GATE DID OUTRANKS WHAT THE REGULATOR WANTED. Said the other way round, the line reads "42 Pa of 50 Pa asked, at 31 % of air" while the triac is at zero or anywhere at all.
    if let Some(words) = regulator::gate_words(carried) {
        return words;
    }
    match (held, asked) {
        (regulator::Held::Blind(why), _) => regulator::words(why),
        (_, regulator::Asked::ByHand) => "by hand - the regulator is not holding anything".to_string(),
        (regulator::Held::Alone, _) => "by hand".to_string(),
        (regulator::Held::At(_), regulator::Asked::Stop) => "off".to_string(),
        (regulator::Held::At(angle), regulator::Asked::Purge) => format!("purging at {angle:.0} %"),
        // The wording, and the rule that a fault never goes where a number is expected, live with the sensor's own words in crates/draught - where they can be judged on a computer.
        (regulator::Held::At(angle), regulator::Asked::Hold(pa)) => match crate::roles::DRAUGHT.latest() {
            Some(said) => ::draught::holding_words(said, pa, angle),
            None => ::draught::holding_words_unread(pa, angle),
        },
    }
}

/// The console's `dim`: the gate's slider from the console, through the one point a person's commands pass.
fn dim(percent: u32) -> Result<String, String> {
    firmware::site::hold(GATE_ROW, percent)?;
    Ok(format!("{GATE_ROW} {percent}%"))
}

/// The console word `dim 40`: the fan held at a percent of air, `dim 0` to stop it.
fn dim_word(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    Some(module::say::one_number("dim", words, "dim 40, dim 0 to stop it").and_then(dim).map(|done| line(&done)))
}

/// The console word `zc`: what the chip holds for the zero-crossing pin (the IDF's dump, then the fields it leaves out) and the edges counted. It only reads.
fn zc_word(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    if !words.is_empty() {
        return Some(Err("zc takes no words".into()));
    }
    let (rising, falling) = fan_gate::edges_counted();
    Some(crate::hardware::zc_dump::dump(line).map(|()| {
        line(&crate::hardware::zc_dump::registers().line(rising, falling));
        // the gate that stops itself when the fronts stop (fan_gate::gate_stop): whether it stands now, and how many times the timer's peak has stopped it since the start
        line(&format!("gate: armed={} stopped={} stops={}", u8::from(fan_gate::gate_stop_armed()), u8::from(fan_gate::gate_stopped()), fan_gate::gate_stops()));
    }))
}

static VERBS: [module::Verb; 2] = [
    module::Verb { name: "dim", help: "Hold the fan at a percent of air: dim 40, dim 0 to stop it", run: dim_word },
    module::Verb { name: "zc", help: "What the chip holds for the zero-crossing pin and the edges counted: zc", run: zc_word },
];

/// Asks the fan's regulator, and waits for it.
fn ask(request: Request) -> Answer {
    DESK.ask(request, WITHIN).unwrap_or_else(|refused| Answer::Done(Err(refused.to_string())))
}

fn done(answer: Answer) -> Result<(), String> {
    match answer {
        Answer::Done(done) => done,
        Answer::Curve(_) => Err("the fan answered a command with a curve".into()),
    }
}

/// The fan by the modules' contract (crates/module).
pub struct Module;

impl module::Module for Module {
    fn card(&self) -> &'static str {
        api::CARD
    }

    fn verbs(&self) -> &'static [module::Verb] {
        &VERBS
    }

    fn takes(&self) -> &'static [&'static str] {
        &["toggle", "on", "off"]
    }

    fn published(&self) -> Option<&dyn Reads> {
        Some(&PUBLISHED)
    }

    fn is_on(&self, name: &str) -> Option<bool> {
        (name == PURGE_ROW).then(|| PURGING.load(Ordering::Relaxed))
    }

    fn settings(&self) -> Vec<&'static Setting> {
        api::SETTINGS.to_vec()
    }

    /// What turning each knob of the regulator does, worked out where the settings are asked about, on the settings of now and the point the regulator last knew.
    fn setting_values(&self, each: &mut dyn FnMut(usize, &Value)) {
        api::put_setting_values(&tuning(), point(), &mut |at, value| each(at, &value));
    }

    fn curve(&self, name: &str) -> Option<page::Curve> {
        if !CURVES.contains(&name) {
            return None;
        }
        match ask(Request::Curve(name.to_string())) {
            Answer::Curve(curve) => curve,
            Answer::Done(refused) => {
                log::warn!("fan: the curve {name} was not answered: {refused:?}");
                None
            }
        }
    }

    /// THE PURGE IS THE FAN'S, AND ITS TWO SLIDERS; every other thing is some other module's, whose own refusal then stands.
    fn order(&self, name: &str, order: module::Order) -> Option<Result<(), String>> {
        BY_HAND.contains(&name).then(|| done(ask(Request::Order(name.to_string(), order))))
    }

    /// The automaton's hand on the fan (role `fan`), carried out where a person's is.
    fn command(&self, role: &str, what: &dyn std::any::Any) -> Option<Result<(), String>> {
        (role == crate::roles::FAN.name).then(|| {
            let asked = module::take::<regulator::Asked>(what, role)?;
            done(ask(Request::Automaton(asked)))
        })
    }
}
