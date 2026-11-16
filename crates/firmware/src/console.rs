//! Where the shell meets a person on the wire: the console UART behind the board's USB. The consoles on TCP port 23
//! and in browsers are the network loop's (crates/web), their shells run in the task `orders`.

use std::io::{Read, Write};
use std::time::Duration;

use web::orders::Order;

/// The console on the UART behind USB, as a bridge: what is typed goes to its shell in the task `orders` (the place
/// [`web::orders::USB`]), and that task writes the shell's answers onto the wire itself (`orders::Wire`) - stdin blocks
/// under the UART's driver, so this task could not be waiting for a key and writing at once. The shell's commands run
/// in `orders`, one at a time with the other consoles'.
pub fn usb() {
    let orders = &crate::orders::ORDERS;
    let place = web::orders::USB;
    orders.opened(place);
    while between::block_on(orders.ask(Order::Open(place), TYPED)).is_err() {
        std::thread::sleep(Duration::from_secs(1));
    }
    let mut buf = [0u8; web::orders::LINE];
    loop {
        let n = match std::io::stdin().read(&mut buf) {
            Ok(n) if n > 0 => n,
            _ => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
        };
        let typed = heapless::Vec::from_slice(&buf[..n]).unwrap_or_default();
        if let Err(why) = between::block_on(orders.ask(Order::Feed(place, typed), TYPED)) {
            let words: &[u8] = match why {
                between::Refused::Busy => b"busy: a command of another console is still running - type it again when it ends\r\n",
                between::Refused::Silent => b"the board did not answer in time\r\n",
            };
            let _ = std::io::stdout().write_all(words);
        }
    }
}

/// How long a line typed on USB waits for its shell: `bus scan` walks 247 addresses at 80 ms.
const TYPED: Duration = Duration::from_secs(35);

/// The stack of the USB console's bridge: the plan's (8 KB; 5 KB on a board without external RAM, where the bridge was seen to use 3.2).
pub const STACK: usize = plan::PLAN.usb_console_stack;

/// Starts F as the task NAME on the network's core: `orders` and the USB console's bridge are the network's side.
pub fn spawn(name: &str, stack: usize, f: impl FnOnce() + Send + 'static) {
    if let Err(e) = round::round(name, stack, round::Core::Net, f) {
        log::error!("thread {name} not started: {e}");
    }
}
