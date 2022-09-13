#![allow(dead_code)]
#![allow(unused_mut)]

#[cfg(madsim)]
#[path = "sim.rs"]
mod sim;

#[cfg(not(madsim))]
pub use aws_sdk_s3::*;
#[cfg(madsim)]
pub use sim::*;
