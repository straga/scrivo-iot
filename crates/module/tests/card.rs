//! A module's card, published by its owner and read by whoever wants it.

use module::card::{self, Card, Fill, Published, Reads};
use module::Module;
use page::dictionary::{Entry, Kind, Said, Value};
use page::Wanted;

/// Everything a card handed over, texts as owned strings.
fn read_all(published: &dyn Reads) -> (bool, Vec<(usize, String)>) {
    let mut got = Vec::new();
    let was = published.read(&mut |at, said| got.push((at, serde_json::to_string(&said).unwrap())));
    (was, got)
}

fn card_of(pairs: &[(usize, Value)]) -> Card<4, 1> {
    let mut card = Card::new();
    for (at, value) in pairs {
        card.put(*at, *value);
    }
    card
}

#[test]
fn a_reader_sees_the_last_of_two_cards_published_before_it_read() {
    let published = Published::<4, 1>::new();
    published.publish(card_of(&[(5, Value::Number(20.0))])).unwrap();
    published.publish(card_of(&[(5, Value::Number(61.5))])).unwrap();
    assert_eq!(read_all(&published), (true, vec![(5, "61.5".to_string())]));
}

#[test]
fn a_reader_that_fell_behind_gets_the_newest_card_and_not_a_queue_of_the_old() {
    let published = Published::<4, 1>::new();
    let mut late = published.receiver().expect("a place for a reader");
    for pa in [10.0, 20.0, 30.0] {
        published.publish(card_of(&[(1, Value::Number(pa))])).unwrap();
    }
    let mut got = Vec::new();
    late.try_changed().expect("news").read(&mut |at, said| got.push((at, serde_json::to_string(&said).unwrap())));
    assert_eq!(got, vec![(1, "30.0".to_string())]);
    assert!(late.try_changed().is_none(), "the older cards are not waiting behind it");
}

#[test]
fn free_text_is_read_from_its_slot_as_text() {
    let published = Published::<4, 1>::new();
    let mut card = card_of(&[(2, Value::Word("gate_stopped"))]);
    card.text(3, "the comparator refused 5200 µs");
    published.publish(card).unwrap();
    let (_, got) = read_all(&published);
    assert_eq!(got, vec![(2, "\"gate_stopped\"".to_string()), (3, "{\"text\":\"the comparator refused 5200 µs\"}".to_string())]);
}

#[test]
fn a_card_that_does_not_fit_is_refused_by_name_and_the_last_one_stays() {
    let published = Published::<2, 1>::new();
    let mut fits = Card::<2, 1>::new();
    fits.put(0, Value::On(true));
    published.publish(fits).unwrap();
    let mut over = Card::<2, 1>::new();
    for at in 0..3 {
        over.put(at, Value::On(false));
    }
    let why = published.publish(over).expect_err("three values in a card of two");
    assert!(why.contains('3') && why.contains('2'), "{why}");
    let mut too_many_texts = Card::<2, 1>::new();
    too_many_texts.text(0, "one");
    too_many_texts.text(1, "two");
    assert!(published.publish(too_many_texts).is_err(), "two texts in a card of one");
    assert_eq!(read_all(&published), (true, vec![(0, "true".to_string())]), "the last card that fitted stays");
}

/// A module of the card "fan" whose owner has or has not published yet.
struct Fan(Published<4, 1>);

impl Module for Fan {
    fn card(&self) -> &'static str {
        "fan"
    }
    fn published(&self) -> Option<&dyn Reads> {
        Some(&self.0)
    }
}

static TABLE: [Entry; 4] = [
    Entry::row("flame", "flame", &[Kind::Word]),
    Entry::row("fan", "draught", &[Kind::Number]),
    Entry::part("fan", "draught", "asked", &[Kind::Number]),
    Entry::row("fan", "holding", &[Kind::Word]),
];

#[test]
fn a_module_that_has_published_nothing_says_nothing_at_every_address_of_its_card() {
    let fan: &'static Fan = Box::leak(Box::new(Fan(Published::new())));
    let modules: &module::Modules = &[fan];
    let mut got = Vec::new();
    card::read(modules, &TABLE, &Wanted::ALL, &mut |at, said| got.push((at, said == Said::Value(Value::None))));
    assert_eq!(got, vec![(1, true), (2, true), (3, true)], "the fan's three addresses, each without a value");
}

#[test]
fn a_module_whose_card_nobody_wants_is_not_read_and_one_that_is_gives_what_it_published() {
    let flame: &'static Flame = Box::leak(Box::new(Flame(Published::new())));
    let fan: &'static Fan = Box::leak(Box::new(Fan(Published::new())));
    fan.0.publish(card_of(&[(1, Value::Number(31.5))])).unwrap();
    let modules: &module::Modules = &[flame, fan];
    let cards = module::cards(modules);
    let mut got = Vec::new();
    card::read(modules, &TABLE, &Wanted::of(&cards, &["flame"]), &mut |at, said| got.push((at, said == Said::Value(Value::None))));
    assert_eq!(got, vec![(0, true)], "the flame's one address, and nothing of the fan");
    let mut got = Vec::new();
    card::read(modules, &TABLE, &Wanted::of(&cards, &["fan"]), &mut |at, said| got.push((at, serde_json::to_string(&said).unwrap())));
    assert_eq!(got, vec![(1, "31.5".to_string())]);
}

/// A module of the card "flame" whose owner has published nothing.
struct Flame(Published<4, 1>);

impl Module for Flame {
    fn card(&self) -> &'static str {
        "flame"
    }
    fn published(&self) -> Option<&dyn Reads> {
        Some(&self.0)
    }
}

/// A card with what never carries a value on the wire - a button, a number fixed in the image - beside a reading.
static WITH_FIXED: [Entry; 3] = [
    Entry::row("fan", "draught", &[Kind::Number]),
    Entry { fixed: Some(95.0), ..Entry::part("fan", "draught", "alarm_c", &[Kind::Number]) },
    Entry::row("fan", "stop", &[Kind::Button]),
];

#[test]
fn a_module_not_published_yet_says_nothing_at_a_button_or_a_fixed_number_only_at_a_reading() {
    let fan: &'static Fan = Box::leak(Box::new(Fan(Published::new())));
    let modules: &module::Modules = &[fan];
    let mut got = Vec::new();
    card::read(modules, &WITH_FIXED, &Wanted::ALL, &mut |at, _| got.push(at));
    assert_eq!(got, vec![0], "the contract: a button and a fixed number have no value and never go on the wire");
}

/// A module of contract 1, or one with nothing to publish: no card at all.
struct Rows;

impl Module for Rows {
    fn card(&self) -> &'static str {
        "fan"
    }
}

#[test]
fn a_module_without_a_card_of_values_says_nothing_at_all() {
    let modules: &module::Modules = &[&Rows];
    let mut got = Vec::new();
    card::read(modules, &TABLE, &Wanted::ALL, &mut |at, _| got.push(at));
    assert!(got.is_empty(), "it publishes no card, so it has no addresses to say nothing at: {got:?}");
}
