//! A command by its role: the fan is asked by what it does for the burner, not by which module drives it.

use std::any::Any;
use std::sync::Mutex;

use module::{command, take, Module, Role};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Air {
    Stop,
    Hold(u32),
}

static FAN: Role<Air> = Role::new("fan");

/// A module that carries out the role `fan`, refusing whatever it was told to refuse.
struct Fan {
    got: Mutex<Vec<Air>>,
    refuses: bool,
    /// takes the command as a percent - the wrong type for the role
    as_percent: bool,
}

impl Module for Fan {
    fn card(&self) -> &'static str {
        "fan"
    }


    fn command(&self, role: &str, what: &dyn Any) -> Option<Result<(), String>> {
        (role == "fan").then(|| {
            if self.as_percent {
                return take::<u32>(what, role).map(|_| ());
            }
            let air = take::<Air>(what, role)?;
            if self.refuses {
                return Err("the fan is held by hand".into());
            }
            self.got.lock().unwrap().push(air);
            Ok(())
        })
    }
}

/// A module playing no role at all.
struct Other;

impl Module for Other {
    fn card(&self) -> &'static str {
        "other"
    }

}

static OTHER: Other = Other;

#[test]
fn a_command_reaches_the_module_that_plays_the_role() {
    static TAKES: Fan = Fan { got: Mutex::new(Vec::new()), refuses: false, as_percent: false };
    assert_eq!(command(&[&OTHER, &TAKES], &FAN, Air::Hold(51)), Ok(()));
    assert_eq!(command(&[&OTHER, &TAKES], &FAN, Air::Stop), Ok(()));
    assert_eq!(*TAKES.got.lock().unwrap(), vec![Air::Hold(51), Air::Stop]);
}

#[test]
fn a_command_nobody_carries_out_is_refused_naming_the_role() {
    let why = command(&[&OTHER], &FAN, Air::Stop).expect_err("a command nobody takes was taken");
    assert!(why.contains("fan") && why.contains("nothing"), "{why:?}");
}

#[test]
fn the_players_refusal_stands() {
    static REFUSES: Fan = Fan { got: Mutex::new(Vec::new()), refuses: true, as_percent: false };
    assert_eq!(command(&[&REFUSES], &FAN, Air::Stop), Err("the fan is held by hand".to_string()));
}

#[test]
fn a_command_of_another_type_is_refused_and_not_carried_out() {
    static PERCENT: Fan = Fan { got: Mutex::new(Vec::new()), refuses: false, as_percent: true };
    let why = command(&[&PERCENT], &FAN, Air::Stop).expect_err("a command of another type was carried out");
    assert!(why.contains("fan"), "{why:?}");
}
