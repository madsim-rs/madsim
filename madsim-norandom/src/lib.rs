/// Obtain a series of random bytes.
///
/// # Safety
///
/// Input must be a valid buffer.
///
/// Ref: <https://man7.org/linux/man-pages/man2/getrandom.2.html>
#[no_mangle]
pub unsafe extern "C" fn getrandom(mut buf: *mut u8, mut buflen: usize, _flags: u32) -> isize {
    // let s = core::slice::from_raw_parts(buf, buflen);
    while buflen >= std::mem::size_of::<u64>() {
        (buf as *mut u64).write(RNG.with(|rng| rng.borrow_mut().gen()));
        buf = buf.add(std::mem::size_of::<u64>());
        buflen -= std::mem::size_of::<u64>();
    }
    let val = RNG.with(|rng| rng.borrow_mut().gen::<u64>().to_ne_bytes());
    core::ptr::copy(val.as_ptr(), buf, buflen);
    // eprintln!("getrandom: {:?}", s);
    0
}

/// Fill a buffer with random bytes.
///
/// # Safety
///
/// Input must be a valid buffer.
///
/// Ref: <https://man7.org/linux/man-pages/man3/getentropy.3.html>
#[no_mangle]
pub unsafe extern "C" fn getentropy(buf: *mut u8, buflen: usize) -> i32 {
    if buflen > 256 {
        return -1;
    }
    getrandom(buf, buflen, 0) as _
}

use rand::{prelude::SmallRng, Rng, SeedableRng};
use std::cell::RefCell;

thread_local! {
    static RNG: RefCell<SmallRng> = RefCell::new(SmallRng::seed_from_u64(*SEED));
}
lazy_static::lazy_static! {
    static ref SEED: u64 = {
        std::env::var("MADSIM_TEST_SEED").map_or_else(
            |_| 0,
            |s| s.parse().expect("MADSIM_TEST_SEED must be a number"),
        )
    };
}
