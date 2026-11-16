//! The task `orders`: what the network's loop hands over because it waits on another task - a command to a module, a
//! saved setting, a curve, a line typed into a console. One order at a time, in the order asked; the loop meanwhile
//! serves everyone else (`web::orders`).

use std::io::Write;

use web::orders::{Answer, Hands, Order, Orders, Said, CONSOLES, USB};

use crate::board::Esp;
use crate::site::{self, Site};

/// The desk the loop asks at.
pub static ORDERS: Orders = Orders::new();

/// The stack of `orders`: it runs the shell's commands and writes their answers, as each `tcp console` thread did.
/// Measured on `.30` under two consoles at once with a word answering 324 lines: 7204 bytes of 8192
/// used; that and `board::ROOMY` (1536), up to a multiple of 512. On a device with a long answer, 26.09 09:40, the least ever free was
/// 1500 - below ROOMY by 36 -, so 512 more.
const STACK: usize = 9728;

/// Where a console's shell writes: the console's pipe, which the loop drains into its socket; or, for the console on
/// USB, the wire itself, from this task.
enum Wire {
    Pipe(Said<'static>),
    Usb,
}

impl embedded_io::ErrorType for Wire {
    type Error = core::convert::Infallible;
}

impl embedded_io::Write for Wire {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        match self {
            Wire::Pipe(said) => embedded_io::Write::write(said, buf),
            Wire::Usb => {
                let _ = std::io::stdout().write_all(buf);
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        if let Wire::Usb = self {
            let _ = std::io::stdout().flush();
        }
        Ok(())
    }
}

struct Firmware {
    site: &'static Site,
    /// A shell a console place, made when a console opens there. In the heap: five of them are over a kilobyte.
    shells: Vec<Option<shell::Shell<Wire>>>,
}

impl Hands for Firmware {
    fn carry(&mut self, order: Order, _out: &Orders) -> Answer {
        let site = self.site;
        match order {
            Order::Act { name, command } => match site::act(site, name, command) {
                Ok(()) => Answer::Done,
                Err(why) => Answer::Refused(why),
            },
            Order::Save(form) => Answer::Json(page::save(site.dictionary.table, &form, |pairs| {
                crate::nvs_settings::change_all(pairs).map(|moved| moved.iter().map(settings::Moved::wire).collect()).map_err(|why| why.wire())
            })),
            Order::Curve(name) => Answer::Curve(site::curve_of(site, &name)),
            Order::Waiting => match crate::flash_owner::waiting() {
                Ok(waiting) => Answer::Waiting(waiting),
                Err(why) => Answer::Refused(why),
            },
            Order::Tail => match crate::flash_owner::tail() {
                Ok(tail) => Answer::Tail(tail),
                Err(why) => Answer::Refused(why),
            },
            Order::Open(place) => {
                // the new shell's first prompt goes into the console's pipe as it is made
                let wire = match place {
                    USB => Wire::Usb,
                    _ => Wire::Pipe(Said { orders: &ORDERS, place }),
                };
                self.shells[place] = shell::Shell::new(wire);
                match self.shells[place] {
                    Some(_) => Answer::Done,
                    None => Answer::Refused("the console could not start".into()),
                }
            }
            Order::Feed(place, bytes) => {
                let Some(shell) = self.shells[place].as_mut() else {
                    return Answer::Refused("no shell at this console".into());
                };
                for b in bytes {
                    let _ = shell.feed(b, &mut Esp);
                }
                Answer::Done
            }
        }
    }
}

/// Starts the task `orders` over SITE.
pub fn start(site: &'static Site) {
    crate::console::spawn("orders", STACK, move || {
        let mut shells = Vec::with_capacity(CONSOLES);
        shells.resize_with(CONSOLES, || None);
        let mut hands = Firmware { site, shells };
        ORDERS.serve(&mut hands)
    });
}
