/// Override the libc `gettimeofday` function. For macOS.
#[no_mangle]
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
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
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

/// Override the libc `clock_gettime` function. For Linux.
#[no_mangle]
unsafe extern "C" fn clock_gettime(
    clockid: libc::clockid_t,
    tp: *mut libc::timespec,
) -> libc::c_int {
    if let Some(time) = super::TimeHandle::try_current() {
        // inside a madsim context, use the simulated time.
        let dur = time
            .now_time()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
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

// #[no_mangle]
// extern "C" fn mach_absolute_time() -> u64 {
//     todo!("mach_absolute_time");
// }

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
    #[ignore] // TODO: intercept std Instant
    fn deterministic_std_instant() {
        let mut times = BTreeSet::new();
        for i in 0..9 {
            let runtime = Runtime::with_seed_and_config(i / 3, crate::Config::default());
            let time = runtime.block_on(async {
                let t0 = Instant::now();
                crate::time::sleep(Duration::from_secs(1)).await;
                assert!(t0.elapsed() >= Duration::from_secs(1));
                t0
            });
            times.insert(time);
        }
        assert_eq!(times.len(), 3);
    }
}
