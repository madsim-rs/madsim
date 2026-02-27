//! A deterministic simulator for distributed systems.
//!
//! ## Features
//!
//! - `rpc`: Enables built-in RPC framework.
//! - `macros`: Enables `#[madsim::main]` and `#[madsim::test]` macros.

#![cfg_attr(docsrs, feature(doc_cfg))]
// Allow Rust 2024's stricter unsafe-in-unsafe-fn rules (temporary fix)
#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(all(feature = "rpc", feature = "macros"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "rpc", feature = "macros"))))]
pub use madsim_macros::{Request, service};

#[cfg(madsim)]
mod sim;
#[cfg(madsim)]
pub use sim::*;

#[cfg(not(madsim))]
#[path = "std/mod.rs"]
mod _std;
#[cfg(not(madsim))]
pub use _std::*;

// Includes re-exports used by macros.
#[doc(hidden)]
pub mod export {
    pub use futures_util as futures;
}
