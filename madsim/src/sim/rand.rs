//! Utilities for random number generation.
//!
//! This module re-exports the [`rand`] crate, except for the random number generators.
//!
//! User should call [`thread_rng()`] to retrieve the deterministic random number
//! generator from the current madsim context. **Do not** use [`rand`] crate directly,
//! because no determinism is guaranteed.
//!
//! # Example
//!
//! ```
//! use madsim::{runtime::Runtime, rand::{thread_rng, Rng}};
//!
//! Runtime::new().block_on(async {
//!     let mut rng = thread_rng();
//!     rng.gen_bool(0.5);
//!     rng.gen_range(0..10);
//! });
//! ```
//!
//! [`rand`]: rand

use rand::{
    distributions::Standard,
    prelude::{Distribution, SmallRng},
};
use std::sync::{Arc, Mutex};

// TODO: mock `rngs` module

#[doc(no_inline)]
pub use rand::{distributions, seq, CryptoRng, Error, Fill, Rng, RngCore, SeedableRng};

/// Convenience re-export of common members
pub mod prelude {
    #[doc(no_inline)]
    pub use super::{random, thread_rng};
    #[doc(no_inline)]
    pub use rand::prelude::{
        CryptoRng, Distribution, IteratorRandom, Rng, RngCore, SeedableRng, SliceRandom,
    };
}

/// Global deterministic random number generator.
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
#[derive(Clone)]
pub struct GlobalRng {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    rng: SmallRng,
    log: Option<Vec<u8>>,
    check: Option<(Vec<u8>, usize)>,
}

impl GlobalRng {
    /// Create a new RNG using the given seed.
    pub(crate) fn new_with_seed(seed: u64) -> Self {
        let inner = Inner {
            rng: SeedableRng::seed_from_u64(seed),
            log: None,
            check: None,
        };
        GlobalRng {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Call function on the inner RNG.
    pub(crate) fn with<T>(&self, f: impl FnOnce(&mut SmallRng) -> T) -> T {
        let mut lock = self.inner.lock().unwrap();
        let ret = f(&mut lock.rng);
        // log or check
        if lock.log.is_some() || lock.check.is_some() {
            let t = crate::time::TimeHandle::try_current().map(|t| t.elapsed());
            fn hash_u128(x: u128) -> u8 {
                x.to_ne_bytes().iter().fold(0, |a, b| a ^ b)
            }
            let v = lock.rng.clone().gen::<u8>() ^ hash_u128(t.unwrap_or_default().as_nanos());
            if let Some(log) = &mut lock.log {
                log.push(v);
            }
            if let Some((check, i)) = &mut lock.check {
                if check.get(*i) != Some(&v) {
                    if let Some(time) = t {
                        panic!("non-determinism detected at {:?}", time);
                    }
                    panic!("non-determinism detected");
                }
                *i += 1;
            }
        }
        ret
    }

    pub(crate) fn enable_check(&self, log: Log) {
        let mut lock = self.inner.lock().unwrap();
        lock.check = Some((log.0, 0));
    }

    pub(crate) fn enable_log(&self) {
        let mut lock = self.inner.lock().unwrap();
        lock.log = Some(Vec::new());
    }

    pub(crate) fn take_log(&self) -> Option<Log> {
        let mut lock = self.inner.lock().unwrap();
        lock.log
            .take()
            .or_else(|| lock.check.take().map(|(s, _)| s))
            .map(Log)
    }
}

/// Retrieve the deterministic random number generator from the current madsim context.
pub fn thread_rng() -> GlobalRng {
    crate::context::current(|h| h.rand.clone())
}

impl RngCore for GlobalRng {
    fn next_u32(&mut self) -> u32 {
        self.with(|rng| rng.next_u32())
    }

    fn next_u64(&mut self) -> u64 {
        self.with(|rng| rng.next_u64())
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.with(|rng| rng.fill_bytes(dest))
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.with(|rng| rng.try_fill_bytes(dest))
    }
}

/// Generates a random value using the global random number generator.
#[inline]
pub fn random<T>() -> T
where
    Standard: Distribution<T>,
{
    thread_rng().gen()
}

/// Random log for determinism check.
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
#[derive(Debug, PartialEq, Eq)]
pub struct Log(Vec<u8>);
