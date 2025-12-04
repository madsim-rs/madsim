//! Utilities for random number generation.
//!
//! This module re-exports the [`rand`] crate, except for the random number generators.

use rand::{distributions::Standard, prelude::Distribution};
use rand_xoshiro::Xoshiro256PlusPlus;

use spin::Mutex;
use std::cell::Cell;
use std::sync::Arc;

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
    rng: Xoshiro256PlusPlus,
    log: Option<Vec<u8>>,
    check: Option<(Vec<u8>, usize)>,
    buggify: bool,
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
            buggify: false,
        };
        GlobalRng {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Call function on the inner RNG.
    pub(crate) fn with<T>(&self, f: impl FnOnce(&mut Xoshiro256PlusPlus) -> T) -> T {
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
                        panic!("non-determinism detected at {time:?}");
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

    pub(crate) fn enable_buggify(&self) {
        let mut lock = self.inner.lock();
        lock.buggify = true;
    }

    pub(crate) fn disable_buggify(&self) {
        let mut lock = self.inner.lock();
        lock.buggify = false;
    }

    pub(crate) fn is_buggify_enabled(&self) -> bool {
        let lock = self.inner.lock();
        lock.buggify
    }

    pub(crate) fn buggify(&self) -> bool {
        self.is_buggify_enabled() && self.with(|rng| rng.gen_bool(0.25))
    }

    pub(crate) fn buggify_with_prob(&self, probability: f64) -> bool {
        self.is_buggify_enabled() && self.with(|rng| rng.gen_bool(probability))
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
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Log(Vec<u8>);

/// Initialize std `RandomState` with specified seed.
///
/// You should call this function before constructing any `HashMap` or `HashSet` in a new thread.
fn init_std_random_state(seed: u64) -> bool {
    SEED.with(|s| s.set(Some(seed)));
    let _ = std::collections::hash_map::RandomState::new();
    SEED.with(|s| s.replace(None)).is_none()
}

thread_local! {
    static SEED: Cell<Option<u64>> = const { Cell::new(None) };
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
unsafe extern "C" fn getrandom(buf: *mut u8, buflen: usize, _flags: u32) -> isize {
    if let Some(seed) = SEED.with(|s| s.get()) {
        assert_eq!(buflen, 16);
        std::slice::from_raw_parts_mut(buf as *mut u64, 2).fill(seed);
        SEED.with(|s| s.set(None));
        return 16;
    } else if let Some(rand) = crate::context::try_current(|h| h.rand.clone()) {
        // inside a madsim context, use the global RNG.
        if buflen == 0 {
            return 0;
        }
        let buf = std::slice::from_raw_parts_mut(buf, buflen);
        rand.with(|rng| rng.fill_bytes(buf));
        return buflen as _;
    }
    #[cfg(target_os = "linux")]
    {
        // not in madsim, call the original function.
        static GETRANDOM: std::sync::LazyLock<
            unsafe extern "C" fn(buf: *mut u8, buflen: usize, flags: u32) -> isize,
        > = std::sync::LazyLock::new(|| unsafe {
            let ptr = libc::dlsym(libc::RTLD_NEXT, c"getrandom".as_ptr() as _);
            assert!(!ptr.is_null());
            std::mem::transmute(ptr)
        });
        GETRANDOM(buf, buflen, _flags)
    }
    #[cfg(target_os = "macos")]
    {
        static GETENTROPY: std::sync::LazyLock<
            unsafe extern "C" fn(buf: *mut u8, buflen: usize) -> libc::c_int,
        > = std::sync::LazyLock::new(|| unsafe {
            let ptr = libc::dlsym(libc::RTLD_NEXT, c"getentropy".as_ptr() as _);
            assert!(!ptr.is_null());
            std::mem::transmute(ptr)
        });
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

/// Fill a buffer with random bytes.
///
/// # Safety
///
/// Input must be a valid buffer.
///
/// After Rust [1.83](https://github.com/rust-lang/rust/commit/b9d47cfa9b8f0805bfab2c254a02e598a906f102#diff-52bbc1fe799cc445f2244dcc180148b89102c1d9b5d34b14bc9425ec688b27a8),
///
/// `getentropy` is no longer used to generate random bytes in macOS, `CCRandomGenerateBytes` is used instead.
///
/// Reference:
/// - <https://github.com/apple-oss-distributions/CommonCrypto/blob/a0ac082c490b65585ade764511acfdbf1d97bc5e/include/CommonRandom.h#L56>
/// - <https://github.com/apple-oss-distributions/CommonCrypto/blob/0c0a068edd73f84671f1fba8c0e171caa114ee0a/lib/CommonRandom.c#L65-L85>
#[cfg(target_os = "macos")]
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn CCRandomGenerateBytes(bytes: *mut u8, count: usize) -> i32 {
    match getrandom(bytes, count, 0) {
        -1 => -1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;
    use std::collections::{BTreeSet, HashMap, HashSet};

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

    // https://github.com/madsim-rs/madsim/issues/201
    #[test]
    fn getrandom_should_be_deterministic() {
        let rnd_fn = || async {
            let mut dst = [0];
            getrandom::getrandom(&mut dst).unwrap();
            dst
        };
        let builder = crate::runtime::Builder::from_env();
        let seed = builder.seed;
        let set = (0..10)
            .map(|_| {
                crate::runtime::Builder {
                    seed,
                    count: 1,
                    jobs: 1,
                    config: crate::Config::default(),
                    time_limit: None,
                    check: false,
                    allow_system_thread: false,
                }
                .run(rnd_fn)
            })
            .collect::<HashSet<_>>();
        assert_eq!(set.len(), 1);
    }
}
