//! A module's values as its owner publishes them: a card of fixed length, put together by the task that owns
//! the values and handed to every reader as a copy - the network, a deciding module, a regulator. Nobody reads a
//! module's statics from another task any more, and nobody runs a module's code on the network's behalf.
//!
//! The form is ESPHome's turned into memory that does not move: there an entity publishes its state and each
//! controller reads it when it sends (`controller_registry.h`, `api_connection.cpp`); here the owner publishes a
//! whole card into a `Watch`, which keeps the last one and nothing older.

use between::Lock;
use embassy_sync::watch::{self, Watch};
use page::dictionary::{Entry, Kind, Said, Text, Value};
use page::Wanted;

use crate::Modules;

/// Readers with a place of their own on one card: the network, a deciding module, a regulator.
/// Any number more read without one ([`Reads`]).
pub const READERS: usize = 3;

/// Where a card is put together: its values by address, and free text by address into a slot of its own.
pub trait Fill {
    fn put(&mut self, at: usize, value: Value);
    fn text(&mut self, at: usize, said: &str);

    /// A value or a text, whichever SAID is - for a card made by something that hands over what it says.
    fn said(&mut self, at: usize, said: Said<'_>) {
        match said {
            Said::Value(value) => self.put(at, value),
            Said::Text(text) => self.text(at, text),
        }
    }
}

/// A card of up to N values and T texts, put together by its owner and copied to every reader whole.
#[derive(Clone)]
pub struct Card<const N: usize, const T: usize> {
    values: heapless::Vec<(u16, Value), N>,
    texts: heapless::Vec<(u16, Text), T>,
    /// What did not fit, values and texts: a card that lost any is refused when published, not sent short.
    over: (usize, usize),
}

impl<const N: usize, const T: usize> Card<N, T> {
    pub const fn new() -> Self {
        Card { values: heapless::Vec::new(), texts: heapless::Vec::new(), over: (0, 0) }
    }

    /// Hands over every value, then every text.
    pub fn read(&self, each: &mut dyn FnMut(usize, Said<'_>)) {
        for (at, value) in &self.values {
            each(usize::from(*at), Said::Value(*value));
        }
        for (at, text) in &self.texts {
            each(usize::from(*at), Said::Text(text.as_str()));
        }
    }
}

impl<const N: usize, const T: usize> Default for Card<N, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, const T: usize> Fill for Card<N, T> {
    fn put(&mut self, at: usize, value: Value) {
        if self.values.push((at as u16, value)).is_err() {
            self.over.0 += 1;
        }
    }

    fn text(&mut self, at: usize, said: &str) {
        if self.texts.push((at as u16, Text::from(said))).is_err() {
            self.over.1 += 1;
        }
    }
}

/// What a card can be read as, whatever its size.
pub trait Reads: Sync {
    /// Hands over every value and text of the last card published, and answers whether there was one.
    fn read(&self, each: &mut dyn FnMut(usize, Said<'_>)) -> bool;
}

/// The last card of a module, published by its owner.
pub struct Published<const N: usize, const T: usize> {
    watch: Watch<Lock, Card<N, T>, READERS>,
}

impl<const N: usize, const T: usize> Published<N, T> {
    pub const fn new() -> Self {
        Published { watch: Watch::new() }
    }

    /// Publishes CARD for every reader; a card that overflowed is refused in words, and the last one stays.
    pub fn publish(&self, card: Card<N, T>) -> Result<(), String> {
        let (values, texts) = card.over;
        if values > 0 || texts > 0 {
            return Err(format!(
                "a card of {N} values and {T} texts was given {} values and {} texts",
                N + values,
                card.texts.len() + texts
            ));
        }
        self.watch.sender().send(card);
        Ok(())
    }

    /// A reader with a place of its own, told of each new card; None when every place is taken.
    pub fn receiver(&self) -> Option<Receiver<'_, N, T>> {
        self.watch.receiver().map(Receiver)
    }
}

/// A reader of one module's cards with a place of its own.
pub struct Receiver<'a, const N: usize, const T: usize>(watch::Receiver<'a, Lock, Card<N, T>, READERS>);

impl<const N: usize, const T: usize> Receiver<'_, N, T> {
    /// The newest card, if one came since this reader last looked.
    pub fn try_changed(&mut self) -> Option<Card<N, T>> {
        self.0.try_changed()
    }

    /// The newest card, waiting for the first one when none has come yet.
    pub async fn get(&mut self) -> Card<N, T> {
        self.0.get().await
    }
}

impl<const N: usize, const T: usize> Default for Published<N, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, const T: usize> Reads for Published<N, T> {
    fn read(&self, each: &mut dyn FnMut(usize, Said<'_>)) -> bool {
        match self.watch.try_get() {
            Some(card) => {
                card.read(each);
                true
            }
            None => false,
        }
    }
}

/// The values of the modules whose card is WANTED, by address, from what each has published. A module with a card that
/// has published nothing yet says "nothing" at every address of its card in TABLE that ever carries a value; a module
/// without a card of values says nothing at all.
pub fn read(modules: &Modules, table: &[Entry], wanted: &Wanted, each: &mut dyn FnMut(usize, Said<'_>)) {
    for module in modules.iter().filter(|m| wanted.has(m.card())) {
        let Some(published) = module.published() else { continue };
        if !published.read(each) {
            for (at, _) in table.iter().enumerate().filter(|(_, e)| e.card == module.card() && carries_a_value(e)) {
                each(at, Said::Value(Value::None));
            }
        }
    }
}

/// Whether a value of ENTRY ever goes on the wire: a button and a number fixed in the image never do (the contract).
fn carries_a_value(entry: &Entry) -> bool {
    entry.fixed.is_none() && !entry.kinds.contains(&Kind::Button)
}
