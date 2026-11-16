//! The automaton and the orders it carries out: the one thing every command is judged by and applied to.

use crate::{hand_over, Doing, Hands, OnChange, Order, Seen, Sound, State, Told};

/// The automaton a board runs.
///
/// **A COMMAND IS JUDGED BY THE STATE THE AUTOMATON IS IN AT THE MOMENT IT IS APPLIED, AND IS APPLIED THE MOMENT IT IS
/// JUDGED**. A press kept for the next second and judged by the card beside it acted on
/// a state that was already gone. So a press is one application of no length, taken at once: the thermostat counts it and
/// the order that carries the count is handed to [`Automaton::heat`] in the same breath, so start from a stop IS a purge
/// when the press is judged. The board keeps this under one lock with its seconds, and a person's hand on
/// an organ is judged and carried out under the same lock (`by_hand`), so nothing the automaton does can come between.
pub struct Automaton {
    burning: crate::Burning,
    /// Whether a person's hand is taken in every state (Semi) or only in a stop and a fault (Auto).
    semi: bool,
}

impl Automaton {
    pub fn new(burning: crate::Burning) -> Self {
        Self { burning, semi: false }
    }

    /// Whether the automaton runs in Semi.
    pub fn semi(&self) -> bool {
        self.semi
    }

    /// Auto or Semi, as a person switched it.
    pub fn set_semi(&mut self, semi: bool) {
        self.semi = semi;
    }

    /// The state it is in now.
    pub fn state(&self) -> State {
        self.burning.state()
    }

    /// What ends in the state it is in, and when.
    pub fn clocks(&self, sound: &crate::Sound) -> Vec<crate::Clock> {
        self.burning.clocks(sound)
    }

    /// The seconds the draught has been counted by, for the journal's line about a burning off by the draught ([`crate::Burning::draught_counts`]).
    pub fn draught_counts(&self) -> crate::DraughtCounts {
        self.burning.draught_counts()
    }

    /// An order that carries a press, judged by the state now and applied now - a second of no length, on what was SEEN last.
    pub fn heat(&mut self, order: &Order, seen: crate::Seen, sound: &crate::Sound) -> crate::Doing {
        self.burning.step(order, seen, 0.0, sound)
    }

    /// One second of SECS on ORDER.
    pub fn second(&mut self, order: &Order, seen: crate::Seen, secs: f32, sound: &crate::Sound) -> crate::Doing {
        self.burning.step(order, seen, secs, sound)
    }

    /// What a person pressed since the fire last took an order: the edge ORDER's counts make against the ones it took.
    pub fn told(&self, order: &Order) -> Told {
        self.burning.told(order)
    }

    /// Lets go of a fault of the reason WHY by the board's own rule ([`crate::Relief`]), not by a person's Stop: true when it did.
    pub fn clear_fault(&mut self, why: crate::Why) -> bool {
        self.burning.clear_fault(why)
    }

    /// Whether the auger may turn: allowed unless a person banned the feed.
    pub fn feed_allowed(&self) -> bool {
        self.burning.feed_allowed()
    }

    /// A person allows or bans the feed, applied now like a press - a second of no length - so a ban in the middle of a
    /// dose stops the auger at once rather than at the next second.
    pub fn feed(&mut self, allowed: bool, order: &Order, seen: crate::Seen, sound: &crate::Sound) -> crate::Doing {
        self.burning.allow_feed(allowed);
        self.burning.step(order, seen, 0.0, sound)
    }

    /// Whether a person may command THING by hand now.
    pub fn by_hand(&self, thing: &str) -> Result<(), String> {
        crate::by_hand(self.state(), self.feed_allowed(), self.semi, thing)
    }
}

/// The automaton with its hands on the organs: the one road its decisions take to them, and the one place that says
/// when it takes the organs back from a person's hand. The board holds one in the automaton's task; a person's own
/// command reaches the organ past it (the modules' `person`), which is why the hands pass a decision on only when it
/// changes ([`OnChange`]).
pub struct Driving<H: Hands> {
    pub automaton: Automaton,
    pub hands: OnChange<H>,
}

impl<H: Hands> Driving<H> {
    pub fn new(automaton: Automaton, organs: H) -> Self {
        Self { automaton, hands: OnChange::new(organs) }
    }

    /// One second of SECS on ORDER, handed to the organs. **A STOP TAKES THE ORGANS BACK ITSELF** wherever it is taken.
    pub fn second(&mut self, order: &Order, seen: Seen, secs: f32, sound: &Sound) -> Doing {
        let was = self.automaton.state();
        let told = self.automaton.told(order);
        let doing = self.automaton.second(order, seen, secs, sound);
        if told == Told::Stop {
            self.hands.forget();
        }
        self.drive(was, doing);
        doing
    }

    /// An order that carries a press, judged and applied now, handed to the organs. **STOP TAKES THE ORGANS BACK ITSELF**:
    /// in a burning off it changes no state, and a hand's auger switched on there would turn on into the stop.
    pub fn heat(&mut self, order: &Order, seen: Seen, sound: &Sound) -> Doing {
        let was = self.automaton.state();
        let told = self.automaton.told(order);
        let doing = self.automaton.heat(order, seen, sound);
        if told == Told::Stop {
            self.hands.forget();
        }
        self.drive(was, doing);
        doing
    }

    /// A fault let go of by the board itself: a stop, and **the organs taken back as a Stop takes them**, so the next decision reaches them whatever they were handed last.
    pub fn clear_fault(&mut self, why: crate::Why) -> bool {
        let cleared = self.automaton.clear_fault(why);
        if cleared {
            self.hands.forget();
        }
        cleared
    }

    /// The feed allowed or banned, applied now: an auger a person holds is off with the ban.
    pub fn feed(&mut self, allowed: bool, order: &Order, seen: Seen, sound: &Sound) -> Doing {
        let was = self.automaton.state();
        let doing = self.automaton.feed(allowed, order, seen, sound);
        if !allowed {
            self.hands.forget_auger();
        }
        self.drive(was, doing);
        doing
    }

    /// Auto or Semi, as a person switched it. **BACK TO AUTO, THE AUTOMATON TAKES THE ORGANS BACK**: its next second
    /// reaches every organ, whatever the hand left on.
    pub fn set_semi(&mut self, semi: bool) {
        if self.automaton.semi() && !semi {
            self.hands.forget();
        }
        self.automaton.set_semi(semi);
    }

    /// Hands DOING to the organs, taking them back from a person first where [`crate::takes_back`] says so.
    fn drive(&mut self, was: State, doing: Doing) {
        if crate::takes_back(was, doing.state) {
            self.hands.forget();
        }
        hand_over(&doing, &mut self.hands);
    }
}
