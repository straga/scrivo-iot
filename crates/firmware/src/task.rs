//! The rounds of a device are started by crate `round`, which the page's crate starts its tasks with too - one
//! mechanism under both, since `web` sits below `firmware` and cannot reach up to it.

pub use round::{round, Core, ON_STACK_AT_MOST, STACK};
