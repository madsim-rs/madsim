#![allow(unused_mut)]
#![allow(dead_code)]

#[cfg(madsim)]
#[path = "sim.rs"]
mod sim;

#[cfg(not(madsim))]
pub use aws_config::*;
#[cfg(madsim)]
pub use sim::*;
