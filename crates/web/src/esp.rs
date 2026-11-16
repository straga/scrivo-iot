//! The loop on the board: the eventfd the reactor wakes by, the task `net` the loop runs on, the reactor's thread
//! named `reactor`, the ports bound over lwIP.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use edge_nal::TcpBind;
use between::block_on;
use esp_idf_svc::io::vfs::MountedEventfs;
use esp_idf_svc::sys::EspError;

use crate::orders::Orders;
use crate::{Board, Limits, LogFeed, HTTP_PORT, TELNET_PORT};

/// Event descriptors: the reactor takes one to be woken when a socket is added (async-io-mini, reactor.rs).
const EVENTFDS: usize = 1;

/// The loop's stack. Measured on `.30` under the load of the probes and loop_probe (`.25.18`): 12332 bytes
/// used - the deepest poll runs a module's card, LittleFS reading a page file in pieces of 4 KB, and serde inside one
/// handler; that and `ROOMY` (1536), up to a multiple of 512. On a device under load, 26.09 09:40, the least ever free was 1504 -
/// below ROOMY by 32 -, so 512 more. Then 26.10.04.4 under load, 04.10: the least ever free went 2060 -> 1760 over the run (the stack was not cut), 224 over ROOMY, and the owner's word is 2 KB more.
///
/// **THE NUMBER IS THE PLAN'S** (crate `plan`): the one above is the large profile, for a board with external RAM. A board whose
/// internal RAM is all its heap takes the small one, which keeps what the loop was seen to use there (10.1 KB under an open page and
/// both browser consoles, 11.4) and the room over it.
const STACK: usize = plan::PLAN.net_stack;

/// Starts the loop as the task `net` over BOARD, its commands asked of ORDERS, the log's lines taken from LOG. A board without the eventfd
/// filesystem says so in the log and goes on without its network: a board that falls at start for want of it would be
/// rolled back with the reason gone.
pub fn start<B: Board>(board: &'static B, orders: &'static Orders, log: &'static LogFeed) -> Result<(), EspError> {
    match MountedEventfs::mount(EVENTFDS) {
        // mounted for the life of the program: its Drop would unregister it under the reactor
        Ok(mounted) => core::mem::forget(mounted),
        Err(e) => {
            log::error!("net loop: eventfd not mounted ({e}), the page and port 23 are not served");
            return Ok(());
        }
    }
    round::round("net", STACK, round::Core::Net, move || run(board, orders, log))
}

fn bound<A>(what: &str, port: u16, made: std::io::Result<A>) -> Option<A> {
    match made {
        Ok(a) => Some(a),
        Err(e) => {
            log::error!("net loop: {what} port {port} not bound: {e}");
            None
        }
    }
}

/// The body of the task `net`: binds both ports and serves for good.
fn run<B: Board>(board: &'static B, orders: &'static Orders, log: &'static LogFeed) {
    let at = |port| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port));
    // The first socket starts the reactor's thread, inside async-io-mini; named here, or it is one more `pthread`.
    let page = round::named("reactor", async_io_mini::REACTOR_STACK_SIZE, round::Core::Net, || {
        block_on(edge_nal_std::Stack::new().bind(at(HTTP_PORT)))
    });
    let page = match page {
        Ok(made) => bound("page", HTTP_PORT, made),
        Err(e) => return log::error!("net loop: the reactor's task not named: {e}"),
    };
    let port = bound("console", TELNET_PORT, block_on(edge_nal_std::Stack::new().bind(at(TELNET_PORT))));
    let (Some(page), Some(port)) = (page, port) else { return };
    // what the loop keeps itself is its tasks' boxes; each task is born alone (crate::serve)
    let serving = crate::serve(board, orders, log, page, port, Limits::BOARD);
    log::info!("net loop: page on port {HTTP_PORT}, console on port {TELNET_PORT}, its future {} bytes", core::mem::size_of_val(&serving));
    between::block_on_stepped(serving, &crate::STEPS, crate::STEP_MOST);
    log::error!("net loop: stopped");
}
