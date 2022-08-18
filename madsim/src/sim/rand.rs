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

use spin::Mutex;
use std::cell::Cell;
use std::sync::Arc;

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
#[cfg_attr(docsrs, doc(cfg(madsim)))]
#[derive(Clone)]
pub struct GlobalRng {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    seed: u64,
    rng: SmallRng,
    log: Option<Vec<u8>>,
    check: Option<(Vec<u8>, usize)>,
}

impl GlobalRng {
    /// Create a new RNG using the given seed.
    pub(crate) fn new_with_seed(seed: u64) -> Self {
        // XXX: call this function to make sure it won't be gc.
        unsafe { getentropy(std::ptr::null_mut(), 0) };
        if !init_std_random_state(seed) {
            tracing::warn!(
                "failed to initialize std random state, std HashMap will not be deterministic"
            );
        }

        let inner = Inner {
            seed,
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
        let mut lock = self.inner.lock();
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

    pub(crate) fn seed(&self) -> u64 {
        let lock = self.inner.lock();
        lock.seed
    }

    pub(crate) fn enable_check(&self, log: Log) {
        let mut lock = self.inner.lock();
        lock.check = Some((log.0, 0));
    }

    pub(crate) fn enable_log(&self) {
        let mut lock = self.inner.lock();
        lock.log = Some(Vec::new());
    }

    pub(crate) fn take_log(&self) -> Option<Log> {
        let mut lock = self.inner.lock();
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
#[cfg_attr(docsrs, doc(cfg(madsim)))]
#[derive(Debug, PartialEq, Eq)]
pub struct Log(Vec<u8>);

/// Initialize std `RandomState` with specified seed.
///
/// You should call this function before constructing any `HashMap` or `HashSet` in a new thread.
fn init_std_random_state(seed: u64) -> bool {
    SEED.with(|s| s.set(Some(seed)));
    let _ = std::collections::hash_map::RandomState::new();
    SEED.with(|s| s.replace(None)).is_none()
}

thread_local! {
    static SEED: Cell<Option<u64>> = Cell::new(None);
}

/// Obtain a series of random bytes.
///
/// Returns the number of bytes that were copied to the buffer buf.
///
/// # Safety
///
/// Input must be a valid buffer.
///
/// Ref: <https://man7.org/linux/man-pages/man2/getrandom.2.html>
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn getrandom(mut buf: *mut u8, mut buflen: usize, _flags: u32) -> isize {
    if let Some(seed) = SEED.with(|s| s.get()) {
        assert_eq!(buflen, 16);
        std::slice::from_raw_parts_mut(buf as *mut u64, 2).fill(seed);
        SEED.with(|s| s.set(None));
        return 16;
    } else if let Some(rand) = crate::context::try_current(|h| h.rand.clone()) {
        // inside a madsim context, use the global RNG.
        let len = buflen;
        while buflen >= std::mem::size_of::<u64>() {
            (buf as *mut u64).write(rand.with(|rng| rng.gen()));
            buf = buf.add(std::mem::size_of::<u64>());
            buflen -= std::mem::size_of::<u64>();
        }
        let val = rand.with(|rng| rng.gen::<u64>().to_ne_bytes());
        core::ptr::copy(val.as_ptr(), buf, buflen);
        return len as _;
    }
    #[cfg(target_os = "linux")]
    {
        // not in madsim, call the original function.
        lazy_static::lazy_static! {
            static ref GETRANDOM: unsafe extern "C" fn(buf: *mut u8, buflen: usize, flags: u32) -> isize = unsafe {
                let ptr = libc::dlsym(libc::RTLD_NEXT, b"getrandom\0".as_ptr() as _);
                assert!(!ptr.is_null());
                std::mem::transmute(ptr)
            };
        }
        GETRANDOM(buf, buflen, _flags)
    }
    #[cfg(target_os = "macos")]
    {
        lazy_static::lazy_static! {
            static ref GETENTROPY: unsafe extern "C" fn(buf: *mut u8, buflen: usize) -> libc::c_int = unsafe {
                let ptr = libc::dlsym(libc::RTLD_NEXT, b"getentropy\0".as_ptr() as _);
                assert!(!ptr.is_null());
                std::mem::transmute(ptr)
            };
        }
        match GETENTROPY(buf, buflen) {
            -1 => -1,
            0 => buflen as _,
            _ => unreachable!(),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    compile_error!("unsupported os");
}

/// Fill a buffer with random bytes.
///
/// # Safety
///
/// Input must be a valid buffer.
///
/// Ref: <https://man7.org/linux/man-pages/man3/getentropy.3.html>
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn getentropy(buf: *mut u8, buflen: usize) -> i32 {
    if buflen > 256 {
        return -1;
    }
    match getrandom(buf, buflen, 0) {
        -1 => -1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;
    use std::collections::{BTreeSet, HashMap};

    #[test]
    #[cfg_attr(target_os = "linux", ignore)]
    // NOTE:
    //   Deterministic rand is only available on macOS.
    //   On linux, the call stack is `rand` -> `getrandom` -> `SYS_getrandom`,
    //   which is hard to intercept.
    fn deterministic_rand() {
        let mut seqs = BTreeSet::new();
        for i in 0..9 {
            let seq = std::thread::spawn(move || {
                let runtime = Runtime::with_seed_and_config(i / 3, crate::Config::default());
                runtime
                    .block_on(async { (0..10).map(|_| rand::random::<u64>()).collect::<Vec<_>>() })
            })
            .join()
            .unwrap();
            seqs.insert(seq);
        }
        assert_eq!(seqs.len(), 3);
    }

    #[test]
    fn deterministic_std_hashmap() {
        let mut seqs = BTreeSet::new();
        for i in 0..9 {
            let seq = std::thread::spawn(move || {
                let runtime = Runtime::with_seed_and_config(i / 3, crate::Config::default());
                runtime.block_on(async {
                    let set = (0..10).map(|i| (i, i)).collect::<HashMap<_, _>>();
                    set.into_iter().collect::<Vec<_>>()
                })
            })
            .join()
            .unwrap();
            seqs.insert(seq);
        }
        assert_eq!(seqs.len(), 3, "hashmap is not deterministic");
    }
}
