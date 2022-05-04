#[cfg(feature = "sim")]
#[path = "sim.rs"]
mod sim;

#[cfg(feature = "sim")]
pub use sim::*;
#[cfg(not(feature = "sim"))]
pub use tonic::*;
