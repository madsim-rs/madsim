//! A deterministic simulator for distributed systems.
//!
//! ## Features
//!
//! - `rpc`: Enables RPC through network.
//! - `logger`: Enables built-in logger.
//! - `macros`: Enables `#[madsim::main]` and `#[madsim::test]` macros.

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub use madsim_macros::main;

#[cfg(all(feature = "rpc", feature = "macros"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "rpc", feature = "macros"))))]
pub use madsim_macros::{service, Request};

#[cfg(feature = "sim")]
mod sim;
#[cfg(feature = "sim")]
pub use sim::*;

#[cfg(not(feature = "sim"))]
#[path = "std/mod.rs"]
mod _std;
#[cfg(not(feature = "sim"))]
pub use _std::*;

// Includes re-exports used by macros.
#[doc(hidden)]
pub mod export {
    pub use futures;
}
