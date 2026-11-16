//! The walk of a device's list of modules: who answers a name, what is asked of whom, and what nobody answers.

use std::sync::atomic::{AtomicU32, Ordering};

use module::Module;
use page::Curve;
use settings::{Kind, Setting};

const fn setting(name: &'static str, card: &'static str) -> Setting {
    Setting { name, label: name, default: "0", kind: Kind::Whole { from: 0, to: 9, unit: "" }, level: settings::Level::Work, card: Some(card) }
}

static A_ONE: Setting = setting("a_one", "a");
static A_TWO: Setting = setting("a_two", "a");
static B_ONE: Setting = setting("b_one", "b");

/// A module of the tests: a card, rows by name, a curve by name, a setting and a switch that refuses when told to.
struct Fake {
    card: &'static str,
    rows: &'static [&'static str],
    curve: &'static str,
    step_ms: u32,
    settings: &'static [&'static Setting],
    refuses: bool,
    lines_fail: bool,
    /// How many commands this module carried out.
    acted: AtomicU32,
}

impl Fake {
    const fn new(card: &'static str, rows: &'static [&'static str], curve: &'static str, step_ms: u32) -> Self {
        Fake { card, rows, curve, step_ms, settings: &[], refuses: false, lines_fail: false, acted: AtomicU32::new(0) }
    }

    fn own(&self, name: &str) -> Option<Result<(), String>> {
        self.rows.contains(&name).then(|| match self.refuses {
            true => Err(format!("{} refuses {name}", self.card)),
            false => {
                self.acted.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        })
    }
}

impl Module for Fake {
    fn card(&self) -> &'static str {
        self.card
    }

    fn settings(&self) -> Vec<&'static Setting> {
        self.settings.to_vec()
    }

    fn curve(&self, name: &str) -> Option<Curve> {
        (name == self.curve).then(|| Curve { step_ms: self.step_ms, lines: vec![] })
    }

    fn lines(&self, line: &mut dyn FnMut(&str)) -> Result<(), String> {
        if self.lines_fail {
            return Err(format!("{} cannot say", self.card));
        }
        line(&format!("{} line", self.card));
        Ok(())
    }

    fn order(&self, name: &str, _order: module::Order) -> Option<Result<(), String>> {
        self.own(name)
    }
}

// ---- curves ----

static CURVE_A: Fake = Fake::new("a", &[], "same", 1);
static CURVE_B: Fake = Fake::new("b", &[], "same", 2);
static CURVE_C: Fake = Fake::new("c", &[], "c-only", 3);

#[test]
fn the_first_module_that_knows_a_curve_answers_it() {
    let modules: &module::Modules = &[&CURVE_A, &CURVE_B, &CURVE_C];
    assert_eq!(module::curve(modules, "same").map(|c| c.step_ms), Some(1));
    assert_eq!(module::curve(modules, "c-only").map(|c| c.step_ms), Some(3));
}

#[test]
fn a_curve_nobody_knows_is_none() {
    let modules: &module::Modules = &[&CURVE_A, &CURVE_B, &CURVE_C];
    assert!(module::curve(modules, "nosuch").is_none());
    assert!(module::curve(&[], "same").is_none());
}

// ---- cards ----

static ROWS_A: Fake = Fake::new("a", &["a1", "a2"], "", 0);
static ROWS_B: Fake = Fake::new("b", &["b1"], "", 0);
static ROWS_SYS: Fake = Fake::new(page::SYSTEM, &["probe"], "", 0);
static ROWS_A_AGAIN: Fake = Fake::new("a", &["a3"], "", 0);

#[test]
fn the_cards_are_the_modules_own_in_their_order_each_once() {
    let modules: &module::Modules = &[&ROWS_B, &ROWS_SYS, &ROWS_A, &ROWS_A_AGAIN];
    assert_eq!(module::cards(modules), vec!["b", "a"]);
}

#[test]
fn a_device_with_no_modules_has_no_cards() {
    assert!(module::cards(&[]).is_empty());
}



// ---- settings and notes ----

static SET_A: Fake = Fake { settings: &[&A_ONE, &A_TWO], ..Fake::new("a", &[], "", 0) };
static SET_B: Fake = Fake { settings: &[&B_ONE], ..Fake::new("b", &[], "", 0) };
static SET_NONE: Fake = Fake::new("c", &[], "", 0);

#[test]
fn the_settings_are_every_modules_in_the_modules_order() {
    let modules: &module::Modules = &[&SET_B, &SET_NONE, &SET_A];
    let names: Vec<_> = module::settings(modules).iter().map(|s| s.name).collect();
    assert_eq!(names, vec!["b_one", "a_one", "a_two"]);
}

// ---- meter ----

static SAY_A: Fake = Fake::new("a", &[], "", 0);
static SAY_B: Fake = Fake::new("b", &[], "", 0);
static SAY_FAILS: Fake = Fake { lines_fail: true, ..Fake::new("f", &[], "", 0) };

#[test]
fn the_lines_are_every_modules_in_order() {
    let modules: &module::Modules = &[&SAY_B, &SAY_A];
    let mut got = vec![];
    assert_eq!(module::lines(modules, &mut |l| got.push(l.to_string())), Ok(()));
    assert_eq!(got, vec!["b line", "a line"]);
}

#[test]
fn a_module_that_cannot_say_ends_the_lines_with_its_reason() {
    let modules: &module::Modules = &[&SAY_A, &SAY_FAILS, &SAY_B];
    let mut got = vec![];
    assert_eq!(module::lines(modules, &mut |l| got.push(l.to_string())), Err("f cannot say".to_string()));
    assert_eq!(got, vec!["a line"]);
}

// ---- actions ----

static ACT_FAN: Fake = Fake::new("fan", &["purge", "gate"], "", 0);
static ACT_RELAYS: Fake = Fake { refuses: true, ..Fake::new("relays", &["auger"], "", 0) };
static ACT_OK: Fake = Fake::new("ok", &["pump"], "", 0);
static ACTS: [&dyn Module; 3] = [&ACT_FAN, &ACT_RELAYS, &ACT_OK];

#[test]
fn an_action_goes_to_the_module_whose_thing_it_is() {
    let modules: &'static module::Modules = &ACTS;
    let acted = |f: &Fake| f.acted.load(Ordering::Relaxed);
    let before = (acted(&ACT_FAN), acted(&ACT_OK));
    assert_eq!(module::toggle(modules, "purge"), Ok(()));
    assert_eq!(module::switch(modules, "pump", true), Ok(()));
    assert_eq!(module::hold(modules, "gate", 40), Ok(()));
    assert_eq!((acted(&ACT_FAN), acted(&ACT_OK)), (before.0 + 2, before.1 + 1), "each command went to the module whose thing it is");
}

#[test]
fn the_owners_refusal_stands_and_is_not_hidden_by_another_module() {
    let modules: &'static module::Modules = &ACTS;
    assert_eq!(module::toggle(modules, "auger"), Err("relays refuses auger".to_string()));
    assert_eq!(module::switch(modules, "auger", false), Err("relays refuses auger".to_string()));
}

#[test]
fn an_action_on_a_thing_nobody_has_is_refused_by_its_name() {
    let modules: &'static module::Modules = &ACTS;
    for said in [
        module::toggle(modules, "nosuch").map(|_| ()),
        module::switch(modules, "nosuch", true).map(|_| ()),
        module::hold(modules, "nosuch", 1).map(|_| ()),
        module::toggle(&[], "nosuch").map(|_| ()),
    ] {
        let why = said.expect_err("a thing nobody has was acted on");
        assert!(why.contains("nosuch"), "the refusal does not name the thing: {why:?}");
    }
}
