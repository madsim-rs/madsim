use futures::{future::poll_fn, select, FutureExt};
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    future::Future,
    sync::{Arc, Mutex},
    task::Poll,
    time::{Duration, Instant},
};

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
        if let Some(time) = timer.next_time() {
            timer.fire(time);
            self.handle.clock.set(time);
            true
        } else {
            false
        }
    }

    /// Get the current time.
    pub fn now(&self) -> Instant {
        self.handle.now()
    }
}

#[derive(Clone)]
pub struct TimeHandle {
    timer: Arc<Mutex<Timer>>,
    clock: ClockHandle,
}

impl TimeHandle {
    pub fn now(&self) -> Instant {
        self.clock.now()
    }

    pub fn sleep(&self, duration: Duration) -> impl Future<Output = ()> {
        self.sleep_until(self.clock.now() + duration)
    }

    pub fn sleep_until(&self, deadline: Instant) -> impl Future<Output = ()> {
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

    pub fn timeout<T: Future>(
        &self,
        duration: Duration,
        future: T,
    ) -> impl Future<Output = Result<T::Output, Elapsed>> {
        let timeout = self.sleep(duration);
        async move {
            select! {
                res = future.fuse() => Ok(res),
                _ = timeout.fuse() => Err(Elapsed),
            }
        }
    }

    pub fn add_timer(&self, deadline: Instant, callback: impl FnOnce() + Send + Sync + 'static) {
        let mut timer = self.timer.lock().unwrap();
        timer.push(deadline, callback);
    }
}

#[derive(Debug, PartialEq)]
pub struct Elapsed;

pub fn now() -> Instant {
    let handle = crate::context::time_handle();
    handle.now()
}

pub fn sleep(duration: Duration) -> impl Future<Output = ()> {
    let handle = crate::context::time_handle();
    handle.sleep(duration)
}

pub fn sleep_until(deadline: Instant) -> impl Future<Output = ()> {
    let handle = crate::context::time_handle();
    handle.sleep_until(deadline)
}

pub fn timeout<T: Future>(
    duration: Duration,
    future: T,
) -> impl Future<Output = Result<T::Output, Elapsed>> {
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
    base: Instant,
    /// The amount of mock time which has elapsed.
    advance: Duration,
}

impl ClockHandle {
    fn new() -> Self {
        let clock = Clock {
            base: Instant::now(),
            advance: Duration::default(),
        };
        ClockHandle {
            inner: Arc::new(Mutex::new(clock)),
        }
    }

    fn set(&self, time: Instant) {
        let mut inner = self.inner.lock().unwrap();
        inner.advance = time.duration_since(inner.base);
    }

    fn now(&self) -> Instant {
        let inner = self.inner.lock().unwrap();
        inner.base + inner.advance
    }
}

#[derive(Default)]
struct Timer {
    queue: BinaryHeap<Event>,
}

impl Timer {
    fn push(&mut self, deadline: Instant, callback: impl FnOnce() + Send + Sync + 'static) {
        self.queue.push(Event {
            deadline,
            callback: Box::new(callback),
        });
    }

    fn next_time(&self) -> Option<Instant> {
        self.queue.peek().map(|e| e.deadline)
    }

    /// Fire all timers before or at the `time`.
    fn fire(&mut self, time: Instant) {
        while let Some(t) = self.next_time() {
            if t > time {
                break;
            }
            let event = self.queue.pop().unwrap();
            (event.callback)();
        }
    }
}

struct Event {
    deadline: Instant,
    callback: Box<dyn FnOnce() + Send + Sync + 'static>,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.deadline.eq(&other.deadline)
    }
}

impl Eq for Event {}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.deadline.partial_cmp(&self.deadline)
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Event) -> Ordering {
        other.deadline.cmp(&self.deadline)
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
            let t0 = now();

            sleep(Duration::from_secs(1)).await;
            assert_eq!(now(), t0 + Duration::from_secs(1));

            sleep_until(t0 + Duration::from_secs(2)).await;
            assert_eq!(now(), t0 + Duration::from_secs(2));

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
