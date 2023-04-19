use std::ffi::c_void;
use std::ptr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Specifies a timeout for a Kafka operation.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Timeout {
    /// Time out after the specified duration elapses.
    After(Duration),
    /// Block forever.
    Never,
}

impl std::ops::SubAssign for Timeout {
    fn sub_assign(&mut self, other: Self) {
        match (self, other) {
            (Timeout::After(lhs), Timeout::After(rhs)) => *lhs -= rhs,
            (Timeout::Never, Timeout::After(_)) => (),
            _ => panic!("subtraction of Timeout::Never is ill-defined"),
        }
    }
}

impl From<Duration> for Timeout {
    fn from(d: Duration) -> Timeout {
        Timeout::After(d)
    }
}

impl From<Option<Duration>> for Timeout {
    fn from(v: Option<Duration>) -> Timeout {
        match v {
            None => Timeout::Never,
            Some(d) => Timeout::After(d),
        }
    }
}

/// Converts the given time to the number of milliseconds since the Unix epoch.
pub fn millis_to_epoch(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as i64
}

/// Converts Rust data to and from raw pointers.
///
/// This conversion is used to pass opaque objects to the C library and vice
/// versa.
pub trait IntoOpaque: Send + Sync + Sized {
    /// Converts the object into a raw pointer.
    fn into_ptr(self) -> *mut c_void;

    /// Converts the raw pointer back to the original Rust object.
    ///
    /// # Safety
    ///
    /// The pointer must be created with [into_ptr](IntoOpaque::into_ptr).
    ///
    /// Care must be taken to not call more than once if it would result
    /// in an aliasing violation (e.g. [Box]).
    unsafe fn from_ptr(_: *mut c_void) -> Self;
}

impl IntoOpaque for () {
    fn into_ptr(self) -> *mut c_void {
        ptr::null_mut()
    }

    unsafe fn from_ptr(_: *mut c_void) -> Self {}
}

impl IntoOpaque for usize {
    fn into_ptr(self) -> *mut c_void {
        self as *mut c_void
    }

    unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        ptr as usize
    }
}

impl<T: Send + Sync> IntoOpaque for Box<T> {
    fn into_ptr(self) -> *mut c_void {
        Box::into_raw(self) as *mut c_void
    }

    unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        Box::from_raw(ptr as *mut T)
    }
}

impl<T: Send + Sync> IntoOpaque for Arc<T> {
    fn into_ptr(self) -> *mut c_void {
        Arc::into_raw(self) as *mut c_void
    }

    unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        Arc::from_raw(ptr as *const T)
    }
}
