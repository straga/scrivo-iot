//! The one road from what the automaton decides to the organs that carry it out.
//!
//! **THE AUTOMATON SWITCHES THE ORGANS FOR REAL, ALWAYS**. A mode where the fan turned and
//! the relays did not was not asked for and confused: nothing on the scheme came on.

use crate::{Asked, Doing, Feeding, State};

/// The organs the automaton commands, by the names the device's card and the page's scheme know them by.
///
/// **ONE LIST, AND IT IS WHAT [`hand_over`] REACHES** (a row in `tests/board.rs` holds the two together). The device
/// sends it on the card, and the page checks its scheme against it: an organ the automaton commands and the scheme
/// says nothing of is refused by name - a scheme on which something is "not seen" does not pass.
pub const ORGANS: [&str; 4] = ["auger", "igniter", "pump", "fan"];

/// The organs as the automaton reaches them: the device's half, the relays and the regulator behind them.
pub trait Hands {
    /// The auger is given a plan as a COMMAND: the decision is new, or the automaton takes the organ back from a hand. The relay's task follows the plan from now and puts the relay on it whatever a hand left it in.
    fn auger(&mut self, feeding: Feeding);
    /// The auger is given a plan for the decision that stands: **THE AUTOMATON BUILDS ONE EVERY SECOND** from its own count, and the relay's task follows the last for a while and then leaves the relay open. It takes nothing back from a hand.
    fn auger_still(&mut self, feeding: Feeding);
    fn igniter(&mut self, on: bool);
    fn fan(&mut self, asked: Asked);
    fn pump(&mut self, on: bool);
}

/// Hands one second of DOING to the organs: every one of them, every second.
pub fn hand_over(doing: &Doing, hands: &mut dyn Hands) {
    hands.auger(doing.feeding);
    hands.igniter(doing.igniter);
    hands.pump(doing.pump);
    hands.fan(doing.fan);
}

/// Whether a person may command THING by hand while the automaton is in STATE, with the feed FEED_ALLOWED or banned, in
/// Semi or in Auto: in a stop and in a fault the hand is free; in Auto every other state is the automaton's, and a person
/// is told what to switch. **IN SEMI THE HAND IS FREE IN EVERY STATE**:
/// "in the automaton I cannot take control of anything" - the automaton runs the burner as in Auto, and what the
/// hand switched holds until the automaton decides that organ anew ([`OnChange`]).
///
/// **A BANNED FEED IS BANNED TO THE HAND TOO**: the ban stands for every road to
/// the auger, and a person who wants to pour by hand allows the feed first. The auger is refused both ways - the ban
/// has already turned it off.
pub fn by_hand(state: State, feed_allowed: bool, semi: bool, thing: &str) -> Result<(), String> {
    if !semi && !hand_is_free(state) {
        return Err(format!(
            "the automaton is running the burner ({}) - switch Semi on the scheme to take the {thing} by hand, or press stop and wait for the stop",
            crate::stage(state)
        ));
    }
    match feed_allowed || thing != "auger" {
        true => Ok(()),
        false => Err("the feed is banned - press Allow feed, then switch the auger by hand".into()),
    }
}

/// Whether the automaton takes the organs back from a person's hand at a change from WAS to NOW: it has left a stop or
/// a fault, where the organs were a person's, or it has begun to burn off.
///
/// **STOP IS ABOVE THE HAND IN BOTH MODES**: a Stop, and every failure, goes through
/// burning off, and an auger or a plug the hand left on in Semi would otherwise run on through it - the automaton says
/// "off" there as it said a second before, and its hands pass on only what changes.
pub fn takes_back(was: State, now: State) -> bool {
    let burning_off = |s: State| matches!(s, State::BurningOff(_));
    (hand_is_free(was) && !hand_is_free(now)) || (!burning_off(was) && burning_off(now))
}

/// Whether the organs are a person's in STATE: in a stop and in a fault.
pub fn hand_is_free(state: State) -> bool {
    matches!(state, State::Stop | State::Fault(_))
}

/// The automaton's hand, passed on only when a command changes: a relay the fuse opened stays open until the automaton
/// decides something new, rather than being closed again by the next second of the same decision.
///
/// **A PERSON'S HAND IN BETWEEN IS FORGOTTEN BY [`OnChange::forget`]**: an igniter switched on by hand in a stop is on
/// while this still remembers "off", and the automaton taking the organs back must say "off" again.
pub struct OnChange<H: Hands> {
    pub inner: H,
    auger: Option<Feeding>,
    igniter: Option<bool>,
    pump: Option<bool>,
    fan: Option<Asked>,
}

impl<H: Hands> OnChange<H> {
    /// Passes on to INNER, which nothing has been told yet.
    pub fn new(inner: H) -> Self {
        Self { inner, auger: None, igniter: None, pump: None, fan: None }
    }

    /// Forgets what was passed on, so the next decision reaches every organ.
    pub fn forget(&mut self) {
        (self.auger, self.igniter, self.pump, self.fan) = (None, None, None, None);
    }

    /// Forgets what was passed on to the auger alone, so the next decision reaches it: a ban turns off an auger a
    /// person turned on in a stop, and leaves the organs a person holds as they are.
    pub fn forget_auger(&mut self) {
        self.auger = None;
    }
}

impl<H: Hands> Hands for OnChange<H> {
    fn auger(&mut self, feeding: Feeding) {
        // **A DECISION THAT STANDS GETS A NEW PLAN, NOT A COMMAND**: a hand's switch is not taken back by it; the plan itself changes every second, because it says where the automaton stands
        let same = self.auger.is_some_and(|was| was.same_decision(&feeding));
        self.auger = Some(feeding);
        if same {
            self.inner.auger_still(feeding);
        } else {
            self.inner.auger(feeding);
        }
    }
    fn auger_still(&mut self, feeding: Feeding) {
        self.inner.auger_still(feeding);
    }
    fn igniter(&mut self, on: bool) {
        if self.igniter != Some(on) {
            self.igniter = Some(on);
            self.inner.igniter(on);
        }
    }
    fn pump(&mut self, on: bool) {
        if self.pump != Some(on) {
            self.pump = Some(on);
            self.inner.pump(on);
        }
    }
    fn fan(&mut self, asked: Asked) {
        if self.fan != Some(asked) {
            self.fan = Some(asked);
            self.inner.fan(asked);
        }
    }
}

/// Whether a person may switch the plug: CLOSING on, or off, with the gate at GATE % and the fan making DRAUGHT_PA.
/// **ONLY WITH AIR** (switched on by hand with the fan standing, the plug burnt through): switching it on
/// without air is refused in words that name the key, and switching it off is never refused.
pub fn plug_by_hand(closing: bool, gate: u32, draught_pa: Option<f32>) -> Result<(), String> {
    if !closing || crate::air(gate, draught_pa) {
        return Ok(());
    }
    let blows = match (gate, draught_pa) {
        (0, _) => "the fan is asked for no air".to_string(),
        (_, None) => "no draught is read".to_string(),
        (_, Some(pa)) => format!("the fan blows {pa:.0} Pa"),
    };
    Err(format!(
        "{blows} - the igniter burns out without air: set Draught on the Burner tab to {:.0} Pa or more first",
        crate::AIR_PA
    ))
}

/// Whether a plug CLOSED - by a person, since the automaton's own is decided in its step - may stay closed with the gate
/// at GATE % and the fan making DRAUGHT_PA: not once the air has gone, which the board then carries out at once.
pub fn plug_stays(closed: bool, gate: u32, draught_pa: Option<f32>) -> bool {
    closed && crate::air(gate, draught_pa)
}
