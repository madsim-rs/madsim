#[cfg(madsim)]
#[path = "sim.rs"]
mod sim;

#[cfg(madsim)]
pub use sim::*;
#[cfg(not(madsim))]
pub use tonic::*;
