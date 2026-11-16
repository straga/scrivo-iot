//! MODULES: the one place the layers meet. A module here joins ONE subject across the layers: it reads src/hardware, asks the logic in crates/ what
//! the numbers mean, and publishes the result as the entities api/ describes. Each file is one module of burner.yaml.
//! HERE THERE IS NO OWN NUMBER AND NO OWN WORD FOR THE SCREEN: a pin is burner.yaml's, a threshold is a setting of crates/settings, a phrase on the
//! page is page/menu.json (the log and the console say what they say). A module that needs one of them says so in the layer it belongs to.

pub mod automaton;
pub mod demand;
pub mod draught;
pub mod fan;
pub mod flame;
pub mod flue;
pub mod mains;
pub mod pellets;
pub mod relief;
pub mod restart;
pub mod relays;
pub mod soot;
pub mod temperatures;
pub mod thermostat;

/// The modules of the burner, in the order the page lists their cards.
pub static ALL: &module::Modules = &[&thermostat::Module, &demand::Module, &automaton::Module, &relays::Module, &temperatures::Module, &flue::Module, &fan::Module, &draught::Module, &flame::Module, &pellets::Module, &restart::Module, &mains::Module, &soot::Module];
