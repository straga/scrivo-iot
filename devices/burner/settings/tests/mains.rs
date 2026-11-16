//! The socket the burner reads (Tasmota, over HTTP): where it is, written as an IPv4 address by a person or as none, and how often the burner asks it. Expected from what a person does - the
//! socket is none and the card offers an address, a person may take the socket away, and an address that cannot be dialled is refused with the words of the refusal and not kept.

use burner_settings::{ALL, RULES};
use settings::{change, seed, value, Setting, Store};
use std::collections::BTreeMap;

#[derive(Default)]
struct Nvs(BTreeMap<String, String>);

impl Store for Nvs {
    fn get(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
    fn put(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.0.insert(name.into(), value.into());
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<bool, String> {
        Ok(self.0.remove(name).is_some())
    }
}

fn seeded() -> Nvs {
    let mut nvs = Nvs::default();
    seed(&ALL, &RULES, &mut nvs, &[]);
    nvs
}

fn setting(name: &str) -> &'static Setting {
    ALL.iter().copied().find(|s| s.name == name).unwrap_or_else(|| panic!("no setting {name} in ALL"))
}

#[test]
fn a_new_board_reads_no_socket_and_will_ask_one_every_five_seconds() {
    let nvs = seeded();
    assert_eq!(value(setting("mains_at"), &nvs), "off");
    assert_eq!(value(setting("mains_every"), &nvs), "5");
}

#[test]
fn both_stand_on_the_card_of_the_socket() {
    assert_eq!(setting("mains_at").card, Some("mains"));
    assert_eq!(setting("mains_every").card, Some("mains"));
}

#[test]
fn an_address_typed_bare_or_behind_its_word_is_kept_behind_its_word() {
    for (typed, kept) in [("192.0.2.10", "address:192.0.2.10"), ("address:192.0.2.5", "address:192.0.2.5"), (" 192.0.2.7 ", "address:192.0.2.7")] {
        let mut nvs = seeded();
        change(&ALL, &RULES, &mut nvs, "mains_at", typed).unwrap_or_else(|why| panic!("{typed:?} refused: {why}"));
        assert_eq!(value(setting("mains_at"), &nvs), kept, "{typed:?}");
    }
}

#[test]
fn a_person_may_take_the_socket_away_and_bring_it_back() {
    let mut nvs = seeded();
    change(&ALL, &RULES, &mut nvs, "mains_at", "off").unwrap();
    assert_eq!(value(setting("mains_at"), &nvs), "off");
    change(&ALL, &RULES, &mut nvs, "mains_at", "192.0.2.10").unwrap();
    assert_eq!(value(setting("mains_at"), &nvs), "address:192.0.2.10");
}

#[test]
fn what_cannot_be_dialled_is_refused_and_the_address_stays() {
    // a name needs a lookup the board does not make; a port the socket does not listen on; a quad that is not one; nothing
    for typed in ["socket.local", "192.0.2.10:80", "192.0.2", "192.0.2.256", "192.0.2.10.1", "999.1.1.1", "address:", "", "http://192.0.2.10", "192.0.2.-1"] {
        let mut nvs = seeded();
        let before = value(setting("mains_at"), &nvs);
        assert!(change(&ALL, &RULES, &mut nvs, "mains_at", typed).is_err(), "{typed:?} was taken");
        assert_eq!(value(setting("mains_at"), &nvs), before, "{typed:?} moved the address though it was refused");
    }
}

#[test]
fn how_often_is_a_span_of_seconds_from_one_to_sixty() {
    for ok in ["1", "5", "60"] {
        let mut nvs = seeded();
        change(&ALL, &RULES, &mut nvs, "mains_every", ok).unwrap_or_else(|why| panic!("{ok} refused: {why}"));
        assert_eq!(value(setting("mains_every"), &nvs), ok);
    }
    for bad in ["0", "61", "-1", "five", "", "5.5"] {
        let mut nvs = seeded();
        assert!(change(&ALL, &RULES, &mut nvs, "mains_every", bad).is_err(), "{bad:?} was taken");
        assert_eq!(value(setting("mains_every"), &nvs), "5");
    }
}
