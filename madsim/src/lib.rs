#[cfg(feature = "macros")]
pub use madsim_macros::*;

#[cfg(feature = "sim")]
pub use madsim_sim::*;

#[cfg(not(feature = "sim"))]
pub use madsim_std::*;
