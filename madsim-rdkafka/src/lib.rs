#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(madsim)]
mod sim;
#[cfg(not(madsim))]
#[path = "std/mod.rs"]
mod std_;

#[cfg(madsim)]
pub use sim::*;
#[cfg(not(madsim))]
pub use std_::*;
