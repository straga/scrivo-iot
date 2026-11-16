//! The board's log on its way to its readers: every line into a channel of fixed size from whichever task
//! wrote it ([`Feed`]), the network's loop keeping the last of them in a ring for its consoles ([`Ring`]), and the lines
//! worth reading after the fact kept on flash ([`kept`]). Nothing is made per reader, and a reader that falls behind
//! loses the oldest lines and is told how many - never the time of the task that logs.

pub mod feed;
pub mod folded;
pub mod kept;
pub mod repeats;
pub mod ring;

pub use folded::{Fold, Summary, Verdict, BLACK_BOX, BOX_CLOCK, BOX_FRONTS, BOX_TICK, BLACK_BOX_PER_MINUTE, FOLD_BURST, FOLD_PERIOD_S};
pub use feed::{line_of, Feed, Line, LINE};
pub use repeats::{Each, Keyed, Repeats, Said};
pub use ring::{Read, Ring};
