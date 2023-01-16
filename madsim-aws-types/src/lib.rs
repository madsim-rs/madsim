#![allow(unused_mut)]
#![allow(dead_code)]

#[cfg(madsim)]
#[path = "sim.rs"]
mod sim;

#[cfg(not(madsim))]
pub use aws_types::*;
#[cfg(madsim)]
pub use sim::*;
