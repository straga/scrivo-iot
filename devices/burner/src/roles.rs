//! WHAT THE BURNER'S QUANTITIES ARE CALLED when somebody uses them, and what each is given as. The module that plays a role publishes it every round, and a consumer holds the role and not the module (crates/module, `Role`): which module
//! gives it is the modules' business, and a sensor that moves does not move its consumers.
//! NEVER HERE: a reading being made (the modules), what it means (crates/), a word for the page.
//!
//! A SENSOR'S ROLE GROWS OLD by the period of its round: silent past three periods (at least 3 s) it says nothing, whatever its round last published. A stopped round is not a reading - on 27.09 the draught's round stood still and the
//! regulator held a pressure of 34 seconds ago.

use module::Role;

/// The pressure the fan makes, a mean over the last second, through the sensor's own words for under and over its scale (crates/draught).
pub static DRAUGHT: Role<::draught::Reading> = Role::sensor("draught", crate::modules::draught::every);

/// What the fire is doing, through the thresholds (crates/flame).
pub static FLAME: Role<::flame::Flame> = Role::sensor("flame", crate::modules::flame::every);
/// The flue gas, in degrees.
pub static FLUE_GAS: Role<f32> = Role::sensor("flue_gas", crate::modules::flue::every);
/// The water at the top of the boiler, in degrees: the socket of the temperatures set to `boiler`. What the automaton holds the boiler by; the supply is another role and is never taken for it.
pub static BOILER: Role<f32> = Role::sensor(burner_settings::BOILER, crate::modules::temperatures::every);
/// The water coming back, in degrees: the socket set to `return`. What the burner holds when it is set to hold the return; the protections never look at it.
pub static RETURN: Role<f32> = Role::sensor(burner_settings::RETURN, crate::modules::temperatures::every);
/// The air of the boiler room, in degrees: the socket set to `room`. What the room says its target against when the thermostat asks by the room (modules/demand.rs).
pub static ROOM: Role<f32> = Role::sensor(burner_settings::ROOM, crate::modules::temperatures::every);

/// The power the burner draws from the mains, in watts, read from the socket it is plugged into (modules/mains.rs); silent while the socket does not answer or none is set.
pub static MAINS_POWER: Role<f32> = Role::sensor("mains_power", crate::modules::mains::every);

/// What the fan is asked to do - the automaton's hand on it, the same place the card's slider and purge write, so the regulator's round takes it as it takes a person's.
pub static FAN: Role<regulator::Asked> = Role::new("fan");

/// The relays the automaton drives, each on or off: its hand on them, which is not a person's and does not pass the point a person's commands pass (crates/module, `through`). A role of decisions, it grows no old.
/// The auger is told a rhythm and not a level: the relays' owner times the portions to a few tens of ms (crates/relay, `Feeding`), and a plan the automaton does not replace within `relay::HORIZON_MS` runs out and leaves the relay open.
pub static AUGER: Role<relay::Told> = Role::new("auger");
pub static IGNITER: Role<bool> = Role::new("igniter");
pub static PUMP: Role<bool> = Role::new("pump");

/// What the house says to the boiler: cold or warm, and why (crates/demand). Played by the module of the demand - the room against its target, or the schedule - and read by the thermostat, which does not know which; a contact on a leg or a command over the network is another player of the same role. A role of decisions, it grows no old; nobody playing it says nothing, and the boiler works by its water alone.
pub static DEMAND: Role<::demand::Asked> = Role::new("demand");

/// What the thermostat orders of the fire this second: whether the water asks for burning, on which level, whether a person asked for heat, and how many times each button was pressed (crates/thermostat). The automaton holds the role and not the
/// module that plays it; a role of decisions, it grows no old.
pub static HEAT: Role<::automaton::Order> = Role::new("heat");
