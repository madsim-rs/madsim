use std::time::SystemTime;

/// Override the libc `gettimeofday` function. For `SystemTime` on macOS.
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn gettimeofday(tp: *mut libc::timeval, tz: *mut libc::c_void) -> libc::c_int {
    // NOTE: tz should be NULL.
    // Linux: The use of the timezone structure is obsolete; the tz argument should normally be specified as NULL.
    // macOS: timezone is no longer used; this information is kept outside the kernel.
    if tp.is_null() {
        return 0;
    }
    if let Some(time) = super::TimeHandle::try_current() {
        // inside a madsim context, use the simulated time.
        let dur = time
            .now_time()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        tp.write(libc::timeval {
            tv_sec: dur.as_secs() as _,
            tv_usec: dur.subsec_micros() as _,
        });
        0
    } else {
        // not in madsim, call the original function.
        lazy_static::lazy_static! {
            static ref GETTIMEOFDAY: unsafe extern "C" fn(
                tp: *mut libc::timeval,
                tz: *mut libc::c_void,
            ) -> libc::c_int = unsafe {
                let ptr = libc::dlsym(libc::RTLD_NEXT, b"gettimeofday\0".as_ptr() as _);
                assert!(!ptr.is_null());
                std::mem::transmute(ptr)
            };
        }
        GETTIMEOFDAY(tp, tz)
    }
}

/// Override the libc `clock_gettime` function. For Linux and ARM64 macOS.
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn clock_gettime(
    clockid: libc::clockid_t,
    tp: *mut libc::timespec,
) -> libc::c_int {
    if let Some(time) = super::TimeHandle::try_current() {
        // inside a madsim context, use the simulated time.
        let system_time_duration = || {
            time.now_time()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
        };
        let instant_duration = || time.now_instant().duration_since(std::mem::zeroed());

        let dur = match clockid {
            // used by SystemTime
            libc::CLOCK_REALTIME => system_time_duration(),
            #[cfg(target_os = "linux")]
            libc::CLOCK_REALTIME_COARSE => system_time_duration(),
            // used by Instant
            libc::CLOCK_MONOTONIC | libc::CLOCK_MONOTONIC_RAW => instant_duration(),
            #[cfg(target_os = "linux")]
            libc::CLOCK_MONOTONIC_COARSE | libc::CLOCK_BOOTTIME => instant_duration(),
            #[cfg(target_os = "macos")]
            libc::CLOCK_UPTIME_RAW => instant_duration(),
            _ => panic!("unsupported clockid: {clockid}"),
        };
        tp.write(libc::timespec {
            tv_sec: dur.as_secs() as _,
            tv_nsec: dur.subsec_nanos() as _,
        });
        0
    } else {
        lazy_static::lazy_static! {
            static ref CLOCK_GETTIME: unsafe extern "C" fn(
                clockid: libc::clockid_t,
                tp: *mut libc::timespec,
            ) -> libc::c_int = unsafe {
                let ptr = libc::dlsym(libc::RTLD_NEXT, b"clock_gettime\0".as_ptr() as _);
                assert!(!ptr.is_null());
                std::mem::transmute(ptr)
            };
        }
        CLOCK_GETTIME(clockid, tp)
    }
}

/// Override the `mach_absolute_time` function. For `Instant` on x86_64 macOS.
#[no_mangle]
#[inline(never)]
#[cfg(target_os = "macos")]
// not used on ARM64 macOS after https://github.com/rust-lang/rust/pull/103594
#[rustversion::attr(since(1.67), cfg(not(target_arch = "aarch64")))]
extern "C" fn mach_absolute_time() -> u64 {
    if let Some(time) = super::TimeHandle::try_current() {
        // inside a madsim context, use the simulated time.
        let instant = time.now_instant();
        unsafe { std::mem::transmute(instant) }
    } else {
        lazy_static::lazy_static! {
            static ref MACH_ABSOLUTE_TIME: extern "C" fn() -> u64 = unsafe {
                let ptr = libc::dlsym(libc::RTLD_NEXT, b"mach_absolute_time\0".as_ptr() as _);
                assert!(!ptr.is_null());
                std::mem::transmute(ptr)
            };
        }
        MACH_ABSOLUTE_TIME()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Runtime;
    use std::collections::BTreeSet;
    use std::time::{Duration, Instant, SystemTime};

    #[test]
    fn deterministic_std_system_time() {
        let _real_now = SystemTime::now();

        let mut times = BTreeSet::new();
        for i in 0..9 {
            let runtime = Runtime::with_seed_and_config(i / 3, crate::Config::default());
            let time = runtime.block_on(async {
                let t0 = SystemTime::now();
                crate::time::sleep(Duration::from_secs(1)).await;
                assert!(t0.elapsed().unwrap() >= Duration::from_secs(1));
                t0
            });
            times.insert(time);
        }
        assert_eq!(times.len(), 3);
    }

    #[test]
    fn deterministic_std_instant() {
        let mut times = BTreeSet::new();
        for i in 0..9 {
            let runtime = Runtime::with_seed_and_config(i / 3, crate::Config::default());
            let dur = runtime.block_on(async {
                let t0 = Instant::now();
                crate::time::sleep(Duration::from_secs(1)).await;
                let dur = t0.elapsed();
                assert!(dur >= Duration::from_secs(1));
                dur
            });
            times.insert(dur);
        }
        assert_eq!(times.len(), 1);
    }
}
