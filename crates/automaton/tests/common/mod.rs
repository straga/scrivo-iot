//! The rows of the fire as they were written when the fire also decided whether heat was needed (before 6.10): the numbers of the fire
//! and the thermostat's in ONE record, the readings of every sensor in one `Seen`, a button as a `Told`, and a burner that takes all of
//! it and steps. **THE ROWS ARE THE SAME AND SO ARE THEIR EXPECTATIONS**: what changed is the machine under them - the fire, and the
//! real `thermostat` that orders it - so a row that passed on the fire alone and passes on the pair says the decisions moved
//! without moving a second. Nothing here decides anything: the water's decisions are `thermostat`'s own, and the fire is the crate's.
#![allow(dead_code)]

pub use thermostat::{Hold, Press};

use automaton::{Doing, Heard, Refused, Told};
use flame::Flame;
use thermostat::{Temperatures, Thermostat};

/// The fire's numbers and the thermostat's, as one record the rows write.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tuning {
    pub purge_before_s: f32,
    pub purge_after_s: f32,
    pub igniter_s: f32,
    pub wait_for_flame_s: f32,
    pub tries: u32,
    pub dose_after_first: f32,
    pub rise_s: f32,
    pub flame_lost_s: f32,
    pub smoulder_s: f32,
    pub burn_off_s: f32,
    pub burn_out_min_s: f32,
    pub pump_on_c: f32,
    pub flue_holds_c: f32,
    pub fire_numbers: FireNumbers,
    pub want_c: f32,
    pub hysteresis_c: f32,
    pub hold: Hold,
    pub power: automaton::Step,
    pub pump_off_below_c: f32,
    pub too_hot_c: f32,
    pub alarm_c: f32,
    pub sure_s: f32,
    pub relight_s: [f32; 3],
    pub relight_hot_c: f32,
    pub spin_up_s: f32,
    pub low_pressure_s: f32,
    pub kindling: automaton::Lighting,
    pub step: automaton::Step,
    pub smoulder: automaton::Step,
}

/// The numbers of the flue and the flame the card gives and the rows of 6.6 change one at a time: a cold flue, a fall and its window, the hold of a flame.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FireNumbers {
    pub flue_cold_c: f32,
    pub flue_fall_c: f32,
    pub flue_fall_s: f32,
    pub lit_s: f32,
}

impl FireNumbers {
    /// The card's own: 70 °C cold, a fall of 10 °C over 10 minutes, a flame held 5 s (27.09).
    pub const CARD: FireNumbers = FireNumbers { flue_cold_c: 70.0, flue_fall_c: 10.0, flue_fall_s: 600.0, lit_s: 5.0 };
}

/// The two doors' answers together: the fire's sound numbers and the thermostat's.
#[derive(Clone, Copy, Debug)]
pub struct Sound {
    pub fire: automaton::Sound,
    pub heat: thermostat::Sound,
    numbers: (automaton::Tuning, thermostat::Tuning),
}

impl Sound {
    /// Both doors' answers for the fire's numbers and the thermostat's: the first refusal of either.
    pub fn of(fire: automaton::Tuning, heat: thermostat::Tuning) -> Result<Sound, Refused> {
        match (fire.sound(), heat.sound(fire.too_hot_c)) {
            (Ok(sound), Ok(heat_sound)) => Ok(Sound { fire: sound, heat: heat_sound, numbers: (fire, heat) }),
            (Err(why), _) | (_, Err(why)) => Err(why),
        }
    }

    /// The numbers inside, both doors' together.
    pub fn numbers(&self) -> &(automaton::Tuning, thermostat::Tuning) {
        &self.numbers
    }
}

impl Tuning {
    fn fire(self) -> automaton::Tuning {
        automaton::Tuning {
            purge_before_s: self.purge_before_s,
            purge_after_s: self.purge_after_s,
            igniter_s: self.igniter_s,
            wait_for_flame_s: self.wait_for_flame_s,
            tries: self.tries,
            dose_after_first: self.dose_after_first,
            rise_s: self.rise_s,
            flame_lost_s: self.flame_lost_s,
            smoulder_s: self.smoulder_s,
            burn_off_s: self.burn_off_s,
            burn_out_min_s: self.burn_out_min_s,
            pump_on_c: self.pump_on_c,
            flue_holds_c: self.flue_holds_c,
            flue_cold_c: self.fire_numbers.flue_cold_c,
            flue_fall_c: self.fire_numbers.flue_fall_c,
            flue_fall_s: self.fire_numbers.flue_fall_s,
            lit_s: self.fire_numbers.lit_s,
            pump_off_below_c: self.pump_off_below_c,
            too_hot_c: self.too_hot_c,
            alarm_c: self.alarm_c,
            sure_s: self.sure_s,
            relight_s: self.relight_s,
            relight_hot_c: self.relight_hot_c,
            spin_up_s: self.spin_up_s,
            low_pressure_s: self.low_pressure_s,
            kindling: self.kindling,
            smoulder: self.smoulder,
        }
    }

    fn heat(self, lockout: thermostat::Lockout) -> thermostat::Tuning {
        thermostat::Tuning { hold: self.hold, want_c: self.want_c, hysteresis_c: self.hysteresis_c, step: self.step, mid: self.step, small: self.step, mid_c: 8.0, small_c: 3.0, power: self.power, lockout }
    }

    /// Both doors, the thermostat's asked first: the order the fields stood in the one record the rows were written on.
    pub fn sound(self) -> Result<Sound, Refused> {
        self.sound_with(thermostat::Lockout::default())
    }

    /// The same, for a thermostat that waits LOCKOUT before it lights the fire again (the pause after the fire went out).
    pub fn sound_with(self, lockout: thermostat::Lockout) -> Result<Sound, Refused> {
        Sound::of(self.fire(), self.heat(lockout))
    }
}

/// Which sensors have gone quiet, as the rows write it: every one of the five.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Silence {
    pub draught: Heard,
    pub flame: Heard,
    pub flue: Heard,
    pub water: Heard,
    pub return_c: Heard,
    pub fronts_s: Option<u32>,
}

impl Silence {
    pub const HEARD: Silence = Silence { draught: Heard::Speaks, flame: Heard::Speaks, flue: Heard::Speaks, water: Heard::Speaks, return_c: Heard::Speaks, fronts_s: None };
}

/// What the board knows this second, as the rows write it: the fire's readings and the thermostat's.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Seen {
    pub flame: Flame,
    pub boiler_c: Option<f32>,
    pub return_c: Option<f32>,
    pub flue_c: Option<f32>,
    pub draught_pa: Option<f32>,
    pub draught_low: bool,
    pub gate: u32,
    pub silence: Silence,
}

impl Seen {
    fn fire(&self) -> automaton::Seen {
        automaton::Seen {
            flame: self.flame,
            boiler_c: self.boiler_c,
            flue_c: self.flue_c,
            draught_pa: self.draught_pa,
            draught_low: self.draught_low,
            gate: self.gate,
            silence: automaton::Silence { draught: self.silence.draught, flame: self.silence.flame, flue: self.silence.flue, water: self.silence.water, fronts_s: self.silence.fronts_s },
        }
    }

    fn readings(&self) -> Temperatures {
        Temperatures { boiler_c: self.boiler_c, return_c: self.return_c, return_heard: self.silence.return_c, asked: None, room_c: None, room_target_c: None, fire: thermostat::Fire::default() }
    }
}

/// The fire and the thermostat that orders it, stepped together: what a board runs.
pub struct Burning {
    fire: automaton::Burning,
    heater: Thermostat,
    /// What the house says to the boiler from now on: none - nobody plays the role.
    asked: Option<demand::Asked>,
    /// Whether the thermostat waits for the first round of the sensors, as the board's does for its first seconds: then it is told, as the board tells it, whether the automaton stands in Stop.
    waiting: bool,
    /// The room as its role reads it and the target the house holds it at: none, none - there is no room.
    room: (Option<f32>, Option<f32>),
    /// The last Light the thermostat refused, with its numbers; none when the last one was taken.
    refused: Option<thermostat::Refusal>,
}

impl Burning {
    /// A board that has come up remembering nothing: stopped, and not asking for anything.
    pub fn new() -> Self {
        Self { fire: automaton::Burning::new(), heater: Thermostat::new(), asked: None, waiting: false, room: (None, None), refused: None }
    }

    /// A board that has come up and remembers WAS, looking at what the sensors say now.
    pub fn resumed(was: automaton::State, seen: &Seen, t: &Sound) -> Self {
        Self { fire: automaton::Burning::resumed(was, &seen.fire(), &t.fire), heater: Thermostat::new(), asked: None, waiting: false, room: (None, None), refused: None }
    }

    /// A board that has come up remembering WAS and found KEPT - the word the flash holds of the switch, none when nothing is - in the thermostat's memory: the switch the person left, and no press of its own.
    pub fn resumed_kept(was: automaton::State, seen: &Seen, t: &Sound, kept: Option<&str>) -> Self {
        Self { fire: automaton::Burning::resumed(was, &seen.fire(), &t.fire), heater: Thermostat::restored(thermostat::kept_switch(kept)), asked: None, waiting: false, room: (None, None), refused: None }
    }

    /// One second: what a person pressed, what the board sees, how long since the last step. The press is the thermostat's, counted
    /// before the order of the second is made, as the board does it.
    pub fn step(&mut self, told: Told, seen: Seen, secs: f32, t: &Sound) -> Doing {
        // the board tells the thermostat whether the automaton stands in Stop; the rows tell it only while they ask for the barrier of the first round, so that the others keep the readings they always had
        let fire = if self.waiting { thermostat::Fire { out: self.fire.state() == automaton::State::Stop, steady: seen.flame == Flame::Steady, elapsed_s: secs } } else { thermostat::Fire::default() };
        let readings = Temperatures { asked: self.asked, room_c: self.room.0, room_target_c: self.room.1, fire, ..seen.readings() };
        self.heater.faulted(matches!(self.fire.state(), automaton::State::Fault(_)));
        match told {
            Told::Start => {
                let _ = self.heater.press(Press::Start);
            }
            Told::Stop => {
                let _ = self.heater.press(Press::Stop);
            }
            // the board's Light is judged against the readings of this moment: a refused one is not counted and the fire is not asked
            Told::Light => self.refused = self.heater.light(&readings, &t.heat).err(),
            Told::Nothing => {}
        }
        let order = self.heater.order(&readings, &t.heat);
        self.fire.step(&order, seen.fire(), secs, &t.fire)
    }

    /// The thermostat waits for the first round of the sensors, as the board's does in its first seconds.
    pub fn wait_for_first_round(&mut self) {
        self.heater = std::mem::take(&mut self.heater).waiting_for_first_round();
        self.waiting = true;
    }

    /// The first round of the sensors is over.
    pub fn first_round_done(&mut self) {
        self.heater.first_round_done();
    }

    /// What the house says to the boiler from the next second on: none - nobody plays the role.
    pub fn ask(&mut self, asked: Option<demand::Asked>) {
        self.asked = asked;
    }

    /// The room as its role reads it from the next second on, and the target the house holds it at: none, none - there is no room, and the water is what a cycle is carried by.
    pub fn room(&mut self, room_c: Option<f32>, target_c: Option<f32>) {
        self.room = (room_c, target_c);
    }

    /// The last Light the thermostat refused: its numbers; none when the last was taken or none has been pressed.
    pub fn refused(&self) -> Option<thermostat::Refusal> {
        self.refused
    }

    /// The manual cycle that is going and why the last one ended (read once), as the thermostat says them.
    pub fn cycle(&self) -> Option<thermostat::Cycle> {
        self.heater.cycle()
    }

    pub fn ended(&mut self) -> Option<thermostat::Ended> {
        self.heater.ended()
    }

    pub fn state(&self) -> automaton::State {
        self.fire.state()
    }

    /// A fault of the reason WHY let go of by the board itself, not by a person's Stop: the thermostat's switch is not touched.
    pub fn clear_fault(&mut self, why: automaton::Why) -> bool {
        self.fire.clear_fault(why)
    }

    /// Whether a person asked for heat and has not asked it away: the thermostat's switch.
    pub fn wanted(&self) -> bool {
        self.heater.switch() == automaton::Switch::On
    }

    pub fn clocks(&self, t: &Sound) -> Vec<automaton::Clock> {
        self.fire.clocks(&t.fire)
    }

    pub fn feed_allowed(&self) -> bool {
        self.fire.feed_allowed()
    }

    pub fn allow_feed(&mut self, allowed: bool) {
        self.fire.allow_feed(allowed);
    }
}

/// The automaton a board runs, with the thermostat that orders it: a press is the thermostat's, counted and handed to the fire in the same breath.
pub struct Automaton {
    inner: automaton::Automaton,
    heater: Thermostat,
}

impl Automaton {
    pub fn new(burning: Burning) -> Self {
        Self { inner: automaton::Automaton::new(burning.fire), heater: burning.heater }
    }

    fn order(&mut self, seen: &Seen, t: &Sound) -> automaton::Order {
        self.heater.order(&seen.readings(), &t.heat)
    }

    /// A press of a button, judged by the state now and applied now - a second of no length.
    pub fn press(&mut self, press: Press, seen: Seen, t: &Sound) -> Doing {
        self.heater.press(press);
        let order = self.order(&seen, t);
        self.inner.heat(&order, seen.fire(), &t.fire)
    }

    /// One second of SECS with nobody pressing anything.
    pub fn second(&mut self, seen: Seen, secs: f32, t: &Sound) -> Doing {
        let order = self.order(&seen, t);
        self.inner.second(&order, seen.fire(), secs, &t.fire)
    }

    pub fn feed(&mut self, allowed: bool, seen: Seen, t: &Sound) -> Doing {
        let order = self.order(&seen, t);
        self.inner.feed(allowed, &order, seen.fire(), &t.fire)
    }

    pub fn state(&self) -> automaton::State {
        self.inner.state()
    }

    pub fn feed_allowed(&self) -> bool {
        self.inner.feed_allowed()
    }

    pub fn semi(&self) -> bool {
        self.inner.semi()
    }

    pub fn clocks(&self, t: &Sound) -> Vec<automaton::Clock> {
        self.inner.clocks(&t.fire)
    }

    pub fn by_hand(&self, thing: &str) -> Result<(), String> {
        self.inner.by_hand(thing)
    }
}

/// The automaton with its hands on the organs, and the thermostat that orders it: what a board runs.
pub struct Driving<H: automaton::Hands> {
    inner: automaton::Driving<H>,
    heater: Thermostat,
}

impl<H: automaton::Hands> Driving<H> {
    pub fn new(automaton: Automaton, organs: H) -> Self {
        Self { inner: automaton::Driving::new(automaton.inner, organs), heater: automaton.heater }
    }

    /// A press, judged and applied now, handed to the organs.
    pub fn press(&mut self, press: Press, seen: Seen, t: &Sound) -> Doing {
        self.heater.press(press);
        let order = self.heater.order(&seen.readings(), &t.heat);
        self.inner.heat(&order, seen.fire(), &t.fire)
    }

    /// One second of SECS, handed to the organs.
    pub fn second(&mut self, seen: Seen, secs: f32, t: &Sound) -> Doing {
        let order = self.heater.order(&seen.readings(), &t.heat);
        self.inner.second(&order, seen.fire(), secs, &t.fire)
    }

    pub fn feed(&mut self, allowed: bool, seen: Seen, t: &Sound) -> Doing {
        let order = self.heater.order(&seen.readings(), &t.heat);
        self.inner.feed(allowed, &order, seen.fire(), &t.fire)
    }
}

impl<H: automaton::Hands> std::ops::Deref for Driving<H> {
    type Target = automaton::Driving<H>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H: automaton::Hands> std::ops::DerefMut for Driving<H> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
