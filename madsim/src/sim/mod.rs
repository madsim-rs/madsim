#![deny(missing_docs)]

pub use self::config::Config;
pub(crate) use self::runtime::context;

#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub use madsim_macros::test;

pub mod collections;
mod config;
pub mod fs;
pub mod net;
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
pub mod plugin;
pub mod rand;
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
pub mod runtime;
pub mod task;
pub mod time;
mod utils;
