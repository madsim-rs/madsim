//! A deterministic simulator for distributed systems.
//!
//! ## Features
//!
//! - `rpc`: Enables RPC through network.
//! - `logger`: Enables built-in logger.
//! - `macros`: Enables `#[madsim::main]` and `#[madsim::test]` macros.

#[cfg(feature = "macros")]
pub use madsim_macros::*;

#[cfg(feature = "sim")]
mod sim;
#[cfg(feature = "sim")]
pub use sim::*;

#[cfg(not(feature = "sim"))]
#[path = "std/mod.rs"]
mod _std;
#[cfg(not(feature = "sim"))]
pub use _std::*;

#[doc(hidden)]
pub mod export {
    pub use futures;
}
