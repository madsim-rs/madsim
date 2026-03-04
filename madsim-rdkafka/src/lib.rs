#![cfg_attr(docsrs, feature(doc_cfg))]
// Allow the legacy behavior for unsafe operations in unsafe functions.
// This is FFI code from rdkafka that requires extensive changes to properly
// wrap all unsafe operations.
#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(madsim)]
mod sim;
#[cfg(not(madsim))]
#[path = "std/mod.rs"]
mod std_;

#[cfg(madsim)]
pub use sim::*;
#[cfg(not(madsim))]
pub use std_::*;
