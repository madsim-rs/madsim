#[cfg(madsim)]
#[path = "sim.rs"]
mod sim;

#[cfg(not(madsim))]
pub use etcd_client::*;
#[cfg(madsim)]
pub use sim::*;
