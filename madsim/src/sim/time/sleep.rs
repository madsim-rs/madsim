use super::*;
use std::{fmt, pin::Pin, task::Poll};

/// Waits until `duration` has elapsed.
pub fn sleep(duration: Duration) -> Sleep {
    let handle = TimeHandle::current();
    handle.sleep(duration)
}

/// Waits until `deadline` is reached.
pub fn sleep_until(deadline: Instant) -> Sleep {
    let handle = TimeHandle::current();
    handle.sleep_until(deadline)
}

/// Future returned by [`sleep`] and [`sleep_until`].
///
/// [`sleep`]: sleep()
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Sleep {
    pub(super) handle: TimeHandle,
    pub(super) deadline: Instant,
}

impl Sleep {
    /// Returns the instant at which the future will complete.
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Returns `true` if `Sleep` has elapsed.
    ///
    /// A `Sleep` instance is elapsed when the requested duration has elapsed.
    pub fn is_elapsed(&self) -> bool {
        self.handle.clock.now_instant() >= self.deadline
    }

    /// Resets the `Sleep` instance to a new deadline.
    pub fn reset(mut self: Pin<&mut Self>, deadline: Instant) {
        self.deadline = deadline;
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if self.is_elapsed() {
            return Poll::Ready(());
        }
        let waker = cx.waker().clone();
        self.handle.add_timer_at(self.deadline, || waker.wake());
        Poll::Pending
    }
}

impl fmt::Debug for Sleep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sleep")
            .field("deadline", &self.deadline)
            .finish()
    }
}
