//! Utilities for tracking time.
//!
//!

use crate::rand::{GlobalRng, Rng};
use futures_util::{select_biased, FutureExt};
use naive_timer::Timer;
use spin::Mutex;
#[doc(no_inline)]
pub use std::time::{Duration, Instant};
use std::{future::Future, sync::Arc, time::SystemTime};

pub mod error;
mod interval;
mod sleep;
mod system_time;

pub use self::interval::{interval, interval_at, Interval, MissedTickBehavior};
pub use self::sleep::{sleep, sleep_until, Sleep};

pub(crate) struct TimeRuntime {
    handle: TimeHandle,
}

impl TimeRuntime {
    pub fn new(rand: &GlobalRng) -> Self {
        // around 2022
        let base_time = SystemTime::UNIX_EPOCH
            + Duration::from_secs(
                60 * 60 * 24 * 365 * (2022 - 1970)
                    + rand.with(|rng| rng.gen_range(0..60 * 60 * 24 * 365)),
            );
        let handle = TimeHandle {
            timer: Arc::new(Mutex::new(Timer::default())),
            clock: Arc::new(Clock::new(base_time)),
        };
        TimeRuntime { handle }
    }

    pub fn handle(&self) -> &TimeHandle {
        &self.handle
    }

    /// Advances time to the closest timer event. Returns true if succeed.
    pub fn advance_to_next_event(&self) -> bool {
        let mut timer = self.handle.timer.lock();
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
    pub fn now_instant(&self) -> Instant {
        self.handle.now_instant()
    }
}

/// Handle to a shared time source.
#[derive(Clone)]
pub struct TimeHandle {
    timer: Arc<Mutex<Timer>>,
    clock: Arc<Clock>,
}

impl TimeHandle {
    /// Returns a `TimeHandle` view over the currently running Runtime.
    pub fn current() -> Self {
        crate::context::current(|h| h.time.clone())
    }

    /// Returns a `TimeHandle` view over the currently running Runtime.
    pub fn try_current() -> Option<Self> {
        crate::context::try_current(|h| h.time.clone())
    }

    /// Return the current time.
    pub fn now_instant(&self) -> Instant {
        self.clock.now_instant()
    }

    /// Return the current time.
    pub fn now_time(&self) -> SystemTime {
        self.clock.now_time()
    }

    /// Returns the amount of time elapsed since this handle was created.
    pub fn elapsed(&self) -> Duration {
        self.clock.elapsed()
    }

    /// Advances time.
    pub fn advance(&self, duration: Duration) {
        let time = self.clock.advance(duration);
        self.timer.lock().expire(time);
    }

    /// Waits until `duration` has elapsed.
    ///
    /// It will sleep for at least 1ms to be consistent with the behavior of `tokio::time::sleep`.
    pub fn sleep(&self, duration: Duration) -> Sleep {
        self.sleep_until(self.clock.now_instant() + duration)
    }

    /// Waits until `deadline` is reached.
    ///
    /// It will sleep for at least 1ms to be consistent with the behavior of `tokio::time::sleep_until`.
    pub fn sleep_until(&self, deadline: Instant) -> Sleep {
        let min_deadline = self.clock.now_instant() + Duration::from_millis(1);
        Sleep {
            handle: self.clone(),
            deadline: deadline.max(min_deadline),
        }
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

    /// Require a `Future` to complete before the specified instant in time.
    // TODO: make it Send
    pub fn timeout_at<T: Future>(
        &self,
        deadline: Instant,
        future: T,
    ) -> impl Future<Output = Result<T::Output, error::Elapsed>> {
        let timeout = self.sleep_until(deadline);
        async move {
            select_biased! {
                res = future.fuse() => Ok(res),
                _ = timeout.fuse() => Err(error::Elapsed),
            }
        }
    }

    pub(crate) fn add_timer_at(
        &self,
        deadline: Instant,
        callback: impl FnOnce() + Send + Sync + 'static,
    ) {
        let mut timer = self.timer.lock();
        timer.add(deadline - self.clock.base_instant(), |_| callback());
    }

    pub(crate) fn add_timer(&self, dur: Duration, callback: impl FnOnce() + Send + Sync + 'static) {
        self.add_timer_at(self.clock.now_instant() + dur, callback);
    }
}

/// Require a `Future` to complete before the specified duration has elapsed.
pub fn timeout<T: Future>(
    duration: Duration,
    future: T,
) -> impl Future<Output = Result<T::Output, error::Elapsed>> {
    let handle = TimeHandle::current();
    handle.timeout(duration, future)
}

/// Require a `Future` to complete before the specified instant in time.
pub fn timeout_at<T: Future>(
    deadline: Instant,
    future: T,
) -> impl Future<Output = Result<T::Output, error::Elapsed>> {
    let handle = TimeHandle::current();
    handle.timeout_at(deadline, future)
}

/// Advances time.
///
/// Increments the saved `Instant::now()` value by `duration`.
/// Subsequent calls to `Instant::now()` will return the result of the increment.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
pub fn advance(duration: Duration) {
    let handle = TimeHandle::current();
    handle.advance(duration);
}

struct Clock {
    inner: Mutex<ClockInner>,
}

#[derive(Debug)]
struct ClockInner {
    /// Time basis for which mock time is derived.
    base_time: std::time::SystemTime,
    base_instant: std::time::Instant,
    /// The amount of mock time which has elapsed.
    advance: Duration,
}

impl Clock {
    fn new(base_time: SystemTime) -> Self {
        let clock = ClockInner {
            base_time,
            base_instant: unsafe { std::mem::zeroed() },
            advance: Duration::default(),
        };
        Clock {
            inner: Mutex::new(clock),
        }
    }

    fn set_elapsed(&self, time: Duration) {
        let mut inner = self.inner.lock();
        inner.advance = time;
    }

    fn elapsed(&self) -> Duration {
        let inner = self.inner.lock();
        inner.advance
    }

    fn advance(&self, duration: Duration) -> Duration {
        let mut inner = self.inner.lock();
        inner.advance += duration;
        inner.advance
    }

    fn base_instant(&self) -> Instant {
        let inner = self.inner.lock();
        inner.base_instant
    }

    fn now_instant(&self) -> Instant {
        let inner = self.inner.lock();
        inner.base_instant + inner.advance
    }

    fn now_time(&self) -> SystemTime {
        let inner = self.inner.lock();
        inner.base_time + inner.advance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Runtime;

    #[test]
    fn time() {
        let runtime = Runtime::new();
        runtime.block_on(async {
            // sleep for at least 1ms
            let t0 = Instant::now();
            sleep(Duration::default()).await;
            assert!(t0.elapsed() >= Duration::from_millis(1));

            let t0 = Instant::now();
            sleep_until(t0).await;
            assert!(t0.elapsed() >= Duration::from_millis(1));

            let t0 = Instant::now();

            sleep(Duration::from_secs(1)).await;
            assert!(t0.elapsed() >= Duration::from_secs(1));

            sleep_until(t0 + Duration::from_secs(2)).await;
            assert!(t0.elapsed() >= Duration::from_secs(2));

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

    #[test]
    fn test_advance() {
        let runtime = Runtime::new();
        runtime.block_on(async {
            let t0 = Instant::now();
            advance(Duration::from_secs(1));
            assert!(t0.elapsed() >= Duration::from_secs(1));
        });
    }
}
