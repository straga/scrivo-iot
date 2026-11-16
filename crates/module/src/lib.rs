//! The contract of a module (the concept of modules): what it shows, what it can do, its settings and
//! its task. A device lists its modules once, and everything the page and the console ask of the device is
//! answered by walking that list - as ESPHome's application walks its registered components.
//!
//! THE LIST IS THE ONLY PLACE A MODULE IS NAMED. Without it the device would stitch its modules by hand, one chain
//! per point of the contract - an `or_else` of curves, a `match` of switches, a `.chain` of settings - and a new
//! module would be an edit in seven places of main.rs.
//!
//! The hardware half of a module - its pins, its bus, its round - is started by the device, which is the one that
//! hands pins out; the contract is what the module answers once it runs.

pub mod card;
/// The round that reads a sensor on the board: its task, its period, its desk; built for the board only.
#[cfg(target_os = "espidf")]
pub mod polling;
pub mod retry;
pub mod sensor;

use std::any::Any;
use between::Lock;
use embassy_sync::watch::{self, Watch};

use page::dictionary::Value;
use page::Curve;
use settings::Setting;

/// One module of a device. Every answer but `card` has a default of "nothing of mine", so a module writes only what it has.
pub trait Module: Sync {
    /// The card its rows and its settings lie in on the device's page.
    fn card(&self) -> &'static str;

    /// Where its owner publishes its card of values (`card`) - contract 3. Read by whoever wants them; the module's
    /// own code does not run for the reader.
    fn published(&self) -> Option<&dyn card::Reads> {
        None
    }

    /// What its settings that say something now say, by address in the card of settings (a divider's zero on the
    /// pin, the order of two thresholds) - contract 3; handed over when the settings are asked about.
    fn setting_values(&self, _each: &mut dyn FnMut(usize, &Value)) {}

    /// Whether its thing NAME that turns on and off is on now; None - NAME is not its own, or does not turn. What the console answers after a command.
    fn is_on(&self, _name: &str) -> Option<bool> {
        None
    }

    /// Its settings, in the order its card shows them.
    fn settings(&self) -> Vec<&'static Setting> {
        Vec::new()
    }

    /// The curve called NAME; None for a name that is not its own.
    fn curve(&self, _name: &str) -> Option<Curve> {
        None
    }

    /// The console words that are the module's own, as `dim` or `reset` would be: a board without the module does not
    /// have them, and on it the word is unknown. Several modules may use one name; the first whose `run` takes the words answers.
    fn verbs(&self) -> &'static [Verb] {
        &[]
    }

    /// Which of the console's own words that not every board has - `toggle`, `on`, `off`, `press` - this module carries out. A
    /// board where no module does lists none of them.
    fn takes(&self) -> &'static [&'static str] {
        &[]
    }

    /// Its lines, one each, for a console word that asks the modules for them (a device's word that reads its sensors, which [`lines`] gathers).
    fn lines(&self, _line: &mut dyn FnMut(&str)) -> Result<(), String> {
        Ok(())
    }

    /// A person's command to NAME, whoever's thing NAME is, passing through this module: it judges ORDER by what it is
    /// doing now and, if it lets it through, carries it out itself - [`carry`] over MODULES - or refuses in words. None -
    /// NAME is not one it keeps.
    ///
    /// **THE JUDGING AND THE CARRYING HAPPEN IN THE KEEPER'S OWN TASK**, one command at a time: nothing the keeper does
    /// can come between its verdict and the organ, because while it waits for the organ it does nothing else.
    fn through(&self, _modules: &'static Modules, _name: &str, _order: Order) -> Option<Result<(), String>> {
        None
    }

    /// Carries out ORDER on its thing NAME. None - NAME is not its own, and the next module is asked; Some(Err) - its
    /// own, and refused, which stands.
    ///
    /// **ITS OWNER CARRIES IT OUT, ONE COMMAND AT A TIME**: a toggle reads what the thing is and
    /// switches it with no other command in between, because only the owner writes it.
    fn order(&self, _name: &str, _order: Order) -> Option<Result<(), String>> {
        None
    }

    /// Carries out the command WHAT for ROLE - taken with `take`, which refuses a command of another type. None -
    /// the module plays no such role; Some(Err) - it does, and refused, in words.
    fn command(&self, _role: &str, _what: &dyn Any) -> Option<Result<(), String>> {
        None
    }
}

/// A quantity a consumer asks for by what it IS - the water, a pressure - and not by which module measures it:
/// the concept's "a value comes by its role". T is what it is given as, the same
/// whichever module gives it, so the source can change and the consumer does not.
///
/// **A SENSOR'S ROLE KNOWS HOW OLD WHAT IT HOLDS IS.** Its round publishes every period, and a round that stopped
/// publishing left its last number standing as if it were read now: 27.09 a sensor's round stood still at the
/// switching of an organ, and a regulator held its last number for 34 s and drove its output to the ceiling. So a
/// role played by a sensor is made with the period of its round, and past three periods (at least 3 s) it says nothing
/// - the same "cannot say" every consumer already acts on safely. A role a deciding module plays (its hand on an organ)
/// publishes when it decides, not on a period, and never grows old.
pub struct Role<T: Clone> {
    pub name: &'static str,
    /// The period its player's round publishes on; None - a role played by decisions, which does not grow old.
    every: Option<fn() -> core::time::Duration>,
    /// The quantity as the module playing the role last published it, and when it last published a number; None in it -
    /// it cannot say now.
    now: Watch<Lock, (Option<T>, Moment), { card::READERS }>,
}

/// How long a sensor's role has not been published, judged against the period of its round.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Quiet {
    /// Published within three periods of its round (at least 3 s): what it holds is a reading.
    Speaks,
    /// Silent past three periods: what it holds is not given out. The span since it last published a number - or since
    /// the board started, when it never did.
    Silent(core::time::Duration),
    /// Silent past ten periods (at least 30 s): long enough that a device working without it has to stop.
    TooLong(core::time::Duration),
}

impl<T: Clone> Role<T> {
    /// A role played by decisions - a deciding module's hand on an organ: it holds what was last published
    /// for as long as nobody publishes again.
    pub const fn new(name: &'static str) -> Self {
        Self { name, every: None, now: Watch::new() }
    }

    /// A role played by a sensor whose round publishes every EVERY (asked afresh, as a period that is a setting is).
    pub const fn sensor(name: &'static str, every: fn() -> core::time::Duration) -> Self {
        Self { name, every: Some(every), now: Watch::new() }
    }

    /// What the module playing the role says now - its reading, or None when it cannot say. Called by its owner every
    /// round; the last one stands, and nobody reads the module for it.
    pub fn publish(&self, now: Option<T>) {
        self.publish_at(now, between::clock::now());
    }

    /// [`Role::publish`] at the moment AT of the board's steady clock.
    ///
    /// **THE MOMENT KEPT IS THE LAST NUMBER'S, NOT THE LAST PUBLISHING'S**: a round that goes on and
    /// publishes "cannot say" every second is as silent as one that stopped, and counted from its publishing it would
    /// never fall silent at all. A value a sensor says in its own words - a reading under its own zero - is a number
    /// of its kind and moves the moment like any other.
    pub fn publish_at(&self, now: Option<T>, at: Moment) {
        let read_at = match now {
            Some(_) => at,
            None => self.now.try_get().map(|(_, when)| when).unwrap_or_default(),
        };
        self.now.sender().send((now, read_at));
    }

    /// The quantity as last published, for a reader without a place of its own - one called from more than one task,
    /// like a card put together wherever its values move. A consumer that runs on its own takes a [`Role::receiver`].
    /// None past the silence of a sensor's role, as [`Given::now`].
    pub fn latest(&self) -> Option<T> {
        self.latest_at(between::clock::now())
    }

    /// [`Role::latest`] at the moment AT.
    pub fn latest_at(&self, at: Moment) -> Option<T> {
        given(self.every, self.now.try_get(), at)
    }

    /// How long the role has been silent at the moment AT, for a reader without a place of its own.
    pub fn quiet_at(&self, at: Moment) -> Quiet {
        judge(self.every, self.now.try_get().map(|(_, when)| when), at)
    }

    /// A consumer's place on the role, taken once when it starts; None when every place is taken.
    pub fn receiver(&self) -> Option<Given<'_, T>> {
        self.now.receiver().map(|place| Given { every: self.every, place })
    }
}

/// A consumer's hold on a role: the quantity as last published, read when the consumer wants it.
pub struct Given<'a, T: Clone> {
    every: Option<fn() -> core::time::Duration>,
    place: watch::Receiver<'a, Lock, (Option<T>, Moment), { card::READERS }>,
}

impl<T: Clone> Given<'_, T> {
    /// The quantity now; None - nothing has been published yet, the module playing the role cannot say, or a sensor's
    /// role has been silent past three periods of its round. Never a zero, and never the last number of a round that
    /// stopped.
    pub fn now(&mut self) -> Option<T> {
        self.now_at(between::clock::now())
    }

    /// [`Given::now`] at the moment AT of the board's steady clock.
    pub fn now_at(&mut self, at: Moment) -> Option<T> {
        given(self.every, self.place.try_get(), at)
    }

    /// How long the role has been silent now.
    pub fn quiet(&mut self) -> Quiet {
        self.quiet_at(between::clock::now())
    }

    /// [`Given::quiet`] at the moment AT.
    pub fn quiet_at(&mut self, at: Moment) -> Quiet {
        judge(self.every, self.place.try_get().map(|(_, when)| when), at)
    }
}

use between::clock::Moment;

/// What a role holding PUBLISHED gives out at AT: the quantity, unless a sensor's role has fallen silent.
fn given<T>(every: Option<fn() -> core::time::Duration>, published: Option<(Option<T>, Moment)>, at: Moment) -> Option<T> {
    let (now, when) = published?;
    match judge(every, Some(when), at) {
        Quiet::Speaks => now,
        Quiet::Silent(_) | Quiet::TooLong(_) => None,
    }
}

/// How long a role with the period EVERY, last given a number WHEN, has been silent at AT. A role never published has been
/// silent since the board started: a round that never began is the longest silence there is.
fn judge(every: Option<fn() -> core::time::Duration>, when: Option<Moment>, at: Moment) -> Quiet {
    let Some(every) = every else { return Quiet::Speaks };
    let when = when.unwrap_or_default();
    quiet_after(every(), at.since(when), between::clock::padded_between(when, at))
}

/// What a role of the period PERIOD that has been silent for SILENT is, SILENT being measured by the board's clock and
/// PADDED of it being leaps of the chip's counter, which the clock took as time (each adds its PAD) and a
/// reading a second apart did not live through: a sensor read once a second, silent for a second, must not fall silent
/// because a leap fell at the end of its period.
pub fn quiet_after(period: core::time::Duration, silent: core::time::Duration, padded: core::time::Duration) -> Quiet {
    use core::time::Duration;
    let silent = silent.saturating_sub(padded);
    // three missed periods are not chance, and ten are long enough that a device run without the reading must stop;
    // a round of a tenth of a second is still given seconds, which is how long a regulator or a deciding module takes
    if silent > (period * 10).max(Duration::from_secs(30)) {
        Quiet::TooLong(silent)
    } else if silent > (period * 3).max(Duration::from_secs(3)) {
        Quiet::Silent(silent)
    } else {
        Quiet::Speaks
    }
}

/// The command WHAT for the role called ROLE, as the type the module carries it out as.
pub fn take<T: Copy + 'static>(what: &dyn Any, role: &str) -> Result<T, String> {
    what.downcast_ref::<T>().copied().ok_or_else(|| {
        format!(
            "{role} is carried out as {} and was commanded as something else - the module and its caller disagree",
            std::any::type_name::<T>()
        )
    })
}

/// Hands WHAT to the first module that plays ROLE; its refusal stands. Nobody plays it - refused in words naming
/// the role.
pub fn command<T: Clone + 'static>(modules: &Modules, role: &Role<T>, what: T) -> Result<(), String> {
    for module in modules {
        if let Some(done) = module.command(role.name, &what) {
            return done;
        }
    }
    Err(format!("nothing on this board carries out {}", role.name))
}

/// A console word a module adds: its name, one line of help for `help`, and what it does with the words after it.
pub struct Verb {
    pub name: &'static str,
    pub help: &'static str,
    /// Carries the word out for WORDS after it, its lines handed to `line`. None - nothing of this module takes WORDS after the
    /// name, and the next module that uses the name is asked; Some(Err) - it does, and refuses, in words.
    pub run: fn(words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>>,
}

/// The console words of every module, each name once, with the help of the first module that has it.
pub fn verbs(modules: &Modules) -> Vec<&'static Verb> {
    let mut all: Vec<&'static Verb> = Vec::new();
    for verb in modules.iter().flat_map(|m| m.verbs()) {
        if !all.iter().any(|known| known.name == verb.name) {
            all.push(verb);
        }
    }
    all
}

/// The console word NAME with WORDS after it: the first module that uses the name and takes the words answers. None - no
/// module has the name, or none takes the words.
pub fn verb(modules: &Modules, name: &str, words: &[&str], line: &mut dyn FnMut(&str)) -> Option<Result<(), String>> {
    modules.iter().flat_map(|m| m.verbs()).filter(|v| v.name == name).find_map(|v| (v.run)(words, line))
}

/// Whether any module carries out the console's word NAME that not every board has (see [`Module::takes`]).
pub fn takes(modules: &Modules, name: &str) -> bool {
    modules.iter().any(|m| m.takes().contains(&name))
}

/// A device's modules, in the order its page draws their cards.
pub type Modules = [&'static dyn Module];

/// The device's own cards, in the order of its modules, each once. The parts every board has (`page::SYSTEM`,
/// `page::SETTINGS`, `page::OTHER`) are not the device's own and are left out.
pub fn cards(modules: &Modules) -> Vec<&'static str> {
    let mut cards: Vec<&'static str> = Vec::new();
    for card in modules.iter().map(|m| m.card()) {
        let reserved = [page::SYSTEM, page::SETTINGS, page::OTHER].contains(&card);
        if !reserved && !cards.contains(&card) {
            cards.push(card);
        }
    }
    cards
}

/// What the modules' settings say now, by address, from every module.
pub fn setting_values(modules: &Modules, each: &mut dyn FnMut(usize, &Value)) {
    for module in modules {
        module.setting_values(each);
    }
}

/// Whether the thing NAME that turns on and off is on now, from the module whose thing it is.
pub fn is_on(modules: &Modules, name: &str) -> Option<bool> {
    modules.iter().find_map(|m| m.is_on(name))
}

/// The settings of every module, in the order of the modules.
pub fn settings(modules: &Modules) -> Vec<&'static Setting> {
    modules.iter().flat_map(|m| m.settings()).collect()
}

/// The curve called NAME: the first module that knows it answers.
pub fn curve(modules: &Modules, name: &str) -> Option<Curve> {
    modules.iter().find_map(|m| m.curve(name))
}

/// The lines of every module for a console word that gathers them, in the order of the modules; the first refusal ends the answer.
pub fn lines(modules: &Modules, line: &mut dyn FnMut(&str)) -> Result<(), String> {
    modules.iter().try_for_each(|m| m.lines(line))
}

/// A person's command to a thing, as the page, the web and the console send it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Press,
    /// On or off - the state asked for, whatever it was.
    Switch(bool),
    /// The other side of what it is now.
    Toggle,
    /// A number, from a slider.
    Hold(u32),
}

/// A person's command to NAME: through the module that keeps NAME, which judges it and carries it out, or carried out
/// at once when nobody keeps it.
///
/// **EVERY COMMAND OF A PERSON COMES HERE**: the page, the web routes and the console all reach
/// the organs through `press`, `switch`, `toggle` and `hold` below, and nothing else of a person's does. What a module
/// does for itself - a deciding module driving its organs - goes by role (`command`) and does not pass here.
pub fn person(modules: &'static Modules, name: &str, order: Order) -> Result<(), String> {
    for module in modules {
        if let Some(done) = module.through(modules, name, order) {
            return done;
        }
    }
    carry(modules, name, order)
}

/// ORDER carried out on NAME by the module whose thing it is - for a keeper that has let a person's command through.
pub fn carry(modules: &Modules, name: &str, order: Order) -> Result<(), String> {
    modules.iter().find_map(|m| m.order(name, order)).unwrap_or_else(|| Err(nobody(name, order)))
}

/// The refusal of a command nobody on this board carries out, naming the thing.
fn nobody(name: &str, order: Order) -> String {
    match order {
        Order::Press => format!("nothing on this board presses {name}"),
        Order::Switch(_) => format!("nothing on this board turns {name} on or off"),
        Order::Toggle => format!("nothing on this board switches {name} on and off"),
        Order::Hold(_) => format!("this board holds nothing at a number, and no {name}"),
    }
}

/// Switches NAME over: the other side of what it is now, decided by its owner.
pub fn toggle(modules: &'static Modules, name: &str) -> Result<(), String> {
    person(modules, name, Order::Toggle)
}

/// Presses the button NAME of the module whose button it is.
pub fn press(modules: &'static Modules, name: &str) -> Result<(), String> {
    person(modules, name, Order::Press)
}

/// Turns NAME on or off by the module whose thing it is.
pub fn switch(modules: &'static Modules, name: &str, on: bool) -> Result<(), String> {
    person(modules, name, Order::Switch(on))
}

/// Sets NAME to a number by the module whose thing it is.
pub fn hold(modules: &'static Modules, name: &str, value: u32) -> Result<(), String> {
    person(modules, name, Order::Hold(value))
}

/// What a module's console word says to a person, written once for the words that take a number or ask before they do.
pub mod say {
    /// The one whole number a word takes: `dim 40`. No word, more than one, or a word that is no number is refused in words
    /// that say how the word is written (USAGE).
    pub fn one_number(name: &str, words: &[&str], usage: &str) -> Result<u32, String> {
        match words {
            [] => Err(format!("{name} takes a number: {usage}")),
            [word] => word.parse().map_err(|_| format!("'{word}' is no whole number: {usage}")),
            [_, extra, ..] => Err(format!("{name} takes one number, and '{extra}' is a second: {usage}")),
        }
    }

    /// Whether a word that asks before it zeroes anything was confirmed: `reset counter yes`. No word asks (Ok(false)); `yes` is
    /// the confirmation; anything else is refused and nothing is done.
    pub fn confirmed(rest: &[&str]) -> Result<bool, String> {
        match rest {
            [] => Ok(false),
            ["yes"] => Ok(true),
            [other, ..] => Err(format!("'{other}' is not yes; nothing zeroed")),
        }
    }
}
