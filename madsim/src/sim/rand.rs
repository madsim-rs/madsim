//! Deterministic random number generator.
//!
//! This module re-exports the prelude traits of [`rand`] crate.
//!
//! User should call [`rng()`] to retrieve the deterministic random number
//! generator from the current madsim context. **Do not** use [`random()`] or
//! [`thread_rng()`] from the rand crate directly, because no determinism is
//! guaranteed.
//!
//! # Example
//!
//! ```
//! use madsim::{runtime::Runtime, rand::{self, Rng}};
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
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
#[derive(Clone)]
pub struct RandHandle {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    rng: SmallRng,
    log: Option<Vec<u8>>,
    check: Option<(Vec<u8>, usize)>,
}

impl RandHandle {
    /// Create a new RNG using the given seed.
    pub(crate) fn new_with_seed(seed: u64) -> Self {
        let inner = Inner {
            rng: SeedableRng::seed_from_u64(seed),
            log: None,
            check: None,
        };
        RandHandle {
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
pub fn rng() -> RandHandle {
    crate::context::current(|h| h.rand.clone())
}

impl RngCore for RandHandle {
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

/// Random log for determinism check.
#[cfg_attr(docsrs, doc(cfg(feature = "sim")))]
#[derive(Debug, PartialEq, Eq)]
pub struct Log(Vec<u8>);
