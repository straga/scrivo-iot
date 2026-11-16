//! What every address of each device is: the dictionary the image carries and a client reads once.
//!
//! **ONE TABLE PER DEVICE, AND IT IS BOTH ENDS.** The image serves it as JSON (`page::dictionary::json`, made when the
//! image is built), and a module writes its values by addresses worked out from the same table when the image is
//! compiled (`page::dictionary::at`) - a row the table has not got stops the build. So what a client reads and what a
//! board sends cannot come apart.
//!
//! Built on the computer as well as for the board, so the words are checked here against the functions that said
//! them before (tests/), and the page's own renderer is held to the same examples.

pub mod board;
pub mod image;
pub mod stand;
pub mod system;

pub use board::setting;
