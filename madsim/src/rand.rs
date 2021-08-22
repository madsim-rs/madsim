//! Deterministic random number generator.
//!
//! This module re-exports the prelude traits of [`rand`] crate.
//!
//! User should call [`rng()`] to retrieve the deterministic random number
//! generator from the current madsim context. **Do not** use [`random()`] or
//! [`thread_rng()`] from the rand crate directly, because no deterministic is
//! guaranteed.
//!
//! # Example
//!
//! ```
//! use madsim::{Runtime, rand::{self, Rng}};
//!
//! Runtime::new().block_on(async {
//!     let mut rng = rand::rng();
//!     rng.gen_bool(0.5);
//!     rng.gen_range(0..10);
//! });
//! ```
//!
//! [`rand`]: rand
//! [`rng()`]: rng
//! [`random()`]: rand::random
//! [`thread_rng()`]: rand::thread_rng

use rand::prelude::SmallRng;
#[doc(no_inline)]
pub use rand::prelude::{
    CryptoRng, Distribution, IteratorRandom, Rng, RngCore, SeedableRng, SliceRandom,
};
use std::sync::{Arc, Mutex};

/// Handle to a shared random state.
#[derive(Debug, Clone)]
pub struct RandHandle {
    rng: Arc<Mutex<SmallRng>>,
}

impl RandHandle {
    /// Create a new RNG using the given seed.
    pub(crate) fn new_with_seed(seed: u64) -> Self {
        RandHandle {
            rng: Arc::new(Mutex::new(SeedableRng::seed_from_u64(seed))),
        }
    }

    /// Call function on the inner RNG.
    pub(crate) fn with<T>(&self, f: impl FnOnce(&mut SmallRng) -> T) -> T {
        let mut lock = self.rng.lock().unwrap();
        f(&mut *lock)
    }
}

/// Retrieve the deterministic random number generator from the current madsim context.
pub fn rng() -> RandHandle {
    crate::context::rand_handle()
}

impl RngCore for RandHandle {
    fn next_u32(&mut self) -> u32 {
        self.rng.lock().unwrap().next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.rng.lock().unwrap().next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.lock().unwrap().fill_bytes(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.rng.lock().unwrap().try_fill_bytes(dest)
    }
}
