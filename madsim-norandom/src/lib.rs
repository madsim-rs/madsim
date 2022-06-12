/// Obtain a series of random bytes.
///
/// # Safety
///
/// Input must be a valid buffer.
///
/// Ref: <https://man7.org/linux/man-pages/man2/getrandom.2.html>
#[no_mangle]
pub unsafe extern "C" fn getrandom(mut buf: *mut u8, mut buflen: usize, _flags: u32) -> isize {
    let seed = *SEED;
    while buflen >= std::mem::size_of::<u64>() {
        (buf as *mut u64).write(seed);
        buf = buf.add(std::mem::size_of::<u64>());
        buflen -= std::mem::size_of::<u64>();
    }
    core::ptr::write_bytes(buf, *SEED as u8, buflen);
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

lazy_static::lazy_static! {
    static ref SEED: u64 = {
        std::env::var("MADSIM_TEST_SEED").map_or_else(
            |_| 0,
            |s| s.parse().expect("MADSIM_TEST_SEED must be a number"),
        )
    };
}
