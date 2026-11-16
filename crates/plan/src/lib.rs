//! What the image keeps in its heap, by the board it runs on.
//!
//! **THE SIZES WERE MEASURED ON A BOARD WITH PSRAM.** A board with PSRAM has two megabytes of external RAM: whatever is
//! bigger than 512 bytes lives outside the chip, and a buffer of ten kilobytes costs the internal heap nothing. A board with
//! a classic ESP32 has none; every byte of the same buffers is taken from the heap that holds its stacks, its sockets
//! and its Wi-Fi, and on 08.10 an open page left it 30 KB.
//!
//! Two profiles, then: [`LARGE`] is what a board with PSRAM runs with - the numbers measured on a device in service - and
//! [`SMALL`] is for a board with no external RAM. Which one an image gets is the board's word, `psram` of its board.toml,
//! handed to the build as `STRAGA_PSRAM` (tools/board_env.py); the build script of this crate turns `0` into the cfg
//! `small_heap`. An image built without the variable - a test on the computer - gets the large one.
//!
//! The small numbers are what each task was seen to use under an open page with both browser consoles, plus the room a
//! stack keeps (`firmware::board::ROOMY`, 1536 bytes), rounded up to 512.

/// The sizes one profile holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Stack of the task `net`, which serves the page, the streams and the consoles.
    pub net_stack: usize,
    /// Stack of the task `flash`, the one writer of the flash.
    pub flash_stack: usize,
    /// Stack of the console on the USB.
    pub usb_console_stack: usize,
    /// Bytes of the buffer each page handler reads the head of a request into.
    pub head_buffer: usize,
    /// Lines of the log the loop keeps for the consoles that connect later.
    pub log_ring: usize,
    /// Lines of the log that wait for the owner of the flash before it starts.
    pub to_flash_lines: usize,
    /// Handlers of the page: each serves one connection at a time and holds its buffer of [`Plan::head_buffer`] and a future of
    /// 2.4 KB all the while, whether a connection is there or not (22 KB at six).
    pub handlers: usize,
}

impl Plan {
    /// How many of the handlers connections that LAST may hold at once - a stream of a page, a console of a browser -: all but one.
    /// The one left is for the short requests (the page's files, /state, a command), which a handler held by a stream or a
    /// console would otherwise leave waiting in the queue until the board refuses them.
    pub const fn long_holders(&self) -> usize {
        self.handlers - 1
    }
}

/// The numbers of a board with PSRAM, measured on a device in service.
pub const LARGE: Plan = Plan {
    net_stack: 14848 + 2048,
    flash_stack: 8192,
    usb_console_stack: 8192,
    head_buffer: 2048,
    log_ring: 64,
    to_flash_lines: 32,
    handlers: 6,
};

/// For a board with no external RAM. The stacks are what the task used plus the room, up to 512: net 10.1 KB seen, flash 3.3,
/// the USB console 3.2. The head buffer is a little over the longest head a Chrome sends this page (about 900 bytes); the ring
/// of the log keeps sixteen lines for a console that comes in late, and the owner of the flash waits behind eight. Three
/// handlers: a stream, a console of a browser and the short requests beside them; a second page takes the place of the console.
pub const SMALL: Plan = Plan {
    net_stack: 12288,
    flash_stack: 5120,
    usb_console_stack: 5120,
    head_buffer: 1280,
    log_ring: 16,
    to_flash_lines: 8,
    handlers: 3,
};

/// The plan of this image.
pub const PLAN: Plan = if cfg!(small_heap) { SMALL } else { LARGE };
