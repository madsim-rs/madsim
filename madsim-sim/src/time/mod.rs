//! Utilities for tracking time.
//!
//!

pub use self::instant::Instant;
use futures::{future::poll_fn, select_biased, FutureExt};
use naive_timer::Timer;
#[doc(no_inline)]
pub use std::time::Duration;
use std::{
    future::Future,
    sync::{Arc, Mutex},
    task::Poll,
};

mod instant;

pub(crate) struct TimeRuntime {
    handle: TimeHandle,
}

impl TimeRuntime {
    pub fn new() -> Self {
        let handle = TimeHandle {
            timer: Arc::new(Mutex::new(Timer::default())),
            clock: ClockHandle::new(),
        };
        TimeRuntime { handle }
    }

    pub fn handle(&self) -> &TimeHandle {
        &self.handle
    }

    /// Advance the time and fire timers.
    pub fn advance(&self) -> bool {
        let mut timer = self.handle.timer.lock().unwrap();
        if let Some(mut time) = timer.next() {
            // WARN: in some platform such as M1 macOS,
            //       let t0: Instant;
            //       let t1: Instant;
            //       t0 + (t1 - t0) < t1 !!
            // we should add eps to make sure 'now >= deadline' and avoid deadlock
            time += Duration::from_nanos(50);

            timer.expire(time);
            self.handle.clock.set_elapsed(time);
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    /// Get the current time.
    pub fn now(&self) -> Instant {
        self.handle.now()
    }
}

/// Handle to a shared time source.
#[derive(Clone)]
pub(crate) struct TimeHandle {
    timer: Arc<Mutex<Timer>>,
    clock: ClockHandle,
}

impl TimeHandle {
    #[allow(dead_code)]
    /// Returns a `TimeHandle` view over the currently running Runtime.
    pub fn try_current() -> Option<Self> {
        crate::context::try_time_handle()
    }

    /// Return the current time.
    pub fn now(&self) -> Instant {
        self.clock.now()
    }

    /// Returns the amount of time elapsed since this handle was created.
    pub fn elapsed(&self) -> Duration {
        self.clock.elapsed()
    }

    /// Waits until `duration` has elapsed.
    pub fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        self.sleep_until(self.clock.now() + duration)
    }

    /// Waits until `deadline` is reached.
    pub fn sleep_until(&self, deadline: Instant) -> impl Future<Output = ()> + Send {
        let handle = self.clone();
        poll_fn(move |cx| {
            if handle.clock.now() >= deadline {
                return Poll::Ready(());
            }
            let waker = cx.waker().clone();
            handle.add_timer(deadline, || waker.wake());
            Poll::Pending
        })
    }

    /// Require a `Future` to complete before the specified duration has elapsed.
    // TODO: make it Send
    pub fn timeout<T: Future>(
        &self,
        duration: Duration,
        future: T,
    ) -> impl Future<Output = Result<T::Output, error::Elapsed>> {
        let timeout = self.sleep(duration);
        async move {
            select_biased! {
                res = future.fuse() => Ok(res),
                _ = timeout.fuse() => Err(error::Elapsed),
            }
        }
    }

    pub(crate) fn add_timer(
        &self,
        deadline: Instant,
        callback: impl FnOnce() + Send + Sync + 'static,
    ) {
        let mut timer = self.timer.lock().unwrap();
        timer.add(deadline - self.clock.base(), |_| callback());
    }
}

/// Time error types.
pub mod error {
    /// Error returned by `timeout`.
    #[derive(Debug, PartialEq)]
    pub struct Elapsed;
}

/// Waits until `duration` has elapsed.
pub fn sleep(duration: Duration) -> impl Future<Output = ()> + Send {
    let handle = crate::context::time_handle();
    handle.sleep(duration)
}

/// Waits until `deadline` is reached.
pub fn sleep_until(deadline: Instant) -> impl Future<Output = ()> + Send {
    let handle = crate::context::time_handle();
    handle.sleep_until(deadline)
}

/// Require a `Future` to complete before the specified duration has elapsed.
pub fn timeout<T: Future>(
    duration: Duration,
    future: T,
) -> impl Future<Output = Result<T::Output, error::Elapsed>> {
    let handle = crate::context::time_handle();
    handle.timeout(duration, future)
}

#[derive(Clone)]
struct ClockHandle {
    inner: Arc<Mutex<Clock>>,
}

#[derive(Debug)]
struct Clock {
    /// Time basis for which mock time is derived.
    base: std::time::Instant,
    /// The amount of mock time which has elapsed.
    advance: Duration,
}

impl ClockHandle {
    fn new() -> Self {
        let clock = Clock {
            base: std::time::Instant::now(),
            advance: Duration::default(),
        };
        ClockHandle {
            inner: Arc::new(Mutex::new(clock)),
        }
    }

    fn set_elapsed(&self, time: Duration) {
        let mut inner = self.inner.lock().unwrap();
        inner.advance = time;
    }

    fn elapsed(&self) -> Duration {
        let inner = self.inner.lock().unwrap();
        inner.advance
    }

    fn base(&self) -> Instant {
        let inner = self.inner.lock().unwrap();
        Instant::from_std(inner.base)
    }

    fn now(&self) -> Instant {
        let inner = self.inner.lock().unwrap();
        Instant::from_std(inner.base + inner.advance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Runtime;

    #[test]
    fn time() {
        let runtime = Runtime::new();
        runtime.block_on(async {
            let t0 = Instant::now();

            sleep(Duration::from_secs(1)).await;
            assert_eq!(t0.elapsed(), Duration::from_secs(1));

            sleep_until(t0 + Duration::from_secs(2)).await;
            assert_eq!(t0.elapsed(), Duration::from_secs(2));

            assert!(
                timeout(Duration::from_secs(2), sleep(Duration::from_secs(1)))
                    .await
                    .is_ok()
            );
            assert!(
                timeout(Duration::from_secs(1), sleep(Duration::from_secs(2)))
                    .await
                    .is_err()
            );
        });
    }
}
