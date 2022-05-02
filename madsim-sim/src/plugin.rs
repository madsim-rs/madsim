//! Simulator plugin framework.

use std::{
    any::{Any, TypeId},
    net::SocketAddr,
    sync::Arc,
};

use downcast_rs::{impl_downcast, DowncastSync};

use crate::{rand::RandHandle, time::TimeHandle, Config};

/// Simulator
pub trait Simulator: Any + Send + Sync + DowncastSync {
    /// Create a new simulator.
    ///
    /// This will be called on the first access via [`simulator`].
    fn new(rand: &RandHandle, time: &TimeHandle, config: &Config) -> Self
    where
        Self: Sized;

    /// Create a host.
    fn create(&self, _addr: SocketAddr) {}

    /// Reset a host.
    fn reset(&self, _addr: SocketAddr) {}
}

impl_downcast!(sync Simulator);

/// Get the simulator.
pub fn simulator<S: Simulator>() -> Arc<S> {
    crate::context::current(|h| {
        let sims = h.sims.lock().unwrap();
        sims[&TypeId::of::<S>()]
            .clone()
            .downcast_arc()
            .ok()
            .unwrap()
    })
}

/// Get the address of current task.
pub fn addr() -> SocketAddr {
    crate::context::current_addr()
}
